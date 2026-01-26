//! Database service: wraps the SQLite pool for lifecycle (start/stop/health) and dependencies.
//!
//! Other services that need the database (e.g. logging) should declare `dependencies: ["database"]`.

use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::Engine;
use sqlx::query;
use tracing::{info, warn};

use crate::db::schema_sync::{run_seeds, sync_all_entity_schemas};
use crate::db::{Database, connect_with_retry};
use crate::services::manager::{Service, ServiceHealth};

/// Key used to store the JWT signing secret in auth_secrets. Not exposed via GraphQL.
const AUTH_SECRETS_JWT_KEY: &str = "jwt_secret";

/// Ensure a JWT secret row exists in auth_secrets (generated if missing).
/// The auth_secrets table is created by schema_sync. Secret is stored only in the database
/// and must never be exposed via GraphQL.
async fn initialize_jwt_secret(pool: &Database) -> Result<()> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM auth_secrets WHERE key = ?")
            .bind(AUTH_SECRETS_JWT_KEY)
            .fetch_optional(pool)
            .await?;

    if let Some((value,)) = row {
        if value.trim().is_empty() {
            let secret = generate_jwt_secret();
            sqlx::query("INSERT OR REPLACE INTO auth_secrets (key, value) VALUES (?, ?)")
                .bind(AUTH_SECRETS_JWT_KEY)
                .bind(&secret)
                .execute(pool)
                .await?;
            info!(service = "database", "JWT secret was empty; generated and stored new secret");
        }
        return Ok(());
    }

    let secret = generate_jwt_secret();
    sqlx::query("INSERT INTO auth_secrets (key, value) VALUES (?, ?)")
        .bind(AUTH_SECRETS_JWT_KEY)
        .bind(&secret)
        .execute(pool)
        .await?;
    info!(service = "database", "JWT secret generated and stored in database");
    Ok(())
}

fn generate_jwt_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

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

#[async_trait]
impl Service for DatabaseService {
    fn name(&self) -> &str {
        "database"
    }

    fn dependencies(&self) -> Vec<String> {
        Vec::new()
    }

    async fn start(&self) -> Result<()> {
        info!(service = "database", "Database service starting");
        // Pool is already connected by caller; just verify
        query("SELECT 1").execute(self.pool()).await?;

        info!(service = "database", "Syncing entity schemas");
        let sync_result = sync_all_entity_schemas(self.pool()).await;
        if !sync_result.tables_created.is_empty() {
            info!(
                service = "database",
                tables = ?sync_result.tables_created,
                "Created tables"
            );
        }
        if !sync_result.columns_added.is_empty() {
            info!(
                service = "database",
                columns = ?sync_result.columns_added,
                "Added columns"
            );
        }
        for err in &sync_result.errors {
            warn!(service = "database", error = %err, "Schema sync error");
        }
        info!(service = "database", "Entity schema sync complete");

        info!(service = "database", "Running pre-seed data");
        let seed_result = run_seeds(self.pool()).await;
        for err in &seed_result.errors {
            warn!(service = "database", error = %err, "Seed error");
        }
        if !seed_result.tables_seeded.is_empty() {
            info!(
                service = "database",
                tables = ?seed_result.tables_seeded,
                "Pre-seed complete"
            );
        }

        initialize_jwt_secret(self.pool()).await?;

        info!(service = "database", "Database service started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.pool.close().await;
        info!(service = "database", "Database service stopped");
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        match query("SELECT 1").execute(self.pool()).await {
            Ok(_) => Ok(ServiceHealth::healthy()),
            Err(e) => {
                warn!(service = "database", error = %e, "Health check failed");
                Ok(ServiceHealth::unhealthy(e.to_string()))
            }
        }
    }
}
