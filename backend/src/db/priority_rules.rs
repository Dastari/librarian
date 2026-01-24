//! Source priority rules database repository
//!
//! Handles CRUD operations for source priority rules that determine
//! which sources to search first for different library types.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite")]
use crate::db::sqlite_helpers::{
    bool_to_int, from_json, int_to_bool, str_to_datetime, str_to_uuid, to_json, uuid_to_str,
};

#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

/// Source type enum for priority ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    TorrentIndexer,
    UsenetIndexer,
    // Future: Irc, Ftp, Http, Manual
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::TorrentIndexer => write!(f, "torrent_indexer"),
            SourceType::UsenetIndexer => write!(f, "usenet_indexer"),
        }
    }
}

impl std::str::FromStr for SourceType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "torrent_indexer" | "torrentindexer" => Ok(SourceType::TorrentIndexer),
            "usenet_indexer" | "usenetindexer" => Ok(SourceType::UsenetIndexer),
            _ => Err(anyhow::anyhow!("Unknown source type: {}", s)),
        }
    }
}

/// A source reference in the priority order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRef {
    pub source_type: SourceType,
    pub id: String,
}

/// Source priority rule record from database
#[derive(Debug, Clone)]
pub struct PriorityRuleRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub library_type: Option<String>,
    pub library_id: Option<Uuid>,
    pub priority_order: Vec<SourceRef>,
    pub search_all_sources: bool,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for PriorityRuleRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let library_id_str: Option<String> = row.try_get("library_id")?;
        let priority_order_json: String = row.try_get("priority_order")?;
        let search_all_sources_int: i32 = row.try_get("search_all_sources")?;
        let enabled_int: i32 = row.try_get("enabled")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            library_type: row.try_get("library_type")?,
            library_id: library_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            priority_order: from_json(&priority_order_json)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            search_all_sources: int_to_bool(search_all_sources_int),
            enabled: int_to_bool(enabled_int),
            created_at: str_to_datetime(&created_at_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_at_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// Data for creating a new priority rule
#[derive(Debug, Clone)]
pub struct CreatePriorityRule {
    pub user_id: Uuid,
    pub library_type: Option<String>,
    pub library_id: Option<Uuid>,
    pub priority_order: Vec<SourceRef>,
    pub search_all_sources: bool,
}

/// Data for updating a priority rule
#[derive(Debug, Clone, Default)]
pub struct UpdatePriorityRule {
    pub priority_order: Option<Vec<SourceRef>>,
    pub search_all_sources: Option<bool>,
    pub enabled: Option<bool>,
}

/// Priority rules database repository
pub struct PriorityRulesRepository {
    pool: DbPool,
}

impl PriorityRulesRepository {
    /// Create a new repository instance
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get a priority rule by ID

    #[cfg(feature = "sqlite")]
    pub async fn get(&self, id: Uuid) -> Result<Option<PriorityRuleRecord>> {
        let record = sqlx::query_as::<_, PriorityRuleRecord>(
            r#"
            SELECT id, user_id, library_type, library_id, priority_order,
                   search_all_sources, enabled, created_at, updated_at
            FROM source_priority_rules
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get all priority rules for a user

    #[cfg(feature = "sqlite")]
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<PriorityRuleRecord>> {
        let records = sqlx::query_as::<_, PriorityRuleRecord>(
            r#"
            SELECT id, user_id, library_type, library_id, priority_order,
                   search_all_sources, enabled, created_at, updated_at
            FROM source_priority_rules
            WHERE user_id = ?1
            ORDER BY 
                CASE WHEN library_id IS NOT NULL THEN 0
                     WHEN library_type IS NOT NULL THEN 1
                     ELSE 2 END,
                library_type
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get a user's default priority rule (no library_type or library_id)

    #[cfg(feature = "sqlite")]
    pub async fn get_user_default(&self, user_id: Uuid) -> Result<Option<PriorityRuleRecord>> {
        let record = sqlx::query_as::<_, PriorityRuleRecord>(
            r#"
            SELECT id, user_id, library_type, library_id, priority_order,
                   search_all_sources, enabled, created_at, updated_at
            FROM source_priority_rules
            WHERE user_id = ?1
              AND library_type IS NULL
              AND library_id IS NULL
              AND enabled = 1
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a priority rule by library type (e.g., "tv", "movies")

    #[cfg(feature = "sqlite")]
    pub async fn get_by_library_type(
        &self,
        user_id: Uuid,
        library_type: &str,
    ) -> Result<Option<PriorityRuleRecord>> {
        let record = sqlx::query_as::<_, PriorityRuleRecord>(
            r#"
            SELECT id, user_id, library_type, library_id, priority_order,
                   search_all_sources, enabled, created_at, updated_at
            FROM source_priority_rules
            WHERE user_id = ?1
              AND library_type = ?2
              AND library_id IS NULL
              AND enabled = 1
            "#,
        )
        .bind(uuid_to_str(user_id))
        .bind(library_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a priority rule by specific library

    #[cfg(feature = "sqlite")]
    pub async fn get_by_library(
        &self,
        user_id: Uuid,
        library_id: Uuid,
    ) -> Result<Option<PriorityRuleRecord>> {
        let record = sqlx::query_as::<_, PriorityRuleRecord>(
            r#"
            SELECT id, user_id, library_type, library_id, priority_order,
                   search_all_sources, enabled, created_at, updated_at
            FROM source_priority_rules
            WHERE user_id = ?1
              AND library_id = ?2
              AND enabled = 1
            "#,
        )
        .bind(uuid_to_str(user_id))
        .bind(uuid_to_str(library_id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get the most specific applicable rule for a given context
    ///
    /// Order of precedence:
    /// 1. Library-specific rule (library_id set)
    /// 2. Library-type rule (library_type set, library_id NULL)
    /// 3. User default rule (both NULL)
    pub async fn get_applicable_rule(
        &self,
        user_id: Uuid,
        library_type: Option<&str>,
        library_id: Option<Uuid>,
    ) -> Result<Option<PriorityRuleRecord>> {
        // Try library-specific first
        if let Some(lib_id) = library_id {
            if let Some(rule) = self.get_by_library(user_id, lib_id).await? {
                return Ok(Some(rule));
            }
        }

        // Try library-type
        if let Some(lib_type) = library_type {
            if let Some(rule) = self.get_by_library_type(user_id, lib_type).await? {
                return Ok(Some(rule));
            }
        }

        // Fall back to user default
        self.get_user_default(user_id).await
    }

    /// Create a new priority rule

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, data: CreatePriorityRule) -> Result<PriorityRuleRecord> {
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);
        let user_id_str = uuid_to_str(data.user_id);
        let library_id_str = data.library_id.map(uuid_to_str);
        let priority_order_json = to_json(&data.priority_order);

        sqlx::query(
            r#"
            INSERT INTO source_priority_rules (
                id, user_id, library_type, library_id, priority_order, 
                search_all_sources, enabled, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(&user_id_str)
        .bind(&data.library_type)
        .bind(&library_id_str)
        .bind(&priority_order_json)
        .bind(bool_to_int(data.search_all_sources))
        .execute(&self.pool)
        .await?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to create priority rule"))
    }

    /// Update a priority rule

    #[cfg(feature = "sqlite")]
    pub async fn update(&self, id: Uuid, data: UpdatePriorityRule) -> Result<PriorityRuleRecord> {
        // Build dynamic update query
        let mut set_clauses = Vec::new();
        let mut param_idx = 2; // ?1 is the ID

        if data.priority_order.is_some() {
            set_clauses.push(format!("priority_order = ?{}", param_idx));
            param_idx += 1;
        }
        if data.search_all_sources.is_some() {
            set_clauses.push(format!("search_all_sources = ?{}", param_idx));
            param_idx += 1;
        }
        if data.enabled.is_some() {
            set_clauses.push(format!("enabled = ?{}", param_idx));
            // param_idx += 1;
        }

        if set_clauses.is_empty() {
            // Nothing to update, just return current record
            return self.get(id).await?.ok_or_else(|| anyhow::anyhow!("Rule not found"));
        }

        set_clauses.push("updated_at = datetime('now')".to_string());

        let query = format!(
            r#"
            UPDATE source_priority_rules
            SET {}
            WHERE id = ?1
            "#,
            set_clauses.join(", ")
        );

        let id_str = uuid_to_str(id);
        let mut query_builder = sqlx::query(&query).bind(&id_str);

        if let Some(ref order) = data.priority_order {
            query_builder = query_builder.bind(to_json(order));
        }
        if let Some(search_all) = data.search_all_sources {
            query_builder = query_builder.bind(bool_to_int(search_all));
        }
        if let Some(enabled) = data.enabled {
            query_builder = query_builder.bind(bool_to_int(enabled));
        }

        query_builder.execute(&self.pool).await?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Rule not found after update"))
    }

    /// Upsert a priority rule (create or update based on scope)

    #[cfg(feature = "sqlite")]
    pub async fn upsert(&self, data: CreatePriorityRule) -> Result<PriorityRuleRecord> {
        let user_id_str = uuid_to_str(data.user_id);
        let library_id_str = data.library_id.map(uuid_to_str);
        let priority_order_json = to_json(&data.priority_order);

        // Check if rule exists for this scope
        let existing = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM source_priority_rules
            WHERE user_id = ?1
              AND COALESCE(library_type, '') = COALESCE(?2, '')
              AND COALESCE(library_id, '') = COALESCE(?3, '')
            "#,
        )
        .bind(&user_id_str)
        .bind(&data.library_type)
        .bind(&library_id_str)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(existing_id) = existing {
            // Update existing
            sqlx::query(
                r#"
                UPDATE source_priority_rules SET
                    priority_order = ?1,
                    search_all_sources = ?2,
                    enabled = 1,
                    updated_at = datetime('now')
                WHERE id = ?3
                "#,
            )
            .bind(&priority_order_json)
            .bind(bool_to_int(data.search_all_sources))
            .bind(&existing_id)
            .execute(&self.pool)
            .await?;

            let id = str_to_uuid(&existing_id)?;
            self.get(id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Rule not found after update"))
        } else {
            // Create new
            self.create(data).await
        }
    }

    /// Delete a priority rule

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM source_priority_rules WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a priority rule by scope

    #[cfg(feature = "sqlite")]
    pub async fn delete_by_scope(
        &self,
        user_id: Uuid,
        library_type: Option<&str>,
        library_id: Option<Uuid>,
    ) -> Result<bool> {
        // SQLite doesn't have IS NOT DISTINCT FROM, so we use COALESCE comparison
        let library_id_str = library_id.map(uuid_to_str);

        let result = sqlx::query(
            r#"
            DELETE FROM source_priority_rules
            WHERE user_id = ?1
              AND COALESCE(library_type, '') = COALESCE(?2, '')
              AND COALESCE(library_id, '') = COALESCE(?3, '')
            "#,
        )
        .bind(uuid_to_str(user_id))
        .bind(library_type)
        .bind(library_id_str)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
