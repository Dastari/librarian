//! Cast devices and sessions database repository
//!
//! Manages storage for cast devices (Chromecast, etc.) and casting sessions.

use anyhow::Result;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use sqlx::SqlitePool;

#[cfg(feature = "sqlite")]
type DbPool = SqlitePool;

#[cfg(feature = "sqlite")]
use crate::db::sqlite_helpers::{
    bool_to_int, int_to_bool, str_to_datetime, str_to_datetime_opt, str_to_uuid, str_to_uuid_opt,
    uuid_to_str,
};

/// Cast device record from database
#[derive(Debug, Clone)]
pub struct CastDeviceRecord {
    pub id: Uuid,
    pub name: String,
    /// IP address stored as string (e.g., "192.168.1.100")
    pub address: String,
    pub port: i32,
    pub model: Option<String>,
    pub device_type: String,
    pub is_favorite: bool,
    pub is_manual: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for CastDeviceRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let is_favorite: i32 = row.try_get("is_favorite")?;
        let is_manual: i32 = row.try_get("is_manual")?;
        let last_seen_str: Option<String> = row.try_get("last_seen_at")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            name: row.try_get("name")?,
            address: row.try_get("address")?,
            port: row.try_get("port")?,
            model: row.try_get("model")?,
            device_type: row.try_get("device_type")?,
            is_favorite: int_to_bool(is_favorite),
            is_manual: int_to_bool(is_manual),
            last_seen_at: str_to_datetime_opt(last_seen_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            created_at: str_to_datetime(&created_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// Cast session record from database
#[derive(Debug, Clone)]
pub struct CastSessionRecord {
    pub id: Uuid,
    pub device_id: Option<Uuid>,
    pub media_file_id: Option<Uuid>,
    pub episode_id: Option<Uuid>,
    pub stream_url: String,
    pub player_state: String,
    pub current_position: f64,
    pub duration: Option<f64>,
    pub volume: f32,
    pub is_muted: bool,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub last_position: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for CastSessionRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let device_id_str: Option<String> = row.try_get("device_id")?;
        let media_file_id_str: Option<String> = row.try_get("media_file_id")?;
        let episode_id_str: Option<String> = row.try_get("episode_id")?;
        let is_muted: i32 = row.try_get("is_muted")?;
        let started_str: String = row.try_get("started_at")?;
        let ended_str: Option<String> = row.try_get("ended_at")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            device_id: str_to_uuid_opt(device_id_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            media_file_id: str_to_uuid_opt(media_file_id_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            episode_id: str_to_uuid_opt(episode_id_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            stream_url: row.try_get("stream_url")?,
            player_state: row.try_get("player_state")?,
            current_position: row.try_get("current_position")?,
            duration: row.try_get("duration")?,
            volume: row.try_get("volume")?,
            is_muted: int_to_bool(is_muted),
            started_at: str_to_datetime(&started_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            ended_at: str_to_datetime_opt(ended_str.as_deref())
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            last_position: row.try_get("last_position")?,
            created_at: str_to_datetime(&created_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// Cast settings record from database
#[derive(Debug, Clone)]
pub struct CastSettingsRecord {
    pub id: Uuid,
    pub auto_discovery_enabled: bool,
    pub discovery_interval_seconds: i32,
    pub default_volume: f32,
    pub transcode_incompatible: bool,
    pub preferred_quality: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


#[cfg(feature = "sqlite")]
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for CastSettingsRecord {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let auto_discovery: i32 = row.try_get("auto_discovery_enabled")?;
        let transcode: i32 = row.try_get("transcode_incompatible")?;
        let created_str: String = row.try_get("created_at")?;
        let updated_str: String = row.try_get("updated_at")?;

        Ok(Self {
            id: str_to_uuid(&id_str).map_err(|e| sqlx::Error::Decode(e.into()))?,
            auto_discovery_enabled: int_to_bool(auto_discovery),
            discovery_interval_seconds: row.try_get("discovery_interval_seconds")?,
            default_volume: row.try_get("default_volume")?,
            transcode_incompatible: int_to_bool(transcode),
            preferred_quality: row.try_get("preferred_quality")?,
            created_at: str_to_datetime(&created_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
            updated_at: str_to_datetime(&updated_str)
                .map_err(|e| sqlx::Error::Decode(e.into()))?,
        })
    }
}

/// Input for creating a cast device
#[derive(Debug)]
pub struct CreateCastDevice {
    pub name: String,
    /// IP address as string (e.g., "192.168.1.100")
    pub address: String,
    pub port: i32,
    pub model: Option<String>,
    pub device_type: String,
    pub is_manual: bool,
}

/// Input for updating a cast device
#[derive(Debug, Default)]
pub struct UpdateCastDevice {
    pub name: Option<String>,
    pub is_favorite: Option<bool>,
}

/// Input for creating a cast session
#[derive(Debug)]
pub struct CreateCastSession {
    pub device_id: Uuid,
    pub media_file_id: Option<Uuid>,
    pub episode_id: Option<Uuid>,
    pub stream_url: String,
}

/// Input for updating a cast session
#[derive(Debug, Default)]
pub struct UpdateCastSession {
    pub player_state: Option<String>,
    pub current_position: Option<f64>,
    pub duration: Option<f64>,
    pub volume: Option<f32>,
    pub is_muted: Option<bool>,
}

/// Input for updating cast settings
#[derive(Debug, Default)]
pub struct UpdateCastSettings {
    pub auto_discovery_enabled: Option<bool>,
    pub discovery_interval_seconds: Option<i32>,
    pub default_volume: Option<f32>,
    pub transcode_incompatible: Option<bool>,
    pub preferred_quality: Option<String>,
}

pub struct CastRepository {
    pool: DbPool,
}

impl CastRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Cast Devices
    // ========================================================================

    /// List all cast devices
    pub async fn list_devices(&self) -> Result<Vec<CastDeviceRecord>> {
        let records = sqlx::query_as::<_, CastDeviceRecord>(
            r#"
            SELECT id, name, address, port, model, device_type, is_favorite, is_manual,
                   last_seen_at, created_at, updated_at
            FROM cast_devices
            ORDER BY is_favorite DESC, name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Get a cast device by ID

    #[cfg(feature = "sqlite")]
    pub async fn get_device(&self, id: Uuid) -> Result<Option<CastDeviceRecord>> {
        let record = sqlx::query_as::<_, CastDeviceRecord>(
            r#"
            SELECT id, name, address, port, model, device_type, is_favorite, is_manual,
                   last_seen_at, created_at, updated_at
            FROM cast_devices
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// Get a cast device by address

    #[cfg(feature = "sqlite")]
    pub async fn get_device_by_address(&self, address: &str) -> Result<Option<CastDeviceRecord>> {
        let record = sqlx::query_as::<_, CastDeviceRecord>(
            r#"
            SELECT id, name, address, port, model, device_type, is_favorite, is_manual,
                   last_seen_at, created_at, updated_at
            FROM cast_devices
            WHERE address = ?1
            "#,
        )
        .bind(address)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// Create a new cast device

    #[cfg(feature = "sqlite")]
    pub async fn create_device(&self, input: CreateCastDevice) -> Result<CastDeviceRecord> {
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            INSERT INTO cast_devices (id, name, address, port, model, device_type, is_manual, is_favorite, last_seen_at, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, datetime('now'), datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(&input.name)
        .bind(&input.address)
        .bind(input.port)
        .bind(&input.model)
        .bind(&input.device_type)
        .bind(bool_to_int(input.is_manual))
        .execute(&self.pool)
        .await?;

        self.get_device(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve device after insert"))
    }

    /// Upsert a cast device (update if exists by address, create if not)

    #[cfg(feature = "sqlite")]
    pub async fn upsert_device(&self, input: CreateCastDevice) -> Result<CastDeviceRecord> {
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            INSERT INTO cast_devices (id, name, address, port, model, device_type, is_manual, is_favorite, last_seen_at, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, datetime('now'), datetime('now'), datetime('now'))
            ON CONFLICT (address) DO UPDATE SET
                name = excluded.name,
                port = excluded.port,
                model = excluded.model,
                device_type = excluded.device_type,
                last_seen_at = datetime('now'),
                updated_at = datetime('now')
            "#,
        )
        .bind(&id_str)
        .bind(&input.name)
        .bind(&input.address)
        .bind(input.port)
        .bind(&input.model)
        .bind(&input.device_type)
        .bind(bool_to_int(input.is_manual))
        .execute(&self.pool)
        .await?;

        self.get_device_by_address(&input.address)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve device after upsert"))
    }

    /// Update a cast device

    #[cfg(feature = "sqlite")]
    pub async fn update_device(
        &self,
        id: Uuid,
        input: UpdateCastDevice,
    ) -> Result<Option<CastDeviceRecord>> {
        let id_str = uuid_to_str(id);

        // For SQLite, we need to handle COALESCE with optional bools differently
        // Build the update query dynamically based on what's provided
        let mut updates = vec!["updated_at = datetime('now')".to_string()];
        let mut param_idx = 2;

        if input.name.is_some() {
            updates.push(format!("name = ?{}", param_idx));
            param_idx += 1;
        }
        if input.is_favorite.is_some() {
            updates.push(format!("is_favorite = ?{}", param_idx));
        }

        let query = format!(
            "UPDATE cast_devices SET {} WHERE id = ?1",
            updates.join(", ")
        );

        let mut q = sqlx::query(&query).bind(&id_str);

        if let Some(ref name) = input.name {
            q = q.bind(name);
        }
        if let Some(is_favorite) = input.is_favorite {
            q = q.bind(bool_to_int(is_favorite));
        }

        let result = q.execute(&self.pool).await?;

        if result.rows_affected() > 0 {
            self.get_device(id).await
        } else {
            Ok(None)
        }
    }

    /// Update last seen timestamp for a device

    #[cfg(feature = "sqlite")]
    pub async fn update_device_last_seen(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE cast_devices SET last_seen_at = datetime('now') WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete a cast device

    #[cfg(feature = "sqlite")]
    pub async fn delete_device(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM cast_devices WHERE id = ?1")
            .bind(uuid_to_str(id))
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // ========================================================================
    // Cast Sessions
    // ========================================================================

    /// List all active cast sessions
    pub async fn list_active_sessions(&self) -> Result<Vec<CastSessionRecord>> {
        let records = sqlx::query_as::<_, CastSessionRecord>(
            r#"
            SELECT id, device_id, media_file_id, episode_id, stream_url, player_state,
                   current_position, duration, volume, is_muted, started_at, ended_at,
                   last_position, created_at, updated_at
            FROM cast_sessions
            WHERE ended_at IS NULL
            ORDER BY started_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    /// Get a cast session by ID

    #[cfg(feature = "sqlite")]
    pub async fn get_session(&self, id: Uuid) -> Result<Option<CastSessionRecord>> {
        let record = sqlx::query_as::<_, CastSessionRecord>(
            r#"
            SELECT id, device_id, media_file_id, episode_id, stream_url, player_state,
                   current_position, duration, volume, is_muted, started_at, ended_at,
                   last_position, created_at, updated_at
            FROM cast_sessions
            WHERE id = ?1
            "#,
        )
        .bind(uuid_to_str(id))
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// Get active session for a device

    #[cfg(feature = "sqlite")]
    pub async fn get_active_session_for_device(
        &self,
        device_id: Uuid,
    ) -> Result<Option<CastSessionRecord>> {
        let record = sqlx::query_as::<_, CastSessionRecord>(
            r#"
            SELECT id, device_id, media_file_id, episode_id, stream_url, player_state,
                   current_position, duration, volume, is_muted, started_at, ended_at,
                   last_position, created_at, updated_at
            FROM cast_sessions
            WHERE device_id = ?1 AND ended_at IS NULL
            ORDER BY started_at DESC
            LIMIT 1
            "#,
        )
        .bind(uuid_to_str(device_id))
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// Create a new cast session

    #[cfg(feature = "sqlite")]
    pub async fn create_session(&self, input: CreateCastSession) -> Result<CastSessionRecord> {
        let id = Uuid::new_v4();
        let id_str = uuid_to_str(id);

        sqlx::query(
            r#"
            INSERT INTO cast_sessions (id, device_id, media_file_id, episode_id, stream_url, player_state,
                                        current_position, duration, volume, is_muted, started_at, ended_at,
                                        last_position, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, 'idle', 0.0, NULL, 1.0, 0, datetime('now'), NULL, NULL, datetime('now'), datetime('now'))
            "#,
        )
        .bind(&id_str)
        .bind(uuid_to_str(input.device_id))
        .bind(input.media_file_id.map(uuid_to_str))
        .bind(input.episode_id.map(uuid_to_str))
        .bind(&input.stream_url)
        .execute(&self.pool)
        .await?;

        self.get_session(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve session after insert"))
    }

    /// Update a cast session

    #[cfg(feature = "sqlite")]
    pub async fn update_session(
        &self,
        id: Uuid,
        input: UpdateCastSession,
    ) -> Result<Option<CastSessionRecord>> {
        let id_str = uuid_to_str(id);

        // Build dynamic update query
        let mut updates = vec!["updated_at = datetime('now')".to_string()];
        let mut param_idx = 2;

        if input.player_state.is_some() {
            updates.push(format!("player_state = ?{}", param_idx));
            param_idx += 1;
        }
        if input.current_position.is_some() {
            updates.push(format!("current_position = ?{}", param_idx));
            updates.push(format!("last_position = ?{}", param_idx));
            param_idx += 1;
        }
        if input.duration.is_some() {
            updates.push(format!("duration = ?{}", param_idx));
            param_idx += 1;
        }
        if input.volume.is_some() {
            updates.push(format!("volume = ?{}", param_idx));
            param_idx += 1;
        }
        if input.is_muted.is_some() {
            updates.push(format!("is_muted = ?{}", param_idx));
        }

        let query = format!(
            "UPDATE cast_sessions SET {} WHERE id = ?1",
            updates.join(", ")
        );

        let mut q = sqlx::query(&query).bind(&id_str);

        if let Some(ref player_state) = input.player_state {
            q = q.bind(player_state);
        }
        if let Some(current_position) = input.current_position {
            q = q.bind(current_position);
        }
        if let Some(duration) = input.duration {
            q = q.bind(duration);
        }
        if let Some(volume) = input.volume {
            q = q.bind(volume);
        }
        if let Some(is_muted) = input.is_muted {
            q = q.bind(bool_to_int(is_muted));
        }

        let result = q.execute(&self.pool).await?;

        if result.rows_affected() > 0 {
            self.get_session(id).await
        } else {
            Ok(None)
        }
    }

    /// End a cast session

    #[cfg(feature = "sqlite")]
    pub async fn end_session(&self, id: Uuid) -> Result<Option<CastSessionRecord>> {
        let id_str = uuid_to_str(id);

        let result = sqlx::query(
            r#"
            UPDATE cast_sessions SET
                ended_at = datetime('now'),
                player_state = 'idle',
                updated_at = datetime('now')
            WHERE id = ?1
            "#,
        )
        .bind(&id_str)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() > 0 {
            self.get_session(id).await
        } else {
            Ok(None)
        }
    }

    /// End all active sessions for a device

    #[cfg(feature = "sqlite")]
    pub async fn end_sessions_for_device(&self, device_id: Uuid) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE cast_sessions SET
                ended_at = datetime('now'),
                player_state = 'idle',
                updated_at = datetime('now')
            WHERE device_id = ?1 AND ended_at IS NULL
            "#,
        )
        .bind(uuid_to_str(device_id))
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    // ========================================================================
    // Cast Settings
    // ========================================================================

    /// Get cast settings (singleton)
    pub async fn get_settings(&self) -> Result<Option<CastSettingsRecord>> {
        let record = sqlx::query_as::<_, CastSettingsRecord>(
            r#"
            SELECT id, auto_discovery_enabled, discovery_interval_seconds, default_volume,
                   transcode_incompatible, preferred_quality, created_at, updated_at
            FROM cast_settings
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// Update cast settings

    #[cfg(feature = "sqlite")]
    pub async fn update_settings(&self, input: UpdateCastSettings) -> Result<CastSettingsRecord> {
        // First ensure settings exist
        let existing = self.get_settings().await?;

        if existing.is_none() {
            // Create default settings if not exists
            let id = uuid_to_str(Uuid::new_v4());
            sqlx::query(
                r#"
                INSERT INTO cast_settings (id, auto_discovery_enabled, discovery_interval_seconds, default_volume, transcode_incompatible, preferred_quality, created_at, updated_at)
                VALUES (?1, 1, 30, 1.0, 1, '1080p', datetime('now'), datetime('now'))
                "#
            )
            .bind(&id)
            .execute(&self.pool)
            .await?;
        }

        // Build dynamic update query
        let mut updates = vec!["updated_at = datetime('now')".to_string()];
        let mut param_idx = 1;

        if input.auto_discovery_enabled.is_some() {
            updates.push(format!("auto_discovery_enabled = ?{}", param_idx));
            param_idx += 1;
        }
        if input.discovery_interval_seconds.is_some() {
            updates.push(format!("discovery_interval_seconds = ?{}", param_idx));
            param_idx += 1;
        }
        if input.default_volume.is_some() {
            updates.push(format!("default_volume = ?{}", param_idx));
            param_idx += 1;
        }
        if input.transcode_incompatible.is_some() {
            updates.push(format!("transcode_incompatible = ?{}", param_idx));
            param_idx += 1;
        }
        if input.preferred_quality.is_some() {
            updates.push(format!("preferred_quality = ?{}", param_idx));
        }

        let query = format!("UPDATE cast_settings SET {}", updates.join(", "));

        let mut q = sqlx::query(&query);

        if let Some(auto_discovery) = input.auto_discovery_enabled {
            q = q.bind(bool_to_int(auto_discovery));
        }
        if let Some(interval) = input.discovery_interval_seconds {
            q = q.bind(interval);
        }
        if let Some(volume) = input.default_volume {
            q = q.bind(volume);
        }
        if let Some(transcode) = input.transcode_incompatible {
            q = q.bind(bool_to_int(transcode));
        }
        if let Some(ref quality) = input.preferred_quality {
            q = q.bind(quality);
        }

        q.execute(&self.pool).await?;

        self.get_settings()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve settings after update"))
    }
}
