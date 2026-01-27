//! User library access operations.

use anyhow::Result;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::super::sqlite_helpers::now_iso8601;

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
