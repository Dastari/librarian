//! Torrent files database operations
//!
//! Stores all files within torrents with live progress updates from librqbit.
//! The media_file_id column provides the canonical link from torrent content to library files.

use anyhow::Result;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// A torrent file record in the database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TorrentFileRecord {
    pub id: Uuid,
    pub torrent_id: Uuid,
    pub file_index: i32,
    pub file_path: String,
    pub relative_path: String,
    pub file_size: i64,
    pub downloaded_bytes: i64,
    pub progress: f32,
    pub media_file_id: Option<Uuid>,
    pub is_excluded: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Input for upserting a torrent file during sync
#[derive(Debug, Clone)]
pub struct UpsertTorrentFile {
    pub file_index: i32,
    pub file_path: String,
    pub relative_path: String,
    pub file_size: i64,
    pub downloaded_bytes: i64,
    pub progress: f32,
    pub is_excluded: bool,
}

/// Repository for torrent file operations
pub struct TorrentFileRepository {
    pool: PgPool,
}

impl TorrentFileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert a batch of torrent files for a torrent
    /// This is called during the periodic sync from librqbit
    pub async fn upsert_batch(
        &self,
        torrent_id: Uuid,
        files: &[UpsertTorrentFile],
    ) -> Result<()> {
        // Use a transaction for atomicity
        let mut tx = self.pool.begin().await?;

        for file in files {
            sqlx::query(
                r#"
                INSERT INTO torrent_files (
                    torrent_id, file_index, file_path, relative_path, 
                    file_size, downloaded_bytes, progress, is_excluded
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (torrent_id, file_index) 
                DO UPDATE SET 
                    file_path = EXCLUDED.file_path,
                    relative_path = EXCLUDED.relative_path,
                    file_size = EXCLUDED.file_size,
                    downloaded_bytes = EXCLUDED.downloaded_bytes,
                    progress = EXCLUDED.progress,
                    is_excluded = EXCLUDED.is_excluded,
                    updated_at = NOW()
                "#,
            )
            .bind(torrent_id)
            .bind(file.file_index)
            .bind(&file.file_path)
            .bind(&file.relative_path)
            .bind(file.file_size)
            .bind(file.downloaded_bytes)
            .bind(file.progress)
            .bind(file.is_excluded)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Get all files for a torrent
    pub async fn get_by_torrent(&self, torrent_id: Uuid) -> Result<Vec<TorrentFileRecord>> {
        let records = sqlx::query_as::<_, TorrentFileRecord>(
            "SELECT * FROM torrent_files WHERE torrent_id = $1 ORDER BY file_index",
        )
        .bind(torrent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get a specific file by torrent and file index
    pub async fn get_by_index(
        &self,
        torrent_id: Uuid,
        file_index: i32,
    ) -> Result<Option<TorrentFileRecord>> {
        let record = sqlx::query_as::<_, TorrentFileRecord>(
            "SELECT * FROM torrent_files WHERE torrent_id = $1 AND file_index = $2",
        )
        .bind(torrent_id)
        .bind(file_index)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a torrent file by its linked media_file_id
    pub async fn get_by_media_file(&self, media_file_id: Uuid) -> Result<Option<TorrentFileRecord>> {
        let record = sqlx::query_as::<_, TorrentFileRecord>(
            "SELECT * FROM torrent_files WHERE media_file_id = $1",
        )
        .bind(media_file_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Set the media_file_id for a torrent file (called after file processing)
    pub async fn set_media_file_id(
        &self,
        torrent_id: Uuid,
        file_index: i32,
        media_file_id: Uuid,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE torrent_files
            SET media_file_id = $3, updated_at = NOW()
            WHERE torrent_id = $1 AND file_index = $2
            "#,
        )
        .bind(torrent_id)
        .bind(file_index)
        .bind(media_file_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Clear the media_file_id for a torrent file
    pub async fn clear_media_file_id(&self, torrent_id: Uuid, file_index: i32) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE torrent_files
            SET media_file_id = NULL, updated_at = NOW()
            WHERE torrent_id = $1 AND file_index = $2
            "#,
        )
        .bind(torrent_id)
        .bind(file_index)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get download progress for a track (via pending_file_matches link)
    /// Returns progress (0.0-1.0) if the track is actively downloading, None otherwise
    pub async fn get_download_progress_for_track(&self, track_id: Uuid) -> Result<Option<f32>> {
        let progress: Option<(f32,)> = sqlx::query_as(
            r#"
            SELECT tf.progress
            FROM torrent_files tf
            JOIN pending_file_matches pfm ON pfm.source_type = 'torrent' 
                AND pfm.source_id = tf.torrent_id 
                AND pfm.source_file_index = tf.file_index
            JOIN torrents t ON t.id = tf.torrent_id
            WHERE pfm.track_id = $1
                AND pfm.copied_at IS NULL
                AND t.state IN ('downloading', 'queued')
                AND tf.progress < 1.0
            ORDER BY tf.updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(track_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(progress.map(|(p,)| p))
    }

    /// Get download progress for an episode (via pending_file_matches link)
    pub async fn get_download_progress_for_episode(&self, episode_id: Uuid) -> Result<Option<f32>> {
        let progress: Option<(f32,)> = sqlx::query_as(
            r#"
            SELECT tf.progress
            FROM torrent_files tf
            JOIN pending_file_matches pfm ON pfm.source_type = 'torrent' 
                AND pfm.source_id = tf.torrent_id 
                AND pfm.source_file_index = tf.file_index
            JOIN torrents t ON t.id = tf.torrent_id
            WHERE pfm.episode_id = $1
                AND pfm.copied_at IS NULL
                AND t.state IN ('downloading', 'queued')
                AND tf.progress < 1.0
            ORDER BY tf.updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(episode_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(progress.map(|(p,)| p))
    }

    /// Get download progress for a movie (via pending_file_matches link)
    pub async fn get_download_progress_for_movie(&self, movie_id: Uuid) -> Result<Option<f32>> {
        let progress: Option<(f32,)> = sqlx::query_as(
            r#"
            SELECT tf.progress
            FROM torrent_files tf
            JOIN pending_file_matches pfm ON pfm.source_type = 'torrent' 
                AND pfm.source_id = tf.torrent_id 
                AND pfm.source_file_index = tf.file_index
            JOIN torrents t ON t.id = tf.torrent_id
            WHERE pfm.movie_id = $1
                AND pfm.copied_at IS NULL
                AND t.state IN ('downloading', 'queued')
                AND tf.progress < 1.0
            ORDER BY tf.updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(movie_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(progress.map(|(p,)| p))
    }

    /// Get download progress for a chapter (via pending_file_matches link)
    pub async fn get_download_progress_for_chapter(&self, chapter_id: Uuid) -> Result<Option<f32>> {
        let progress: Option<(f32,)> = sqlx::query_as(
            r#"
            SELECT tf.progress
            FROM torrent_files tf
            JOIN pending_file_matches pfm ON pfm.source_type = 'torrent' 
                AND pfm.source_id = tf.torrent_id 
                AND pfm.source_file_index = tf.file_index
            JOIN torrents t ON t.id = tf.torrent_id
            WHERE pfm.chapter_id = $1
                AND pfm.copied_at IS NULL
                AND t.state IN ('downloading', 'queued')
                AND tf.progress < 1.0
            ORDER BY tf.updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(chapter_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(progress.map(|(p,)| p))
    }

    /// Get all actively downloading files (for subscription broadcasts)
    pub async fn get_downloading_files(&self) -> Result<Vec<TorrentFileRecord>> {
        let records = sqlx::query_as::<_, TorrentFileRecord>(
            r#"
            SELECT tf.*
            FROM torrent_files tf
            JOIN torrents t ON t.id = tf.torrent_id
            WHERE t.state IN ('downloading', 'queued')
                AND tf.progress < 1.0
                AND NOT tf.is_excluded
            ORDER BY t.id, tf.file_index
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Delete all files for a torrent (called when torrent is removed)
    /// Note: This should happen automatically via CASCADE, but provided for explicit cleanup
    pub async fn delete_by_torrent(&self, torrent_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM torrent_files WHERE torrent_id = $1")
            .bind(torrent_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Mark a file as excluded (not to be downloaded)
    pub async fn set_excluded(&self, torrent_id: Uuid, file_index: i32, excluded: bool) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE torrent_files
            SET is_excluded = $3, updated_at = NOW()
            WHERE torrent_id = $1 AND file_index = $2
            "#,
        )
        .bind(torrent_id)
        .bind(file_index)
        .bind(excluded)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
