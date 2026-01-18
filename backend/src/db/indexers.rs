//! Indexer database repository
//!
//! Handles CRUD operations for indexer configurations, credentials, and settings.
//! Uses runtime query validation to avoid requiring tables to exist at compile time.

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Indexer configuration record from database
#[derive(Debug, Clone, FromRow)]
pub struct IndexerConfigRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub indexer_type: String,
    pub definition_id: Option<String>,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,
    pub site_url: Option<String>,
    pub supports_search: Option<bool>,
    pub supports_tv_search: Option<bool>,
    pub supports_movie_search: Option<bool>,
    pub supports_music_search: Option<bool>,
    pub supports_book_search: Option<bool>,
    pub supports_imdb_search: Option<bool>,
    pub supports_tvdb_search: Option<bool>,
    pub capabilities: Option<sqlx::types::Json<serde_json::Value>>,
    pub last_error: Option<String>,
    pub error_count: i32,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_error_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Indexer credential record from database
#[derive(Debug, Clone, FromRow)]
pub struct IndexerCredentialRecord {
    pub id: Uuid,
    pub indexer_config_id: Uuid,
    pub credential_type: String,
    pub encrypted_value: String,
    pub nonce: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Indexer setting record from database
#[derive(Debug, Clone, FromRow)]
pub struct IndexerSettingRecord {
    pub id: Uuid,
    pub indexer_config_id: Uuid,
    pub setting_key: String,
    pub setting_value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a new indexer configuration
#[derive(Debug, Clone)]
pub struct CreateIndexerConfig {
    pub user_id: Uuid,
    pub indexer_type: String,
    pub definition_id: Option<String>,
    pub name: String,
    pub site_url: Option<String>,
}

/// Data for updating an indexer configuration
#[derive(Debug, Clone, Default)]
pub struct UpdateIndexerConfig {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
    pub site_url: Option<String>,
    pub supports_search: Option<bool>,
    pub supports_tv_search: Option<bool>,
    pub supports_movie_search: Option<bool>,
    pub supports_music_search: Option<bool>,
    pub supports_book_search: Option<bool>,
    pub supports_imdb_search: Option<bool>,
    pub supports_tvdb_search: Option<bool>,
    pub capabilities: Option<serde_json::Value>,
    pub last_error: Option<String>,
    pub error_count: Option<i32>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_error_at: Option<DateTime<Utc>>,
}

/// Data for creating/updating a credential
#[derive(Debug, Clone)]
pub struct UpsertCredential {
    pub credential_type: String,
    pub encrypted_value: String,
    pub nonce: String,
}

/// Indexer database repository
pub struct IndexerRepository {
    pool: PgPool,
}

impl IndexerRepository {
    /// Create a new repository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ========== Config CRUD ==========

    /// Create a new indexer configuration
    pub async fn create(&self, data: CreateIndexerConfig) -> Result<IndexerConfigRecord> {
        let record = sqlx::query_as::<_, IndexerConfigRecord>(
            r#"
            INSERT INTO indexer_configs (
                user_id, indexer_type, definition_id, name, site_url
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING 
                id, user_id, indexer_type, definition_id, name, enabled, priority,
                site_url, supports_search, supports_tv_search, supports_movie_search,
                supports_music_search, supports_book_search, supports_imdb_search,
                supports_tvdb_search, capabilities,
                last_error, error_count, last_success_at, last_error_at,
                created_at, updated_at
            "#,
        )
        .bind(data.user_id)
        .bind(&data.indexer_type)
        .bind(&data.definition_id)
        .bind(&data.name)
        .bind(&data.site_url)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get an indexer configuration by ID
    pub async fn get(&self, id: Uuid) -> Result<Option<IndexerConfigRecord>> {
        let record = sqlx::query_as::<_, IndexerConfigRecord>(
            r#"
            SELECT 
                id, user_id, indexer_type, definition_id, name, enabled, priority,
                site_url, supports_search, supports_tv_search, supports_movie_search,
                supports_music_search, supports_book_search, supports_imdb_search,
                supports_tvdb_search, capabilities,
                last_error, error_count, last_success_at, last_error_at,
                created_at, updated_at
            FROM indexer_configs
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// List all indexer configurations for a user
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<IndexerConfigRecord>> {
        let records = sqlx::query_as::<_, IndexerConfigRecord>(
            r#"
            SELECT 
                id, user_id, indexer_type, definition_id, name, enabled, priority,
                site_url, supports_search, supports_tv_search, supports_movie_search,
                supports_music_search, supports_book_search, supports_imdb_search,
                supports_tvdb_search, capabilities,
                last_error, error_count, last_success_at, last_error_at,
                created_at, updated_at
            FROM indexer_configs
            WHERE user_id = $1
            ORDER BY priority DESC, name ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List enabled indexer configurations for a user
    pub async fn list_enabled_by_user(&self, user_id: Uuid) -> Result<Vec<IndexerConfigRecord>> {
        let records = sqlx::query_as::<_, IndexerConfigRecord>(
            r#"
            SELECT 
                id, user_id, indexer_type, definition_id, name, enabled, priority,
                site_url, supports_search, supports_tv_search, supports_movie_search,
                supports_music_search, supports_book_search, supports_imdb_search,
                supports_tvdb_search, capabilities,
                last_error, error_count, last_success_at, last_error_at,
                created_at, updated_at
            FROM indexer_configs
            WHERE user_id = $1 AND enabled = true
            ORDER BY priority DESC, name ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Update an indexer configuration
    pub async fn update(
        &self,
        id: Uuid,
        data: UpdateIndexerConfig,
    ) -> Result<Option<IndexerConfigRecord>> {
        let record = sqlx::query_as::<_, IndexerConfigRecord>(
            r#"
            UPDATE indexer_configs
            SET
                name = COALESCE($2, name),
                enabled = COALESCE($3, enabled),
                priority = COALESCE($4, priority),
                site_url = COALESCE($5, site_url),
                supports_search = COALESCE($6, supports_search),
                supports_tv_search = COALESCE($7, supports_tv_search),
                supports_movie_search = COALESCE($8, supports_movie_search),
                supports_music_search = COALESCE($9, supports_music_search),
                supports_book_search = COALESCE($10, supports_book_search),
                supports_imdb_search = COALESCE($11, supports_imdb_search),
                supports_tvdb_search = COALESCE($12, supports_tvdb_search),
                capabilities = COALESCE($13, capabilities),
                last_error = COALESCE($14, last_error),
                error_count = COALESCE($15, error_count),
                last_success_at = COALESCE($16, last_success_at),
                last_error_at = COALESCE($17, last_error_at),
                updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, user_id, indexer_type, definition_id, name, enabled, priority,
                site_url, supports_search, supports_tv_search, supports_movie_search,
                supports_music_search, supports_book_search, supports_imdb_search,
                supports_tvdb_search, capabilities,
                last_error, error_count, last_success_at, last_error_at,
                created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(data.enabled)
        .bind(data.priority)
        .bind(&data.site_url)
        .bind(data.supports_search)
        .bind(data.supports_tv_search)
        .bind(data.supports_movie_search)
        .bind(data.supports_music_search)
        .bind(data.supports_book_search)
        .bind(data.supports_imdb_search)
        .bind(data.supports_tvdb_search)
        .bind(&data.capabilities)
        .bind(&data.last_error)
        .bind(data.error_count)
        .bind(data.last_success_at)
        .bind(data.last_error_at)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete an indexer configuration
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM indexer_configs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Record a successful search
    pub async fn record_success(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE indexer_configs
            SET 
                error_count = 0,
                last_error = NULL,
                last_success_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Record a failed search
    pub async fn record_error(&self, id: Uuid, error: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE indexer_configs
            SET 
                error_count = error_count + 1,
                last_error = $2,
                last_error_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========== Credentials ==========

    /// Get all credentials for an indexer
    pub async fn get_credentials(&self, indexer_id: Uuid) -> Result<Vec<IndexerCredentialRecord>> {
        let records = sqlx::query_as::<_, IndexerCredentialRecord>(
            r#"
            SELECT id, indexer_config_id, credential_type, encrypted_value, nonce, created_at, updated_at
            FROM indexer_credentials
            WHERE indexer_config_id = $1
            "#,
        )
        .bind(indexer_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Upsert a credential (insert or update)
    pub async fn upsert_credential(
        &self,
        indexer_id: Uuid,
        cred: UpsertCredential,
    ) -> Result<IndexerCredentialRecord> {
        let record = sqlx::query_as::<_, IndexerCredentialRecord>(
            r#"
            INSERT INTO indexer_credentials (indexer_config_id, credential_type, encrypted_value, nonce)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (indexer_config_id, credential_type)
            DO UPDATE SET
                encrypted_value = EXCLUDED.encrypted_value,
                nonce = EXCLUDED.nonce,
                updated_at = NOW()
            RETURNING id, indexer_config_id, credential_type, encrypted_value, nonce, created_at, updated_at
            "#,
        )
        .bind(indexer_id)
        .bind(&cred.credential_type)
        .bind(&cred.encrypted_value)
        .bind(&cred.nonce)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete a specific credential
    pub async fn delete_credential(&self, indexer_id: Uuid, credential_type: &str) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM indexer_credentials WHERE indexer_config_id = $1 AND credential_type = $2",
        )
        .bind(indexer_id)
        .bind(credential_type)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ========== Settings ==========

    /// Get all settings for an indexer
    pub async fn get_settings(&self, indexer_id: Uuid) -> Result<Vec<IndexerSettingRecord>> {
        let records = sqlx::query_as::<_, IndexerSettingRecord>(
            r#"
            SELECT id, indexer_config_id, setting_key, setting_value, created_at, updated_at
            FROM indexer_settings
            WHERE indexer_config_id = $1
            "#,
        )
        .bind(indexer_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Upsert a setting (insert or update)
    pub async fn upsert_setting(
        &self,
        indexer_id: Uuid,
        key: &str,
        value: &str,
    ) -> Result<IndexerSettingRecord> {
        let record = sqlx::query_as::<_, IndexerSettingRecord>(
            r#"
            INSERT INTO indexer_settings (indexer_config_id, setting_key, setting_value)
            VALUES ($1, $2, $3)
            ON CONFLICT (indexer_config_id, setting_key)
            DO UPDATE SET
                setting_value = EXCLUDED.setting_value,
                updated_at = NOW()
            RETURNING id, indexer_config_id, setting_key, setting_value, created_at, updated_at
            "#,
        )
        .bind(indexer_id)
        .bind(key)
        .bind(value)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete a setting
    pub async fn delete_setting(&self, indexer_id: Uuid, key: &str) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM indexer_settings WHERE indexer_config_id = $1 AND setting_key = $2",
        )
        .bind(indexer_id)
        .bind(key)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Set multiple settings at once
    pub async fn set_settings(
        &self,
        indexer_id: Uuid,
        settings: &[(String, String)],
    ) -> Result<()> {
        for (key, value) in settings {
            self.upsert_setting(indexer_id, key, value).await?;
        }
        Ok(())
    }

    // ========== Cache ==========

    /// Clean up expired cache entries
    pub async fn cleanup_expired_cache(&self) -> Result<u64> {
        let result = sqlx::query("DELETE FROM indexer_search_cache WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
