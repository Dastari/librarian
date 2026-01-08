//! Application settings database operations

use anyhow::Result;
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// A setting record in the database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SettingRecord {
    pub id: Uuid,
    pub key: String,
    pub value: JsonValue,
    pub description: Option<String>,
    pub category: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Settings repository for database operations
pub struct SettingsRepository {
    pool: PgPool,
}

impl SettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a setting by key
    pub async fn get(&self, key: &str) -> Result<Option<SettingRecord>> {
        let record = sqlx::query_as::<_, SettingRecord>(
            "SELECT * FROM app_settings WHERE key = $1",
        )
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
    pub async fn get_or_default<T: serde::de::DeserializeOwned>(&self, key: &str, default: T) -> Result<T> {
        match self.get_value(key).await? {
            Some(v) => Ok(v),
            None => Ok(default),
        }
    }

    /// Get all settings in a category
    pub async fn list_by_category(&self, category: &str) -> Result<Vec<SettingRecord>> {
        let records = sqlx::query_as::<_, SettingRecord>(
            "SELECT * FROM app_settings WHERE category = $1 ORDER BY key",
        )
        .bind(category)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get all settings
    pub async fn list_all(&self) -> Result<Vec<SettingRecord>> {
        let records = sqlx::query_as::<_, SettingRecord>(
            "SELECT * FROM app_settings ORDER BY category, key",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Set a setting value
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

    /// Set a setting value with category
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

    /// Delete a setting
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM app_settings WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
