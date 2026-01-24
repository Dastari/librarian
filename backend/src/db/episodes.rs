//! Episode database repository

use anyhow::Result;
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

/// Episode record from database
#[derive(Debug, Clone)]
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


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for EpisodeRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use crate::db::sqlite_helpers::{str_to_uuid, str_to_datetime};

        let id_str: String = row.try_get("id")?;
        let tv_show_id_str: String = row.try_get("tv_show_id")?;
        let media_file_id_str: Option<String> = row.try_get("media_file_id")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;
        let air_date_str: Option<String> = row.try_get("air_date")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            tv_show_id: str_to_uuid(&tv_show_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            season: row.try_get("season")?,
            episode: row.try_get("episode")?,
            absolute_number: row.try_get("absolute_number")?,
            title: row.try_get("title")?,
            overview: row.try_get("overview")?,
            air_date: air_date_str
                .map(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            runtime: row.try_get("runtime")?,
            tvmaze_id: row.try_get("tvmaze_id")?,
            tmdb_id: row.try_get("tmdb_id")?,
            tvdb_id: row.try_get("tvdb_id")?,
            media_file_id: media_file_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            created_at: str_to_datetime(&created_at_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_at_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
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

/// Record for upcoming episode queries with joined show info
#[derive(Debug, Clone)]
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


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for UpcomingEpisodeRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use crate::db::sqlite_helpers::str_to_uuid;

        let id_str: String = row.try_get("id")?;
        let tv_show_id_str: String = row.try_get("tv_show_id")?;
        let media_file_id_str: Option<String> = row.try_get("media_file_id")?;
        let show_id_str: String = row.try_get("show_id")?;
        let library_id_str: String = row.try_get("library_id")?;
        let air_date_str: Option<String> = row.try_get("air_date")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            tv_show_id: str_to_uuid(&tv_show_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            season: row.try_get("season")?,
            episode: row.try_get("episode")?,
            episode_title: row.try_get("episode_title")?,
            air_date: air_date_str
                .map(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            episode_tvmaze_id: row.try_get("episode_tvmaze_id")?,
            media_file_id: media_file_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            show_id: str_to_uuid(&show_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            show_name: row.try_get("show_name")?,
            show_year: row.try_get("show_year")?,
            show_network: row.try_get("show_network")?,
            show_poster_url: row.try_get("show_poster_url")?,
            library_id: str_to_uuid(&library_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

pub struct EpisodeRepository {
    pool: DbPool,
}

impl EpisodeRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get all episodes for a TV show

    #[cfg(feature = "sqlite")]
    pub async fn list_by_show(&self, tv_show_id: Uuid) -> Result<Vec<EpisodeRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let records = sqlx::query_as::<_, EpisodeRecord>(
            r#"
            SELECT id, tv_show_id, season, episode, absolute_number, title,
                   overview, air_date, runtime, tvmaze_id, tmdb_id, tvdb_id,
                   media_file_id, created_at, updated_at
            FROM episodes
            WHERE tv_show_id = ?1
            ORDER BY season, episode
            "#,
        )
        .bind(uuid_to_str(tv_show_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get episodes for a specific season

    #[cfg(feature = "sqlite")]
    pub async fn list_by_season(
        &self,
        tv_show_id: Uuid,
        season: i32,
    ) -> Result<Vec<EpisodeRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let records = sqlx::query_as::<_, EpisodeRecord>(
            r#"
            SELECT id, tv_show_id, season, episode, absolute_number, title,
                   overview, air_date, runtime, tvmaze_id, tmdb_id, tvdb_id,
                   media_file_id, created_at, updated_at
            FROM episodes
            WHERE tv_show_id = ?1 AND season = ?2
            ORDER BY episode
            "#,
        )
        .bind(uuid_to_str(tv_show_id))
        .bind(season)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get wanted episodes (no media file linked) for a library

    #[cfg(feature = "sqlite")]
    pub async fn list_wanted_by_library(&self, library_id: Uuid) -> Result<Vec<EpisodeRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let records = sqlx::query_as::<_, EpisodeRecord>(
            r#"
            SELECT e.id, e.tv_show_id, e.season, e.episode, e.absolute_number, e.title,
                   e.overview, e.air_date, e.runtime, e.tvmaze_id, e.tmdb_id, e.tvdb_id,
                   e.media_file_id, e.created_at, e.updated_at
            FROM episodes e
            JOIN tv_shows ts ON ts.id = e.tv_show_id
            WHERE ts.library_id = ?1 
              AND ts.monitored = 1
              AND e.media_file_id IS NULL
              AND (e.air_date IS NULL OR e.air_date <= date('now'))
            ORDER BY e.air_date DESC, ts.name, e.season, e.episode
            "#,
        )
        .bind(uuid_to_str(library_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get wanted episodes for a user across all libraries

    #[cfg(feature = "sqlite")]
    pub async fn list_wanted_by_user(&self, user_id: Uuid) -> Result<Vec<EpisodeRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let records = sqlx::query_as::<_, EpisodeRecord>(
            r#"
            SELECT e.id, e.tv_show_id, e.season, e.episode, e.absolute_number, e.title,
                   e.overview, e.air_date, e.runtime, e.tvmaze_id, e.tmdb_id, e.tvdb_id,
                   e.media_file_id, e.created_at, e.updated_at
            FROM episodes e
            JOIN tv_shows ts ON ts.id = e.tv_show_id
            WHERE ts.user_id = ?1 
              AND ts.monitored = 1
              AND e.media_file_id IS NULL
              AND (e.air_date IS NULL OR e.air_date <= date('now'))
            ORDER BY e.air_date DESC, ts.name, e.season, e.episode
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get an episode by ID

    #[cfg(feature = "sqlite")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<EpisodeRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let record = sqlx::query_as::<_, EpisodeRecord>(
            r#"
            SELECT id, tv_show_id, season, episode, absolute_number, title,
                   overview, air_date, runtime, tvmaze_id, tmdb_id, tvdb_id,
                   media_file_id, created_at, updated_at
            FROM episodes
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get an episode by show, season, and episode number

    #[cfg(feature = "sqlite")]
    pub async fn get_by_show_season_episode(
        &self,
        tv_show_id: Uuid,
        season: i32,
        episode: i32,
    ) -> Result<Option<EpisodeRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let record = sqlx::query_as::<_, EpisodeRecord>(
            r#"
            SELECT id, tv_show_id, season, episode, absolute_number, title,
                   overview, air_date, runtime, tvmaze_id, tmdb_id, tvdb_id,
                   media_file_id, created_at, updated_at
            FROM episodes
            WHERE tv_show_id = ?1 AND season = ?2 AND episode = ?3
            "#,
        )
        .bind(uuid_to_str(tv_show_id))
        .bind(season)
        .bind(episode)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create a new episode

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, input: CreateEpisode) -> Result<EpisodeRecord> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let tv_show_id_str = uuid_to_str(input.tv_show_id);
        let air_date_str = input.air_date.map(|d| d.format("%Y-%m-%d").to_string());

        // Check if exists
        let existing = self
            .get_by_show_season_episode(input.tv_show_id, input.season, input.episode)
            .await?;

        if existing.is_some() {
            // Update existing
            sqlx::query(
                r#"
                UPDATE episodes SET
                    title = COALESCE(?4, title),
                    overview = COALESCE(?5, overview),
                    air_date = COALESCE(?6, air_date),
                    runtime = COALESCE(?7, runtime),
                    tvmaze_id = COALESCE(?8, tvmaze_id),
                    tmdb_id = COALESCE(?9, tmdb_id),
                    tvdb_id = COALESCE(?10, tvdb_id),
                    updated_at = datetime('now')
                WHERE tv_show_id = ?1 AND season = ?2 AND episode = ?3
                "#,
            )
            .bind(&tv_show_id_str)
            .bind(input.season)
            .bind(input.episode)
            .bind(&input.title)
            .bind(&input.overview)
            .bind(&air_date_str)
            .bind(input.runtime)
            .bind(input.tvmaze_id)
            .bind(input.tmdb_id)
            .bind(input.tvdb_id)
            .execute(&self.pool)
            .await?;
        } else {
            // Insert new
            let id = Uuid::new_v4();
            let id_str = uuid_to_str(id);

            sqlx::query(
                r#"
                INSERT INTO episodes (
                    id, tv_show_id, season, episode, absolute_number, title,
                    overview, air_date, runtime, tvmaze_id, tmdb_id, tvdb_id,
                    created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, datetime('now'), datetime('now'))
                "#,
            )
            .bind(&id_str)
            .bind(&tv_show_id_str)
            .bind(input.season)
            .bind(input.episode)
            .bind(input.absolute_number)
            .bind(&input.title)
            .bind(&input.overview)
            .bind(&air_date_str)
            .bind(input.runtime)
            .bind(input.tvmaze_id)
            .bind(input.tmdb_id)
            .bind(input.tvdb_id)
            .execute(&self.pool)
            .await?;
        }

        self.get_by_show_season_episode(input.tv_show_id, input.season, input.episode)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve episode after upsert"))
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

    #[cfg(feature = "sqlite")]
    pub async fn set_media_file(&self, episode_id: Uuid, media_file_id: Uuid) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;

        sqlx::query("UPDATE episodes SET media_file_id = ?2, updated_at = datetime('now') WHERE id = ?1")
            .bind(uuid_to_str(episode_id))
            .bind(uuid_to_str(media_file_id))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Clear the media file link from an episode

    #[cfg(feature = "sqlite")]
    pub async fn clear_media_file(&self, episode_id: Uuid) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;

        sqlx::query("UPDATE episodes SET media_file_id = NULL, updated_at = datetime('now') WHERE id = ?1")
            .bind(uuid_to_str(episode_id))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Mark episode as downloaded by linking to a media file

    #[cfg(feature = "sqlite")]
    pub async fn mark_downloaded(&self, episode_id: Uuid, media_file_id: Uuid) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;

        sqlx::query("UPDATE episodes SET media_file_id = ?2, updated_at = datetime('now') WHERE id = ?1")
            .bind(uuid_to_str(episode_id))
            .bind(uuid_to_str(media_file_id))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get distinct seasons for a show

    #[cfg(feature = "sqlite")]
    pub async fn get_seasons(&self, tv_show_id: Uuid) -> Result<Vec<i32>> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let seasons: Vec<(i32,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT season
            FROM episodes
            WHERE tv_show_id = ?1
            ORDER BY season
            "#,
        )
        .bind(uuid_to_str(tv_show_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(seasons.into_iter().map(|(s,)| s).collect())
    }

    /// Delete episodes for a show (used when refreshing metadata)

    #[cfg(feature = "sqlite")]
    pub async fn delete_by_show(&self, tv_show_id: Uuid) -> Result<u64> {
        use crate::db::sqlite_helpers::uuid_to_str;

        let result = sqlx::query("DELETE FROM episodes WHERE tv_show_id = ?1")
            .bind(uuid_to_str(tv_show_id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Get upcoming episodes for a user across all libraries
    ///
    /// Returns episodes with air_date between today and today + days,
    /// ordered by air date ascending.

    #[cfg(feature = "sqlite")]
    pub async fn list_upcoming_by_user(
        &self,
        user_id: Uuid,
        days: i32,
    ) -> Result<Vec<UpcomingEpisodeRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;

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
            WHERE l.user_id = ?1 
              AND ts.monitored = 1
              AND e.air_date >= date('now')
              AND e.air_date <= date('now', '+' || ?2 || ' days')
            ORDER BY e.air_date ASC, ts.name ASC, e.season ASC, e.episode ASC
            LIMIT 50
            "#,
        )
        .bind(uuid_to_str(user_id))
        .bind(days)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }
}
