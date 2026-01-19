//! Library database repository

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Library record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LibraryRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub path: String,
    pub library_type: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub auto_scan: bool,
    pub scan_interval_minutes: i32,
    pub watch_for_changes: bool,
    pub post_download_action: String,
    pub organize_files: bool,
    pub rename_style: String,
    pub naming_pattern: Option<String>,
    pub default_quality_profile_id: Option<Uuid>,
    pub auto_add_discovered: bool,
    pub auto_download: bool,
    pub auto_hunt: bool,
    pub scanning: bool,
    pub last_scanned_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    // Inline quality settings (empty = any)
    pub allowed_resolutions: Vec<String>,
    pub allowed_video_codecs: Vec<String>,
    pub allowed_audio_formats: Vec<String>,
    pub require_hdr: bool,
    pub allowed_hdr_types: Vec<String>,
    pub allowed_sources: Vec<String>,
    pub release_group_blacklist: Vec<String>,
    pub release_group_whitelist: Vec<String>,
    // Subtitle settings
    pub auto_download_subtitles: Option<bool>,
    pub preferred_subtitle_languages: Option<Vec<String>>,
}

/// Input for creating a library
#[derive(Debug)]
pub struct CreateLibrary {
    pub user_id: Uuid,
    pub name: String,
    pub path: String,
    pub library_type: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub auto_scan: bool,
    pub scan_interval_minutes: i32,
    pub watch_for_changes: bool,
    pub post_download_action: String,
    pub organize_files: bool,
    pub rename_style: String,
    pub naming_pattern: Option<String>,
    pub default_quality_profile_id: Option<Uuid>,
    pub auto_add_discovered: bool,
    pub auto_download: bool,
    pub auto_hunt: bool,
    // Inline quality settings
    pub allowed_resolutions: Vec<String>,
    pub allowed_video_codecs: Vec<String>,
    pub allowed_audio_formats: Vec<String>,
    pub require_hdr: bool,
    pub allowed_hdr_types: Vec<String>,
    pub allowed_sources: Vec<String>,
    pub release_group_blacklist: Vec<String>,
    pub release_group_whitelist: Vec<String>,
}

/// Input for updating a library
#[derive(Debug, Default)]
pub struct UpdateLibrary {
    pub name: Option<String>,
    pub path: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub auto_scan: Option<bool>,
    pub scan_interval_minutes: Option<i32>,
    pub watch_for_changes: Option<bool>,
    pub post_download_action: Option<String>,
    pub organize_files: Option<bool>,
    pub rename_style: Option<String>,
    pub naming_pattern: Option<String>,
    pub default_quality_profile_id: Option<Uuid>,
    pub auto_add_discovered: Option<bool>,
    pub auto_download: Option<bool>,
    pub auto_hunt: Option<bool>,
    // Inline quality settings
    pub allowed_resolutions: Option<Vec<String>>,
    pub allowed_video_codecs: Option<Vec<String>>,
    pub allowed_audio_formats: Option<Vec<String>>,
    pub require_hdr: Option<bool>,
    pub allowed_hdr_types: Option<Vec<String>>,
    pub allowed_sources: Option<Vec<String>>,
    pub release_group_blacklist: Option<Vec<String>>,
    pub release_group_whitelist: Option<Vec<String>>,
}

/// Library statistics
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct LibraryStats {
    pub file_count: Option<i64>,
    pub total_size_bytes: Option<i64>,
    pub show_count: Option<i64>,
    pub movie_count: Option<i64>,
}

pub struct LibraryRepository {
    pool: PgPool,
}

impl LibraryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all libraries for a user
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<LibraryRecord>> {
        let records = sqlx::query_as::<_, LibraryRecord>(
            r#"
            SELECT id, user_id, name, path, library_type, icon, color, 
                   auto_scan, scan_interval_minutes, watch_for_changes,
                   post_download_action, organize_files, rename_style, naming_pattern,
                   default_quality_profile_id, auto_add_discovered, auto_download, auto_hunt,
                   scanning, last_scanned_at, created_at, updated_at,
                   allowed_resolutions, allowed_video_codecs, allowed_audio_formats,
                   require_hdr, allowed_hdr_types, allowed_sources,
                   release_group_blacklist, release_group_whitelist,
                   auto_download_subtitles, preferred_subtitle_languages
            FROM libraries
            WHERE user_id = $1
            ORDER BY name
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get a library by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<LibraryRecord>> {
        let record = sqlx::query_as::<_, LibraryRecord>(
            r#"
            SELECT id, user_id, name, path, library_type, icon, color, 
                   auto_scan, scan_interval_minutes, watch_for_changes,
                   post_download_action, organize_files, rename_style, naming_pattern,
                   default_quality_profile_id, auto_add_discovered, auto_download, auto_hunt,
                   scanning, last_scanned_at, created_at, updated_at,
                   allowed_resolutions, allowed_video_codecs, allowed_audio_formats,
                   require_hdr, allowed_hdr_types, allowed_sources,
                   release_group_blacklist, release_group_whitelist,
                   auto_download_subtitles, preferred_subtitle_languages
            FROM libraries
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a library by ID and user (for auth check)
    pub async fn get_by_id_and_user(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<LibraryRecord>> {
        let record = sqlx::query_as::<_, LibraryRecord>(
            r#"
            SELECT id, user_id, name, path, library_type, icon, color, 
                   auto_scan, scan_interval_minutes, watch_for_changes,
                   post_download_action, organize_files, rename_style, naming_pattern,
                   default_quality_profile_id, auto_add_discovered, auto_download, auto_hunt,
                   scanning, last_scanned_at, created_at, updated_at,
                   allowed_resolutions, allowed_video_codecs, allowed_audio_formats,
                   require_hdr, allowed_hdr_types, allowed_sources,
                   release_group_blacklist, release_group_whitelist,
                   auto_download_subtitles, preferred_subtitle_languages
            FROM libraries
            WHERE id = $1 AND user_id = $2
            "#,
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create a new library
    pub async fn create(&self, input: CreateLibrary) -> Result<LibraryRecord> {
        let record = sqlx::query_as::<_, LibraryRecord>(
            r#"
            INSERT INTO libraries (
                user_id, name, path, library_type, icon, color,
                auto_scan, scan_interval_minutes, watch_for_changes,
                post_download_action, organize_files, rename_style, naming_pattern,
                default_quality_profile_id, auto_add_discovered, auto_download, auto_hunt,
                allowed_resolutions, allowed_video_codecs, allowed_audio_formats,
                require_hdr, allowed_hdr_types, allowed_sources,
                release_group_blacklist, release_group_whitelist
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                    $18, $19, $20, $21, $22, $23, $24, $25)
            RETURNING id, user_id, name, path, library_type, icon, color, 
                      auto_scan, scan_interval_minutes, watch_for_changes,
                      post_download_action, organize_files, rename_style, naming_pattern,
                      default_quality_profile_id, auto_add_discovered, auto_download, auto_hunt,
                      scanning, last_scanned_at, created_at, updated_at,
                      allowed_resolutions, allowed_video_codecs, allowed_audio_formats,
                      require_hdr, allowed_hdr_types, allowed_sources,
                      release_group_blacklist, release_group_whitelist,
                      auto_download_subtitles, preferred_subtitle_languages
            "#,
        )
        .bind(input.user_id)
        .bind(&input.name)
        .bind(&input.path)
        .bind(&input.library_type)
        .bind(&input.icon)
        .bind(&input.color)
        .bind(input.auto_scan)
        .bind(input.scan_interval_minutes)
        .bind(input.watch_for_changes)
        .bind(&input.post_download_action)
        .bind(input.organize_files)
        .bind(&input.rename_style)
        .bind(&input.naming_pattern)
        .bind(input.default_quality_profile_id)
        .bind(input.auto_add_discovered)
        .bind(input.auto_download)
        .bind(input.auto_hunt)
        .bind(&input.allowed_resolutions)
        .bind(&input.allowed_video_codecs)
        .bind(&input.allowed_audio_formats)
        .bind(input.require_hdr)
        .bind(&input.allowed_hdr_types)
        .bind(&input.allowed_sources)
        .bind(&input.release_group_blacklist)
        .bind(&input.release_group_whitelist)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update a library
    pub async fn update(&self, id: Uuid, input: UpdateLibrary) -> Result<Option<LibraryRecord>> {
        // Build dynamic update query
        let record = sqlx::query_as::<_, LibraryRecord>(
            r#"
            UPDATE libraries SET
                name = COALESCE($2, name),
                path = COALESCE($3, path),
                icon = COALESCE($4, icon),
                color = COALESCE($5, color),
                auto_scan = COALESCE($6, auto_scan),
                scan_interval_minutes = COALESCE($7, scan_interval_minutes),
                watch_for_changes = COALESCE($8, watch_for_changes),
                post_download_action = COALESCE($9, post_download_action),
                organize_files = COALESCE($10, organize_files),
                rename_style = COALESCE($11, rename_style),
                naming_pattern = COALESCE($12, naming_pattern),
                default_quality_profile_id = COALESCE($13, default_quality_profile_id),
                auto_add_discovered = COALESCE($14, auto_add_discovered),
                auto_download = COALESCE($15, auto_download),
                auto_hunt = COALESCE($16, auto_hunt),
                allowed_resolutions = COALESCE($17, allowed_resolutions),
                allowed_video_codecs = COALESCE($18, allowed_video_codecs),
                allowed_audio_formats = COALESCE($19, allowed_audio_formats),
                require_hdr = COALESCE($20, require_hdr),
                allowed_hdr_types = COALESCE($21, allowed_hdr_types),
                allowed_sources = COALESCE($22, allowed_sources),
                release_group_blacklist = COALESCE($23, release_group_blacklist),
                release_group_whitelist = COALESCE($24, release_group_whitelist),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, user_id, name, path, library_type, icon, color, 
                      auto_scan, scan_interval_minutes, watch_for_changes,
                      post_download_action, organize_files, rename_style, naming_pattern,
                      default_quality_profile_id, auto_add_discovered, auto_download, auto_hunt,
                      scanning, last_scanned_at, created_at, updated_at,
                      allowed_resolutions, allowed_video_codecs, allowed_audio_formats,
                      require_hdr, allowed_hdr_types, allowed_sources,
                      release_group_blacklist, release_group_whitelist,
                      auto_download_subtitles, preferred_subtitle_languages
            "#,
        )
        .bind(id)
        .bind(&input.name)
        .bind(&input.path)
        .bind(&input.icon)
        .bind(&input.color)
        .bind(input.auto_scan)
        .bind(input.scan_interval_minutes)
        .bind(input.watch_for_changes)
        .bind(&input.post_download_action)
        .bind(input.organize_files)
        .bind(&input.rename_style)
        .bind(&input.naming_pattern)
        .bind(input.default_quality_profile_id)
        .bind(input.auto_add_discovered)
        .bind(input.auto_download)
        .bind(input.auto_hunt)
        .bind(&input.allowed_resolutions)
        .bind(&input.allowed_video_codecs)
        .bind(&input.allowed_audio_formats)
        .bind(input.require_hdr)
        .bind(&input.allowed_hdr_types)
        .bind(&input.allowed_sources)
        .bind(&input.release_group_blacklist)
        .bind(&input.release_group_whitelist)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete a library
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM libraries WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update last scanned timestamp
    pub async fn update_last_scanned(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE libraries SET last_scanned_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Set the scanning state for a library
    pub async fn set_scanning(&self, id: Uuid, scanning: bool) -> Result<()> {
        sqlx::query("UPDATE libraries SET scanning = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(scanning)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get library statistics
    ///
    /// Only counts media files that are actually within the library path,
    /// not files in other locations (like downloads) that happen to be linked.
    pub async fn get_stats(&self, id: Uuid) -> Result<LibraryStats> {
        // First get the library path to filter files
        let library_path: Option<String> = sqlx::query_scalar(
            "SELECT path FROM libraries WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let path_pattern = library_path
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "%".to_string());

        // Count only files that are within the library path
        let file_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM media_files WHERE library_id = $1 AND path LIKE $2"
        )
        .bind(id)
        .bind(&path_pattern)
        .fetch_one(&self.pool)
        .await?;

        // Sum size only for files within the library path
        let total_size: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(size), 0)::BIGINT FROM media_files WHERE library_id = $1 AND path LIKE $2",
        )
        .bind(id)
        .bind(&path_pattern)
        .fetch_one(&self.pool)
        .await?;

        let show_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tv_shows WHERE library_id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;

        let movie_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM movies WHERE library_id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;

        tracing::debug!(
            library_id = %id,
            file_count = file_count,
            total_size = total_size,
            show_count = show_count,
            movie_count = movie_count,
            "Library stats fetched"
        );

        Ok(LibraryStats {
            file_count: Some(file_count),
            total_size_bytes: Some(total_size),
            show_count: Some(show_count),
            movie_count: Some(movie_count),
        })
    }
}
