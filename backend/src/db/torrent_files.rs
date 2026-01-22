//! Torrent files database operations
//!
//! Stores all files within torrents with live progress updates from librqbit.
//! The media_file_id column provides the canonical link from torrent content to library files.

use anyhow::Result;
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "postgres")]
type DbPool = PgPool;
#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

/// A torrent file record in the database
#[derive(Debug, Clone)]
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

#[cfg(feature = "postgres")]
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for TorrentFileRecord {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            torrent_id: row.try_get("torrent_id")?,
            file_index: row.try_get("file_index")?,
            file_path: row.try_get("file_path")?,
            relative_path: row.try_get("relative_path")?,
            file_size: row.try_get("file_size")?,
            downloaded_bytes: row.try_get("downloaded_bytes")?,
            progress: row.try_get("progress")?,
            media_file_id: row.try_get("media_file_id")?,
            is_excluded: row.try_get("is_excluded")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for TorrentFileRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use crate::db::sqlite_helpers::{int_to_bool, str_to_uuid};
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let torrent_id_str: String = row.try_get("torrent_id")?;
        let media_file_id_str: Option<String> = row.try_get("media_file_id")?;
        let is_excluded_int: i32 = row.try_get("is_excluded")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;

        // SQLite stores floats as REAL (f64), so we need to cast
        let progress: f64 = row.try_get("progress")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            torrent_id: str_to_uuid(&torrent_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            file_index: row.try_get("file_index")?,
            file_path: row.try_get("file_path")?,
            relative_path: row.try_get("relative_path")?,
            file_size: row.try_get("file_size")?,
            downloaded_bytes: row.try_get("downloaded_bytes")?,
            progress: progress as f32,
            media_file_id: media_file_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            is_excluded: int_to_bool(is_excluded_int),
            created_at: time::OffsetDateTime::parse(
                &created_str,
                &time::format_description::well_known::Rfc3339,
            )
            .or_else(|_| {
                // Try SQLite datetime format
                let format = time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
                time::PrimitiveDateTime::parse(&created_str, &format)
                    .map(|dt| dt.assume_utc())
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))
            })?,
            updated_at: time::OffsetDateTime::parse(
                &updated_str,
                &time::format_description::well_known::Rfc3339,
            )
            .or_else(|_| {
                let format = time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
                time::PrimitiveDateTime::parse(&updated_str, &format)
                    .map(|dt| dt.assume_utc())
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))
            })?,
        })
    }
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
    pool: DbPool,
}

impl TorrentFileRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Upsert a batch of torrent files for a torrent
    /// This is called during the periodic sync from librqbit
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn upsert_batch(
        &self,
        torrent_id: Uuid,
        files: &[UpsertTorrentFile],
    ) -> Result<()> {
        use crate::db::sqlite_helpers::{bool_to_int, uuid_to_str};

        // Use a transaction for atomicity
        let mut tx = self.pool.begin().await?;
        let torrent_id_str = uuid_to_str(torrent_id);

        for file in files {
            sqlx::query(
                r#"
                INSERT INTO torrent_files (
                    torrent_id, file_index, file_path, relative_path, 
                    file_size, downloaded_bytes, progress, is_excluded
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT (torrent_id, file_index) 
                DO UPDATE SET 
                    file_path = excluded.file_path,
                    relative_path = excluded.relative_path,
                    file_size = excluded.file_size,
                    downloaded_bytes = excluded.downloaded_bytes,
                    progress = excluded.progress,
                    is_excluded = excluded.is_excluded,
                    updated_at = datetime('now')
                "#,
            )
            .bind(&torrent_id_str)
            .bind(file.file_index)
            .bind(&file.file_path)
            .bind(&file.relative_path)
            .bind(file.file_size)
            .bind(file.downloaded_bytes)
            .bind(file.progress as f64)
            .bind(bool_to_int(file.is_excluded))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Get all files for a torrent
    #[cfg(feature = "postgres")]
    pub async fn get_by_torrent(&self, torrent_id: Uuid) -> Result<Vec<TorrentFileRecord>> {
        let records = sqlx::query_as::<_, TorrentFileRecord>(
            "SELECT * FROM torrent_files WHERE torrent_id = $1 ORDER BY file_index",
        )
        .bind(torrent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    #[cfg(feature = "sqlite")]
    pub async fn get_by_torrent(&self, torrent_id: Uuid) -> Result<Vec<TorrentFileRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let records = sqlx::query_as::<_, TorrentFileRecord>(
            "SELECT * FROM torrent_files WHERE torrent_id = ?1 ORDER BY file_index",
        )
        .bind(uuid_to_str(torrent_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get a specific file by torrent and file index
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn get_by_index(
        &self,
        torrent_id: Uuid,
        file_index: i32,
    ) -> Result<Option<TorrentFileRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let record = sqlx::query_as::<_, TorrentFileRecord>(
            "SELECT * FROM torrent_files WHERE torrent_id = ?1 AND file_index = ?2",
        )
        .bind(uuid_to_str(torrent_id))
        .bind(file_index)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a torrent file by its linked media_file_id
    #[cfg(feature = "postgres")]
    pub async fn get_by_media_file(&self, media_file_id: Uuid) -> Result<Option<TorrentFileRecord>> {
        let record = sqlx::query_as::<_, TorrentFileRecord>(
            "SELECT * FROM torrent_files WHERE media_file_id = $1",
        )
        .bind(media_file_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    pub async fn get_by_media_file(&self, media_file_id: Uuid) -> Result<Option<TorrentFileRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let record = sqlx::query_as::<_, TorrentFileRecord>(
            "SELECT * FROM torrent_files WHERE media_file_id = ?1",
        )
        .bind(uuid_to_str(media_file_id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Set the media_file_id for a torrent file (called after file processing)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn set_media_file_id(
        &self,
        torrent_id: Uuid,
        file_index: i32,
        media_file_id: Uuid,
    ) -> Result<bool> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let result = sqlx::query(
            r#"
            UPDATE torrent_files
            SET media_file_id = ?3, updated_at = datetime('now')
            WHERE torrent_id = ?1 AND file_index = ?2
            "#,
        )
        .bind(uuid_to_str(torrent_id))
        .bind(file_index)
        .bind(uuid_to_str(media_file_id))
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Clear the media_file_id for a torrent file
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn clear_media_file_id(&self, torrent_id: Uuid, file_index: i32) -> Result<bool> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let result = sqlx::query(
            r#"
            UPDATE torrent_files
            SET media_file_id = NULL, updated_at = datetime('now')
            WHERE torrent_id = ?1 AND file_index = ?2
            "#,
        )
        .bind(uuid_to_str(torrent_id))
        .bind(file_index)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get download progress for a track (via pending_file_matches link)
    /// Returns progress (0.0-1.0) if the track is actively downloading, None otherwise
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn get_download_progress_for_track(&self, track_id: Uuid) -> Result<Option<f32>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let progress: Option<(f64,)> = sqlx::query_as(
            r#"
            SELECT tf.progress
            FROM torrent_files tf
            JOIN pending_file_matches pfm ON pfm.source_type = 'torrent' 
                AND pfm.source_id = tf.torrent_id 
                AND pfm.source_file_index = tf.file_index
            JOIN torrents t ON t.id = tf.torrent_id
            WHERE pfm.track_id = ?1
                AND pfm.copied_at IS NULL
                AND t.state IN ('downloading', 'queued')
                AND tf.progress < 1.0
            ORDER BY tf.updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(uuid_to_str(track_id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(progress.map(|(p,)| p as f32))
    }

    /// Get download progress for an episode (via pending_file_matches link)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn get_download_progress_for_episode(&self, episode_id: Uuid) -> Result<Option<f32>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let progress: Option<(f64,)> = sqlx::query_as(
            r#"
            SELECT tf.progress
            FROM torrent_files tf
            JOIN pending_file_matches pfm ON pfm.source_type = 'torrent' 
                AND pfm.source_id = tf.torrent_id 
                AND pfm.source_file_index = tf.file_index
            JOIN torrents t ON t.id = tf.torrent_id
            WHERE pfm.episode_id = ?1
                AND pfm.copied_at IS NULL
                AND t.state IN ('downloading', 'queued')
                AND tf.progress < 1.0
            ORDER BY tf.updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(uuid_to_str(episode_id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(progress.map(|(p,)| p as f32))
    }

    /// Get download progress for a movie (via pending_file_matches link)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn get_download_progress_for_movie(&self, movie_id: Uuid) -> Result<Option<f32>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let progress: Option<(f64,)> = sqlx::query_as(
            r#"
            SELECT tf.progress
            FROM torrent_files tf
            JOIN pending_file_matches pfm ON pfm.source_type = 'torrent' 
                AND pfm.source_id = tf.torrent_id 
                AND pfm.source_file_index = tf.file_index
            JOIN torrents t ON t.id = tf.torrent_id
            WHERE pfm.movie_id = ?1
                AND pfm.copied_at IS NULL
                AND t.state IN ('downloading', 'queued')
                AND tf.progress < 1.0
            ORDER BY tf.updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(uuid_to_str(movie_id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(progress.map(|(p,)| p as f32))
    }

    /// Get download progress for a chapter (via pending_file_matches link)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn get_download_progress_for_chapter(&self, chapter_id: Uuid) -> Result<Option<f32>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let progress: Option<(f64,)> = sqlx::query_as(
            r#"
            SELECT tf.progress
            FROM torrent_files tf
            JOIN pending_file_matches pfm ON pfm.source_type = 'torrent' 
                AND pfm.source_id = tf.torrent_id 
                AND pfm.source_file_index = tf.file_index
            JOIN torrents t ON t.id = tf.torrent_id
            WHERE pfm.chapter_id = ?1
                AND pfm.copied_at IS NULL
                AND t.state IN ('downloading', 'queued')
                AND tf.progress < 1.0
            ORDER BY tf.updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(uuid_to_str(chapter_id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(progress.map(|(p,)| p as f32))
    }

    /// Get all actively downloading files (for subscription broadcasts)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn get_downloading_files(&self) -> Result<Vec<TorrentFileRecord>> {
        let records = sqlx::query_as::<_, TorrentFileRecord>(
            r#"
            SELECT tf.*
            FROM torrent_files tf
            JOIN torrents t ON t.id = tf.torrent_id
            WHERE t.state IN ('downloading', 'queued')
                AND tf.progress < 1.0
                AND tf.is_excluded = 0
            ORDER BY t.id, tf.file_index
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Delete all files for a torrent (called when torrent is removed)
    /// Note: This should happen automatically via CASCADE, but provided for explicit cleanup
    #[cfg(feature = "postgres")]
    pub async fn delete_by_torrent(&self, torrent_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM torrent_files WHERE torrent_id = $1")
            .bind(torrent_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    #[cfg(feature = "sqlite")]
    pub async fn delete_by_torrent(&self, torrent_id: Uuid) -> Result<u64> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let result = sqlx::query("DELETE FROM torrent_files WHERE torrent_id = ?1")
            .bind(uuid_to_str(torrent_id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Mark a file as excluded (not to be downloaded)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn set_excluded(&self, torrent_id: Uuid, file_index: i32, excluded: bool) -> Result<bool> {
        use crate::db::sqlite_helpers::{bool_to_int, uuid_to_str};

        let result = sqlx::query(
            r#"
            UPDATE torrent_files
            SET is_excluded = ?3, updated_at = datetime('now')
            WHERE torrent_id = ?1 AND file_index = ?2
            "#,
        )
        .bind(uuid_to_str(torrent_id))
        .bind(file_index)
        .bind(bool_to_int(excluded))
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
