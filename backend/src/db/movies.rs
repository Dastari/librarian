//! Movie database repository

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::services::text_utils::normalize_title;

/// Movie record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MovieRecord {
    pub id: Uuid,
    pub library_id: Uuid,
    pub user_id: Uuid,
    // Basic info
    pub title: String,
    pub sort_title: Option<String>,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    // External IDs
    pub tmdb_id: Option<i32>,
    pub imdb_id: Option<String>,
    // Metadata
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub runtime: Option<i32>,
    pub genres: Vec<String>,
    pub production_countries: Vec<String>,
    pub spoken_languages: Vec<String>,
    // Credits
    pub director: Option<String>,
    pub cast_names: Vec<String>,
    // Ratings
    pub tmdb_rating: Option<rust_decimal::Decimal>,
    pub tmdb_vote_count: Option<i32>,
    // Artwork
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    // Collection
    pub collection_id: Option<i32>,
    pub collection_name: Option<String>,
    pub collection_poster_url: Option<String>,
    // Release info
    pub release_date: Option<chrono::NaiveDate>,
    pub certification: Option<String>,
    // Status
    pub status: Option<String>,
    pub monitored: bool,
    // Media file link
    pub media_file_id: Option<Uuid>,
    // Timestamps
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Input for creating a movie
#[derive(Debug)]
pub struct CreateMovie {
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub sort_title: Option<String>,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub imdb_id: Option<String>,
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub runtime: Option<i32>,
    pub genres: Vec<String>,
    pub production_countries: Vec<String>,
    pub spoken_languages: Vec<String>,
    pub director: Option<String>,
    pub cast_names: Vec<String>,
    pub tmdb_rating: Option<rust_decimal::Decimal>,
    pub tmdb_vote_count: Option<i32>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub collection_id: Option<i32>,
    pub collection_name: Option<String>,
    pub collection_poster_url: Option<String>,
    pub release_date: Option<chrono::NaiveDate>,
    pub certification: Option<String>,
    pub status: Option<String>,
    pub monitored: bool,
}

/// Input for updating a movie
#[derive(Debug, Default)]
pub struct UpdateMovie {
    pub title: Option<String>,
    pub sort_title: Option<String>,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub runtime: Option<i32>,
    pub genres: Option<Vec<String>>,
    pub director: Option<String>,
    pub cast_names: Option<Vec<String>>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub monitored: Option<bool>,
}

/// Movie collection record
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MovieCollectionRecord {
    pub id: Uuid,
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub tmdb_collection_id: i32,
    pub name: String,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub movie_count: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub struct MovieRepository {
    pool: PgPool,
}

impl MovieRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// List all movies in a library
    pub async fn list_by_library(&self, library_id: Uuid) -> Result<Vec<MovieRecord>> {
        let records = sqlx::query_as::<_, MovieRecord>(
            r#"
            SELECT id, library_id, user_id, title, sort_title, original_title, year,
                   tmdb_id, imdb_id, overview, tagline, runtime, genres,
                   production_countries, spoken_languages, director, cast_names,
                   tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                   collection_id, collection_name, collection_poster_url,
                   release_date, certification, status, monitored,
                   media_file_id, created_at, updated_at
            FROM movies
            WHERE library_id = $1
            ORDER BY COALESCE(sort_title, title)
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List movies in a library with pagination and filtering
    ///
    /// Returns (records, total_count)
    #[allow(clippy::too_many_arguments)]
    pub async fn list_by_library_paginated(
        &self,
        library_id: Uuid,
        offset: i64,
        limit: i64,
        title_filter: Option<&str>,
        year_filter: Option<i32>,
        monitored_filter: Option<bool>,
        has_file_filter: Option<bool>,
        sort_column: &str,
        sort_asc: bool,
    ) -> Result<(Vec<MovieRecord>, i64)> {
        // Build dynamic WHERE clause conditions
        let mut conditions = vec!["library_id = $1".to_string()];
        let mut param_idx = 2;

        if title_filter.is_some() {
            conditions.push(format!("LOWER(title) LIKE ${}", param_idx));
            param_idx += 1;
        }
        if year_filter.is_some() {
            conditions.push(format!("year = ${}", param_idx));
            param_idx += 1;
        }
        if monitored_filter.is_some() {
            conditions.push(format!("monitored = ${}", param_idx));
            param_idx += 1;
        }
        if let Some(has_file) = has_file_filter {
            // has_file is now derived from media_file_id presence
            if has_file {
                conditions.push("media_file_id IS NOT NULL".to_string());
            } else {
                conditions.push("media_file_id IS NULL".to_string());
            }
        }
        let _ = param_idx; // Suppress unused warning

        let where_clause = conditions.join(" AND ");

        // Validate sort column to prevent SQL injection
        let valid_sort_columns = ["title", "sort_title", "year", "created_at", "release_date"];
        let sort_col = if valid_sort_columns.contains(&sort_column) {
            sort_column
        } else {
            "sort_title"
        };
        let order_dir = if sort_asc { "ASC" } else { "DESC" };
        let order_clause = format!(
            "ORDER BY COALESCE({}, title) {} NULLS LAST",
            sort_col, order_dir
        );

        // Build count query
        let count_query = format!("SELECT COUNT(*) FROM movies WHERE {}", where_clause);

        // Build data query
        let data_query = format!(
            r#"
            SELECT id, library_id, user_id, title, sort_title, original_title, year,
                   tmdb_id, imdb_id, overview, tagline, runtime, genres,
                   production_countries, spoken_languages, director, cast_names,
                   tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                   collection_id, collection_name, collection_poster_url,
                   release_date, certification, status, monitored,
                   media_file_id, created_at, updated_at
            FROM movies
            WHERE {}
            {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, order_clause, limit, offset
        );

        // Build and execute count query with bindings
        let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query).bind(library_id);
        if let Some(title) = title_filter {
            count_builder = count_builder.bind(format!("%{}%", title.to_lowercase()));
        }
        if let Some(year) = year_filter {
            count_builder = count_builder.bind(year);
        }
        if let Some(monitored) = monitored_filter {
            count_builder = count_builder.bind(monitored);
        }

        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        // Build and execute data query with bindings
        let mut data_builder = sqlx::query_as::<_, MovieRecord>(&data_query).bind(library_id);
        if let Some(title) = title_filter {
            data_builder = data_builder.bind(format!("%{}%", title.to_lowercase()));
        }
        if let Some(year) = year_filter {
            data_builder = data_builder.bind(year);
        }
        if let Some(monitored) = monitored_filter {
            data_builder = data_builder.bind(monitored);
        }

        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    /// List all movies for a user (across all libraries)
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<MovieRecord>> {
        let records = sqlx::query_as::<_, MovieRecord>(
            r#"
            SELECT id, library_id, user_id, title, sort_title, original_title, year,
                   tmdb_id, imdb_id, overview, tagline, runtime, genres,
                   production_countries, spoken_languages, director, cast_names,
                   tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                   collection_id, collection_name, collection_poster_url,
                   release_date, certification, status, monitored,
                   media_file_id, created_at, updated_at
            FROM movies
            WHERE user_id = $1
            ORDER BY COALESCE(sort_title, title)
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get a movie by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<MovieRecord>> {
        let record = sqlx::query_as::<_, MovieRecord>(
            r#"
            SELECT id, library_id, user_id, title, sort_title, original_title, year,
                   tmdb_id, imdb_id, overview, tagline, runtime, genres,
                   production_countries, spoken_languages, director, cast_names,
                   tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                   collection_id, collection_name, collection_poster_url,
                   release_date, certification, status, monitored,
                   media_file_id, created_at, updated_at
            FROM movies
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a movie by TMDB ID within a library
    pub async fn get_by_tmdb_id(
        &self,
        library_id: Uuid,
        tmdb_id: i32,
    ) -> Result<Option<MovieRecord>> {
        let record = sqlx::query_as::<_, MovieRecord>(
            r#"
            SELECT id, library_id, user_id, title, sort_title, original_title, year,
                   tmdb_id, imdb_id, overview, tagline, runtime, genres,
                   production_countries, spoken_languages, director, cast_names,
                   tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                   collection_id, collection_name, collection_poster_url,
                   release_date, certification, status, monitored,
                   media_file_id, created_at, updated_at
            FROM movies
            WHERE library_id = $1 AND tmdb_id = $2
            "#,
        )
        .bind(library_id)
        .bind(tmdb_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create a new movie
    pub async fn create(&self, input: CreateMovie) -> Result<MovieRecord> {
        let record = sqlx::query_as::<_, MovieRecord>(
            r#"
            INSERT INTO movies (
                library_id, user_id, title, sort_title, original_title, year,
                tmdb_id, imdb_id, overview, tagline, runtime, genres,
                production_countries, spoken_languages, director, cast_names,
                tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                collection_id, collection_name, collection_poster_url,
                release_date, certification, status, monitored
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                    $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27)
            RETURNING id, library_id, user_id, title, sort_title, original_title, year,
                      tmdb_id, imdb_id, overview, tagline, runtime, genres,
                      production_countries, spoken_languages, director, cast_names,
                      tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                      collection_id, collection_name, collection_poster_url,
                      release_date, certification, status, monitored,
                      media_file_id, created_at, updated_at
            "#,
        )
        .bind(input.library_id)
        .bind(input.user_id)
        .bind(&input.title)
        .bind(&input.sort_title)
        .bind(&input.original_title)
        .bind(input.year)
        .bind(input.tmdb_id)
        .bind(&input.imdb_id)
        .bind(&input.overview)
        .bind(&input.tagline)
        .bind(input.runtime)
        .bind(&input.genres)
        .bind(&input.production_countries)
        .bind(&input.spoken_languages)
        .bind(&input.director)
        .bind(&input.cast_names)
        .bind(input.tmdb_rating)
        .bind(input.tmdb_vote_count)
        .bind(&input.poster_url)
        .bind(&input.backdrop_url)
        .bind(input.collection_id)
        .bind(&input.collection_name)
        .bind(&input.collection_poster_url)
        .bind(input.release_date)
        .bind(&input.certification)
        .bind(&input.status)
        .bind(input.monitored)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update a movie
    pub async fn update(&self, id: Uuid, input: UpdateMovie) -> Result<Option<MovieRecord>> {
        let record = sqlx::query_as::<_, MovieRecord>(
            r#"
            UPDATE movies SET
                title = COALESCE($2, title),
                sort_title = COALESCE($3, sort_title),
                original_title = COALESCE($4, original_title),
                overview = COALESCE($5, overview),
                tagline = COALESCE($6, tagline),
                runtime = COALESCE($7, runtime),
                genres = COALESCE($8, genres),
                director = COALESCE($9, director),
                cast_names = COALESCE($10, cast_names),
                poster_url = COALESCE($11, poster_url),
                backdrop_url = COALESCE($12, backdrop_url),
                monitored = COALESCE($13, monitored),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, library_id, user_id, title, sort_title, original_title, year,
                      tmdb_id, imdb_id, overview, tagline, runtime, genres,
                      production_countries, spoken_languages, director, cast_names,
                      tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                      collection_id, collection_name, collection_poster_url,
                      release_date, certification, status, monitored,
                      media_file_id, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&input.title)
        .bind(&input.sort_title)
        .bind(&input.original_title)
        .bind(&input.overview)
        .bind(&input.tagline)
        .bind(input.runtime)
        .bind(&input.genres)
        .bind(&input.director)
        .bind(&input.cast_names)
        .bind(&input.poster_url)
        .bind(&input.backdrop_url)
        .bind(input.monitored)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete a movie
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM movies WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Link a movie to a media file
    pub async fn set_media_file(&self, movie_id: Uuid, media_file_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE movies SET media_file_id = $2, updated_at = NOW() WHERE id = $1")
            .bind(movie_id)
            .bind(media_file_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Clear the media file link from a movie
    pub async fn clear_media_file(&self, movie_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE movies SET media_file_id = NULL, updated_at = NOW() WHERE id = $1")
            .bind(movie_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get movie count for a library
    pub async fn count_by_library(&self, library_id: Uuid) -> Result<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM movies WHERE library_id = $1")
            .bind(library_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(count)
    }

    /// List movies in a collection
    pub async fn list_by_collection(&self, collection_id: i32) -> Result<Vec<MovieRecord>> {
        let records = sqlx::query_as::<_, MovieRecord>(
            r#"
            SELECT id, library_id, user_id, title, sort_title, original_title, year,
                   tmdb_id, imdb_id, overview, tagline, runtime, genres,
                   production_countries, spoken_languages, director, cast_names,
                   tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                   collection_id, collection_name, collection_poster_url,
                   release_date, certification, status, monitored,
                   media_file_id, created_at, updated_at
            FROM movies
            WHERE collection_id = $1
            ORDER BY release_date
            "#,
        )
        .bind(collection_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Search movies by title
    pub async fn search(&self, library_id: Uuid, query: &str) -> Result<Vec<MovieRecord>> {
        let search_pattern = format!("%{}%", query.to_lowercase());
        let records = sqlx::query_as::<_, MovieRecord>(
            r#"
            SELECT id, library_id, user_id, title, sort_title, original_title, year,
                   tmdb_id, imdb_id, overview, tagline, runtime, genres,
                   production_countries, spoken_languages, director, cast_names,
                   tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                   collection_id, collection_name, collection_poster_url,
                   release_date, certification, status, monitored,
                   media_file_id, created_at, updated_at
            FROM movies
            WHERE library_id = $1 AND (
                LOWER(title) LIKE $2 OR
                LOWER(original_title) LIKE $2 OR
                LOWER(director) LIKE $2
            )
            ORDER BY COALESCE(sort_title, title)
            LIMIT 50
            "#,
        )
        .bind(library_id)
        .bind(&search_pattern)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Find a movie in a library by title and optional year
    ///
    /// This uses fuzzy matching to find movies that match the parsed filename.
    /// The matching logic:
    /// 1. If year is provided, prefer exact year matches
    /// 2. Otherwise use normalized title matching
    pub async fn find_by_title_in_library(
        &self,
        library_id: Uuid,
        title: &str,
        year: Option<i32>,
    ) -> Result<Option<MovieRecord>> {
        // Normalize the input title for comparison
        let normalized_title = normalize_title(title);

        // First try exact year match if year is provided
        if let Some(y) = year {
            let record = sqlx::query_as::<_, MovieRecord>(
                r#"
                SELECT id, library_id, user_id, title, sort_title, original_title, year,
                       tmdb_id, imdb_id, overview, tagline, runtime, genres,
                       production_countries, spoken_languages, director, cast_names,
                       tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                       collection_id, collection_name, collection_poster_url,
                       release_date, certification, status, monitored,
                       media_file_id, created_at, updated_at
                FROM movies
                WHERE library_id = $1 AND year = $2 AND (
                    LOWER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(title, '''', ''), ':', ''), '-', ''), '.', ''), '_', '')) = $3 OR
                    LOWER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(original_title, '''', ''), ':', ''), '-', ''), '.', ''), '_', '')) = $3
                )
                LIMIT 1
                "#,
            )
            .bind(library_id)
            .bind(y)
            .bind(&normalized_title)
            .fetch_optional(&self.pool)
            .await?;

            if record.is_some() {
                return Ok(record);
            }

            // Try with year +/- 1 (sometimes metadata years differ slightly)
            let record = sqlx::query_as::<_, MovieRecord>(
                r#"
                SELECT id, library_id, user_id, title, sort_title, original_title, year,
                       tmdb_id, imdb_id, overview, tagline, runtime, genres,
                       production_countries, spoken_languages, director, cast_names,
                       tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                       collection_id, collection_name, collection_poster_url,
                       release_date, certification, status, monitored,
                       media_file_id, created_at, updated_at
                FROM movies
                WHERE library_id = $1 AND year BETWEEN $2 AND $3 AND (
                    LOWER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(title, '''', ''), ':', ''), '-', ''), '.', ''), '_', '')) = $4 OR
                    LOWER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(original_title, '''', ''), ':', ''), '-', ''), '.', ''), '_', '')) = $4
                )
                ORDER BY ABS(year - $5)
                LIMIT 1
                "#,
            )
            .bind(library_id)
            .bind(y - 1)
            .bind(y + 1)
            .bind(&normalized_title)
            .bind(y)
            .fetch_optional(&self.pool)
            .await?;

            if record.is_some() {
                return Ok(record);
            }
        }

        // Try title-only match (less reliable, but may work for unique titles)
        let record = sqlx::query_as::<_, MovieRecord>(
            r#"
            SELECT id, library_id, user_id, title, sort_title, original_title, year,
                   tmdb_id, imdb_id, overview, tagline, runtime, genres,
                   production_countries, spoken_languages, director, cast_names,
                   tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                   collection_id, collection_name, collection_poster_url,
                   release_date, certification, status, monitored,
                   media_file_id, created_at, updated_at
            FROM movies
            WHERE library_id = $1 AND (
                LOWER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(title, '''', ''), ':', ''), '-', ''), '.', ''), '_', '')) = $2 OR
                LOWER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(original_title, '''', ''), ':', ''), '-', ''), '.', ''), '_', '')) = $2
            )
            LIMIT 1
            "#,
        )
        .bind(library_id)
        .bind(&normalized_title)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Find movies in a library that need files (monitored and missing files)
    pub async fn list_wanted(&self, library_id: Uuid) -> Result<Vec<MovieRecord>> {
        let records = sqlx::query_as::<_, MovieRecord>(
            r#"
            SELECT id, library_id, user_id, title, sort_title, original_title, year,
                   tmdb_id, imdb_id, overview, tagline, runtime, genres,
                   production_countries, spoken_languages, director, cast_names,
                   tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                   collection_id, collection_name, collection_poster_url,
                   release_date, certification, status, monitored,
                   media_file_id, created_at, updated_at
            FROM movies
            WHERE library_id = $1 AND monitored = true AND media_file_id IS NULL
            ORDER BY COALESCE(sort_title, title)
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }
}

// normalize_title moved to services/text_utils.rs
// NOTE: The SQL queries in find_by_title_in_library must match the normalization!
