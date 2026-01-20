//! Playback sessions database repository
//!
//! Manages user playback state for persistent video/audio player.
//! Supports all content types: episodes, movies, tracks, and audiobooks.

use anyhow::Result;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// Playback session record from database
#[derive(Debug, Clone, sqlx::FromRow)]
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
    pool: PgPool,
}

impl PlaybackRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get the active playback session for a user
    pub async fn get_active_session(&self, user_id: Uuid) -> Result<Option<PlaybackSessionRecord>> {
        let record = sqlx::query_as::<_, PlaybackSessionRecord>(
            r#"
            SELECT id, user_id, episode_id, media_file_id, tv_show_id,
                   movie_id, track_id, audiobook_id, album_id, content_type,
                   current_position, duration, volume, is_muted, is_playing,
                   started_at, last_updated_at, completed_at, created_at, updated_at
            FROM playback_sessions
            WHERE user_id = $1 AND completed_at IS NULL
            ORDER BY last_updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create or update a playback session (upsert by user_id)
    pub async fn upsert_session(&self, input: UpsertPlaybackSession) -> Result<PlaybackSessionRecord> {
        let record = sqlx::query_as::<_, PlaybackSessionRecord>(
            r#"
            INSERT INTO playback_sessions (
                user_id, content_type, media_file_id,
                episode_id, movie_id, track_id, audiobook_id,
                tv_show_id, album_id,
                current_position, duration, volume, is_muted, is_playing,
                started_at, last_updated_at, completed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW(), NOW(), NULL)
            ON CONFLICT (user_id) DO UPDATE SET
                content_type = EXCLUDED.content_type,
                media_file_id = EXCLUDED.media_file_id,
                episode_id = EXCLUDED.episode_id,
                movie_id = EXCLUDED.movie_id,
                track_id = EXCLUDED.track_id,
                audiobook_id = EXCLUDED.audiobook_id,
                tv_show_id = EXCLUDED.tv_show_id,
                album_id = EXCLUDED.album_id,
                current_position = EXCLUDED.current_position,
                duration = EXCLUDED.duration,
                volume = EXCLUDED.volume,
                is_muted = EXCLUDED.is_muted,
                is_playing = EXCLUDED.is_playing,
                started_at = CASE 
                    WHEN playback_sessions.content_type != EXCLUDED.content_type 
                         OR COALESCE(playback_sessions.episode_id, playback_sessions.movie_id, 
                                     playback_sessions.track_id, playback_sessions.audiobook_id) 
                            != COALESCE(EXCLUDED.episode_id, EXCLUDED.movie_id, 
                                        EXCLUDED.track_id, EXCLUDED.audiobook_id)
                    THEN NOW() 
                    ELSE playback_sessions.started_at 
                END,
                last_updated_at = NOW(),
                completed_at = NULL,
                updated_at = NOW()
            RETURNING id, user_id, episode_id, media_file_id, tv_show_id,
                      movie_id, track_id, audiobook_id, album_id, content_type,
                      current_position, duration, volume, is_muted, is_playing,
                      started_at, last_updated_at, completed_at, created_at, updated_at
            "#,
        )
        .bind(input.user_id)
        .bind(&input.content_type)
        .bind(input.media_file_id)
        .bind(input.episode_id)
        .bind(input.movie_id)
        .bind(input.track_id)
        .bind(input.audiobook_id)
        .bind(input.tv_show_id)
        .bind(input.album_id)
        .bind(input.current_position)
        .bind(input.duration)
        .bind(input.volume)
        .bind(input.is_muted)
        .bind(input.is_playing)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update playback position for a session
    pub async fn update_position(
        &self,
        user_id: Uuid,
        input: UpdatePlaybackPosition,
    ) -> Result<Option<PlaybackSessionRecord>> {
        let record = sqlx::query_as::<_, PlaybackSessionRecord>(
            r#"
            UPDATE playback_sessions SET
                current_position = COALESCE($2, current_position),
                duration = COALESCE($3, duration),
                volume = COALESCE($4, volume),
                is_muted = COALESCE($5, is_muted),
                is_playing = COALESCE($6, is_playing),
                last_updated_at = NOW(),
                updated_at = NOW()
            WHERE user_id = $1 AND completed_at IS NULL
            RETURNING id, user_id, episode_id, media_file_id, tv_show_id,
                      movie_id, track_id, audiobook_id, album_id, content_type,
                      current_position, duration, volume, is_muted, is_playing,
                      started_at, last_updated_at, completed_at, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(input.current_position)
        .bind(input.duration)
        .bind(input.volume)
        .bind(input.is_muted)
        .bind(input.is_playing)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Mark a session as completed (stopped watching)
    pub async fn complete_session(&self, user_id: Uuid) -> Result<Option<PlaybackSessionRecord>> {
        let record = sqlx::query_as::<_, PlaybackSessionRecord>(
            r#"
            UPDATE playback_sessions SET
                is_playing = false,
                completed_at = NOW(),
                updated_at = NOW()
            WHERE user_id = $1 AND completed_at IS NULL
            RETURNING id, user_id, episode_id, media_file_id, tv_show_id,
                      movie_id, track_id, audiobook_id, album_id, content_type,
                      current_position, duration, volume, is_muted, is_playing,
                      started_at, last_updated_at, completed_at, created_at, updated_at
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete old completed sessions (cleanup)
    pub async fn cleanup_old_sessions(&self, days_old: i32) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM playback_sessions
            WHERE completed_at IS NOT NULL 
              AND completed_at < NOW() - INTERVAL '1 day' * $1
            "#,
        )
        .bind(days_old)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
