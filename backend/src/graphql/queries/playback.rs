use super::prelude::*;

#[derive(Default)]
pub struct PlaybackQueries;

#[Object]
impl PlaybackQueries {
    /// Get all discovered and saved cast devices
    async fn cast_devices(&self, ctx: &Context<'_>) -> Result<Vec<CastDevice>> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let devices = cast_service
            .get_devices()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // TODO: Track connected state per device
        Ok(devices
            .into_iter()
            .map(|d| CastDevice::from_record(d, false))
            .collect())
    }

    /// Get a specific cast device by ID
    async fn cast_device(&self, ctx: &Context<'_>, id: String) -> Result<Option<CastDevice>> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let device_id =
            Uuid::parse_str(&id).map_err(|_| async_graphql::Error::new("Invalid device ID"))?;

        let device = cast_service
            .get_device(device_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(device.map(|d| CastDevice::from_record(d, false)))
    }

    /// Get all active cast sessions
    async fn cast_sessions(&self, ctx: &Context<'_>) -> Result<Vec<CastSession>> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let sessions = cast_service
            .get_active_sessions()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Get device names for each session
        let mut results = Vec::new();
        for session in sessions {
            let device_name = if let Some(device_id) = session.device_id {
                cast_service
                    .get_device(device_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|d| d.name)
            } else {
                None
            };
            results.push(CastSession::from_record(session, device_name));
        }

        Ok(results)
    }

    /// Get a specific cast session by ID
    async fn cast_session(&self, ctx: &Context<'_>, id: String) -> Result<Option<CastSession>> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let session_id =
            Uuid::parse_str(&id).map_err(|_| async_graphql::Error::new("Invalid session ID"))?;

        let session = cast_service
            .get_session(session_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if let Some(session) = session {
            let device_name = if let Some(device_id) = session.device_id {
                cast_service
                    .get_device(device_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|d| d.name)
            } else {
                None
            };
            Ok(Some(CastSession::from_record(session, device_name)))
        } else {
            Ok(None)
        }
    }

    /// Get cast settings
    async fn cast_settings(&self, ctx: &Context<'_>) -> Result<Option<CastSettings>> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let settings = cast_service
            .get_settings()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(settings.map(CastSettings::from_record))
    }

    /// Get the current user's active playback session
    async fn playback_session(&self, ctx: &Context<'_>) -> Result<Option<PlaybackSession>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let session = db
            .playback()
            .get_active_session(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(session.map(PlaybackSession::from_record))
    }

    /// Get playback settings (sync interval, etc.)
    async fn playback_settings(&self, ctx: &Context<'_>) -> Result<PlaybackSettings> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let sync_interval = db
            .settings()
            .get_or_default::<i32>("playback_sync_interval", 15)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(PlaybackSettings {
            sync_interval_seconds: sync_interval,
        })
    }
}
