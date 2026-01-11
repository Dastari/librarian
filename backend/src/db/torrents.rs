//! Torrent database operations
//!
//! Handles persistence of torrent state for resuming after restarts.

use anyhow::Result;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// A torrent record in the database
#[derive(Debug, Clone, sqlx::FromRow)]
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
    pub media_item_id: Option<Uuid>,
    pub subscription_id: Option<Uuid>,
    pub library_id: Option<Uuid>,
    pub episode_id: Option<Uuid>,
    pub download_path: Option<String>,
    pub post_process_status: Option<String>,
    pub added_at: OffsetDateTime,
    pub completed_at: Option<OffsetDateTime>,
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
    pool: PgPool,
}

impl TorrentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new torrent record
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

    /// Get all torrents for a user
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<TorrentRecord>> {
        let records = sqlx::query_as::<_, TorrentRecord>(
            "SELECT * FROM torrents WHERE user_id = $1 ORDER BY added_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get all torrents that should be resumed (not completed, not errored)
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
    pub async fn get_by_info_hash(&self, info_hash: &str) -> Result<Option<TorrentRecord>> {
        let record = sqlx::query_as::<_, TorrentRecord>(
            "SELECT * FROM torrents WHERE info_hash = $1",
        )
        .bind(info_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update torrent progress and state
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

    /// Update torrent state
    pub async fn update_state(&self, info_hash: &str, state: &str) -> Result<()> {
        sqlx::query("UPDATE torrents SET state = $2 WHERE info_hash = $1")
            .bind(info_hash)
            .bind(state)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete a torrent record
    pub async fn delete(&self, info_hash: &str) -> Result<()> {
        sqlx::query("DELETE FROM torrents WHERE info_hash = $1")
            .bind(info_hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Mark torrent as completed
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

    /// List torrents that are completed but not yet processed
    pub async fn list_pending_processing(&self) -> Result<Vec<TorrentRecord>> {
        let records = sqlx::query_as::<_, TorrentRecord>(
            r#"
            SELECT id, user_id, info_hash, magnet_uri, name, state, progress,
                   total_bytes, downloaded_bytes, uploaded_bytes, save_path,
                   media_item_id, subscription_id, library_id, episode_id,
                   download_path, post_process_status, added_at, completed_at
            FROM torrents 
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
    pub async fn mark_processed(&self, info_hash: &str) -> Result<()> {
        sqlx::query(
            "UPDATE torrents SET post_process_status = 'completed' WHERE info_hash = $1"
        )
        .bind(info_hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Link torrent to an episode
    pub async fn link_to_episode(&self, info_hash: &str, episode_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE torrents SET episode_id = $2, post_process_status = 'pending' WHERE info_hash = $1"
        )
        .bind(info_hash)
        .bind(episode_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a default user ID from the database (for session sync when no user context)
    pub async fn get_default_user_id(&self) -> Result<Option<Uuid>> {
        // Try to get a user from existing torrents first
        let result = sqlx::query_scalar::<_, Uuid>(
            "SELECT DISTINCT user_id FROM torrents LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;

        if result.is_some() {
            return Ok(result);
        }

        // Fall back to any user from libraries table
        let result = sqlx::query_scalar::<_, Uuid>(
            "SELECT DISTINCT user_id FROM libraries LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// Upsert a torrent by info_hash - creates if not exists, updates if exists
    /// This is used for syncing session torrents to the database
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
        // First, check if the torrent exists
        let existing = self.get_by_info_hash(info_hash).await?;

        if existing.is_some() {
            // Update existing record
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
            // Create new record
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

    /// List all torrents (for admin/sync purposes)
    pub async fn list_all(&self) -> Result<Vec<TorrentRecord>> {
        let records = sqlx::query_as::<_, TorrentRecord>(
            "SELECT * FROM torrents ORDER BY added_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }
}
