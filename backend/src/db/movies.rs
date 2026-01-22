//! Movie database repository

use anyhow::Result;
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "postgres")]
type DbPool = PgPool;
#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

use crate::services::text_utils::normalize_title;

/// Movie record from database
#[derive(Debug, Clone)]
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

#[cfg(feature = "postgres")]
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for MovieRecord {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            library_id: row.try_get("library_id")?,
            user_id: row.try_get("user_id")?,
            title: row.try_get("title")?,
            sort_title: row.try_get("sort_title")?,
            original_title: row.try_get("original_title")?,
            year: row.try_get("year")?,
            tmdb_id: row.try_get("tmdb_id")?,
            imdb_id: row.try_get("imdb_id")?,
            overview: row.try_get("overview")?,
            tagline: row.try_get("tagline")?,
            runtime: row.try_get("runtime")?,
            genres: row.try_get("genres")?,
            production_countries: row.try_get("production_countries")?,
            spoken_languages: row.try_get("spoken_languages")?,
            director: row.try_get("director")?,
            cast_names: row.try_get("cast_names")?,
            tmdb_rating: row.try_get("tmdb_rating")?,
            tmdb_vote_count: row.try_get("tmdb_vote_count")?,
            poster_url: row.try_get("poster_url")?,
            backdrop_url: row.try_get("backdrop_url")?,
            collection_id: row.try_get("collection_id")?,
            collection_name: row.try_get("collection_name")?,
            collection_poster_url: row.try_get("collection_poster_url")?,
            release_date: row.try_get("release_date")?,
            certification: row.try_get("certification")?,
            status: row.try_get("status")?,
            monitored: row.try_get("monitored")?,
            media_file_id: row.try_get("media_file_id")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for MovieRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use crate::db::sqlite_helpers::{str_to_uuid, str_to_datetime, int_to_bool, json_to_vec};
        use std::str::FromStr;
        
        let id_str: String = row.try_get("id")?;
        let library_id_str: String = row.try_get("library_id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let media_file_id_str: Option<String> = row.try_get("media_file_id")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;
        
        // JSON arrays stored as TEXT
        let genres_json: String = row.try_get("genres")?;
        let production_countries_json: String = row.try_get("production_countries")?;
        let spoken_languages_json: String = row.try_get("spoken_languages")?;
        let cast_names_json: String = row.try_get("cast_names")?;
        
        // Decimal stored as TEXT
        let tmdb_rating_str: Option<String> = row.try_get("tmdb_rating")?;
        
        // Boolean stored as INTEGER
        let monitored: i32 = row.try_get("monitored")?;
        
        // NaiveDate stored as TEXT (YYYY-MM-DD)
        let release_date_str: Option<String> = row.try_get("release_date")?;
        
        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            library_id: str_to_uuid(&library_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            title: row.try_get("title")?,
            sort_title: row.try_get("sort_title")?,
            original_title: row.try_get("original_title")?,
            year: row.try_get("year")?,
            tmdb_id: row.try_get("tmdb_id")?,
            imdb_id: row.try_get("imdb_id")?,
            overview: row.try_get("overview")?,
            tagline: row.try_get("tagline")?,
            runtime: row.try_get("runtime")?,
            genres: json_to_vec(&genres_json),
            production_countries: json_to_vec(&production_countries_json),
            spoken_languages: json_to_vec(&spoken_languages_json),
            director: row.try_get("director")?,
            cast_names: json_to_vec(&cast_names_json),
            tmdb_rating: tmdb_rating_str
                .map(|s| rust_decimal::Decimal::from_str(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            tmdb_vote_count: row.try_get("tmdb_vote_count")?,
            poster_url: row.try_get("poster_url")?,
            backdrop_url: row.try_get("backdrop_url")?,
            collection_id: row.try_get("collection_id")?,
            collection_name: row.try_get("collection_name")?,
            collection_poster_url: row.try_get("collection_poster_url")?,
            release_date: release_date_str
                .map(|s| chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            certification: row.try_get("certification")?,
            status: row.try_get("status")?,
            monitored: int_to_bool(monitored),
            media_file_id: media_file_id_str
                .map(|s| str_to_uuid(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            created_at: str_to_datetime(&created_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
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
#[derive(Debug, Clone)]
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

#[cfg(feature = "postgres")]
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for MovieCollectionRecord {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            library_id: row.try_get("library_id")?,
            user_id: row.try_get("user_id")?,
            tmdb_collection_id: row.try_get("tmdb_collection_id")?,
            name: row.try_get("name")?,
            overview: row.try_get("overview")?,
            poster_url: row.try_get("poster_url")?,
            backdrop_url: row.try_get("backdrop_url")?,
            movie_count: row.try_get("movie_count")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for MovieCollectionRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use crate::db::sqlite_helpers::{str_to_uuid, str_to_datetime};
        
        let id_str: String = row.try_get("id")?;
        let library_id_str: String = row.try_get("library_id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;
        
        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            library_id: str_to_uuid(&library_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            tmdb_collection_id: row.try_get("tmdb_collection_id")?,
            name: row.try_get("name")?,
            overview: row.try_get("overview")?,
            poster_url: row.try_get("poster_url")?,
            backdrop_url: row.try_get("backdrop_url")?,
            movie_count: row.try_get("movie_count")?,
            created_at: str_to_datetime(&created_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

pub struct MovieRepository {
    pool: DbPool,
}

impl MovieRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// List all movies in a library
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn list_by_library(&self, library_id: Uuid) -> Result<Vec<MovieRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
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
            WHERE library_id = ?1
            ORDER BY COALESCE(sort_title, title)
            "#,
        )
        .bind(uuid_to_str(library_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List movies in a library with pagination and filtering
    ///
    /// Returns (records, total_count)
    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "postgres")]
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

    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "sqlite")]
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
        use crate::db::sqlite_helpers::{uuid_to_str, bool_to_int};
        
        // Build dynamic WHERE clause conditions
        let mut conditions = vec!["library_id = ?1".to_string()];
        let mut param_idx = 2;

        if title_filter.is_some() {
            conditions.push(format!("LOWER(title) LIKE ?{}", param_idx));
            param_idx += 1;
        }
        if year_filter.is_some() {
            conditions.push(format!("year = ?{}", param_idx));
            param_idx += 1;
        }
        if monitored_filter.is_some() {
            conditions.push(format!("monitored = ?{}", param_idx));
            param_idx += 1;
        }
        if let Some(has_file) = has_file_filter {
            if has_file {
                conditions.push("media_file_id IS NOT NULL".to_string());
            } else {
                conditions.push("media_file_id IS NULL".to_string());
            }
        }
        let _ = param_idx;

        let where_clause = conditions.join(" AND ");

        // Validate sort column to prevent SQL injection
        let valid_sort_columns = ["title", "sort_title", "year", "created_at", "release_date"];
        let sort_col = if valid_sort_columns.contains(&sort_column) {
            sort_column
        } else {
            "sort_title"
        };
        let order_dir = if sort_asc { "ASC" } else { "DESC" };
        // SQLite doesn't support NULLS LAST directly, use CASE expression
        let order_clause = format!(
            "ORDER BY CASE WHEN COALESCE({}, title) IS NULL THEN 1 ELSE 0 END, COALESCE({}, title) {}",
            sort_col, sort_col, order_dir
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

        let library_id_str = uuid_to_str(library_id);
        
        // Build and execute count query with bindings
        let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query).bind(&library_id_str);
        if let Some(title) = title_filter {
            count_builder = count_builder.bind(format!("%{}%", title.to_lowercase()));
        }
        if let Some(year) = year_filter {
            count_builder = count_builder.bind(year);
        }
        if let Some(monitored) = monitored_filter {
            count_builder = count_builder.bind(bool_to_int(monitored));
        }

        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        // Build and execute data query with bindings
        let mut data_builder = sqlx::query_as::<_, MovieRecord>(&data_query).bind(&library_id_str);
        if let Some(title) = title_filter {
            data_builder = data_builder.bind(format!("%{}%", title.to_lowercase()));
        }
        if let Some(year) = year_filter {
            data_builder = data_builder.bind(year);
        }
        if let Some(monitored) = monitored_filter {
            data_builder = data_builder.bind(bool_to_int(monitored));
        }

        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    /// List all movies for a user (across all libraries)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<MovieRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
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
            WHERE user_id = ?1
            ORDER BY COALESCE(sort_title, title)
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get a movie by ID
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<MovieRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
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
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a movie by TMDB ID within a library
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn get_by_tmdb_id(
        &self,
        library_id: Uuid,
        tmdb_id: i32,
    ) -> Result<Option<MovieRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
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
            WHERE library_id = ?1 AND tmdb_id = ?2
            "#,
        )
        .bind(uuid_to_str(library_id))
        .bind(tmdb_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create a new movie
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, input: CreateMovie) -> Result<MovieRecord> {
        use crate::db::sqlite_helpers::{uuid_to_str, vec_to_json, bool_to_int};
        
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);
        
        sqlx::query(
            r#"
            INSERT INTO movies (
                id, library_id, user_id, title, sort_title, original_title, year,
                tmdb_id, imdb_id, overview, tagline, runtime, genres,
                production_countries, spoken_languages, director, cast_names,
                tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                collection_id, collection_name, collection_poster_url,
                release_date, certification, status, monitored,
                created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                    ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28,
                    datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(uuid_to_str(input.library_id))
        .bind(uuid_to_str(input.user_id))
        .bind(&input.title)
        .bind(&input.sort_title)
        .bind(&input.original_title)
        .bind(input.year)
        .bind(input.tmdb_id)
        .bind(&input.imdb_id)
        .bind(&input.overview)
        .bind(&input.tagline)
        .bind(input.runtime)
        .bind(vec_to_json(&input.genres))
        .bind(vec_to_json(&input.production_countries))
        .bind(vec_to_json(&input.spoken_languages))
        .bind(&input.director)
        .bind(vec_to_json(&input.cast_names))
        .bind(input.tmdb_rating.map(|d| d.to_string()))
        .bind(input.tmdb_vote_count)
        .bind(&input.poster_url)
        .bind(&input.backdrop_url)
        .bind(input.collection_id)
        .bind(&input.collection_name)
        .bind(&input.collection_poster_url)
        .bind(input.release_date.map(|d| d.format("%Y-%m-%d").to_string()))
        .bind(&input.certification)
        .bind(&input.status)
        .bind(bool_to_int(input.monitored))
        .execute(&self.pool)
        .await?;

        self.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to retrieve movie after insert"))
    }

    /// Update a movie
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn update(&self, id: Uuid, input: UpdateMovie) -> Result<Option<MovieRecord>> {
        use crate::db::sqlite_helpers::{uuid_to_str, vec_to_json, bool_to_int};
        
        let id_str = uuid_to_str(id);
        
        // First get current record
        let current = match self.get_by_id(id).await? {
            Some(r) => r,
            None => return Ok(None),
        };
        
        sqlx::query(
            r#"
            UPDATE movies SET
                title = ?2,
                sort_title = ?3,
                original_title = ?4,
                overview = ?5,
                tagline = ?6,
                runtime = ?7,
                genres = ?8,
                director = ?9,
                cast_names = ?10,
                poster_url = ?11,
                backdrop_url = ?12,
                monitored = ?13,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(&id_str)
        .bind(input.title.unwrap_or(current.title))
        .bind(input.sort_title.or(current.sort_title))
        .bind(input.original_title.or(current.original_title))
        .bind(input.overview.or(current.overview))
        .bind(input.tagline.or(current.tagline))
        .bind(input.runtime.or(current.runtime))
        .bind(vec_to_json(&input.genres.unwrap_or(current.genres)))
        .bind(input.director.or(current.director))
        .bind(vec_to_json(&input.cast_names.unwrap_or(current.cast_names)))
        .bind(input.poster_url.or(current.poster_url))
        .bind(input.backdrop_url.or(current.backdrop_url))
        .bind(bool_to_int(input.monitored.unwrap_or(current.monitored)))
        .execute(&self.pool)
        .await?;

        self.get_by_id(id).await
    }

    /// Delete a movie
    #[cfg(feature = "postgres")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM movies WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let result = sqlx::query("DELETE FROM movies WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Link a movie to a media file
    #[cfg(feature = "postgres")]
    pub async fn set_media_file(&self, movie_id: Uuid, media_file_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE movies SET media_file_id = $2, updated_at = NOW() WHERE id = $1")
            .bind(movie_id)
            .bind(media_file_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn set_media_file(&self, movie_id: Uuid, media_file_id: Uuid) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        sqlx::query("UPDATE movies SET media_file_id = ?2, updated_at = datetime('now') WHERE id = ?1")
            .bind(uuid_to_str(movie_id))
            .bind(uuid_to_str(media_file_id))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Clear the media file link from a movie
    #[cfg(feature = "postgres")]
    pub async fn clear_media_file(&self, movie_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE movies SET media_file_id = NULL, updated_at = NOW() WHERE id = $1")
            .bind(movie_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn clear_media_file(&self, movie_id: Uuid) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        sqlx::query("UPDATE movies SET media_file_id = NULL, updated_at = datetime('now') WHERE id = ?1")
            .bind(uuid_to_str(movie_id))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get movie count for a library
    #[cfg(feature = "postgres")]
    pub async fn count_by_library(&self, library_id: Uuid) -> Result<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM movies WHERE library_id = $1")
            .bind(library_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(count)
    }

    #[cfg(feature = "sqlite")]
    pub async fn count_by_library(&self, library_id: Uuid) -> Result<i64> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM movies WHERE library_id = ?1")
            .bind(uuid_to_str(library_id))
            .fetch_one(&self.pool)
            .await?;

        Ok(count)
    }

    /// List movies in a collection
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
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
            WHERE collection_id = ?1
            ORDER BY release_date
            "#,
        )
        .bind(collection_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Search movies by title
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn search(&self, library_id: Uuid, query: &str) -> Result<Vec<MovieRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
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
            WHERE library_id = ?1 AND (
                LOWER(title) LIKE ?2 OR
                LOWER(original_title) LIKE ?2 OR
                LOWER(director) LIKE ?2
            )
            ORDER BY COALESCE(sort_title, title)
            LIMIT 50
            "#,
        )
        .bind(uuid_to_str(library_id))
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
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn find_by_title_in_library(
        &self,
        library_id: Uuid,
        title: &str,
        year: Option<i32>,
    ) -> Result<Option<MovieRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        // Normalize the input title for comparison
        let normalized_title = normalize_title(title);
        let library_id_str = uuid_to_str(library_id);

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
                WHERE library_id = ?1 AND year = ?2 AND (
                    LOWER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(title, '''', ''), ':', ''), '-', ''), '.', ''), '_', '')) = ?3 OR
                    LOWER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(original_title, '''', ''), ':', ''), '-', ''), '.', ''), '_', '')) = ?3
                )
                LIMIT 1
                "#,
            )
            .bind(&library_id_str)
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
                WHERE library_id = ?1 AND year BETWEEN ?2 AND ?3 AND (
                    LOWER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(title, '''', ''), ':', ''), '-', ''), '.', ''), '_', '')) = ?4 OR
                    LOWER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(original_title, '''', ''), ':', ''), '-', ''), '.', ''), '_', '')) = ?4
                )
                ORDER BY ABS(year - ?5)
                LIMIT 1
                "#,
            )
            .bind(&library_id_str)
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
            WHERE library_id = ?1 AND (
                LOWER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(title, '''', ''), ':', ''), '-', ''), '.', ''), '_', '')) = ?2 OR
                LOWER(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(original_title, '''', ''), ':', ''), '-', ''), '.', ''), '_', '')) = ?2
            )
            LIMIT 1
            "#,
        )
        .bind(&library_id_str)
        .bind(&normalized_title)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Find movies in a library that need files (monitored and missing files)
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn list_wanted(&self, library_id: Uuid) -> Result<Vec<MovieRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
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
            WHERE library_id = ?1 AND monitored = 1 AND media_file_id IS NULL
            ORDER BY COALESCE(sort_title, title)
            "#,
        )
        .bind(uuid_to_str(library_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }
}

// normalize_title moved to services/text_utils.rs
// NOTE: The SQL queries in find_by_title_in_library must match the normalization!
