//! Play history: one row per play, written from the playback-session pipeline
//! (design.md Q134).
//!
//! The player already reports a `PlaybackSession` every few seconds. Each report is compared
//! with the one before it, in the session's own lifecycle hook and transaction:
//!
//! - Only time spent playing counts. Moving forward by about the wall time between two reports
//!   is playing (credited by the smaller of the two, so 2x speed counts real minutes); a seek
//!   counts the wall time between the reports, at most a minute, never the distance jumped.
//! - A play continues while it is resumed within [`RESUME_WINDOW_SECS`] of its last activity,
//!   on the same device: pausing and resuming, or closing and reopening the same item, is one
//!   play. After a longer gap the next playing starts a new one.
//! - A play is complete by the same rule as watched: the session's `completedAt`, or a position
//!   at [`WATCHED_FRACTION`] of the duration.
//!
//! Content ids are plain references with title snapshots, so history outlives its titles.

pub mod access;
pub mod devices;
pub mod service;
pub mod stats;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use async_graphql::ErrorExtensions;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use graphql_orm::graphql::filters::{BoolFilter, DateFilter, StringFilter};
use graphql_orm::graphql::orm::{
    ChangeAction, MutationContext, MutationEvent, MutationPhase, OrderDirection, TransactionMode,
};

use crate::db::Database;
use crate::graphql::entities::*;

/// The fraction of a title that marks it watched; the web player uses the same value.
pub const WATCHED_FRACTION: f64 = 0.92;
/// A pause or a reopen within this long continues the same play.
pub const RESUME_WINDOW_SECS: i64 = 30 * 60;
/// Faster than this, a move forward is a seek rather than playing (covers 2x and 3x speech).
const MAX_PLAY_RATE: f64 = 3.0;
/// Reports are not perfectly spaced; this much extra movement still reads as playing.
const POSITION_SLACK_SECS: f64 = 5.0;
/// The most wall time one report can credit when the position jumped.
const JUMP_CREDIT_CAP_SECS: f64 = 60.0;
/// Administrators' retention, in days; absent, empty or 0 keeps everything.
pub const RETENTION_SETTING: &str = "play_history.retention_days";
/// Set once the first progress import ran, so it runs on its own only once.
pub const IMPORTED_SETTING: &str = "play_history.imported_at";

/// The canonical timestamp: UTC with milliseconds and `Z`, the same text the web client's
/// `toISOString()` writes, so stored times compare as text.
pub fn timestamp(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn now() -> String {
    timestamp(Utc::now())
}

pub fn parse_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}

/// One report from a player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Checkpoint {
    pub position: f64,
    pub playing: bool,
    pub at: DateTime<Utc>,
}

/// Seconds of playing between two reports of one session.
pub fn credited_seconds(before: Checkpoint, after: Checkpoint) -> f64 {
    let wall = (after.at - before.at).num_milliseconds() as f64 / 1000.0;
    if wall <= 0.0 || !(before.playing || after.playing) {
        return 0.0;
    }
    let moved = after.position - before.position;
    if !moved.is_finite() {
        return 0.0;
    }
    if moved > 0.0 && moved <= wall * MAX_PLAY_RATE + POSITION_SLACK_SECS {
        return moved.min(wall);
    }
    if moved == 0.0 {
        // Stalled or buffering: nothing played.
        return 0.0;
    }
    // A seek, either way. It was playing on both sides, so the wall time between them was
    // spent playing, up to a minute; the distance jumped never counts.
    if before.playing && after.playing {
        wall.min(JUMP_CREDIT_CAP_SECS)
    } else {
        0.0
    }
}

/// Watched: the player said so, or it reached [`WATCHED_FRACTION`].
pub fn is_complete(position: f64, duration: Option<f64>, completed_at: Option<&str>) -> bool {
    completed_at.is_some()
        || duration
            .is_some_and(|duration| duration > 0.0 && position / duration >= WATCHED_FRACTION)
}

/// Whether activity at `now` continues a play last active at `last_activity`.
pub fn continues(last_activity: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    now >= last_activity && now - last_activity <= Duration::seconds(RESUME_WINDOW_SECS)
}

fn checkpoint(session: &PlaybackSession) -> Option<Checkpoint> {
    Some(Checkpoint {
        position: session.current_position,
        playing: session.is_playing,
        at: parse_time(&session.last_updated_at)?,
    })
}

fn eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_owned()),
        ..Default::default()
    }
}

fn db_error(error: impl std::fmt::Display) -> async_graphql::Error {
    async_graphql::Error::new(error.to_string())
}

/// What a play is of, resolved once when it starts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub content_type: String,
    pub media_file_id: Option<String>,
    pub movie_id: Option<String>,
    pub episode_id: Option<String>,
    pub show_id: Option<String>,
    pub track_id: Option<String>,
    pub album_id: Option<String>,
    pub artist_id: Option<String>,
    pub audiobook_id: Option<String>,
    pub chapter_id: Option<String>,
    pub title: String,
    pub parent_title: Option<String>,
    pub subtitle: Option<String>,
}

macro_rules! by_id {
    ($tx:expr, $ty:ty, $where:ident, $id:expr) => {
        $tx.query::<$ty>()
            .filter($where {
                id: Some(eq($id)),
                ..Default::default()
            })
            .fetch_one()
            .await
            .map_err(db_error)?
    };
}

/// Names and parent ids for a play of a movie, episode, track or audiobook (chapter).
pub(crate) async fn resolve_snapshot(
    tx: &mut MutationContext<'_>,
    content_type: Option<&str>,
    ids: SnapshotIds<'_>,
) -> async_graphql::Result<Option<Snapshot>> {
    let mut snapshot = Snapshot {
        media_file_id: ids.media_file.map(str::to_owned),
        ..Default::default()
    };
    // A chapter is reached through its book; a file can name the chapter when the caller did not.
    let mut chapter_id = ids.chapter.map(str::to_owned);
    if chapter_id.is_none()
        && ids.audiobook.is_some()
        && let Some(file_id) = ids.media_file
        && let Some(file) = by_id!(tx, MediaFile, MediaFileWhereInput, file_id)
    {
        chapter_id = file.chapter_id;
    }
    let kind = content_type.unwrap_or_default();
    if let Some(movie_id) = ids.movie.filter(|_| kind.is_empty() || kind == "movie") {
        let Some(movie) = by_id!(tx, Movie, MovieWhereInput, movie_id) else {
            return Ok(None);
        };
        snapshot.content_type = "movie".into();
        snapshot.movie_id = Some(movie.id);
        snapshot.title = movie.title;
        snapshot.subtitle = movie.year.map(|year| year.to_string());
    } else if let Some(episode_id) = ids.episode {
        let Some(episode) = by_id!(tx, Episode, EpisodeWhereInput, episode_id) else {
            return Ok(None);
        };
        let show = by_id!(tx, Show, ShowWhereInput, &episode.show_id);
        snapshot.content_type = "episode".into();
        snapshot.episode_id = Some(episode.id);
        snapshot.show_id = Some(episode.show_id);
        snapshot.title = episode
            .title
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| format!("Episode {}", episode.episode));
        snapshot.parent_title = show.map(|show| show.name);
        snapshot.subtitle = Some(format!("S{} · E{}", episode.season, episode.episode));
    } else if let Some(track_id) = ids.track {
        let Some(track) = by_id!(tx, Track, TrackWhereInput, track_id) else {
            return Ok(None);
        };
        let album = by_id!(tx, Album, AlbumWhereInput, &track.album_id);
        let artist_id = track
            .artist_id
            .clone()
            .or_else(|| album.as_ref().map(|album| album.artist_id.clone()));
        let artist_name = match (&track.artist_name, &artist_id) {
            (Some(name), _) if !name.trim().is_empty() => Some(name.clone()),
            (_, Some(artist_id)) => {
                by_id!(tx, Artist, ArtistWhereInput, artist_id).map(|artist| artist.name)
            }
            _ => None,
        };
        snapshot.content_type = "track".into();
        snapshot.track_id = Some(track.id);
        snapshot.album_id = Some(track.album_id);
        snapshot.artist_id = artist_id;
        snapshot.title = track.title;
        snapshot.parent_title = artist_name;
        snapshot.subtitle = album.map(|album| album.name);
    } else if let Some(audiobook_id) = ids.audiobook {
        let Some(book) = by_id!(tx, Audiobook, AudiobookWhereInput, audiobook_id) else {
            return Ok(None);
        };
        snapshot.content_type = "audiobook".into();
        snapshot.audiobook_id = Some(book.id);
        snapshot.title = book.title;
        snapshot.parent_title = book.author_name;
        if let Some(chapter_id) = chapter_id
            && let Some(chapter) = by_id!(tx, Chapter, ChapterWhereInput, &chapter_id)
        {
            snapshot.subtitle = Some(
                chapter
                    .title
                    .filter(|title| !title.trim().is_empty())
                    .unwrap_or_else(|| format!("Chapter {}", chapter.chapter_number)),
            );
            snapshot.chapter_id = Some(chapter.id);
        }
    } else {
        return Ok(None);
    }
    Ok(Some(snapshot))
}

#[derive(Clone, Copy, Default)]
pub(crate) struct SnapshotIds<'a> {
    pub media_file: Option<&'a str>,
    pub movie: Option<&'a str>,
    pub episode: Option<&'a str>,
    pub track: Option<&'a str>,
    pub audiobook: Option<&'a str>,
    pub chapter: Option<&'a str>,
}

/// The filter that finds an earlier play of the same item.
fn same_item(session: &PlaybackSession) -> Option<PlayHistoryWhereInput> {
    let mut filter = PlayHistoryWhereInput {
        user_id: Some(eq(&session.user_id)),
        ..Default::default()
    };
    if let Some(id) = &session.episode_id {
        filter.episode_id = Some(eq(id));
    } else if let Some(id) = &session.movie_id {
        filter.movie_id = Some(eq(id));
    } else if let Some(id) = &session.track_id {
        filter.track_id = Some(eq(id));
    } else if let Some(id) = &session.audiobook_id {
        filter.audiobook_id = Some(eq(id));
    } else {
        return None;
    }
    filter.device_id = Some(match &session.device_id {
        Some(device_id) => eq(device_id),
        None => StringFilter {
            is_null: Some(true),
            ..Default::default()
        },
    });
    Some(filter)
}

async fn latest(
    tx: &mut MutationContext<'_>,
    filter: PlayHistoryWhereInput,
) -> async_graphql::Result<Option<PlayHistory>> {
    Ok(tx
        .query::<PlayHistory>()
        .filter(filter)
        .order_by(PlayHistoryOrderByInput {
            ended_at: Some(OrderDirection::Desc),
            ..Default::default()
        })
        .limit(1)
        .fetch_all()
        .await
        .map_err(db_error)?
        .into_iter()
        .next())
}

/// The `PlaybackSession` After hook: credit the time played since the last report.
pub(crate) async fn record_session_change(
    tx: &mut MutationContext<'_>,
    event: &MutationEvent,
) -> async_graphql::Result<()> {
    if event.phase != MutationPhase::After || event.action != ChangeAction::Updated {
        return Ok(());
    }
    let (Some(before), Some(after)) = (
        event.before::<PlaybackSession>()?,
        event.after::<PlaybackSession>()?,
    ) else {
        return Ok(());
    };
    // Internal writes (a removed file, a stop request) are not playback reports.
    if before.current_position == after.current_position
        && before.last_updated_at == after.last_updated_at
    {
        return Ok(());
    }
    let (Some(from), Some(to)) = (checkpoint(before), checkpoint(after)) else {
        return Ok(());
    };
    let credit = credited_seconds(from, to);
    let complete = is_complete(
        after.current_position,
        after.duration,
        after.completed_at.as_deref(),
    );

    let mut play = latest(
        tx,
        PlayHistoryWhereInput {
            playback_session_id: Some(eq(&after.id)),
            ..Default::default()
        },
    )
    .await?
    .filter(|play| parse_time(&play.ended_at).is_some_and(|ended| continues(ended, to.at)));
    if credit <= 0.0 && !(complete && play.is_some()) {
        return Ok(());
    }
    if play.is_none()
        && let Some(mut filter) = same_item(after)
    {
        // Reopened soon after: the same unfinished play carries on.
        filter.completed = Some(BoolFilter {
            eq: Some(false),
            ..Default::default()
        });
        filter.imported = Some(BoolFilter {
            eq: Some(false),
            ..Default::default()
        });
        filter.ended_at = Some(DateFilter {
            gte: Some(timestamp(to.at - Duration::seconds(RESUME_WINDOW_SECS))),
            ..Default::default()
        });
        play = latest(tx, filter).await?;
    }

    let ended_at = timestamp(to.at);
    match play {
        Some(play) => {
            tx.update_by_id::<PlayHistory>(
                &play.id,
                UpdatePlayHistoryInput {
                    playback_session_id: Some(Some(after.id.clone())),
                    seconds_played: Some(play.seconds_played + credit),
                    ended_at: (credit > 0.0).then(|| ended_at.clone()),
                    end_position: Some(after.current_position),
                    duration: Some(after.duration.or(play.duration)),
                    completed: Some(play.completed || complete),
                    play_method: Some(after.play_method.clone().or(play.play_method)),
                    ..Default::default()
                },
            )
            .await
            .map_err(db_error)?;
        }
        None => {
            let Some(snapshot) = resolve_snapshot(
                tx,
                after.content_type.as_deref().map(|kind| match kind {
                    "chapter" => "audiobook",
                    other => other,
                }),
                SnapshotIds {
                    media_file: after.media_file_id.as_deref(),
                    movie: after.movie_id.as_deref(),
                    episode: after.episode_id.as_deref(),
                    track: after.track_id.as_deref(),
                    audiobook: after.audiobook_id.as_deref(),
                    chapter: after.chapter_id.as_deref(),
                },
            )
            .await?
            else {
                return Ok(());
            };
            let device_name = match &after.device_id {
                Some(id) => by_id!(tx, Device, DeviceWhereInput, id).map(|device| device.name),
                None => None,
            };
            tx.insert::<PlayHistory>(CreatePlayHistoryInput {
                user_id: after.user_id.clone(),
                device_id: after.device_id.clone(),
                device_name,
                playback_session_id: Some(after.id.clone()),
                content_type: snapshot.content_type,
                media_file_id: snapshot.media_file_id,
                movie_id: snapshot.movie_id,
                episode_id: snapshot.episode_id,
                show_id: snapshot.show_id,
                track_id: snapshot.track_id,
                album_id: snapshot.album_id,
                artist_id: snapshot.artist_id,
                audiobook_id: snapshot.audiobook_id,
                chapter_id: snapshot.chapter_id,
                title: snapshot.title,
                parent_title: snapshot.parent_title,
                subtitle: snapshot.subtitle,
                started_at: timestamp(from.at),
                ended_at,
                seconds_played: credit,
                start_position: before.current_position,
                end_position: after.current_position,
                duration: after.duration,
                completed: complete,
                play_method: after.play_method.clone(),
                imported: false,
                source_progress_id: None,
            })
            .await
            .map_err(db_error)?;
        }
    }
    Ok(())
}

fn forbidden(message: &str) -> async_graphql::Error {
    async_graphql::Error::new(message)
        .extend_with(|_, extensions| extensions.set("code", "FORBIDDEN"))
}

fn invalid(message: &str) -> async_graphql::Error {
    async_graphql::Error::new(message)
        .extend_with(|_, extensions| extensions.set("code", "BAD_USER_INPUT"))
}

/// A create through the public GraphQL API (the before-create transform, request context only).
pub(crate) async fn prepare_public_create(
    db: &Database,
    entity_name: &str,
    input: &mut (dyn std::any::Any + Send + Sync),
) -> async_graphql::Result<()> {
    match entity_name {
        "PlayHistory" => Err(forbidden(
            "Play history is recorded by playback and can only be read or deleted",
        )),
        "Device" => Err(forbidden(
            "Devices are added when a client checks in (checkInDevice)",
        )),
        "StatsShare" => {
            let Some(input) = input.downcast_mut::<CreateStatsShareInput>() else {
                return Err(forbidden("Invalid share"));
            };
            validate_share(db, input).await
        }
        "PlaybackSession" => {
            // Server time, so sessions from clients with wrong clocks compare and stale out.
            if let Some(input) = input.downcast_mut::<CreatePlaybackSessionInput>() {
                let now = now();
                input.started_at = now.clone();
                input.last_updated_at = now;
                input.ended_at = None;
                input.stop_requested_at = None;
                if input.state.is_none() {
                    input.state = Some(
                        if input.is_playing {
                            "playing"
                        } else {
                            "paused"
                        }
                        .into(),
                    );
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// An update through the public GraphQL API.
pub(crate) fn prepare_public_update(
    entity_name: &str,
    input: &mut (dyn std::any::Any + Send + Sync),
) -> async_graphql::Result<()> {
    match entity_name {
        "PlayHistory" => Err(forbidden(
            "Play history is recorded by playback and can only be read or deleted",
        )),
        "StatsShare" => Err(forbidden(
            "A share cannot be changed; remove it and share again",
        )),
        "Device" => {
            let Some(input) = input.downcast_mut::<UpdateDeviceInput>() else {
                return Err(forbidden("Invalid device update"));
            };
            let reported = input.client_id.is_some()
                || input.name_is_custom.is_some()
                || input.form_factor.is_some()
                || input.platform.is_some()
                || input.product.is_some()
                || input.app_version.is_some()
                || input.user_agent.is_some()
                || input.last_ip.is_some()
                || input.last_seen_at.is_some()
                || input.session_id.is_some()
                || input.signed_out_at.is_some();
            if reported {
                return Err(forbidden(
                    "Only a device's name can be changed; the rest is what it reported",
                ));
            }
            if let Some(name) = &mut input.name {
                let trimmed = name.trim();
                if trimmed.is_empty() || trimmed.chars().count() > 80 {
                    return Err(invalid("A device name needs 1 to 80 characters"));
                }
                *name = trimmed.to_owned();
                input.name_is_custom = Some(true);
            }
            Ok(())
        }
        "PlaybackSession" => {
            if let Some(input) = input.downcast_mut::<UpdatePlaybackSessionInput>() {
                if input.last_updated_at.is_some() {
                    input.last_updated_at = Some(now());
                }
                if matches!(input.stop_requested_at, Some(Some(_))) {
                    input.stop_requested_at = Some(Some(now()));
                }
                if matches!(input.ended_at, Some(Some(_))) {
                    input.ended_at = Some(Some(now()));
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

pub const AUDIENCE_EVERYONE: &str = "everyone";
pub const AUDIENCE_USER: &str = "user";

/// A share names everyone, or one other active user, once.
async fn validate_share(
    db: &Database,
    input: &mut CreateStatsShareInput,
) -> async_graphql::Result<()> {
    match input.audience.as_str() {
        AUDIENCE_EVERYONE => input.viewer_user_id = None,
        AUDIENCE_USER => {
            let Some(viewer) = input.viewer_user_id.as_deref() else {
                return Err(invalid("Choose who to share with"));
            };
            if viewer == input.user_id {
                return Err(invalid("Your statistics are always visible to you"));
            }
            let active = User::get(db.pool(), &viewer.to_owned())
                .await
                .map_err(db_error)?
                .is_some_and(|user| user.is_active);
            if !active {
                return Err(invalid("That user is not on this server"));
            }
        }
        _ => return Err(invalid("A share is with everyone or with one user")),
    }
    let existing = StatsShare::query(db.pool())
        .filter(StatsShareWhereInput {
            user_id: Some(eq(&input.user_id)),
            audience: Some(eq(&input.audience)),
            viewer_user_id: Some(match &input.viewer_user_id {
                Some(viewer) => eq(viewer),
                None => StringFilter {
                    is_null: Some(true),
                    ..Default::default()
                },
            }),
            ..Default::default()
        })
        .limit(1)
        .fetch_all()
        .await
        .map_err(db_error)?;
    if !existing.is_empty() {
        return Err(invalid("Already shared"));
    }
    Ok(())
}

/// A user's plays, devices and shares (both ways) go with the user.
pub(crate) async fn cleanup_deleted_user(
    tx: &mut MutationContext<'_>,
    user_id: &str,
) -> async_graphql::Result<()> {
    tx.delete_where::<PlayHistory>(PlayHistoryWhereInput {
        user_id: Some(eq(user_id)),
        ..Default::default()
    })
    .await
    .map_err(db_error)?;
    tx.delete_where::<StatsShare>(StatsShareWhereInput {
        user_id: Some(eq(user_id)),
        ..Default::default()
    })
    .await
    .map_err(db_error)?;
    tx.delete_where::<StatsShare>(StatsShareWhereInput {
        viewer_user_id: Some(eq(user_id)),
        ..Default::default()
    })
    .await
    .map_err(db_error)?;
    tx.update_where::<PlaybackSession>(
        PlaybackSessionWhereInput {
            user_id: Some(eq(user_id)),
            device_id: Some(StringFilter {
                is_null: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        },
        UpdatePlaybackSessionInput {
            device_id: Some(None),
            ..Default::default()
        },
    )
    .await
    .map_err(db_error)?;
    tx.delete_where::<Device>(DeviceWhereInput {
        user_id: Some(eq(user_id)),
        ..Default::default()
    })
    .await
    .map_err(db_error)?;
    Ok(())
}

/// What the import did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportOutcome {
    pub imported: u64,
    pub skipped: u64,
}

/// Seed history from watch progress: one imported play per watched checkpoint, completed,
/// with no time claimed. Runs once per checkpoint (`sourceProgressId`), so it can be re-run.
/// `user_id` limits it to one person; `None` imports everyone's (the startup run).
pub async fn import_from_progress(
    db: &Database,
    user_id: Option<&str>,
) -> anyhow::Result<ImportOutcome> {
    const PAGE: i64 = 100;
    let mut outcome = ImportOutcome::default();
    let mut offset = 0_i64;
    loop {
        let page = PlaybackProgress::query(db.pool())
            .filter(PlaybackProgressWhereInput {
                user_id: user_id.map(eq),
                is_watched: Some(BoolFilter {
                    eq: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .order_by(PlaybackProgressOrderByInput {
                created_at: Some(OrderDirection::Asc),
                ..Default::default()
            })
            .limit(PAGE)
            .offset(offset)
            .fetch_all()
            .await?;
        let page_len = page.len();
        let mut tx = MutationContext::begin(db, TransactionMode::Default).await?;
        for progress in page {
            let already = tx
                .query::<PlayHistory>()
                .filter(PlayHistoryWhereInput {
                    source_progress_id: Some(eq(&progress.id)),
                    ..Default::default()
                })
                .limit(1)
                .fetch_all()
                .await?;
            let Some(file_id) = progress.media_file_id.as_deref() else {
                outcome.skipped += 1;
                continue;
            };
            if !already.is_empty() {
                outcome.skipped += 1;
                continue;
            }
            let file = tx
                .query::<MediaFile>()
                .filter(MediaFileWhereInput {
                    id: Some(eq(file_id)),
                    ..Default::default()
                })
                .fetch_one()
                .await?;
            let Some(file) = file else {
                outcome.skipped += 1;
                continue;
            };
            let audiobook_id = match &file.chapter_id {
                Some(chapter_id) => tx
                    .query::<Chapter>()
                    .filter(ChapterWhereInput {
                        id: Some(eq(chapter_id)),
                        ..Default::default()
                    })
                    .fetch_one()
                    .await?
                    .map(|chapter| chapter.audiobook_id),
                None => None,
            };
            let snapshot = resolve_snapshot(
                &mut tx,
                None,
                SnapshotIds {
                    media_file: Some(file_id),
                    movie: file.movie_id.as_deref(),
                    episode: file.episode_id.as_deref(),
                    track: file.track_id.as_deref(),
                    audiobook: audiobook_id.as_deref(),
                    chapter: file.chapter_id.as_deref(),
                },
            )
            .await
            .map_err(|error| anyhow::anyhow!(error.message))?;
            let Some(snapshot) = snapshot else {
                outcome.skipped += 1;
                continue;
            };
            let at = progress
                .watched_at
                .as_deref()
                .and_then(parse_time)
                .or_else(|| parse_time(&progress.updated_at))
                .map(timestamp)
                .unwrap_or_else(now);
            tx.insert::<PlayHistory>(CreatePlayHistoryInput {
                user_id: progress.user_id.clone(),
                device_id: None,
                device_name: None,
                playback_session_id: None,
                content_type: snapshot.content_type,
                media_file_id: snapshot.media_file_id,
                movie_id: snapshot.movie_id,
                episode_id: snapshot.episode_id,
                show_id: snapshot.show_id,
                track_id: snapshot.track_id,
                album_id: snapshot.album_id,
                artist_id: snapshot.artist_id,
                audiobook_id: snapshot.audiobook_id,
                chapter_id: snapshot.chapter_id,
                title: snapshot.title,
                parent_title: snapshot.parent_title,
                subtitle: snapshot.subtitle,
                started_at: at.clone(),
                ended_at: at,
                seconds_played: 0.0,
                start_position: 0.0,
                end_position: progress.current_position,
                duration: progress.duration,
                completed: true,
                play_method: None,
                imported: true,
                source_progress_id: Some(progress.id.clone()),
            })
            .await?;
            outcome.imported += 1;
        }
        tx.commit_and_emit().await?;
        if page_len < PAGE as usize {
            break;
        }
        offset += PAGE;
    }
    Ok(outcome)
}

/// Delete plays that started before the retention period. `days == 0` keeps everything.
pub async fn prune(db: &Database, days: i64, at: DateTime<Utc>) -> anyhow::Result<i64> {
    if days <= 0 {
        return Ok(0);
    }
    let cutoff = timestamp(at - Duration::days(days));
    let mut tx = MutationContext::begin(db, TransactionMode::Default).await?;
    let removed = tx
        .delete_where::<PlayHistory>(PlayHistoryWhereInput {
            started_at: Some(DateFilter {
                lt: Some(cutoff),
                ..Default::default()
            }),
            ..Default::default()
        })
        .await?;
    tx.commit_and_emit().await?;
    Ok(removed)
}

/// The retention setting's value in days (0 when unset or not a number).
pub fn retention_days(value: Option<&str>) -> i64 {
    value
        .map(|value| value.trim().trim_matches('"'))
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|days| days.is_finite() && *days > 0.0)
        .map(|days| days.floor() as i64)
        .unwrap_or(0)
}

/// Display names for user ids: the display name, else the username.
pub async fn user_names(db: &Database) -> anyhow::Result<HashMap<String, (String, bool)>> {
    let mut names = HashMap::new();
    let mut offset = 0_i64;
    loop {
        let page = User::query(db.pool())
            .limit(100)
            .offset(offset)
            .fetch_all()
            .await?;
        let len = page.len();
        for user in page {
            let name = user
                .display_name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or(user.username);
            names.insert(user.id, (name, user.is_active));
        }
        if len < 100 {
            return Ok(names);
        }
        offset += 100;
    }
}
