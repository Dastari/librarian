//! TV Show database repository

use anyhow::Result;
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

#[cfg(feature = "sqlite")]
use crate::db::sqlite_helpers::{
    bool_to_int, int_to_bool, json_to_vec, str_to_datetime, str_to_uuid, uuid_to_str, vec_to_json,
};

/// TV Show record from database
#[derive(Debug, Clone)]
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
    /// When true, auto-hunt searches for individual episodes instead of season packs
    /// Set after a partial season download completes
    pub hunt_individual_items: bool,
}


#[cfg(feature = "sqlite")]
impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for TvShowRecord {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> std::result::Result<Self, sqlx::Error> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let library_id_str: String = row.try_get("library_id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;

        let monitored_int: i32 = row.try_get("monitored")?;
        let backfill_existing_int: i32 = row.try_get("backfill_existing")?;
        let auto_download_override_int: Option<i32> = row.try_get("auto_download_override")?;
        let organize_files_override_int: Option<i32> = row.try_get("organize_files_override")?;
        let auto_hunt_override_int: Option<i32> = row.try_get("auto_hunt_override")?;
        let require_hdr_override_int: Option<i32> = row.try_get("require_hdr_override")?;

        let genres_json: String = row.try_get("genres")?;
        let allowed_resolutions_json: Option<String> =
            row.try_get("allowed_resolutions_override")?;
        let allowed_codecs_json: Option<String> =
            row.try_get("allowed_video_codecs_override")?;
        let allowed_audio_json: Option<String> =
            row.try_get("allowed_audio_formats_override")?;
        let allowed_hdr_types_json: Option<String> =
            row.try_get("allowed_hdr_types_override")?;
        let allowed_sources_json: Option<String> = row.try_get("allowed_sources_override")?;
        let blacklist_json: Option<String> =
            row.try_get("release_group_blacklist_override")?;
        let whitelist_json: Option<String> =
            row.try_get("release_group_whitelist_override")?;

        Ok(TvShowRecord {
            id: str_to_uuid(&id_str)
                .map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))))?,
            library_id: str_to_uuid(&library_id_str)
                .map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))))?,
            user_id: str_to_uuid(&user_id_str)
                .map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))))?,
            name: row.try_get("name")?,
            sort_name: row.try_get("sort_name")?,
            year: row.try_get("year")?,
            status: row.try_get("status")?,
            tvmaze_id: row.try_get("tvmaze_id")?,
            tmdb_id: row.try_get("tmdb_id")?,
            tvdb_id: row.try_get("tvdb_id")?,
            imdb_id: row.try_get("imdb_id")?,
            overview: row.try_get("overview")?,
            network: row.try_get("network")?,
            runtime: row.try_get("runtime")?,
            genres: json_to_vec(&genres_json),
            poster_url: row.try_get("poster_url")?,
            backdrop_url: row.try_get("backdrop_url")?,
            monitored: int_to_bool(monitored_int),
            monitor_type: row.try_get("monitor_type")?,
            path: row.try_get("path")?,
            auto_download_override: auto_download_override_int.map(int_to_bool),
            backfill_existing: int_to_bool(backfill_existing_int),
            organize_files_override: organize_files_override_int.map(int_to_bool),
            rename_style_override: row.try_get("rename_style_override")?,
            auto_hunt_override: auto_hunt_override_int.map(int_to_bool),
            episode_count: row.try_get("episode_count")?,
            episode_file_count: row.try_get("episode_file_count")?,
            size_bytes: row.try_get("size_bytes")?,
            created_at: str_to_datetime(&created_at_str)
                .map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))))?,
            updated_at: str_to_datetime(&updated_at_str)
                .map_err(|e| sqlx::Error::Decode(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))))?,
            allowed_resolutions_override: allowed_resolutions_json
                .as_deref()
                .map(json_to_vec),
            allowed_video_codecs_override: allowed_codecs_json.as_deref().map(json_to_vec),
            allowed_audio_formats_override: allowed_audio_json.as_deref().map(json_to_vec),
            require_hdr_override: require_hdr_override_int.map(int_to_bool),
            allowed_hdr_types_override: allowed_hdr_types_json.as_deref().map(json_to_vec),
            allowed_sources_override: allowed_sources_json.as_deref().map(json_to_vec),
            release_group_blacklist_override: blacklist_json.as_deref().map(json_to_vec),
            release_group_whitelist_override: whitelist_json.as_deref().map(json_to_vec),
            hunt_individual_items: {
                let v: i32 = row.try_get("hunt_individual_items").unwrap_or(0);
                v != 0
            },
        })
    }
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
    pool: DbPool,
}

impl TvShowRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get all TV shows for a library

    #[cfg(feature = "sqlite")]
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
            WHERE library_id = ?1
            ORDER BY COALESCE(sort_name, name)
            "#,
        )
        .bind(uuid_to_str(library_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// List TV shows in a library with pagination and filtering
    ///
    /// Returns (records, total_count)
    #[allow(clippy::too_many_arguments)]

    #[allow(clippy::too_many_arguments)]
    #[cfg(feature = "sqlite")]
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
        let mut conditions = vec!["library_id = ?1".to_string()];
        let mut param_idx = 2;

        if name_filter.is_some() {
            conditions.push(format!("LOWER(name) LIKE ?{}", param_idx));
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
        if status_filter.is_some() {
            conditions.push(format!("LOWER(status) = ?{}", param_idx));
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
        // SQLite doesn't have NULLS LAST, use CASE expression
        let order_clause = format!(
            "ORDER BY CASE WHEN {} IS NULL THEN 1 ELSE 0 END, COALESCE({}, name) {}",
            sort_col, sort_col, order_dir
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

        let library_id_str = uuid_to_str(library_id);

        // Execute count query
        let mut count_builder =
            sqlx::query_scalar::<_, i64>(&count_query).bind(library_id_str.clone());
        if let Some(name) = name_filter {
            count_builder = count_builder.bind(format!("%{}%", name.to_lowercase()));
        }
        if let Some(year) = year_filter {
            count_builder = count_builder.bind(year);
        }
        if let Some(monitored) = monitored_filter {
            count_builder = count_builder.bind(bool_to_int(monitored));
        }
        if let Some(status) = status_filter {
            count_builder = count_builder.bind(status.to_lowercase());
        }

        let total: i64 = count_builder.fetch_one(&self.pool).await?;

        // Execute data query
        let mut data_builder =
            sqlx::query_as::<_, TvShowRecord>(&data_query).bind(library_id_str);
        if let Some(name) = name_filter {
            data_builder = data_builder.bind(format!("%{}%", name.to_lowercase()));
        }
        if let Some(year) = year_filter {
            data_builder = data_builder.bind(year);
        }
        if let Some(monitored) = monitored_filter {
            data_builder = data_builder.bind(bool_to_int(monitored));
        }
        if let Some(status) = status_filter {
            data_builder = data_builder.bind(status.to_lowercase());
        }

        let records = data_builder.fetch_all(&self.pool).await?;

        Ok((records, total))
    }

    /// Get all TV shows for a user (across all libraries)

    #[cfg(feature = "sqlite")]
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
            WHERE user_id = ?1
            ORDER BY name
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get all monitored TV shows for a user

    #[cfg(feature = "sqlite")]
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
            WHERE user_id = ?1 AND monitored = 1
            ORDER BY name
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get a TV show by ID

    #[cfg(feature = "sqlite")]
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
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a TV show by TVMaze ID in a library

    #[cfg(feature = "sqlite")]
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
            WHERE library_id = ?1 AND tvmaze_id = ?2
            "#,
        )
        .bind(uuid_to_str(library_id))
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

    #[cfg(feature = "sqlite")]
    pub async fn find_by_name_in_library(
        &self,
        library_id: Uuid,
        name: &str,
    ) -> Result<Option<TvShowRecord>> {
        // Normalize search name: remove punctuation, normalize spaces
        let normalized_search = Self::normalize_show_name(name);
        let normalized_no_the = normalized_search.trim_start_matches("the ").trim();

        // SQLite doesn't have REGEXP_REPLACE, so we fetch all shows and match in Rust
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
            WHERE library_id = ?1
            "#,
        )
        .bind(uuid_to_str(library_id))
        .fetch_all(&self.pool)
        .await?;

        // First try exact match (case-insensitive)
        if let Some(record) = records
            .iter()
            .find(|r| r.name.to_lowercase() == name.to_lowercase())
        {
            return Ok(Some(record.clone()));
        }

        // Try normalized match
        if let Some(record) = records
            .iter()
            .find(|r| Self::normalize_show_name(&r.name) == normalized_search)
        {
            return Ok(Some(record.clone()));
        }

        // Try without "The " prefix on both sides
        if let Some(record) = records.iter().find(|r| {
            let db_normalized = Self::normalize_show_name(&r.name);
            let db_no_the = db_normalized.trim_start_matches("the ").trim();
            db_no_the == normalized_no_the
        }) {
            return Ok(Some(record.clone()));
        }

        Ok(None)
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

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, input: CreateTvShow) -> Result<TvShowRecord> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            r#"
            INSERT INTO tv_shows (
                id, library_id, user_id, name, sort_name, year, status,
                tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                runtime, genres, poster_url, backdrop_url, monitored,
                monitor_type, path,
                auto_download_override, backfill_existing,
                organize_files_override, rename_style_override, auto_hunt_override,
                allowed_resolutions_override, allowed_video_codecs_override,
                allowed_audio_formats_override, require_hdr_override,
                allowed_hdr_types_override, allowed_sources_override,
                release_group_blacklist_override, release_group_whitelist_override,
                created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25,
                    ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35)
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(uuid_to_str(input.library_id))
        .bind(uuid_to_str(input.user_id))
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
        .bind(vec_to_json(&input.genres))
        .bind(&input.poster_url)
        .bind(&input.backdrop_url)
        .bind(bool_to_int(input.monitored))
        .bind(&input.monitor_type)
        .bind(&input.path)
        .bind(input.auto_download_override.map(bool_to_int))
        .bind(bool_to_int(input.backfill_existing))
        .bind(input.organize_files_override.map(bool_to_int))
        .bind(&input.rename_style_override)
        .bind(input.auto_hunt_override.map(bool_to_int))
        .bind(input.allowed_resolutions_override.as_ref().map(|v| vec_to_json(v)))
        .bind(input.allowed_video_codecs_override.as_ref().map(|v| vec_to_json(v)))
        .bind(input.allowed_audio_formats_override.as_ref().map(|v| vec_to_json(v)))
        .bind(input.require_hdr_override.map(bool_to_int))
        .bind(input.allowed_hdr_types_override.as_ref().map(|v| vec_to_json(v)))
        .bind(input.allowed_sources_override.as_ref().map(|v| vec_to_json(v)))
        .bind(input.release_group_blacklist_override.as_ref().map(|v| vec_to_json(v)))
        .bind(input.release_group_whitelist_override.as_ref().map(|v| vec_to_json(v)))
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        // Fetch the created record
        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to fetch created TV show"))
    }

    /// Update a TV show
    ///
    /// For nullable override fields (Option<Option<T>>):
    /// - None (outer) = don't update the field
    /// - Some(None) = set the field to NULL (inherit from library)
    /// - Some(Some(value)) = set the field to the value

    #[cfg(feature = "sqlite")]
    pub async fn update(&self, id: Uuid, input: UpdateTvShow) -> Result<Option<TvShowRecord>> {
        // First check if the record exists
        let existing = self.get_by_id(id).await?;
        if existing.is_none() {
            return Ok(None);
        }
        let existing = existing.unwrap();

        // Build the update dynamically based on what's provided
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // For Option<Option<T>> fields, we need to distinguish between:
        // - "don't update" (outer None) -> use existing value
        // - "set to NULL" (Some(None)) -> set to NULL
        // - "set value" (Some(Some(value))) -> set to value

        let name = input.name.as_ref().unwrap_or(&existing.name);
        let sort_name = input.sort_name.clone().or(existing.sort_name.clone());
        let year = input.year.or(existing.year);
        let status = input.status.as_ref().unwrap_or(&existing.status);
        let overview = input.overview.clone().or(existing.overview.clone());
        let network = input.network.clone().or(existing.network.clone());
        let runtime = input.runtime.or(existing.runtime);
        let genres = input.genres.clone().unwrap_or(existing.genres.clone());
        let poster_url = input.poster_url.clone().or(existing.poster_url.clone());
        let backdrop_url = input.backdrop_url.clone().or(existing.backdrop_url.clone());
        let monitored = input.monitored.unwrap_or(existing.monitored);
        let monitor_type = input.monitor_type.as_ref().unwrap_or(&existing.monitor_type);
        let path = input.path.clone().or(existing.path.clone());
        let backfill_existing = input.backfill_existing.unwrap_or(existing.backfill_existing);

        // Handle Option<Option<T>> fields
        let auto_download_override = match &input.auto_download_override {
            None => existing.auto_download_override,
            Some(inner) => *inner,
        };
        let organize_files_override = match &input.organize_files_override {
            None => existing.organize_files_override,
            Some(inner) => *inner,
        };
        let rename_style_override = match &input.rename_style_override {
            None => existing.rename_style_override.clone(),
            Some(inner) => inner.clone(),
        };
        let auto_hunt_override = match &input.auto_hunt_override {
            None => existing.auto_hunt_override,
            Some(inner) => *inner,
        };
        let allowed_resolutions_override = match &input.allowed_resolutions_override {
            None => existing.allowed_resolutions_override.clone(),
            Some(inner) => inner.clone(),
        };
        let allowed_video_codecs_override = match &input.allowed_video_codecs_override {
            None => existing.allowed_video_codecs_override.clone(),
            Some(inner) => inner.clone(),
        };
        let allowed_audio_formats_override = match &input.allowed_audio_formats_override {
            None => existing.allowed_audio_formats_override.clone(),
            Some(inner) => inner.clone(),
        };
        let require_hdr_override = match &input.require_hdr_override {
            None => existing.require_hdr_override,
            Some(inner) => *inner,
        };
        let allowed_hdr_types_override = match &input.allowed_hdr_types_override {
            None => existing.allowed_hdr_types_override.clone(),
            Some(inner) => inner.clone(),
        };
        let allowed_sources_override = match &input.allowed_sources_override {
            None => existing.allowed_sources_override.clone(),
            Some(inner) => inner.clone(),
        };
        let release_group_blacklist_override = match &input.release_group_blacklist_override {
            None => existing.release_group_blacklist_override.clone(),
            Some(inner) => inner.clone(),
        };
        let release_group_whitelist_override = match &input.release_group_whitelist_override {
            None => existing.release_group_whitelist_override.clone(),
            Some(inner) => inner.clone(),
        };

        sqlx::query(
            r#"
            UPDATE tv_shows SET
                name = ?2,
                sort_name = ?3,
                year = ?4,
                status = ?5,
                overview = ?6,
                network = ?7,
                runtime = ?8,
                genres = ?9,
                poster_url = ?10,
                backdrop_url = ?11,
                monitored = ?12,
                monitor_type = ?13,
                path = ?14,
                auto_download_override = ?15,
                backfill_existing = ?16,
                organize_files_override = ?17,
                rename_style_override = ?18,
                auto_hunt_override = ?19,
                allowed_resolutions_override = ?20,
                allowed_video_codecs_override = ?21,
                allowed_audio_formats_override = ?22,
                require_hdr_override = ?23,
                allowed_hdr_types_override = ?24,
                allowed_sources_override = ?25,
                release_group_blacklist_override = ?26,
                release_group_whitelist_override = ?27,
                updated_at = ?28
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(name)
        .bind(&sort_name)
        .bind(year)
        .bind(status)
        .bind(&overview)
        .bind(&network)
        .bind(runtime)
        .bind(vec_to_json(&genres))
        .bind(&poster_url)
        .bind(&backdrop_url)
        .bind(bool_to_int(monitored))
        .bind(monitor_type)
        .bind(&path)
        .bind(auto_download_override.map(bool_to_int))
        .bind(bool_to_int(backfill_existing))
        .bind(organize_files_override.map(bool_to_int))
        .bind(&rename_style_override)
        .bind(auto_hunt_override.map(bool_to_int))
        .bind(allowed_resolutions_override.as_ref().map(|v| vec_to_json(v)))
        .bind(allowed_video_codecs_override.as_ref().map(|v| vec_to_json(v)))
        .bind(allowed_audio_formats_override.as_ref().map(|v| vec_to_json(v)))
        .bind(require_hdr_override.map(bool_to_int))
        .bind(allowed_hdr_types_override.as_ref().map(|v| vec_to_json(v)))
        .bind(allowed_sources_override.as_ref().map(|v| vec_to_json(v)))
        .bind(release_group_blacklist_override.as_ref().map(|v| vec_to_json(v)))
        .bind(release_group_whitelist_override.as_ref().map(|v| vec_to_json(v)))
        .bind(&now)
        .execute(&self.pool)
        .await?;

        // Fetch the updated record
        self.get_by_id(id).await
    }

    /// Delete a TV show

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM tv_shows WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update episode statistics for a show

    #[cfg(feature = "sqlite")]
    pub async fn update_stats(&self, id: Uuid) -> Result<()> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            UPDATE tv_shows SET
                episode_count = (SELECT COUNT(*) FROM episodes WHERE tv_show_id = ?1),
                episode_file_count = (SELECT COUNT(*) FROM episodes WHERE tv_show_id = ?1 AND status = 'downloaded'),
                size_bytes = (
                    SELECT COALESCE(SUM(mf.size), 0)
                    FROM episodes e
                    JOIN media_files mf ON mf.episode_id = e.id
                    WHERE e.tv_show_id = ?1
                ),
                updated_at = ?2
            WHERE id = ?1
            "#,
        )
        .bind(&id_str)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Set the hunt_individual_items flag for a TV show
    ///
    /// When true, auto-hunt will search for individual episodes instead of season packs.
    /// This is set after a partial season download completes.
    #[cfg(feature = "sqlite")]
    pub async fn set_hunt_individual_items(&self, id: Uuid, value: bool) -> Result<()> {
        use crate::db::sqlite_helpers::bool_to_int;

        sqlx::query(
            r#"
            UPDATE tv_shows SET 
                hunt_individual_items = ?2,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(bool_to_int(value))
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
