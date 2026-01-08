//! TV Show database repository

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// TV Show record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TvShowRecord {
    pub id: Uuid,
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub sort_name: Option<String>,
    pub year: Option<i32>,
    pub status: String,
    pub tvmaze_id: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub tvdb_id: Option<i32>,
    pub imdb_id: Option<String>,
    pub overview: Option<String>,
    pub network: Option<String>,
    pub runtime: Option<i32>,
    pub genres: Vec<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub monitored: bool,
    pub monitor_type: String,
    pub quality_profile_id: Option<Uuid>,
    pub path: Option<String>,
    pub episode_count: Option<i32>,
    pub episode_file_count: Option<i32>,
    pub size_bytes: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Input for creating a TV show
#[derive(Debug)]
pub struct CreateTvShow {
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub sort_name: Option<String>,
    pub year: Option<i32>,
    pub status: Option<String>,
    pub tvmaze_id: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub tvdb_id: Option<i32>,
    pub imdb_id: Option<String>,
    pub overview: Option<String>,
    pub network: Option<String>,
    pub runtime: Option<i32>,
    pub genres: Vec<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub monitored: bool,
    pub monitor_type: String,
    pub quality_profile_id: Option<Uuid>,
    pub path: Option<String>,
}

/// Input for updating a TV show
#[derive(Debug, Default)]
pub struct UpdateTvShow {
    pub name: Option<String>,
    pub sort_name: Option<String>,
    pub year: Option<i32>,
    pub status: Option<String>,
    pub overview: Option<String>,
    pub network: Option<String>,
    pub runtime: Option<i32>,
    pub genres: Option<Vec<String>>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub monitored: Option<bool>,
    pub monitor_type: Option<String>,
    pub quality_profile_id: Option<Uuid>,
    pub path: Option<String>,
}

pub struct TvShowRepository {
    pool: PgPool,
}

impl TvShowRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all TV shows for a library
    pub async fn list_by_library(&self, library_id: Uuid) -> Result<Vec<TvShowRecord>> {
        let records = sqlx::query_as::<_, TvShowRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, year, status,
                   tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                   runtime, genres, poster_url, backdrop_url, monitored,
                   monitor_type, quality_profile_id, path, episode_count,
                   episode_file_count, size_bytes, created_at, updated_at
            FROM tv_shows
            WHERE library_id = $1
            ORDER BY COALESCE(sort_name, name)
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get all monitored TV shows for a user
    pub async fn list_monitored_by_user(&self, user_id: Uuid) -> Result<Vec<TvShowRecord>> {
        let records = sqlx::query_as::<_, TvShowRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, year, status,
                   tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                   runtime, genres, poster_url, backdrop_url, monitored,
                   monitor_type, quality_profile_id, path, episode_count,
                   episode_file_count, size_bytes, created_at, updated_at
            FROM tv_shows
            WHERE user_id = $1 AND monitored = true
            ORDER BY name
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get a TV show by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<TvShowRecord>> {
        let record = sqlx::query_as::<_, TvShowRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, year, status,
                   tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                   runtime, genres, poster_url, backdrop_url, monitored,
                   monitor_type, quality_profile_id, path, episode_count,
                   episode_file_count, size_bytes, created_at, updated_at
            FROM tv_shows
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a TV show by TVMaze ID in a library
    pub async fn get_by_tvmaze_id(&self, library_id: Uuid, tvmaze_id: i32) -> Result<Option<TvShowRecord>> {
        let record = sqlx::query_as::<_, TvShowRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, year, status,
                   tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                   runtime, genres, poster_url, backdrop_url, monitored,
                   monitor_type, quality_profile_id, path, episode_count,
                   episode_file_count, size_bytes, created_at, updated_at
            FROM tv_shows
            WHERE library_id = $1 AND tvmaze_id = $2
            "#,
        )
        .bind(library_id)
        .bind(tvmaze_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create a new TV show
    pub async fn create(&self, input: CreateTvShow) -> Result<TvShowRecord> {
        let record = sqlx::query_as::<_, TvShowRecord>(
            r#"
            INSERT INTO tv_shows (
                library_id, user_id, name, sort_name, year, status,
                tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                runtime, genres, poster_url, backdrop_url, monitored,
                monitor_type, quality_profile_id, path
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
            RETURNING id, library_id, user_id, name, sort_name, year, status,
                      tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                      runtime, genres, poster_url, backdrop_url, monitored,
                      monitor_type, quality_profile_id, path, episode_count,
                      episode_file_count, size_bytes, created_at, updated_at
            "#,
        )
        .bind(input.library_id)
        .bind(input.user_id)
        .bind(&input.name)
        .bind(&input.sort_name)
        .bind(input.year)
        .bind(input.status.as_deref().unwrap_or("unknown"))
        .bind(input.tvmaze_id)
        .bind(input.tmdb_id)
        .bind(input.tvdb_id)
        .bind(&input.imdb_id)
        .bind(&input.overview)
        .bind(&input.network)
        .bind(input.runtime)
        .bind(&input.genres)
        .bind(&input.poster_url)
        .bind(&input.backdrop_url)
        .bind(input.monitored)
        .bind(&input.monitor_type)
        .bind(input.quality_profile_id)
        .bind(&input.path)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update a TV show
    pub async fn update(&self, id: Uuid, input: UpdateTvShow) -> Result<Option<TvShowRecord>> {
        let record = sqlx::query_as::<_, TvShowRecord>(
            r#"
            UPDATE tv_shows SET
                name = COALESCE($2, name),
                sort_name = COALESCE($3, sort_name),
                year = COALESCE($4, year),
                status = COALESCE($5, status),
                overview = COALESCE($6, overview),
                network = COALESCE($7, network),
                runtime = COALESCE($8, runtime),
                genres = COALESCE($9, genres),
                poster_url = COALESCE($10, poster_url),
                backdrop_url = COALESCE($11, backdrop_url),
                monitored = COALESCE($12, monitored),
                monitor_type = COALESCE($13, monitor_type),
                quality_profile_id = COALESCE($14, quality_profile_id),
                path = COALESCE($15, path),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, library_id, user_id, name, sort_name, year, status,
                      tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                      runtime, genres, poster_url, backdrop_url, monitored,
                      monitor_type, quality_profile_id, path, episode_count,
                      episode_file_count, size_bytes, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&input.name)
        .bind(&input.sort_name)
        .bind(input.year)
        .bind(&input.status)
        .bind(&input.overview)
        .bind(&input.network)
        .bind(input.runtime)
        .bind(&input.genres)
        .bind(&input.poster_url)
        .bind(&input.backdrop_url)
        .bind(input.monitored)
        .bind(&input.monitor_type)
        .bind(input.quality_profile_id)
        .bind(&input.path)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete a TV show
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM tv_shows WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update episode statistics for a show
    pub async fn update_stats(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE tv_shows SET
                episode_count = (SELECT COUNT(*) FROM episodes WHERE tv_show_id = $1),
                episode_file_count = (SELECT COUNT(*) FROM episodes WHERE tv_show_id = $1 AND status = 'downloaded'),
                size_bytes = (
                    SELECT COALESCE(SUM(mf.size), 0)
                    FROM episodes e
                    JOIN media_files mf ON mf.episode_id = e.id
                    WHERE e.tv_show_id = $1
                ),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
