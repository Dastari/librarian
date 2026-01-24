//! Indexer database repository
//!
//! Handles CRUD operations for indexer configurations, credentials, and settings.
//! Uses runtime query validation to avoid requiring tables to exist at compile time.

use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite")]
use crate::db::sqlite_helpers::{
    bool_to_int, int_to_bool, str_to_datetime, str_to_uuid, uuid_to_str,
};

#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

/// Indexer configuration record from database
#[derive(Debug, Clone)]
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
    pub capabilities: Option<serde_json::Value>,
    /// Post-download action override (copy-only today; future source rules) - NULL uses library setting
    pub post_download_action: Option<String>,
    pub last_error: Option<String>,
    pub error_count: i32,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_error_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for IndexerConfigRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let enabled_int: i32 = row.try_get("enabled")?;
        let supports_search_int: Option<i32> = row.try_get("supports_search")?;
        let supports_tv_search_int: Option<i32> = row.try_get("supports_tv_search")?;
        let supports_movie_search_int: Option<i32> = row.try_get("supports_movie_search")?;
        let supports_music_search_int: Option<i32> = row.try_get("supports_music_search")?;
        let supports_book_search_int: Option<i32> = row.try_get("supports_book_search")?;
        let supports_imdb_search_int: Option<i32> = row.try_get("supports_imdb_search")?;
        let supports_tvdb_search_int: Option<i32> = row.try_get("supports_tvdb_search")?;
        let capabilities_str: Option<String> = row.try_get("capabilities")?;
        let last_success_at_str: Option<String> = row.try_get("last_success_at")?;
        let last_error_at_str: Option<String> = row.try_get("last_error_at")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            indexer_type: row.try_get("indexer_type")?,
            definition_id: row.try_get("definition_id")?,
            name: row.try_get("name")?,
            enabled: int_to_bool(enabled_int),
            priority: row.try_get("priority")?,
            site_url: row.try_get("site_url")?,
            supports_search: supports_search_int.map(int_to_bool),
            supports_tv_search: supports_tv_search_int.map(int_to_bool),
            supports_movie_search: supports_movie_search_int.map(int_to_bool),
            supports_music_search: supports_music_search_int.map(int_to_bool),
            supports_book_search: supports_book_search_int.map(int_to_bool),
            supports_imdb_search: supports_imdb_search_int.map(int_to_bool),
            supports_tvdb_search: supports_tvdb_search_int.map(int_to_bool),
            capabilities: capabilities_str
                .and_then(|s| serde_json::from_str(&s).ok()),
            post_download_action: row.try_get("post_download_action")?,
            last_error: row.try_get("last_error")?,
            error_count: row.try_get("error_count")?,
            last_success_at: last_success_at_str
                .map(|s| str_to_datetime(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            last_error_at: last_error_at_str
                .map(|s| str_to_datetime(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            created_at: str_to_datetime(&created_at_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_at_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// Indexer credential record from database
#[derive(Debug, Clone)]
pub struct IndexerCredentialRecord {
    pub id: Uuid,
    pub indexer_config_id: Uuid,
    pub credential_type: String,
    pub encrypted_value: String,
    pub nonce: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for IndexerCredentialRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let indexer_config_id_str: String = row.try_get("indexer_config_id")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            indexer_config_id: str_to_uuid(&indexer_config_id_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            credential_type: row.try_get("credential_type")?,
            encrypted_value: row.try_get("encrypted_value")?,
            nonce: row.try_get("nonce")?,
            created_at: str_to_datetime(&created_at_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_at_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// Indexer setting record from database
#[derive(Debug, Clone)]
pub struct IndexerSettingRecord {
    pub id: Uuid,
    pub indexer_config_id: Uuid,
    pub setting_key: String,
    pub setting_value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for IndexerSettingRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let indexer_config_id_str: String = row.try_get("indexer_config_id")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            indexer_config_id: str_to_uuid(&indexer_config_id_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            setting_key: row.try_get("setting_key")?,
            setting_value: row.try_get("setting_value")?,
            created_at: str_to_datetime(&created_at_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_at_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
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
    pool: DbPool,
}

impl IndexerRepository {
    /// Create a new repository instance
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========== Config CRUD ==========

    /// Create a new indexer configuration

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, data: CreateIndexerConfig) -> Result<IndexerConfigRecord> {
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);
        let user_id_str = uuid_to_str(data.user_id);

        sqlx::query(
            r#"
            INSERT INTO indexer_configs (
                id, user_id, indexer_type, definition_id, name, site_url,
                enabled, priority, error_count, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 0, 0, datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(&user_id_str)
        .bind(&data.indexer_type)
        .bind(&data.definition_id)
        .bind(&data.name)
        .bind(&data.site_url)
        .execute(&self.pool)
        .await?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve indexer config after insert"))
    }

    /// Get an indexer configuration by ID

    #[cfg(feature = "sqlite")]
    pub async fn get(&self, id: Uuid) -> Result<Option<IndexerConfigRecord>> {
        let record = sqlx::query_as::<_, IndexerConfigRecord>(
            r#"
            SELECT 
                id, user_id, indexer_type, definition_id, name, enabled, priority,
                site_url, supports_search, supports_tv_search, supports_movie_search,
                supports_music_search, supports_book_search, supports_imdb_search,
                supports_tvdb_search, capabilities, post_download_action,
                last_error, error_count, last_success_at, last_error_at,
                created_at, updated_at
            FROM indexer_configs
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// List all indexer configurations for a user

    #[cfg(feature = "sqlite")]
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<IndexerConfigRecord>> {
        let records = sqlx::query_as::<_, IndexerConfigRecord>(
            r#"
            SELECT 
                id, user_id, indexer_type, definition_id, name, enabled, priority,
                site_url, supports_search, supports_tv_search, supports_movie_search,
                supports_music_search, supports_book_search, supports_imdb_search,
                supports_tvdb_search, capabilities, post_download_action,
                last_error, error_count, last_success_at, last_error_at,
                created_at, updated_at
            FROM indexer_configs
            WHERE user_id = ?1
            ORDER BY priority DESC, name ASC
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List enabled indexer configurations for a user

    #[cfg(feature = "sqlite")]
    pub async fn list_enabled_by_user(&self, user_id: Uuid) -> Result<Vec<IndexerConfigRecord>> {
        let records = sqlx::query_as::<_, IndexerConfigRecord>(
            r#"
            SELECT 
                id, user_id, indexer_type, definition_id, name, enabled, priority,
                site_url, supports_search, supports_tv_search, supports_movie_search,
                supports_music_search, supports_book_search, supports_imdb_search,
                supports_tvdb_search, capabilities, post_download_action,
                last_error, error_count, last_success_at, last_error_at,
                created_at, updated_at
            FROM indexer_configs
            WHERE user_id = ?1 AND enabled = 1
            ORDER BY priority DESC, name ASC
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Update an indexer configuration

    #[cfg(feature = "sqlite")]
    pub async fn update(
        &self,
        id: Uuid,
        data: UpdateIndexerConfig,
    ) -> Result<Option<IndexerConfigRecord>> {
        let id_str = uuid_to_str(id);

        // Convert optional booleans to optional integers
        let enabled_int = data.enabled.map(bool_to_int);
        let supports_search_int = data.supports_search.map(bool_to_int);
        let supports_tv_search_int = data.supports_tv_search.map(bool_to_int);
        let supports_movie_search_int = data.supports_movie_search.map(bool_to_int);
        let supports_music_search_int = data.supports_music_search.map(bool_to_int);
        let supports_book_search_int = data.supports_book_search.map(bool_to_int);
        let supports_imdb_search_int = data.supports_imdb_search.map(bool_to_int);
        let supports_tvdb_search_int = data.supports_tvdb_search.map(bool_to_int);

        // Convert capabilities to JSON string
        let capabilities_str = data
            .capabilities
            .as_ref()
            .map(|c| serde_json::to_string(c).unwrap_or_else(|_| "null".to_string()));

        // Convert datetimes to strings
        let last_success_at_str = data.last_success_at.map(|dt| dt.to_rfc3339());
        let last_error_at_str = data.last_error_at.map(|dt| dt.to_rfc3339());

        sqlx::query(
            r#"
            UPDATE indexer_configs
            SET
                name = COALESCE(?2, name),
                enabled = COALESCE(?3, enabled),
                priority = COALESCE(?4, priority),
                site_url = COALESCE(?5, site_url),
                supports_search = COALESCE(?6, supports_search),
                supports_tv_search = COALESCE(?7, supports_tv_search),
                supports_movie_search = COALESCE(?8, supports_movie_search),
                supports_music_search = COALESCE(?9, supports_music_search),
                supports_book_search = COALESCE(?10, supports_book_search),
                supports_imdb_search = COALESCE(?11, supports_imdb_search),
                supports_tvdb_search = COALESCE(?12, supports_tvdb_search),
                capabilities = COALESCE(?13, capabilities),
                last_error = COALESCE(?14, last_error),
                error_count = COALESCE(?15, error_count),
                last_success_at = COALESCE(?16, last_success_at),
                last_error_at = COALESCE(?17, last_error_at),
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(&id_str)
        .bind(&data.name)
        .bind(enabled_int)
        .bind(data.priority)
        .bind(&data.site_url)
        .bind(supports_search_int)
        .bind(supports_tv_search_int)
        .bind(supports_movie_search_int)
        .bind(supports_music_search_int)
        .bind(supports_book_search_int)
        .bind(supports_imdb_search_int)
        .bind(supports_tvdb_search_int)
        .bind(&capabilities_str)
        .bind(&data.last_error)
        .bind(data.error_count)
        .bind(&last_success_at_str)
        .bind(&last_error_at_str)
        .execute(&self.pool)
        .await?;

        self.get(id).await
    }

    /// Delete an indexer configuration

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM indexer_configs WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Record a successful search

    #[cfg(feature = "sqlite")]
    pub async fn record_success(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE indexer_configs
            SET 
                error_count = 0,
                last_error = NULL,
                last_success_at = datetime('now'),
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Record a failed search

    #[cfg(feature = "sqlite")]
    pub async fn record_error(&self, id: Uuid, error: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE indexer_configs
            SET 
                error_count = error_count + 1,
                last_error = ?2,
                last_error_at = datetime('now'),
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========== Credentials ==========

    /// Get all credentials for an indexer

    #[cfg(feature = "sqlite")]
    pub async fn get_credentials(&self, indexer_id: Uuid) -> Result<Vec<IndexerCredentialRecord>> {
        let records = sqlx::query_as::<_, IndexerCredentialRecord>(
            r#"
            SELECT id, indexer_config_id, credential_type, encrypted_value, nonce, created_at, updated_at
            FROM indexer_credentials
            WHERE indexer_config_id = ?1
            "#,
        )
        .bind(uuid_to_str(indexer_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Upsert a credential (insert or update)

    #[cfg(feature = "sqlite")]
    pub async fn upsert_credential(
        &self,
        indexer_id: Uuid,
        cred: UpsertCredential,
    ) -> Result<IndexerCredentialRecord> {
        let indexer_id_str = uuid_to_str(indexer_id);

        // Check if exists
        let existing: Option<String> = sqlx::query_scalar(
            r#"
            SELECT id FROM indexer_credentials 
            WHERE indexer_config_id = ?1 AND credential_type = ?2
            "#,
        )
        .bind(&indexer_id_str)
        .bind(&cred.credential_type)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(existing_id) = existing {
            // Update existing
            sqlx::query(
                r#"
                UPDATE indexer_credentials
                SET encrypted_value = ?2, nonce = ?3, updated_at = datetime('now')
                WHERE id = ?1
                "#,
            )
            .bind(&existing_id)
            .bind(&cred.encrypted_value)
            .bind(&cred.nonce)
            .execute(&self.pool)
            .await?;

            let id = str_to_uuid(&existing_id)?;
            return self
                .get_credential_by_id(id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Failed to retrieve credential after update"));
        }

        // Insert new
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            INSERT INTO indexer_credentials (id, indexer_config_id, credential_type, encrypted_value, nonce, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(&indexer_id_str)
        .bind(&cred.credential_type)
        .bind(&cred.encrypted_value)
        .bind(&cred.nonce)
        .execute(&self.pool)
        .await?;

        self.get_credential_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve credential after insert"))
    }

    /// Get a credential by ID (helper for SQLite)
    #[cfg(feature = "sqlite")]
    async fn get_credential_by_id(&self, id: Uuid) -> Result<Option<IndexerCredentialRecord>> {
        let record = sqlx::query_as::<_, IndexerCredentialRecord>(
            r#"
            SELECT id, indexer_config_id, credential_type, encrypted_value, nonce, created_at, updated_at
            FROM indexer_credentials
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete a specific credential

    #[cfg(feature = "sqlite")]
    pub async fn delete_credential(&self, indexer_id: Uuid, credential_type: &str) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM indexer_credentials WHERE indexer_config_id = ?1 AND credential_type = ?2",
        )
        .bind(uuid_to_str(indexer_id))
        .bind(credential_type)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ========== Settings ==========

    /// Get all settings for an indexer

    #[cfg(feature = "sqlite")]
    pub async fn get_settings(&self, indexer_id: Uuid) -> Result<Vec<IndexerSettingRecord>> {
        let records = sqlx::query_as::<_, IndexerSettingRecord>(
            r#"
            SELECT id, indexer_config_id, setting_key, setting_value, created_at, updated_at
            FROM indexer_settings
            WHERE indexer_config_id = ?1
            "#,
        )
        .bind(uuid_to_str(indexer_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Upsert a setting (insert or update)

    #[cfg(feature = "sqlite")]
    pub async fn upsert_setting(
        &self,
        indexer_id: Uuid,
        key: &str,
        value: &str,
    ) -> Result<IndexerSettingRecord> {
        let indexer_id_str = uuid_to_str(indexer_id);

        // Check if exists
        let existing: Option<String> = sqlx::query_scalar(
            r#"
            SELECT id FROM indexer_settings 
            WHERE indexer_config_id = ?1 AND setting_key = ?2
            "#,
        )
        .bind(&indexer_id_str)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(existing_id) = existing {
            // Update existing
            sqlx::query(
                r#"
                UPDATE indexer_settings
                SET setting_value = ?2, updated_at = datetime('now')
                WHERE id = ?1
                "#,
            )
            .bind(&existing_id)
            .bind(value)
            .execute(&self.pool)
            .await?;

            let id = str_to_uuid(&existing_id)?;
            return self
                .get_setting_by_id(id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Failed to retrieve setting after update"));
        }

        // Insert new
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            INSERT INTO indexer_settings (id, indexer_config_id, setting_key, setting_value, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(&indexer_id_str)
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;

        self.get_setting_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve setting after insert"))
    }

    /// Get a setting by ID (helper for SQLite)
    #[cfg(feature = "sqlite")]
    async fn get_setting_by_id(&self, id: Uuid) -> Result<Option<IndexerSettingRecord>> {
        let record = sqlx::query_as::<_, IndexerSettingRecord>(
            r#"
            SELECT id, indexer_config_id, setting_key, setting_value, created_at, updated_at
            FROM indexer_settings
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete a setting

    #[cfg(feature = "sqlite")]
    pub async fn delete_setting(&self, indexer_id: Uuid, key: &str) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM indexer_settings WHERE indexer_config_id = ?1 AND setting_key = ?2",
        )
        .bind(uuid_to_str(indexer_id))
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

    #[cfg(feature = "sqlite")]
    pub async fn cleanup_expired_cache(&self) -> Result<u64> {
        let result =
            sqlx::query("DELETE FROM indexer_search_cache WHERE expires_at < datetime('now')")
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected())
    }
}
