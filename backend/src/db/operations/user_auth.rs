//! User auth operations (used by AuthService; password_hash handled here).

use anyhow::Result;
use sqlx::SqlitePool;

use super::super::sqlite_helpers::now_iso8601;

/// Check if any admin user exists
pub async fn has_admin_user(pool: &SqlitePool) -> Result<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin' AND is_active = 1")
            .fetch_one(pool)
            .await?;
    Ok(count > 0)
}

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
