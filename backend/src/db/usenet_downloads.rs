//! Usenet downloads database repository
//!
//! Handles CRUD operations for Usenet download tracking.

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Usenet download state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsenetDownloadState {
    Queued,
    Downloading,
    Paused,
    Completed,
    Failed,
    Removed,
}

impl std::fmt::Display for UsenetDownloadState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UsenetDownloadState::Queued => write!(f, "queued"),
            UsenetDownloadState::Downloading => write!(f, "downloading"),
            UsenetDownloadState::Paused => write!(f, "paused"),
            UsenetDownloadState::Completed => write!(f, "completed"),
            UsenetDownloadState::Failed => write!(f, "failed"),
            UsenetDownloadState::Removed => write!(f, "removed"),
        }
    }
}

impl std::str::FromStr for UsenetDownloadState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "queued" => Ok(UsenetDownloadState::Queued),
            "downloading" => Ok(UsenetDownloadState::Downloading),
            "paused" => Ok(UsenetDownloadState::Paused),
            "completed" => Ok(UsenetDownloadState::Completed),
            "failed" => Ok(UsenetDownloadState::Failed),
            "removed" => Ok(UsenetDownloadState::Removed),
            _ => Err(anyhow::anyhow!("Unknown usenet download state: {}", s)),
        }
    }
}

/// Usenet download record from database
#[derive(Debug, Clone, FromRow)]
pub struct UsenetDownloadRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub nzb_name: String,
    pub nzb_hash: Option<String>,
    pub state: String,
    pub progress: Option<rust_decimal::Decimal>,
    pub size_bytes: Option<i64>,
    pub downloaded_bytes: Option<i64>,
    pub download_speed: Option<i64>,
    pub eta_seconds: Option<i32>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub download_path: Option<String>,
    pub library_id: Option<Uuid>,
    pub episode_id: Option<Uuid>,
    pub movie_id: Option<Uuid>,
    pub album_id: Option<Uuid>,
    pub audiobook_id: Option<Uuid>,
    pub indexer_id: Option<Uuid>,
    pub post_process_status: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Usenet file match record from database
#[derive(Debug, Clone, FromRow)]
pub struct UsenetFileMatchRecord {
    pub id: Uuid,
    pub usenet_download_id: Uuid,
    pub file_path: String,
    pub file_size: Option<i64>,
    pub episode_id: Option<Uuid>,
    pub movie_id: Option<Uuid>,
    pub album_id: Option<Uuid>,
    pub track_id: Option<Uuid>,
    pub audiobook_id: Option<Uuid>,
    pub processed: bool,
    pub media_file_id: Option<Uuid>,
    pub match_confidence: Option<rust_decimal::Decimal>,
    pub match_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a new usenet download
#[derive(Debug, Clone)]
pub struct CreateUsenetDownload {
    pub user_id: Uuid,
    pub nzb_name: String,
    pub nzb_hash: Option<String>,
    pub size_bytes: Option<i64>,
    pub download_path: Option<String>,
    pub library_id: Option<Uuid>,
    pub episode_id: Option<Uuid>,
    pub movie_id: Option<Uuid>,
    pub album_id: Option<Uuid>,
    pub audiobook_id: Option<Uuid>,
    pub indexer_id: Option<Uuid>,
}

/// Data for updating a usenet download
#[derive(Debug, Clone, Default)]
pub struct UpdateUsenetDownload {
    pub state: Option<String>,
    pub progress: Option<f64>,
    pub downloaded_bytes: Option<i64>,
    pub download_speed: Option<i64>,
    pub eta_seconds: Option<i32>,
    pub error_message: Option<String>,
    pub download_path: Option<String>,
    pub post_process_status: Option<String>,
}

/// Usenet downloads database repository
pub struct UsenetDownloadsRepository {
    pool: PgPool,
}

impl UsenetDownloadsRepository {
    /// Create a new repository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a usenet download by ID
    pub async fn get(&self, id: Uuid) -> Result<Option<UsenetDownloadRecord>> {
        let record = sqlx::query_as::<_, UsenetDownloadRecord>(
            r#"
            SELECT id, user_id, nzb_name, nzb_hash, state, progress,
                   size_bytes, downloaded_bytes, download_speed, eta_seconds,
                   error_message, retry_count, download_path,
                   library_id, episode_id, movie_id, album_id, audiobook_id,
                   indexer_id, post_process_status, created_at, updated_at, completed_at
            FROM usenet_downloads
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a usenet download by NZB hash
    pub async fn get_by_hash(&self, nzb_hash: &str) -> Result<Option<UsenetDownloadRecord>> {
        let record = sqlx::query_as::<_, UsenetDownloadRecord>(
            r#"
            SELECT id, user_id, nzb_name, nzb_hash, state, progress,
                   size_bytes, downloaded_bytes, download_speed, eta_seconds,
                   error_message, retry_count, download_path,
                   library_id, episode_id, movie_id, album_id, audiobook_id,
                   indexer_id, post_process_status, created_at, updated_at, completed_at
            FROM usenet_downloads
            WHERE nzb_hash = $1
            "#,
        )
        .bind(nzb_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get all usenet downloads for a user
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<UsenetDownloadRecord>> {
        let records = sqlx::query_as::<_, UsenetDownloadRecord>(
            r#"
            SELECT id, user_id, nzb_name, nzb_hash, state, progress,
                   size_bytes, downloaded_bytes, download_speed, eta_seconds,
                   error_message, retry_count, download_path,
                   library_id, episode_id, movie_id, album_id, audiobook_id,
                   indexer_id, post_process_status, created_at, updated_at, completed_at
            FROM usenet_downloads
            WHERE user_id = $1 AND state != 'removed'
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get active usenet downloads (downloading or queued)
    pub async fn list_active(&self, user_id: Uuid) -> Result<Vec<UsenetDownloadRecord>> {
        let records = sqlx::query_as::<_, UsenetDownloadRecord>(
            r#"
            SELECT id, user_id, nzb_name, nzb_hash, state, progress,
                   size_bytes, downloaded_bytes, download_speed, eta_seconds,
                   error_message, retry_count, download_path,
                   library_id, episode_id, movie_id, album_id, audiobook_id,
                   indexer_id, post_process_status, created_at, updated_at, completed_at
            FROM usenet_downloads
            WHERE user_id = $1 AND state IN ('queued', 'downloading')
            ORDER BY created_at ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get completed downloads pending processing
    pub async fn list_pending_processing(&self) -> Result<Vec<UsenetDownloadRecord>> {
        let records = sqlx::query_as::<_, UsenetDownloadRecord>(
            r#"
            SELECT id, user_id, nzb_name, nzb_hash, state, progress,
                   size_bytes, downloaded_bytes, download_speed, eta_seconds,
                   error_message, retry_count, download_path,
                   library_id, episode_id, movie_id, album_id, audiobook_id,
                   indexer_id, post_process_status, created_at, updated_at, completed_at
            FROM usenet_downloads
            WHERE state = 'completed'
              AND (post_process_status IS NULL OR post_process_status = 'pending')
            ORDER BY completed_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Create a new usenet download
    pub async fn create(&self, data: CreateUsenetDownload) -> Result<UsenetDownloadRecord> {
        let record = sqlx::query_as::<_, UsenetDownloadRecord>(
            r#"
            INSERT INTO usenet_downloads (
                user_id, nzb_name, nzb_hash, size_bytes, download_path,
                library_id, episode_id, movie_id, album_id, audiobook_id, indexer_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, user_id, nzb_name, nzb_hash, state, progress,
                      size_bytes, downloaded_bytes, download_speed, eta_seconds,
                      error_message, retry_count, download_path,
                      library_id, episode_id, movie_id, album_id, audiobook_id,
                      indexer_id, post_process_status, created_at, updated_at, completed_at
            "#,
        )
        .bind(data.user_id)
        .bind(&data.nzb_name)
        .bind(&data.nzb_hash)
        .bind(data.size_bytes)
        .bind(&data.download_path)
        .bind(data.library_id)
        .bind(data.episode_id)
        .bind(data.movie_id)
        .bind(data.album_id)
        .bind(data.audiobook_id)
        .bind(data.indexer_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update download progress
    pub async fn update_progress(
        &self,
        id: Uuid,
        progress: f64,
        downloaded_bytes: i64,
        speed: i64,
        eta: Option<i32>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET progress = $2,
                downloaded_bytes = $3,
                download_speed = $4,
                eta_seconds = $5,
                state = CASE WHEN state = 'queued' THEN 'downloading' ELSE state END,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(progress)
        .bind(downloaded_bytes)
        .bind(speed)
        .bind(eta)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark download as completed
    pub async fn mark_completed(&self, id: Uuid, download_path: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET state = 'completed',
                progress = 100.0,
                download_path = $2,
                completed_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(download_path)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark download as failed
    pub async fn mark_failed(&self, id: Uuid, error: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET state = 'failed',
                error_message = $2,
                retry_count = retry_count + 1,
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

    /// Pause a download
    pub async fn pause(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET state = 'paused', updated_at = NOW()
            WHERE id = $1 AND state IN ('queued', 'downloading')
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Resume a paused download
    pub async fn resume(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET state = 'queued', updated_at = NOW()
            WHERE id = $1 AND state = 'paused'
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Remove a download (soft delete)
    pub async fn remove(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET state = 'removed', updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a download permanently
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM usenet_downloads WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update post-processing status
    pub async fn set_post_process_status(&self, id: Uuid, status: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET post_process_status = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Link download to library item
    pub async fn link_to_library(
        &self,
        id: Uuid,
        library_id: Option<Uuid>,
        episode_id: Option<Uuid>,
        movie_id: Option<Uuid>,
        album_id: Option<Uuid>,
        audiobook_id: Option<Uuid>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET library_id = $2,
                episode_id = $3,
                movie_id = $4,
                album_id = $5,
                audiobook_id = $6,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(library_id)
        .bind(episode_id)
        .bind(movie_id)
        .bind(album_id)
        .bind(audiobook_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========== File Match Operations ==========

    /// Create a file match record
    pub async fn create_file_match(
        &self,
        download_id: Uuid,
        file_path: &str,
        file_size: Option<i64>,
        episode_id: Option<Uuid>,
        movie_id: Option<Uuid>,
        album_id: Option<Uuid>,
        track_id: Option<Uuid>,
        audiobook_id: Option<Uuid>,
        confidence: Option<f64>,
        reason: Option<&str>,
    ) -> Result<UsenetFileMatchRecord> {
        let record = sqlx::query_as::<_, UsenetFileMatchRecord>(
            r#"
            INSERT INTO usenet_file_matches (
                usenet_download_id, file_path, file_size,
                episode_id, movie_id, album_id, track_id, audiobook_id,
                match_confidence, match_reason
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, usenet_download_id, file_path, file_size,
                      episode_id, movie_id, album_id, track_id, audiobook_id,
                      processed, media_file_id, match_confidence, match_reason,
                      created_at, updated_at
            "#,
        )
        .bind(download_id)
        .bind(file_path)
        .bind(file_size)
        .bind(episode_id)
        .bind(movie_id)
        .bind(album_id)
        .bind(track_id)
        .bind(audiobook_id)
        .bind(confidence)
        .bind(reason)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get file matches for a download
    pub async fn get_file_matches(&self, download_id: Uuid) -> Result<Vec<UsenetFileMatchRecord>> {
        let records = sqlx::query_as::<_, UsenetFileMatchRecord>(
            r#"
            SELECT id, usenet_download_id, file_path, file_size,
                   episode_id, movie_id, album_id, track_id, audiobook_id,
                   processed, media_file_id, match_confidence, match_reason,
                   created_at, updated_at
            FROM usenet_file_matches
            WHERE usenet_download_id = $1
            ORDER BY file_path
            "#,
        )
        .bind(download_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get unprocessed file matches for a download
    pub async fn get_unprocessed_file_matches(
        &self,
        download_id: Uuid,
    ) -> Result<Vec<UsenetFileMatchRecord>> {
        let records = sqlx::query_as::<_, UsenetFileMatchRecord>(
            r#"
            SELECT id, usenet_download_id, file_path, file_size,
                   episode_id, movie_id, album_id, track_id, audiobook_id,
                   processed, media_file_id, match_confidence, match_reason,
                   created_at, updated_at
            FROM usenet_file_matches
            WHERE usenet_download_id = $1 AND processed = false
            ORDER BY file_path
            "#,
        )
        .bind(download_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Mark file match as processed
    pub async fn mark_file_match_processed(
        &self,
        match_id: Uuid,
        media_file_id: Option<Uuid>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE usenet_file_matches
            SET processed = true,
                media_file_id = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(match_id)
        .bind(media_file_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
