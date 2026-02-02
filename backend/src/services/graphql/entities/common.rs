//! Common Enums for Media Entities
//!
//! This module contains shared enums used across multiple media entity types
//! (Show, Movie, Album, Audiobook).

use async_graphql::Enum;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Content status for playable media items (episodes, movies, tracks, chapters)
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
#[graphql(name = "ContentStatus")]
pub enum ContentStatus {
    /// Content is currently being played
    #[graphql(name = "PLAYING")]
    Playing,
    /// Content playback is paused
    #[graphql(name = "PAUSED")]
    Paused,
    /// Content file is available (has media file)
    #[graphql(name = "AVAILABLE")]
    Available,
    /// Content is currently downloading
    #[graphql(name = "DOWNLOADING")]
    Downloading,
    /// Content is wanted but not yet downloaded
    #[graphql(name = "WANTED")]
    Wanted,
    /// Content is missing (no file, not wanted)
    #[graphql(name = "MISSING")]
    Missing,
}

/// Content type for status calculation
pub enum ContentType {
    Episode,
    Movie,
    Track,
    Chapter,
}

/// Calculate the content status for a media item
///
/// Priority order:
/// 1. Playing - active playback session with is_playing=true
/// 2. Paused - active playback session with is_playing=false
/// 3. Available - has media_file
/// 4. Downloading - has pending_file_match linked to downloading torrent
/// 5. Wanted - no media_file, wanted=true
/// 6. Missing - no media_file, wanted=false
pub async fn calculate_content_status(
    pool: &SqlitePool,
    content_type: ContentType,
    content_id: &str,
    user_id: &str,
    media_file_id: Option<&str>,
    wanted: bool,
) -> ContentStatus {
    // 1. Check for active playback session
    let column_name = match content_type {
        ContentType::Episode => "episode_id",
        ContentType::Movie => "movie_id",
        ContentType::Track => "track_id",
        ContentType::Chapter => "audiobook_id", // chapters are part of audiobooks
    };

    // Query for active playback session (not completed)
    let session_query = format!(
        r#"
        SELECT is_playing FROM playback_sessions
        WHERE user_id = ? AND {} = ? AND completed_at IS NULL
        ORDER BY last_updated_at DESC
        LIMIT 1
        "#,
        column_name
    );

    if let Ok(session) = sqlx::query_scalar::<_, bool>(&session_query)
        .bind(user_id)
        .bind(content_id)
        .fetch_optional(pool)
        .await
    {
        if let Some(is_playing) = session {
            if is_playing {
                return ContentStatus::Playing;
            } else {
                return ContentStatus::Paused;
            }
        }
    }

    // 2. Check if has media file (available)
    if media_file_id.is_some() {
        // Check if the media file is linked to a downloading torrent_file
        if let Some(mf_id) = media_file_id {
            let downloading_query = r#"
                SELECT t.state FROM torrent_files tf
                JOIN torrents t ON tf.torrent_id = t.id
                WHERE tf.media_file_id = ? AND LOWER(t.state) = 'downloading'
                LIMIT 1
            "#;

            if let Ok(Some(_)) = sqlx::query_scalar::<_, String>(downloading_query)
                .bind(mf_id)
                .fetch_optional(pool)
                .await
            {
                return ContentStatus::Downloading;
            }
        }
        return ContentStatus::Available;
    }

    // 3. Check for downloading (pending file match linked to downloading torrent)
    let match_column = match content_type {
        ContentType::Episode => "episode_id",
        ContentType::Movie => "movie_id",
        ContentType::Track => "track_id",
        ContentType::Chapter => "chapter_id",
    };

    let downloading_match_query = format!(
        r#"
        SELECT t.state FROM pending_file_matches pfm
        JOIN torrents t ON pfm.source_id = t.info_hash
        WHERE pfm.{} = ? AND pfm.source_type = 'torrent' AND LOWER(t.state) = 'downloading'
        LIMIT 1
        "#,
        match_column
    );

    if let Ok(Some(_)) = sqlx::query_scalar::<_, String>(&downloading_match_query)
        .bind(content_id)
        .fetch_optional(pool)
        .await
    {
        return ContentStatus::Downloading;
    }

    // 4. Check wanted status
    if wanted {
        return ContentStatus::Wanted;
    }

    // 5. Default to missing
    ContentStatus::Missing
}

/// Auto-download mode for media items
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize, sqlx::Type)]
#[graphql(name = "AutoDownloadMode")]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum AutoDownloadMode {
    /// Do not auto-download
    #[graphql(name = "NONE")]
    None,
    /// Auto-download all items
    #[graphql(name = "ALL")]
    All,
    /// Auto-download only wanted items
    #[graphql(name = "WANTED")]
    Wanted,
}

impl Default for AutoDownloadMode {
    fn default() -> Self {
        Self::None
    }
}

impl std::fmt::Display for AutoDownloadMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::All => write!(f, "all"),
            Self::Wanted => write!(f, "wanted"),
        }
    }
}
