//! Sources Service
//!
//! Lifecycle service that manages content acquisition sources.
//! Loads sources from the database on start, provides search/download capabilities
//! to GraphQL resolvers.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::services::manager::{Service, ServiceHealth, ServicesManager};

use super::encryption::CredentialEncryption;
use super::manager::SourcesManager;

/// Configuration for the Sources service
#[derive(Debug, Clone, Default)]
pub struct SourcesServiceConfig {}

/// The Sources service: manages all configured content acquisition sources.
///
/// Register with `ServicesManager` via `register_sources()`. Depends on `database`.
/// On start, loads sources from DB, instantiates `Source` trait objects.
pub struct SourcesService {
    services: Arc<ServicesManager>,
    manager: RwLock<Option<Arc<SourcesManager>>>,
}

impl SourcesService {
    pub fn new(services: Arc<ServicesManager>) -> Self {
        Self {
            services,
            manager: RwLock::new(None),
        }
    }

    /// Get the SourcesManager (only available after start)
    pub async fn get_manager(&self) -> Option<Arc<SourcesManager>> {
        self.manager.read().await.clone()
    }

    /// Load the encryption key from the database, creating one if needed
    async fn get_or_create_encryption_key(&self) -> Result<String> {
        let db = self
            .services
            .get_database()
            .await
            .ok_or_else(|| anyhow::anyhow!("Database service not available"))?;

        let pool = db.pool();

        // Try to get existing key
        let result = sqlx::query_scalar::<_, String>(
            "SELECT value FROM app_settings WHERE key = 'sources_encryption_key'",
        )
        .fetch_optional(pool)
        .await?;

        if let Some(key) = result {
            return Ok(key);
        }

        // Generate a new key
        let new_key = CredentialEncryption::generate_key();

        sqlx::query(
            "INSERT INTO app_settings (id, key, value, category, created_at, updated_at)
             VALUES (lower(hex(randomblob(16))), 'sources_encryption_key', ?, 'sources', datetime('now'), datetime('now'))"
        )
        .bind(&new_key)
        .execute(pool)
        .await?;

        info!(
            "Generated new sources encryption key and stored it in app_settings key 'sources_encryption_key'"
        );
        Ok(new_key)
    }

    /// Load all enabled sources from the database
    async fn load_sources(&self, manager: &SourcesManager) -> Result<()> {
        let db = self
            .services
            .get_database()
            .await
            .ok_or_else(|| anyhow::anyhow!("Database service not available"))?;

        let pool = db.pool();

        // Check if the sources table exists
        let table_exists = sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sources'",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        if table_exists == 0 {
            info!("Sources table does not exist yet; skipping source loading during startup");
            return Ok(());
        }

        let rows = sqlx::query_as::<_, SourceRow>(
            "SELECT id, name, source_type, definition_id, enabled, priority,
                    site_url, credentials, settings
             FROM sources
             WHERE enabled = 1
             ORDER BY priority ASC",
        )
        .fetch_all(pool)
        .await?;

        let mut loaded = 0;
        for row in rows {
            // Decrypt credentials from combined "nonce:ciphertext" format
            let credentials = if !row.credentials.is_empty() {
                // Split on first ':' → (nonce_b64, data_b64)
                match row.credentials.split_once(':') {
                    Some((nonce_b64, data_b64)) => {
                        match manager.encryption().decrypt(data_b64, nonce_b64) {
                            Ok(json) => serde_json::from_str::<HashMap<String, String>>(&json)
                                .unwrap_or_default(),
                            Err(e) => {
                                warn!(
                                    source_id = %row.id,
                                    source_name = %row.name,
                                    error = %e,
                                    "Failed to decrypt credentials for source '{}', skipping",
                                    row.name
                                );
                                continue;
                            }
                        }
                    }
                    None => {
                        warn!(
                            source_id = %row.id,
                            source_name = %row.name,
                            "Invalid credentials format for source '{}' (expected nonce:ciphertext), skipping",
                            row.name
                        );
                        continue;
                    }
                }
            } else {
                HashMap::new()
            };

            // Parse settings
            let settings: HashMap<String, String> = row
                .settings
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();

            match manager.load_source(
                &row.id,
                &row.definition_id,
                &row.name,
                row.site_url,
                row.priority,
                credentials,
                settings,
            ) {
                Ok(()) => loaded += 1,
                Err(e) => {
                    error!(
                        source_id = %row.id,
                        source_name = %row.name,
                        error = %e,
                        "Failed to load source '{}'",
                        row.name
                    );
                }
            }
        }

        info!(
            count = loaded,
            "Loaded sources from database: loaded_count={}", loaded
        );
        Ok(())
    }

    /// Reload all sources from the database (e.g., after adding/removing a source)
    pub async fn reload(&self) -> Result<()> {
        let guard = self.manager.read().await;
        if let Some(manager) = guard.as_ref() {
            self.load_sources(manager).await?;
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct SourceRow {
    id: String,
    name: String,
    #[allow(dead_code)]
    source_type: String,
    definition_id: String,
    #[allow(dead_code)]
    enabled: bool,
    priority: i32,
    site_url: Option<String>,
    /// Combined "nonce_b64:ciphertext_b64" format
    credentials: String,
    settings: Option<String>,
}

#[async_trait]
impl Service for SourcesService {
    fn name(&self) -> &str {
        "sources"
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["database".to_string()]
    }

    async fn start(&self) -> Result<()> {
        info!(
            service = "sources",
            "Sources service starting: loading encryption key and enabled sources from database"
        );

        let encryption_key = self.get_or_create_encryption_key().await?;
        let sources_manager = Arc::new(SourcesManager::new(&encryption_key)?);

        // Load sources from DB
        if let Err(e) = self.load_sources(&sources_manager).await {
            warn!(
                service = "sources",
                error = %e,
                "Failed to load sources from database, service will start empty"
            );
        }

        *self.manager.write().await = Some(sources_manager);

        info!(
            service = "sources",
            "Sources service started: sources manager initialized"
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        info!(
            service = "sources",
            "Sources service stopping: clearing in-memory sources manager"
        );
        *self.manager.write().await = None;
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        let guard = self.manager.read().await;
        if guard.is_some() {
            Ok(ServiceHealth::healthy())
        } else {
            Ok(ServiceHealth::degraded("Sources manager not initialized"))
        }
    }
}
