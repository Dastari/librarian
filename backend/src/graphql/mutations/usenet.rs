use super::prelude::*;
use crate::db::{CreateUsenetServer, UpdateUsenetServer};
use crate::indexer::encryption::CredentialEncryption;
use crate::services::usenet::{UsenetService, UsenetServiceConfig};

#[derive(Default)]
pub struct UsenetMutations;

#[Object]
impl UsenetMutations {
    /// Create a new usenet server
    async fn create_usenet_server(
        &self,
        ctx: &Context<'_>,
        input: CreateUsenetServerInput,
    ) -> Result<UsenetServerResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        // Encrypt password if provided
        let (encrypted_password, password_nonce) = if let Some(ref password) = input.password {
            let encryption_key = db
                .settings()
                .get_or_create_indexer_encryption_key()
                .await
                .map_err(|e| async_graphql::Error::new(format!("Failed to get encryption key: {}", e)))?;
            let encryption = CredentialEncryption::from_base64_key(&encryption_key)
                .map_err(|e| async_graphql::Error::new(format!("Encryption error: {}", e)))?;
            let (encrypted, nonce) = encryption.encrypt(password)
                .map_err(|e| async_graphql::Error::new(format!("Encryption error: {}", e)))?;
            (Some(encrypted), Some(nonce))
        } else {
            (None, None)
        };

        let record = db
            .usenet_servers()
            .create(CreateUsenetServer {
                user_id,
                name: input.name,
                host: input.host,
                port: input.port,
                use_ssl: input.use_ssl.unwrap_or(true),
                username: input.username,
                encrypted_password,
                password_nonce,
                connections: input.connections.unwrap_or(10),
                priority: input.priority.unwrap_or(0),
                retention_days: input.retention_days,
            })
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(UsenetServerResult {
            success: true,
            error: None,
            server: Some(UsenetServer::from(record)),
        })
    }

    /// Update a usenet server
    async fn update_usenet_server(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateUsenetServerInput,
    ) -> Result<UsenetServerResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let server_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid server ID: {}", e)))?;

        // Verify ownership
        let existing = db
            .usenet_servers()
            .get(server_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Server not found"))?;

        let user_id = Uuid::parse_str(&user.user_id)?;
        if existing.user_id != user_id {
            return Err(async_graphql::Error::new("Not authorized"));
        }

        // Encrypt password if provided
        let (encrypted_password, password_nonce) = if let Some(ref password) = input.password {
            let encryption_key = db
                .settings()
                .get_or_create_indexer_encryption_key()
                .await
                .map_err(|e| async_graphql::Error::new(format!("Failed to get encryption key: {}", e)))?;
            let encryption = CredentialEncryption::from_base64_key(&encryption_key)
                .map_err(|e| async_graphql::Error::new(format!("Encryption error: {}", e)))?;
            let (encrypted, nonce) = encryption.encrypt(password)
                .map_err(|e| async_graphql::Error::new(format!("Encryption error: {}", e)))?;
            (Some(encrypted), Some(nonce))
        } else {
            (None, None)
        };

        let record = db
            .usenet_servers()
            .update(
                server_id,
                UpdateUsenetServer {
                    name: input.name,
                    host: input.host,
                    port: input.port,
                    use_ssl: input.use_ssl,
                    username: input.username,
                    encrypted_password,
                    password_nonce,
                    connections: input.connections,
                    priority: input.priority,
                    enabled: input.enabled,
                    retention_days: input.retention_days,
                },
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(UsenetServerResult {
            success: true,
            error: None,
            server: Some(UsenetServer::from(record)),
        })
    }

    /// Delete a usenet server
    async fn delete_usenet_server(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let server_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid server ID: {}", e)))?;

        // Verify ownership
        let existing = db
            .usenet_servers()
            .get(server_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Server not found"))?;

        let user_id = Uuid::parse_str(&user.user_id)?;
        if existing.user_id != user_id {
            return Err(async_graphql::Error::new("Not authorized"));
        }

        let deleted = db
            .usenet_servers()
            .delete(server_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: deleted,
            error: if deleted { None } else { Some("Failed to delete".to_string()) },
        })
    }

    /// Reorder usenet servers
    async fn reorder_usenet_servers(
        &self,
        ctx: &Context<'_>,
        ids: Vec<String>,
    ) -> Result<MutationResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let server_ids: Vec<Uuid> = ids
            .iter()
            .filter_map(|id| Uuid::parse_str(id).ok())
            .collect();

        db.usenet_servers()
            .reorder(user_id, &server_ids)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: true,
            error: None,
        })
    }

    /// Add a usenet download from URL
    async fn add_usenet_download(
        &self,
        ctx: &Context<'_>,
        input: AddUsenetDownloadInput,
    ) -> Result<UsenetDownloadResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        // TODO: Get UsenetService from context
        // For now, create a temporary one
        let usenet_service = UsenetService::new(
            db.clone(),
            UsenetServiceConfig::default(),
        );

        let library_id = input.library_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());
        let episode_id = input.episode_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());
        let movie_id = input.movie_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());
        let album_id = input.album_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());
        let audiobook_id = input.audiobook_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());
        let indexer_id = input.indexer_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());

        let info = usenet_service
            .add_nzb_url(
                &input.nzb_url,
                user_id,
                library_id,
                episode_id,
                movie_id,
                album_id,
                audiobook_id,
                indexer_id,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(UsenetDownloadResult {
            success: true,
            error: None,
            download: Some(UsenetDownload {
                id: info.id.to_string(),
                name: info.name,
                state: info.state,
                progress: info.progress,
                size: info.size_bytes,
                downloaded: info.downloaded_bytes,
                download_speed: info.download_speed,
                eta_seconds: info.eta_seconds,
                error_message: info.error_message,
                library_id: info.library_id.map(|id| id.to_string()),
                episode_id: info.episode_id.map(|id| id.to_string()),
                movie_id: info.movie_id.map(|id| id.to_string()),
                album_id: info.album_id.map(|id| id.to_string()),
                audiobook_id: info.audiobook_id.map(|id| id.to_string()),
            }),
        })
    }

    /// Pause a usenet download
    async fn pause_usenet_download(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let download_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid download ID: {}", e)))?;

        // Verify ownership
        let existing = db
            .usenet_downloads()
            .get(download_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Download not found"))?;

        let user_id = Uuid::parse_str(&user.user_id)?;
        if existing.user_id != user_id {
            return Err(async_graphql::Error::new("Not authorized"));
        }

        db.usenet_downloads()
            .pause(download_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: true,
            error: None,
        })
    }

    /// Resume a paused usenet download
    async fn resume_usenet_download(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let download_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid download ID: {}", e)))?;

        // Verify ownership
        let existing = db
            .usenet_downloads()
            .get(download_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Download not found"))?;

        let user_id = Uuid::parse_str(&user.user_id)?;
        if existing.user_id != user_id {
            return Err(async_graphql::Error::new("Not authorized"));
        }

        db.usenet_downloads()
            .resume(download_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: true,
            error: None,
        })
    }

    /// Remove a usenet download
    async fn remove_usenet_download(
        &self,
        ctx: &Context<'_>,
        id: String,
        delete_files: Option<bool>,
    ) -> Result<MutationResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let download_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid download ID: {}", e)))?;

        // Verify ownership
        let existing = db
            .usenet_downloads()
            .get(download_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Download not found"))?;

        let user_id = Uuid::parse_str(&user.user_id)?;
        if existing.user_id != user_id {
            return Err(async_graphql::Error::new("Not authorized"));
        }

        // Remove from database (soft delete)
        db.usenet_downloads()
            .remove(download_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Delete files if requested
        if delete_files.unwrap_or(false) {
            if let Some(path) = existing.download_path {
                let _ = std::fs::remove_dir_all(&path);
            }
        }

        Ok(MutationResult {
            success: true,
            error: None,
        })
    }
}
