//! Torrent file matches database operations
//!
//! Tracks the mapping between individual files within torrents and library items.
//! This enables file-level matching instead of torrent-level matching.

use anyhow::Result;
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// A torrent file match record
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TorrentFileMatchRecord {
    pub id: Uuid,
    pub torrent_id: Uuid,
    pub file_index: i32,
    pub file_path: String,
    pub file_size: i64,
    // Match targets (only one should be set)
    pub episode_id: Option<Uuid>,
    pub movie_id: Option<Uuid>,
    pub track_id: Option<Uuid>,
    pub chapter_id: Option<Uuid>,
    // Match metadata
    pub match_type: String,
    pub match_confidence: Option<Decimal>,
    // Parsed quality info
    pub parsed_resolution: Option<String>,
    pub parsed_codec: Option<String>,
    pub parsed_source: Option<String>,
    pub parsed_audio: Option<String>,
    // Processing status
    pub skip_download: bool,
    pub processed: bool,
    pub processed_at: Option<OffsetDateTime>,
    pub media_file_id: Option<Uuid>,
    pub error_message: Option<String>,
    // Timestamps
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Input for creating a new torrent file match
#[derive(Debug, Clone)]
pub struct CreateTorrentFileMatch {
    pub torrent_id: Uuid,
    pub file_index: i32,
    pub file_path: String,
    pub file_size: i64,
    // Match target
    pub episode_id: Option<Uuid>,
    pub movie_id: Option<Uuid>,
    pub track_id: Option<Uuid>,
    pub chapter_id: Option<Uuid>,
    // Match metadata
    pub match_type: String,
    pub match_confidence: Option<Decimal>,
    // Parsed quality
    pub parsed_resolution: Option<String>,
    pub parsed_codec: Option<String>,
    pub parsed_source: Option<String>,
    pub parsed_audio: Option<String>,
    // Skip?
    pub skip_download: bool,
}

/// Update for marking a file match as processed
#[derive(Debug)]
pub struct MarkProcessed {
    pub media_file_id: Option<Uuid>,
    pub error_message: Option<String>,
}

/// Repository for torrent file matches
pub struct TorrentFileMatchRepository {
    pool: PgPool,
}

impl TorrentFileMatchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new file match record
    pub async fn create(&self, input: CreateTorrentFileMatch) -> Result<TorrentFileMatchRecord> {
        let record = sqlx::query_as::<_, TorrentFileMatchRecord>(
            r#"
            INSERT INTO torrent_file_matches (
                torrent_id, file_index, file_path, file_size,
                episode_id, movie_id, track_id, chapter_id,
                match_type, match_confidence,
                parsed_resolution, parsed_codec, parsed_source, parsed_audio,
                skip_download
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT (torrent_id, file_index)
            DO UPDATE SET
                episode_id = EXCLUDED.episode_id,
                movie_id = EXCLUDED.movie_id,
                track_id = EXCLUDED.track_id,
                chapter_id = EXCLUDED.chapter_id,
                match_type = EXCLUDED.match_type,
                match_confidence = EXCLUDED.match_confidence,
                parsed_resolution = EXCLUDED.parsed_resolution,
                parsed_codec = EXCLUDED.parsed_codec,
                parsed_source = EXCLUDED.parsed_source,
                parsed_audio = EXCLUDED.parsed_audio,
                skip_download = EXCLUDED.skip_download,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(input.torrent_id)
        .bind(input.file_index)
        .bind(&input.file_path)
        .bind(input.file_size)
        .bind(input.episode_id)
        .bind(input.movie_id)
        .bind(input.track_id)
        .bind(input.chapter_id)
        .bind(&input.match_type)
        .bind(input.match_confidence)
        .bind(&input.parsed_resolution)
        .bind(&input.parsed_codec)
        .bind(&input.parsed_source)
        .bind(&input.parsed_audio)
        .bind(input.skip_download)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create multiple file matches at once
    pub async fn create_batch(
        &self,
        inputs: Vec<CreateTorrentFileMatch>,
    ) -> Result<Vec<TorrentFileMatchRecord>> {
        let mut records = Vec::with_capacity(inputs.len());
        for input in inputs {
            let record = self.create(input).await?;
            records.push(record);
        }
        Ok(records)
    }

    /// Get all file matches for a torrent
    pub async fn list_by_torrent(&self, torrent_id: Uuid) -> Result<Vec<TorrentFileMatchRecord>> {
        let records = sqlx::query_as::<_, TorrentFileMatchRecord>(
            "SELECT * FROM torrent_file_matches WHERE torrent_id = $1 ORDER BY file_index",
        )
        .bind(torrent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get unprocessed file matches for a torrent
    pub async fn list_unprocessed(&self, torrent_id: Uuid) -> Result<Vec<TorrentFileMatchRecord>> {
        let records = sqlx::query_as::<_, TorrentFileMatchRecord>(
            r#"
            SELECT * FROM torrent_file_matches 
            WHERE torrent_id = $1 AND NOT processed AND NOT skip_download
            ORDER BY file_index
            "#,
        )
        .bind(torrent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get file matches for a specific episode
    pub async fn list_by_episode(&self, episode_id: Uuid) -> Result<Vec<TorrentFileMatchRecord>> {
        let records = sqlx::query_as::<_, TorrentFileMatchRecord>(
            "SELECT * FROM torrent_file_matches WHERE episode_id = $1 ORDER BY created_at",
        )
        .bind(episode_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get file matches for a specific movie
    pub async fn list_by_movie(&self, movie_id: Uuid) -> Result<Vec<TorrentFileMatchRecord>> {
        let records = sqlx::query_as::<_, TorrentFileMatchRecord>(
            "SELECT * FROM torrent_file_matches WHERE movie_id = $1 ORDER BY created_at",
        )
        .bind(movie_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get file matches for a specific track
    pub async fn list_by_track(&self, track_id: Uuid) -> Result<Vec<TorrentFileMatchRecord>> {
        let records = sqlx::query_as::<_, TorrentFileMatchRecord>(
            "SELECT * FROM torrent_file_matches WHERE track_id = $1 ORDER BY created_at",
        )
        .bind(track_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Mark a file match as processed
    pub async fn mark_processed(
        &self,
        id: Uuid,
        update: MarkProcessed,
    ) -> Result<TorrentFileMatchRecord> {
        let record = sqlx::query_as::<_, TorrentFileMatchRecord>(
            r#"
            UPDATE torrent_file_matches
            SET processed = true,
                processed_at = NOW(),
                media_file_id = $2,
                error_message = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(update.media_file_id)
        .bind(&update.error_message)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Mark a file match as skipped (already have the item)
    pub async fn mark_skipped(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE torrent_file_matches
            SET skip_download = true, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete all file matches for a torrent
    pub async fn delete_by_torrent(&self, torrent_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM torrent_file_matches WHERE torrent_id = $1")
            .bind(torrent_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Check if any files in a torrent are downloading a specific episode
    pub async fn is_episode_downloading(&self, episode_id: Uuid) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM torrent_file_matches tfm
            JOIN torrents t ON t.id = tfm.torrent_id
            WHERE tfm.episode_id = $1 
              AND NOT tfm.processed 
              AND NOT tfm.skip_download
              AND t.state IN ('queued', 'downloading')
            "#,
        )
        .bind(episode_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Check if any files in a torrent are downloading a specific movie
    pub async fn is_movie_downloading(&self, movie_id: Uuid) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM torrent_file_matches tfm
            JOIN torrents t ON t.id = tfm.torrent_id
            WHERE tfm.movie_id = $1 
              AND NOT tfm.processed 
              AND NOT tfm.skip_download
              AND t.state IN ('queued', 'downloading')
            "#,
        )
        .bind(movie_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Check if any files in a torrent are downloading a specific track
    pub async fn is_track_downloading(&self, track_id: Uuid) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM torrent_file_matches tfm
            JOIN torrents t ON t.id = tfm.torrent_id
            WHERE tfm.track_id = $1 
              AND NOT tfm.processed 
              AND NOT tfm.skip_download
              AND t.state IN ('queued', 'downloading')
            "#,
        )
        .bind(track_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Get count of matched vs unmatched files for a torrent
    pub async fn get_match_stats(&self, torrent_id: Uuid) -> Result<(i64, i64, i64)> {
        let row: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT 
                COUNT(*) FILTER (WHERE episode_id IS NOT NULL OR movie_id IS NOT NULL OR track_id IS NOT NULL OR chapter_id IS NOT NULL) as matched,
                COUNT(*) FILTER (WHERE episode_id IS NULL AND movie_id IS NULL AND track_id IS NULL AND chapter_id IS NULL) as unmatched,
                COUNT(*) FILTER (WHERE skip_download = true) as skipped
            FROM torrent_file_matches
            WHERE torrent_id = $1
            "#,
        )
        .bind(torrent_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Count total file matches for a torrent
    pub async fn count_by_torrent_id(&self, torrent_id: Uuid) -> Result<i64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM torrent_file_matches WHERE torrent_id = $1")
                .bind(torrent_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(count)
    }
}
