//! Watch and listening statistics, a computed read over play history (design.md Q135–Q136).
//!
//! Generated roots cannot group plays by local day, show, artist or genre, so this query reads
//! the authorized person's plays through the typed entity layer and sums them in Rust
//! (`play_history::stats`). Authorization comes first: your own, an administrator's view, or a
//! share (`play_history::access`). Queries never write.

use std::collections::{HashMap, HashSet};

use async_graphql::{Context, ErrorExtensions, Object, Result, SimpleObject};
use chrono::{DateTime, Duration, Utc};
use graphql_orm::graphql::filters::{DateFilter, StringFilter};
use graphql_orm::graphql::orm::OrderDirection;

use crate::db::Database;
use crate::graphql::auth::AuthExt;
use crate::graphql::entities::*;
use crate::services::play_history::stats::{
    Bucket, DeviceUsage, OriginUsage, Period, Streak, TopItem, Totals, UserUsage, aggregate,
};
use crate::services::play_history::{access, parse_time, timestamp, user_names};

/// Plays read for one answer; a longer period says it is incomplete.
const MAX_SCANNED: usize = 50_000;
const PAGE: i64 = 100;
const MAX_TOP: i32 = 50;
const RECENT: usize = 20;

fn bad_input(message: impl Into<String>) -> async_graphql::Error {
    async_graphql::Error::new(message.into())
        .extend_with(|_, extensions| extensions.set("code", "BAD_USER_INPUT"))
}

fn not_found() -> async_graphql::Error {
    // The same answer for "no such user" and "not shared with you".
    async_graphql::Error::new("Statistics not found or not shared with you")
        .extend_with(|_, extensions| extensions.set("code", "NOT_FOUND"))
}

fn internal(error: impl std::fmt::Display) -> async_graphql::Error {
    async_graphql::Error::new(error.to_string())
}

/// One play as statistics show it: no device address, user agent or session.
#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "PlayStatisticsPlay", rename_fields = "camelCase")]
pub struct RecentPlay {
    pub id: String,
    pub content_type: String,
    pub title: String,
    pub parent_title: Option<String>,
    pub subtitle: Option<String>,
    pub movie_id: Option<String>,
    pub episode_id: Option<String>,
    pub show_id: Option<String>,
    pub track_id: Option<String>,
    pub album_id: Option<String>,
    pub artist_id: Option<String>,
    pub audiobook_id: Option<String>,
    pub started_at: String,
    pub ended_at: String,
    pub seconds_played: f64,
    pub completed: bool,
    pub imported: bool,
    pub device_name: Option<String>,
    pub play_method: Option<String>,
}

impl From<&PlayHistory> for RecentPlay {
    fn from(row: &PlayHistory) -> Self {
        Self {
            id: row.id.clone(),
            content_type: row.content_type.clone(),
            title: row.title.clone(),
            parent_title: row.parent_title.clone(),
            subtitle: row.subtitle.clone(),
            movie_id: row.movie_id.clone(),
            episode_id: row.episode_id.clone(),
            show_id: row.show_id.clone(),
            track_id: row.track_id.clone(),
            album_id: row.album_id.clone(),
            artist_id: row.artist_id.clone(),
            audiobook_id: row.audiobook_id.clone(),
            started_at: row.started_at.clone(),
            ended_at: row.ended_at.clone(),
            seconds_played: row.seconds_played,
            completed: row.completed,
            imported: row.imported,
            device_name: row.device_name.clone(),
            play_method: row.play_method.clone(),
        }
    }
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "PlayStatistics", rename_fields = "camelCase")]
pub struct PlayStatistics {
    /// Whose statistics; null for an administrator's everyone view.
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub from: String,
    pub to: String,
    pub totals: Totals,
    pub streak: Streak,
    /// One per local day of the period (periods over 400 days have weeks only).
    pub days: Vec<Bucket>,
    pub weeks: Vec<Bucket>,
    /// Seconds by local hour, 0–23.
    pub hours: Vec<f64>,
    pub top_shows: Vec<TopItem>,
    pub top_movies: Vec<TopItem>,
    pub top_artists: Vec<TopItem>,
    pub top_albums: Vec<TopItem>,
    pub top_audiobooks: Vec<TopItem>,
    pub top_authors: Vec<TopItem>,
    pub top_genres: Vec<TopItem>,
    pub devices: Vec<DeviceUsage>,
    /// Per person; only in an administrator's everyone view.
    pub users: Vec<UserUsage>,
    /// Per origin: this server, and later the servers shared libraries are on. One entry
    /// (this server) until sharing exists, so clients show an origin choice only past one.
    pub origins: Vec<OriginUsage>,
    pub recent: Vec<RecentPlay>,
    /// False when the period held more plays than one answer reads.
    pub complete: bool,
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "StatisticsPerson", rename_fields = "camelCase")]
pub struct StatisticsPerson {
    pub user_id: String,
    pub name: String,
    pub is_self: bool,
    /// Their statistics reach you through a share.
    pub shared: bool,
}

#[derive(Default)]
pub struct StatisticsQueries;

async fn read_plays(
    db: &Database,
    user_id: Option<&str>,
    from: &str,
    to: &str,
) -> anyhow::Result<(Vec<PlayHistory>, bool)> {
    let mut rows = Vec::new();
    loop {
        let page = PlayHistory::query(db.pool())
            .filter(PlayHistoryWhereInput {
                user_id: user_id.map(|id| StringFilter {
                    eq: Some(id.to_owned()),
                    ..Default::default()
                }),
                started_at: Some(DateFilter {
                    gte: Some(from.to_owned()),
                    lte: Some(to.to_owned()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .order_by(PlayHistoryOrderByInput {
                started_at: Some(OrderDirection::Asc),
                ..Default::default()
            })
            .limit(PAGE)
            .offset(rows.len() as i64)
            .fetch_all()
            .await?;
        let len = page.len();
        rows.extend(page);
        if len < PAGE as usize {
            return Ok((rows, true));
        }
        if rows.len() >= MAX_SCANNED {
            return Ok((rows, false));
        }
    }
}

/// Genres of the movies, shows and albums in `rows`, by id.
async fn genres(
    db: &Database,
    rows: &[PlayHistory],
) -> anyhow::Result<HashMap<String, Vec<String>>> {
    let collect = |pick: fn(&PlayHistory) -> Option<&String>| -> Vec<String> {
        rows.iter()
            .filter_map(pick)
            .cloned()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    };
    let in_list = |ids: &[String]| StringFilter {
        in_list: Some(ids.to_vec()),
        ..Default::default()
    };
    let mut map = HashMap::new();
    for ids in collect(|row| row.movie_id.as_ref()).chunks(PAGE as usize) {
        for movie in Movie::query(db.pool())
            .filter(MovieWhereInput {
                id: Some(in_list(ids)),
                ..Default::default()
            })
            .limit(PAGE)
            .fetch_all()
            .await?
        {
            map.insert(movie.id, movie.genres);
        }
    }
    for ids in collect(|row| row.show_id.as_ref()).chunks(PAGE as usize) {
        for show in Show::query(db.pool())
            .filter(ShowWhereInput {
                id: Some(in_list(ids)),
                ..Default::default()
            })
            .limit(PAGE)
            .fetch_all()
            .await?
        {
            map.insert(show.id, show.genres);
        }
    }
    for ids in collect(|row| row.album_id.as_ref()).chunks(PAGE as usize) {
        for album in Album::query(db.pool())
            .filter(AlbumWhereInput {
                id: Some(in_list(ids)),
                ..Default::default()
            })
            .limit(PAGE)
            .fetch_all()
            .await?
        {
            map.insert(album.id, album.genres);
        }
    }
    Ok(map)
}

fn parse_bound(value: Option<&str>, name: &str) -> Result<Option<DateTime<Utc>>> {
    value
        .map(|value| {
            parse_time(value)
                .ok_or_else(|| bad_input(format!("{name} must be an ISO 8601 date and time")))
        })
        .transpose()
}

#[Object]
impl StatisticsQueries {
    /// Totals, days, weeks, top titles, genres, streaks and devices for one person (yours by
    /// default, or someone's who shares them with you), or for everyone (administrators,
    /// `allUsers`). `from`/`to` default to the last 30 days; `utcOffsetMinutes` places local
    /// days (east of UTC is positive, so UTC+10 is 600).
    #[graphql(name = "playStatistics")]
    #[allow(clippy::too_many_arguments)]
    async fn play_statistics(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "userId")] user_id: Option<String>,
        #[graphql(name = "allUsers", default = false)] all_users: bool,
        from: Option<String>,
        to: Option<String>,
        #[graphql(name = "utcOffsetMinutes", default = 0)] utc_offset_minutes: i32,
        #[graphql(default = 10)] top: i32,
        // Only these plays: `local` for this server, or another origin's id.
        origin: Option<String>,
    ) -> Result<PlayStatistics> {
        let viewer = ctx.require_member()?;
        if !(1..=MAX_TOP).contains(&top) {
            return Err(bad_input(format!("top must be 1-{MAX_TOP}")));
        }
        if !(-14 * 60..=14 * 60).contains(&utc_offset_minutes) {
            return Err(bad_input("utcOffsetMinutes must be within ±14 hours"));
        }
        let now = Utc::now();
        let to = parse_bound(to.as_deref(), "to")?.unwrap_or(now);
        let from = parse_bound(from.as_deref(), "from")?.unwrap_or(to - Duration::days(30));
        if from > to {
            return Err(bad_input("from must be before to"));
        }
        if to - from > Duration::days(3660) {
            return Err(bad_input("A period is at most ten years"));
        }
        let db = ctx.data::<Database>()?;
        let target = if all_users {
            if !viewer.is_admin() {
                return Err(async_graphql::Error::new(
                    "Everyone's statistics need an administrator",
                )
                .extend_with(|_, extensions| extensions.set("code", "FORBIDDEN")));
            }
            None
        } else {
            let target = user_id.unwrap_or_else(|| viewer.user_id.clone());
            if !access::can_view(db, viewer, &target)
                .await
                .map_err(internal)?
            {
                return Err(not_found());
            }
            Some(target)
        };
        let (from_text, to_text) = (timestamp(from), timestamp(to));
        let (mut rows, complete) = read_plays(db, target.as_deref(), &from_text, &to_text)
            .await
            .map_err(internal)?;
        if let Some(origin) = origin.as_deref() {
            rows.retain(|row| match origin {
                "local" => row.origin.is_none(),
                other => row.origin.as_deref() == Some(other),
            });
        }
        let genres = genres(db, &rows).await.map_err(internal)?;
        let names: HashMap<String, String> = user_names(db)
            .await
            .map_err(internal)?
            .into_iter()
            .map(|(id, (name, _))| (id, name))
            .collect();
        let summed = aggregate(
            &rows,
            Period {
                from,
                to,
                utc_offset_minutes,
                now,
            },
            &genres,
            &names,
            top as usize,
        );
        let recent = rows
            .iter()
            .rev()
            .take(RECENT)
            .map(RecentPlay::from)
            .collect();
        Ok(PlayStatistics {
            user_name: target.as_ref().and_then(|id| names.get(id).cloned()),
            user_id: target.clone(),
            from: from_text,
            to: to_text,
            totals: summed.totals,
            streak: summed.streak,
            days: summed.days,
            weeks: summed.weeks,
            hours: summed.hours,
            top_shows: summed.top_shows,
            top_movies: summed.top_movies,
            top_artists: summed.top_artists,
            top_albums: summed.top_albums,
            top_audiobooks: summed.top_audiobooks,
            top_authors: summed.top_authors,
            top_genres: summed.top_genres,
            devices: summed.devices,
            users: if target.is_none() {
                summed.users
            } else {
                Vec::new()
            },
            origins: summed.origins,
            recent,
            complete,
        })
    }

    /// Whose statistics you can open: yourself, people sharing with you, and (administrators)
    /// everyone.
    #[graphql(name = "statisticsPeople")]
    async fn statistics_people(&self, ctx: &Context<'_>) -> Result<Vec<StatisticsPerson>> {
        let viewer = ctx.require_member()?;
        let db = ctx.data::<Database>()?;
        Ok(access::viewable_users(db, viewer)
            .await
            .map_err(internal)?
            .into_iter()
            .map(|person| StatisticsPerson {
                user_id: person.user_id,
                name: person.name,
                is_self: person.is_self,
                shared: person.shared,
            })
            .collect())
    }

    /// The other people on this server, to share your statistics with.
    #[graphql(name = "statisticsShareCandidates")]
    async fn statistics_share_candidates(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<StatisticsPerson>> {
        let viewer = ctx.require_member()?;
        let db = ctx.data::<Database>()?;
        Ok(access::share_candidates(db, viewer)
            .await
            .map_err(internal)?
            .into_iter()
            .map(|(user_id, name)| StatisticsPerson {
                user_id,
                name,
                is_self: false,
                shared: false,
            })
            .collect())
    }
}
