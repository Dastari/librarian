//! Usenet servers database repository
//!
//! Handles CRUD operations for Usenet NNTP server configurations.

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

/// Usenet server record from database
#[derive(Debug, Clone, FromRow)]
pub struct UsenetServerRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub use_ssl: bool,
    pub username: Option<String>,
    pub encrypted_password: Option<String>,
    pub password_nonce: Option<String>,
    pub connections: i32,
    pub priority: i32,
    pub enabled: bool,
    pub retention_days: Option<i32>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub error_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a new usenet server
#[derive(Debug, Clone)]
pub struct CreateUsenetServer {
    pub user_id: Uuid,
    pub name: String,
    pub host: String,
    pub port: i32,
    pub use_ssl: bool,
    pub username: Option<String>,
    pub encrypted_password: Option<String>,
    pub password_nonce: Option<String>,
    pub connections: i32,
    pub priority: i32,
    pub retention_days: Option<i32>,
}

/// Data for updating a usenet server
#[derive(Debug, Clone, Default)]
pub struct UpdateUsenetServer {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub use_ssl: Option<bool>,
    pub username: Option<String>,
    pub encrypted_password: Option<String>,
    pub password_nonce: Option<String>,
    pub connections: Option<i32>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
    pub retention_days: Option<i32>,
}

/// Usenet servers database repository
pub struct UsenetServersRepository {
    pool: PgPool,
}

impl UsenetServersRepository {
    /// Create a new repository instance
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get a usenet server by ID
    pub async fn get(&self, id: Uuid) -> Result<Option<UsenetServerRecord>> {
        let record = sqlx::query_as::<_, UsenetServerRecord>(
            r#"
            SELECT id, user_id, name, host, port, use_ssl, username,
                   encrypted_password, password_nonce, connections, priority,
                   enabled, retention_days, last_success_at, last_error,
                   error_count, created_at, updated_at
            FROM usenet_servers
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get all usenet servers for a user
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<UsenetServerRecord>> {
        let records = sqlx::query_as::<_, UsenetServerRecord>(
            r#"
            SELECT id, user_id, name, host, port, use_ssl, username,
                   encrypted_password, password_nonce, connections, priority,
                   enabled, retention_days, last_success_at, last_error,
                   error_count, created_at, updated_at
            FROM usenet_servers
            WHERE user_id = $1
            ORDER BY priority ASC, name ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get enabled usenet servers for a user, ordered by priority
    pub async fn list_enabled_by_user(&self, user_id: Uuid) -> Result<Vec<UsenetServerRecord>> {
        let records = sqlx::query_as::<_, UsenetServerRecord>(
            r#"
            SELECT id, user_id, name, host, port, use_ssl, username,
                   encrypted_password, password_nonce, connections, priority,
                   enabled, retention_days, last_success_at, last_error,
                   error_count, created_at, updated_at
            FROM usenet_servers
            WHERE user_id = $1 AND enabled = true
            ORDER BY priority ASC, name ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Create a new usenet server
    pub async fn create(&self, data: CreateUsenetServer) -> Result<UsenetServerRecord> {
        let record = sqlx::query_as::<_, UsenetServerRecord>(
            r#"
            INSERT INTO usenet_servers (
                user_id, name, host, port, use_ssl, username,
                encrypted_password, password_nonce, connections, priority, retention_days
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, user_id, name, host, port, use_ssl, username,
                      encrypted_password, password_nonce, connections, priority,
                      enabled, retention_days, last_success_at, last_error,
                      error_count, created_at, updated_at
            "#,
        )
        .bind(data.user_id)
        .bind(&data.name)
        .bind(&data.host)
        .bind(data.port)
        .bind(data.use_ssl)
        .bind(&data.username)
        .bind(&data.encrypted_password)
        .bind(&data.password_nonce)
        .bind(data.connections)
        .bind(data.priority)
        .bind(data.retention_days)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Update a usenet server
    pub async fn update(&self, id: Uuid, data: UpdateUsenetServer) -> Result<UsenetServerRecord> {
        let record = sqlx::query_as::<_, UsenetServerRecord>(
            r#"
            UPDATE usenet_servers
            SET name = COALESCE($2, name),
                host = COALESCE($3, host),
                port = COALESCE($4, port),
                use_ssl = COALESCE($5, use_ssl),
                username = COALESCE($6, username),
                encrypted_password = COALESCE($7, encrypted_password),
                password_nonce = COALESCE($8, password_nonce),
                connections = COALESCE($9, connections),
                priority = COALESCE($10, priority),
                enabled = COALESCE($11, enabled),
                retention_days = COALESCE($12, retention_days),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, user_id, name, host, port, use_ssl, username,
                      encrypted_password, password_nonce, connections, priority,
                      enabled, retention_days, last_success_at, last_error,
                      error_count, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(&data.host)
        .bind(data.port)
        .bind(data.use_ssl)
        .bind(&data.username)
        .bind(&data.encrypted_password)
        .bind(&data.password_nonce)
        .bind(data.connections)
        .bind(data.priority)
        .bind(data.enabled)
        .bind(data.retention_days)
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    /// Delete a usenet server
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM usenet_servers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Record a successful connection
    pub async fn record_success(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE usenet_servers
            SET last_success_at = NOW(),
                error_count = 0,
                last_error = NULL,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Record a connection error
    pub async fn record_error(&self, id: Uuid, error: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE usenet_servers
            SET last_error = $2,
                error_count = error_count + 1,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Reorder servers by setting their priorities
    pub async fn reorder(&self, user_id: Uuid, server_ids: &[Uuid]) -> Result<()> {
        for (idx, server_id) in server_ids.iter().enumerate() {
            sqlx::query(
                r#"
                UPDATE usenet_servers
                SET priority = $3, updated_at = NOW()
                WHERE id = $1 AND user_id = $2
                "#,
            )
            .bind(server_id)
            .bind(user_id)
            .bind(idx as i32)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }
}
