//! Source priority rules database repository
//!
//! Handles CRUD operations for source priority rules that determine
//! which sources to search first for different library types.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

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
#[derive(Debug, Clone, FromRow)]
pub struct PriorityRuleRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub library_type: Option<String>,
    pub library_id: Option<Uuid>,
    pub priority_order: sqlx::types::Json<Vec<SourceRef>>,
    pub search_all_sources: bool,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pool: PgPool,
}

impl PriorityRulesRepository {
    /// Create a new repository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a priority rule by ID
    pub async fn get(&self, id: Uuid) -> Result<Option<PriorityRuleRecord>> {
        let record = sqlx::query_as::<_, PriorityRuleRecord>(
            r#"
            SELECT id, user_id, library_type, library_id, priority_order,
                   search_all_sources, enabled, created_at, updated_at
            FROM source_priority_rules
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get all priority rules for a user
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<PriorityRuleRecord>> {
        let records = sqlx::query_as::<_, PriorityRuleRecord>(
            r#"
            SELECT id, user_id, library_type, library_id, priority_order,
                   search_all_sources, enabled, created_at, updated_at
            FROM source_priority_rules
            WHERE user_id = $1
            ORDER BY 
                CASE WHEN library_id IS NOT NULL THEN 0
                     WHEN library_type IS NOT NULL THEN 1
                     ELSE 2 END,
                library_type NULLS LAST
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get a user's default priority rule (no library_type or library_id)
    pub async fn get_user_default(&self, user_id: Uuid) -> Result<Option<PriorityRuleRecord>> {
        let record = sqlx::query_as::<_, PriorityRuleRecord>(
            r#"
            SELECT id, user_id, library_type, library_id, priority_order,
                   search_all_sources, enabled, created_at, updated_at
            FROM source_priority_rules
            WHERE user_id = $1
              AND library_type IS NULL
              AND library_id IS NULL
              AND enabled = true
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a priority rule by library type (e.g., "tv", "movies")
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
            WHERE user_id = $1
              AND library_type = $2
              AND library_id IS NULL
              AND enabled = true
            "#,
        )
        .bind(user_id)
        .bind(library_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a priority rule by specific library
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
            WHERE user_id = $1
              AND library_id = $2
              AND enabled = true
            "#,
        )
        .bind(user_id)
        .bind(library_id)
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
    pub async fn create(&self, data: CreatePriorityRule) -> Result<PriorityRuleRecord> {
        let record = sqlx::query_as::<_, PriorityRuleRecord>(
            r#"
            INSERT INTO source_priority_rules (
                user_id, library_type, library_id, priority_order, search_all_sources
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, user_id, library_type, library_id, priority_order,
                      search_all_sources, enabled, created_at, updated_at
            "#,
        )
        .bind(data.user_id)
        .bind(&data.library_type)
        .bind(data.library_id)
        .bind(sqlx::types::Json(&data.priority_order))
        .bind(data.search_all_sources)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update a priority rule
    pub async fn update(&self, id: Uuid, data: UpdatePriorityRule) -> Result<PriorityRuleRecord> {
        // Build dynamic update query
        let mut set_clauses = Vec::new();
        let mut param_idx = 2; // $1 is the ID

        if data.priority_order.is_some() {
            set_clauses.push(format!("priority_order = ${}", param_idx));
            param_idx += 1;
        }
        if data.search_all_sources.is_some() {
            set_clauses.push(format!("search_all_sources = ${}", param_idx));
            param_idx += 1;
        }
        if data.enabled.is_some() {
            set_clauses.push(format!("enabled = ${}", param_idx));
            // param_idx += 1;
        }

        if set_clauses.is_empty() {
            // Nothing to update, just return current record
            return self.get(id).await?.ok_or_else(|| anyhow::anyhow!("Rule not found"));
        }

        set_clauses.push("updated_at = NOW()".to_string());

        let query = format!(
            r#"
            UPDATE source_priority_rules
            SET {}
            WHERE id = $1
            RETURNING id, user_id, library_type, library_id, priority_order,
                      search_all_sources, enabled, created_at, updated_at
            "#,
            set_clauses.join(", ")
        );

        let mut query_builder = sqlx::query_as::<_, PriorityRuleRecord>(&query).bind(id);

        if let Some(ref order) = data.priority_order {
            query_builder = query_builder.bind(sqlx::types::Json(order));
        }
        if let Some(search_all) = data.search_all_sources {
            query_builder = query_builder.bind(search_all);
        }
        if let Some(enabled) = data.enabled {
            query_builder = query_builder.bind(enabled);
        }

        let record = query_builder.fetch_one(&self.pool).await?;

        Ok(record)
    }

    /// Upsert a priority rule (create or update based on scope)
    pub async fn upsert(&self, data: CreatePriorityRule) -> Result<PriorityRuleRecord> {
        let record = sqlx::query_as::<_, PriorityRuleRecord>(
            r#"
            INSERT INTO source_priority_rules (
                user_id, library_type, library_id, priority_order, search_all_sources
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id, COALESCE(library_type, ''), COALESCE(library_id, '00000000-0000-0000-0000-000000000000'::uuid))
            DO UPDATE SET 
                priority_order = EXCLUDED.priority_order,
                search_all_sources = EXCLUDED.search_all_sources,
                enabled = true,
                updated_at = NOW()
            RETURNING id, user_id, library_type, library_id, priority_order,
                      search_all_sources, enabled, created_at, updated_at
            "#,
        )
        .bind(data.user_id)
        .bind(&data.library_type)
        .bind(data.library_id)
        .bind(sqlx::types::Json(&data.priority_order))
        .bind(data.search_all_sources)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete a priority rule
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM source_priority_rules WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a priority rule by scope
    pub async fn delete_by_scope(
        &self,
        user_id: Uuid,
        library_type: Option<&str>,
        library_id: Option<Uuid>,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM source_priority_rules
            WHERE user_id = $1
              AND (library_type IS NOT DISTINCT FROM $2)
              AND (library_id IS NOT DISTINCT FROM $3)
            "#,
        )
        .bind(user_id)
        .bind(library_type)
        .bind(library_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
