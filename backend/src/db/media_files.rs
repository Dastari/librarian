//! Media files database repository

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Media file record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MediaFileRecord {
    pub id: Uuid,
    pub media_item_id: Option<Uuid>,
    pub library_id: Uuid,
    pub path: String,
    pub size_bytes: i64,
    pub container: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration: Option<i32>,
    pub bitrate: Option<i32>,
    pub file_hash: Option<String>,
    pub episode_id: Option<Uuid>,
    pub relative_path: Option<String>,
    pub original_name: Option<String>,
    pub video_bitrate: Option<i32>,
    pub audio_channels: Option<String>,
    pub audio_language: Option<String>,
    pub resolution: Option<String>,
    pub is_hdr: Option<bool>,
    pub hdr_type: Option<String>,
    pub organized: bool,
    pub organized_at: Option<chrono::DateTime<chrono::Utc>>,
    pub original_path: Option<String>,
    pub added_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Input for creating a media file
#[derive(Debug)]
pub struct CreateMediaFile {
    pub library_id: Uuid,
    pub path: String,
    pub size_bytes: i64,
    pub container: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration: Option<i32>,
    pub bitrate: Option<i32>,
    pub file_hash: Option<String>,
    pub episode_id: Option<Uuid>,
    pub relative_path: Option<String>,
    pub original_name: Option<String>,
    pub resolution: Option<String>,
    pub is_hdr: Option<bool>,
    pub hdr_type: Option<String>,
}

pub struct MediaFileRepository {
    pool: PgPool,
}

impl MediaFileRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all media files for a library
    pub async fn list_by_library(&self, library_id: Uuid) -> Result<Vec<MediaFileRecord>> {
        let records = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, media_item_id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, added_at, modified_at
            FROM media_files
            WHERE library_id = $1
            ORDER BY path
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Check if a file path already exists
    pub async fn exists_by_path(&self, path: &str) -> Result<bool> {
        let result = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM media_files WHERE path = $1"
        )
        .bind(path)
        .fetch_one(&self.pool)
        .await?;

        Ok(result > 0)
    }

    /// Get a media file by path
    pub async fn get_by_path(&self, path: &str) -> Result<Option<MediaFileRecord>> {
        let record = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, media_item_id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, added_at, modified_at
            FROM media_files
            WHERE path = $1
            "#,
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create a new media file
    pub async fn create(&self, input: CreateMediaFile) -> Result<MediaFileRecord> {
        let record = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            INSERT INTO media_files (
                library_id, path, size, container, video_codec, audio_codec,
                width, height, duration, bitrate, file_hash, episode_id,
                relative_path, original_name, resolution, is_hdr, hdr_type
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING id, media_item_id, library_id, path, size as size_bytes, 
                      container, video_codec, audio_codec, width, height,
                      duration, bitrate, file_hash, episode_id, relative_path,
                      original_name, video_bitrate, audio_channels, audio_language,
                      resolution, is_hdr, hdr_type, organized, organized_at,
                      original_path, added_at, modified_at
            "#,
        )
        .bind(input.library_id)
        .bind(&input.path)
        .bind(input.size_bytes)
        .bind(&input.container)
        .bind(&input.video_codec)
        .bind(&input.audio_codec)
        .bind(input.width)
        .bind(input.height)
        .bind(input.duration)
        .bind(input.bitrate)
        .bind(&input.file_hash)
        .bind(input.episode_id)
        .bind(&input.relative_path)
        .bind(&input.original_name)
        .bind(&input.resolution)
        .bind(input.is_hdr)
        .bind(&input.hdr_type)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Upsert a media file (insert or update if path exists)
    pub async fn upsert(&self, input: CreateMediaFile) -> Result<MediaFileRecord> {
        let record = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            INSERT INTO media_files (
                library_id, path, size, container, video_codec, audio_codec,
                width, height, duration, bitrate, file_hash, episode_id,
                relative_path, original_name, resolution, is_hdr, hdr_type
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            ON CONFLICT (path) DO UPDATE SET
                size = EXCLUDED.size,
                container = EXCLUDED.container,
                video_codec = EXCLUDED.video_codec,
                audio_codec = EXCLUDED.audio_codec,
                width = EXCLUDED.width,
                height = EXCLUDED.height,
                duration = EXCLUDED.duration,
                bitrate = EXCLUDED.bitrate,
                file_hash = EXCLUDED.file_hash,
                resolution = EXCLUDED.resolution,
                is_hdr = EXCLUDED.is_hdr,
                hdr_type = EXCLUDED.hdr_type,
                modified_at = NOW()
            RETURNING id, media_item_id, library_id, path, size as size_bytes, 
                      container, video_codec, audio_codec, width, height,
                      duration, bitrate, file_hash, episode_id, relative_path,
                      original_name, video_bitrate, audio_channels, audio_language,
                      resolution, is_hdr, hdr_type, organized, organized_at,
                      original_path, added_at, modified_at
            "#,
        )
        .bind(input.library_id)
        .bind(&input.path)
        .bind(input.size_bytes)
        .bind(&input.container)
        .bind(&input.video_codec)
        .bind(&input.audio_codec)
        .bind(input.width)
        .bind(input.height)
        .bind(input.duration)
        .bind(input.bitrate)
        .bind(&input.file_hash)
        .bind(input.episode_id)
        .bind(&input.relative_path)
        .bind(&input.original_name)
        .bind(&input.resolution)
        .bind(input.is_hdr)
        .bind(&input.hdr_type)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete media files that no longer exist on disk
    pub async fn delete_missing(&self, library_id: Uuid, existing_paths: &[String]) -> Result<u64> {
        if existing_paths.is_empty() {
            // Delete all files for this library if none exist
            let result = sqlx::query("DELETE FROM media_files WHERE library_id = $1")
                .bind(library_id)
                .execute(&self.pool)
                .await?;
            return Ok(result.rows_affected());
        }

        let result = sqlx::query(
            "DELETE FROM media_files WHERE library_id = $1 AND path != ALL($2)"
        )
        .bind(library_id)
        .bind(existing_paths)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get count of files in a library
    pub async fn count_by_library(&self, library_id: Uuid) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM media_files WHERE library_id = $1"
        )
        .bind(library_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Link a media file to an episode
    pub async fn link_to_episode(&self, file_id: Uuid, episode_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE media_files SET episode_id = $2 WHERE id = $1")
            .bind(file_id)
            .bind(episode_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get unorganized files for a library
    pub async fn list_unorganized_by_library(&self, library_id: Uuid) -> Result<Vec<MediaFileRecord>> {
        let records = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, media_item_id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, added_at, modified_at
            FROM media_files
            WHERE library_id = $1 AND organized = false
            ORDER BY path
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Mark a file as organized (moved to library structure)
    pub async fn mark_organized(&self, file_id: Uuid, new_path: &str, original_path: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE media_files SET 
                path = $2, 
                original_path = $3,
                organized = true, 
                organized_at = NOW(),
                modified_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(file_id)
        .bind(new_path)
        .bind(original_path)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update only the path of a media file
    pub async fn update_path(&self, file_id: Uuid, new_path: &str) -> Result<()> {
        sqlx::query("UPDATE media_files SET path = $2, modified_at = NOW() WHERE id = $1")
            .bind(file_id)
            .bind(new_path)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get a media file by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<MediaFileRecord>> {
        let record = sqlx::query_as::<_, MediaFileRecord>(
            r#"
            SELECT id, media_item_id, library_id, path, size as size_bytes, 
                   container, video_codec, audio_codec, width, height,
                   duration, bitrate, file_hash, episode_id, relative_path,
                   original_name, video_bitrate, audio_channels, audio_language,
                   resolution, is_hdr, hdr_type, organized, organized_at,
                   original_path, added_at, modified_at
            FROM media_files
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }
}
