//! Artwork repository for SQLite BLOB storage
//!
//! Stores artwork images directly in the SQLite database as BLOBs.
//! This simplifies deployment (single database file) and backup.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool as Pool;

use super::sqlite_helpers::now_iso8601;

// ============================================================================
// Artwork Record
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtworkRecord {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub artwork_type: String,
    pub content_hash: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub source_url: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub created_at: String,
}

/// Artwork with image data
pub struct ArtworkWithData {
    pub record: ArtworkRecord,
    pub data: Vec<u8>,
}

/// Input for creating/updating artwork
pub struct UpsertArtwork {
    pub entity_type: String,
    pub entity_id: String,
    pub artwork_type: String,
    pub content_hash: String,
    pub mime_type: String,
    pub data: Vec<u8>,
    pub source_url: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

// ============================================================================
// Repository
// ============================================================================

pub struct ArtworkRepository {
    pool: Pool,
}

impl ArtworkRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Store or update artwork (upsert)
    pub async fn upsert(&self, artwork: UpsertArtwork) -> Result<ArtworkRecord> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso8601();
        let size_bytes = artwork.data.len() as i64;

        sqlx::query(
            r#"
            INSERT INTO artwork_cache (
                id, entity_type, entity_id, artwork_type, content_hash, 
                mime_type, data, size_bytes, source_url, width, height, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(entity_type, entity_id, artwork_type) DO UPDATE SET
                content_hash = excluded.content_hash,
                mime_type = excluded.mime_type,
                data = excluded.data,
                size_bytes = excluded.size_bytes,
                source_url = excluded.source_url,
                width = excluded.width,
                height = excluded.height
            "#,
        )
        .bind(&id)
        .bind(&artwork.entity_type)
        .bind(&artwork.entity_id)
        .bind(&artwork.artwork_type)
        .bind(&artwork.content_hash)
        .bind(&artwork.mime_type)
        .bind(&artwork.data)
        .bind(size_bytes)
        .bind(&artwork.source_url)
        .bind(artwork.width)
        .bind(artwork.height)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get(&artwork.entity_type, &artwork.entity_id, &artwork.artwork_type)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to store artwork"))
    }

    /// Get artwork metadata (without image data)
    pub async fn get(
        &self,
        entity_type: &str,
        entity_id: &str,
        artwork_type: &str,
    ) -> Result<Option<ArtworkRecord>> {
        let row = sqlx::query_as::<_, (String, String, String, String, String, String, i64, Option<String>, Option<i32>, Option<i32>, String)>(
            r#"
            SELECT id, entity_type, entity_id, artwork_type, content_hash, 
                   mime_type, size_bytes, source_url, width, height, created_at
            FROM artwork_cache 
            WHERE entity_type = ? AND entity_id = ? AND artwork_type = ?
            "#,
        )
        .bind(entity_type)
        .bind(entity_id)
        .bind(artwork_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| ArtworkRecord {
            id: r.0,
            entity_type: r.1,
            entity_id: r.2,
            artwork_type: r.3,
            content_hash: r.4,
            mime_type: r.5,
            size_bytes: r.6,
            source_url: r.7,
            width: r.8,
            height: r.9,
            created_at: r.10,
        }))
    }

    /// Get artwork with image data
    pub async fn get_with_data(
        &self,
        entity_type: &str,
        entity_id: &str,
        artwork_type: &str,
    ) -> Result<Option<ArtworkWithData>> {
        let row = sqlx::query_as::<_, (String, String, String, String, String, String, Vec<u8>, i64, Option<String>, Option<i32>, Option<i32>, String)>(
            r#"
            SELECT id, entity_type, entity_id, artwork_type, content_hash, 
                   mime_type, data, size_bytes, source_url, width, height, created_at
            FROM artwork_cache 
            WHERE entity_type = ? AND entity_id = ? AND artwork_type = ?
            "#,
        )
        .bind(entity_type)
        .bind(entity_id)
        .bind(artwork_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| ArtworkWithData {
            record: ArtworkRecord {
                id: r.0,
                entity_type: r.1,
                entity_id: r.2,
                artwork_type: r.3,
                content_hash: r.4,
                mime_type: r.5,
                size_bytes: r.7,
                source_url: r.8,
                width: r.9,
                height: r.10,
                created_at: r.11,
            },
            data: r.6,
        }))
    }

    /// Check if artwork exists by content hash (for deduplication)
    pub async fn exists_by_hash(&self, content_hash: &str) -> Result<bool> {
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM artwork_cache WHERE content_hash = ?"
        )
        .bind(content_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0 > 0)
    }

    /// List all artwork for an entity
    pub async fn list_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Vec<ArtworkRecord>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, String, i64, Option<String>, Option<i32>, Option<i32>, String)>(
            r#"
            SELECT id, entity_type, entity_id, artwork_type, content_hash, 
                   mime_type, size_bytes, source_url, width, height, created_at
            FROM artwork_cache 
            WHERE entity_type = ? AND entity_id = ?
            "#,
        )
        .bind(entity_type)
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| ArtworkRecord {
            id: r.0,
            entity_type: r.1,
            entity_id: r.2,
            artwork_type: r.3,
            content_hash: r.4,
            mime_type: r.5,
            size_bytes: r.6,
            source_url: r.7,
            width: r.8,
            height: r.9,
            created_at: r.10,
        }).collect())
    }

    /// Delete artwork for an entity
    pub async fn delete_for_entity(&self, entity_type: &str, entity_id: &str) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM artwork_cache WHERE entity_type = ? AND entity_id = ?"
        )
        .bind(entity_type)
        .bind(entity_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Delete specific artwork
    pub async fn delete(
        &self,
        entity_type: &str,
        entity_id: &str,
        artwork_type: &str,
    ) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM artwork_cache WHERE entity_type = ? AND entity_id = ? AND artwork_type = ?"
        )
        .bind(entity_type)
        .bind(entity_id)
        .bind(artwork_type)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get total storage used by artwork
    pub async fn total_storage_bytes(&self) -> Result<i64> {
        let row = sqlx::query_as::<_, (Option<i64>,)>(
            "SELECT SUM(size_bytes) FROM artwork_cache"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0.unwrap_or(0))
    }

    /// Get artwork count
    pub async fn count(&self) -> Result<i64> {
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM artwork_cache"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0)
    }

    /// Get storage stats by entity type
    pub async fn storage_stats(&self) -> Result<Vec<(String, i64, i64)>> {
        let rows = sqlx::query_as::<_, (String, i64, i64)>(
            r#"
            SELECT entity_type, COUNT(*) as count, SUM(size_bytes) as total_bytes
            FROM artwork_cache
            GROUP BY entity_type
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
