//! Download Source Trait
//!
//! Universal interface for any download method (torrent, usenet, IRC, FTP, etc.).
//! This trait allows the download processing pipeline to work with any source
//! without knowing the specifics of how files were obtained.

use std::path::Path;
use uuid::Uuid;

/// What library item this download is linked to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkedItem {
    Episode(Uuid),
    Movie(Uuid),
    Album(Uuid),
    Audiobook(Uuid),
    TvShow(Uuid),  // For season packs
    Artist(Uuid),  // For discographies
    Track(Uuid),   // For single tracks
}

impl LinkedItem {
    /// Get the item type as a string
    pub fn item_type(&self) -> &'static str {
        match self {
            LinkedItem::Episode(_) => "episode",
            LinkedItem::Movie(_) => "movie",
            LinkedItem::Album(_) => "album",
            LinkedItem::Audiobook(_) => "audiobook",
            LinkedItem::TvShow(_) => "tv_show",
            LinkedItem::Artist(_) => "artist",
            LinkedItem::Track(_) => "track",
        }
    }

    /// Get the ID regardless of type
    pub fn id(&self) -> Uuid {
        match self {
            LinkedItem::Episode(id) => *id,
            LinkedItem::Movie(id) => *id,
            LinkedItem::Album(id) => *id,
            LinkedItem::Audiobook(id) => *id,
            LinkedItem::TvShow(id) => *id,
            LinkedItem::Artist(id) => *id,
            LinkedItem::Track(id) => *id,
        }
    }
}

/// Download method type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DownloadSourceType {
    Torrent,
    Usenet,
    Irc,    // Future: XDCC
    Ftp,    // Future: FTP/SFTP
    Http,   // Future: Direct HTTP downloads
    Manual, // Files manually placed in folder
    Other,
}

impl std::fmt::Display for DownloadSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadSourceType::Torrent => write!(f, "torrent"),
            DownloadSourceType::Usenet => write!(f, "usenet"),
            DownloadSourceType::Irc => write!(f, "irc"),
            DownloadSourceType::Ftp => write!(f, "ftp"),
            DownloadSourceType::Http => write!(f, "http"),
            DownloadSourceType::Manual => write!(f, "manual"),
            DownloadSourceType::Other => write!(f, "other"),
        }
    }
}

impl std::str::FromStr for DownloadSourceType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "torrent" => Ok(DownloadSourceType::Torrent),
            "usenet" => Ok(DownloadSourceType::Usenet),
            "irc" | "xdcc" => Ok(DownloadSourceType::Irc),
            "ftp" | "sftp" => Ok(DownloadSourceType::Ftp),
            "http" | "https" => Ok(DownloadSourceType::Http),
            "manual" => Ok(DownloadSourceType::Manual),
            _ => Ok(DownloadSourceType::Other),
        }
    }
}

/// Universal interface for any download source
///
/// This trait abstracts over the download method completely.
/// The download processing pipeline uses this interface to:
/// - Find where files are located
/// - Determine what library items they're for
/// - Know which user owns the download
///
/// Implementations exist for:
/// - TorrentRecord (via librqbit)
/// - UsenetDownloadRecord (via UsenetService)
/// - Future: IrcDownload, FtpDownload, etc.
pub trait DownloadSource: Send + Sync {
    /// Unique identifier for this download
    fn id(&self) -> String;

    /// Human-readable name (for UI/logging)
    fn name(&self) -> &str;

    /// The download method type (for UI badges, statistics)
    fn source_type(&self) -> DownloadSourceType;

    /// Path where downloaded files are located
    fn download_path(&self) -> &Path;

    /// User who initiated this download
    fn user_id(&self) -> Uuid;

    /// Optional: Library this download is linked to
    fn library_id(&self) -> Option<Uuid>;

    /// Optional: Specific item this download is for
    fn linked_item(&self) -> Option<LinkedItem>;

    /// Optional: Indexer that provided this download
    fn indexer_id(&self) -> Option<String>;

    /// Current processing status
    fn post_process_status(&self) -> Option<&str>;
}

// Implement DownloadSource for TorrentRecord
use crate::db::TorrentRecord;
#[allow(unused_imports)]
use std::path::PathBuf;

impl DownloadSource for TorrentRecord {
    fn id(&self) -> String {
        self.info_hash.clone()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn source_type(&self) -> DownloadSourceType {
        DownloadSourceType::Torrent
    }

    fn download_path(&self) -> &Path {
        // Note: This creates a temporary Path, which might not be ideal
        // but works for the trait interface
        Path::new(self.download_path.as_deref().unwrap_or(""))
    }

    fn user_id(&self) -> Uuid {
        self.user_id
    }

    fn library_id(&self) -> Option<Uuid> {
        // Note: Legacy linking removed from TorrentRecord
        // Use pending_file_matches for actual file-level linking
        None
    }

    fn linked_item(&self) -> Option<LinkedItem> {
        // Note: Legacy linking removed from TorrentRecord
        // Use pending_file_matches for actual file-level linking
        None
    }

    fn indexer_id(&self) -> Option<String> {
        self.source_indexer_id.map(|id| id.to_string())
    }

    fn post_process_status(&self) -> Option<&str> {
        self.post_process_status.as_deref()
    }
}

// Implement DownloadSource for UsenetDownloadRecord
use crate::db::UsenetDownloadRecord;

impl DownloadSource for UsenetDownloadRecord {
    fn id(&self) -> String {
        self.id.to_string()
    }

    fn name(&self) -> &str {
        &self.nzb_name
    }

    fn source_type(&self) -> DownloadSourceType {
        DownloadSourceType::Usenet
    }

    fn download_path(&self) -> &Path {
        Path::new(self.download_path.as_deref().unwrap_or(""))
    }

    fn user_id(&self) -> Uuid {
        self.user_id
    }

    fn library_id(&self) -> Option<Uuid> {
        self.library_id
    }

    fn linked_item(&self) -> Option<LinkedItem> {
        self.episode_id
            .map(LinkedItem::Episode)
            .or_else(|| self.movie_id.map(LinkedItem::Movie))
            .or_else(|| self.album_id.map(LinkedItem::Album))
            .or_else(|| self.audiobook_id.map(LinkedItem::Audiobook))
    }

    fn indexer_id(&self) -> Option<String> {
        self.indexer_id.map(|id| id.to_string())
    }

    fn post_process_status(&self) -> Option<&str> {
        self.post_process_status.as_deref()
    }
}
