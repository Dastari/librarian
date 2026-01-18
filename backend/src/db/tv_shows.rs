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
    pub quality_profile_id: Option<Uuid>,
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
    pub quality_profile_id: Option<Uuid>,
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
                   monitor_type, quality_profile_id, path,
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

    /// Get all TV shows for a user (across all libraries)
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<TvShowRecord>> {
        let records = sqlx::query_as::<_, TvShowRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, year, status,
                   tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                   runtime, genres, poster_url, backdrop_url, monitored,
                   monitor_type, quality_profile_id, path,
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
                   monitor_type, quality_profile_id, path,
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
                   monitor_type, quality_profile_id, path,
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
                   monitor_type, quality_profile_id, path,
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
    pub async fn find_by_name_in_library(
        &self,
        library_id: Uuid,
        name: &str,
    ) -> Result<Option<TvShowRecord>> {
        // First try exact match (case-insensitive)
        let record = sqlx::query_as::<_, TvShowRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, year, status,
                   tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                   runtime, genres, poster_url, backdrop_url, monitored,
                   monitor_type, quality_profile_id, path,
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

        // Try fuzzy match using LIKE with common variations
        // Remove "The " prefix and try again, also try with dots replaced by spaces
        let normalized = name
            .trim()
            .trim_start_matches("The ")
            .trim_start_matches("the ")
            .replace(['.', '_'], " ");

        let record = sqlx::query_as::<_, TvShowRecord>(
            r#"
            SELECT id, library_id, user_id, name, sort_name, year, status,
                   tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                   runtime, genres, poster_url, backdrop_url, monitored,
                   monitor_type, quality_profile_id, path,
                   auto_download_override, backfill_existing,
                   organize_files_override, rename_style_override, auto_hunt_override,
                   episode_count, episode_file_count, size_bytes, created_at, updated_at,
                   allowed_resolutions_override, allowed_video_codecs_override,
                   allowed_audio_formats_override, require_hdr_override,
                   allowed_hdr_types_override, allowed_sources_override,
                   release_group_blacklist_override, release_group_whitelist_override
            FROM tv_shows
            WHERE library_id = $1 
              AND (
                LOWER(name) LIKE LOWER($2) 
                OR LOWER(REPLACE(REPLACE(name, '.', ' '), '_', ' ')) = LOWER($2)
              )
            LIMIT 1
            "#,
        )
        .bind(library_id)
        .bind(&normalized)
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
                monitor_type, quality_profile_id, path,
                auto_download_override, backfill_existing,
                organize_files_override, rename_style_override, auto_hunt_override,
                allowed_resolutions_override, allowed_video_codecs_override,
                allowed_audio_formats_override, require_hdr_override,
                allowed_hdr_types_override, allowed_sources_override,
                release_group_blacklist_override, release_group_whitelist_override
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25,
                    $26, $27, $28, $29, $30, $31, $32, $33)
            RETURNING id, library_id, user_id, name, sort_name, year, status,
                      tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                      runtime, genres, poster_url, backdrop_url, monitored,
                      monitor_type, quality_profile_id, path,
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
        .bind(input.quality_profile_id)
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
                quality_profile_id = COALESCE($14, quality_profile_id),
                path = COALESCE($15, path),
                -- Nullable override fields use CASE to allow setting to NULL
                auto_download_override = CASE WHEN $16 THEN $17 ELSE auto_download_override END,
                backfill_existing = COALESCE($18, backfill_existing),
                organize_files_override = CASE WHEN $19 THEN $20 ELSE organize_files_override END,
                rename_style_override = CASE WHEN $21 THEN $22 ELSE rename_style_override END,
                auto_hunt_override = CASE WHEN $23 THEN $24 ELSE auto_hunt_override END,
                allowed_resolutions_override = CASE WHEN $25 THEN $26 ELSE allowed_resolutions_override END,
                allowed_video_codecs_override = CASE WHEN $27 THEN $28 ELSE allowed_video_codecs_override END,
                allowed_audio_formats_override = CASE WHEN $29 THEN $30 ELSE allowed_audio_formats_override END,
                require_hdr_override = CASE WHEN $31 THEN $32 ELSE require_hdr_override END,
                allowed_hdr_types_override = CASE WHEN $33 THEN $34 ELSE allowed_hdr_types_override END,
                allowed_sources_override = CASE WHEN $35 THEN $36 ELSE allowed_sources_override END,
                release_group_blacklist_override = CASE WHEN $37 THEN $38 ELSE release_group_blacklist_override END,
                release_group_whitelist_override = CASE WHEN $39 THEN $40 ELSE release_group_whitelist_override END,
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, library_id, user_id, name, sort_name, year, status,
                      tvmaze_id, tmdb_id, tvdb_id, imdb_id, overview, network,
                      runtime, genres, poster_url, backdrop_url, monitored,
                      monitor_type, quality_profile_id, path,
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
        .bind(input.quality_profile_id)     // $14
        .bind(&input.path)                  // $15
        .bind(update_auto_download)         // $16 - flag
        .bind(auto_download_value)          // $17 - value
        .bind(input.backfill_existing)      // $18
        .bind(update_organize_files)        // $19 - flag
        .bind(organize_files_value)         // $20 - value
        .bind(update_rename_style)          // $21 - flag
        .bind(&rename_style_value)          // $22 - value
        .bind(update_auto_hunt)             // $23 - flag
        .bind(auto_hunt_value)              // $24 - value
        .bind(update_resolutions)           // $25 - flag
        .bind(&resolutions_value)           // $26 - value
        .bind(update_codecs)                // $27 - flag
        .bind(&codecs_value)                // $28 - value
        .bind(update_audio)                 // $29 - flag
        .bind(&audio_value)                 // $30 - value
        .bind(update_require_hdr)           // $31 - flag
        .bind(require_hdr_value)            // $32 - value
        .bind(update_hdr_types)             // $33 - flag
        .bind(&hdr_types_value)             // $34 - value
        .bind(update_sources)               // $35 - flag
        .bind(&sources_value)               // $36 - value
        .bind(update_blacklist)             // $37 - flag
        .bind(&blacklist_value)             // $38 - value
        .bind(update_whitelist)             // $39 - flag
        .bind(&whitelist_value)             // $40 - value
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
