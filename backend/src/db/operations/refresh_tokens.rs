//! Refresh token operations.

use anyhow::Result;
use sqlx::SqlitePool;

use super::super::sqlite_helpers::now_iso8601;

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
