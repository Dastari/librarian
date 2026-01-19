//! Watch progress database repository
//!
//! Tracks per-user, per-episode watch progress for resume functionality
//! and watched episode tracking.

use anyhow::Result;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// Watch progress record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WatchProgressRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub episode_id: Uuid,
    pub media_file_id: Option<Uuid>,
    pub current_position: f64,
    pub duration: Option<f64>,
    pub progress_percent: f32,
    pub is_watched: bool,
    pub watched_at: Option<OffsetDateTime>,
    pub last_watched_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Input for upserting watch progress
#[derive(Debug)]
pub struct UpsertWatchProgress {
    pub user_id: Uuid,
    pub episode_id: Uuid,
    pub media_file_id: Option<Uuid>,
    pub current_position: f64,
    pub duration: Option<f64>,
}

/// Watch progress repository for database operations
pub struct WatchProgressRepository {
    pool: PgPool,
}

/// Threshold for marking an episode as "watched" (90%)
const WATCHED_THRESHOLD: f32 = 0.9;

impl WatchProgressRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert watch progress for an episode
    /// 
    /// Automatically calculates progress percentage and marks as watched if >= 90%
    pub async fn upsert_progress(&self, input: UpsertWatchProgress) -> Result<WatchProgressRecord> {
        let progress_percent = if let Some(duration) = input.duration {
            if duration > 0.0 {
                (input.current_position / duration) as f32
            } else {
                0.0
            }
        } else {
            0.0
        };

        let is_watched = progress_percent >= WATCHED_THRESHOLD;
        
        let record = sqlx::query_as::<_, WatchProgressRecord>(
            r#"
            INSERT INTO watch_progress (
                user_id, episode_id, media_file_id,
                current_position, duration, progress_percent,
                is_watched, watched_at, last_watched_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 
                    CASE WHEN $7 THEN NOW() ELSE NULL END, 
                    NOW())
            ON CONFLICT (user_id, episode_id) DO UPDATE SET
                media_file_id = COALESCE($3, watch_progress.media_file_id),
                current_position = $4,
                duration = COALESCE($5, watch_progress.duration),
                progress_percent = $6,
                is_watched = CASE 
                    WHEN $7 THEN true 
                    ELSE watch_progress.is_watched 
                END,
                watched_at = CASE 
                    WHEN $7 AND watch_progress.watched_at IS NULL THEN NOW()
                    ELSE watch_progress.watched_at
                END,
                last_watched_at = NOW(),
                updated_at = NOW()
            RETURNING id, user_id, episode_id, media_file_id,
                      current_position, duration, progress_percent,
                      is_watched, watched_at, last_watched_at,
                      created_at, updated_at
            "#,
        )
        .bind(input.user_id)
        .bind(input.episode_id)
        .bind(input.media_file_id)
        .bind(input.current_position)
        .bind(input.duration)
        .bind(progress_percent)
        .bind(is_watched)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get watch progress for a single episode
    pub async fn get_progress(
        &self,
        user_id: Uuid,
        episode_id: Uuid,
    ) -> Result<Option<WatchProgressRecord>> {
        let record = sqlx::query_as::<_, WatchProgressRecord>(
            r#"
            SELECT id, user_id, episode_id, media_file_id,
                   current_position, duration, progress_percent,
                   is_watched, watched_at, last_watched_at,
                   created_at, updated_at
            FROM watch_progress
            WHERE user_id = $1 AND episode_id = $2
            "#,
        )
        .bind(user_id)
        .bind(episode_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get watch progress for multiple episodes (batch fetch)
    pub async fn get_progress_batch(
        &self,
        user_id: Uuid,
        episode_ids: &[Uuid],
    ) -> Result<Vec<WatchProgressRecord>> {
        if episode_ids.is_empty() {
            return Ok(vec![]);
        }

        let records = sqlx::query_as::<_, WatchProgressRecord>(
            r#"
            SELECT id, user_id, episode_id, media_file_id,
                   current_position, duration, progress_percent,
                   is_watched, watched_at, last_watched_at,
                   created_at, updated_at
            FROM watch_progress
            WHERE user_id = $1 AND episode_id = ANY($2)
            "#,
        )
        .bind(user_id)
        .bind(episode_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Manually mark an episode as watched
    pub async fn mark_watched(
        &self,
        user_id: Uuid,
        episode_id: Uuid,
    ) -> Result<Option<WatchProgressRecord>> {
        let record = sqlx::query_as::<_, WatchProgressRecord>(
            r#"
            UPDATE watch_progress SET
                is_watched = true,
                watched_at = COALESCE(watched_at, NOW()),
                updated_at = NOW()
            WHERE user_id = $1 AND episode_id = $2
            RETURNING id, user_id, episode_id, media_file_id,
                      current_position, duration, progress_percent,
                      is_watched, watched_at, last_watched_at,
                      created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(episode_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Manually mark an episode as unwatched (reset progress)
    pub async fn mark_unwatched(
        &self,
        user_id: Uuid,
        episode_id: Uuid,
    ) -> Result<Option<WatchProgressRecord>> {
        let record = sqlx::query_as::<_, WatchProgressRecord>(
            r#"
            UPDATE watch_progress SET
                is_watched = false,
                watched_at = NULL,
                current_position = 0,
                progress_percent = 0,
                updated_at = NOW()
            WHERE user_id = $1 AND episode_id = $2
            RETURNING id, user_id, episode_id, media_file_id,
                      current_position, duration, progress_percent,
                      is_watched, watched_at, last_watched_at,
                      created_at, updated_at
            "#,
        )
        .bind(user_id)
        .bind(episode_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get all watched episodes for a user (optionally filtered by show)
    pub async fn get_watched_episodes(
        &self,
        user_id: Uuid,
        tv_show_id: Option<Uuid>,
    ) -> Result<Vec<WatchProgressRecord>> {
        let records = if let Some(show_id) = tv_show_id {
            sqlx::query_as::<_, WatchProgressRecord>(
                r#"
                SELECT wp.id, wp.user_id, wp.episode_id, wp.media_file_id,
                       wp.current_position, wp.duration, wp.progress_percent,
                       wp.is_watched, wp.watched_at, wp.last_watched_at,
                       wp.created_at, wp.updated_at
                FROM watch_progress wp
                JOIN episodes e ON e.id = wp.episode_id
                WHERE wp.user_id = $1 AND wp.is_watched = true AND e.tv_show_id = $2
                ORDER BY wp.watched_at DESC
                "#,
            )
            .bind(user_id)
            .bind(show_id)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, WatchProgressRecord>(
                r#"
                SELECT id, user_id, episode_id, media_file_id,
                       current_position, duration, progress_percent,
                       is_watched, watched_at, last_watched_at,
                       created_at, updated_at
                FROM watch_progress
                WHERE user_id = $1 AND is_watched = true
                ORDER BY watched_at DESC
                "#,
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(records)
    }

    /// Delete watch progress for an episode
    pub async fn delete_progress(&self, user_id: Uuid, episode_id: Uuid) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM watch_progress WHERE user_id = $1 AND episode_id = $2",
        )
        .bind(user_id)
        .bind(episode_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
