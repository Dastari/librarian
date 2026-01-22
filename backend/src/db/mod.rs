//! Database connection and operations
//!
//! Re-exports are provided for convenience, even if not all are used within the crate.

#![allow(unused_imports)]

pub mod albums;
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
pub mod subtitles;
pub mod torrent_files;
pub mod torrents;
pub mod tracks;
pub mod tv_shows;
pub mod usenet_downloads;
pub mod usenet_servers;
pub mod watch_progress;

use anyhow::Result;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub use albums::{AlbumRecord, AlbumRepository, ArtistRecord};
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

/// Database wrapper providing connection pool access
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database wrapper from an existing pool
    pub fn new(pool: PgPool) -> Self {
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
    pub async fn connect(url: &str) -> Result<Self> {
        let max_connections = Self::get_max_connections();
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(url)
            .await?;

        Ok(Self { pool })
    }

    /// Create a new database connection pool with retry logic
    /// Retries every `retry_interval` until successful
    pub async fn connect_with_retry(url: &str, retry_interval: std::time::Duration) -> Self {
        let max_connections = Self::get_max_connections();
        loop {
            match PgPoolOptions::new()
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

    /// Get the connection pool
    pub fn pool(&self) -> &PgPool {
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

    /// Run database migrations
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }
}
