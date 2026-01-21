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
    pub path: Option<String>,
    pub auto_download_override: Option<bool>,
    pub backfill_existing: bool,
    pub organize_files_override: Option<bool>,
    pub rename_style_override: Option<String>,
    pub auto_hunt_override: Option<bool>,
    pub episode_count: Option<i32>,
    pub episode_file_count: Option<i32>,
    pub size_bytes: Option<i64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    // Quality override fields (NULL = inherit from library)
    pub allowed_resolutions_override: Option<Vec<String>>,
    pub allowed_video_codecs_override: Option<Vec<String>>,
    pub allowed_audio_formats_override: Option<Vec<String>>,
    pub require_hdr_override: Option<bool>,
    pub allowed_hdr_types_override: Option<Vec<String>>,
    pub allowed_sources_override: Option<Vec<String>>,
    pub release_group_blacklist_override: Option<Vec<String>>,
    pub release_group_whitelist_override: Option<Vec<String>>,
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
    pub path: Option<String>,
    pub auto_download_override: Option<bool>,
    pub backfill_existing: bool,
    pub organize_files_override: Option<bool>,
    pub rename_style_override: Option<String>,
    pub auto_hunt_override: Option<bool>,
    // Quality override fields
    pub allowed_resolutions_override: Option<Vec<String>>,
    pub allowed_video_codecs_override: Option<Vec<String>>,
    pub allowed_audio_formats_override: Option<Vec<String>>,
    pub require_hdr_override: Option<bool>,
    pub allowed_hdr_types_override: Option<Vec<String>>,
    pub allowed_sources_override: Option<Vec<String>>,
    pub release_group_blacklist_override: Option<Vec<String>>,
    pub release_group_whitelist_override: Option<Vec<String>>,
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
    pub path: Option<String>,
    pub auto_download_override: Option<Option<bool>>,
    pub backfill_existing: Option<bool>,
    pub organize_files_override: Option<Option<bool>>,
    pub rename_style_override: Option<Option<String>>,
    pub auto_hunt_override: Option<Option<bool>>,
    // Quality override fields (Option<Option<...>> for nullable override)
    pub allowed_resolutions_override: Option<Option<Vec<String>>>,
    pub allowed_video_codecs_override: Option<Option<Vec<String>>>,
    pub allowed_audio_formats_override: Option<Option<Vec<String>>>,
    pub require_hdr_override: Option<Option<bool>>,
    pub allowed_hdr_types_override: Option<Option<Vec<String>>>,
    pub allowed_sources_override: Option<Option<Vec<String>>>,
    pub release_group_blacklist_override: Option<Option<Vec<String>>>,
    pub release_group_whitelist_override: Option<Option<Vec<String>>>,
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
                   monitor_type, path,
                   auto_download_override, backfill_existing,
                   organize_files_override, rename_style_override, auto_hunt_override,
                   episode_count, episode_file_count, size_bytes, created_at, updated_at,
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override
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

    /// List TV shows in a library with pagination and filtering
    ///
    /// Returns (records, total_count)
    #[allow(clippy::too_many_arguments)]
    pub async fn list_by_library_paginated(
        &self,
        library_id: Uuid,
        offset: i64,
        limit: i64,
        name_filter: Option<&str>,
        year_filter: Option<i32>,
        monitored_filter: Option<bool>,
        status_filter: Option<&str>,
        sort_column: &str,
        sort_asc: bool,
    ) -> Result<(Vec<TvShowRecord>, i64)> {
        // Build dynamic WHERE clause conditions
        let mut conditions = vec!["library_id = $1".to_string()];
        let mut param_idx = 2;

        if name_filter.is_some() {
            conditions.push(format!("LOWER(name) LIKE ${}", param_idx));
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
        if status_filter.is_some() {
            conditions.push(format!("LOWER(status) = ${}", param_idx));
        }

        let where_clause = conditions.join(" AND ");

        // Validate sort column
        let valid_sort_columns = ["name", "sort_name", "year", "created_at"];
        let sort_col = if valid_sort_columns.contains(&sort_column) {
            sort_column
        } else {
            "sort_name"
        };
        let order_dir = if sort_asc { "ASC" } else { "DESC" };
        let order_clause = format!(
            "ORDER BY COALESCE({}, name) {} NULLS LAST",
            sort_col, order_dir
        );

        // Build count query
        let count_query = format!("SELECT COUNT(*) FROM tv_shows WHERE {}", where_clause);

        // Build data query
        let data_query = format!(
            r#"
            SELECT id, library_id, user_id, name, sort_name, year, status,
                   tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                   runtime, genres, poster_url, backdrop_url, monitored,
                   monitor_type, path,
                   auto_download_override, backfill_existing,
                   organize_files_override, rename_style_override, auto_hunt_override,
                   episode_count, episode_file_count, size_bytes, created_at, updated_at,
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override
            FROM tv_shows
            WHERE {}
            {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, order_clause, limit, offset
        );

        // Execute count query
        let mut count_builder = sqlx::query_scalar::<_, i64>(&count_query).bind(library_id);
        if let Some(name) = name_filter {
            count_builder = count_builder.bind(format!("%{}%", name.to_lowercase()));
        }
        if let Some(year) = year_filter {
            count_builder = count_builder.bind(year);
        }
        if let Some(monitored) = monitored_filter {
            count_builder = count_builder.bind(monitored);
        }
        if let Some(status) = status_filter {
            count_builder = count_builder.bind(status.to_lowercase());
        }

        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        // Execute data query
        let mut data_builder = sqlx::query_as::<_, TvShowRecord>(&data_query).bind(library_id);
        if let Some(name) = name_filter {
            data_builder = data_builder.bind(format!("%{}%", name.to_lowercase()));
        }
        if let Some(year) = year_filter {
            data_builder = data_builder.bind(year);
        }
        if let Some(monitored) = monitored_filter {
            data_builder = data_builder.bind(monitored);
        }
        if let Some(status) = status_filter {
            data_builder = data_builder.bind(status.to_lowercase());
        }

        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    /// Get all TV shows for a user (across all libraries)
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<TvShowRecord>> {
        let records = sqlx::query_as::<_, TvShowRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, year, status,
                   tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                   runtime, genres, poster_url, backdrop_url, monitored,
                   monitor_type, path,
                   auto_download_override, backfill_existing,
                   organize_files_override, rename_style_override, auto_hunt_override,
                   episode_count, episode_file_count, size_bytes, created_at, updated_at,
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override
            FROM tv_shows
            WHERE user_id = $1
            ORDER BY name
            "#,
        )
        .bind(user_id)
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
                   monitor_type, path,
                   auto_download_override, backfill_existing,
                   organize_files_override, rename_style_override, auto_hunt_override,
                   episode_count, episode_file_count, size_bytes, created_at, updated_at,
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override
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
                   monitor_type, path,
                   auto_download_override, backfill_existing,
                   organize_files_override, rename_style_override, auto_hunt_override,
                   episode_count, episode_file_count, size_bytes, created_at, updated_at,
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override
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
    pub async fn get_by_tvmaze_id(
        &self,
        library_id: Uuid,
        tvmaze_id: i32,
    ) -> Result<Option<TvShowRecord>> {
        let record = sqlx::query_as::<_, TvShowRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, year, status,
                   tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                   runtime, genres, poster_url, backdrop_url, monitored,
                   monitor_type, path,
                   auto_download_override, backfill_existing,
                   organize_files_override, rename_style_override, auto_hunt_override,
                   episode_count, episode_file_count, size_bytes, created_at, updated_at,
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override
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

    /// Find a TV show by name in a library (case-insensitive fuzzy match)
    ///
    /// Handles common naming variations:
    /// - Case differences: "Star Trek" vs "STAR TREK"
    /// - Punctuation differences: "Star Trek: Deep Space Nine" vs "Star Trek Deep Space Nine"
    /// - Separator differences: "Doctor.Who" vs "Doctor Who" vs "Doctor_Who"
    /// - Article prefix: "The Office" vs "Office"
    pub async fn find_by_name_in_library(
        &self,
        library_id: Uuid,
        name: &str,
    ) -> Result<Option<TvShowRecord>> {
        // Normalize search name: remove punctuation, normalize spaces
        let normalized_search = Self::normalize_show_name(name);

        // First try exact match (case-insensitive)
        let record = sqlx::query_as::<_, TvShowRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, year, status,
                   tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                   runtime, genres, poster_url, backdrop_url, monitored,
                   monitor_type, path,
                   auto_download_override, backfill_existing,
                   organize_files_override, rename_style_override, auto_hunt_override,
                   episode_count, episode_file_count, size_bytes, created_at, updated_at,
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override
            FROM tv_shows
            WHERE library_id = $1 AND LOWER(name) = LOWER($2)
            LIMIT 1
            "#,
        )
        .bind(library_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;

        if record.is_some() {
            return Ok(record);
        }

        // Try normalized match - compare normalized versions of both names
        // This handles "Star Trek: Deep Space Nine" vs "Star Trek Deep Space Nine"
        let record = sqlx::query_as::<_, TvShowRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, year, status,
                   tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                   runtime, genres, poster_url, backdrop_url, monitored,
                   monitor_type, path,
                   auto_download_override, backfill_existing,
                   organize_files_override, rename_style_override, auto_hunt_override,
                   episode_count, episode_file_count, size_bytes, created_at, updated_at,
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override
            FROM tv_shows
            WHERE library_id = $1 
              AND LOWER(REGEXP_REPLACE(REGEXP_REPLACE(name, '[^a-zA-Z0-9\s]', '', 'g'), '\s+', ' ', 'g')) = LOWER($2)
            LIMIT 1
            "#,
        )
        .bind(library_id)
        .bind(&normalized_search)
        .fetch_optional(&self.pool)
        .await?;

        if record.is_some() {
            return Ok(record);
        }

        // Try without "The " prefix on both sides
        let normalized_no_the = normalized_search
            .trim_start_matches("the ")
            .trim();

        let record = sqlx::query_as::<_, TvShowRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, year, status,
                   tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                   runtime, genres, poster_url, backdrop_url, monitored,
                   monitor_type, path,
                   auto_download_override, backfill_existing,
                   organize_files_override, rename_style_override, auto_hunt_override,
                   episode_count, episode_file_count, size_bytes, created_at, updated_at,
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override
            FROM tv_shows
            WHERE library_id = $1 
              AND LOWER(REGEXP_REPLACE(REGEXP_REPLACE(
                  REGEXP_REPLACE(name, '^[Tt]he\s+', '', 'g'),
                  '[^a-zA-Z0-9\s]', '', 'g'), '\s+', ' ', 'g')) = LOWER($2)
            LIMIT 1
            "#,
        )
        .bind(library_id)
        .bind(normalized_no_the)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Normalize a show name for matching
    /// Removes punctuation, normalizes whitespace, and lowercases
    fn normalize_show_name(name: &str) -> String {
        // Remove non-alphanumeric characters except spaces
        let cleaned: String = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c.is_whitespace() {
                    c
                } else {
                    ' '
                }
            })
            .collect();

        // Normalize whitespace and lowercase
        cleaned
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    }

    /// Create a new TV show
    pub async fn create(&self, input: CreateTvShow) -> Result<TvShowRecord> {
        let record = sqlx::query_as::<_, TvShowRecord>(
            r#"
            INSERT INTO tv_shows (
                library_id, user_id, name, sort_name, year, status,
                tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                runtime, genres, poster_url, backdrop_url, monitored,
                monitor_type, path,
                auto_download_override, backfill_existing,
                organize_files_override, rename_style_override, auto_hunt_override,
                allowed_resolutions_override, allowed_video_codecs_override,
                allowed_audio_formats_override, require_hdr_override,
                allowed_hdr_types_override, allowed_sources_override,
                release_group_blacklist_override, release_group_whitelist_override
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25,
                    $26, $27, $28, $29, $30, $31, $32)
            RETURNING id, library_id, user_id, name, sort_name, year, status,
                      tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                      runtime, genres, poster_url, backdrop_url, monitored,
                      monitor_type, path,
                      auto_download_override, backfill_existing,
                      organize_files_override, rename_style_override, auto_hunt_override,
                      episode_count, episode_file_count, size_bytes, created_at, updated_at,
                      allowed_resolutions_override, allowed_video_codecs_override,
                      allowed_audio_formats_override, require_hdr_override,
                      allowed_hdr_types_override, allowed_sources_override,
                      release_group_blacklist_override, release_group_whitelist_override
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
        .bind(&input.path)
        .bind(input.auto_download_override)
        .bind(input.backfill_existing)
        .bind(input.organize_files_override)
        .bind(&input.rename_style_override)
        .bind(input.auto_hunt_override)
        .bind(&input.allowed_resolutions_override)
        .bind(&input.allowed_video_codecs_override)
        .bind(&input.allowed_audio_formats_override)
        .bind(input.require_hdr_override)
        .bind(&input.allowed_hdr_types_override)
        .bind(&input.allowed_sources_override)
        .bind(&input.release_group_blacklist_override)
        .bind(&input.release_group_whitelist_override)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update a TV show
    ///
    /// For nullable override fields (Option<Option<T>>):
    /// - None (outer) = don't update the field
    /// - Some(None) = set the field to NULL (inherit from library)
    /// - Some(Some(value)) = set the field to the value
    pub async fn update(&self, id: Uuid, input: UpdateTvShow) -> Result<Option<TvShowRecord>> {
        // For Option<Option<T>> fields, we need to distinguish between:
        // - "don't update" (outer None)
        // - "set to NULL" (Some(None))
        // We use CASE expressions with boolean flags to handle this.

        // Extract the "should update" flags and flattened values for nullable override fields
        let update_auto_download = input.auto_download_override.is_some();
        let auto_download_value = input.auto_download_override.flatten();

        let update_organize_files = input.organize_files_override.is_some();
        let organize_files_value = input.organize_files_override.flatten();

        let update_rename_style = input.rename_style_override.is_some();
        let rename_style_value = input.rename_style_override.flatten();

        let update_auto_hunt = input.auto_hunt_override.is_some();
        let auto_hunt_value = input.auto_hunt_override.flatten();

        let update_resolutions = input.allowed_resolutions_override.is_some();
        let resolutions_value = input.allowed_resolutions_override.flatten();

        let update_codecs = input.allowed_video_codecs_override.is_some();
        let codecs_value = input.allowed_video_codecs_override.flatten();

        let update_audio = input.allowed_audio_formats_override.is_some();
        let audio_value = input.allowed_audio_formats_override.flatten();

        let update_require_hdr = input.require_hdr_override.is_some();
        let require_hdr_value = input.require_hdr_override.flatten();

        let update_hdr_types = input.allowed_hdr_types_override.is_some();
        let hdr_types_value = input.allowed_hdr_types_override.flatten();

        let update_sources = input.allowed_sources_override.is_some();
        let sources_value = input.allowed_sources_override.flatten();

        let update_blacklist = input.release_group_blacklist_override.is_some();
        let blacklist_value = input.release_group_blacklist_override.flatten();

        let update_whitelist = input.release_group_whitelist_override.is_some();
        let whitelist_value = input.release_group_whitelist_override.flatten();

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
                path = COALESCE($14, path),
                -- Nullable override fields use CASE to allow setting to NULL
                auto_download_override = CASE WHEN $15 THEN $16 ELSE auto_download_override END,
                backfill_existing = COALESCE($17, backfill_existing),
                organize_files_override = CASE WHEN $18 THEN $19 ELSE organize_files_override END,
                rename_style_override = CASE WHEN $20 THEN $21 ELSE rename_style_override END,
                auto_hunt_override = CASE WHEN $22 THEN $23 ELSE auto_hunt_override END,
                allowed_resolutions_override = CASE WHEN $24 THEN $25 ELSE allowed_resolutions_override END,
                allowed_video_codecs_override = CASE WHEN $26 THEN $27 ELSE allowed_video_codecs_override END,
                allowed_audio_formats_override = CASE WHEN $28 THEN $29 ELSE allowed_audio_formats_override END,
                require_hdr_override = CASE WHEN $30 THEN $31 ELSE require_hdr_override END,
                allowed_hdr_types_override = CASE WHEN $32 THEN $33 ELSE allowed_hdr_types_override END,
                allowed_sources_override = CASE WHEN $34 THEN $35 ELSE allowed_sources_override END,
                release_group_blacklist_override = CASE WHEN $36 THEN $37 ELSE release_group_blacklist_override END,
                release_group_whitelist_override = CASE WHEN $38 THEN $39 ELSE release_group_whitelist_override END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, library_id, user_id, name, sort_name, year, status,
                      tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                      runtime, genres, poster_url, backdrop_url, monitored,
                      monitor_type, path,
                      auto_download_override, backfill_existing,
                      organize_files_override, rename_style_override, auto_hunt_override,
                      episode_count, episode_file_count, size_bytes, created_at, updated_at,
                      allowed_resolutions_override, allowed_video_codecs_override,
                      allowed_audio_formats_override, require_hdr_override,
                      allowed_hdr_types_override, allowed_sources_override,
                      release_group_blacklist_override, release_group_whitelist_override
            "#,
        )
        .bind(id)                           // $1
        .bind(&input.name)                  // $2
        .bind(&input.sort_name)             // $3
        .bind(input.year)                   // $4
        .bind(&input.status)                // $5
        .bind(&input.overview)              // $6
        .bind(&input.network)               // $7
        .bind(input.runtime)                // $8
        .bind(&input.genres)                // $9
        .bind(&input.poster_url)            // $10
        .bind(&input.backdrop_url)          // $11
        .bind(input.monitored)              // $12
        .bind(&input.monitor_type)          // $13
        .bind(&input.path)                  // $14
        .bind(update_auto_download)         // $15 - flag
        .bind(auto_download_value)          // $16 - value
        .bind(input.backfill_existing)      // $17
        .bind(update_organize_files)        // $18 - flag
        .bind(organize_files_value)         // $19 - value
        .bind(update_rename_style)          // $20 - flag
        .bind(&rename_style_value)          // $21 - value
        .bind(update_auto_hunt)             // $22 - flag
        .bind(auto_hunt_value)              // $23 - value
        .bind(update_resolutions)           // $24 - flag
        .bind(&resolutions_value)           // $25 - value
        .bind(update_codecs)                // $26 - flag
        .bind(&codecs_value)                // $27 - value
        .bind(update_audio)                 // $28 - flag
        .bind(&audio_value)                 // $29 - value
        .bind(update_require_hdr)           // $30 - flag
        .bind(require_hdr_value)            // $31 - value
        .bind(update_hdr_types)             // $32 - flag
        .bind(&hdr_types_value)             // $33 - value
        .bind(update_sources)               // $34 - flag
        .bind(&sources_value)               // $35 - value
        .bind(update_blacklist)             // $36 - flag
        .bind(&blacklist_value)             // $37 - value
        .bind(update_whitelist)             // $38 - flag
        .bind(&whitelist_value)             // $39 - value
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
