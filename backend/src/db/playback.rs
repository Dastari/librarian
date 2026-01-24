//! Playback sessions database repository
//!
//! Manages user playback state for persistent video/audio player.
//! Supports all content types: episodes, movies, tracks, and audiobooks.

use anyhow::Result;
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

#[cfg(feature = "sqlite")]
use crate::db::sqlite_helpers::{bool_to_int, int_to_bool, str_to_uuid, uuid_to_str};

/// Playback session record from database
#[derive(Debug, Clone)]
pub struct PlaybackSessionRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub episode_id: Option<Uuid>,
    pub media_file_id: Option<Uuid>,
    pub tv_show_id: Option<Uuid>,
    pub movie_id: Option<Uuid>,
    pub track_id: Option<Uuid>,
    pub audiobook_id: Option<Uuid>,
    pub album_id: Option<Uuid>,
    pub content_type: Option<String>,
    pub current_position: f64,
    pub duration: Option<f64>,
    pub volume: f32,
    pub is_muted: bool,
    pub is_playing: bool,
    pub started_at: OffsetDateTime,
    pub last_updated_at: OffsetDateTime,
    pub completed_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for PlaybackSessionRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use time::format_description::well_known::Rfc3339;

        fn parse_timestamp(s: &str) -> sqlx::Result<OffsetDateTime> {
            OffsetDateTime::parse(s, &Rfc3339)
                .or_else(|_| {
                    // Try SQLite datetime format: "YYYY-MM-DD HH:MM:SS"
                    let format = time::format_description::parse(
                        "[year]-[month]-[day] [hour]:[minute]:[second]",
                    )
                    .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
                    time::PrimitiveDateTime::parse(s, &format)
                        .map(|pdt| pdt.assume_utc())
                        .map_err(|e| sqlx::Error::Decode(Box::new(e)))
                })
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))
        }

        fn parse_optional_timestamp(s: Option<String>) -> sqlx::Result<Option<OffsetDateTime>> {
            match s {
                Some(s) if !s.is_empty() => Ok(Some(parse_timestamp(&s)?)),
                _ => Ok(None),
            }
        }

        let id_str: String = row.try_get("id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let episode_id_str: Option<String> = row.try_get("episode_id")?;
        let media_file_id_str: Option<String> = row.try_get("media_file_id")?;
        let tv_show_id_str: Option<String> = row.try_get("tv_show_id")?;
        let movie_id_str: Option<String> = row.try_get("movie_id")?;
        let track_id_str: Option<String> = row.try_get("track_id")?;
        let audiobook_id_str: Option<String> = row.try_get("audiobook_id")?;
        let album_id_str: Option<String> = row.try_get("album_id")?;

        // Booleans stored as INTEGER
        let is_muted: i32 = row.try_get("is_muted")?;
        let is_playing: i32 = row.try_get("is_playing")?;

        // Timestamps stored as TEXT
        let started_at_str: String = row.try_get("started_at")?;
        let last_updated_at_str: String = row.try_get("last_updated_at")?;
        let completed_at_str: Option<String> = row.try_get("completed_at")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            episode_id: episode_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            media_file_id: media_file_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            tv_show_id: tv_show_id_str
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
            audiobook_id: audiobook_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            album_id: album_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            content_type: row.try_get("content_type")?,
            current_position: row.try_get("current_position")?,
            duration: row.try_get("duration")?,
            volume: row.try_get("volume")?,
            is_muted: int_to_bool(is_muted),
            is_playing: int_to_bool(is_playing),
            started_at: parse_timestamp(&started_at_str)?,
            last_updated_at: parse_timestamp(&last_updated_at_str)?,
            completed_at: parse_optional_timestamp(completed_at_str)?,
            created_at: parse_timestamp(&created_at_str)?,
            updated_at: parse_timestamp(&updated_at_str)?,
        })
    }
}

impl PlaybackSessionRecord {
    /// Get the content ID based on content type
    pub fn content_id(&self) -> Option<Uuid> {
        match self.content_type.as_deref() {
            Some("episode") => self.episode_id,
            Some("movie") => self.movie_id,
            Some("track") => self.track_id,
            Some("audiobook") => self.audiobook_id,
            _ => None,
        }
    }

    /// Get the parent ID (show for episodes, album for tracks, etc.)
    pub fn parent_id(&self) -> Option<Uuid> {
        match self.content_type.as_deref() {
            Some("episode") => self.tv_show_id,
            Some("track") => self.album_id,
            _ => None,
        }
    }
}

/// Input for creating/updating a playback session
#[derive(Debug)]
pub struct UpsertPlaybackSession {
    pub user_id: Uuid,
    pub content_type: String,
    pub media_file_id: Option<Uuid>,
    // Content IDs (only one should be set based on content_type)
    pub episode_id: Option<Uuid>,
    pub movie_id: Option<Uuid>,
    pub track_id: Option<Uuid>,
    pub audiobook_id: Option<Uuid>,
    // Parent IDs for context
    pub tv_show_id: Option<Uuid>,
    pub album_id: Option<Uuid>,
    // Playback state
    pub current_position: f64,
    pub duration: Option<f64>,
    pub volume: f32,
    pub is_muted: bool,
    pub is_playing: bool,
}

/// Input for updating playback position
#[derive(Debug, Default)]
pub struct UpdatePlaybackPosition {
    pub current_position: Option<f64>,
    pub duration: Option<f64>,
    pub volume: Option<f32>,
    pub is_muted: Option<bool>,
    pub is_playing: Option<bool>,
}

pub struct PlaybackRepository {
    pool: DbPool,
}

impl PlaybackRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get the active playback session for a user

    #[cfg(feature = "sqlite")]
    pub async fn get_active_session(&self, user_id: Uuid) -> Result<Option<PlaybackSessionRecord>> {
        let record = sqlx::query_as::<_, PlaybackSessionRecord>(
            r#"
            SELECT id, user_id, episode_id, media_file_id, tv_show_id,
                   movie_id, track_id, audiobook_id, album_id, content_type,
                   current_position, duration, volume, is_muted, is_playing,
                   started_at, last_updated_at, completed_at, created_at, updated_at
            FROM playback_sessions
            WHERE user_id = ?1 AND completed_at IS NULL
            ORDER BY last_updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create or update a playback session (upsert by user_id)

    #[cfg(feature = "sqlite")]
    pub async fn upsert_session(
        &self,
        input: UpsertPlaybackSession,
    ) -> Result<PlaybackSessionRecord> {
        let user_id_str = uuid_to_str(input.user_id);

        // Check if session exists for this user
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM playback_sessions WHERE user_id = ?1",
        )
        .bind(&user_id_str)
        .fetch_optional(&self.pool)
        .await?;

        if let Some((existing_id,)) = existing {
            // Update existing session
            sqlx::query(
                r#"
                UPDATE playback_sessions SET
                    content_type = ?2,
                    media_file_id = ?3,
                    episode_id = ?4,
                    movie_id = ?5,
                    track_id = ?6,
                    audiobook_id = ?7,
                    tv_show_id = ?8,
                    album_id = ?9,
                    current_position = ?10,
                    duration = ?11,
                    volume = ?12,
                    is_muted = ?13,
                    is_playing = ?14,
                    started_at = CASE 
                        WHEN content_type != ?2 
                             OR COALESCE(episode_id, movie_id, track_id, audiobook_id) 
                                != COALESCE(?4, ?5, ?6, ?7)
                        THEN datetime('now') 
                        ELSE started_at 
                    END,
                    last_updated_at = datetime('now'),
                    completed_at = NULL,
                    updated_at = datetime('now')
                WHERE user_id = ?1
                "#,
            )
            .bind(&user_id_str)
            .bind(&input.content_type)
            .bind(input.media_file_id.map(uuid_to_str))
            .bind(input.episode_id.map(uuid_to_str))
            .bind(input.movie_id.map(uuid_to_str))
            .bind(input.track_id.map(uuid_to_str))
            .bind(input.audiobook_id.map(uuid_to_str))
            .bind(input.tv_show_id.map(uuid_to_str))
            .bind(input.album_id.map(uuid_to_str))
            .bind(input.current_position)
            .bind(input.duration)
            .bind(input.volume)
            .bind(bool_to_int(input.is_muted))
            .bind(bool_to_int(input.is_playing))
            .execute(&self.pool)
            .await?;

            // Fetch and return the updated record
            let id = str_to_uuid(&existing_id)?;
            self.get_by_id(id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Failed to retrieve session after update"))
        } else {
            // Insert new session
            let id = Uuid::new_v4();
            let id_str = uuid_to_str(id);

            sqlx::query(
                r#"
                INSERT INTO playback_sessions (
                    id, user_id, content_type, media_file_id,
                    episode_id, movie_id, track_id, audiobook_id,
                    tv_show_id, album_id,
                    current_position, duration, volume, is_muted, is_playing,
                    started_at, last_updated_at, completed_at,
                    created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                        datetime('now'), datetime('now'), NULL,
                        datetime('now'), datetime('now'))
                "#,
            )
            .bind(&id_str)
            .bind(&user_id_str)
            .bind(&input.content_type)
            .bind(input.media_file_id.map(uuid_to_str))
            .bind(input.episode_id.map(uuid_to_str))
            .bind(input.movie_id.map(uuid_to_str))
            .bind(input.track_id.map(uuid_to_str))
            .bind(input.audiobook_id.map(uuid_to_str))
            .bind(input.tv_show_id.map(uuid_to_str))
            .bind(input.album_id.map(uuid_to_str))
            .bind(input.current_position)
            .bind(input.duration)
            .bind(input.volume)
            .bind(bool_to_int(input.is_muted))
            .bind(bool_to_int(input.is_playing))
            .execute(&self.pool)
            .await?;

            self.get_by_id(id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Failed to retrieve session after insert"))
        }
    }

    /// Get a session by ID (helper for SQLite upsert)
    #[cfg(feature = "sqlite")]
    async fn get_by_id(&self, id: Uuid) -> Result<Option<PlaybackSessionRecord>> {
        let record = sqlx::query_as::<_, PlaybackSessionRecord>(
            r#"
            SELECT id, user_id, episode_id, media_file_id, tv_show_id,
                   movie_id, track_id, audiobook_id, album_id, content_type,
                   current_position, duration, volume, is_muted, is_playing,
                   started_at, last_updated_at, completed_at, created_at, updated_at
            FROM playback_sessions
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update playback position for a session

    #[cfg(feature = "sqlite")]
    pub async fn update_position(
        &self,
        user_id: Uuid,
        input: UpdatePlaybackPosition,
    ) -> Result<Option<PlaybackSessionRecord>> {
        let user_id_str = uuid_to_str(user_id);

        // Get existing session to apply COALESCE logic
        let existing = self.get_active_session(user_id).await?;
        let Some(current) = existing else {
            return Ok(None);
        };

        sqlx::query(
            r#"
            UPDATE playback_sessions SET
                current_position = ?2,
                duration = ?3,
                volume = ?4,
                is_muted = ?5,
                is_playing = ?6,
                last_updated_at = datetime('now'),
                updated_at = datetime('now')
            WHERE user_id = ?1 AND completed_at IS NULL
            "#,
        )
        .bind(&user_id_str)
        .bind(input.current_position.unwrap_or(current.current_position))
        .bind(input.duration.or(current.duration))
        .bind(input.volume.unwrap_or(current.volume))
        .bind(bool_to_int(input.is_muted.unwrap_or(current.is_muted)))
        .bind(bool_to_int(input.is_playing.unwrap_or(current.is_playing)))
        .execute(&self.pool)
        .await?;

        self.get_active_session(user_id).await
    }

    /// Mark a session as completed (stopped watching)

    #[cfg(feature = "sqlite")]
    pub async fn complete_session(&self, user_id: Uuid) -> Result<Option<PlaybackSessionRecord>> {
        let user_id_str = uuid_to_str(user_id);

        // Get the session ID before updating
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM playback_sessions WHERE user_id = ?1 AND completed_at IS NULL",
        )
        .bind(&user_id_str)
        .fetch_optional(&self.pool)
        .await?;

        let Some((session_id_str,)) = existing else {
            return Ok(None);
        };

        sqlx::query(
            r#"
            UPDATE playback_sessions SET
                is_playing = 0,
                completed_at = datetime('now'),
                updated_at = datetime('now')
            WHERE user_id = ?1 AND completed_at IS NULL
            "#,
        )
        .bind(&user_id_str)
        .execute(&self.pool)
        .await?;

        let session_id = str_to_uuid(&session_id_str)?;
        self.get_by_id(session_id).await
    }

    /// Delete old completed sessions (cleanup)

    #[cfg(feature = "sqlite")]
    pub async fn cleanup_old_sessions(&self, days_old: i32) -> Result<u64> {
        // SQLite uses datetime modifiers for interval arithmetic
        let result = sqlx::query(
            r#"
            DELETE FROM playback_sessions
            WHERE completed_at IS NOT NULL 
              AND completed_at < datetime('now', '-' || ?1 || ' days')
            "#,
        )
        .bind(days_old)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
