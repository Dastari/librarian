//! Movie database repository

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

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
    // Quality overrides
    pub allowed_resolutions_override: Option<Vec<String>>,
    pub allowed_video_codecs_override: Option<Vec<String>>,
    pub allowed_audio_formats_override: Option<Vec<String>>,
    pub require_hdr_override: Option<bool>,
    pub allowed_hdr_types_override: Option<Vec<String>>,
    pub allowed_sources_override: Option<Vec<String>>,
    pub release_group_blacklist_override: Option<Vec<String>>,
    pub release_group_whitelist_override: Option<Vec<String>>,
    // File status
    pub has_file: bool,
    pub size_bytes: Option<i64>,
    pub path: Option<String>,
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
    pub path: Option<String>,
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
    pub path: Option<String>,
    // Quality overrides
    pub allowed_resolutions_override: Option<Vec<String>>,
    pub allowed_video_codecs_override: Option<Vec<String>>,
    pub allowed_audio_formats_override: Option<Vec<String>>,
    pub require_hdr_override: Option<bool>,
    pub allowed_hdr_types_override: Option<Vec<String>>,
    pub allowed_sources_override: Option<Vec<String>>,
    pub release_group_blacklist_override: Option<Vec<String>>,
    pub release_group_whitelist_override: Option<Vec<String>>,
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
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override,
                   has_file, size_bytes, path, created_at, updated_at
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
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override,
                   has_file, size_bytes, path, created_at, updated_at
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
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override,
                   has_file, size_bytes, path, created_at, updated_at
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
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override,
                   has_file, size_bytes, path, created_at, updated_at
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
                release_date, certification, status, monitored, path
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                    $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28)
            RETURNING id, library_id, user_id, title, sort_title, original_title, year,
                      tmdb_id, imdb_id, overview, tagline, runtime, genres,
                      production_countries, spoken_languages, director, cast_names,
                      tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                      collection_id, collection_name, collection_poster_url,
                      release_date, certification, status, monitored,
                      allowed_resolutions_override, allowed_video_codecs_override,
                      allowed_audio_formats_override, require_hdr_override,
                      allowed_hdr_types_override, allowed_sources_override,
                      release_group_blacklist_override, release_group_whitelist_override,
                      has_file, size_bytes, path, created_at, updated_at
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
        .bind(&input.path)
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
                path = COALESCE($14, path),
                allowed_resolutions_override = COALESCE($15, allowed_resolutions_override),
                allowed_video_codecs_override = COALESCE($16, allowed_video_codecs_override),
                allowed_audio_formats_override = COALESCE($17, allowed_audio_formats_override),
                require_hdr_override = COALESCE($18, require_hdr_override),
                allowed_hdr_types_override = COALESCE($19, allowed_hdr_types_override),
                allowed_sources_override = COALESCE($20, allowed_sources_override),
                release_group_blacklist_override = COALESCE($21, release_group_blacklist_override),
                release_group_whitelist_override = COALESCE($22, release_group_whitelist_override),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, library_id, user_id, title, sort_title, original_title, year,
                      tmdb_id, imdb_id, overview, tagline, runtime, genres,
                      production_countries, spoken_languages, director, cast_names,
                      tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                      collection_id, collection_name, collection_poster_url,
                      release_date, certification, status, monitored,
                      allowed_resolutions_override, allowed_video_codecs_override,
                      allowed_audio_formats_override, require_hdr_override,
                      allowed_hdr_types_override, allowed_sources_override,
                      release_group_blacklist_override, release_group_whitelist_override,
                      has_file, size_bytes, path, created_at, updated_at
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
        .bind(&input.path)
        .bind(&input.allowed_resolutions_override)
        .bind(&input.allowed_video_codecs_override)
        .bind(&input.allowed_audio_formats_override)
        .bind(input.require_hdr_override)
        .bind(&input.allowed_hdr_types_override)
        .bind(&input.allowed_sources_override)
        .bind(&input.release_group_blacklist_override)
        .bind(&input.release_group_whitelist_override)
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

    /// Update movie file status
    pub async fn update_file_status(
        &self,
        id: Uuid,
        has_file: bool,
        size_bytes: Option<i64>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE movies 
            SET has_file = $2, size_bytes = $3, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(has_file)
        .bind(size_bytes)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update movie has_file flag
    pub async fn update_has_file(&self, id: Uuid, has_file: bool) -> Result<()> {
        sqlx::query("UPDATE movies SET has_file = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(has_file)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get movie count for a library
    pub async fn count_by_library(&self, library_id: Uuid) -> Result<i64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM movies WHERE library_id = $1")
                .bind(library_id)
                .fetch_one(&self.pool)
                .await?;

        Ok(count)
    }

    /// Get total size for a library's movies
    pub async fn total_size_by_library(&self, library_id: Uuid) -> Result<i64> {
        let size: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(size_bytes), 0)::BIGINT FROM movies WHERE library_id = $1",
        )
        .bind(library_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(size)
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
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override,
                   has_file, size_bytes, path, created_at, updated_at
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
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override,
                   has_file, size_bytes, path, created_at, updated_at
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
                       allowed_resolutions_override, allowed_video_codecs_override,
                       allowed_audio_formats_override, require_hdr_override,
                       allowed_hdr_types_override, allowed_sources_override,
                       release_group_blacklist_override, release_group_whitelist_override,
                       has_file, size_bytes, path, created_at, updated_at
                FROM movies
                WHERE library_id = $1 AND year = $2 AND (
                    LOWER(REPLACE(REPLACE(title, '''', ''), ':', '')) = $3 OR
                    LOWER(REPLACE(REPLACE(original_title, '''', ''), ':', '')) = $3
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
                       allowed_resolutions_override, allowed_video_codecs_override,
                       allowed_audio_formats_override, require_hdr_override,
                       allowed_hdr_types_override, allowed_sources_override,
                       release_group_blacklist_override, release_group_whitelist_override,
                       has_file, size_bytes, path, created_at, updated_at
                FROM movies
                WHERE library_id = $1 AND year BETWEEN $2 AND $3 AND (
                    LOWER(REPLACE(REPLACE(title, '''', ''), ':', '')) = $4 OR
                    LOWER(REPLACE(REPLACE(original_title, '''', ''), ':', '')) = $4
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
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override,
                   has_file, size_bytes, path, created_at, updated_at
            FROM movies
            WHERE library_id = $1 AND (
                LOWER(REPLACE(REPLACE(title, '''', ''), ':', '')) = $2 OR
                LOWER(REPLACE(REPLACE(original_title, '''', ''), ':', '')) = $2
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
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override,
                   has_file, size_bytes, path, created_at, updated_at
            FROM movies
            WHERE library_id = $1 AND monitored = true AND has_file = false
            ORDER BY COALESCE(sort_title, title)
            "#,
        )
        .bind(library_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }
}

/// Normalize a title for comparison
/// - Lowercase
/// - Remove special characters (apostrophes, colons)
/// - Collapse whitespace
fn normalize_title(title: &str) -> String {
    title
        .to_lowercase()
        .replace(['\'', '\u{2019}', ':', '-', '.', '_'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
