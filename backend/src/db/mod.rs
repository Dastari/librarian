//! Database connection and operations

pub mod episodes;
pub mod libraries;
pub mod media_files;
pub mod quality_profiles;
pub mod rss_feeds;
pub mod settings;
pub mod torrents;
pub mod tv_shows;

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub use episodes::{CreateEpisode, EpisodeRepository};
pub use libraries::{CreateLibrary, LibraryRepository, LibraryStats, UpdateLibrary};
pub use media_files::{CreateMediaFile, MediaFileRepository};
pub use quality_profiles::{CreateQualityProfile, QualityProfileRepository, UpdateQualityProfile};
pub use rss_feeds::{CreateRssFeed, RssFeedRepository, UpdateRssFeed};
pub use settings::SettingsRepository;
pub use torrents::{CreateTorrent, TorrentRepository};
pub use tv_shows::{CreateTvShow, TvShowRecord, TvShowRepository, UpdateTvShow};

/// Database wrapper providing connection pool access
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection pool
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(url)
            .await?;

        Ok(Self { pool })
    }

    /// Get the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get a torrent repository
    pub fn torrents(&self) -> TorrentRepository {
        TorrentRepository::new(self.pool.clone())
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

    /// Get a quality profile repository
    pub fn quality_profiles(&self) -> QualityProfileRepository {
        QualityProfileRepository::new(self.pool.clone())
    }

    /// Get an RSS feed repository
    pub fn rss_feeds(&self) -> RssFeedRepository {
        RssFeedRepository::new(self.pool.clone())
    }

    /// Get a media files repository
    pub fn media_files(&self) -> MediaFileRepository {
        MediaFileRepository::new(self.pool.clone())
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }
}
