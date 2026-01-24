use super::prelude::*;

#[derive(Default)]
pub struct PlaybackMutations;

#[Object]
impl PlaybackMutations {
    /// Trigger mDNS device discovery scan
    async fn discover_cast_devices(&self, ctx: &Context<'_>) -> Result<Vec<CastDevice>> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        // Start discovery (it runs in background)
        cast_service
            .start_discovery()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Return current list of devices
        let devices = cast_service
            .get_devices()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(devices
            .into_iter()
            .map(|d| CastDevice::from_record(d, false))
            .collect())
    }

    /// Manually add a cast device by IP address
    async fn add_cast_device(
        &self,
        ctx: &Context<'_>,
        input: AddCastDeviceInput,
    ) -> Result<CastDeviceResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let address: std::net::IpAddr = input
            .address
            .parse()
            .map_err(|_| async_graphql::Error::new("Invalid IP address"))?;

        match cast_service
            .add_device_manual(address, input.port.map(|p| p as u16), input.name)
            .await
        {
            Ok(device) => Ok(CastDeviceResult {
                success: true,
                device: Some(CastDevice::from_record(device, false)),
                error: None,
            }),
            Err(e) => Ok(CastDeviceResult {
                success: false,
                device: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update a cast device (name, favorite status)
    async fn update_cast_device(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateCastDeviceInput,
    ) -> Result<CastDeviceResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let device_id =
            Uuid::parse_str(&id).map_err(|_| async_graphql::Error::new("Invalid device ID"))?;

        match cast_service
            .update_device(device_id, input.name, input.is_favorite)
            .await
        {
            Ok(Some(device)) => Ok(CastDeviceResult {
                success: true,
                device: Some(CastDevice::from_record(device, false)),
                error: None,
            }),
            Ok(None) => Ok(CastDeviceResult {
                success: false,
                device: None,
                error: Some("Device not found".to_string()),
            }),
            Err(e) => Ok(CastDeviceResult {
                success: false,
                device: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Remove a cast device
    async fn remove_cast_device(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let device_id =
            Uuid::parse_str(&id).map_err(|_| async_graphql::Error::new("Invalid device ID"))?;

        match cast_service.remove_device(device_id).await {
            Ok(true) => Ok(MutationResult {
                success: true,
                error: None,
            }),
            Ok(false) => Ok(MutationResult {
                success: false,
                error: Some("Device not found".to_string()),
            }),
            Err(e) => Ok(MutationResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Cast media to a device
    async fn cast_media(
        &self,
        ctx: &Context<'_>,
        input: CastMediaInput,
    ) -> Result<CastSessionResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let device_id = Uuid::parse_str(&input.device_id)
            .map_err(|_| async_graphql::Error::new("Invalid device ID"))?;
        let media_file_id = Uuid::parse_str(&input.media_file_id)
            .map_err(|_| async_graphql::Error::new("Invalid media file ID"))?;
        let episode_id = input
            .episode_id
            .as_ref()
            .map(|id| Uuid::parse_str(id))
            .transpose()
            .map_err(|_| async_graphql::Error::new("Invalid episode ID"))?;

        match cast_service
            .cast_media(device_id, media_file_id, episode_id, input.start_position)
            .await
        {
            Ok(session) => {
                let device_name = cast_service
                    .get_device(device_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|d| d.name);
                Ok(CastSessionResult {
                    success: true,
                    session: Some(CastSession::from_record(session, device_name)),
                    error: None,
                })
            }
            Err(e) => Ok(CastSessionResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Play/resume current cast session
    async fn cast_play(&self, ctx: &Context<'_>, session_id: String) -> Result<CastSessionResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let id = Uuid::parse_str(&session_id)
            .map_err(|_| async_graphql::Error::new("Invalid session ID"))?;

        match cast_service.play(id).await {
            Ok(session) => {
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
                Ok(CastSessionResult {
                    success: true,
                    session: Some(CastSession::from_record(session, device_name)),
                    error: None,
                })
            }
            Err(e) => Ok(CastSessionResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Pause current cast session
    async fn cast_pause(&self, ctx: &Context<'_>, session_id: String) -> Result<CastSessionResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let id = Uuid::parse_str(&session_id)
            .map_err(|_| async_graphql::Error::new("Invalid session ID"))?;

        match cast_service.pause(id).await {
            Ok(session) => {
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
                Ok(CastSessionResult {
                    success: true,
                    session: Some(CastSession::from_record(session, device_name)),
                    error: None,
                })
            }
            Err(e) => Ok(CastSessionResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Stop casting and end session
    async fn cast_stop(&self, ctx: &Context<'_>, session_id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let id = Uuid::parse_str(&session_id)
            .map_err(|_| async_graphql::Error::new("Invalid session ID"))?;

        match cast_service.stop(id).await {
            Ok(_) => Ok(MutationResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(MutationResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Seek to position in current cast session
    async fn cast_seek(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        position: f64,
    ) -> Result<CastSessionResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let id = Uuid::parse_str(&session_id)
            .map_err(|_| async_graphql::Error::new("Invalid session ID"))?;

        match cast_service.seek(id, position).await {
            Ok(session) => {
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
                Ok(CastSessionResult {
                    success: true,
                    session: Some(CastSession::from_record(session, device_name)),
                    error: None,
                })
            }
            Err(e) => Ok(CastSessionResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Set volume for current cast session
    async fn cast_set_volume(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        volume: f32,
    ) -> Result<CastSessionResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let id = Uuid::parse_str(&session_id)
            .map_err(|_| async_graphql::Error::new("Invalid session ID"))?;

        match cast_service.set_volume(id, volume).await {
            Ok(session) => {
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
                Ok(CastSessionResult {
                    success: true,
                    session: Some(CastSession::from_record(session, device_name)),
                    error: None,
                })
            }
            Err(e) => Ok(CastSessionResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Set mute state for current cast session
    async fn cast_set_muted(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        muted: bool,
    ) -> Result<CastSessionResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let id = Uuid::parse_str(&session_id)
            .map_err(|_| async_graphql::Error::new("Invalid session ID"))?;

        match cast_service.set_muted(id, muted).await {
            Ok(session) => {
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
                Ok(CastSessionResult {
                    success: true,
                    session: Some(CastSession::from_record(session, device_name)),
                    error: None,
                })
            }
            Err(e) => Ok(CastSessionResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update cast settings
    async fn update_cast_settings(
        &self,
        ctx: &Context<'_>,
        input: UpdateCastSettingsInput,
    ) -> Result<CastSettingsResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let db_input = crate::db::UpdateCastSettings {
            auto_discovery_enabled: input.auto_discovery_enabled,
            discovery_interval_seconds: input.discovery_interval_seconds,
            default_volume: input.default_volume,
            transcode_incompatible: input.transcode_incompatible,
            preferred_quality: input.preferred_quality,
        };

        match cast_service.update_settings(db_input).await {
            Ok(settings) => Ok(CastSettingsResult {
                success: true,
                settings: Some(CastSettings::from_record(settings)),
                error: None,
            }),
            Err(e) => Ok(CastSettingsResult {
                success: false,
                settings: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Start or resume playback of any content type
    async fn start_playback(
        &self,
        ctx: &Context<'_>,
        input: StartPlaybackInput,
    ) -> Result<PlaybackResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;
        let content_id = Uuid::parse_str(&input.content_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid content ID: {}", e)))?;
        let media_file_id = Uuid::parse_str(&input.media_file_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid media file ID: {}", e)))?;
        let parent_id = input
            .parent_id
            .as_ref()
            .map(|id| Uuid::parse_str(id))
            .transpose()
            .map_err(|e| async_graphql::Error::new(format!("Invalid parent ID: {}", e)))?;

        // Set the appropriate IDs based on content type
        let (episode_id, movie_id, track_id, audiobook_id, tv_show_id, album_id) = match input
            .content_type
        {
            PlaybackContentType::Episode => (Some(content_id), None, None, None, parent_id, None),
            PlaybackContentType::Movie => (None, Some(content_id), None, None, None, None),
            PlaybackContentType::Track => (None, None, Some(content_id), None, None, parent_id),
            PlaybackContentType::Audiobook => (None, None, None, Some(content_id), None, None),
        };

        let db_input = crate::db::UpsertPlaybackSession {
            user_id,
            content_type: input.content_type.as_str().to_string(),
            media_file_id: Some(media_file_id),
            episode_id,
            movie_id,
            track_id,
            audiobook_id,
            tv_show_id,
            album_id,
            current_position: input.start_position.unwrap_or(0.0),
            duration: input.duration,
            volume: 1.0,
            is_muted: false,
            is_playing: true,
        };

        match db.playback().upsert_session(db_input).await {
            Ok(session) => Ok(PlaybackResult {
                success: true,
                session: Some(PlaybackSession::from_record(session)),
                error: None,
            }),
            Err(e) => Ok(PlaybackResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update playback position/state
    /// Also persists watch progress to the watch_progress table for resume functionality
    async fn update_playback(
        &self,
        ctx: &Context<'_>,
        input: UpdatePlaybackInput,
    ) -> Result<PlaybackResult> {
        use crate::db::watch_progress::ContentType as WPContentType;

        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let db_input = crate::db::UpdatePlaybackPosition {
            current_position: input.current_position,
            duration: input.duration,
            volume: input.volume,
            is_muted: input.is_muted,
            is_playing: input.is_playing,
        };

        match db.playback().update_position(user_id, db_input).await {
            Ok(Some(session)) => {
                // Persist watch progress for all content types
                if let Some(position) = input.current_position {
                    // Determine content type and ID from session
                    let content_info: Option<(WPContentType, Uuid)> =
                        match session.content_type.as_deref() {
                            Some("episode") => {
                                session.episode_id.map(|id| (WPContentType::Episode, id))
                            }
                            Some("movie") => session.movie_id.map(|id| (WPContentType::Movie, id)),
                            Some("track") => session.track_id.map(|id| (WPContentType::Track, id)),
                            Some("audiobook") => session
                                .audiobook_id
                                .map(|id| (WPContentType::Audiobook, id)),
                            _ => {
                                // Fallback to old behavior for backwards compatibility
                                session.episode_id.map(|id| (WPContentType::Episode, id))
                            }
                        };

                    if let Some((content_type, content_id)) = content_info {
                        tracing::debug!(
                            "Persisting watch progress: user={}, type={:?}, content={}, position={:.1}s",
                            user_id,
                            content_type,
                            content_id,
                            position
                        );

                        let wp_input = crate::db::UpsertWatchProgress {
                            user_id,
                            content_type,
                            content_id,
                            media_file_id: session.media_file_id,
                            current_position: position,
                            duration: input.duration.or(session.duration),
                        };

                        match db.watch_progress().upsert_progress(wp_input).await {
                            Ok(wp) => tracing::debug!(
                                "Watch progress saved: content={}, progress={:.1}%, is_watched={}",
                                content_id,
                                wp.progress_percent * 100.0,
                                wp.is_watched
                            ),
                            Err(e) => tracing::warn!("Failed to persist watch progress: {}", e),
                        }
                    } else {
                        tracing::debug!("Skipping watch progress: no content ID found in session");
                    }
                }

                Ok(PlaybackResult {
                    success: true,
                    session: Some(PlaybackSession::from_record(session)),
                    error: None,
                })
            }
            Ok(None) => Ok(PlaybackResult {
                success: false,
                session: None,
                error: Some("No active playback session".to_string()),
            }),
            Err(e) => Ok(PlaybackResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Stop playback (mark session as completed)
    async fn stop_playback(&self, ctx: &Context<'_>) -> Result<PlaybackResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match db.playback().complete_session(user_id).await {
            Ok(Some(session)) => Ok(PlaybackResult {
                success: true,
                session: Some(PlaybackSession::from_record(session)),
                error: None,
            }),
            Ok(None) => Ok(PlaybackResult {
                success: true,
                session: None,
                error: None, // No active session is not an error
            }),
            Err(e) => Ok(PlaybackResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update playback settings
    async fn update_playback_settings(
        &self,
        ctx: &Context<'_>,
        input: UpdatePlaybackSettingsInput,
    ) -> Result<PlaybackSettings> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        // Get current settings
        let mut sync_interval = db
            .settings()
            .get_or_default::<i32>("playback_sync_interval", 15)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Update if provided (clamp to 5-60 seconds)
        if let Some(new_interval) = input.sync_interval_seconds {
            sync_interval = new_interval.clamp(5, 60);
            db.settings()
                .set_with_category(
                    "playback_sync_interval",
                    sync_interval,
                    "playback",
                    Some("How often to sync watch progress to the database (in seconds)"),
                )
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }

        Ok(PlaybackSettings {
            sync_interval_seconds: sync_interval,
        })
    }
}
