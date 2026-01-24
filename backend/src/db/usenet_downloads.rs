//! Usenet downloads database repository
//!
//! Handles CRUD operations for Usenet download tracking.

use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

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
#[derive(Debug, Clone)]
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


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for UsenetDownloadRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use crate::db::sqlite_helpers::{str_to_datetime, str_to_uuid};
        use rust_decimal::Decimal;
        use sqlx::Row;
        use std::str::FromStr;

        let id_str: String = row.try_get("id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let library_id_str: Option<String> = row.try_get("library_id")?;
        let episode_id_str: Option<String> = row.try_get("episode_id")?;
        let movie_id_str: Option<String> = row.try_get("movie_id")?;
        let album_id_str: Option<String> = row.try_get("album_id")?;
        let audiobook_id_str: Option<String> = row.try_get("audiobook_id")?;
        let indexer_id_str: Option<String> = row.try_get("indexer_id")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;
        let completed_at_str: Option<String> = row.try_get("completed_at")?;
        let progress_str: Option<String> = row.try_get("progress")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            nzb_name: row.try_get("nzb_name")?,
            nzb_hash: row.try_get("nzb_hash")?,
            state: row.try_get("state")?,
            progress: progress_str
                .map(|s| Decimal::from_str(&s).ok())
                .flatten(),
            size_bytes: row.try_get("size_bytes")?,
            downloaded_bytes: row.try_get("downloaded_bytes")?,
            download_speed: row.try_get("download_speed")?,
            eta_seconds: row.try_get("eta_seconds")?,
            error_message: row.try_get("error_message")?,
            retry_count: row.try_get("retry_count")?,
            download_path: row.try_get("download_path")?,
            library_id: library_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            episode_id: episode_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            movie_id: movie_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            album_id: album_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            audiobook_id: audiobook_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            indexer_id: indexer_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            post_process_status: row.try_get("post_process_status")?,
            created_at: str_to_datetime(&created_at_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_at_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            completed_at: completed_at_str
                .map(|s| str_to_datetime(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
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
    pool: DbPool,
}

impl UsenetDownloadsRepository {
    /// Create a new repository instance
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get a usenet download by ID

    #[cfg(feature = "sqlite")]
    pub async fn get(&self, id: Uuid) -> Result<Option<UsenetDownloadRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let record = sqlx::query_as::<_, UsenetDownloadRecord>(
            r#"
            SELECT id, user_id, nzb_name, nzb_hash, state, progress,
                   size_bytes, downloaded_bytes, download_speed, eta_seconds,
                   error_message, retry_count, download_path,
                   library_id, episode_id, movie_id, album_id, audiobook_id,
                   indexer_id, post_process_status, created_at, updated_at, completed_at
            FROM usenet_downloads
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a usenet download by NZB hash

    #[cfg(feature = "sqlite")]
    pub async fn get_by_hash(&self, nzb_hash: &str) -> Result<Option<UsenetDownloadRecord>> {
        let record = sqlx::query_as::<_, UsenetDownloadRecord>(
            r#"
            SELECT id, user_id, nzb_name, nzb_hash, state, progress,
                   size_bytes, downloaded_bytes, download_speed, eta_seconds,
                   error_message, retry_count, download_path,
                   library_id, episode_id, movie_id, album_id, audiobook_id,
                   indexer_id, post_process_status, created_at, updated_at, completed_at
            FROM usenet_downloads
            WHERE nzb_hash = ?1
            "#,
        )
        .bind(nzb_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get all usenet downloads for a user

    #[cfg(feature = "sqlite")]
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<UsenetDownloadRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let records = sqlx::query_as::<_, UsenetDownloadRecord>(
            r#"
            SELECT id, user_id, nzb_name, nzb_hash, state, progress,
                   size_bytes, downloaded_bytes, download_speed, eta_seconds,
                   error_message, retry_count, download_path,
                   library_id, episode_id, movie_id, album_id, audiobook_id,
                   indexer_id, post_process_status, created_at, updated_at, completed_at
            FROM usenet_downloads
            WHERE user_id = ?1 AND state != 'removed'
            ORDER BY created_at DESC
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get active usenet downloads (downloading or queued)

    #[cfg(feature = "sqlite")]
    pub async fn list_active(&self, user_id: Uuid) -> Result<Vec<UsenetDownloadRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let records = sqlx::query_as::<_, UsenetDownloadRecord>(
            r#"
            SELECT id, user_id, nzb_name, nzb_hash, state, progress,
                   size_bytes, downloaded_bytes, download_speed, eta_seconds,
                   error_message, retry_count, download_path,
                   library_id, episode_id, movie_id, album_id, audiobook_id,
                   indexer_id, post_process_status, created_at, updated_at, completed_at
            FROM usenet_downloads
            WHERE user_id = ?1 AND state IN ('queued', 'downloading')
            ORDER BY created_at ASC
            "#,
        )
        .bind(uuid_to_str(user_id))
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

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, data: CreateUsenetDownload) -> Result<UsenetDownloadRecord> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            INSERT INTO usenet_downloads (
                id, user_id, nzb_name, nzb_hash, state, size_bytes, download_path,
                library_id, episode_id, movie_id, album_id, audiobook_id, indexer_id,
                retry_count, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, 'queued', ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0, datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(uuid_to_str(data.user_id))
        .bind(&data.nzb_name)
        .bind(&data.nzb_hash)
        .bind(data.size_bytes)
        .bind(&data.download_path)
        .bind(data.library_id.map(uuid_to_str))
        .bind(data.episode_id.map(uuid_to_str))
        .bind(data.movie_id.map(uuid_to_str))
        .bind(data.album_id.map(uuid_to_str))
        .bind(data.audiobook_id.map(uuid_to_str))
        .bind(data.indexer_id.map(uuid_to_str))
        .execute(&self.pool)
        .await?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve usenet download after insert"))
    }

    /// Update download progress

    #[cfg(feature = "sqlite")]
    pub async fn update_progress(
        &self,
        id: Uuid,
        progress: f64,
        downloaded_bytes: i64,
        speed: i64,
        eta: Option<i32>,
    ) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;

        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET progress = ?2,
                downloaded_bytes = ?3,
                download_speed = ?4,
                eta_seconds = ?5,
                state = CASE WHEN state = 'queued' THEN 'downloading' ELSE state END,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(progress)
        .bind(downloaded_bytes)
        .bind(speed)
        .bind(eta)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark download as completed

    #[cfg(feature = "sqlite")]
    pub async fn mark_completed(&self, id: Uuid, download_path: &str) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;

        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET state = 'completed',
                progress = 100.0,
                download_path = ?2,
                completed_at = datetime('now'),
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(download_path)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark download as failed

    #[cfg(feature = "sqlite")]
    pub async fn mark_failed(&self, id: Uuid, error: &str) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;

        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET state = 'failed',
                error_message = ?2,
                retry_count = retry_count + 1,
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

    /// Pause a download

    #[cfg(feature = "sqlite")]
    pub async fn pause(&self, id: Uuid) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;

        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET state = 'paused', updated_at = datetime('now')
            WHERE id = ?1 AND state IN ('queued', 'downloading')
            "#,
        )
        .bind(uuid_to_str(id))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Resume a paused download

    #[cfg(feature = "sqlite")]
    pub async fn resume(&self, id: Uuid) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;

        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET state = 'queued', updated_at = datetime('now')
            WHERE id = ?1 AND state = 'paused'
            "#,
        )
        .bind(uuid_to_str(id))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Remove a download (soft delete)

    #[cfg(feature = "sqlite")]
    pub async fn remove(&self, id: Uuid) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;

        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET state = 'removed', updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a download permanently

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let result = sqlx::query("DELETE FROM usenet_downloads WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update post-processing status

    #[cfg(feature = "sqlite")]
    pub async fn set_post_process_status(&self, id: Uuid, status: &str) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;

        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET post_process_status = ?2, updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Link download to library item

    #[cfg(feature = "sqlite")]
    pub async fn link_to_library(
        &self,
        id: Uuid,
        library_id: Option<Uuid>,
        episode_id: Option<Uuid>,
        movie_id: Option<Uuid>,
        album_id: Option<Uuid>,
        audiobook_id: Option<Uuid>,
    ) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;

        sqlx::query(
            r#"
            UPDATE usenet_downloads
            SET library_id = ?2,
                episode_id = ?3,
                movie_id = ?4,
                album_id = ?5,
                audiobook_id = ?6,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(library_id.map(uuid_to_str))
        .bind(episode_id.map(uuid_to_str))
        .bind(movie_id.map(uuid_to_str))
        .bind(album_id.map(uuid_to_str))
        .bind(audiobook_id.map(uuid_to_str))
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
