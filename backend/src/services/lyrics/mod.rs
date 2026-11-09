//! Lyrics for music tracks: from an `.lrc` beside the file, the file's own tags, or LRCLIB,
//! cached as one `Lyric` row per file. See `docs/design.md` Q121.
//!
//! The order is what is most likely right and best to show: synced lyrics from this server
//! (a sidecar, then tags), then LRCLIB's synced lyrics, then plain lyrics from this server, then
//! LRCLIB's plain ones. A track nothing has lyrics for is remembered as `none` and not asked
//! again for [`MISS_RETRY_DAYS`] unless someone asks for a fresh look.

pub mod embedded;
pub mod lrc;
pub mod lrclib;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use graphql_orm::graphql::filters::StringFilter;
use serde_json::json;

use crate::db::Database;
use crate::services::graphql::LibrarianSchema;
use crate::services::graphql::entities::{Album, Artist, Lyric, LyricWhereInput, MediaFile, Track};
use crate::services::library_storage::{StoragePath, library_storage};
use crate::services::torrent::database::{execute_mutation, get_setting_string, system_auth_user};

use lrclib::{LrclibClient, LrclibRecord, LyricsQuery};

/// How long a miss is remembered.
pub const MISS_RETRY_DAYS: i64 = 30;
/// The largest `.lrc` read.
const MAX_LRC_BYTES: u64 = 1024 * 1024;
/// Whether LRCLIB is asked (app setting, default on).
pub const LRCLIB_ENABLED_SETTING: &str = "lyrics.lrclib_enabled";

/// What was found, before it is stored.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FoundLyrics {
    pub source: &'static str,
    pub synced: Option<String>,
    pub plain: Option<String>,
    pub language: Option<String>,
    pub instrumental: bool,
    pub provider_id: Option<String>,
}

/// The `.lrc` beside an audio file: same name, `.lrc` or `.LRC`.
pub fn sidecar_candidates(audio_path: &str) -> Vec<StoragePath> {
    let path = StoragePath::new(audio_path);
    let (Some(folder), Some(name)) = (path.parent(), path.file_name()) else {
        return Vec::new();
    };
    let stem = crate::services::subtitles::sidecar::file_stem(&name);
    vec![
        folder.join(&format!("{stem}.lrc")),
        folder.join(&format!("{stem}.LRC")),
    ]
}

async fn read_sidecar(audio_path: &str) -> Option<FoundLyrics> {
    for candidate in sidecar_candidates(audio_path) {
        let Ok(meta) = library_storage().stat(&candidate).await else {
            continue;
        };
        if !meta.is_file() || meta.size > MAX_LRC_BYTES {
            continue;
        }
        let Ok(bytes) = library_storage().read_all(&candidate).await else {
            continue;
        };
        let text = crate::services::subtitles::decode_text(&bytes);
        let parsed = lrc::parse(&text);
        let found = if parsed.lines.len() >= 2 {
            FoundLyrics {
                source: "sidecar",
                plain: Some(lrc::plain_from_lines(&parsed.lines)).filter(|p| !p.is_empty()),
                synced: Some(lrc::to_lrc(&parsed.lines)),
                language: parsed.language,
                ..Default::default()
            }
        } else {
            FoundLyrics {
                source: "sidecar",
                plain: lrc::normalise_plain(&text),
                ..Default::default()
            }
        };
        if found.synced.is_some() || found.plain.is_some() {
            return Some(found);
        }
    }
    None
}

/// Choose among this server's lyrics and LRCLIB's.
pub fn choose(local: Option<FoundLyrics>, remote: Option<&LrclibRecord>) -> FoundLyrics {
    let remote_found = remote.map(|record| FoundLyrics {
        source: "lrclib",
        synced: record.synced_lyrics.as_deref().and_then(lrc::normalise),
        plain: record
            .plain_lyrics
            .as_deref()
            .and_then(lrc::normalise_plain),
        language: None,
        instrumental: record.instrumental,
        provider_id: Some(record.id.to_string()),
    });
    if let Some(local) = local.as_ref().filter(|local| local.synced.is_some()) {
        return local.clone();
    }
    if let Some(remote) = remote_found
        .as_ref()
        .filter(|remote| remote.synced.is_some())
    {
        let mut remote = remote.clone();
        // Words this server has are kept beside LRCLIB's times.
        if let Some(plain) = local.as_ref().and_then(|local| local.plain.clone()) {
            remote.plain = Some(plain);
        }
        return remote;
    }
    if let Some(local) = local.filter(|local| local.plain.is_some()) {
        return local;
    }
    if let Some(remote) =
        remote_found.filter(|remote| remote.plain.is_some() || remote.instrumental)
    {
        return remote;
    }
    FoundLyrics {
        source: "none",
        ..Default::default()
    }
}

/// Whether a stored row is still the answer.
pub fn is_fresh(row: &Lyric, now: DateTime<Utc>) -> bool {
    if row.source != "none" {
        return true;
    }
    DateTime::parse_from_rfc3339(&row.fetched_at)
        .map(|at| now.signed_duration_since(at.with_timezone(&Utc)).num_days() < MISS_RETRY_DAYS)
        .unwrap_or(false)
}

async fn stored(db: &Database, media_file_id: &str) -> Result<Option<Lyric>> {
    Ok(Lyric::query(db.pool())
        .filter(LyricWhereInput {
            media_file_id: Some(StringFilter {
                eq: Some(media_file_id.to_owned()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?
        .into_iter()
        .next())
}

async fn track_query(db: &Database, track: &Track, file: &MediaFile) -> Result<LyricsQuery> {
    let album = Album::get(db.pool(), &track.album_id).await?;
    let album_artist = match album.as_ref() {
        Some(album) => Artist::get(db.pool(), &album.artist_id)
            .await?
            .map(|artist| artist.name),
        None => None,
    };
    let artist = track
        .artist_name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .or(album_artist)
        .unwrap_or_default();
    Ok(LyricsQuery {
        artist,
        title: track.title.clone(),
        album: album.map(|album| album.name),
        duration_secs: track
            .duration_secs
            .or(file.duration)
            .map(i64::from)
            .filter(|seconds| *seconds > 0),
    })
}

/// The lyrics of a track's file: the stored row when it is still good, else a fresh look that is
/// stored. `refresh` looks again whatever is stored.
pub async fn ensure(
    db: &Database,
    schema: &LibrarianSchema,
    track: &Track,
    file: &MediaFile,
    refresh: bool,
) -> Result<Lyric> {
    let existing = stored(db, &file.id).await?;
    if let Some(row) = existing.as_ref()
        && !refresh
        && is_fresh(row, Utc::now())
    {
        return Ok(row.clone());
    }

    let mut local = read_sidecar(&file.path).await;
    if local.as_ref().is_none_or(|found| found.synced.is_none()) {
        match embedded::read(&file.path).await {
            Ok(Some(tags)) => {
                let tagged = FoundLyrics {
                    source: "embedded",
                    synced: tags.synced,
                    plain: tags.plain,
                    language: tags.language,
                    ..Default::default()
                };
                // A sidecar's plain words lose to tags' synced ones, and win over tags' plain.
                local = match local {
                    Some(sidecar) if tagged.synced.is_none() => Some(sidecar),
                    _ => Some(tagged),
                };
            }
            Ok(None) => {}
            Err(error) => {
                tracing::debug!(media_file_id = %file.id, %error, "Could not read lyrics from tags");
            }
        }
    }

    let mut remote = None;
    let lrclib_enabled = get_setting_string(db, LRCLIB_ENABLED_SETTING)
        .await
        .ok()
        .flatten()
        .is_none_or(|value| value.trim().trim_matches('"') != "false");
    if lrclib_enabled && local.as_ref().is_none_or(|found| found.synced.is_none()) {
        let query = track_query(db, track, file).await?;
        if !query.artist.is_empty() && !query.title.is_empty() {
            let client = LrclibClient::new(lrclib::DEFAULT_BASE_URL)?;
            match client.find(&query).await {
                Ok(record) => remote = record,
                Err(error) => {
                    // A provider that is down is not a miss: keep what was stored, try next time.
                    tracing::warn!(track_id = %track.id, %error, "LRCLIB lookup failed");
                    if let Some(row) = existing.as_ref().filter(|row| row.source != "none") {
                        return Ok(row.clone());
                    }
                    if local.is_none() {
                        anyhow::bail!("LRCLIB could not be reached. Try again later.");
                    }
                }
            }
        }
    }

    let found = choose(local, remote.as_ref());
    save(db, schema, existing.as_ref(), track, file, &found).await
}

async fn save(
    db: &Database,
    schema: &LibrarianSchema,
    existing: Option<&Lyric>,
    track: &Track,
    file: &MediaFile,
    found: &FoundLyrics,
) -> Result<Lyric> {
    let fields = json!({
        "trackId": track.id,
        "source": found.source,
        "synced": found.synced.is_some(),
        "syncedLyrics": found.synced,
        "plainLyrics": found.plain,
        "language": found.language,
        "instrumental": found.instrumental,
        "providerId": found.provider_id,
        "fetchedAt": Utc::now().to_rfc3339(),
    });
    let auth = system_auth_user();
    match existing {
        Some(row) => {
            execute_mutation(
                schema,
                &auth,
                r#"mutation SaveLyric($id: String!, $input: UpdateLyricInput!) {
                    UpdateLyric: updateLyric(id: $id, input: $input) { success error }
                }"#,
                json!({ "id": row.id, "input": fields }),
            )
            .await?;
        }
        None => {
            let mut input = fields;
            input["mediaFileId"] = json!(file.id);
            execute_mutation(
                schema,
                &auth,
                r#"mutation CreateLyric($input: CreateLyricInput!) {
                    CreateLyric: createLyric(input: $input) { success error }
                }"#,
                json!({ "input": input }),
            )
            .await?;
        }
    }
    stored(db, &file.id)
        .await?
        .context("The lyrics were saved but could not be read back")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(synced: Option<&str>, plain: Option<&str>) -> LrclibRecord {
        LrclibRecord {
            id: 7,
            track_name: "t".into(),
            artist_name: "a".into(),
            album_name: None,
            duration: Some(100.0),
            instrumental: false,
            plain_lyrics: plain.map(str::to_owned),
            synced_lyrics: synced.map(str::to_owned),
        }
    }

    #[test]
    fn synced_beats_plain_and_local_beats_remote() {
        let local_plain = FoundLyrics {
            source: "embedded",
            plain: Some("words".into()),
            ..Default::default()
        };
        let local_synced = FoundLyrics {
            source: "sidecar",
            synced: Some("[00:01.00]a\n[00:02.00]b\n".into()),
            ..Default::default()
        };
        let remote_synced = record(Some("[00:01.00]x\n[00:02.00]y"), Some("x\ny"));
        let remote_plain = record(None, Some("x\ny"));

        assert_eq!(
            choose(Some(local_synced.clone()), Some(&remote_synced)).source,
            "sidecar"
        );
        let mixed = choose(Some(local_plain.clone()), Some(&remote_synced));
        assert_eq!(mixed.source, "lrclib");
        assert_eq!(mixed.plain.as_deref(), Some("words"));
        assert_eq!(mixed.provider_id.as_deref(), Some("7"));
        assert_eq!(
            choose(Some(local_plain.clone()), Some(&remote_plain)).source,
            "embedded"
        );
        assert_eq!(
            choose(None, Some(&remote_plain)).plain.as_deref(),
            Some("x\ny")
        );
        assert_eq!(choose(None, None).source, "none");
        let mut instrumental = record(None, None);
        instrumental.instrumental = true;
        assert!(choose(None, Some(&instrumental)).instrumental);
    }

    #[test]
    fn sidecar_names_follow_the_audio_file() {
        let local = sidecar_candidates("/music/Artist/Album/01 - Song.flac");
        assert_eq!(local[0].as_str(), "/music/Artist/Album/01 - Song.lrc");
        let share = sidecar_candidates("smb://nas/music/Artist/Album/01 - Song.mp3");
        assert!(
            share[0].as_str().ends_with("/Album/01 - Song.lrc"),
            "{}",
            share[0]
        );
        assert!(share[0].as_str().starts_with("smb://nas/music/"));
    }
}
