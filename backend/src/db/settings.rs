//! Application settings database operations

use anyhow::Result;
use serde_json::Value as JsonValue;
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "postgres")]
type DbPool = PgPool;
#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

/// A setting record in the database
#[derive(Debug, Clone)]
pub struct SettingRecord {
    pub id: Uuid,
    pub key: String,
    pub value: JsonValue,
    pub description: Option<String>,
    pub category: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "postgres")]
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for SettingRecord {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            key: row.try_get("key")?,
            value: row.try_get("value")?,
            description: row.try_get("description")?,
            category: row.try_get("category")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for SettingRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use crate::db::sqlite_helpers::{str_to_uuid, str_to_datetime};
        
        let id_str: String = row.try_get("id")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;
        let value_str: String = row.try_get("value")?;
        
        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            key: row.try_get("key")?,
            value: serde_json::from_str(&value_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            description: row.try_get("description")?,
            category: row.try_get("category")?,
            created_at: str_to_datetime(&created_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// Settings repository for database operations
pub struct SettingsRepository {
    pool: DbPool,
}

impl SettingsRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get a setting by key
    #[cfg(feature = "postgres")]
    pub async fn get(&self, key: &str) -> Result<Option<SettingRecord>> {
        let record =
            sqlx::query_as::<_, SettingRecord>("SELECT * FROM app_settings WHERE key = $1")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn get(&self, key: &str) -> Result<Option<SettingRecord>> {
        let record =
            sqlx::query_as::<_, SettingRecord>("SELECT * FROM app_settings WHERE key = ?1")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;

        Ok(record)
    }

    /// Get a setting value as a specific type
    pub async fn get_value<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let record = self.get(key).await?;
        match record {
            Some(r) => Ok(Some(serde_json::from_value(r.value)?)),
            None => Ok(None),
        }
    }

    /// Get a setting value with a default
    pub async fn get_or_default<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
        default: T,
    ) -> Result<T> {
        match self.get_value(key).await? {
            Some(v) => Ok(v),
            None => Ok(default),
        }
    }

    /// Get all settings in a category
    #[cfg(feature = "postgres")]
    pub async fn list_by_category(&self, category: &str) -> Result<Vec<SettingRecord>> {
        let records = sqlx::query_as::<_, SettingRecord>(
            "SELECT * FROM app_settings WHERE category = $1 ORDER BY key",
        )
        .bind(category)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    #[cfg(feature = "sqlite")]
    pub async fn list_by_category(&self, category: &str) -> Result<Vec<SettingRecord>> {
        let records = sqlx::query_as::<_, SettingRecord>(
            "SELECT * FROM app_settings WHERE category = ?1 ORDER BY key",
        )
        .bind(category)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get all settings
    pub async fn list_all(&self) -> Result<Vec<SettingRecord>> {
        let records =
            sqlx::query_as::<_, SettingRecord>("SELECT * FROM app_settings ORDER BY category, key")
                .fetch_all(&self.pool)
                .await?;

        Ok(records)
    }

    /// Set a setting value
    #[cfg(feature = "postgres")]
    pub async fn set<T: serde::Serialize>(&self, key: &str, value: T) -> Result<SettingRecord> {
        let json_value = serde_json::to_value(value)?;

        let record = sqlx::query_as::<_, SettingRecord>(
            r#"
            INSERT INTO app_settings (key, value, category)
            VALUES ($1, $2, 'general')
            ON CONFLICT (key) DO UPDATE SET value = $2
            RETURNING *
            "#,
        )
        .bind(key)
        .bind(json_value)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn set<T: serde::Serialize>(&self, key: &str, value: T) -> Result<SettingRecord> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let json_value = serde_json::to_string(&serde_json::to_value(value)?)?;
        let id = uuid_to_str(Uuid::new_v4());

        sqlx::query(
            r#"
            INSERT INTO app_settings (id, key, value, category, created_at, updated_at)
            VALUES (?1, ?2, ?3, 'general', datetime('now'), datetime('now'))
            ON CONFLICT (key) DO UPDATE SET 
                value = ?3,
                updated_at = datetime('now')
            "#,
        )
        .bind(&id)
        .bind(key)
        .bind(&json_value)
        .execute(&self.pool)
        .await?;

        // Fetch the record back
        self.get(key).await?.ok_or_else(|| anyhow::anyhow!("Failed to retrieve setting after insert"))
    }

    /// Set a setting value with category
    #[cfg(feature = "postgres")]
    pub async fn set_with_category<T: serde::Serialize>(
        &self,
        key: &str,
        value: T,
        category: &str,
        description: Option<&str>,
    ) -> Result<SettingRecord> {
        let json_value = serde_json::to_value(value)?;

        let record = sqlx::query_as::<_, SettingRecord>(
            r#"
            INSERT INTO app_settings (key, value, category, description)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (key) DO UPDATE SET 
                value = $2,
                category = $3,
                description = COALESCE($4, app_settings.description)
            RETURNING *
            "#,
        )
        .bind(key)
        .bind(json_value)
        .bind(category)
        .bind(description)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn set_with_category<T: serde::Serialize>(
        &self,
        key: &str,
        value: T,
        category: &str,
        description: Option<&str>,
    ) -> Result<SettingRecord> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let json_value = serde_json::to_string(&serde_json::to_value(value)?)?;
        let id = uuid_to_str(Uuid::new_v4());

        sqlx::query(
            r#"
            INSERT INTO app_settings (id, key, value, category, description, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'), datetime('now'))
            ON CONFLICT (key) DO UPDATE SET 
                value = ?3,
                category = ?4,
                description = COALESCE(?5, app_settings.description),
                updated_at = datetime('now')
            "#,
        )
        .bind(&id)
        .bind(key)
        .bind(&json_value)
        .bind(category)
        .bind(description)
        .execute(&self.pool)
        .await?;

        self.get(key).await?.ok_or_else(|| anyhow::anyhow!("Failed to retrieve setting after insert"))
    }

    /// Delete a setting
    #[cfg(feature = "postgres")]
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM app_settings WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM app_settings WHERE key = ?1")
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get the encryption key for indexer credentials
    ///
    /// Uses the JWT_SECRET for encryption, which is automatically generated
    /// and persisted on first startup. This simplifies configuration by
    /// using a single secret for both authentication and credential encryption.
    pub async fn get_or_create_indexer_encryption_key(&self) -> Result<String> {
        // Use JWT_SECRET for indexer credential encryption
        // This is set during startup by initialize_jwt_secret()
        self.get_value::<String>("jwt_secret")
            .await?
            .ok_or_else(|| anyhow::anyhow!("JWT secret not found - this should be auto-generated on startup"))
    }
}
