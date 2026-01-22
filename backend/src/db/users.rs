//! Users repository for authentication and authorization
//!
//! Handles users, library access, restrictions, invite tokens, and refresh tokens.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool as Pool;
#[cfg(feature = "postgres")]
use sqlx::PgPool as Pool;

use super::sqlite_helpers::{now_iso8601, uuid_to_str, vec_to_json, json_to_vec};

// ============================================================================
// User Records
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub role: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    pub last_login_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct CreateUser {
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub role: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateUser {
    pub email: Option<Option<String>>,
    pub password_hash: Option<String>,
    pub role: Option<String>,
    pub display_name: Option<Option<String>>,
    pub avatar_url: Option<Option<String>>,
    pub is_active: Option<bool>,
    pub last_login_at: Option<String>,
}

// ============================================================================
// Library Access Records
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLibraryAccessRecord {
    pub id: String,
    pub user_id: String,
    pub library_id: String,
    pub access_level: String,
    pub granted_by: Option<String>,
    pub created_at: String,
}

// ============================================================================
// User Restrictions Records
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRestrictionRecord {
    pub id: String,
    pub user_id: String,
    pub allowed_ratings: Vec<String>,
    pub allowed_content_types: Vec<String>,
    pub viewing_start_time: Option<String>,
    pub viewing_end_time: Option<String>,
    pub bypass_pin_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Invite Token Records
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteTokenRecord {
    pub id: String,
    pub token: String,
    pub created_by: String,
    pub library_ids: Vec<String>,
    pub role: String,
    pub access_level: String,
    pub expires_at: Option<String>,
    pub max_uses: Option<i32>,
    pub use_count: i32,
    pub apply_restrictions: bool,
    pub restrictions_template: Option<String>,
    pub is_active: bool,
    pub created_at: String,
}

// ============================================================================
// Refresh Token Records
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenRecord {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
    pub expires_at: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

// ============================================================================
// Repository
// ============================================================================

pub struct UsersRepository {
    pool: Pool,
}

impl UsersRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // User CRUD
    // ========================================================================

    /// Create a new user
    pub async fn create(&self, user: CreateUser) -> Result<UserRecord> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso8601();

        sqlx::query(
            r#"
            INSERT INTO users (id, username, email, password_hash, role, display_name, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.role)
        .bind(&user.display_name)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_by_id(&id).await?.ok_or_else(|| anyhow::anyhow!("Failed to create user"))
    }

    /// Get user by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Option<UserRecord>> {
        let row = sqlx::query_as::<_, (String, String, Option<String>, String, String, Option<String>, Option<String>, i32, Option<String>, String, String)>(
            "SELECT id, username, email, password_hash, role, display_name, avatar_url, is_active, last_login_at, created_at, updated_at FROM users WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| UserRecord {
            id: r.0,
            username: r.1,
            email: r.2,
            password_hash: r.3,
            role: r.4,
            display_name: r.5,
            avatar_url: r.6,
            is_active: r.7 != 0,
            last_login_at: r.8,
            created_at: r.9,
            updated_at: r.10,
        }))
    }

    /// Get user by username (case-insensitive)
    pub async fn get_by_username(&self, username: &str) -> Result<Option<UserRecord>> {
        let row = sqlx::query_as::<_, (String, String, Option<String>, String, String, Option<String>, Option<String>, i32, Option<String>, String, String)>(
            "SELECT id, username, email, password_hash, role, display_name, avatar_url, is_active, last_login_at, created_at, updated_at FROM users WHERE username = ? COLLATE NOCASE"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| UserRecord {
            id: r.0,
            username: r.1,
            email: r.2,
            password_hash: r.3,
            role: r.4,
            display_name: r.5,
            avatar_url: r.6,
            is_active: r.7 != 0,
            last_login_at: r.8,
            created_at: r.9,
            updated_at: r.10,
        }))
    }

    /// Get user by email (case-insensitive)
    pub async fn get_by_email(&self, email: &str) -> Result<Option<UserRecord>> {
        let row = sqlx::query_as::<_, (String, String, Option<String>, String, String, Option<String>, Option<String>, i32, Option<String>, String, String)>(
            "SELECT id, username, email, password_hash, role, display_name, avatar_url, is_active, last_login_at, created_at, updated_at FROM users WHERE email = ? COLLATE NOCASE"
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| UserRecord {
            id: r.0,
            username: r.1,
            email: r.2,
            password_hash: r.3,
            role: r.4,
            display_name: r.5,
            avatar_url: r.6,
            is_active: r.7 != 0,
            last_login_at: r.8,
            created_at: r.9,
            updated_at: r.10,
        }))
    }

    /// List all users
    pub async fn list_all(&self) -> Result<Vec<UserRecord>> {
        let rows = sqlx::query_as::<_, (String, String, Option<String>, String, String, Option<String>, Option<String>, i32, Option<String>, String, String)>(
            "SELECT id, username, email, password_hash, role, display_name, avatar_url, is_active, last_login_at, created_at, updated_at FROM users ORDER BY created_at"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| UserRecord {
            id: r.0,
            username: r.1,
            email: r.2,
            password_hash: r.3,
            role: r.4,
            display_name: r.5,
            avatar_url: r.6,
            is_active: r.7 != 0,
            last_login_at: r.8,
            created_at: r.9,
            updated_at: r.10,
        }).collect())
    }

    /// Update user
    pub async fn update(&self, id: &str, update: UpdateUser) -> Result<Option<UserRecord>> {
        let now = now_iso8601();
        let mut query = String::from("UPDATE users SET updated_at = ?");
        let mut has_update = false;

        if update.email.is_some() {
            query.push_str(", email = ?");
            has_update = true;
        }
        if update.password_hash.is_some() {
            query.push_str(", password_hash = ?");
            has_update = true;
        }
        if update.role.is_some() {
            query.push_str(", role = ?");
            has_update = true;
        }
        if update.display_name.is_some() {
            query.push_str(", display_name = ?");
            has_update = true;
        }
        if update.avatar_url.is_some() {
            query.push_str(", avatar_url = ?");
            has_update = true;
        }
        if update.is_active.is_some() {
            query.push_str(", is_active = ?");
            has_update = true;
        }
        if update.last_login_at.is_some() {
            query.push_str(", last_login_at = ?");
            has_update = true;
        }

        if !has_update {
            return self.get_by_id(id).await;
        }

        query.push_str(" WHERE id = ?");

        // Build and execute query dynamically
        // For simplicity, we'll just do individual updates
        if let Some(email) = update.email {
            sqlx::query("UPDATE users SET email = ?, updated_at = ? WHERE id = ?")
                .bind(email)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        if let Some(password_hash) = update.password_hash {
            sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
                .bind(password_hash)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        if let Some(role) = update.role {
            sqlx::query("UPDATE users SET role = ?, updated_at = ? WHERE id = ?")
                .bind(role)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        if let Some(display_name) = update.display_name {
            sqlx::query("UPDATE users SET display_name = ?, updated_at = ? WHERE id = ?")
                .bind(display_name)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        if let Some(avatar_url) = update.avatar_url {
            sqlx::query("UPDATE users SET avatar_url = ?, updated_at = ? WHERE id = ?")
                .bind(avatar_url)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        if let Some(is_active) = update.is_active {
            sqlx::query("UPDATE users SET is_active = ?, updated_at = ? WHERE id = ?")
                .bind(if is_active { 1 } else { 0 })
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        if let Some(last_login_at) = update.last_login_at {
            sqlx::query("UPDATE users SET last_login_at = ?, updated_at = ? WHERE id = ?")
                .bind(last_login_at)
                .bind(&now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        self.get_by_id(id).await
    }

    /// Update last login timestamp
    pub async fn update_last_login(&self, id: &str) -> Result<()> {
        let now = now_iso8601();
        sqlx::query("UPDATE users SET last_login_at = ?, updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete user
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Count users (for checking if this is first user setup)
    pub async fn count(&self) -> Result<i64> {
        let row = sqlx::query_as::<_, (i64,)>("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0)
    }

    /// Check if any admin exists
    pub async fn has_admin(&self) -> Result<bool> {
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM users WHERE role = 'admin' AND is_active = 1"
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0 > 0)
    }

    // ========================================================================
    // Library Access
    // ========================================================================

    /// Grant library access to a user
    pub async fn grant_library_access(
        &self,
        user_id: &str,
        library_id: &str,
        access_level: &str,
        granted_by: Option<&str>,
    ) -> Result<UserLibraryAccessRecord> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso8601();

        sqlx::query(
            r#"
            INSERT INTO user_library_access (id, user_id, library_id, access_level, granted_by, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id, library_id) DO UPDATE SET
                access_level = excluded.access_level,
                granted_by = excluded.granted_by
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(library_id)
        .bind(access_level)
        .bind(granted_by)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_library_access(user_id, library_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to grant access"))
    }

    /// Get library access for a user
    pub async fn get_library_access(
        &self,
        user_id: &str,
        library_id: &str,
    ) -> Result<Option<UserLibraryAccessRecord>> {
        let row = sqlx::query_as::<_, (String, String, String, String, Option<String>, String)>(
            "SELECT id, user_id, library_id, access_level, granted_by, created_at FROM user_library_access WHERE user_id = ? AND library_id = ?"
        )
        .bind(user_id)
        .bind(library_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| UserLibraryAccessRecord {
            id: r.0,
            user_id: r.1,
            library_id: r.2,
            access_level: r.3,
            granted_by: r.4,
            created_at: r.5,
        }))
    }

    /// List all library access for a user
    pub async fn list_user_library_access(&self, user_id: &str) -> Result<Vec<UserLibraryAccessRecord>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>, String)>(
            "SELECT id, user_id, library_id, access_level, granted_by, created_at FROM user_library_access WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| UserLibraryAccessRecord {
            id: r.0,
            user_id: r.1,
            library_id: r.2,
            access_level: r.3,
            granted_by: r.4,
            created_at: r.5,
        }).collect())
    }

    /// Check if user has access to a library (admins always have access)
    pub async fn has_library_access(&self, user_id: &str, library_id: &str) -> Result<bool> {
        // Check if user is admin first
        if let Some(user) = self.get_by_id(user_id).await? {
            if user.role == "admin" {
                return Ok(true);
            }
        }

        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM user_library_access WHERE user_id = ? AND library_id = ?"
        )
        .bind(user_id)
        .bind(library_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0 > 0)
    }

    /// Revoke library access
    pub async fn revoke_library_access(&self, user_id: &str, library_id: &str) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM user_library_access WHERE user_id = ? AND library_id = ?"
        )
        .bind(user_id)
        .bind(library_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Refresh Tokens
    // ========================================================================

    /// Create a refresh token
    pub async fn create_refresh_token(
        &self,
        user_id: &str,
        token_hash: &str,
        expires_at: &str,
        device_info: Option<&str>,
        ip_address: Option<&str>,
    ) -> Result<RefreshTokenRecord> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso8601();

        sqlx::query(
            r#"
            INSERT INTO refresh_tokens (id, user_id, token_hash, device_info, ip_address, expires_at, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(token_hash)
        .bind(device_info)
        .bind(ip_address)
        .bind(expires_at)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(RefreshTokenRecord {
            id,
            user_id: user_id.to_string(),
            token_hash: token_hash.to_string(),
            device_info: device_info.map(String::from),
            ip_address: ip_address.map(String::from),
            expires_at: expires_at.to_string(),
            created_at: now,
            last_used_at: None,
        })
    }

    /// Get refresh token by hash
    pub async fn get_refresh_token_by_hash(&self, token_hash: &str) -> Result<Option<RefreshTokenRecord>> {
        let row = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, String, String, Option<String>)>(
            "SELECT id, user_id, token_hash, device_info, ip_address, expires_at, created_at, last_used_at FROM refresh_tokens WHERE token_hash = ?"
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| RefreshTokenRecord {
            id: r.0,
            user_id: r.1,
            token_hash: r.2,
            device_info: r.3,
            ip_address: r.4,
            expires_at: r.5,
            created_at: r.6,
            last_used_at: r.7,
        }))
    }

    /// Update refresh token last used timestamp
    pub async fn update_refresh_token_used(&self, id: &str) -> Result<()> {
        let now = now_iso8601();
        sqlx::query("UPDATE refresh_tokens SET last_used_at = ? WHERE id = ?")
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete refresh token
    pub async fn delete_refresh_token(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM refresh_tokens WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Delete all refresh tokens for a user (logout all sessions)
    pub async fn delete_user_refresh_tokens(&self, user_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Clean up expired refresh tokens
    pub async fn cleanup_expired_refresh_tokens(&self) -> Result<u64> {
        let now = now_iso8601();
        let result = sqlx::query("DELETE FROM refresh_tokens WHERE expires_at < ?")
            .bind(&now)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    // ========================================================================
    // User Restrictions
    // ========================================================================

    /// Get or create restrictions for a user
    pub async fn get_user_restrictions(&self, user_id: &str) -> Result<Option<UserRestrictionRecord>> {
        let row = sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, Option<String>, String, String)>(
            "SELECT id, user_id, allowed_ratings, allowed_content_types, viewing_start_time, viewing_end_time, bypass_pin_hash, created_at, updated_at FROM user_restrictions WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| UserRestrictionRecord {
            id: r.0,
            user_id: r.1,
            allowed_ratings: json_to_vec(&r.2),
            allowed_content_types: json_to_vec(&r.3),
            viewing_start_time: r.4,
            viewing_end_time: r.5,
            bypass_pin_hash: r.6,
            created_at: r.7,
            updated_at: r.8,
        }))
    }

    /// Set user restrictions
    pub async fn set_user_restrictions(
        &self,
        user_id: &str,
        allowed_ratings: &[String],
        allowed_content_types: &[String],
        viewing_start_time: Option<&str>,
        viewing_end_time: Option<&str>,
        bypass_pin_hash: Option<&str>,
    ) -> Result<UserRestrictionRecord> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso8601();
        let ratings_json = vec_to_json(allowed_ratings);
        let types_json = vec_to_json(allowed_content_types);

        sqlx::query(
            r#"
            INSERT INTO user_restrictions (id, user_id, allowed_ratings, allowed_content_types, viewing_start_time, viewing_end_time, bypass_pin_hash, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id) DO UPDATE SET
                allowed_ratings = excluded.allowed_ratings,
                allowed_content_types = excluded.allowed_content_types,
                viewing_start_time = excluded.viewing_start_time,
                viewing_end_time = excluded.viewing_end_time,
                bypass_pin_hash = COALESCE(excluded.bypass_pin_hash, user_restrictions.bypass_pin_hash),
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(&ratings_json)
        .bind(&types_json)
        .bind(viewing_start_time)
        .bind(viewing_end_time)
        .bind(bypass_pin_hash)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        self.get_user_restrictions(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to set restrictions"))
    }

    /// Delete user restrictions
    pub async fn delete_user_restrictions(&self, user_id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM user_restrictions WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
