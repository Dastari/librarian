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
    pub auto_rename: bool,
    pub naming_pattern: Option<String>,
    pub default_quality_profile_id: Option<Uuid>,
    pub auto_add_discovered: bool,
    pub last_scanned_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
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
    pub auto_rename: bool,
    pub naming_pattern: Option<String>,
    pub default_quality_profile_id: Option<Uuid>,
    pub auto_add_discovered: bool,
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
    pub auto_rename: Option<bool>,
    pub naming_pattern: Option<String>,
    pub default_quality_profile_id: Option<Uuid>,
    pub auto_add_discovered: Option<bool>,
}

/// Library statistics
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct LibraryStats {
    pub file_count: Option<i64>,
    pub total_size_bytes: Option<i64>,
    pub show_count: Option<i64>,
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
                   post_download_action, auto_rename, naming_pattern,
                   default_quality_profile_id, auto_add_discovered,
                   last_scanned_at, created_at, updated_at
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
                   post_download_action, auto_rename, naming_pattern,
                   default_quality_profile_id, auto_add_discovered,
                   last_scanned_at, created_at, updated_at
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
    pub async fn get_by_id_and_user(&self, id: Uuid, user_id: Uuid) -> Result<Option<LibraryRecord>> {
        let record = sqlx::query_as::<_, LibraryRecord>(
            r#"
            SELECT id, user_id, name, path, library_type, icon, color, 
                   auto_scan, scan_interval_minutes, watch_for_changes,
                   post_download_action, auto_rename, naming_pattern,
                   default_quality_profile_id, auto_add_discovered,
                   last_scanned_at, created_at, updated_at
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
                post_download_action, auto_rename, naming_pattern,
                default_quality_profile_id, auto_add_discovered
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING id, user_id, name, path, library_type, icon, color, 
                      auto_scan, scan_interval_minutes, watch_for_changes,
                      post_download_action, auto_rename, naming_pattern,
                      default_quality_profile_id, auto_add_discovered,
                      last_scanned_at, created_at, updated_at
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
        .bind(input.auto_rename)
        .bind(&input.naming_pattern)
        .bind(input.default_quality_profile_id)
        .bind(input.auto_add_discovered)
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
                auto_rename = COALESCE($10, auto_rename),
                naming_pattern = COALESCE($11, naming_pattern),
                default_quality_profile_id = COALESCE($12, default_quality_profile_id),
                auto_add_discovered = COALESCE($13, auto_add_discovered),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, user_id, name, path, library_type, icon, color, 
                      auto_scan, scan_interval_minutes, watch_for_changes,
                      post_download_action, auto_rename, naming_pattern,
                      default_quality_profile_id, auto_add_discovered,
                      last_scanned_at, created_at, updated_at
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
        .bind(input.auto_rename)
        .bind(&input.naming_pattern)
        .bind(input.default_quality_profile_id)
        .bind(input.auto_add_discovered)
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

    /// Get library statistics
    pub async fn get_stats(&self, id: Uuid) -> Result<LibraryStats> {
        // Use separate queries to ensure proper type handling
        let file_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM media_files WHERE library_id = $1"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let total_size: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(size), 0)::BIGINT FROM media_files WHERE library_id = $1"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        let show_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tv_shows WHERE library_id = $1"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        tracing::debug!(
            library_id = %id,
            file_count = file_count,
            total_size = total_size,
            show_count = show_count,
            "Library stats fetched"
        );

        Ok(LibraryStats {
            file_count: Some(file_count),
            total_size_bytes: Some(total_size),
            show_count: Some(show_count),
        })
    }
}
