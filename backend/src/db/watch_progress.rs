//! Watch progress database repository
//!
//! Tracks per-user watch/listen progress for all content types:
//! episodes, movies, tracks, and audiobooks.

use anyhow::Result;
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite")]
use crate::db::sqlite_helpers::{
    bool_to_int, int_to_bool, str_to_uuid, str_to_uuid_opt, uuid_to_str,
};

#[cfg(feature = "postgres")]
type DbPool = PgPool;
#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

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
#[derive(Debug, Clone)]
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

#[cfg(feature = "postgres")]
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for WatchProgressRecord {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            episode_id: row.try_get("episode_id")?,
            movie_id: row.try_get("movie_id")?,
            track_id: row.try_get("track_id")?,
            audiobook_id: row.try_get("audiobook_id")?,
            content_type: row.try_get("content_type")?,
            media_file_id: row.try_get("media_file_id")?,
            current_position: row.try_get("current_position")?,
            duration: row.try_get("duration")?,
            progress_percent: row.try_get("progress_percent")?,
            is_watched: row.try_get("is_watched")?,
            watched_at: row.try_get("watched_at")?,
            last_watched_at: row.try_get("last_watched_at")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for WatchProgressRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use time::format_description::well_known::Rfc3339;

        let id_str: String = row.try_get("id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let episode_id_str: Option<String> = row.try_get("episode_id")?;
        let movie_id_str: Option<String> = row.try_get("movie_id")?;
        let track_id_str: Option<String> = row.try_get("track_id")?;
        let audiobook_id_str: Option<String> = row.try_get("audiobook_id")?;
        let media_file_id_str: Option<String> = row.try_get("media_file_id")?;
        let is_watched_int: i32 = row.try_get("is_watched")?;
        let watched_at_str: Option<String> = row.try_get("watched_at")?;
        let last_watched_at_str: String = row.try_get("last_watched_at")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;

        // Helper to parse datetime strings
        fn parse_datetime(s: &str) -> sqlx::Result<OffsetDateTime> {
            // Try RFC3339 first
            if let Ok(dt) = OffsetDateTime::parse(s, &Rfc3339) {
                return Ok(dt);
            }
            // Try SQLite datetime format: "YYYY-MM-DD HH:MM:SS"
            let format = time::format_description::parse(
                "[year]-[month]-[day] [hour]:[minute]:[second]"
            ).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
            time::PrimitiveDateTime::parse(s, &format)
                .map(|pdt| pdt.assume_utc())
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))
        }

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            episode_id: str_to_uuid_opt(episode_id_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            movie_id: str_to_uuid_opt(movie_id_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            track_id: str_to_uuid_opt(track_id_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            audiobook_id: str_to_uuid_opt(audiobook_id_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            content_type: row.try_get("content_type")?,
            media_file_id: str_to_uuid_opt(media_file_id_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            current_position: row.try_get("current_position")?,
            duration: row.try_get("duration")?,
            progress_percent: row.try_get("progress_percent")?,
            is_watched: int_to_bool(is_watched_int),
            watched_at: watched_at_str.as_ref()
                .map(|s| parse_datetime(s))
                .transpose()?,
            last_watched_at: parse_datetime(&last_watched_at_str)?,
            created_at: parse_datetime(&created_at_str)?,
            updated_at: parse_datetime(&updated_at_str)?,
        })
    }
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
    pool: DbPool,
}

/// Threshold for marking content as "watched" (90%)
const WATCHED_THRESHOLD: f32 = 0.9;

impl WatchProgressRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Upsert watch progress for any content type
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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

        // Check if progress already exists and get ID
        let existing = self.get_progress(input.user_id, input.content_type, input.content_id).await?;

        let id = if let Some(existing_record) = existing {
            // Update existing record
            let id_column = match input.content_type {
                ContentType::Episode => "episode_id",
                ContentType::Movie => "movie_id",
                ContentType::Track => "track_id",
                ContentType::Audiobook => "audiobook_id",
            };

            let query = format!(
                r#"
                UPDATE watch_progress SET
                    media_file_id = COALESCE(?1, media_file_id),
                    current_position = ?2,
                    duration = COALESCE(?3, duration),
                    progress_percent = ?4,
                    is_watched = CASE WHEN ?5 = 1 THEN 1 ELSE is_watched END,
                    watched_at = CASE 
                        WHEN ?5 = 1 AND watched_at IS NULL THEN datetime('now')
                        ELSE watched_at
                    END,
                    last_watched_at = datetime('now'),
                    updated_at = datetime('now')
                WHERE user_id = ?6 AND {} = ?7
                "#,
                id_column
            );

            sqlx::query(&query)
                .bind(input.media_file_id.map(uuid_to_str))
                .bind(input.current_position)
                .bind(input.duration)
                .bind(progress_percent)
                .bind(bool_to_int(is_watched))
                .bind(uuid_to_str(input.user_id))
                .bind(uuid_to_str(input.content_id))
                .execute(&self.pool)
                .await?;

            existing_record.id
        } else {
            // Insert new record
            let id = Uuid::new_v4();
            let watched_at_value = if is_watched { "datetime('now')" } else { "NULL" };

            let query = format!(
                r#"
                INSERT INTO watch_progress (
                    id, user_id, content_type, episode_id, movie_id, track_id, audiobook_id,
                    media_file_id, current_position, duration, progress_percent,
                    is_watched, watched_at, last_watched_at, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, {}, datetime('now'), datetime('now'), datetime('now'))
                "#,
                watched_at_value
            );

            sqlx::query(&query)
                .bind(uuid_to_str(id))
                .bind(uuid_to_str(input.user_id))
                .bind(content_type_str)
                .bind(episode_id.map(uuid_to_str))
                .bind(movie_id.map(uuid_to_str))
                .bind(track_id.map(uuid_to_str))
                .bind(audiobook_id.map(uuid_to_str))
                .bind(input.media_file_id.map(uuid_to_str))
                .bind(input.current_position)
                .bind(input.duration)
                .bind(progress_percent)
                .bind(bool_to_int(is_watched))
                .execute(&self.pool)
                .await?;

            id
        };

        // Fetch and return the updated/inserted record
        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve watch progress after upsert"))
    }

    /// Get watch progress by ID
    #[cfg(feature = "postgres")]
    async fn get_by_id(&self, id: Uuid) -> Result<Option<WatchProgressRecord>> {
        let record = sqlx::query_as::<_, WatchProgressRecord>(
            r#"
            SELECT id, user_id, episode_id, movie_id, track_id, audiobook_id,
                   content_type, media_file_id, current_position, duration, 
                   progress_percent, is_watched, watched_at, last_watched_at,
                   created_at, updated_at
            FROM watch_progress
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    #[cfg(feature = "sqlite")]
    async fn get_by_id(&self, id: Uuid) -> Result<Option<WatchProgressRecord>> {
        let record = sqlx::query_as::<_, WatchProgressRecord>(
            r#"
            SELECT id, user_id, episode_id, movie_id, track_id, audiobook_id,
                   content_type, media_file_id, current_position, duration, 
                   progress_percent, is_watched, watched_at, last_watched_at,
                   created_at, updated_at
            FROM watch_progress
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get watch progress for a single content item
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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
            WHERE user_id = ?1 AND {} = ?2
            "#,
            id_column
        );

        let record = sqlx::query_as::<_, WatchProgressRecord>(&query)
            .bind(uuid_to_str(user_id))
            .bind(uuid_to_str(content_id))
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
        self.get_progress(user_id, ContentType::Episode, episode_id)
            .await
    }

    /// Get watch progress for a movie (convenience method)
    pub async fn get_movie_progress(
        &self,
        user_id: Uuid,
        movie_id: Uuid,
    ) -> Result<Option<WatchProgressRecord>> {
        self.get_progress(user_id, ContentType::Movie, movie_id)
            .await
    }

    /// Get watch progress for multiple episodes (batch fetch)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn get_episode_progress_batch(
        &self,
        user_id: Uuid,
        episode_ids: &[Uuid],
    ) -> Result<Vec<WatchProgressRecord>> {
        if episode_ids.is_empty() {
            return Ok(vec![]);
        }

        // SQLite doesn't support ANY(), so we build an IN clause with placeholders
        let placeholders: Vec<String> = (0..episode_ids.len())
            .map(|i| format!("?{}", i + 2))
            .collect();
        let placeholders_str = placeholders.join(", ");

        let query = format!(
            r#"
            SELECT id, user_id, episode_id, movie_id, track_id, audiobook_id,
                   content_type, media_file_id, current_position, duration, 
                   progress_percent, is_watched, watched_at, last_watched_at,
                   created_at, updated_at
            FROM watch_progress
            WHERE user_id = ?1 AND content_type = 'episode' AND episode_id IN ({})
            "#,
            placeholders_str
        );

        let mut query_builder = sqlx::query_as::<_, WatchProgressRecord>(&query)
            .bind(uuid_to_str(user_id));

        for ep_id in episode_ids {
            query_builder = query_builder.bind(uuid_to_str(*ep_id));
        }

        let records = query_builder.fetch_all(&self.pool).await?;

        Ok(records)
    }

    /// Manually mark content as watched
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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
                is_watched = 1,
                watched_at = COALESCE(watched_at, datetime('now')),
                updated_at = datetime('now')
            WHERE user_id = ?1 AND {} = ?2
            "#,
            id_column
        );

        sqlx::query(&query)
            .bind(uuid_to_str(user_id))
            .bind(uuid_to_str(content_id))
            .execute(&self.pool)
            .await?;

        self.get_progress(user_id, content_type, content_id).await
    }

    /// Manually mark content as unwatched (reset progress)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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
                is_watched = 0,
                watched_at = NULL,
                current_position = 0,
                progress_percent = 0,
                updated_at = datetime('now')
            WHERE user_id = ?1 AND {} = ?2
            "#,
            id_column
        );

        sqlx::query(&query)
            .bind(uuid_to_str(user_id))
            .bind(uuid_to_str(content_id))
            .execute(&self.pool)
            .await?;

        self.get_progress(user_id, content_type, content_id).await
    }

    /// Get recently watched content (continue watching)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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
            WHERE user_id = ?1 
              AND is_watched = 0 
              AND progress_percent > 0.01
              AND progress_percent < 0.95
            ORDER BY last_watched_at DESC
            LIMIT ?2
            "#,
        )
        .bind(uuid_to_str(user_id))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get watched episodes for a TV show
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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
            WHERE wp.user_id = ?1 AND wp.is_watched = 1 AND e.tv_show_id = ?2
            ORDER BY wp.watched_at DESC
            "#,
        )
        .bind(uuid_to_str(user_id))
        .bind(uuid_to_str(tv_show_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Delete watch progress for content
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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
            "DELETE FROM watch_progress WHERE user_id = ?1 AND {} = ?2",
            id_column
        );

        let result = sqlx::query(&query)
            .bind(uuid_to_str(user_id))
            .bind(uuid_to_str(content_id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
