//! Database service: wraps the SQLite pool for lifecycle (start/stop/health) and dependencies.
//!
//! Other services that need the database (e.g. logging) should declare `dependencies: ["database"]`.

use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use graphql_orm::graphql::orm::{Entity, SchemaStage, SchemaStageRunner};
use tracing::{info, warn};

use crate::db::{Database, connect_with_retry};
use crate::services::graphql::entities::*;
use crate::services::manager::{Service, ServiceHealth};

/// Configuration for the database service (connection URL, timeouts, etc.).
#[derive(Debug, Clone)]
pub struct DatabaseServiceConfig {
    /// SQLite connection URL (e.g. `sqlite:///data/librarian.db` or `sqlite::memory:`).
    pub database_url: String,
    /// How long to retry connecting before giving up.
    pub connect_timeout: Duration,
}

impl Default for DatabaseServiceConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite:librarian.db".to_string(),
            connect_timeout: Duration::from_secs(30),
        }
    }
}

/// Service that owns the database pool and provides start/stop/health.
/// Register this first so that services depending on `"database"` can start after it.
pub struct DatabaseService {
    pool: Database,
}

impl DatabaseService {
    /// Create a new database service with an already-connected pool.
    /// Use [from_config](Self::from_config) to create from URL and timeout.
    pub fn new(pool: Database) -> Self {
        Self { pool }
    }

    /// Create and connect the database service from config. Call this when building
    /// the service manager (e.g. in [ServicesManagerBuilder](crate::services::manager::ServicesManagerBuilder)).
    pub async fn from_config(config: DatabaseServiceConfig) -> Result<Self> {
        let pool = connect_with_retry(&config.database_url, config.connect_timeout)
            .await
            .context("Database service: connect_with_retry failed")?;
        Ok(Self::new(pool))
    }

    /// Access the pool (e.g. to clone for app state). Valid until [Service::stop] is called.
    pub fn pool(&self) -> &Database {
        &self.pool
    }
}

fn entity_schema_stage() -> SchemaStage {
    SchemaStage::from_entities(
        "0001",
        "create current graphql entity schema",
        &[
            <Library as Entity>::metadata(),
            <Movie as Entity>::metadata(),
            <Person as Entity>::metadata(),
            <MovieCastCredit as Entity>::metadata(),
            <Collection as Entity>::metadata(),
            <Show as Entity>::metadata(),
            <Episode as Entity>::metadata(),
            <MediaFile as Entity>::metadata(),
            <Artist as Entity>::metadata(),
            <Album as Entity>::metadata(),
            <Track as Entity>::metadata(),
            <Audiobook as Entity>::metadata(),
            <Chapter as Entity>::metadata(),
            <Torrent as Entity>::metadata(),
            <TorrentFile as Entity>::metadata(),
            <RssFeed as Entity>::metadata(),
            <RssFeedItem as Entity>::metadata(),
            <PendingFileMatch as Entity>::metadata(),
            <Source as Entity>::metadata(),
            <User as Entity>::metadata(),
            <InviteToken as Entity>::metadata(),
            <RefreshToken as Entity>::metadata(),
            <AppSetting as Entity>::metadata(),
            <AppLog as Entity>::metadata(),
            <VideoStream as Entity>::metadata(),
            <AudioStream as Entity>::metadata(),
            <Subtitle as Entity>::metadata(),
            <MediaChapter as Entity>::metadata(),
            <PlaybackSession as Entity>::metadata(),
            <PlaybackProgress as Entity>::metadata(),
            <CastDevice as Entity>::metadata(),
            <CastSession as Entity>::metadata(),
            <CastSetting as Entity>::metadata(),
            <UsenetServer as Entity>::metadata(),
            <UsenetDownload as Entity>::metadata(),
            <ScheduleCache as Entity>::metadata(),
            <ScheduleSyncState as Entity>::metadata(),
            <NamingPattern as Entity>::metadata(),
            <MetadataCache as Entity>::metadata(),
            <SourcePriorityRule as Entity>::metadata(),
            <Notification as Entity>::metadata(),
            <ArtworkCache as Entity>::metadata(),
            <TorznabCategory as Entity>::metadata(),
        ],
    )
}

#[async_trait]
impl Service for DatabaseService {
    fn name(&self) -> &str {
        "database"
    }

    fn dependencies(&self) -> Vec<String> {
        Vec::new()
    }

    async fn start(&self) -> Result<()> {
        info!(
            service = "database",
            "Database service starting: validating connection, syncing schema, and seeding defaults"
        );
        info!(service = "database", "Applying GraphQL ORM entity schema");
        if let Err(err) = self
            .pool()
            .apply_schema_stages(&[entity_schema_stage()])
            .await
        {
            warn!(
                service = "database",
                error = %err,
                "GraphQL ORM schema stage failed: error={}",
                err
            );
            return Err(err).context("GraphQL ORM schema stage failed");
        }
        info!(service = "database", "GraphQL ORM entity schema applied");

        info!(service = "database", "Database service started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.pool.pool().close().await;
        info!(
            service = "database",
            "Database service stopped: connection pool closed"
        );
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        Ok(ServiceHealth::healthy())
    }
}
