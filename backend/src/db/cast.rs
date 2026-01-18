//! Cast devices and sessions database repository
//!
//! Manages storage for cast devices (Chromecast, etc.) and casting sessions.

use anyhow::Result;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// Cast device record from database
#[derive(Debug, Clone, sqlx::FromRow)]
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
    pub last_seen_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Cast session record from database
#[derive(Debug, Clone, sqlx::FromRow)]
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
    pub started_at: OffsetDateTime,
    pub ended_at: Option<OffsetDateTime>,
    pub last_position: Option<f64>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Cast settings record from database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CastSettingsRecord {
    pub id: Uuid,
    pub auto_discovery_enabled: bool,
    pub discovery_interval_seconds: i32,
    pub default_volume: f32,
    pub transcode_incompatible: bool,
    pub preferred_quality: Option<String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
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
    pool: PgPool,
}

impl CastRepository {
    pub fn new(pool: PgPool) -> Self {
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
    pub async fn get_device(&self, id: Uuid) -> Result<Option<CastDeviceRecord>> {
        let record = sqlx::query_as::<_, CastDeviceRecord>(
            r#"
            SELECT id, name, address, port, model, device_type, is_favorite, is_manual,
                   last_seen_at, created_at, updated_at
            FROM cast_devices
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// Get a cast device by address
    pub async fn get_device_by_address(&self, address: &str) -> Result<Option<CastDeviceRecord>> {
        let record = sqlx::query_as::<_, CastDeviceRecord>(
            r#"
            SELECT id, name, address, port, model, device_type, is_favorite, is_manual,
                   last_seen_at, created_at, updated_at
            FROM cast_devices
            WHERE address = $1
            "#,
        )
        .bind(address)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// Create a new cast device
    pub async fn create_device(&self, input: CreateCastDevice) -> Result<CastDeviceRecord> {
        let record = sqlx::query_as::<_, CastDeviceRecord>(
            r#"
            INSERT INTO cast_devices (name, address, port, model, device_type, is_manual, last_seen_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            RETURNING id, name, address, port, model, device_type, is_favorite, is_manual,
                      last_seen_at, created_at, updated_at
            "#,
        )
        .bind(&input.name)
        .bind(input.address)
        .bind(input.port)
        .bind(&input.model)
        .bind(&input.device_type)
        .bind(input.is_manual)
        .fetch_one(&self.pool)
        .await?;
        Ok(record)
    }

    /// Upsert a cast device (update if exists by address, create if not)
    pub async fn upsert_device(&self, input: CreateCastDevice) -> Result<CastDeviceRecord> {
        let record = sqlx::query_as::<_, CastDeviceRecord>(
            r#"
            INSERT INTO cast_devices (name, address, port, model, device_type, is_manual, last_seen_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (address) DO UPDATE SET
                name = EXCLUDED.name,
                port = EXCLUDED.port,
                model = EXCLUDED.model,
                device_type = EXCLUDED.device_type,
                last_seen_at = NOW(),
                updated_at = NOW()
            RETURNING id, name, address, port, model, device_type, is_favorite, is_manual,
                      last_seen_at, created_at, updated_at
            "#,
        )
        .bind(&input.name)
        .bind(input.address)
        .bind(input.port)
        .bind(&input.model)
        .bind(&input.device_type)
        .bind(input.is_manual)
        .fetch_one(&self.pool)
        .await?;
        Ok(record)
    }

    /// Update a cast device
    pub async fn update_device(
        &self,
        id: Uuid,
        input: UpdateCastDevice,
    ) -> Result<Option<CastDeviceRecord>> {
        let record = sqlx::query_as::<_, CastDeviceRecord>(
            r#"
            UPDATE cast_devices SET
                name = COALESCE($2, name),
                is_favorite = COALESCE($3, is_favorite),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, name, address, port, model, device_type, is_favorite, is_manual,
                      last_seen_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(input.name)
        .bind(input.is_favorite)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// Update last seen timestamp for a device
    pub async fn update_device_last_seen(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE cast_devices SET last_seen_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete a cast device
    pub async fn delete_device(&self, id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM cast_devices WHERE id = $1")
            .bind(id)
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
    pub async fn get_session(&self, id: Uuid) -> Result<Option<CastSessionRecord>> {
        let record = sqlx::query_as::<_, CastSessionRecord>(
            r#"
            SELECT id, device_id, media_file_id, episode_id, stream_url, player_state,
                   current_position, duration, volume, is_muted, started_at, ended_at,
                   last_position, created_at, updated_at
            FROM cast_sessions
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// Get active session for a device
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
            WHERE device_id = $1 AND ended_at IS NULL
            ORDER BY started_at DESC
            LIMIT 1
            "#,
        )
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// Create a new cast session
    pub async fn create_session(&self, input: CreateCastSession) -> Result<CastSessionRecord> {
        let record = sqlx::query_as::<_, CastSessionRecord>(
            r#"
            INSERT INTO cast_sessions (device_id, media_file_id, episode_id, stream_url)
            VALUES ($1, $2, $3, $4)
            RETURNING id, device_id, media_file_id, episode_id, stream_url, player_state,
                      current_position, duration, volume, is_muted, started_at, ended_at,
                      last_position, created_at, updated_at
            "#,
        )
        .bind(input.device_id)
        .bind(input.media_file_id)
        .bind(input.episode_id)
        .bind(&input.stream_url)
        .fetch_one(&self.pool)
        .await?;
        Ok(record)
    }

    /// Update a cast session
    pub async fn update_session(
        &self,
        id: Uuid,
        input: UpdateCastSession,
    ) -> Result<Option<CastSessionRecord>> {
        let record = sqlx::query_as::<_, CastSessionRecord>(
            r#"
            UPDATE cast_sessions SET
                player_state = COALESCE($2, player_state),
                current_position = COALESCE($3, current_position),
                duration = COALESCE($4, duration),
                volume = COALESCE($5, volume),
                is_muted = COALESCE($6, is_muted),
                last_position = COALESCE($3, last_position),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, device_id, media_file_id, episode_id, stream_url, player_state,
                      current_position, duration, volume, is_muted, started_at, ended_at,
                      last_position, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(input.player_state)
        .bind(input.current_position)
        .bind(input.duration)
        .bind(input.volume)
        .bind(input.is_muted)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// End a cast session
    pub async fn end_session(&self, id: Uuid) -> Result<Option<CastSessionRecord>> {
        let record = sqlx::query_as::<_, CastSessionRecord>(
            r#"
            UPDATE cast_sessions SET
                ended_at = NOW(),
                player_state = 'idle',
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, device_id, media_file_id, episode_id, stream_url, player_state,
                      current_position, duration, volume, is_muted, started_at, ended_at,
                      last_position, created_at, updated_at
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(record)
    }

    /// End all active sessions for a device
    pub async fn end_sessions_for_device(&self, device_id: Uuid) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE cast_sessions SET
                ended_at = NOW(),
                player_state = 'idle',
                updated_at = NOW()
            WHERE device_id = $1 AND ended_at IS NULL
            "#,
        )
        .bind(device_id)
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
    pub async fn update_settings(&self, input: UpdateCastSettings) -> Result<CastSettingsRecord> {
        // First ensure settings exist
        let existing = self.get_settings().await?;

        if existing.is_none() {
            // Create default settings if not exists
            sqlx::query(
                r#"
                INSERT INTO cast_settings (auto_discovery_enabled, discovery_interval_seconds, default_volume, transcode_incompatible, preferred_quality)
                VALUES (true, 30, 1.0, true, '1080p')
                "#
            )
            .execute(&self.pool)
            .await?;
        }

        let record = sqlx::query_as::<_, CastSettingsRecord>(
            r#"
            UPDATE cast_settings SET
                auto_discovery_enabled = COALESCE($1, auto_discovery_enabled),
                discovery_interval_seconds = COALESCE($2, discovery_interval_seconds),
                default_volume = COALESCE($3, default_volume),
                transcode_incompatible = COALESCE($4, transcode_incompatible),
                preferred_quality = COALESCE($5, preferred_quality),
                updated_at = NOW()
            RETURNING id, auto_discovery_enabled, discovery_interval_seconds, default_volume,
                      transcode_incompatible, preferred_quality, created_at, updated_at
            "#,
        )
        .bind(input.auto_discovery_enabled)
        .bind(input.discovery_interval_seconds)
        .bind(input.default_volume)
        .bind(input.transcode_incompatible)
        .bind(input.preferred_quality)
        .fetch_one(&self.pool)
        .await?;
        Ok(record)
    }
}
