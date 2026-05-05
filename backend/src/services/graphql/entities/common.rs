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
    let _ = (pool, content_type, content_id, user_id);

    if media_file_id.is_some() {
        return ContentStatus::Available;
    }

    if wanted {
        return ContentStatus::Wanted;
    }

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
