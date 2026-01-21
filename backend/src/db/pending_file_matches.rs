//! Pending file matches database operations
//!
//! Source-agnostic tracking of file-to-library-item matches.
//! Works for any download source: torrents, usenet, IRC, library scans, manual drops.

use anyhow::Result;
use rust_decimal::Decimal;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// A pending file match record
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PendingFileMatchRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    // Source file info
    pub source_path: String,
    pub source_type: String, // 'torrent', 'usenet', 'irc', 'scan', 'manual'
    pub source_id: Option<Uuid>,
    pub source_file_index: Option<i32>,
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
    pub copied_at: Option<OffsetDateTime>,
    pub copy_error: Option<String>,
    // Timestamps
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl PendingFileMatchRecord {
    /// Returns the target type as a string
    pub fn target_type_str(&self) -> &'static str {
        if self.episode_id.is_some() {
            "episode"
        } else if self.movie_id.is_some() {
            "movie"
        } else if self.track_id.is_some() {
            "track"
        } else if self.chapter_id.is_some() {
            "chapter"
        } else {
            "unknown"
        }
    }

    /// Returns the target as a MatchTarget enum
    pub fn target_type(&self) -> Option<MatchTarget> {
        if let Some(id) = self.episode_id {
            Some(MatchTarget::Episode(id))
        } else if let Some(id) = self.movie_id {
            Some(MatchTarget::Movie(id))
        } else if let Some(id) = self.track_id {
            Some(MatchTarget::Track(id))
        } else if let Some(id) = self.chapter_id {
            Some(MatchTarget::Chapter(id))
        } else {
            None
        }
    }

    /// Returns the target ID
    pub fn target_id(&self) -> Option<Uuid> {
        self.episode_id
            .or(self.movie_id)
            .or(self.track_id)
            .or(self.chapter_id)
    }

    /// Returns true if this match has been copied to the library
    pub fn is_copied(&self) -> bool {
        self.copied_at.is_some()
    }
}

/// Match target enum for type-safe target specification
#[derive(Debug, Clone)]
pub enum MatchTarget {
    Episode(Uuid),
    Movie(Uuid),
    Track(Uuid),
    Chapter(Uuid),
}

impl MatchTarget {
    pub fn target_type(&self) -> &'static str {
        match self {
            MatchTarget::Episode(_) => "episode",
            MatchTarget::Movie(_) => "movie",
            MatchTarget::Track(_) => "track",
            MatchTarget::Chapter(_) => "chapter",
        }
    }

    pub fn id(&self) -> Uuid {
        match self {
            MatchTarget::Episode(id) => *id,
            MatchTarget::Movie(id) => *id,
            MatchTarget::Track(id) => *id,
            MatchTarget::Chapter(id) => *id,
        }
    }
}

/// Input for creating a new pending file match
#[derive(Debug, Clone)]
pub struct CreatePendingFileMatch {
    pub user_id: Uuid,
    pub source_path: String,
    pub source_type: String,
    pub source_id: Option<Uuid>,
    pub source_file_index: Option<i32>,
    pub file_size: i64,
    // Match target
    pub target: MatchTarget,
    // Match metadata
    pub match_type: String,
    pub match_confidence: Option<Decimal>,
    // Parsed quality
    pub parsed_resolution: Option<String>,
    pub parsed_codec: Option<String>,
    pub parsed_source: Option<String>,
    pub parsed_audio: Option<String>,
}

/// Repository for pending file matches
pub struct PendingFileMatchRepository {
    pool: PgPool,
}

impl PendingFileMatchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new pending file match record
    pub async fn create(&self, input: CreatePendingFileMatch) -> Result<PendingFileMatchRecord> {
        let (episode_id, movie_id, track_id, chapter_id) = match input.target {
            MatchTarget::Episode(id) => (Some(id), None, None, None),
            MatchTarget::Movie(id) => (None, Some(id), None, None),
            MatchTarget::Track(id) => (None, None, Some(id), None),
            MatchTarget::Chapter(id) => (None, None, None, Some(id)),
        };

        let record = sqlx::query_as::<_, PendingFileMatchRecord>(
            r#"
            INSERT INTO pending_file_matches (
                user_id, source_path, source_type, source_id, source_file_index, file_size,
                episode_id, movie_id, track_id, chapter_id,
                match_type, match_confidence,
                parsed_resolution, parsed_codec, parsed_source, parsed_audio
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
            "#,
        )
        .bind(input.user_id)
        .bind(&input.source_path)
        .bind(&input.source_type)
        .bind(input.source_id)
        .bind(input.source_file_index)
        .bind(input.file_size)
        .bind(episode_id)
        .bind(movie_id)
        .bind(track_id)
        .bind(chapter_id)
        .bind(&input.match_type)
        .bind(input.match_confidence)
        .bind(&input.parsed_resolution)
        .bind(&input.parsed_codec)
        .bind(&input.parsed_source)
        .bind(&input.parsed_audio)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create multiple pending file matches at once
    pub async fn create_batch(
        &self,
        inputs: Vec<CreatePendingFileMatch>,
    ) -> Result<Vec<PendingFileMatchRecord>> {
        let mut records = Vec::with_capacity(inputs.len());
        for input in inputs {
            let record = self.create(input).await?;
            records.push(record);
        }
        Ok(records)
    }

    /// Get a pending file match by ID
    pub async fn get(&self, id: Uuid) -> Result<Option<PendingFileMatchRecord>> {
        let record = sqlx::query_as::<_, PendingFileMatchRecord>(
            "SELECT * FROM pending_file_matches WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get all pending file matches for a source (e.g., all matches for a torrent)
    pub async fn list_by_source(
        &self,
        source_type: &str,
        source_id: Uuid,
    ) -> Result<Vec<PendingFileMatchRecord>> {
        let records = sqlx::query_as::<_, PendingFileMatchRecord>(
            r#"
            SELECT * FROM pending_file_matches 
            WHERE source_type = $1 AND source_id = $2
            ORDER BY source_file_index, created_at
            "#,
        )
        .bind(source_type)
        .bind(source_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get all uncopied matches for a source
    pub async fn list_uncopied_by_source(
        &self,
        source_type: &str,
        source_id: Uuid,
    ) -> Result<Vec<PendingFileMatchRecord>> {
        let records = sqlx::query_as::<_, PendingFileMatchRecord>(
            r#"
            SELECT * FROM pending_file_matches 
            WHERE source_type = $1 AND source_id = $2 AND copied_at IS NULL
            ORDER BY source_file_index, created_at
            "#,
        )
        .bind(source_type)
        .bind(source_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get all pending matches for a user
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<PendingFileMatchRecord>> {
        let records = sqlx::query_as::<_, PendingFileMatchRecord>(
            r#"
            SELECT * FROM pending_file_matches 
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get matches for a specific track
    pub async fn list_by_track(&self, track_id: Uuid) -> Result<Vec<PendingFileMatchRecord>> {
        let records = sqlx::query_as::<_, PendingFileMatchRecord>(
            "SELECT * FROM pending_file_matches WHERE track_id = $1 ORDER BY created_at DESC",
        )
        .bind(track_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get matches for a specific episode
    pub async fn list_by_episode(&self, episode_id: Uuid) -> Result<Vec<PendingFileMatchRecord>> {
        let records = sqlx::query_as::<_, PendingFileMatchRecord>(
            "SELECT * FROM pending_file_matches WHERE episode_id = $1 ORDER BY created_at DESC",
        )
        .bind(episode_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get matches for a specific movie
    pub async fn list_by_movie(&self, movie_id: Uuid) -> Result<Vec<PendingFileMatchRecord>> {
        let records = sqlx::query_as::<_, PendingFileMatchRecord>(
            "SELECT * FROM pending_file_matches WHERE movie_id = $1 ORDER BY created_at DESC",
        )
        .bind(movie_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get matches for a specific chapter
    pub async fn list_by_chapter(&self, chapter_id: Uuid) -> Result<Vec<PendingFileMatchRecord>> {
        let records = sqlx::query_as::<_, PendingFileMatchRecord>(
            "SELECT * FROM pending_file_matches WHERE chapter_id = $1 ORDER BY created_at DESC",
        )
        .bind(chapter_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Mark a match as copied (successfully processed)
    pub async fn mark_copied(&self, id: Uuid) -> Result<PendingFileMatchRecord> {
        let record = sqlx::query_as::<_, PendingFileMatchRecord>(
            r#"
            UPDATE pending_file_matches
            SET copied_at = NOW(), copy_error = NULL, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Mark a match as failed
    pub async fn mark_failed(&self, id: Uuid, error: &str) -> Result<PendingFileMatchRecord> {
        let record = sqlx::query_as::<_, PendingFileMatchRecord>(
            r#"
            UPDATE pending_file_matches
            SET copy_error = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(error)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update the target of a match (for manual matching)
    pub async fn update_target(
        &self,
        id: Uuid,
        target: MatchTarget,
    ) -> Result<PendingFileMatchRecord> {
        let (episode_id, movie_id, track_id, chapter_id) = match target {
            MatchTarget::Episode(tid) => (Some(tid), None, None, None),
            MatchTarget::Movie(tid) => (None, Some(tid), None, None),
            MatchTarget::Track(tid) => (None, None, Some(tid), None),
            MatchTarget::Chapter(tid) => (None, None, None, Some(tid)),
        };

        let record = sqlx::query_as::<_, PendingFileMatchRecord>(
            r#"
            UPDATE pending_file_matches
            SET episode_id = $2, movie_id = $3, track_id = $4, chapter_id = $5,
                match_type = 'manual', updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(episode_id)
        .bind(movie_id)
        .bind(track_id)
        .bind(chapter_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete a pending file match
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM pending_file_matches WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all matches for a source
    pub async fn delete_by_source(&self, source_type: &str, source_id: Uuid) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM pending_file_matches WHERE source_type = $1 AND source_id = $2",
        )
        .bind(source_type)
        .bind(source_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Count matches for a source
    pub async fn count_by_source(&self, source_type: &str, source_id: Uuid) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM pending_file_matches WHERE source_type = $1 AND source_id = $2",
        )
        .bind(source_type)
        .bind(source_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    /// Count uncopied matches for a source
    pub async fn count_uncopied_by_source(
        &self,
        source_type: &str,
        source_id: Uuid,
    ) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM pending_file_matches 
            WHERE source_type = $1 AND source_id = $2 AND copied_at IS NULL
            "#,
        )
        .bind(source_type)
        .bind(source_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }
}
