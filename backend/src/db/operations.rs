//! Database operations module
//!
//! This module contains complex database operations that cannot be expressed
//! using the entity query system:
//! - Aggregate queries (COUNT, SUM, AVG with GROUP BY)
//! - Cross-table JOINs
//! - Batch operations with transactions
//! - Cleanup/maintenance operations
//!
//! For simple CRUD and filtered queries, use the entity query system instead:
//! ```rust,ignore
//! // Prefer this for simple queries:
//! MovieEntity::query(&pool).filter(...).fetch_all().await?;
//! ```

use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::sqlite_helpers::{now_iso8601, uuid_to_str};
use crate::services::graphql::entities::AppLog;

// ============================================================================
// Library Statistics
// ============================================================================

/// Statistics for a library
#[derive(Debug, Clone, Default)]
pub struct LibraryStats {
    pub movie_count: i64,
    pub tv_show_count: i64,
    pub episode_count: i64,
    pub artist_count: i64,
    pub album_count: i64,
    pub track_count: i64,
    pub audiobook_count: i64,
    pub media_file_count: i64,
    pub total_size_bytes: i64,
}

/// Get statistics for a library
pub async fn get_library_stats(pool: &SqlitePool, library_id: Uuid) -> Result<LibraryStats> {
    let id_str = uuid_to_str(library_id);

    let movie_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM movies WHERE library_id = ?")
        .bind(&id_str)
        .fetch_one(pool)
        .await?;

    let tv_show_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tv_shows WHERE library_id = ?")
            .bind(&id_str)
            .fetch_one(pool)
            .await?;

    let episode_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM episodes e JOIN tv_shows ts ON e.tv_show_id = ts.id WHERE ts.library_id = ?"
    )
    .bind(&id_str)
    .fetch_one(pool)
    .await?;

    let artist_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM artists WHERE library_id = ?")
        .bind(&id_str)
        .fetch_one(pool)
        .await?;

    let album_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM albums WHERE library_id = ?")
        .bind(&id_str)
        .fetch_one(pool)
        .await?;

    let track_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks WHERE library_id = ?")
        .bind(&id_str)
        .fetch_one(pool)
        .await?;

    let audiobook_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audiobooks WHERE library_id = ?")
            .bind(&id_str)
            .fetch_one(pool)
            .await?;

    let media_file_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM media_files WHERE library_id = ?")
            .bind(&id_str)
            .fetch_one(pool)
            .await?;

    let total_size_bytes: i64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(size), 0) FROM media_files WHERE library_id = ?")
            .bind(&id_str)
            .fetch_one(pool)
            .await?;

    Ok(LibraryStats {
        movie_count,
        tv_show_count,
        episode_count,
        artist_count,
        album_count,
        track_count,
        audiobook_count,
        media_file_count,
        total_size_bytes,
    })
}

// ============================================================================
// Log Statistics
// ============================================================================

/// Log counts grouped by level
#[derive(Debug, Clone)]
pub struct LogLevelCount {
    pub level: String,
    pub count: i64,
}

/// Get log counts by level
pub async fn get_log_counts_by_level(pool: &SqlitePool) -> Result<Vec<LogLevelCount>> {
    let rows = sqlx::query_as::<_, (String, i64)>(
        r#"
        SELECT level, COUNT(*) as count
        FROM app_logs
        GROUP BY level
        ORDER BY level
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(level, count)| LogLevelCount { level, count })
        .collect())
}

/// Get distinct log targets (for filtering UI)
pub async fn get_distinct_log_targets(pool: &SqlitePool, limit: i64) -> Result<Vec<String>> {
    let targets = sqlx::query_scalar::<_, String>(
        r#"
        SELECT target
        FROM app_logs
        GROUP BY target
        ORDER BY COUNT(*) DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(targets)
}

/// Insert a batch of app logs (used by the logging service).
pub async fn insert_app_logs_batch(pool: &SqlitePool, logs: &[AppLog]) -> Result<()> {
    if logs.is_empty() {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    let sql = r#"
        INSERT INTO app_logs (id, timestamp, level, target, message, fields, span_name, span_id, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
    "#;
    for log in logs {
        sqlx::query(sql)
            .bind(&log.id)
            .bind(&log.timestamp)
            .bind(&log.level)
            .bind(&log.target)
            .bind(&log.message)
            .bind(&log.fields)
            .bind(&log.span_name)
            .bind(&log.span_id)
            .bind(&log.created_at)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

// ============================================================================
// Content Progress (JOIN queries)
// ============================================================================

/// Wanted content counts for auto-hunt
#[derive(Debug, Clone, Default)]
pub struct WantedContentCounts {
    pub movies: i64,
    pub episodes: i64,
    pub tracks: i64,
    pub audiobook_chapters: i64,
}

/// Count wanted content (monitored without files) by library
pub async fn count_wanted_by_library(
    pool: &SqlitePool,
    library_id: Uuid,
) -> Result<WantedContentCounts> {
    let id_str = uuid_to_str(library_id);

    let movies: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM movies WHERE library_id = ? AND monitored = 1 AND media_file_id IS NULL"
    )
    .bind(&id_str)
    .fetch_one(pool)
    .await?;

    let episodes: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM episodes e
        JOIN tv_shows ts ON e.tv_show_id = ts.id
        WHERE ts.library_id = ? AND ts.monitored = 1 AND e.media_file_id IS NULL
        "#,
    )
    .bind(&id_str)
    .fetch_one(pool)
    .await?;

    let tracks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tracks WHERE library_id = ? AND monitored = 1 AND media_file_id IS NULL"
    )
    .bind(&id_str)
    .fetch_one(pool)
    .await?;

    let audiobook_chapters: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM chapters c
        JOIN audiobooks a ON c.audiobook_id = a.id
        WHERE a.library_id = ? AND a.monitored = 1 AND c.media_file_id IS NULL
        "#,
    )
    .bind(&id_str)
    .fetch_one(pool)
    .await?;

    Ok(WantedContentCounts {
        movies,
        episodes,
        tracks,
        audiobook_chapters,
    })
}

// ============================================================================
// TV Show Episode Stats
// ============================================================================

/// Update TV show episode counts from episodes table
pub async fn update_tv_show_episode_counts(pool: &SqlitePool, tv_show_id: Uuid) -> Result<()> {
    let id_str = uuid_to_str(tv_show_id);

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
            )
        WHERE id = ?1
        "#,
    )
    .bind(&id_str)
    .execute(pool)
    .await?;

    Ok(())
}

// ============================================================================
// Artwork Statistics
// ============================================================================

/// Artwork storage statistics
#[derive(Debug, Clone)]
pub struct ArtworkStats {
    pub total_count: i64,
    pub total_bytes: i64,
    pub by_entity_type: Vec<ArtworkEntityStats>,
}

#[derive(Debug, Clone)]
pub struct ArtworkEntityStats {
    pub entity_type: String,
    pub count: i64,
    pub total_bytes: i64,
}

/// Get artwork cache statistics
pub async fn get_artwork_stats(pool: &SqlitePool) -> Result<ArtworkStats> {
    let total: (i64, Option<i64>) =
        sqlx::query_as("SELECT COUNT(*), SUM(size_bytes) FROM artwork_cache")
            .fetch_one(pool)
            .await?;

    let by_type = sqlx::query_as::<_, (String, i64, i64)>(
        r#"
        SELECT entity_type, COUNT(*) as count, COALESCE(SUM(size_bytes), 0) as total_bytes
        FROM artwork_cache
        GROUP BY entity_type
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(ArtworkStats {
        total_count: total.0,
        total_bytes: total.1.unwrap_or(0),
        by_entity_type: by_type
            .into_iter()
            .map(|(entity_type, count, total_bytes)| ArtworkEntityStats {
                entity_type,
                count,
                total_bytes,
            })
            .collect(),
    })
}

// ============================================================================
// Notification Counts
// ============================================================================

/// Get unread notification count for a user
pub async fn get_unread_notification_count(pool: &SqlitePool, user_id: Uuid) -> Result<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE user_id = ? AND read_at IS NULL",
    )
    .bind(uuid_to_str(user_id))
    .fetch_one(pool)
    .await?;

    Ok(count)
}

/// Get action-required notification count for a user
pub async fn get_action_required_count(pool: &SqlitePool, user_id: Uuid) -> Result<i64> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM notifications 
        WHERE user_id = ? 
        AND notification_type = 'action_required' 
        AND resolved_at IS NULL
        "#,
    )
    .bind(uuid_to_str(user_id))
    .fetch_one(pool)
    .await?;

    Ok(count)
}

// ============================================================================
// User Counts
// ============================================================================

/// Count total users
pub async fn count_users(pool: &SqlitePool) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

/// Check if any admin user exists
pub async fn has_admin_user(pool: &SqlitePool) -> Result<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin' AND is_active = 1")
            .fetch_one(pool)
            .await?;
    Ok(count > 0)
}

// ============================================================================
// User auth operations (used by AuthService; password_hash handled here)
// ============================================================================

/// Parameters for creating a new user (auth registration)
#[derive(Debug, Clone)]
pub struct CreateUserParams {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub role: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Insert a new user and return the id
pub async fn create_user(pool: &SqlitePool, params: &CreateUserParams) -> Result<String> {
    let now = now_iso8601();
    sqlx::query(
        r#"
        INSERT INTO users (id, username, email, password_hash, role, display_name, avatar_url, is_active, last_login_at, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, 1, NULL, ?, ?)
        "#,
    )
    .bind(&params.id)
    .bind(&params.username)
    .bind(&params.email)
    .bind(&params.password_hash)
    .bind(&params.role)
    .bind(&params.display_name)
    .bind(&params.avatar_url)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(params.id.clone())
}

/// Update user password and optionally updated_at
pub async fn update_user_password(
    pool: &SqlitePool,
    user_id: &str,
    password_hash: &str,
) -> Result<u64> {
    let now = now_iso8601();
    let result = sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
        .bind(password_hash)
        .bind(&now)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Update user's last login timestamp
pub async fn update_user_last_login(pool: &SqlitePool, user_id: &str) -> Result<u64> {
    let now = now_iso8601();
    let result = sqlx::query("UPDATE users SET last_login_at = ?, updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&now)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

// ============================================================================
// Refresh token operations
// ============================================================================

/// Insert a refresh token
pub async fn create_refresh_token(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    token_hash: &str,
    expires_at: &str,
    device_info: Option<&str>,
    ip_address: Option<&str>,
) -> Result<()> {
    let now = now_iso8601();
    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (id, user_id, token_hash, device_info, ip_address, expires_at, created_at, last_used_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, NULL)
        "#,
    )
    .bind(id)
    .bind(user_id)
    .bind(token_hash)
    .bind(device_info)
    .bind(ip_address)
    .bind(expires_at)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Delete a refresh token by id
pub async fn delete_refresh_token(pool: &SqlitePool, token_id: &str) -> Result<u64> {
    let result = sqlx::query("DELETE FROM refresh_tokens WHERE id = ?")
        .bind(token_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Update last_used_at for a refresh token
pub async fn update_refresh_token_used(pool: &SqlitePool, token_id: &str) -> Result<u64> {
    let now = now_iso8601();
    let result = sqlx::query("UPDATE refresh_tokens SET last_used_at = ? WHERE id = ?")
        .bind(&now)
        .bind(token_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Delete all refresh tokens for a user
pub async fn delete_user_refresh_tokens(pool: &SqlitePool, user_id: &str) -> Result<u64> {
    let result = sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Clean up expired refresh tokens; returns number deleted
pub async fn cleanup_expired_refresh_tokens(pool: &SqlitePool) -> Result<u64> {
    let result = sqlx::query("DELETE FROM refresh_tokens WHERE expires_at < datetime('now')")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

// ============================================================================
// User library access
// ============================================================================

/// Check if user has access to a library
pub async fn has_library_access(
    pool: &SqlitePool,
    user_id: &str,
    library_id: &str,
) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_library_access WHERE user_id = ? AND library_id = ?",
    )
    .bind(user_id)
    .bind(library_id)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

/// Grant library access to a user
pub async fn grant_library_access(
    pool: &SqlitePool,
    user_id: &str,
    library_id: &str,
    access_level: &str,
    granted_by: Option<&str>,
) -> Result<()> {
    let id = Uuid::new_v4().to_string();
    let now = now_iso8601();
    sqlx::query(
        r#"
        INSERT INTO user_library_access (id, user_id, library_id, access_level, granted_by, created_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(user_id)
    .bind(library_id)
    .bind(access_level)
    .bind(granted_by)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Revoke library access; returns whether a row was deleted
pub async fn revoke_library_access(
    pool: &SqlitePool,
    user_id: &str,
    library_id: &str,
) -> Result<bool> {
    let result =
        sqlx::query("DELETE FROM user_library_access WHERE user_id = ? AND library_id = ?")
            .bind(user_id)
            .bind(library_id)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() > 0)
}

// ============================================================================
// Cleanup Operations
// ============================================================================

/// Delete logs older than a given number of days
pub async fn delete_old_logs(pool: &SqlitePool, days: i32) -> Result<u64> {
    let result =
        sqlx::query("DELETE FROM app_logs WHERE timestamp < datetime('now', ? || ' days')")
            .bind(-days)
            .execute(pool)
            .await?;

    Ok(result.rows_affected())
}

/// Delete all logs
pub async fn delete_all_logs(pool: &SqlitePool) -> Result<u64> {
    let result = sqlx::query("DELETE FROM app_logs").execute(pool).await?;

    Ok(result.rows_affected())
}

/// Cleanup orphaned artwork (not linked to any entity)
pub async fn cleanup_orphaned_artwork(_pool: &SqlitePool) -> Result<u64> {
    // This would need entity-specific subqueries
    // For now, just return 0 - implement when needed
    Ok(0)
}

// ============================================================================
// Existence Checks
// ============================================================================

/// Check if a media file exists at the given path
pub async fn media_file_exists_at_path(pool: &SqlitePool, path: &str) -> Result<bool> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media_files WHERE path = ?")
        .bind(path)
        .fetch_one(pool)
        .await?;

    Ok(count > 0)
}

/// Check if a subtitle exists for a media file in a language
pub async fn subtitle_exists(
    pool: &SqlitePool,
    media_file_id: Uuid,
    language: &str,
) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subtitles WHERE media_file_id = ? AND language = ?",
    )
    .bind(uuid_to_str(media_file_id))
    .bind(language)
    .fetch_one(pool)
    .await?;

    Ok(count > 0)
}

// ============================================================================
// Batch Count Operations
// ============================================================================

/// Simple table count
pub async fn count_table(pool: &SqlitePool, table: &str) -> Result<i64> {
    // Validate table name to prevent SQL injection
    let valid_tables = [
        "movies",
        "tv_shows",
        "episodes",
        "artists",
        "albums",
        "tracks",
        "audiobooks",
        "chapters",
        "media_files",
        "torrents",
        "torrent_files",
        "libraries",
        "users",
        "notifications",
        "app_logs",
        "rss_feeds",
        "rss_feed_items",
        "pending_file_matches",
        "subtitles",
        "watch_progress",
    ];

    if !valid_tables.contains(&table) {
        anyhow::bail!("Invalid table name: {}", table);
    }

    let sql = format!("SELECT COUNT(*) FROM {}", table);
    let count: i64 = sqlx::query_scalar(&sql).fetch_one(pool).await?;

    Ok(count)
}

/// Count rows matching a simple condition
pub async fn count_where(pool: &SqlitePool, table: &str, column: &str, value: &str) -> Result<i64> {
    // Validate table name
    let valid_tables = [
        "movies",
        "tv_shows",
        "episodes",
        "artists",
        "albums",
        "tracks",
        "audiobooks",
        "chapters",
        "media_files",
        "torrents",
        "libraries",
    ];

    if !valid_tables.contains(&table) {
        anyhow::bail!("Invalid table name: {}", table);
    }

    // Validate column name (alphanumeric + underscore only)
    if !column.chars().all(|c| c.is_alphanumeric() || c == '_') {
        anyhow::bail!("Invalid column name: {}", column);
    }

    let sql = format!("SELECT COUNT(*) FROM {} WHERE {} = ?", table, column);
    let count: i64 = sqlx::query_scalar(&sql).bind(value).fetch_one(pool).await?;

    Ok(count)
}
