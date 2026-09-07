//! Sources Service
//!
//! Lifecycle service that manages content acquisition sources.
//! Loads sources from the database on start, provides search/download capabilities
//! to GraphQL resolvers.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::db::Database;
use crate::graphql::entities::{AppSetting, Source, UpdateSourceInput};
use crate::services::backup::VerifiedFullBackup;
use crate::services::manager::{Service, ServiceHealth, ServicesManager};

use super::encryption::CredentialEncryption;
use super::manager::SourcesManager;

/// Configuration for the Sources service
#[derive(Clone, Default)]
pub struct SourcesServiceConfig {
    credential_key: Option<String>,
}

impl std::fmt::Debug for SourcesServiceConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SourcesServiceConfig")
            .field(
                "credential_key",
                &self.credential_key.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

impl SourcesServiceConfig {
    pub fn from_env() -> Result<Self> {
        let key_file = std::env::var("LIBRARIAN_SOURCE_CREDENTIAL_KEY_FILE")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from);
        let key = if let Some(path) = key_file {
            Some(read_key_file(&path)?)
        } else {
            std::env::var("LIBRARIAN_SOURCE_CREDENTIAL_KEY")
                .or_else(|_| std::env::var("INDEXER_ENCRYPTION_KEY"))
                .ok()
                .filter(|value| !value.trim().is_empty())
        };
        if let Some(key) = key.as_deref() {
            CredentialEncryption::from_base64_key(key)
                .context("Invalid external source credential encryption key")?;
        }
        Ok(Self {
            credential_key: key,
        })
    }

    #[cfg(test)]
    pub fn with_key(key: String) -> Self {
        Self {
            credential_key: Some(key),
        }
    }

    pub(crate) fn credential_key(&self) -> Option<&str> {
        self.credential_key.as_deref()
    }
}

pub(crate) fn read_key_file(path: &Path) -> Result<String> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to inspect credential key file {}", path.display()))?;
    if !metadata.is_file() {
        anyhow::bail!(
            "Credential key path is not a regular file: {}",
            path.display()
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            anyhow::bail!(
                "Credential key file {} permissions are {:o}; require 0600 or stricter",
                path.display(),
                mode
            );
        }
    }
    let key = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read credential key file {}", path.display()))?;
    Ok(key.trim().to_string())
}

#[derive(Debug, Clone)]
pub(crate) struct SourceCredentialRotationPlan {
    pub credential_rows: usize,
    pub current_envelopes: usize,
    pub legacy_envelopes: usize,
    pub plaintext_rows: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct SourceCredentialRotationReport {
    pub rows_rotated: usize,
    pub legacy_database_key_removed: bool,
}

pub(crate) async fn plan_source_credential_rotation(
    db: &Database,
    active_key: Option<&str>,
    new_key: &str,
) -> Result<SourceCredentialRotationPlan> {
    let active = active_key
        .map(CredentialEncryption::from_base64_key)
        .transpose()
        .context("Active source credential key is invalid")?;
    let new_encryption = CredentialEncryption::from_base64_key(new_key)
        .context("New source credential key is invalid")?;
    let legacy_setting = find_legacy_key_setting(db).await?;
    let legacy = legacy_setting
        .as_ref()
        .map(|setting| CredentialEncryption::from_base64_key(&setting.value))
        .transpose()
        .context("Stored legacy source credential key is malformed")?;

    let mut plan = SourceCredentialRotationPlan {
        credential_rows: 0,
        current_envelopes: 0,
        legacy_envelopes: 0,
        plaintext_rows: 0,
    };
    for source in Source::query(db.pool()).fetch_all().await? {
        if source.credentials.is_empty() {
            continue;
        }
        plan.credential_rows += 1;
        if source.credentials.starts_with("v2:") {
            plan.current_envelopes += 1;
        } else if source.credentials.contains(':') {
            plan.legacy_envelopes += 1;
        } else {
            plan.plaintext_rows += 1;
        }
        let plaintext =
            decrypt_source_credentials(&source.credentials, active.as_ref(), legacy.as_ref())
                .with_context(|| format!("Source {} credentials cannot be decrypted", source.id))?;
        validate_credential_json(&plaintext)
            .with_context(|| format!("Source {} credentials are not valid JSON", source.id))?;
        let candidate = new_encryption.encrypt_envelope(&plaintext)?;
        if new_encryption.decrypt_envelope(&candidate)? != plaintext {
            anyhow::bail!("New-key verification failed for source {}", source.id);
        }
    }
    Ok(plan)
}

pub(crate) async fn rotate_source_credentials(
    db: &Database,
    active_key: Option<&str>,
    new_key: &str,
    verified_backup: &VerifiedFullBackup,
) -> Result<SourceCredentialRotationReport> {
    let _ = verified_backup.snapshot_id();
    let active = active_key
        .map(CredentialEncryption::from_base64_key)
        .transpose()
        .context("Active source credential key is invalid")?;
    let new_encryption = CredentialEncryption::from_base64_key(new_key)
        .context("New source credential key is invalid")?;
    let legacy_setting = find_legacy_key_setting(db).await?;
    let legacy = legacy_setting
        .as_ref()
        .map(|setting| CredentialEncryption::from_base64_key(&setting.value))
        .transpose()
        .context("Stored legacy source credential key is malformed")?;

    let mut changes = Vec::new();
    for source in Source::query(db.pool()).fetch_all().await? {
        if source.credentials.is_empty() {
            continue;
        }
        let plaintext =
            decrypt_source_credentials(&source.credentials, active.as_ref(), legacy.as_ref())
                .with_context(|| format!("Source {} credentials cannot be decrypted", source.id))?;
        validate_credential_json(&plaintext)
            .with_context(|| format!("Source {} credentials are not valid JSON", source.id))?;
        let new_envelope = new_encryption.encrypt_envelope(&plaintext)?;
        changes.push((source.id, source.credentials, new_envelope, plaintext));
    }

    let mut applied = Vec::new();
    for (source_id, old_envelope, new_envelope, plaintext) in &changes {
        if let Err(error) = update_source_credentials(db, source_id, new_envelope.clone()).await {
            rollback_source_credentials(db, &applied).await;
            return Err(error).with_context(|| {
                format!(
                    "Credential rotation failed for source {source_id}; applied rows were rolled back"
                )
            });
        }
        let stored = Source::get(db.pool(), source_id)
            .await?
            .context("Rotated source disappeared during verification")?;
        if new_encryption.decrypt_envelope(&stored.credentials)? != *plaintext {
            applied.push((source_id.clone(), old_envelope.clone()));
            rollback_source_credentials(db, &applied).await;
            anyhow::bail!(
                "Credential rotation verification failed for source {source_id}; applied rows were rolled back"
            );
        }
        applied.push((source_id.clone(), old_envelope.clone()));
    }

    if let Some(setting) = legacy_setting.as_ref()
        && let Err(error) = AppSetting::delete_by_id(db, &setting.id).await
    {
        rollback_source_credentials(db, &applied).await;
        return Err(error).context(
            "Rotated credentials but could not remove the legacy database key; rows were rolled back",
        );
    }

    Ok(SourceCredentialRotationReport {
        rows_rotated: changes.len(),
        legacy_database_key_removed: legacy_setting.is_some(),
    })
}

async fn find_legacy_key_setting(db: &Database) -> Result<Option<AppSetting>> {
    Ok(AppSetting::query(db.pool())
        .fetch_all()
        .await?
        .into_iter()
        .find(|setting| setting.key == "sources_encryption_key"))
}

fn decrypt_source_credentials(
    envelope: &str,
    active: Option<&CredentialEncryption>,
    legacy: Option<&CredentialEncryption>,
) -> Result<String> {
    if envelope.starts_with("v2:") {
        return active
            .context("Active external key is required for a versioned envelope")?
            .decrypt_envelope(envelope);
    }
    if envelope.contains(':') {
        if let Some(active) = active
            && let Ok(plaintext) = active.decrypt_legacy_envelope(envelope)
        {
            return Ok(plaintext);
        }
        return legacy
            .context("Legacy database key is required for a legacy envelope")?
            .decrypt_legacy_envelope(envelope);
    }
    Ok(envelope.to_string())
}

fn validate_credential_json(plaintext: &str) -> Result<()> {
    serde_json::from_str::<HashMap<String, String>>(plaintext)
        .context("Credentials must be a JSON object containing string values")?;
    Ok(())
}

async fn update_source_credentials(db: &Database, id: &str, credentials: String) -> Result<()> {
    Source::update_by_id(
        db,
        &id.to_string(),
        UpdateSourceInput {
            credentials: Some(credentials),
            ..UpdateSourceInput::default()
        },
    )
    .await?;
    Ok(())
}

async fn rollback_source_credentials(db: &Database, applied: &[(String, String)]) {
    for (source_id, old_envelope) in applied.iter().rev() {
        if let Err(error) = update_source_credentials(db, source_id, old_envelope.clone()).await {
            error!(
                source_id = %source_id,
                error = %error,
                "Failed to roll back source credentials; restore the verified full backup"
            );
        }
    }
}

/// The Sources service: manages all configured content acquisition sources.
///
/// Register with `ServicesManager` via `register_sources()`. Depends on `database`.
/// On start, loads sources from DB, instantiates `Source` trait objects.
pub struct SourcesService {
    services: Arc<ServicesManager>,
    config: SourcesServiceConfig,
    manager: RwLock<Option<Arc<SourcesManager>>>,
    health_message: RwLock<Option<String>>,
}

impl SourcesService {
    pub fn new(services: Arc<ServicesManager>, config: SourcesServiceConfig) -> Self {
        Self {
            services,
            config,
            manager: RwLock::new(None),
            health_message: RwLock::new(None),
        }
    }

    /// Get the SourcesManager (only available after start)
    pub async fn get_manager(&self) -> Option<Arc<SourcesManager>> {
        self.manager.read().await.clone()
    }

    /// Read the historical database-resident key only for compatibility while
    /// an operator performs the explicit external-key migration.
    async fn legacy_database_key(&self) -> Result<Option<String>> {
        let db = self
            .services
            .get_database()
            .await
            .ok_or_else(|| anyhow::anyhow!("Database service not available"))?;

        if let Some(setting) = AppSetting::query(db.pool().pool())
            .fetch_all()
            .await?
            .into_iter()
            .find(|setting| setting.key == "sources_encryption_key")
        {
            return Ok(Some(setting.value));
        }
        Ok(None)
    }

    /// Load all enabled sources from the database
    async fn load_sources(
        &self,
        manager: &SourcesManager,
        legacy_encryption: Option<&CredentialEncryption>,
    ) -> Result<()> {
        let db = self
            .services
            .get_database()
            .await
            .ok_or_else(|| anyhow::anyhow!("Database service not available"))?;

        let mut rows = Source::query(db.pool().pool())
            .fetch_all()
            .await?
            .into_iter()
            .filter(|source| source.enabled)
            .collect::<Vec<_>>();
        rows.sort_by_key(|a| a.priority);

        manager.clear_sources();
        let mut loaded = 0;
        for row in rows {
            // Decrypt credentials from combined "nonce:ciphertext" format
            let credentials = if !row.credentials.is_empty() {
                let decrypted = if row.credentials.starts_with("v2:") {
                    manager.encryption().decrypt_envelope(&row.credentials)
                } else if row.credentials.contains(':') {
                    manager
                        .encryption()
                        .decrypt_legacy_envelope(&row.credentials)
                        .or_else(|_| {
                            legacy_encryption
                                .context("Legacy source credential key is unavailable")?
                                .decrypt_legacy_envelope(&row.credentials)
                        })
                } else {
                    Ok(row.credentials.clone())
                };
                match decrypted {
                    Ok(json) => {
                        match serde_json::from_str::<HashMap<String, String>>(&row.credentials) {
                            Ok(credentials) if !row.credentials.starts_with("v2:") => {
                                warn!(
                                    source_id = %row.id,
                                    source_name = %row.name,
                                    "Loaded source '{}' with legacy plaintext credentials; save the source to re-encrypt credentials",
                                    row.name
                                );
                                credentials
                            }
                            _ => serde_json::from_str::<HashMap<String, String>>(&json)
                                .unwrap_or_default(),
                        }
                    }
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
            let legacy_key = self.legacy_database_key().await?;
            let legacy = legacy_key
                .as_deref()
                .map(CredentialEncryption::from_base64_key)
                .transpose()?;
            self.load_sources(manager, legacy.as_ref()).await?;
        }
        Ok(())
    }
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

        let Some(encryption_key) = self.config.credential_key.as_deref() else {
            let message = "External source credential key is not configured; source acquisition is disabled. Set LIBRARIAN_SOURCE_CREDENTIAL_KEY or LIBRARIAN_SOURCE_CREDENTIAL_KEY_FILE.";
            error!(service = "sources", "{message}");
            *self.health_message.write().await = Some(message.to_string());
            return Ok(());
        };
        let sources_manager = Arc::new(SourcesManager::new(encryption_key)?);
        let legacy_key = self.legacy_database_key().await?;
        let legacy_encryption = legacy_key
            .as_deref()
            .map(CredentialEncryption::from_base64_key)
            .transpose()
            .context("Stored legacy source credential key is malformed")?;

        // Load sources from DB
        if let Err(e) = self
            .load_sources(&sources_manager, legacy_encryption.as_ref())
            .await
        {
            warn!(
                service = "sources",
                error = %e,
                "Failed to load sources from database, service will start empty"
            );
        }

        *self.manager.write().await = Some(sources_manager);
        *self.health_message.write().await = legacy_key.map(|_| {
            "Legacy database-resident source credential key detected; complete external-key migration"
                .to_string()
        });

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
        *self.health_message.write().await = None;
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        let guard = self.manager.read().await;
        if let Some(message) = self.health_message.read().await.clone() {
            Ok(ServiceHealth::degraded(message))
        } else if guard.is_some() {
            Ok(ServiceHealth::healthy())
        } else {
            Ok(ServiceHealth::degraded("Sources manager not initialized"))
        }
    }
}
