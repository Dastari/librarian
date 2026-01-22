//! Episode database repository

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Episode record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EpisodeRecord {
    pub id: Uuid,
    pub tv_show_id: Uuid,
    pub season: i32,
    pub episode: i32,
    pub absolute_number: Option<i32>,
    pub title: Option<String>,
    pub overview: Option<String>,
    pub air_date: Option<chrono::NaiveDate>,
    pub runtime: Option<i32>,
    pub tvmaze_id: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub tvdb_id: Option<i32>,
    pub media_file_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Input for creating an episode
#[derive(Debug)]
pub struct CreateEpisode {
    pub tv_show_id: Uuid,
    pub season: i32,
    pub episode: i32,
    pub absolute_number: Option<i32>,
    pub title: Option<String>,
    pub overview: Option<String>,
    pub air_date: Option<chrono::NaiveDate>,
    pub runtime: Option<i32>,
    pub tvmaze_id: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub tvdb_id: Option<i32>,
}

/// Input for batch creating episodes
#[derive(Debug)]
pub struct CreateEpisodeBatch {
    pub tv_show_id: Uuid,
    pub episodes: Vec<CreateEpisodeItem>,
}

#[derive(Debug)]
pub struct CreateEpisodeItem {
    pub season: i32,
    pub episode: i32,
    pub absolute_number: Option<i32>,
    pub title: Option<String>,
    pub overview: Option<String>,
    pub air_date: Option<chrono::NaiveDate>,
    pub runtime: Option<i32>,
    pub tvmaze_id: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub tvdb_id: Option<i32>,
}

pub struct EpisodeRepository {
    pool: PgPool,
}

impl EpisodeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all episodes for a TV show
    pub async fn list_by_show(&self, tv_show_id: Uuid) -> Result<Vec<EpisodeRecord>> {
        let records = sqlx::query_as::<_, EpisodeRecord>(
            r#"
            SELECT id, tv_show_id, season, episode, absolute_number, title,
                   overview, air_date, runtime, tvmaze_id, tmdb_id, tvdb_id,
                   media_file_id, created_at, updated_at
            FROM episodes
            WHERE tv_show_id = $1
            ORDER BY season, episode
            "#,
        )
        .bind(tv_show_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get episodes for a specific season
    pub async fn list_by_season(
        &self,
        tv_show_id: Uuid,
        season: i32,
    ) -> Result<Vec<EpisodeRecord>> {
        let records = sqlx::query_as::<_, EpisodeRecord>(
            r#"
            SELECT id, tv_show_id, season, episode, absolute_number, title,
                   overview, air_date, runtime, tvmaze_id, tmdb_id, tvdb_id,
                   media_file_id, created_at, updated_at
            FROM episodes
            WHERE tv_show_id = $1 AND season = $2
            ORDER BY episode
            "#,
        )
        .bind(tv_show_id)
        .bind(season)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get wanted episodes (no media file linked) for a library
    pub async fn list_wanted_by_library(&self, library_id: Uuid) -> Result<Vec<EpisodeRecord>> {
        let records = sqlx::query_as::<_, EpisodeRecord>(
            r#"
            SELECT e.id, e.tv_show_id, e.season, e.episode, e.absolute_number, e.title,
                   e.overview, e.air_date, e.runtime, e.tvmaze_id, e.tmdb_id, e.tvdb_id,
                   e.media_file_id, e.created_at, e.updated_at
            FROM episodes e
            JOIN tv_shows ts ON ts.id = e.tv_show_id
            WHERE ts.library_id = $1 
              AND ts.monitored = true
              AND e.media_file_id IS NULL
              AND (e.air_date IS NULL OR e.air_date <= CURRENT_DATE)
            ORDER BY e.air_date DESC NULLS LAST, ts.name, e.season, e.episode
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get wanted episodes for a user across all libraries
    pub async fn list_wanted_by_user(&self, user_id: Uuid) -> Result<Vec<EpisodeRecord>> {
        let records = sqlx::query_as::<_, EpisodeRecord>(
            r#"
            SELECT e.id, e.tv_show_id, e.season, e.episode, e.absolute_number, e.title,
                   e.overview, e.air_date, e.runtime, e.tvmaze_id, e.tmdb_id, e.tvdb_id,
                   e.media_file_id, e.created_at, e.updated_at
            FROM episodes e
            JOIN tv_shows ts ON ts.id = e.tv_show_id
            WHERE ts.user_id = $1 
              AND ts.monitored = true
              AND e.media_file_id IS NULL
              AND (e.air_date IS NULL OR e.air_date <= CURRENT_DATE)
            ORDER BY e.air_date DESC NULLS LAST, ts.name, e.season, e.episode
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get an episode by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<EpisodeRecord>> {
        let record = sqlx::query_as::<_, EpisodeRecord>(
            r#"
            SELECT id, tv_show_id, season, episode, absolute_number, title,
                   overview, air_date, runtime, tvmaze_id, tmdb_id, tvdb_id,
                   media_file_id, created_at, updated_at
            FROM episodes
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get an episode by show, season, and episode number
    pub async fn get_by_show_season_episode(
        &self,
        tv_show_id: Uuid,
        season: i32,
        episode: i32,
    ) -> Result<Option<EpisodeRecord>> {
        let record = sqlx::query_as::<_, EpisodeRecord>(
            r#"
            SELECT id, tv_show_id, season, episode, absolute_number, title,
                   overview, air_date, runtime, tvmaze_id, tmdb_id, tvdb_id,
                   media_file_id, created_at, updated_at
            FROM episodes
            WHERE tv_show_id = $1 AND season = $2 AND episode = $3
            "#,
        )
        .bind(tv_show_id)
        .bind(season)
        .bind(episode)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create a new episode
    pub async fn create(&self, input: CreateEpisode) -> Result<EpisodeRecord> {
        let record = sqlx::query_as::<_, EpisodeRecord>(
            r#"
            INSERT INTO episodes (
                tv_show_id, season, episode, absolute_number, title,
                overview, air_date, runtime, tvmaze_id, tmdb_id, tvdb_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (tv_show_id, season, episode) DO UPDATE SET
                title = COALESCE(EXCLUDED.title, episodes.title),
                overview = COALESCE(EXCLUDED.overview, episodes.overview),
                air_date = COALESCE(EXCLUDED.air_date, episodes.air_date),
                runtime = COALESCE(EXCLUDED.runtime, episodes.runtime),
                tvmaze_id = COALESCE(EXCLUDED.tvmaze_id, episodes.tvmaze_id),
                tmdb_id = COALESCE(EXCLUDED.tmdb_id, episodes.tmdb_id),
                tvdb_id = COALESCE(EXCLUDED.tvdb_id, episodes.tvdb_id),
                updated_at = NOW()
            RETURNING id, tv_show_id, season, episode, absolute_number, title,
                      overview, air_date, runtime, tvmaze_id, tmdb_id, tvdb_id,
                      media_file_id, created_at, updated_at
            "#,
        )
        .bind(input.tv_show_id)
        .bind(input.season)
        .bind(input.episode)
        .bind(input.absolute_number)
        .bind(&input.title)
        .bind(&input.overview)
        .bind(input.air_date)
        .bind(input.runtime)
        .bind(input.tvmaze_id)
        .bind(input.tmdb_id)
        .bind(input.tvdb_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Batch create/update episodes
    pub async fn create_batch(&self, batch: CreateEpisodeBatch) -> Result<usize> {
        let mut count = 0;
        for ep in batch.episodes {
            self.create(CreateEpisode {
                tv_show_id: batch.tv_show_id,
                season: ep.season,
                episode: ep.episode,
                absolute_number: ep.absolute_number,
                title: ep.title,
                overview: ep.overview,
                air_date: ep.air_date,
                runtime: ep.runtime,
                tvmaze_id: ep.tvmaze_id,
                tmdb_id: ep.tmdb_id,
                tvdb_id: ep.tvdb_id,
            })
            .await?;
            count += 1;
        }
        Ok(count)
    }

    /// Link an episode to a media file
    pub async fn set_media_file(&self, episode_id: Uuid, media_file_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE episodes SET media_file_id = $2, updated_at = NOW() WHERE id = $1")
            .bind(episode_id)
            .bind(media_file_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Clear the media file link from an episode
    pub async fn clear_media_file(&self, episode_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE episodes SET media_file_id = NULL, updated_at = NOW() WHERE id = $1")
            .bind(episode_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Mark episode as downloaded by linking to a media file
    pub async fn mark_downloaded(&self, episode_id: Uuid, media_file_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE episodes SET media_file_id = $2, updated_at = NOW() WHERE id = $1")
            .bind(episode_id)
            .bind(media_file_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get distinct seasons for a show
    pub async fn get_seasons(&self, tv_show_id: Uuid) -> Result<Vec<i32>> {
        let seasons: Vec<(i32,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT season
            FROM episodes
            WHERE tv_show_id = $1
            ORDER BY season
            "#,
        )
        .bind(tv_show_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(seasons.into_iter().map(|(s,)| s).collect())
    }

    /// Delete episodes for a show (used when refreshing metadata)
    pub async fn delete_by_show(&self, tv_show_id: Uuid) -> Result<u64> {
        let result = sqlx::query("DELETE FROM episodes WHERE tv_show_id = $1")
            .bind(tv_show_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Get upcoming episodes for a user across all libraries
    ///
    /// Returns episodes with air_date between today and today + days,
    /// ordered by air date ascending.
    pub async fn list_upcoming_by_user(
        &self,
        user_id: Uuid,
        days: i32,
    ) -> Result<Vec<UpcomingEpisodeRecord>> {
        let records = sqlx::query_as::<_, UpcomingEpisodeRecord>(
            r#"
            SELECT 
                e.id,
                e.tv_show_id,
                e.season,
                e.episode,
                e.title as episode_title,
                e.air_date,
                e.tvmaze_id as episode_tvmaze_id,
                e.media_file_id,
                ts.id as show_id,
                ts.name as show_name,
                ts.year as show_year,
                ts.network as show_network,
                ts.poster_url as show_poster_url,
                ts.library_id
            FROM episodes e
            JOIN tv_shows ts ON ts.id = e.tv_show_id
            JOIN libraries l ON l.id = ts.library_id
            WHERE l.user_id = $1 
              AND ts.monitored = true
              AND e.air_date >= CURRENT_DATE
              AND e.air_date <= CURRENT_DATE + $2::int
            ORDER BY e.air_date ASC, ts.name ASC, e.season ASC, e.episode ASC
            LIMIT 50
            "#,
        )
        .bind(user_id)
        .bind(days)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }
}

/// Record for upcoming episode queries with joined show info
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UpcomingEpisodeRecord {
    pub id: Uuid,
    pub tv_show_id: Uuid,
    pub season: i32,
    pub episode: i32,
    pub episode_title: Option<String>,
    pub air_date: Option<chrono::NaiveDate>,
    pub episode_tvmaze_id: Option<i32>,
    pub media_file_id: Option<Uuid>,
    pub show_id: Uuid,
    pub show_name: String,
    pub show_year: Option<i32>,
    pub show_network: Option<String>,
    pub show_poster_url: Option<String>,
    pub library_id: Uuid,
}
