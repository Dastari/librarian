//! Watch progress database repository
//!
//! Tracks per-user watch/listen progress for all content types:
//! episodes, movies, tracks, and audiobooks.

use anyhow::Result;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// Content type for watch progress
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Episode,
    Movie,
    Track,
    Audiobook,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Episode => "episode",
            ContentType::Movie => "movie",
            ContentType::Track => "track",
            ContentType::Audiobook => "audiobook",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "episode" => Some(ContentType::Episode),
            "movie" => Some(ContentType::Movie),
            "track" => Some(ContentType::Track),
            "audiobook" => Some(ContentType::Audiobook),
            _ => None,
        }
    }
}

/// Watch progress record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WatchProgressRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub episode_id: Option<Uuid>,
    pub movie_id: Option<Uuid>,
    pub track_id: Option<Uuid>,
    pub audiobook_id: Option<Uuid>,
    pub content_type: String,
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

impl WatchProgressRecord {
    /// Get the content ID based on content type
    pub fn content_id(&self) -> Option<Uuid> {
        match self.content_type.as_str() {
            "episode" => self.episode_id,
            "movie" => self.movie_id,
            "track" => self.track_id,
            "audiobook" => self.audiobook_id,
            _ => None,
        }
    }
}

/// Input for upserting watch progress
#[derive(Debug)]
pub struct UpsertWatchProgress {
    pub user_id: Uuid,
    pub content_type: ContentType,
    pub content_id: Uuid,
    pub media_file_id: Option<Uuid>,
    pub current_position: f64,
    pub duration: Option<f64>,
}

/// Watch progress repository for database operations
pub struct WatchProgressRepository {
    pool: PgPool,
}

/// Threshold for marking content as "watched" (90%)
const WATCHED_THRESHOLD: f32 = 0.9;

impl WatchProgressRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Upsert watch progress for any content type
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
        let content_type_str = input.content_type.as_str();

        let (episode_id, movie_id, track_id, audiobook_id) = match input.content_type {
            ContentType::Episode => (Some(input.content_id), None, None, None),
            ContentType::Movie => (None, Some(input.content_id), None, None),
            ContentType::Track => (None, None, Some(input.content_id), None),
            ContentType::Audiobook => (None, None, None, Some(input.content_id)),
        };

        // Use type-specific upsert based on content type
        let record = match input.content_type {
            ContentType::Episode => {
                sqlx::query_as::<_, WatchProgressRecord>(
                    r#"
                    INSERT INTO watch_progress (
                        user_id, content_type, episode_id, media_file_id,
                        current_position, duration, progress_percent,
                        is_watched, watched_at, last_watched_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 
                            CASE WHEN $8 THEN NOW() ELSE NULL END, NOW())
                    ON CONFLICT (user_id, episode_id) WHERE content_type = 'episode' DO UPDATE SET
                        media_file_id = COALESCE($4, watch_progress.media_file_id),
                        current_position = $5,
                        duration = COALESCE($6, watch_progress.duration),
                        progress_percent = $7,
                        is_watched = CASE WHEN $8 THEN true ELSE watch_progress.is_watched END,
                        watched_at = CASE 
                            WHEN $8 AND watch_progress.watched_at IS NULL THEN NOW()
                            ELSE watch_progress.watched_at
                        END,
                        last_watched_at = NOW(),
                        updated_at = NOW()
                    RETURNING id, user_id, episode_id, movie_id, track_id, audiobook_id,
                              content_type, media_file_id, current_position, duration, 
                              progress_percent, is_watched, watched_at, last_watched_at,
                              created_at, updated_at
                    "#,
                )
                .bind(input.user_id)
                .bind(content_type_str)
                .bind(episode_id)
                .bind(input.media_file_id)
                .bind(input.current_position)
                .bind(input.duration)
                .bind(progress_percent)
                .bind(is_watched)
                .fetch_one(&self.pool)
                .await?
            }
            ContentType::Movie => {
                sqlx::query_as::<_, WatchProgressRecord>(
                    r#"
                    INSERT INTO watch_progress (
                        user_id, content_type, movie_id, media_file_id,
                        current_position, duration, progress_percent,
                        is_watched, watched_at, last_watched_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 
                            CASE WHEN $8 THEN NOW() ELSE NULL END, NOW())
                    ON CONFLICT (user_id, movie_id) WHERE content_type = 'movie' DO UPDATE SET
                        media_file_id = COALESCE($4, watch_progress.media_file_id),
                        current_position = $5,
                        duration = COALESCE($6, watch_progress.duration),
                        progress_percent = $7,
                        is_watched = CASE WHEN $8 THEN true ELSE watch_progress.is_watched END,
                        watched_at = CASE 
                            WHEN $8 AND watch_progress.watched_at IS NULL THEN NOW()
                            ELSE watch_progress.watched_at
                        END,
                        last_watched_at = NOW(),
                        updated_at = NOW()
                    RETURNING id, user_id, episode_id, movie_id, track_id, audiobook_id,
                              content_type, media_file_id, current_position, duration, 
                              progress_percent, is_watched, watched_at, last_watched_at,
                              created_at, updated_at
                    "#,
                )
                .bind(input.user_id)
                .bind(content_type_str)
                .bind(movie_id)
                .bind(input.media_file_id)
                .bind(input.current_position)
                .bind(input.duration)
                .bind(progress_percent)
                .bind(is_watched)
                .fetch_one(&self.pool)
                .await?
            }
            ContentType::Track => {
                sqlx::query_as::<_, WatchProgressRecord>(
                    r#"
                    INSERT INTO watch_progress (
                        user_id, content_type, track_id, media_file_id,
                        current_position, duration, progress_percent,
                        is_watched, watched_at, last_watched_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 
                            CASE WHEN $8 THEN NOW() ELSE NULL END, NOW())
                    ON CONFLICT (user_id, track_id) WHERE content_type = 'track' DO UPDATE SET
                        media_file_id = COALESCE($4, watch_progress.media_file_id),
                        current_position = $5,
                        duration = COALESCE($6, watch_progress.duration),
                        progress_percent = $7,
                        is_watched = CASE WHEN $8 THEN true ELSE watch_progress.is_watched END,
                        watched_at = CASE 
                            WHEN $8 AND watch_progress.watched_at IS NULL THEN NOW()
                            ELSE watch_progress.watched_at
                        END,
                        last_watched_at = NOW(),
                        updated_at = NOW()
                    RETURNING id, user_id, episode_id, movie_id, track_id, audiobook_id,
                              content_type, media_file_id, current_position, duration, 
                              progress_percent, is_watched, watched_at, last_watched_at,
                              created_at, updated_at
                    "#,
                )
                .bind(input.user_id)
                .bind(content_type_str)
                .bind(track_id)
                .bind(input.media_file_id)
                .bind(input.current_position)
                .bind(input.duration)
                .bind(progress_percent)
                .bind(is_watched)
                .fetch_one(&self.pool)
                .await?
            }
            ContentType::Audiobook => {
                sqlx::query_as::<_, WatchProgressRecord>(
                    r#"
                    INSERT INTO watch_progress (
                        user_id, content_type, audiobook_id, media_file_id,
                        current_position, duration, progress_percent,
                        is_watched, watched_at, last_watched_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 
                            CASE WHEN $8 THEN NOW() ELSE NULL END, NOW())
                    ON CONFLICT (user_id, audiobook_id) WHERE content_type = 'audiobook' DO UPDATE SET
                        media_file_id = COALESCE($4, watch_progress.media_file_id),
                        current_position = $5,
                        duration = COALESCE($6, watch_progress.duration),
                        progress_percent = $7,
                        is_watched = CASE WHEN $8 THEN true ELSE watch_progress.is_watched END,
                        watched_at = CASE 
                            WHEN $8 AND watch_progress.watched_at IS NULL THEN NOW()
                            ELSE watch_progress.watched_at
                        END,
                        last_watched_at = NOW(),
                        updated_at = NOW()
                    RETURNING id, user_id, episode_id, movie_id, track_id, audiobook_id,
                              content_type, media_file_id, current_position, duration, 
                              progress_percent, is_watched, watched_at, last_watched_at,
                              created_at, updated_at
                    "#,
                )
                .bind(input.user_id)
                .bind(content_type_str)
                .bind(audiobook_id)
                .bind(input.media_file_id)
                .bind(input.current_position)
                .bind(input.duration)
                .bind(progress_percent)
                .bind(is_watched)
                .fetch_one(&self.pool)
                .await?
            }
        };

        Ok(record)
    }

    /// Get watch progress for a single content item
    pub async fn get_progress(
        &self,
        user_id: Uuid,
        content_type: ContentType,
        content_id: Uuid,
    ) -> Result<Option<WatchProgressRecord>> {
        let id_column = match content_type {
            ContentType::Episode => "episode_id",
            ContentType::Movie => "movie_id",
            ContentType::Track => "track_id",
            ContentType::Audiobook => "audiobook_id",
        };

        let query = format!(
            r#"
            SELECT id, user_id, episode_id, movie_id, track_id, audiobook_id,
                   content_type, media_file_id, current_position, duration, 
                   progress_percent, is_watched, watched_at, last_watched_at,
                   created_at, updated_at
            FROM watch_progress
            WHERE user_id = $1 AND {} = $2
            "#,
            id_column
        );

        let record = sqlx::query_as::<_, WatchProgressRecord>(&query)
            .bind(user_id)
            .bind(content_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(record)
    }

    /// Get watch progress for an episode (convenience method)
    pub async fn get_episode_progress(
        &self,
        user_id: Uuid,
        episode_id: Uuid,
    ) -> Result<Option<WatchProgressRecord>> {
        self.get_progress(user_id, ContentType::Episode, episode_id).await
    }

    /// Get watch progress for a movie (convenience method)
    pub async fn get_movie_progress(
        &self,
        user_id: Uuid,
        movie_id: Uuid,
    ) -> Result<Option<WatchProgressRecord>> {
        self.get_progress(user_id, ContentType::Movie, movie_id).await
    }

    /// Get watch progress for multiple episodes (batch fetch)
    pub async fn get_episode_progress_batch(
        &self,
        user_id: Uuid,
        episode_ids: &[Uuid],
    ) -> Result<Vec<WatchProgressRecord>> {
        if episode_ids.is_empty() {
            return Ok(vec![]);
        }

        let records = sqlx::query_as::<_, WatchProgressRecord>(
            r#"
            SELECT id, user_id, episode_id, movie_id, track_id, audiobook_id,
                   content_type, media_file_id, current_position, duration, 
                   progress_percent, is_watched, watched_at, last_watched_at,
                   created_at, updated_at
            FROM watch_progress
            WHERE user_id = $1 AND content_type = 'episode' AND episode_id = ANY($2)
            "#,
        )
        .bind(user_id)
        .bind(episode_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Manually mark content as watched
    pub async fn mark_watched(
        &self,
        user_id: Uuid,
        content_type: ContentType,
        content_id: Uuid,
    ) -> Result<Option<WatchProgressRecord>> {
        let id_column = match content_type {
            ContentType::Episode => "episode_id",
            ContentType::Movie => "movie_id",
            ContentType::Track => "track_id",
            ContentType::Audiobook => "audiobook_id",
        };

        let query = format!(
            r#"
            UPDATE watch_progress SET
                is_watched = true,
                watched_at = COALESCE(watched_at, NOW()),
                updated_at = NOW()
            WHERE user_id = $1 AND {} = $2
            RETURNING id, user_id, episode_id, movie_id, track_id, audiobook_id,
                      content_type, media_file_id, current_position, duration, 
                      progress_percent, is_watched, watched_at, last_watched_at,
                      created_at, updated_at
            "#,
            id_column
        );

        let record = sqlx::query_as::<_, WatchProgressRecord>(&query)
            .bind(user_id)
            .bind(content_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(record)
    }

    /// Manually mark content as unwatched (reset progress)
    pub async fn mark_unwatched(
        &self,
        user_id: Uuid,
        content_type: ContentType,
        content_id: Uuid,
    ) -> Result<Option<WatchProgressRecord>> {
        let id_column = match content_type {
            ContentType::Episode => "episode_id",
            ContentType::Movie => "movie_id",
            ContentType::Track => "track_id",
            ContentType::Audiobook => "audiobook_id",
        };

        let query = format!(
            r#"
            UPDATE watch_progress SET
                is_watched = false,
                watched_at = NULL,
                current_position = 0,
                progress_percent = 0,
                updated_at = NOW()
            WHERE user_id = $1 AND {} = $2
            RETURNING id, user_id, episode_id, movie_id, track_id, audiobook_id,
                      content_type, media_file_id, current_position, duration, 
                      progress_percent, is_watched, watched_at, last_watched_at,
                      created_at, updated_at
            "#,
            id_column
        );

        let record = sqlx::query_as::<_, WatchProgressRecord>(&query)
            .bind(user_id)
            .bind(content_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(record)
    }

    /// Get recently watched content (continue watching)
    pub async fn get_continue_watching(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<WatchProgressRecord>> {
        let records = sqlx::query_as::<_, WatchProgressRecord>(
            r#"
            SELECT id, user_id, episode_id, movie_id, track_id, audiobook_id,
                   content_type, media_file_id, current_position, duration, 
                   progress_percent, is_watched, watched_at, last_watched_at,
                   created_at, updated_at
            FROM watch_progress
            WHERE user_id = $1 
              AND is_watched = false 
              AND progress_percent > 0.01
              AND progress_percent < 0.95
            ORDER BY last_watched_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get watched episodes for a TV show
    pub async fn get_watched_episodes_for_show(
        &self,
        user_id: Uuid,
        tv_show_id: Uuid,
    ) -> Result<Vec<WatchProgressRecord>> {
        let records = sqlx::query_as::<_, WatchProgressRecord>(
            r#"
            SELECT wp.id, wp.user_id, wp.episode_id, wp.movie_id, wp.track_id, wp.audiobook_id,
                   wp.content_type, wp.media_file_id, wp.current_position, wp.duration, 
                   wp.progress_percent, wp.is_watched, wp.watched_at, wp.last_watched_at,
                   wp.created_at, wp.updated_at
            FROM watch_progress wp
            JOIN episodes e ON e.id = wp.episode_id
            WHERE wp.user_id = $1 AND wp.is_watched = true AND e.tv_show_id = $2
            ORDER BY wp.watched_at DESC
            "#,
        )
        .bind(user_id)
        .bind(tv_show_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Delete watch progress for content
    pub async fn delete_progress(
        &self,
        user_id: Uuid,
        content_type: ContentType,
        content_id: Uuid,
    ) -> Result<bool> {
        let id_column = match content_type {
            ContentType::Episode => "episode_id",
            ContentType::Movie => "movie_id",
            ContentType::Track => "track_id",
            ContentType::Audiobook => "audiobook_id",
        };

        let query = format!(
            "DELETE FROM watch_progress WHERE user_id = $1 AND {} = $2",
            id_column
        );

        let result = sqlx::query(&query)
            .bind(user_id)
            .bind(content_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
