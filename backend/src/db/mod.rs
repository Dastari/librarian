//! Database connection and operations
//!
//! This module supports both PostgreSQL and SQLite backends via feature flags.
//! - `postgres` feature: Uses PostgreSQL (original backend)
//! - `sqlite` feature: Uses SQLite (for self-hosted deployment)
//!
//! Re-exports are provided for convenience, even if not all are used within the crate.

#![allow(unused_imports)]

pub mod albums;
pub mod artwork;
pub mod audiobooks;
pub mod cast;
pub mod episodes;
pub mod indexers;
pub mod libraries;
pub mod logs;
pub mod media_files;
pub mod notifications;
pub mod movies;
pub mod naming_patterns;
pub mod pending_file_matches;
pub mod playback;
pub mod priority_rules;
pub mod rss_feeds;
pub mod schedule;
pub mod settings;
pub mod sqlite_helpers;
pub mod subtitles;
pub mod torrent_files;
pub mod torrents;
pub mod tracks;
pub mod tv_shows;
pub mod usenet_downloads;
pub mod usenet_servers;
pub mod users;
pub mod watch_progress;

use anyhow::Result;

// Database pool type aliases based on feature flags
// postgres takes precedence when both features are enabled
#[cfg(feature = "postgres")]
pub type DbPool = sqlx::PgPool;
#[cfg(feature = "postgres")]
use sqlx::postgres::PgPoolOptions as DbPoolOptions;

#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub type DbPool = sqlx::SqlitePool;
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
use sqlx::sqlite::SqlitePoolOptions as DbPoolOptions;

// Re-export the pool type for external use
pub use DbPool as Pool;

pub use albums::{AlbumRecord, AlbumRepository, ArtistRecord};
pub use artwork::{ArtworkRecord, ArtworkRepository, ArtworkWithData, UpsertArtwork};
pub use audiobooks::{
    AudiobookAuthorRecord, AudiobookChapterRecord, AudiobookChapterRepository, AudiobookRecord,
    AudiobookRepository, CreateAudiobook, CreateAudiobookChapter,
};
pub use cast::{
    CastDeviceRecord, CastRepository, CastSessionRecord, CastSettingsRecord, CreateCastDevice,
    CreateCastSession, UpdateCastDevice, UpdateCastSession, UpdateCastSettings,
};
pub use episodes::{CreateEpisode, EpisodeRecord, EpisodeRepository};
pub use indexers::{CreateIndexerConfig, IndexerRepository, UpdateIndexerConfig, UpsertCredential};
pub use libraries::{CreateLibrary, LibraryRecord, LibraryRepository, LibraryStats, UpdateLibrary};
pub use logs::{CreateLog, LogFilter, LogsRepository};
pub use notifications::{
    ActionType, CreateNotification, NotificationCategory, NotificationFilter,
    NotificationRecord, NotificationRepository, NotificationType, PaginatedNotifications,
    Resolution,
};
pub use media_files::{CreateMediaFile, EmbeddedMetadata, MediaFileRecord, MediaFileRepository};
pub use movies::{CreateMovie, MovieCollectionRecord, MovieRecord, MovieRepository, UpdateMovie};
pub use naming_patterns::{CreateNamingPattern, NamingPatternRecord, NamingPatternRepository, UpdateNamingPattern};
pub use playback::{
    PlaybackRepository, PlaybackSessionRecord, UpdatePlaybackPosition, UpsertPlaybackSession,
};
pub use rss_feeds::{
    CreateRssFeed, CreateRssFeedItem, RssFeedRecord, RssFeedRepository, UpdateRssFeed,
};
pub use schedule::{
    ScheduleCacheRecord, ScheduleRepository, ScheduleSyncStateRecord, UpsertScheduleEntry,
};
pub use settings::SettingsRepository;
pub use subtitles::{
    AudioStreamRecord, ChapterRecord, CreateDownloadedSubtitle, CreateEmbeddedSubtitle,
    CreateExternalSubtitle, StreamRepository, SubtitleRecord, SubtitleRepository,
    SubtitleSourceType, VideoStreamRecord,
};
pub use pending_file_matches::{
    CreatePendingFileMatch, MatchTarget, PendingFileMatchRecord, PendingFileMatchRepository,
};
pub use torrent_files::{TorrentFileRecord, TorrentFileRepository, UpsertTorrentFile};
pub use torrents::{CreateTorrent, TorrentRecord, TorrentRepository};
pub use tracks::{CreateTrack, TrackRecord, TrackRepository, TrackWithStatus, UpdateTrack};
pub use tv_shows::{CreateTvShow, TvShowRecord, TvShowRepository, UpdateTvShow};
pub use watch_progress::{UpsertWatchProgress, WatchProgressRecord, WatchProgressRepository};
pub use priority_rules::{
    CreatePriorityRule, PriorityRuleRecord, PriorityRulesRepository, SourceRef, SourceType,
    UpdatePriorityRule,
};
pub use usenet_servers::{
    CreateUsenetServer, UpdateUsenetServer, UsenetServerRecord, UsenetServersRepository,
};
pub use usenet_downloads::{
    CreateUsenetDownload, UpdateUsenetDownload, UsenetDownloadRecord, UsenetDownloadsRepository,
};
pub use users::{
    CreateUser, InviteTokenRecord, RefreshTokenRecord, UpdateUser, UserLibraryAccessRecord,
    UserRecord, UserRestrictionRecord, UsersRepository,
};

/// Database wrapper providing connection pool access
#[derive(Clone)]
pub struct Database {
    pool: DbPool,
}

impl Database {
    /// Create a new database wrapper from an existing pool
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get the maximum connection pool size from environment or default
    fn get_max_connections() -> u32 {
        std::env::var("DATABASE_MAX_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10)
    }

    /// Create a new database connection pool
    #[cfg(feature = "postgres")]
    pub async fn connect(url: &str) -> Result<Self> {
        let max_connections = Self::get_max_connections();
        let pool = DbPoolOptions::new()
            .max_connections(max_connections)
            .connect(url)
            .await?;

        Ok(Self { pool })
    }

    /// Create a new database connection pool for SQLite
    #[cfg(feature = "sqlite")]
    pub async fn connect(url: &str) -> Result<Self> {
        use anyhow::Context;
        use sqlx::sqlite::SqliteConnectOptions;
        use std::str::FromStr;
        use std::path::Path;

        // Ensure parent directory exists for the SQLite file
        // Handle both "sqlite://path" URLs and plain file paths
        let db_path = url.strip_prefix("sqlite://").unwrap_or(url);
        if let Some(parent) = Path::new(db_path).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create database directory: {:?}", parent))?;
            }
        }

        // Parse the URL and enable WAL mode for better concurrency
        let options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(30));

        let pool = DbPoolOptions::new()
            .max_connections(Self::get_max_connections())
            .connect_with(options)
            .await?;

        // Enable foreign keys
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    /// Create a new database connection pool with retry logic
    /// Retries every `retry_interval` until successful
    #[cfg(feature = "postgres")]
    pub async fn connect_with_retry(url: &str, retry_interval: std::time::Duration) -> Self {
        let max_connections = Self::get_max_connections();
        loop {
            match DbPoolOptions::new()
                .max_connections(max_connections)
                .acquire_timeout(std::time::Duration::from_secs(10))
                .connect(url)
                .await
            {
                Ok(pool) => {
                    return Self { pool };
                }
                Err(e) => {
                    eprintln!(
                        "Database connection failed: {}. Retrying in {} seconds...",
                        e,
                        retry_interval.as_secs()
                    );
                    tokio::time::sleep(retry_interval).await;
                }
            }
        }
    }

    /// Create a new database connection pool with retry logic for SQLite
    #[cfg(feature = "sqlite")]
    pub async fn connect_with_retry(url: &str, retry_interval: std::time::Duration) -> Self {
        loop {
            match Self::connect(url).await {
                Ok(db) => return db,
                Err(e) => {
                    eprintln!(
                        "Database connection failed: {}. Retrying in {} seconds...",
                        e,
                        retry_interval.as_secs()
                    );
                    tokio::time::sleep(retry_interval).await;
                }
            }
        }
    }

    /// Get the connection pool
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    /// Get a torrent repository
    pub fn torrents(&self) -> TorrentRepository {
        TorrentRepository::new(self.pool.clone())
    }

    /// Get a torrent files repository
    pub fn torrent_files(&self) -> TorrentFileRepository {
        TorrentFileRepository::new(self.pool.clone())
    }

    /// Get a settings repository
    pub fn settings(&self) -> SettingsRepository {
        SettingsRepository::new(self.pool.clone())
    }

    /// Get a library repository
    pub fn libraries(&self) -> LibraryRepository {
        LibraryRepository::new(self.pool.clone())
    }

    /// Get a TV show repository
    pub fn tv_shows(&self) -> TvShowRepository {
        TvShowRepository::new(self.pool.clone())
    }

    /// Get an episode repository
    pub fn episodes(&self) -> EpisodeRepository {
        EpisodeRepository::new(self.pool.clone())
    }

    /// Get an RSS feed repository
    pub fn rss_feeds(&self) -> RssFeedRepository {
        RssFeedRepository::new(self.pool.clone())
    }

    /// Get a media files repository
    pub fn media_files(&self) -> MediaFileRepository {
        MediaFileRepository::new(self.pool.clone())
    }

    /// Get a movies repository
    pub fn movies(&self) -> MovieRepository {
        MovieRepository::new(self.pool.clone())
    }

    /// Get a logs repository
    pub fn logs(&self) -> LogsRepository {
        LogsRepository::new(self.pool.clone())
    }

    /// Get an indexer repository
    pub fn indexers(&self) -> IndexerRepository {
        IndexerRepository::new(self.pool.clone())
    }

    /// Get a cast repository
    pub fn cast(&self) -> CastRepository {
        CastRepository::new(self.pool.clone())
    }

    /// Get a playback repository
    pub fn playback(&self) -> PlaybackRepository {
        PlaybackRepository::new(self.pool.clone())
    }

    /// Get a schedule cache repository
    pub fn schedule(&self) -> ScheduleRepository {
        ScheduleRepository::new(self.pool.clone())
    }

    /// Get a subtitle repository
    pub fn subtitles(&self) -> SubtitleRepository {
        SubtitleRepository::new(self.pool.clone())
    }

    /// Get a stream repository (video/audio streams, chapters)
    pub fn streams(&self) -> StreamRepository {
        StreamRepository::new(self.pool.clone())
    }

    /// Get a naming patterns repository
    pub fn naming_patterns(&self) -> NamingPatternRepository {
        NamingPatternRepository::new(self.pool.clone())
    }

    /// Get a watch progress repository
    pub fn watch_progress(&self) -> WatchProgressRepository {
        WatchProgressRepository::new(self.pool.clone())
    }

    /// Get an albums repository
    pub fn albums(&self) -> AlbumRepository {
        AlbumRepository::new(self.pool.clone())
    }

    /// Get an audiobooks repository
    pub fn audiobooks(&self) -> AudiobookRepository {
        AudiobookRepository::new(self.pool.clone())
    }

    /// Get an audiobook chapters repository
    pub fn chapters(&self) -> AudiobookChapterRepository {
        AudiobookChapterRepository::new(self.pool.clone())
    }

    /// Get a tracks repository
    pub fn tracks(&self) -> TrackRepository {
        TrackRepository::new(self.pool.clone())
    }

    /// Get a pending file matches repository (source-agnostic)
    pub fn pending_file_matches(&self) -> PendingFileMatchRepository {
        PendingFileMatchRepository::new(self.pool.clone())
    }

    /// Get a priority rules repository
    pub fn priority_rules(&self) -> PriorityRulesRepository {
        PriorityRulesRepository::new(self.pool.clone())
    }

    /// Get a usenet servers repository
    pub fn usenet_servers(&self) -> UsenetServersRepository {
        UsenetServersRepository::new(self.pool.clone())
    }

    /// Get a usenet downloads repository
    pub fn usenet_downloads(&self) -> UsenetDownloadsRepository {
        UsenetDownloadsRepository::new(self.pool.clone())
    }

    /// Get a notifications repository
    pub fn notifications(&self) -> NotificationRepository {
        NotificationRepository::new(self.pool.clone())
    }

    /// Get a users repository
    pub fn users(&self) -> UsersRepository {
        UsersRepository::new(self.pool.clone())
    }

    /// Get an artwork repository
    pub fn artwork(&self) -> ArtworkRepository {
        ArtworkRepository::new(self.pool.clone())
    }

    /// Run database migrations (PostgreSQL)
    #[cfg(feature = "postgres")]
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Run database migrations (SQLite)
    /// 
    /// For a self-contained application, we handle checksum mismatches gracefully:
    /// if a migration was already applied but the file changed (e.g., due to version updates),
    /// we update the checksum rather than failing.
    #[cfg(feature = "sqlite")]
    pub async fn migrate(&self) -> Result<()> {
        let migrator = sqlx::migrate!("./migrations_sqlite");
        
        match migrator.run(&self.pool).await {
            Ok(()) => Ok(()),
            Err(sqlx::migrate::MigrateError::VersionMismatch(version)) => {
                // A migration was modified after being applied
                // For a standalone app, update the checksum and continue
                tracing::warn!(
                    "Migration {} was modified since last run, updating checksum",
                    version
                );
                
                // Get the expected checksum from the migrator
                if let Some(migration) = migrator.migrations.iter().find(|m| m.version == version) {
                    let checksum_bytes = migration.checksum.as_ref();
                    sqlx::query("UPDATE _sqlx_migrations SET checksum = ?1 WHERE version = ?2")
                        .bind(checksum_bytes)
                        .bind(version)
                        .execute(&self.pool)
                        .await?;
                    
                    // Try running migrations again
                    migrator.run(&self.pool).await?;
                }
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }
}
