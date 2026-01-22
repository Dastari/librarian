//! Torrent database operations
//!
//! Handles persistence of torrent state for resuming after restarts.

use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "postgres")]
type DbPool = PgPool;
#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

/// A torrent record in the database
#[derive(Debug, Clone)]
pub struct TorrentRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub info_hash: String,
    pub magnet_uri: Option<String>,
    pub name: String,
    pub state: String,
    pub progress: f32,
    pub total_bytes: i64,
    pub downloaded_bytes: i64,
    pub uploaded_bytes: i64,
    pub save_path: String,
    pub download_path: Option<String>,
    pub source_url: Option<String>,
    pub source_feed_id: Option<Uuid>,
    pub source_indexer_id: Option<Uuid>,
    pub post_process_status: Option<String>,
    pub post_process_error: Option<String>,
    pub processed_at: Option<DateTime<Utc>>,
    pub added_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub excluded_files: Vec<i32>,
}

#[cfg(feature = "postgres")]
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for TorrentRecord {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use time::OffsetDateTime;
        
        fn offset_to_chrono(odt: OffsetDateTime) -> DateTime<Utc> {
            DateTime::from_timestamp(odt.unix_timestamp(), odt.nanosecond()).unwrap_or_default()
        }
        
        let processed_at: Option<OffsetDateTime> = row.try_get("processed_at")?;
        let added_at: OffsetDateTime = row.try_get("added_at")?;
        let completed_at: Option<OffsetDateTime> = row.try_get("completed_at")?;
        
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            info_hash: row.try_get("info_hash")?,
            magnet_uri: row.try_get("magnet_uri")?,
            name: row.try_get("name")?,
            state: row.try_get("state")?,
            progress: row.try_get("progress")?,
            total_bytes: row.try_get("total_bytes")?,
            downloaded_bytes: row.try_get("downloaded_bytes")?,
            uploaded_bytes: row.try_get("uploaded_bytes")?,
            save_path: row.try_get("save_path")?,
            download_path: row.try_get("download_path")?,
            source_url: row.try_get("source_url")?,
            source_feed_id: row.try_get("source_feed_id")?,
            source_indexer_id: row.try_get("source_indexer_id")?,
            post_process_status: row.try_get("post_process_status")?,
            post_process_error: row.try_get("post_process_error")?,
            processed_at: processed_at.map(offset_to_chrono),
            added_at: offset_to_chrono(added_at),
            completed_at: completed_at.map(offset_to_chrono),
            excluded_files: row.try_get("excluded_files")?,
        })
    }
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for TorrentRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use crate::db::sqlite_helpers::{str_to_uuid, str_to_datetime, json_to_vec};
        
        let id_str: String = row.try_get("id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let source_feed_id_str: Option<String> = row.try_get("source_feed_id")?;
        let source_indexer_id_str: Option<String> = row.try_get("source_indexer_id")?;
        let processed_at_str: Option<String> = row.try_get("processed_at")?;
        let added_at_str: String = row.try_get("added_at")?;
        let completed_at_str: Option<String> = row.try_get("completed_at")?;
        let excluded_files_json: String = row.try_get("excluded_files")?;
        
        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            info_hash: row.try_get("info_hash")?,
            magnet_uri: row.try_get("magnet_uri")?,
            name: row.try_get("name")?,
            state: row.try_get("state")?,
            progress: row.try_get("progress")?,
            total_bytes: row.try_get("total_bytes")?,
            downloaded_bytes: row.try_get("downloaded_bytes")?,
            uploaded_bytes: row.try_get("uploaded_bytes")?,
            save_path: row.try_get("save_path")?,
            download_path: row.try_get("download_path")?,
            source_url: row.try_get("source_url")?,
            source_feed_id: source_feed_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            source_indexer_id: source_indexer_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            post_process_status: row.try_get("post_process_status")?,
            post_process_error: row.try_get("post_process_error")?,
            processed_at: processed_at_str
                .map(|s| str_to_datetime(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            added_at: str_to_datetime(&added_at_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            completed_at: completed_at_str
                .map(|s| str_to_datetime(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            excluded_files: json_to_vec(&excluded_files_json),
        })
    }
}

/// Input for creating a new torrent record
#[derive(Debug)]
pub struct CreateTorrent {
    pub user_id: Uuid,
    pub info_hash: String,
    pub magnet_uri: Option<String>,
    pub name: String,
    pub save_path: String,
    pub total_bytes: i64,
}

/// Torrent repository for database operations
pub struct TorrentRepository {
    pool: DbPool,
}

impl TorrentRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Insert a new torrent record
    #[cfg(feature = "postgres")]
    pub async fn create(&self, input: CreateTorrent) -> Result<TorrentRecord> {
        let record = sqlx::query_as::<_, TorrentRecord>(
            r#"
            INSERT INTO torrents (user_id, info_hash, magnet_uri, name, save_path, total_bytes, state)
            VALUES ($1, $2, $3, $4, $5, $6, 'queued')
            ON CONFLICT (user_id, info_hash) 
            DO UPDATE SET 
                name = EXCLUDED.name,
                magnet_uri = COALESCE(EXCLUDED.magnet_uri, torrents.magnet_uri)
            RETURNING *
            "#,
        )
        .bind(input.user_id)
        .bind(&input.info_hash)
        .bind(&input.magnet_uri)
        .bind(&input.name)
        .bind(&input.save_path)
        .bind(input.total_bytes)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, input: CreateTorrent) -> Result<TorrentRecord> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);
        let user_id_str = uuid_to_str(input.user_id);
        
        // First check if exists
        let existing = self.get_by_info_hash(&input.info_hash).await?;
        
        if let Some(_existing) = existing {
            // Update existing
            sqlx::query(
                r#"
                UPDATE torrents SET
                    name = ?2,
                    magnet_uri = COALESCE(?3, magnet_uri)
                WHERE info_hash = ?1
                "#,
            )
            .bind(&input.info_hash)
            .bind(&input.name)
            .bind(&input.magnet_uri)
            .execute(&self.pool)
            .await?;
            
            return self.get_by_info_hash(&input.info_hash).await?
                .ok_or_else(|| anyhow::anyhow!("Failed to retrieve torrent after update"));
        }
        
        // Insert new
        sqlx::query(
            r#"
            INSERT INTO torrents (id, user_id, info_hash, magnet_uri, name, save_path, total_bytes, state, 
                                  progress, downloaded_bytes, uploaded_bytes, excluded_files, added_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'queued', 0.0, 0, 0, '[]', datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(&user_id_str)
        .bind(&input.info_hash)
        .bind(&input.magnet_uri)
        .bind(&input.name)
        .bind(&input.save_path)
        .bind(input.total_bytes)
        .execute(&self.pool)
        .await?;

        self.get_by_info_hash(&input.info_hash).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve torrent after insert"))
    }

    /// Get all torrents for a user
    #[cfg(feature = "postgres")]
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<TorrentRecord>> {
        let records = sqlx::query_as::<_, TorrentRecord>(
            "SELECT * FROM torrents WHERE user_id = $1 ORDER BY added_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    #[cfg(feature = "sqlite")]
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<TorrentRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let records = sqlx::query_as::<_, TorrentRecord>(
            "SELECT * FROM torrents WHERE user_id = ?1 ORDER BY added_at DESC",
        )
        .bind(uuid_to_str(user_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get all torrents that should be resumed
    pub async fn list_resumable(&self) -> Result<Vec<TorrentRecord>> {
        let records = sqlx::query_as::<_, TorrentRecord>(
            r#"
            SELECT * FROM torrents 
            WHERE state NOT IN ('completed', 'error') 
                AND magnet_uri IS NOT NULL
            ORDER BY added_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get a torrent by info_hash
    #[cfg(feature = "postgres")]
    pub async fn get_by_info_hash(&self, info_hash: &str) -> Result<Option<TorrentRecord>> {
        let record =
            sqlx::query_as::<_, TorrentRecord>("SELECT * FROM torrents WHERE info_hash = $1")
                .bind(info_hash)
                .fetch_optional(&self.pool)
                .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn get_by_info_hash(&self, info_hash: &str) -> Result<Option<TorrentRecord>> {
        let record =
            sqlx::query_as::<_, TorrentRecord>("SELECT * FROM torrents WHERE info_hash = ?1")
                .bind(info_hash)
                .fetch_optional(&self.pool)
                .await?;

        Ok(record)
    }

    /// Get a torrent by ID
    #[cfg(feature = "postgres")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<TorrentRecord>> {
        let record =
            sqlx::query_as::<_, TorrentRecord>("SELECT * FROM torrents WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<TorrentRecord>> {
        let record =
            sqlx::query_as::<_, TorrentRecord>("SELECT * FROM torrents WHERE id = ?1")
                .bind(id.to_string())
                .fetch_optional(&self.pool)
                .await?;

        Ok(record)
    }

    /// Update torrent progress and state
    #[cfg(feature = "postgres")]
    pub async fn update_progress(
        &self,
        info_hash: &str,
        state: &str,
        progress: f64,
        downloaded_bytes: i64,
        uploaded_bytes: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE torrents 
            SET state = $2, 
                progress = $3, 
                downloaded_bytes = $4, 
                uploaded_bytes = $5,
                completed_at = CASE WHEN $2 = 'seeding' AND completed_at IS NULL THEN NOW() ELSE completed_at END
            WHERE info_hash = $1
            "#,
        )
        .bind(info_hash)
        .bind(state)
        .bind(progress)
        .bind(downloaded_bytes)
        .bind(uploaded_bytes)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn update_progress(
        &self,
        info_hash: &str,
        state: &str,
        progress: f64,
        downloaded_bytes: i64,
        uploaded_bytes: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE torrents 
            SET state = ?2, 
                progress = ?3, 
                downloaded_bytes = ?4, 
                uploaded_bytes = ?5,
                completed_at = CASE WHEN ?2 = 'seeding' AND completed_at IS NULL THEN datetime('now') ELSE completed_at END
            WHERE info_hash = ?1
            "#,
        )
        .bind(info_hash)
        .bind(state)
        .bind(progress)
        .bind(downloaded_bytes)
        .bind(uploaded_bytes)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update torrent state
    #[cfg(feature = "postgres")]
    pub async fn update_state(&self, info_hash: &str, state: &str) -> Result<()> {
        sqlx::query("UPDATE torrents SET state = $2 WHERE info_hash = $1")
            .bind(info_hash)
            .bind(state)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn update_state(&self, info_hash: &str, state: &str) -> Result<()> {
        sqlx::query("UPDATE torrents SET state = ?2 WHERE info_hash = ?1")
            .bind(info_hash)
            .bind(state)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete a torrent record
    #[cfg(feature = "postgres")]
    pub async fn delete(&self, info_hash: &str) -> Result<()> {
        sqlx::query("DELETE FROM torrents WHERE info_hash = $1")
            .bind(info_hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, info_hash: &str) -> Result<()> {
        sqlx::query("DELETE FROM torrents WHERE info_hash = ?1")
            .bind(info_hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Mark torrent as completed
    #[cfg(feature = "postgres")]
    pub async fn mark_completed(&self, info_hash: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE torrents 
            SET state = 'seeding', 
                progress = 1.0, 
                completed_at = NOW() 
            WHERE info_hash = $1
            "#,
        )
        .bind(info_hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn mark_completed(&self, info_hash: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE torrents 
            SET state = 'seeding', 
                progress = 1.0, 
                completed_at = datetime('now') 
            WHERE info_hash = ?1
            "#,
        )
        .bind(info_hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List torrents that are completed but not yet processed
    pub async fn list_pending_processing(&self) -> Result<Vec<TorrentRecord>> {
        let records = sqlx::query_as::<_, TorrentRecord>(
            r#"
            SELECT * FROM torrents 
            WHERE state = 'seeding' 
              AND completed_at IS NOT NULL
              AND (post_process_status IS NULL OR post_process_status = 'pending')
            ORDER BY completed_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Mark torrent as processed
    #[cfg(feature = "postgres")]
    pub async fn mark_processed(&self, info_hash: &str) -> Result<()> {
        sqlx::query("UPDATE torrents SET post_process_status = 'completed' WHERE info_hash = $1")
            .bind(info_hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn mark_processed(&self, info_hash: &str) -> Result<()> {
        sqlx::query("UPDATE torrents SET post_process_status = 'completed' WHERE info_hash = ?1")
            .bind(info_hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get a default user ID from the database
    #[cfg(feature = "postgres")]
    pub async fn get_default_user_id(&self) -> Result<Option<Uuid>> {
        let result = sqlx::query_scalar::<_, Uuid>("SELECT DISTINCT user_id FROM torrents LIMIT 1")
            .fetch_optional(&self.pool)
            .await?;

        if result.is_some() {
            return Ok(result);
        }

        let result =
            sqlx::query_scalar::<_, Uuid>("SELECT DISTINCT user_id FROM libraries LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;

        Ok(result)
    }

    #[cfg(feature = "sqlite")]
    pub async fn get_default_user_id(&self) -> Result<Option<Uuid>> {
        use crate::db::sqlite_helpers::str_to_uuid;
        
        let result: Option<String> = sqlx::query_scalar("SELECT DISTINCT user_id FROM torrents LIMIT 1")
            .fetch_optional(&self.pool)
            .await?;

        if let Some(id_str) = result {
            return Ok(Some(str_to_uuid(&id_str)?));
        }

        let result: Option<String> =
            sqlx::query_scalar("SELECT DISTINCT user_id FROM libraries LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;

        match result {
            Some(id_str) => Ok(Some(str_to_uuid(&id_str)?)),
            None => Ok(None),
        }
    }

    /// Upsert a torrent by info_hash
    #[cfg(feature = "postgres")]
    pub async fn upsert_from_session(
        &self,
        info_hash: &str,
        name: &str,
        state: &str,
        progress: f64,
        total_bytes: i64,
        downloaded_bytes: i64,
        uploaded_bytes: i64,
        save_path: &str,
        fallback_user_id: Uuid,
    ) -> Result<()> {
        let existing = self.get_by_info_hash(info_hash).await?;

        if existing.is_some() {
            sqlx::query(
                r#"
                UPDATE torrents 
                SET state = $2, 
                    progress = $3, 
                    downloaded_bytes = $4, 
                    uploaded_bytes = $5,
                    name = COALESCE(NULLIF($6, ''), name),
                    total_bytes = CASE WHEN $7 > 0 THEN $7 ELSE total_bytes END,
                    completed_at = CASE WHEN $2 = 'seeding' AND completed_at IS NULL THEN NOW() ELSE completed_at END
                WHERE info_hash = $1
                "#,
            )
            .bind(info_hash)
            .bind(state)
            .bind(progress)
            .bind(downloaded_bytes)
            .bind(uploaded_bytes)
            .bind(name)
            .bind(total_bytes)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                r#"
                INSERT INTO torrents (user_id, info_hash, name, save_path, total_bytes, downloaded_bytes, uploaded_bytes, state, progress)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (user_id, info_hash) DO UPDATE SET
                    state = EXCLUDED.state,
                    progress = EXCLUDED.progress,
                    downloaded_bytes = EXCLUDED.downloaded_bytes,
                    uploaded_bytes = EXCLUDED.uploaded_bytes
                "#,
            )
            .bind(fallback_user_id)
            .bind(info_hash)
            .bind(name)
            .bind(save_path)
            .bind(total_bytes)
            .bind(downloaded_bytes)
            .bind(uploaded_bytes)
            .bind(state)
            .bind(progress)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn upsert_from_session(
        &self,
        info_hash: &str,
        name: &str,
        state: &str,
        progress: f64,
        total_bytes: i64,
        downloaded_bytes: i64,
        uploaded_bytes: i64,
        save_path: &str,
        fallback_user_id: Uuid,
    ) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let existing = self.get_by_info_hash(info_hash).await?;

        if existing.is_some() {
            sqlx::query(
                r#"
                UPDATE torrents 
                SET state = ?2, 
                    progress = ?3, 
                    downloaded_bytes = ?4, 
                    uploaded_bytes = ?5,
                    name = COALESCE(NULLIF(?6, ''), name),
                    total_bytes = CASE WHEN ?7 > 0 THEN ?7 ELSE total_bytes END,
                    completed_at = CASE WHEN ?2 = 'seeding' AND completed_at IS NULL THEN datetime('now') ELSE completed_at END
                WHERE info_hash = ?1
                "#,
            )
            .bind(info_hash)
            .bind(state)
            .bind(progress)
            .bind(downloaded_bytes)
            .bind(uploaded_bytes)
            .bind(name)
            .bind(total_bytes)
            .execute(&self.pool)
            .await?;
        } else {
            let id = Uuid::new_v4();
            sqlx::query(
                r#"
                INSERT INTO torrents (id, user_id, info_hash, name, save_path, total_bytes, downloaded_bytes, 
                                      uploaded_bytes, state, progress, excluded_files, added_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, '[]', datetime('now'))
                "#,
            )
            .bind(uuid_to_str(id))
            .bind(uuid_to_str(fallback_user_id))
            .bind(info_hash)
            .bind(name)
            .bind(save_path)
            .bind(total_bytes)
            .bind(downloaded_bytes)
            .bind(uploaded_bytes)
            .bind(state)
            .bind(progress)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// List all torrents
    pub async fn list_all(&self) -> Result<Vec<TorrentRecord>> {
        let records =
            sqlx::query_as::<_, TorrentRecord>("SELECT * FROM torrents ORDER BY added_at DESC")
                .fetch_all(&self.pool)
                .await?;

        Ok(records)
    }

    /// Update post_process_status
    #[cfg(feature = "postgres")]
    pub async fn update_post_process_status(&self, info_hash: &str, status: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE torrents 
            SET post_process_status = $2,
                processed_at = CASE WHEN $2 = 'completed' THEN NOW() ELSE processed_at END
            WHERE info_hash = $1
            "#,
        )
        .bind(info_hash)
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn update_post_process_status(&self, info_hash: &str, status: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE torrents 
            SET post_process_status = ?2,
                processed_at = CASE WHEN ?2 = 'completed' THEN datetime('now') ELSE processed_at END
            WHERE info_hash = ?1
            "#,
        )
        .bind(info_hash)
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List torrents that completed but weren't matched
    pub async fn list_unmatched(&self) -> Result<Vec<TorrentRecord>> {
        let records = sqlx::query_as::<_, TorrentRecord>(
            r#"
            SELECT t.* FROM torrents t
            WHERE t.state = 'seeding' 
              AND t.completed_at IS NOT NULL
              AND (t.post_process_status = 'unmatched' OR t.post_process_status IS NULL)
              AND NOT EXISTS (
                  SELECT 1 FROM pending_file_matches pfm 
                  WHERE pfm.source_type = 'torrent' 
                    AND pfm.source_id = t.id
              )
            ORDER BY t.completed_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }
}
