//! Pending file matches database operations
//!
//! Source-agnostic tracking of file-to-library-item matches.
//! Works for any download source: torrents, usenet, IRC, library scans, manual drops.

use anyhow::Result;
use chrono::{DateTime, Utc};
use rust_decimal::{Decimal, prelude::{FromPrimitive, ToPrimitive}};
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite")]
use crate::db::sqlite_helpers::{str_to_datetime, str_to_uuid, uuid_to_str};

#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

/// A pending file match record
#[derive(Debug, Clone)]
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
    pub unmatched_reason: Option<String>,
    // Match metadata
    pub match_type: String,
    pub match_confidence: Option<Decimal>,
    pub match_attempts: i32,
    pub verification_status: Option<String>,
    pub verification_reason: Option<String>,
    // Parsed quality info
    pub parsed_resolution: Option<String>,
    pub parsed_codec: Option<String>,
    pub parsed_source: Option<String>,
    pub parsed_audio: Option<String>,
    // Processing status
    pub copied_at: Option<DateTime<Utc>>,
    pub copy_error: Option<String>,
    pub copy_attempts: i32,
    // Timestamps
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for PendingFileMatchRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use std::str::FromStr;

        let id_str: String = row.try_get("id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let source_id_str: Option<String> = row.try_get("source_id")?;
        let episode_id_str: Option<String> = row.try_get("episode_id")?;
        let movie_id_str: Option<String> = row.try_get("movie_id")?;
        let track_id_str: Option<String> = row.try_get("track_id")?;
        let chapter_id_str: Option<String> = row.try_get("chapter_id")?;
        let match_confidence_f64: Option<f64> = row.try_get("match_confidence")?;
        let copied_at_str: Option<String> = row.try_get("copied_at")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            source_path: row.try_get("source_path")?,
            source_type: row.try_get("source_type")?,
            source_id: source_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            source_file_index: row.try_get("source_file_index")?,
            file_size: row.try_get("file_size")?,
            episode_id: episode_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            movie_id: movie_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            track_id: track_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            chapter_id: chapter_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            unmatched_reason: row.try_get("unmatched_reason")?,
            match_type: row.try_get("match_type")?,
            match_confidence: match_confidence_f64
                .and_then(|f| Decimal::from_f64(f)),
            match_attempts: row.try_get("match_attempts")?,
            verification_status: row.try_get("verification_status")?,
            verification_reason: row.try_get("verification_reason")?,
            parsed_resolution: row.try_get("parsed_resolution")?,
            parsed_codec: row.try_get("parsed_codec")?,
            parsed_source: row.try_get("parsed_source")?,
            parsed_audio: row.try_get("parsed_audio")?,
            copied_at: copied_at_str
                .map(|s| str_to_datetime(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            copy_error: row.try_get("copy_error")?,
            copy_attempts: row.try_get("copy_attempts")?,
            created_at: str_to_datetime(&created_at_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_at_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
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
            "unmatched"
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
    pub target: Option<MatchTarget>,
    pub unmatched_reason: Option<String>,
    // Match metadata
    pub match_type: String,
    pub match_confidence: Option<Decimal>,
    pub match_attempts: i32,
    // Parsed quality
    pub parsed_resolution: Option<String>,
    pub parsed_codec: Option<String>,
    pub parsed_source: Option<String>,
    pub parsed_audio: Option<String>,
}

/// Repository for pending file matches
pub struct PendingFileMatchRepository {
    pool: DbPool,
}

impl PendingFileMatchRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new pending file match record

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, input: CreatePendingFileMatch) -> Result<PendingFileMatchRecord> {
        let id = Uuid::new_v4();
        let (episode_id, movie_id, track_id, chapter_id) = match input.target {
            Some(MatchTarget::Episode(tid)) => (Some(tid), None, None, None),
            Some(MatchTarget::Movie(tid)) => (None, Some(tid), None, None),
            Some(MatchTarget::Track(tid)) => (None, None, Some(tid), None),
            Some(MatchTarget::Chapter(tid)) => (None, None, None, Some(tid)),
            None => (None, None, None, None),
        };

        sqlx::query(
            r#"
            INSERT INTO pending_file_matches (
                id, user_id, source_path, source_type, source_id, source_file_index, file_size,
                episode_id, movie_id, track_id, chapter_id, unmatched_reason,
                match_type, match_confidence, match_attempts, verification_status, verification_reason,
                parsed_resolution, parsed_codec, parsed_source, parsed_audio,
                copy_attempts, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, datetime('now'), datetime('now'))
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(uuid_to_str(input.user_id))
        .bind(&input.source_path)
        .bind(&input.source_type)
        .bind(input.source_id.map(uuid_to_str))
        .bind(input.source_file_index)
        .bind(input.file_size)
        .bind(episode_id.map(uuid_to_str))
        .bind(movie_id.map(uuid_to_str))
        .bind(track_id.map(uuid_to_str))
        .bind(chapter_id.map(uuid_to_str))
        .bind(&input.unmatched_reason)
        .bind(&input.match_type)
        .bind(input.match_confidence.and_then(|d| d.to_f64()))
        .bind(input.match_attempts)
        .bind::<Option<String>>(None)
        .bind::<Option<String>>(None)
        .bind(&input.parsed_resolution)
        .bind(&input.parsed_codec)
        .bind(&input.parsed_source)
        .bind(&input.parsed_audio)
        .bind(0_i32)
        .execute(&self.pool)
        .await?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve pending file match after insert"))
    }

    /// Upsert an unmatched entry for a source file
    #[cfg(feature = "sqlite")]
    pub async fn upsert_unmatched(
        &self,
        input: CreatePendingFileMatch,
    ) -> Result<PendingFileMatchRecord> {
        if input.target.is_some() {
            return self.create(input).await;
        }

        let record = sqlx::query_as::<_, PendingFileMatchRecord>(
            r#"
            SELECT * FROM pending_file_matches
            WHERE source_type = ?1
              AND source_path = ?2
              AND (source_id = ?3 OR (source_id IS NULL AND ?3 IS NULL))
              AND (source_file_index = ?4 OR (source_file_index IS NULL AND ?4 IS NULL))
              AND episode_id IS NULL
              AND movie_id IS NULL
              AND track_id IS NULL
              AND chapter_id IS NULL
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(&input.source_type)
        .bind(&input.source_path)
        .bind(input.source_id.map(uuid_to_str))
        .bind(input.source_file_index)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(existing) = record {
            sqlx::query(
                r#"
                UPDATE pending_file_matches
                SET file_size = ?2,
                    unmatched_reason = ?3,
                    match_type = ?4,
                    match_confidence = ?5,
                    match_attempts = match_attempts + 1,
                    parsed_resolution = ?6,
                    parsed_codec = ?7,
                    parsed_source = ?8,
                    parsed_audio = ?9,
                    updated_at = datetime('now')
                WHERE id = ?1
                "#,
            )
            .bind(uuid_to_str(existing.id))
            .bind(input.file_size)
            .bind(&input.unmatched_reason)
            .bind(&input.match_type)
            .bind(input.match_confidence.and_then(|d| d.to_f64()))
            .bind(&input.parsed_resolution)
            .bind(&input.parsed_codec)
            .bind(&input.parsed_source)
            .bind(&input.parsed_audio)
            .execute(&self.pool)
            .await?;

            return self.get(existing.id).await?
                .ok_or_else(|| anyhow::anyhow!("Failed to retrieve pending file match after update"));
        }

        self.create(input).await
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

    #[cfg(feature = "sqlite")]
    pub async fn get(&self, id: Uuid) -> Result<Option<PendingFileMatchRecord>> {
        let record = sqlx::query_as::<_, PendingFileMatchRecord>(
            "SELECT * FROM pending_file_matches WHERE id = ?1",
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get all pending file matches for a source (e.g., all matches for a torrent)

    #[cfg(feature = "sqlite")]
    pub async fn list_by_source(
        &self,
        source_type: &str,
        source_id: Uuid,
    ) -> Result<Vec<PendingFileMatchRecord>> {
        let records = sqlx::query_as::<_, PendingFileMatchRecord>(
            r#"
            SELECT * FROM pending_file_matches 
            WHERE source_type = ?1 AND source_id = ?2
            ORDER BY source_file_index, created_at
            "#,
        )
        .bind(source_type)
        .bind(uuid_to_str(source_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get all uncopied matches for a source

    #[cfg(feature = "sqlite")]
    pub async fn list_uncopied_by_source(
        &self,
        source_type: &str,
        source_id: Uuid,
    ) -> Result<Vec<PendingFileMatchRecord>> {
        let records = sqlx::query_as::<_, PendingFileMatchRecord>(
            r#"
            SELECT * FROM pending_file_matches 
            WHERE source_type = ?1
              AND source_id = ?2
              AND copied_at IS NULL
              AND (episode_id IS NOT NULL OR movie_id IS NOT NULL OR track_id IS NOT NULL OR chapter_id IS NOT NULL)
            ORDER BY source_file_index, created_at
            "#,
        )
        .bind(source_type)
        .bind(uuid_to_str(source_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get all pending matches for a user

    #[cfg(feature = "sqlite")]
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<PendingFileMatchRecord>> {
        let records = sqlx::query_as::<_, PendingFileMatchRecord>(
            r#"
            SELECT * FROM pending_file_matches 
            WHERE user_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get matches for a specific track

    #[cfg(feature = "sqlite")]
    pub async fn list_by_track(&self, track_id: Uuid) -> Result<Vec<PendingFileMatchRecord>> {
        let records = sqlx::query_as::<_, PendingFileMatchRecord>(
            "SELECT * FROM pending_file_matches WHERE track_id = ?1 ORDER BY created_at DESC",
        )
        .bind(uuid_to_str(track_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get matches for a specific episode

    #[cfg(feature = "sqlite")]
    pub async fn list_by_episode(&self, episode_id: Uuid) -> Result<Vec<PendingFileMatchRecord>> {
        let records = sqlx::query_as::<_, PendingFileMatchRecord>(
            "SELECT * FROM pending_file_matches WHERE episode_id = ?1 ORDER BY created_at DESC",
        )
        .bind(uuid_to_str(episode_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get matches for a specific movie

    #[cfg(feature = "sqlite")]
    pub async fn list_by_movie(&self, movie_id: Uuid) -> Result<Vec<PendingFileMatchRecord>> {
        let records = sqlx::query_as::<_, PendingFileMatchRecord>(
            "SELECT * FROM pending_file_matches WHERE movie_id = ?1 ORDER BY created_at DESC",
        )
        .bind(uuid_to_str(movie_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get matches for a specific chapter

    #[cfg(feature = "sqlite")]
    pub async fn list_by_chapter(&self, chapter_id: Uuid) -> Result<Vec<PendingFileMatchRecord>> {
        let records = sqlx::query_as::<_, PendingFileMatchRecord>(
            "SELECT * FROM pending_file_matches WHERE chapter_id = ?1 ORDER BY created_at DESC",
        )
        .bind(uuid_to_str(chapter_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Mark a match as copied (successfully processed)

    #[cfg(feature = "sqlite")]
    pub async fn mark_copied(&self, id: Uuid) -> Result<PendingFileMatchRecord> {
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            UPDATE pending_file_matches
            SET copied_at = datetime('now'), copy_error = NULL, updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(&id_str)
        .execute(&self.pool)
        .await?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve pending file match after update"))
    }

    /// Mark a match as failed

    #[cfg(feature = "sqlite")]
    pub async fn mark_failed(&self, id: Uuid, error: &str) -> Result<PendingFileMatchRecord> {
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            UPDATE pending_file_matches
            SET copy_error = ?2,
                copy_attempts = copy_attempts + 1,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(&id_str)
        .bind(error)
        .execute(&self.pool)
        .await?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve pending file match after update"))
    }

    /// Update the target of a match (for manual matching)

    #[cfg(feature = "sqlite")]
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

        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            UPDATE pending_file_matches
            SET episode_id = ?2, movie_id = ?3, track_id = ?4, chapter_id = ?5,
                unmatched_reason = NULL,
                match_type = 'manual',
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(&id_str)
        .bind(episode_id.map(uuid_to_str))
        .bind(movie_id.map(uuid_to_str))
        .bind(track_id.map(uuid_to_str))
        .bind(chapter_id.map(uuid_to_str))
        .execute(&self.pool)
        .await?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve pending file match after update"))
    }

    /// Update verification status for a match
    #[cfg(feature = "sqlite")]
    pub async fn mark_verification_status(
        &self,
        id: Uuid,
        status: &str,
        reason: Option<&str>,
    ) -> Result<PendingFileMatchRecord> {
        sqlx::query(
            r#"
            UPDATE pending_file_matches
            SET verification_status = ?2,
                verification_reason = ?3,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(status)
        .bind(reason)
        .execute(&self.pool)
        .await?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve pending file match after update"))
    }

    /// Delete a pending file match

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM pending_file_matches WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all matches for a source

    #[cfg(feature = "sqlite")]
    pub async fn delete_by_source(&self, source_type: &str, source_id: Uuid) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM pending_file_matches WHERE source_type = ?1 AND source_id = ?2",
        )
        .bind(source_type)
        .bind(uuid_to_str(source_id))
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Count matches for a source

    #[cfg(feature = "sqlite")]
    pub async fn count_by_source(&self, source_type: &str, source_id: Uuid) -> Result<i64> {
        let count: (i32,) = sqlx::query_as(
            "SELECT COUNT(*) FROM pending_file_matches WHERE source_type = ?1 AND source_id = ?2",
        )
        .bind(source_type)
        .bind(uuid_to_str(source_id))
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 as i64)
    }

    /// Count uncopied matches for a source

    #[cfg(feature = "sqlite")]
    pub async fn count_uncopied_by_source(
        &self,
        source_type: &str,
        source_id: Uuid,
    ) -> Result<i64> {
        let count: (i32,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM pending_file_matches 
            WHERE source_type = ?1 AND source_id = ?2 AND copied_at IS NULL
            "#,
        )
        .bind(source_type)
        .bind(uuid_to_str(source_id))
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 as i64)
    }
}
