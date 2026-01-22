//! Usenet servers database repository
//!
//! Handles CRUD operations for Usenet NNTP server configurations.

use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "postgres")]
type DbPool = PgPool;
#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

#[cfg(feature = "sqlite")]
use crate::db::sqlite_helpers::{
    bool_to_int, int_to_bool, str_to_datetime, str_to_datetime_opt, str_to_uuid, str_to_uuid_opt,
    uuid_to_str,
};

/// Usenet server record from database
#[derive(Debug, Clone)]
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

#[cfg(feature = "postgres")]
impl sqlx::FromRow<'_, sqlx::postgres::PgRow> for UsenetServerRecord {
    fn from_row(row: &sqlx::postgres::PgRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Self {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            name: row.try_get("name")?,
            host: row.try_get("host")?,
            port: row.try_get("port")?,
            use_ssl: row.try_get("use_ssl")?,
            username: row.try_get("username")?,
            encrypted_password: row.try_get("encrypted_password")?,
            password_nonce: row.try_get("password_nonce")?,
            connections: row.try_get("connections")?,
            priority: row.try_get("priority")?,
            enabled: row.try_get("enabled")?,
            retention_days: row.try_get("retention_days")?,
            last_success_at: row.try_get("last_success_at")?,
            last_error: row.try_get("last_error")?,
            error_count: row.try_get("error_count")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }
}

#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for UsenetServerRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let user_id_str: String = row.try_get("user_id")?;
        let use_ssl: i32 = row.try_get("use_ssl")?;
        let enabled: i32 = row.try_get("enabled")?;
        let last_success_str: Option<String> = row.try_get("last_success_at")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            user_id: str_to_uuid(&user_id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            name: row.try_get("name")?,
            host: row.try_get("host")?,
            port: row.try_get("port")?,
            use_ssl: int_to_bool(use_ssl),
            username: row.try_get("username")?,
            encrypted_password: row.try_get("encrypted_password")?,
            password_nonce: row.try_get("password_nonce")?,
            connections: row.try_get("connections")?,
            priority: row.try_get("priority")?,
            enabled: int_to_bool(enabled),
            retention_days: row.try_get("retention_days")?,
            last_success_at: str_to_datetime_opt(last_success_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            last_error: row.try_get("last_error")?,
            error_count: row.try_get("error_count")?,
            created_at: str_to_datetime(&created_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
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
    pool: DbPool,
}

impl UsenetServersRepository {
    /// Create a new repository instance
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get a usenet server by ID
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn get(&self, id: Uuid) -> Result<Option<UsenetServerRecord>> {
        let record = sqlx::query_as::<_, UsenetServerRecord>(
            r#"
            SELECT id, user_id, name, host, port, use_ssl, username,
                   encrypted_password, password_nonce, connections, priority,
                   enabled, retention_days, last_success_at, last_error,
                   error_count, created_at, updated_at
            FROM usenet_servers
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    /// Get all usenet servers for a user
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<UsenetServerRecord>> {
        let records = sqlx::query_as::<_, UsenetServerRecord>(
            r#"
            SELECT id, user_id, name, host, port, use_ssl, username,
                   encrypted_password, password_nonce, connections, priority,
                   enabled, retention_days, last_success_at, last_error,
                   error_count, created_at, updated_at
            FROM usenet_servers
            WHERE user_id = ?1
            ORDER BY priority ASC, name ASC
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get enabled usenet servers for a user, ordered by priority
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn list_enabled_by_user(&self, user_id: Uuid) -> Result<Vec<UsenetServerRecord>> {
        let records = sqlx::query_as::<_, UsenetServerRecord>(
            r#"
            SELECT id, user_id, name, host, port, use_ssl, username,
                   encrypted_password, password_nonce, connections, priority,
                   enabled, retention_days, last_success_at, last_error,
                   error_count, created_at, updated_at
            FROM usenet_servers
            WHERE user_id = ?1 AND enabled = 1
            ORDER BY priority ASC, name ASC
            "#,
        )
        .bind(uuid_to_str(user_id))
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Create a new usenet server
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn create(&self, data: CreateUsenetServer) -> Result<UsenetServerRecord> {
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            INSERT INTO usenet_servers (
                id, user_id, name, host, port, use_ssl, username,
                encrypted_password, password_nonce, connections, priority, retention_days,
                enabled, error_count, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1, 0, datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(uuid_to_str(data.user_id))
        .bind(&data.name)
        .bind(&data.host)
        .bind(data.port)
        .bind(bool_to_int(data.use_ssl))
        .bind(&data.username)
        .bind(&data.encrypted_password)
        .bind(&data.password_nonce)
        .bind(data.connections)
        .bind(data.priority)
        .bind(data.retention_days)
        .execute(&self.pool)
        .await?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve usenet server after insert"))
    }

    /// Update a usenet server
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn update(&self, id: Uuid, data: UpdateUsenetServer) -> Result<UsenetServerRecord> {
        let id_str = uuid_to_str(id);

        // Build dynamic update query based on what's provided
        let mut updates = vec!["updated_at = datetime('now')".to_string()];
        let mut param_idx = 2;

        if data.name.is_some() {
            updates.push(format!("name = ?{}", param_idx));
            param_idx += 1;
        }
        if data.host.is_some() {
            updates.push(format!("host = ?{}", param_idx));
            param_idx += 1;
        }
        if data.port.is_some() {
            updates.push(format!("port = ?{}", param_idx));
            param_idx += 1;
        }
        if data.use_ssl.is_some() {
            updates.push(format!("use_ssl = ?{}", param_idx));
            param_idx += 1;
        }
        if data.username.is_some() {
            updates.push(format!("username = ?{}", param_idx));
            param_idx += 1;
        }
        if data.encrypted_password.is_some() {
            updates.push(format!("encrypted_password = ?{}", param_idx));
            param_idx += 1;
        }
        if data.password_nonce.is_some() {
            updates.push(format!("password_nonce = ?{}", param_idx));
            param_idx += 1;
        }
        if data.connections.is_some() {
            updates.push(format!("connections = ?{}", param_idx));
            param_idx += 1;
        }
        if data.priority.is_some() {
            updates.push(format!("priority = ?{}", param_idx));
            param_idx += 1;
        }
        if data.enabled.is_some() {
            updates.push(format!("enabled = ?{}", param_idx));
            param_idx += 1;
        }
        if data.retention_days.is_some() {
            updates.push(format!("retention_days = ?{}", param_idx));
        }

        let query = format!(
            "UPDATE usenet_servers SET {} WHERE id = ?1",
            updates.join(", ")
        );

        let mut q = sqlx::query(&query).bind(&id_str);

        if let Some(ref name) = data.name {
            q = q.bind(name);
        }
        if let Some(ref host) = data.host {
            q = q.bind(host);
        }
        if let Some(port) = data.port {
            q = q.bind(port);
        }
        if let Some(use_ssl) = data.use_ssl {
            q = q.bind(bool_to_int(use_ssl));
        }
        if let Some(ref username) = data.username {
            q = q.bind(username);
        }
        if let Some(ref encrypted_password) = data.encrypted_password {
            q = q.bind(encrypted_password);
        }
        if let Some(ref password_nonce) = data.password_nonce {
            q = q.bind(password_nonce);
        }
        if let Some(connections) = data.connections {
            q = q.bind(connections);
        }
        if let Some(priority) = data.priority {
            q = q.bind(priority);
        }
        if let Some(enabled) = data.enabled {
            q = q.bind(bool_to_int(enabled));
        }
        if let Some(retention_days) = data.retention_days {
            q = q.bind(retention_days);
        }

        q.execute(&self.pool).await?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve usenet server after update"))
    }

    /// Delete a usenet server
    #[cfg(feature = "postgres")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM usenet_servers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    #[cfg(feature = "sqlite")]
    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM usenet_servers WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Record a successful connection
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn record_success(&self, id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE usenet_servers
            SET last_success_at = datetime('now'),
                error_count = 0,
                last_error = NULL,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Record a connection error
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn record_error(&self, id: Uuid, error: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE usenet_servers
            SET last_error = ?2,
                error_count = error_count + 1,
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .bind(error)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Reorder servers by setting their priorities
    #[cfg(feature = "postgres")]
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

    #[cfg(feature = "sqlite")]
    pub async fn reorder(&self, user_id: Uuid, server_ids: &[Uuid]) -> Result<()> {
        for (idx, server_id) in server_ids.iter().enumerate() {
            sqlx::query(
                r#"
                UPDATE usenet_servers
                SET priority = ?3, updated_at = datetime('now')
                WHERE id = ?1 AND user_id = ?2
                "#,
            )
            .bind(uuid_to_str(*server_id))
            .bind(uuid_to_str(user_id))
            .bind(idx as i32)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }
}
