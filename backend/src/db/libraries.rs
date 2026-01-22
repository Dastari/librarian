//! Library database repository

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

/// Library record from database
#[derive(Debug, Clone)]
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

#[cfg(feature = "postgres")]
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for LibraryRecord {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            name: row.try_get("name")?,
            path: row.try_get("path")?,
            library_type: row.try_get("library_type")?,
            icon: row.try_get("icon")?,
            color: row.try_get("color")?,
            auto_scan: row.try_get("auto_scan")?,
            scan_interval_minutes: row.try_get("scan_interval_minutes")?,
            watch_for_changes: row.try_get("watch_for_changes")?,
            post_download_action: row.try_get("post_download_action")?,
            organize_files: row.try_get("organize_files")?,
            rename_style: row.try_get("rename_style")?,
            naming_pattern: row.try_get("naming_pattern")?,
            auto_add_discovered: row.try_get("auto_add_discovered")?,
            auto_download: row.try_get("auto_download")?,
            auto_hunt: row.try_get("auto_hunt")?,
            scanning: row.try_get("scanning")?,
            last_scanned_at: row.try_get("last_scanned_at")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            allowed_resolutions: row.try_get("allowed_resolutions")?,
            allowed_video_codecs: row.try_get("allowed_video_codecs")?,
            allowed_audio_formats: row.try_get("allowed_audio_formats")?,
            require_hdr: row.try_get("require_hdr")?,
            allowed_hdr_types: row.try_get("allowed_hdr_types")?,
            allowed_sources: row.try_get("allowed_sources")?,
            release_group_blacklist: row.try_get("release_group_blacklist")?,
            release_group_whitelist: row.try_get("release_group_whitelist")?,
            auto_download_subtitles: row.try_get("auto_download_subtitles")?,
            preferred_subtitle_languages: row.try_get("preferred_subtitle_languages")?,
        })
    }
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for LibraryRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        use crate::db::sqlite_helpers::{str_to_uuid, str_to_datetime, int_to_bool, json_to_vec};
        
        let id_str: String = row.try_get("id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;
        let last_scanned_str: Option<String> = row.try_get("last_scanned_at")?;
        
        // JSON arrays stored as TEXT
        let allowed_resolutions_json: String = row.try_get("allowed_resolutions")?;
        let allowed_video_codecs_json: String = row.try_get("allowed_video_codecs")?;
        let allowed_audio_formats_json: String = row.try_get("allowed_audio_formats")?;
        let allowed_hdr_types_json: String = row.try_get("allowed_hdr_types")?;
        let allowed_sources_json: String = row.try_get("allowed_sources")?;
        let release_group_blacklist_json: String = row.try_get("release_group_blacklist")?;
        let release_group_whitelist_json: String = row.try_get("release_group_whitelist")?;
        let preferred_subtitle_languages_json: Option<String> = row.try_get("preferred_subtitle_languages")?;
        
        // Booleans stored as INTEGER
        let auto_scan: i32 = row.try_get("auto_scan")?;
        let watch_for_changes: i32 = row.try_get("watch_for_changes")?;
        let organize_files: i32 = row.try_get("organize_files")?;
        let auto_add_discovered: i32 = row.try_get("auto_add_discovered")?;
        let auto_download: i32 = row.try_get("auto_download")?;
        let auto_hunt: i32 = row.try_get("auto_hunt")?;
        let scanning: i32 = row.try_get("scanning")?;
        let require_hdr: i32 = row.try_get("require_hdr")?;
        let auto_download_subtitles: Option<i32> = row.try_get("auto_download_subtitles")?;
        
        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            name: row.try_get("name")?,
            path: row.try_get("path")?,
            library_type: row.try_get("library_type")?,
            icon: row.try_get("icon")?,
            color: row.try_get("color")?,
            auto_scan: int_to_bool(auto_scan),
            scan_interval_minutes: row.try_get("scan_interval_minutes")?,
            watch_for_changes: int_to_bool(watch_for_changes),
            post_download_action: row.try_get("post_download_action")?,
            organize_files: int_to_bool(organize_files),
            rename_style: row.try_get("rename_style")?,
            naming_pattern: row.try_get("naming_pattern")?,
            auto_add_discovered: int_to_bool(auto_add_discovered),
            auto_download: int_to_bool(auto_download),
            auto_hunt: int_to_bool(auto_hunt),
            scanning: int_to_bool(scanning),
            last_scanned_at: last_scanned_str
                .map(|s| str_to_datetime(&s))
                .transpose()
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            created_at: str_to_datetime(&created_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            allowed_resolutions: json_to_vec(&allowed_resolutions_json),
            allowed_video_codecs: json_to_vec(&allowed_video_codecs_json),
            allowed_audio_formats: json_to_vec(&allowed_audio_formats_json),
            require_hdr: int_to_bool(require_hdr),
            allowed_hdr_types: json_to_vec(&allowed_hdr_types_json),
            allowed_sources: json_to_vec(&allowed_sources_json),
            release_group_blacklist: json_to_vec(&release_group_blacklist_json),
            release_group_whitelist: json_to_vec(&release_group_whitelist_json),
            auto_download_subtitles: auto_download_subtitles.map(int_to_bool),
            preferred_subtitle_languages: preferred_subtitle_languages_json.map(|s| json_to_vec(&s)),
        })
    }
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
#[derive(Debug, Clone, Default)]
pub struct LibraryStats {
    pub file_count: Option<i64>,
    pub total_size_bytes: Option<i64>,
    pub show_count: Option<i64>,
    pub movie_count: Option<i64>,
}

pub struct LibraryRepository {
    pool: DbPool,
}

impl LibraryRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get all libraries for a user
    #[cfg(feature = "postgres")]
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<LibraryRecord>> {
        let records = sqlx::query_as::<_, LibraryRecord>(
            r#"
            SELECT id, user_id, name, path, library_type, icon, color, 
                   auto_scan, scan_interval_minutes, watch_for_changes,
                   post_download_action, organize_files, rename_style, naming_pattern,
                   auto_add_discovered, auto_download, auto_hunt,
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

    #[cfg(feature = "sqlite")]
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<LibraryRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let records = sqlx::query_as::<_, LibraryRecord>(
            r#"
            SELECT id, user_id, name, path, library_type, icon, color, 
                   auto_scan, scan_interval_minutes, watch_for_changes,
                   post_download_action, organize_files, rename_style, naming_pattern,
                   auto_add_discovered, auto_download, auto_hunt,
                   scanning, last_scanned_at, created_at, updated_at,
                   allowed_resolutions, allowed_video_codecs, allowed_audio_formats,
                   require_hdr, allowed_hdr_types, allowed_sources,
                   release_group_blacklist, release_group_whitelist,
                   auto_download_subtitles, preferred_subtitle_languages
            FROM libraries
            WHERE user_id = ?1
            ORDER BY name
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get a library by ID
    #[cfg(feature = "postgres")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<LibraryRecord>> {
        let record = sqlx::query_as::<_, LibraryRecord>(
            r#"
            SELECT id, user_id, name, path, library_type, icon, color, 
                   auto_scan, scan_interval_minutes, watch_for_changes,
                   post_download_action, organize_files, rename_style, naming_pattern,
                   auto_add_discovered, auto_download, auto_hunt,
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

    #[cfg(feature = "sqlite")]
    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<LibraryRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let record = sqlx::query_as::<_, LibraryRecord>(
            r#"
            SELECT id, user_id, name, path, library_type, icon, color, 
                   auto_scan, scan_interval_minutes, watch_for_changes,
                   post_download_action, organize_files, rename_style, naming_pattern,
                   auto_add_discovered, auto_download, auto_hunt,
                   scanning, last_scanned_at, created_at, updated_at,
                   allowed_resolutions, allowed_video_codecs, allowed_audio_formats,
                   require_hdr, allowed_hdr_types, allowed_sources,
                   release_group_blacklist, release_group_whitelist,
                   auto_download_subtitles, preferred_subtitle_languages
            FROM libraries
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get a library by ID and user (for auth check)
    #[cfg(feature = "postgres")]
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
                   auto_add_discovered, auto_download, auto_hunt,
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

    #[cfg(feature = "sqlite")]
    pub async fn get_by_id_and_user(
        &self,
        id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<LibraryRecord>> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let record = sqlx::query_as::<_, LibraryRecord>(
            r#"
            SELECT id, user_id, name, path, library_type, icon, color, 
                   auto_scan, scan_interval_minutes, watch_for_changes,
                   post_download_action, organize_files, rename_style, naming_pattern,
                   auto_add_discovered, auto_download, auto_hunt,
                   scanning, last_scanned_at, created_at, updated_at,
                   allowed_resolutions, allowed_video_codecs, allowed_audio_formats,
                   require_hdr, allowed_hdr_types, allowed_sources,
                   release_group_blacklist, release_group_whitelist,
                   auto_download_subtitles, preferred_subtitle_languages
            FROM libraries
            WHERE id = ?1 AND user_id = ?2
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(uuid_to_str(user_id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Create a new library
    #[cfg(feature = "postgres")]
    pub async fn create(&self, input: CreateLibrary) -> Result<LibraryRecord> {
        let record = sqlx::query_as::<_, LibraryRecord>(
            r#"
            INSERT INTO libraries (
                user_id, name, path, library_type, icon, color,
                auto_scan, scan_interval_minutes, watch_for_changes,
                post_download_action, organize_files, rename_style, naming_pattern,
                auto_add_discovered, auto_download, auto_hunt,
                allowed_resolutions, allowed_video_codecs, allowed_audio_formats,
                require_hdr, allowed_hdr_types, allowed_sources,
                release_group_blacklist, release_group_whitelist
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                    $17, $18, $19, $20, $21, $22, $23, $24)
            RETURNING id, user_id, name, path, library_type, icon, color, 
                      auto_scan, scan_interval_minutes, watch_for_changes,
                      post_download_action, organize_files, rename_style, naming_pattern,
                      auto_add_discovered, auto_download, auto_hunt,
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

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, input: CreateLibrary) -> Result<LibraryRecord> {
        use crate::db::sqlite_helpers::{uuid_to_str, vec_to_json, bool_to_int};
        
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);
        
        sqlx::query(
            r#"
            INSERT INTO libraries (
                id, user_id, name, path, library_type, icon, color,
                auto_scan, scan_interval_minutes, watch_for_changes,
                post_download_action, organize_files, rename_style, naming_pattern,
                auto_add_discovered, auto_download, auto_hunt,
                allowed_resolutions, allowed_video_codecs, allowed_audio_formats,
                require_hdr, allowed_hdr_types, allowed_sources,
                release_group_blacklist, release_group_whitelist,
                scanning, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                    ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, 0, datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(uuid_to_str(input.user_id))
        .bind(&input.name)
        .bind(&input.path)
        .bind(&input.library_type)
        .bind(&input.icon)
        .bind(&input.color)
        .bind(bool_to_int(input.auto_scan))
        .bind(input.scan_interval_minutes)
        .bind(bool_to_int(input.watch_for_changes))
        .bind(&input.post_download_action)
        .bind(bool_to_int(input.organize_files))
        .bind(&input.rename_style)
        .bind(&input.naming_pattern)
        .bind(bool_to_int(input.auto_add_discovered))
        .bind(bool_to_int(input.auto_download))
        .bind(bool_to_int(input.auto_hunt))
        .bind(vec_to_json(&input.allowed_resolutions))
        .bind(vec_to_json(&input.allowed_video_codecs))
        .bind(vec_to_json(&input.allowed_audio_formats))
        .bind(bool_to_int(input.require_hdr))
        .bind(vec_to_json(&input.allowed_hdr_types))
        .bind(vec_to_json(&input.allowed_sources))
        .bind(vec_to_json(&input.release_group_blacklist))
        .bind(vec_to_json(&input.release_group_whitelist))
        .execute(&self.pool)
        .await?;

        self.get_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("Failed to retrieve library after insert"))
    }

    /// Update a library
    #[cfg(feature = "postgres")]
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
                auto_add_discovered = COALESCE($13, auto_add_discovered),
                auto_download = COALESCE($14, auto_download),
                auto_hunt = COALESCE($15, auto_hunt),
                allowed_resolutions = COALESCE($16, allowed_resolutions),
                allowed_video_codecs = COALESCE($17, allowed_video_codecs),
                allowed_audio_formats = COALESCE($18, allowed_audio_formats),
                require_hdr = COALESCE($19, require_hdr),
                allowed_hdr_types = COALESCE($20, allowed_hdr_types),
                allowed_sources = COALESCE($21, allowed_sources),
                release_group_blacklist = COALESCE($22, release_group_blacklist),
                release_group_whitelist = COALESCE($23, release_group_whitelist),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, user_id, name, path, library_type, icon, color, 
                      auto_scan, scan_interval_minutes, watch_for_changes,
                      post_download_action, organize_files, rename_style, naming_pattern,
                      auto_add_discovered, auto_download, auto_hunt,
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

    #[cfg(feature = "sqlite")]
    pub async fn update(&self, id: Uuid, input: UpdateLibrary) -> Result<Option<LibraryRecord>> {
        use crate::db::sqlite_helpers::{uuid_to_str, vec_to_json, bool_to_int};
        
        // For SQLite, we need to build the update dynamically or use COALESCE with proper JSON handling
        // Using a simpler approach: update all fields if provided
        let id_str = uuid_to_str(id);
        
        // First get current record
        let current = match self.get_by_id(id).await? {
            Some(r) => r,
            None => return Ok(None),
        };
        
        sqlx::query(
            r#"
            UPDATE libraries SET
                name = ?2,
                path = ?3,
                icon = ?4,
                color = ?5,
                auto_scan = ?6,
                scan_interval_minutes = ?7,
                watch_for_changes = ?8,
                post_download_action = ?9,
                organize_files = ?10,
                rename_style = ?11,
                naming_pattern = ?12,
                auto_add_discovered = ?13,
                auto_download = ?14,
                auto_hunt = ?15,
                allowed_resolutions = ?16,
                allowed_video_codecs = ?17,
                allowed_audio_formats = ?18,
                require_hdr = ?19,
                allowed_hdr_types = ?20,
                allowed_sources = ?21,
                release_group_blacklist = ?22,
                release_group_whitelist = ?23,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(&id_str)
        .bind(input.name.unwrap_or(current.name))
        .bind(input.path.unwrap_or(current.path))
        .bind(input.icon.or(current.icon))
        .bind(input.color.or(current.color))
        .bind(bool_to_int(input.auto_scan.unwrap_or(current.auto_scan)))
        .bind(input.scan_interval_minutes.unwrap_or(current.scan_interval_minutes))
        .bind(bool_to_int(input.watch_for_changes.unwrap_or(current.watch_for_changes)))
        .bind(input.post_download_action.unwrap_or(current.post_download_action))
        .bind(bool_to_int(input.organize_files.unwrap_or(current.organize_files)))
        .bind(input.rename_style.unwrap_or(current.rename_style))
        .bind(input.naming_pattern.or(current.naming_pattern))
        .bind(bool_to_int(input.auto_add_discovered.unwrap_or(current.auto_add_discovered)))
        .bind(bool_to_int(input.auto_download.unwrap_or(current.auto_download)))
        .bind(bool_to_int(input.auto_hunt.unwrap_or(current.auto_hunt)))
        .bind(vec_to_json(&input.allowed_resolutions.unwrap_or(current.allowed_resolutions)))
        .bind(vec_to_json(&input.allowed_video_codecs.unwrap_or(current.allowed_video_codecs)))
        .bind(vec_to_json(&input.allowed_audio_formats.unwrap_or(current.allowed_audio_formats)))
        .bind(bool_to_int(input.require_hdr.unwrap_or(current.require_hdr)))
        .bind(vec_to_json(&input.allowed_hdr_types.unwrap_or(current.allowed_hdr_types)))
        .bind(vec_to_json(&input.allowed_sources.unwrap_or(current.allowed_sources)))
        .bind(vec_to_json(&input.release_group_blacklist.unwrap_or(current.release_group_blacklist)))
        .bind(vec_to_json(&input.release_group_whitelist.unwrap_or(current.release_group_whitelist)))
        .execute(&self.pool)
        .await?;

        self.get_by_id(id).await
    }

    /// Delete a library
    #[cfg(feature = "postgres")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM libraries WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let result = sqlx::query("DELETE FROM libraries WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update last scanned timestamp
    #[cfg(feature = "postgres")]
    pub async fn update_last_scanned(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE libraries SET last_scanned_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn update_last_scanned(&self, id: Uuid) -> Result<()> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        sqlx::query("UPDATE libraries SET last_scanned_at = datetime('now') WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Set the scanning state for a library
    #[cfg(feature = "postgres")]
    pub async fn set_scanning(&self, id: Uuid, scanning: bool) -> Result<()> {
        sqlx::query("UPDATE libraries SET scanning = $2, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(scanning)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub async fn set_scanning(&self, id: Uuid, scanning: bool) -> Result<()> {
        use crate::db::sqlite_helpers::{uuid_to_str, bool_to_int};
        
        sqlx::query("UPDATE libraries SET scanning = ?2, updated_at = datetime('now') WHERE id = ?1")
            .bind(uuid_to_str(id))
            .bind(bool_to_int(scanning))
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get library statistics
    #[cfg(feature = "postgres")]
    pub async fn get_stats(&self, id: Uuid) -> Result<LibraryStats> {
        // First get the library path to filter files
        let library_path: Option<String> =
            sqlx::query_scalar("SELECT path FROM libraries WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        let path_pattern = library_path
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "%".to_string());

        // Count only files that are within the library path
        let file_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM media_files WHERE library_id = $1 AND path LIKE $2",
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

    #[cfg(feature = "sqlite")]
    pub async fn get_stats(&self, id: Uuid) -> Result<LibraryStats> {
        use crate::db::sqlite_helpers::uuid_to_str;
        
        let id_str = uuid_to_str(id);
        
        // First get the library path to filter files
        let library_path: Option<String> =
            sqlx::query_scalar("SELECT path FROM libraries WHERE id = ?1")
                .bind(&id_str)
                .fetch_optional(&self.pool)
                .await?;

        let path_pattern = library_path
            .map(|p| format!("{}%", p))
            .unwrap_or_else(|| "%".to_string());

        // Count only files that are within the library path
        let file_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM media_files WHERE library_id = ?1 AND path LIKE ?2",
        )
        .bind(&id_str)
        .bind(&path_pattern)
        .fetch_one(&self.pool)
        .await?;

        // Sum size only for files within the library path (no ::BIGINT cast in SQLite)
        let total_size: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(size), 0) FROM media_files WHERE library_id = ?1 AND path LIKE ?2",
        )
        .bind(&id_str)
        .bind(&path_pattern)
        .fetch_one(&self.pool)
        .await?;

        let show_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tv_shows WHERE library_id = ?1")
                .bind(&id_str)
                .fetch_one(&self.pool)
                .await?;

        let movie_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM movies WHERE library_id = ?1")
                .bind(&id_str)
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
