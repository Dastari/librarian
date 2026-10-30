use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Request, Result, Variables};
use graphql_orm::graphql::filters::{DateFilter, StringFilter};
use graphql_orm::graphql::orm::{MutationContext, OrderDirection, TransactionMode};
use serde::Deserialize;

use super::super::auth::{AuthExt, AuthUser, Role};
use super::super::session_authority::SessionAuthority;
use crate::graphql::entities::{
    CastDevice, CastSession, CreateCastSessionInput, CreatePlaybackProgressInput, Library,
    MediaFile, PlaybackProgress, PlaybackProgressOrderByInput, PlaybackProgressWhereInput,
    RefreshToken, RefreshTokenWhereInput, UpdateCastSessionInput, UpdatePlaybackProgressInput,
    User,
};
use crate::services::ServicesManager;
use crate::services::cast::{CastDeviceType, CastGrantClaims, CastPlaybackMode};

#[cfg(test)]
#[path = "cast_tests.rs"]
mod tests;

#[derive(Clone, Debug, async_graphql::SimpleObject)]
pub struct LegacyCastDevice {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: i32,
    pub model: Option<String>,
    pub device_type: String,
    pub is_favorite: bool,
    pub is_manual: bool,
    pub is_connected: bool,
    pub enabled: bool,
    pub playback_supported: bool,
    pub discovery_origin: Option<String>,
    pub last_seen_at: Option<String>,
}

#[derive(Clone, Debug, async_graphql::SimpleObject)]
pub struct LegacyCastSession {
    pub id: String,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub media_file_id: Option<String>,
    pub episode_id: Option<String>,
    pub player_state: String,
    pub current_time: f64,
    pub duration: Option<f64>,
    pub volume: f64,
    pub is_muted: bool,
    pub started_at: String,
    pub last_error: Option<String>,
    pub playback_decision: Option<String>,
    pub playback_reason: Option<String>,
}

#[derive(Clone, Debug, async_graphql::SimpleObject)]
pub struct CastSessionOperationResult {
    pub success: bool,
    pub session: Option<LegacyCastSession>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, async_graphql::SimpleObject)]
pub struct CastDeviceOperationResult {
    pub success: bool,
    pub device: Option<LegacyCastDevice>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, async_graphql::SimpleObject)]
pub struct LegacyCastSettings {
    pub auto_discovery_enabled: bool,
    pub discovery_interval_seconds: i32,
    pub default_volume: f64,
    pub transcode_incompatible: bool,
    pub preferred_quality: Option<String>,
}

#[derive(Clone, Debug, async_graphql::SimpleObject)]
pub struct CastSettingsOperationResult {
    pub success: bool,
    pub settings: Option<LegacyCastSettings>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, async_graphql::SimpleObject)]
pub struct CastActionResult {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(InputObject, Clone, Debug)]
#[graphql(name = "CastMediaInput")]
pub struct CastMediaInput {
    pub device_id: String,
    pub media_file_id: String,
    pub episode_id: Option<String>,
    pub start_position: Option<f64>,
}

#[derive(InputObject, Clone, Debug)]
#[graphql(name = "LegacyAddCastDeviceInput")]
pub struct AddCastDeviceInput {
    pub address: String,
    pub port: Option<i32>,
    pub name: Option<String>,
}

#[derive(InputObject, Clone, Debug)]
#[graphql(name = "LegacyUpdateCastDeviceInput")]
pub struct UpdateCastDeviceInput {
    pub name: Option<String>,
    pub address: Option<String>,
    pub port: Option<i32>,
    pub model: Option<String>,
    pub device_type: Option<String>,
    pub is_favorite: Option<bool>,
    pub is_manual: Option<bool>,
}

#[derive(InputObject, Clone, Debug)]
#[graphql(name = "LegacyUpdateCastSettingsInput")]
pub struct UpdateCastSettingsInput {
    pub auto_discovery_enabled: Option<bool>,
    pub discovery_interval_seconds: Option<i32>,
    pub default_volume: Option<f64>,
    pub transcode_incompatible: Option<bool>,
    pub preferred_quality: Option<String>,
}

#[derive(Default)]
pub struct CastMutations;

#[Object]
impl CastMutations {
    #[graphql(name = "discoverCastDevices")]
    async fn discover_cast_devices(&self, ctx: &Context<'_>) -> Result<Vec<LegacyCastDevice>> {
        let auth_user = ctx.require_admin()?.clone();
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let cast = manager
            .get_cast()
            .await
            .ok_or_else(|| async_graphql::Error::new("Cast service not available"))?;
        let discovered = cast
            .discover_now()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let mut out = Vec::new();
        for device in discovered {
            if let Err(error) =
                crate::services::cast::CastService::validate_target(&device.address, device.port)
            {
                tracing::warn!(
                    cast_address = %device.address,
                    cast_port = device.port,
                    error = %error,
                    "Ignoring discovered Cast target outside the allowed LAN ranges"
                );
                continue;
            }
            let node = upsert_discovered_device(manager, &auth_user, &device).await?;
            out.push(map_device(node));
        }

        Ok(out)
    }

    #[graphql(name = "castMedia")]
    async fn cast_media(
        &self,
        ctx: &Context<'_>,
        input: CastMediaInput,
    ) -> Result<CastSessionOperationResult> {
        let auth_user = ctx.require_member()?.clone();
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let issuer = ctx.data::<SessionAuthority>()?;
        let cast = manager
            .get_cast()
            .await
            .ok_or_else(|| async_graphql::Error::new("Cast service not available"))?;
        if issuer.0.user_id != auth_user.user_id {
            return Err(async_graphql::Error::new(
                "Cast issuing authority does not match the actor",
            ));
        }
        let auth = manager
            .get_auth()
            .await
            .ok_or_else(|| async_graphql::Error::new("Authentication service not available"))?;
        auth.validate_session_authority(&issuer.0)
            .await
            .map_err(|_| {
                async_graphql::Error::new("Cast issuing session is no longer authorized")
            })?;
        if !AuthUser::from(issuer.0.clone()).has_role(Role::Member) {
            return Err(async_graphql::Error::new(
                "Member issuing authority required",
            ));
        }
        cast.validate_advertised_media_url()
            .map_err(|error| async_graphql::Error::new(error.to_string()))?;

        let device = match query_device_by_id(manager, &auth_user, &input.device_id).await? {
            Some(device) => device,
            None => {
                return Ok(CastSessionOperationResult {
                    success: false,
                    session: None,
                    error: Some("Cast device not found".to_string()),
                });
            }
        };

        let (_, receiver_port) =
            crate::services::cast::CastService::validate_target(&device.address, device.port)
                .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        let operation = cast
            .lock_session_operations(&device.address, receiver_port)
            .await
            .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        // Admission may have waited for a detached SDK worker. Recheck the
        // actual caller after the wait, before preparing replacement playback.
        validate_cast_caller(manager, &auth_user, issuer)
            .await
            .map_err(|_| {
                async_graphql::Error::new("Cast issuing session is no longer authorized")
            })?;

        let media_file = match query_media_file(manager, &auth_user, &input.media_file_id).await? {
            Some(media_file) => media_file,
            None => {
                return Ok(CastSessionOperationResult {
                    success: false,
                    session: None,
                    error: Some("Media file not found".to_string()),
                });
            }
        };

        {
            let Some(library_id) = media_file.library_id.as_ref() else {
                return Ok(CastSessionOperationResult {
                    success: false,
                    session: None,
                    error: Some("Media file is not available to this account".to_string()),
                });
            };
            let database = manager
                .get_database()
                .await
                .ok_or_else(|| async_graphql::Error::new("Database service not available"))?;
            let owned = Library::get(database.pool().pool(), library_id)
                .await
                .map_err(|error| async_graphql::Error::new(error.to_string()))?
                .is_some_and(|library| library.user_id == auth_user.user_id);
            if !owned {
                return Ok(CastSessionOperationResult {
                    success: false,
                    session: None,
                    error: Some("Media file is not available to this account".to_string()),
                });
            }
        }

        if device.enabled == Some(false) {
            return Ok(CastSessionOperationResult {
                success: false,
                session: None,
                error: Some("This Cast device is disabled".to_string()),
            });
        }
        let device_type = parse_device_type(&device.device_type);
        if !matches!(
            device_type,
            CastDeviceType::Chromecast | CastDeviceType::ChromecastAudio
        ) || device.playback_supported == Some(false)
        {
            return Ok(CastSessionOperationResult {
                success: false,
                session: None,
                error: Some(
                    "Playback is not supported for this discovered receiver type".to_string(),
                ),
            });
        }
        let (receiver_address, receiver_port) =
            crate::services::cast::CastService::validate_target(&device.address, device.port)
                .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        let start_position = crate::services::cast::CastService::validate_position(
            input.start_position.unwrap_or(0.0),
        )
        .map_err(|error| async_graphql::Error::new(error.to_string()))?;

        let settings = query_cast_playback_settings(manager, &auth_user).await?;
        let decision = crate::services::cast::CastService::decide_playback(
            device_type,
            device.model.as_deref(),
            media_file.container.as_deref(),
            media_file.video_codec.as_deref(),
            media_file.audio_codec.as_deref(),
            media_file.height,
            media_file.is_hdr,
            settings.transcode_incompatible,
            settings.preferred_quality.as_deref(),
        );
        if decision.mode == CastPlaybackMode::Unsupported {
            return Ok(CastSessionOperationResult {
                success: false,
                session: None,
                error: Some(decision.reason.to_string()),
            });
        }
        let (stream_path, content_type) = match decision.mode {
            CastPlaybackMode::Direct => (
                format!("/api/media/{}/stream", input.media_file_id),
                crate::services::cast::CastService::infer_content_type(
                    &media_file.path,
                    media_file.content_type.as_deref(),
                ),
            ),
            CastPlaybackMode::Remux | CastPlaybackMode::Transcode => (
                format!("/api/media/{}/hls/playlist.m3u8", input.media_file_id),
                "application/vnd.apple.mpegurl".to_string(),
            ),
            CastPlaybackMode::Unsupported => {
                return Err(async_graphql::Error::new(
                    "Unsupported Cast playback decision",
                ));
            }
        };
        tracing::info!(
            cast_device_id = %input.device_id,
            media_file_id = %input.media_file_id,
            playback_decision = decision.mode.as_str(),
            playback_reason = decision.reason,
            "Selected Cast playback path"
        );
        let default_volume = settings.default_volume;
        let default_muted = settings.default_muted;
        let expires_at_unix = CastGrantClaims::expiry_for_duration(
            chrono::Utc::now().timestamp(),
            media_file.duration,
        );
        let expires_at = chrono::DateTime::from_timestamp(expires_at_unix, 0)
            .ok_or_else(|| async_graphql::Error::new("Invalid Cast grant expiration"))?;
        for previous in
            cast.release_receiver(&device.id, &receiver_address.to_string(), receiver_port)
        {
            retire_cast_monitor(manager, &previous).await;
        }
        let session = create_cast_session(
            manager,
            CreateCastSessionArgs {
                user_id: Some(auth_user.user_id.clone()),
                device_id: Some(input.device_id.clone()),
                media_file_id: Some(input.media_file_id.clone()),
                episode_id: input.episode_id.clone(),
                stream_url: stream_path,
                receiver_address: Some(receiver_address.to_string()),
                grant_expires_at: Some(expires_at.to_rfc3339()),
                receiver_transport_id: None,
                receiver_session_id: None,
                media_session_id: None,
                last_error: None,
                playback_decision: Some(decision.mode.as_str().to_string()),
                playback_reason: Some(decision.reason.to_string()),
                player_state: "STARTING".to_string(),
                current_position: start_position,
                duration: media_file
                    .duration
                    .filter(|duration| *duration > 0)
                    .map(f64::from),
                volume: default_volume,
                is_muted: default_muted,
            },
        )
        .await?;
        let claims = CastGrantClaims::new(
            session.id.clone(),
            input.media_file_id.clone(),
            auth_user.user_id.clone(),
            receiver_address.to_string(),
            expires_at.timestamp(),
            issuer.0.session_id.to_string(),
            issuer.0.roles.clone(),
            input.device_id.clone(),
            receiver_port,
        );
        validate_cast_grant_authority(manager, &claims)
            .await
            .map_err(|_| async_graphql::Error::new("Cast authority changed before launch"))?;
        let grant = cast
            .mint_grant(&claims)
            .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        let mut stream_url = format!(
            "{}{}?castGrant={}",
            cast.media_base_url().trim_end_matches('/'),
            session.stream_url,
            urlencoding::encode(&grant)
        );
        if matches!(
            decision.mode,
            CastPlaybackMode::Remux | CastPlaybackMode::Transcode
        ) {
            stream_url.push_str("&castMode=");
            stream_url.push_str(if decision.mode == CastPlaybackMode::Remux {
                "remux"
            } else {
                "transcode"
            });
        }

        let launch = match cast
            .cast_media(
                &operation,
                &receiver_address.to_string(),
                receiver_port,
                &stream_url,
                &content_type,
                start_position,
            )
            .await
        {
            Ok(launch) => launch,
            Err(_err) => {
                let correlation_id = uuid::Uuid::new_v4().to_string();
                tracing::error!(
                    correlation_id,
                    cast_session_id = %session.id,
                    cast_device_id = %input.device_id,
                    "Cast media launch failed"
                );
                let _ = update_cast_session(
                    manager,
                    &session.id,
                    CastSessionPatch {
                        player_state: Some("FAILED".to_string()),
                        ended_at: Some(Some(chrono::Utc::now().to_rfc3339())),
                        last_position: Some(Some(start_position)),
                        last_error: Some(Some(format!(
                            "Cast launch failed (reference {correlation_id})"
                        ))),
                        ..Default::default()
                    },
                )
                .await;
                return Ok(CastSessionOperationResult {
                    success: false,
                    session: None,
                    error: Some(format!(
                        "Unable to start playback on this receiver (reference {correlation_id})"
                    )),
                });
            }
        };

        let default_error = match cast
            .set_volume(
                &operation,
                &receiver_address.to_string(),
                receiver_port,
                default_volume as f32,
            )
            .await
        {
            Ok(()) if default_muted => cast
                .set_muted(
                    &operation,
                    &receiver_address.to_string(),
                    receiver_port,
                    true,
                )
                .await
                .err(),
            Ok(()) => None,
            Err(error) => Some(error),
        };
        if default_error.is_some() {
            tracing::warn!(
                cast_session_id = %session.id,
                "Cast playback started but receiver defaults could not be applied"
            );
        }
        let monitor_transport_id = launch.receiver_transport_id.clone();
        let monitor_receiver_session_id = launch.receiver_session_id.clone();
        let monitor_media_session_id = launch.media_session_id;
        let session = update_cast_session(
            manager,
            &session.id,
            CastSessionPatch {
                player_state: Some("PLAYING".to_string()),
                duration: launch.duration.map(Some),
                receiver_transport_id: Some(Some(launch.receiver_transport_id)),
                receiver_session_id: Some(Some(launch.receiver_session_id)),
                media_session_id: Some(Some(launch.media_session_id)),
                last_error: Some(default_error.map(|_| {
                    "Playback started, but receiver volume defaults could not be applied."
                        .to_string()
                })),
                ..Default::default()
            },
        )
        .await?;
        cast.manage_session(
            claims.clone(),
            monitor_transport_id.clone(),
            monitor_receiver_session_id,
            monitor_media_session_id,
        );
        let monitor_cast = Arc::clone(&cast);
        let monitor_manager = Arc::clone(manager);
        let monitor_session_id = session.id.clone();
        let monitor_receiver_address = receiver_address.to_string();
        cast.install_session_monitor(
            monitor_session_id.clone(),
            monitor_cast_session(
                monitor_manager,
                monitor_cast,
                monitor_session_id,
                monitor_receiver_address,
                receiver_port,
                monitor_transport_id,
                monitor_media_session_id,
                claims,
            ),
        )
        .await;

        Ok(CastSessionOperationResult {
            success: true,
            session: Some(map_session(session, Some(device.name))),
            error: None,
        })
    }

    #[graphql(name = "castPlay")]
    async fn cast_play(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<CastSessionOperationResult> {
        update_cast_session_state(ctx, &session_id, SessionCommand::Play).await
    }

    #[graphql(name = "castPause")]
    async fn cast_pause(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<CastSessionOperationResult> {
        update_cast_session_state(ctx, &session_id, SessionCommand::Pause).await
    }

    #[graphql(name = "castStop")]
    async fn cast_stop(&self, ctx: &Context<'_>, session_id: String) -> Result<CastActionResult> {
        update_cast_session_state(ctx, &session_id, SessionCommand::Stop)
            .await
            .map(|result| CastActionResult {
                success: result.success,
                error: result.error,
            })
    }

    #[graphql(name = "castSeek")]
    async fn cast_seek(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        position: f64,
    ) -> Result<CastSessionOperationResult> {
        update_cast_session_state(ctx, &session_id, SessionCommand::Seek(position)).await
    }

    #[graphql(name = "castSetVolume")]
    async fn cast_set_volume(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        volume: f64,
    ) -> Result<CastSessionOperationResult> {
        update_cast_session_state(ctx, &session_id, SessionCommand::SetVolume(volume)).await
    }

    #[graphql(name = "castSetMuted")]
    async fn cast_set_muted(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        muted: bool,
    ) -> Result<CastSessionOperationResult> {
        update_cast_session_state(ctx, &session_id, SessionCommand::SetMuted(muted)).await
    }

    #[graphql(name = "addCastDevice")]
    async fn add_cast_device(
        &self,
        ctx: &Context<'_>,
        input: AddCastDeviceInput,
    ) -> Result<CastDeviceOperationResult> {
        let auth_user = ctx.require_admin()?.clone();
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        crate::services::cast::CastService::validate_target(
            &input.address,
            input.port.unwrap_or(8009),
        )
        .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        let name = input
            .name
            .clone()
            .unwrap_or_else(|| format!("Cast Device ({})", input.address));
        let node = create_cast_device(
            manager,
            &auth_user,
            serde_json::json!({
                "name": name,
                "address": input.address,
                "port": input.port.unwrap_or(8009),
                "model": null,
                "deviceType": "CHROMECAST",
                "isFavorite": false,
                "isManual": true,
                "enabled": true,
                "discoveryOrigin": "MANUAL",
                "playbackSupported": true,
                "firstSeenAt": chrono::Utc::now().to_rfc3339(),
                "lastSeenAt": null,
            }),
        )
        .await?;

        Ok(CastDeviceOperationResult {
            success: true,
            device: Some(map_device(node)),
            error: None,
        })
    }

    #[graphql(name = "updateCastDevice")]
    async fn update_cast_device(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateCastDeviceInput,
    ) -> Result<CastDeviceOperationResult> {
        let auth_user = ctx.require_admin()?.clone();
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let Some(existing) = query_device_by_id(manager, &auth_user, &id).await? else {
            return Ok(CastDeviceOperationResult {
                success: false,
                device: None,
                error: Some("Cast device not found".to_string()),
            });
        };
        let address = input
            .address
            .clone()
            .unwrap_or_else(|| existing.address.clone());
        let port = input.port.unwrap_or(existing.port);
        crate::services::cast::CastService::validate_target(&address, port)
            .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        let device_type = input
            .device_type
            .clone()
            .unwrap_or_else(|| existing.device_type.clone());
        if parse_device_type(&device_type) == CastDeviceType::Unknown {
            return Err(async_graphql::Error::new(
                "Unsupported Cast device type; expected CHROMECAST, CHROMECAST_AUDIO, or DLNA_RENDERER",
            ));
        }

        let node = update_cast_device(
            manager,
            &auth_user,
            &id,
            serde_json::json!({
                "name": input.name.unwrap_or(existing.name),
                "address": address,
                "port": port,
                "model": input.model.or(existing.model),
                "deviceType": device_type,
                "isFavorite": input.is_favorite.unwrap_or(existing.is_favorite),
                "isManual": input.is_manual.unwrap_or(existing.is_manual),
            }),
        )
        .await?;

        Ok(CastDeviceOperationResult {
            success: true,
            device: Some(map_device(node)),
            error: None,
        })
    }

    #[graphql(name = "removeCastDevice")]
    async fn remove_cast_device(&self, ctx: &Context<'_>, id: String) -> Result<CastActionResult> {
        let auth_user = ctx.require_admin()?.clone();
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        delete_cast_device(manager, &auth_user, &id).await?;
        Ok(CastActionResult {
            success: true,
            error: None,
        })
    }

    #[graphql(name = "updateCastSettings")]
    async fn update_cast_settings(
        &self,
        ctx: &Context<'_>,
        input: UpdateCastSettingsInput,
    ) -> Result<CastSettingsOperationResult> {
        let auth_user = ctx.require_admin()?.clone();
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        if let Some(interval) = input.discovery_interval_seconds
            && !(5..=86_400).contains(&interval)
        {
            return Err(async_graphql::Error::new(
                "Discovery interval must be between 5 and 86400 seconds",
            ));
        }
        if let Some(volume) = input.default_volume {
            crate::services::cast::CastService::validate_volume(volume)
                .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        }
        if let Some(quality) = input.preferred_quality.as_deref()
            && !matches!(
                quality.trim().to_ascii_lowercase().as_str(),
                "original" | "2160p" | "1080p" | "720p" | "480p"
            )
        {
            return Err(async_graphql::Error::new(
                "Preferred quality must be original, 2160p, 1080p, 720p, or 480p",
            ));
        }

        let existing = query_latest_cast_setting(manager, &auth_user).await?;
        let node = match existing {
            Some(setting) => {
                update_cast_setting(
                    manager,
                    &auth_user,
                    &setting.id,
                    serde_json::json!({
                        "autoDiscoveryEnabled": input.auto_discovery_enabled.unwrap_or(setting.auto_discovery_enabled),
                        "discoveryIntervalSeconds": input.discovery_interval_seconds.unwrap_or(setting.discovery_interval_seconds),
                        "defaultVolume": input.default_volume.unwrap_or(setting.default_volume),
                        "transcodeIncompatible": input.transcode_incompatible.unwrap_or(setting.transcode_incompatible),
                        "preferredQuality": input.preferred_quality.or(setting.preferred_quality),
                    }),
                )
                .await?
            }
            None => {
                create_cast_setting(
                    manager,
                    &auth_user,
                    serde_json::json!({
                        "autoDiscoveryEnabled": input.auto_discovery_enabled.unwrap_or(true),
                        "discoveryIntervalSeconds": input.discovery_interval_seconds.unwrap_or(30),
                        "defaultVolume": input.default_volume.unwrap_or(1.0),
                        "transcodeIncompatible": input.transcode_incompatible.unwrap_or(false),
                        "preferredQuality": input.preferred_quality,
                    }),
                )
                .await?
            }
        };
        let cast = manager
            .get_cast()
            .await
            .ok_or_else(|| async_graphql::Error::new("Cast service not available"))?;
        cast.apply_runtime_settings(
            node.auto_discovery_enabled,
            u64::try_from(node.discovery_interval_seconds).map_err(|_| {
                async_graphql::Error::new("Discovery interval must be a positive number")
            })?,
        )
        .await
        .map_err(|error| async_graphql::Error::new(error.to_string()))?;

        Ok(CastSettingsOperationResult {
            success: true,
            settings: Some(map_settings(node)),
            error: None,
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
struct DeviceNode {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Address")]
    address: String,
    #[serde(rename = "Port")]
    port: i32,
    #[serde(rename = "Model")]
    model: Option<String>,
    #[serde(rename = "DeviceType")]
    device_type: String,
    #[serde(rename = "IsFavorite")]
    is_favorite: bool,
    #[serde(rename = "IsManual")]
    is_manual: bool,
    #[serde(rename = "Enabled")]
    enabled: Option<bool>,
    #[serde(rename = "PlaybackSupported")]
    playback_supported: Option<bool>,
    #[serde(rename = "DiscoveryOrigin")]
    discovery_origin: Option<String>,
    #[serde(rename = "LastSeenAt")]
    last_seen_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct SessionNode {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "DeviceId")]
    device_id: Option<String>,
    #[serde(rename = "MediaFileId")]
    media_file_id: Option<String>,
    #[serde(rename = "EpisodeId")]
    episode_id: Option<String>,
    #[serde(skip)]
    stream_url: String,
    #[serde(rename = "LastError")]
    last_error: Option<String>,
    #[serde(rename = "PlaybackDecision")]
    playback_decision: Option<String>,
    #[serde(rename = "PlaybackReason")]
    playback_reason: Option<String>,
    #[serde(rename = "PlayerState")]
    player_state: String,
    #[serde(rename = "CurrentPosition")]
    current_position: f64,
    #[serde(rename = "Duration")]
    duration: Option<f64>,
    #[serde(rename = "Volume")]
    volume: f64,
    #[serde(rename = "IsMuted")]
    is_muted: bool,
    #[serde(rename = "StartedAt")]
    started_at: String,
}

#[derive(Clone, Debug, Deserialize)]
struct SettingNode {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "AutoDiscoveryEnabled")]
    auto_discovery_enabled: bool,
    #[serde(rename = "DiscoveryIntervalSeconds")]
    discovery_interval_seconds: i32,
    #[serde(rename = "DefaultVolume")]
    default_volume: f64,
    #[serde(rename = "TranscodeIncompatible")]
    transcode_incompatible: bool,
    #[serde(rename = "PreferredQuality")]
    preferred_quality: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct MediaFileNode {
    #[serde(rename = "Path")]
    path: String,
    #[serde(rename = "Duration")]
    duration: Option<i32>,
    #[serde(rename = "ContentType")]
    content_type: Option<String>,
    #[serde(rename = "LibraryId")]
    library_id: Option<String>,
    #[serde(rename = "Container")]
    container: Option<String>,
    #[serde(rename = "VideoCodec")]
    video_codec: Option<String>,
    #[serde(rename = "AudioCodec")]
    audio_codec: Option<String>,
    #[serde(rename = "Height")]
    height: Option<i32>,
    #[serde(rename = "IsHdr")]
    is_hdr: bool,
}

#[derive(Clone, Debug)]
struct CreateCastSessionArgs {
    user_id: Option<String>,
    device_id: Option<String>,
    media_file_id: Option<String>,
    episode_id: Option<String>,
    stream_url: String,
    receiver_address: Option<String>,
    grant_expires_at: Option<String>,
    receiver_transport_id: Option<String>,
    receiver_session_id: Option<String>,
    media_session_id: Option<i32>,
    last_error: Option<String>,
    playback_decision: Option<String>,
    playback_reason: Option<String>,
    player_state: String,
    current_position: f64,
    duration: Option<f64>,
    volume: f64,
    is_muted: bool,
}

#[derive(Clone, Debug, Default)]
struct CastSessionPatch {
    receiver_transport_id: Option<Option<String>>,
    receiver_session_id: Option<Option<String>>,
    media_session_id: Option<Option<i32>>,
    last_error: Option<Option<String>>,
    playback_decision: Option<Option<String>>,
    playback_reason: Option<Option<String>>,
    player_state: Option<String>,
    current_position: Option<f64>,
    duration: Option<Option<f64>>,
    volume: Option<f64>,
    is_muted: Option<bool>,
    ended_at: Option<Option<String>>,
    last_position: Option<Option<f64>>,
}

enum SessionCommand {
    Stop,
    Play,
    Pause,
    Seek(f64),
    SetVolume(f64),
    SetMuted(bool),
}

async fn update_cast_session_state(
    ctx: &Context<'_>,
    session_id: &str,
    command: SessionCommand,
) -> Result<CastSessionOperationResult> {
    let auth_user = ctx.require_member()?.clone();
    let manager = ctx.data::<Arc<ServicesManager>>()?;
    let caller = ctx.data::<SessionAuthority>()?;
    control_cast_session(manager, &auth_user, caller, session_id, command)
        .await
        .map_err(|error| async_graphql::Error::new(error.to_string()))
}

/// Admission uses the original server-managed grant. Historical rows are never
/// sufficient authority, even when their old receiver IDs are still present.
async fn admit_managed_cast_session(
    manager: &Arc<ServicesManager>,
    cast: &crate::services::cast::CastService,
    session_id: &str,
) -> anyhow::Result<(
    crate::services::cast::service::ManagedCastSession,
    CastSession,
)> {
    let managed = cast
        .managed_session(session_id)
        .ok_or_else(|| anyhow::anyhow!("Cast session is no longer managed"))?;
    let session = validate_cast_grant_authority(manager, &managed.claims).await?;
    validate_managed_receiver(&managed, &session)?;
    Ok((managed, session))
}

fn validate_managed_receiver(
    managed: &crate::services::cast::service::ManagedCastSession,
    session: &CastSession,
) -> anyhow::Result<()> {
    if session.receiver_transport_id.as_deref() != Some(managed.transport_id.as_str())
        || session.receiver_session_id.as_deref() != Some(managed.receiver_session_id.as_str())
        || session.media_session_id != Some(managed.media_session_id)
    {
        anyhow::bail!("Cast receiver session binding changed");
    }
    Ok(())
}

async fn validate_cast_caller(
    manager: &Arc<ServicesManager>,
    actor: &AuthUser,
    caller: &SessionAuthority,
) -> anyhow::Result<()> {
    let authenticated = AuthUser::from(caller.0.clone());
    if actor.user_id != authenticated.user_id
        || !authenticated.has_role(Role::Member)
        || actor.is_admin() != authenticated.is_admin()
    {
        anyhow::bail!("Cast caller authority does not match the actor");
    }
    let auth = manager
        .get_auth()
        .await
        .ok_or_else(|| anyhow::anyhow!("Authentication service not available"))?;
    auth.validate_session_authority(&caller.0).await
}

async fn control_cast_session(
    manager: &Arc<ServicesManager>,
    actor: &AuthUser,
    caller: &SessionAuthority,
    session_id: &str,
    command: SessionCommand,
) -> anyhow::Result<CastSessionOperationResult> {
    let cast = manager
        .get_cast()
        .await
        .ok_or_else(|| anyhow::anyhow!("Cast service not available"))?;
    let binding = cast
        .managed_session(session_id)
        .ok_or_else(|| anyhow::anyhow!("Cast session is no longer managed"))?;
    let operation = cast
        .lock_session_operations(
            &binding.claims.receiver_address,
            binding.claims.receiver_port,
        )
        .await?;
    validate_cast_caller(manager, actor, caller).await?;
    let (managed, session) = admit_managed_cast_session(manager, &cast, session_id).await?;
    if managed.claims != binding.claims {
        anyhow::bail!("Cast managed session changed while awaiting admission");
    }
    if !actor.is_admin() && session.user_id.as_deref() != Some(actor.user_id.as_str()) {
        anyhow::bail!("Cast session not found");
    }
    let claims = &managed.claims;
    let device_name = query_device_by_id(manager, actor, &claims.receiver_device_id)
        .await
        .map_err(|error| anyhow::anyhow!(error.message))?
        .map(|device| device.name);
    let mut status = crate::services::cast::CastRemoteStatus {
        player_state: session.player_state.clone(),
        finished: false,
        current_position: session.current_position,
        duration: session.duration,
        volume: session.volume,
        is_muted: session.is_muted,
    };
    let result = match command {
        SessionCommand::Stop => {
            status.player_state = "IDLE".into();
            cast.stop(
                &operation,
                &claims.receiver_address,
                claims.receiver_port,
                &managed.transport_id,
                managed.media_session_id,
            )
            .await
        }
        SessionCommand::Play => {
            status.player_state = "PLAYING".into();
            cast.play(
                &operation,
                &claims.receiver_address,
                claims.receiver_port,
                &managed.transport_id,
                managed.media_session_id,
            )
            .await
        }
        SessionCommand::Pause => {
            status.player_state = "PAUSED".into();
            cast.pause(
                &operation,
                &claims.receiver_address,
                claims.receiver_port,
                &managed.transport_id,
                managed.media_session_id,
            )
            .await
        }
        SessionCommand::Seek(position) => {
            status.current_position =
                crate::services::cast::CastService::validate_position(position)?;
            cast.seek(
                &operation,
                &claims.receiver_address,
                claims.receiver_port,
                &managed.transport_id,
                managed.media_session_id,
                position,
            )
            .await
        }
        SessionCommand::SetVolume(volume) => {
            let volume = crate::services::cast::CastService::validate_volume(volume)?;
            status.volume = f64::from(volume);
            cast.set_volume(
                &operation,
                &claims.receiver_address,
                claims.receiver_port,
                volume,
            )
            .await
        }
        SessionCommand::SetMuted(muted) => {
            status.is_muted = muted;
            cast.set_muted(
                &operation,
                &claims.receiver_address,
                claims.receiver_port,
                muted,
            )
            .await
        }
    };
    result.map_err(|error| {
        let error = cast_control_error("control playback", session_id, error);
        anyhow::anyhow!(error.message)
    })?;
    validate_cast_caller(manager, actor, caller).await?;
    let ended = checkpoint_cast_status_for_caller(
        manager,
        claims,
        status,
        &managed.transport_id,
        managed.media_session_id,
        Some(caller),
        Some(&session),
    )
    .await?;
    if ended {
        cast.forget_session(session_id);
    }
    let updated = query_session_by_id(manager, actor, session_id)
        .await
        .map_err(|error| anyhow::anyhow!(error.message))?
        .ok_or_else(|| anyhow::anyhow!("Cast session not found"))?;
    Ok(CastSessionOperationResult {
        success: true,
        session: Some(map_session(updated, device_name)),
        error: None,
    })
}

fn map_device(node: DeviceNode) -> LegacyCastDevice {
    let is_connected = node.enabled.unwrap_or(true)
        && node
            .last_seen_at
            .as_deref()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .is_some_and(|seen| {
                chrono::Utc::now().signed_duration_since(seen.with_timezone(&chrono::Utc))
                    <= chrono::Duration::seconds(120)
            });
    LegacyCastDevice {
        id: node.id,
        name: node.name,
        address: node.address,
        port: node.port,
        model: node.model,
        device_type: node.device_type,
        is_favorite: node.is_favorite,
        is_manual: node.is_manual,
        is_connected,
        enabled: node.enabled.unwrap_or(true),
        playback_supported: node.playback_supported.unwrap_or(false),
        discovery_origin: node.discovery_origin,
        last_seen_at: node.last_seen_at,
    }
}

fn map_session(node: SessionNode, device_name: Option<String>) -> LegacyCastSession {
    LegacyCastSession {
        id: node.id,
        device_id: node.device_id,
        device_name,
        media_file_id: node.media_file_id,
        episode_id: node.episode_id,
        player_state: node.player_state,
        current_time: node.current_position,
        duration: node.duration,
        volume: node.volume,
        is_muted: node.is_muted,
        started_at: node.started_at,
        last_error: node.last_error,
        playback_decision: node.playback_decision,
        playback_reason: node.playback_reason,
    }
}

fn parse_device_type(value: &str) -> CastDeviceType {
    match value.trim().to_ascii_uppercase().as_str() {
        "CHROMECAST" => CastDeviceType::Chromecast,
        "CHROMECAST_AUDIO" => CastDeviceType::ChromecastAudio,
        "DLNA_RENDERER" => CastDeviceType::DlnaRenderer,
        _ => CastDeviceType::Unknown,
    }
}

fn cast_control_error(
    action: &str,
    session_id: &str,
    _error: anyhow::Error,
) -> async_graphql::Error {
    let correlation_id = uuid::Uuid::new_v4().to_string();
    tracing::error!(
        correlation_id,
        cast_session_id = session_id,
        action,
        "Cast control command failed"
    );
    async_graphql::Error::new(format!(
        "Unable to {action} on this receiver (reference {correlation_id})"
    ))
}

/// Shared admission boundary for receiver media requests and the server monitor.
/// The caller must first verify the grant signature and request media/IP. All
/// persisted reads use one snapshot; checkpoint writes reuse the same checks in
/// their write transaction so revocation/reassignment cannot race the write.
pub(crate) async fn validate_cast_grant_authority(
    manager: &Arc<ServicesManager>,
    claims: &CastGrantClaims,
) -> anyhow::Result<CastSession> {
    let database = manager
        .get_database()
        .await
        .ok_or_else(|| anyhow::anyhow!("Database service not available"))?;
    let mut tx =
        MutationContext::begin_with_auth(database.pool(), TransactionMode::Default, None).await?;
    let result = validate_cast_authority_in_transaction(&mut tx, claims).await;
    tx.rollback().await?;
    result
}

async fn validate_cast_authority_in_transaction(
    tx: &mut MutationContext<'_>,
    claims: &CastGrantClaims,
) -> anyhow::Result<CastSession> {
    let now = chrono::Utc::now();
    if claims.expires_at <= now.timestamp()
        || claims.issuer_session_id.is_empty()
        || uuid::Uuid::parse_str(&claims.issuer_session_id).is_err()
    {
        anyhow::bail!("Cast issuing authority has expired or is missing");
    }
    validate_cast_principal_in_transaction(
        tx,
        &claims.user_id,
        &claims.issuer_session_id,
        &claims.issuer_roles,
    )
    .await?;
    let file = tx
        .find_by_id::<MediaFile>(&claims.media_file_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Cast media file no longer exists"))?;
    let library_id = file
        .library_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Cast media file has no current library"))?;
    let library = tx
        .find_by_id::<Library>(library_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Cast library no longer exists"))?;
    if library.user_id != claims.user_id {
        anyhow::bail!("Cast media ownership changed");
    }
    let session = tx
        .find_by_id::<CastSession>(&claims.session_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Cast session no longer exists"))?;
    let expiry_matches = session
        .grant_expires_at
        .as_deref()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .is_some_and(|expiry| expiry.timestamp() == claims.expires_at);
    if session.user_id.as_deref() != Some(claims.user_id.as_str())
        || session.media_file_id.as_deref() != Some(claims.media_file_id.as_str())
        || session.device_id.as_deref() != Some(claims.receiver_device_id.as_str())
        || session.receiver_address.as_deref() != Some(claims.receiver_address.as_str())
        || session.ended_at.is_some()
        || !expiry_matches
        || !matches!(
            session.player_state.as_str(),
            "STARTING" | "PLAYING" | "PAUSED" | "BUFFERING"
        )
    {
        anyhow::bail!("Cast session binding is no longer active");
    }
    // STARTING admits the receiver's initial LOAD fetch, before the SDK has
    // returned its transport/media IDs. Established sessions require all IDs.
    if session.player_state != "STARTING"
        && (session
            .receiver_transport_id
            .as_deref()
            .is_none_or(str::is_empty)
            || session
                .receiver_session_id
                .as_deref()
                .is_none_or(str::is_empty)
            || session.media_session_id.is_none())
    {
        anyhow::bail!("Cast receiver session binding is missing");
    }
    let device = tx
        .find_by_id::<CastDevice>(&claims.receiver_device_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Cast receiver no longer exists"))?;
    let (address, port) =
        crate::services::cast::CastService::validate_target(&device.address, device.port)?;
    if device.enabled != Some(true)
        || device.playback_supported != Some(true)
        || !matches!(
            parse_device_type(&device.device_type),
            CastDeviceType::Chromecast | CastDeviceType::ChromecastAudio
        )
        || address.to_string() != claims.receiver_address
        || port != claims.receiver_port
    {
        anyhow::bail!("Cast receiver binding is no longer allowed");
    }
    Ok(session)
}

/// Rechecks real persisted login authority without manufacturing token claims.
async fn validate_cast_principal_in_transaction(
    tx: &mut MutationContext<'_>,
    user_id: &str,
    issuer_session_id: &str,
    issuer_roles: &[String],
) -> anyhow::Result<()> {
    let now = chrono::Utc::now();
    let owner_id = user_id.to_string();
    let user = tx
        .find_by_id::<User>(&owner_id)
        .await?
        .filter(|user| user.is_active)
        .ok_or_else(|| anyhow::anyhow!("Cast owner is no longer active"))?;
    let current_role = Role::parse(&user.role)
        .ok_or_else(|| anyhow::anyhow!("Cast owner role is no longer allowed"))?;
    let roles: Option<Vec<Role>> = issuer_roles.iter().map(|role| Role::parse(role)).collect();
    let roles = roles
        .filter(|roles| !roles.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Cast issuing claims are not allowed"))?;
    if current_role != Role::Admin && roles.contains(&Role::Admin) {
        anyhow::bail!("Cast issuing role is no longer allowed");
    }
    let active = tx
        .query::<RefreshToken>()
        .filter(RefreshTokenWhereInput {
            user_id: Some(cast_eq(user_id)),
            session_id: Some(cast_eq(issuer_session_id)),
            revoked_at: Some(DateFilter {
                is_null: Some(true),
                ..Default::default()
            }),
            expires_at: Some(DateFilter {
                gt: Some(now.to_rfc3339()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .limit(1)
        .fetch_all()
        .await?;
    let active = active
        .first()
        .ok_or_else(|| anyhow::anyhow!("Cast issuing session is no longer active"))?;
    let persisted_roles = active
        .roles
        .as_deref()
        .and_then(|value| serde_json::from_str::<Vec<String>>(value).ok())
        .ok_or_else(|| anyhow::anyhow!("Cast persisted claims are missing"))?;
    if !roles.iter().all(|role| {
        persisted_roles
            .iter()
            .any(|value| Role::parse(value) == Some(*role))
    }) {
        anyhow::bail!("Cast issuing claims are no longer allowed");
    }
    Ok(())
}

fn cast_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

/// Revalidate and save the canonical personal checkpoint and Cast diagnostics
/// atomically. No request user is impersonated: this trusted job uses the owner
/// checked against the current file/library and the signed issuing session.
#[cfg(test)]
async fn checkpoint_cast_status(
    manager: &Arc<ServicesManager>,
    claims: &CastGrantClaims,
    status: crate::services::cast::CastRemoteStatus,
    transport_id: &str,
    media_session_id: i32,
) -> anyhow::Result<bool> {
    checkpoint_cast_status_for_caller(
        manager,
        claims,
        status,
        transport_id,
        media_session_id,
        None,
        None,
    )
    .await
}

async fn checkpoint_cast_status_for_caller(
    manager: &Arc<ServicesManager>,
    claims: &CastGrantClaims,
    status: crate::services::cast::CastRemoteStatus,
    transport_id: &str,
    media_session_id: i32,
    caller: Option<&SessionAuthority>,
    observed: Option<&CastSession>,
) -> anyhow::Result<bool> {
    let database = manager
        .get_database()
        .await
        .ok_or_else(|| anyhow::anyhow!("Database service not available"))?;
    let mut tx =
        MutationContext::begin_with_auth(database.pool(), TransactionMode::StateMachine, None)
            .await?;
    if let Some(caller) = caller {
        validate_cast_principal_in_transaction(
            &mut tx,
            &caller.0.user_id,
            &caller.0.session_id.to_string(),
            &caller.0.roles,
        )
        .await?;
    }
    let session = validate_cast_authority_in_transaction(&mut tx, claims).await?;
    if let Some(observed) = observed {
        // A poll fetched before a confirmed command cannot roll its checkpoint
        // back, even if a future caller accidentally splits the lifecycle lock.
        if session.updated_at != observed.updated_at
            || session.current_position != observed.current_position
            || session.player_state != observed.player_state
            || session.volume != observed.volume
            || session.is_muted != observed.is_muted
            || session.receiver_transport_id != observed.receiver_transport_id
            || session.receiver_session_id != observed.receiver_session_id
            || session.media_session_id != observed.media_session_id
        {
            anyhow::bail!("Cast checkpoint changed after admission");
        }
    }
    if session.receiver_transport_id.as_deref() != Some(transport_id)
        || session.media_session_id != Some(media_session_id)
    {
        anyhow::bail!("Cast receiver media session changed during status poll");
    }
    let (position, duration, ended, finished) = checkpoint_values(&session, &status)?;
    let now = chrono::Utc::now().to_rfc3339();
    let existing = tx
        .query::<PlaybackProgress>()
        .filter(PlaybackProgressWhereInput {
            user_id: Some(cast_eq(&claims.user_id)),
            media_file_id: Some(cast_eq(&claims.media_file_id)),
            ..Default::default()
        })
        .order_by(PlaybackProgressOrderByInput {
            updated_at: Some(OrderDirection::Desc),
            ..Default::default()
        })
        .limit(1)
        .fetch_all()
        .await?;
    let percent = duration
        .map(|duration| (position / duration * 100.0).clamp(0.0, 100.0))
        .unwrap_or(0.0);
    if let Some(existing) = existing.first() {
        tx.update_by_id::<PlaybackProgress>(
            &existing.id,
            UpdatePlaybackProgressInput {
                current_position: Some(position),
                duration: Some(duration),
                progress_percent: Some(percent),
                is_watched: Some(existing.is_watched || finished),
                watched_at: if finished && existing.watched_at.is_none() {
                    Some(Some(now.clone()))
                } else {
                    None
                },
                ..Default::default()
            },
        )
        .await?;
    } else {
        tx.insert::<PlaybackProgress>(CreatePlaybackProgressInput {
            user_id: claims.user_id.clone(),
            media_file_id: Some(claims.media_file_id.clone()),
            current_position: position,
            duration,
            progress_percent: percent,
            is_watched: finished,
            watched_at: finished.then(|| now.clone()),
        })
        .await?;
    }
    tx.update_by_id::<CastSession>(
        &claims.session_id,
        UpdateCastSessionInput {
            player_state: Some(if ended {
                "ENDED".to_string()
            } else {
                status.player_state
            }),
            current_position: Some(position),
            last_position: Some(Some(position)),
            duration: Some(duration),
            volume: Some(status.volume),
            is_muted: Some(status.is_muted),
            last_error: Some(None),
            ended_at: ended.then_some(Some(now)),
            ..Default::default()
        },
    )
    .await?;
    tx.commit_and_emit().await?;
    Ok(ended)
}

fn checkpoint_values(
    session: &CastSession,
    status: &crate::services::cast::CastRemoteStatus,
) -> anyhow::Result<(f64, Option<f64>, bool, bool)> {
    crate::services::cast::CastService::validate_position(status.current_position)?;
    crate::services::cast::CastService::validate_volume(status.volume)?;
    if !matches!(
        status.player_state.as_str(),
        "PLAYING" | "PAUSED" | "BUFFERING" | "IDLE"
    ) {
        anyhow::bail!("Invalid receiver playback state");
    }
    let valid_duration = |value: &f64| value.is_finite() && *value > 0.0;
    let duration = status
        .duration
        .filter(valid_duration)
        .or_else(|| session.duration.filter(valid_duration));
    let ended = status.player_state == "IDLE";
    let finished = ended && status.finished;
    // Receivers commonly clear currentTime on terminal status. Preserve the
    // checkpoint on cancel/error; only FINISHED advances to the known duration.
    crate::services::cast::CastService::validate_position(session.current_position)?;
    let position = if finished {
        duration.unwrap_or(session.current_position.max(status.current_position))
    } else if ended && status.current_position == 0.0 {
        session.current_position
    } else {
        status.current_position
    };
    Ok((
        duration
            .map(|duration| position.min(duration))
            .unwrap_or(position),
        duration,
        ended,
        finished,
    ))
}

async fn retire_cast_monitor(manager: &Arc<ServicesManager>, claims: &CastGrantClaims) {
    retire_cast_monitor_with_binding(manager, claims, None).await;
}

async fn retire_cast_monitor_with_binding(
    manager: &Arc<ServicesManager>,
    claims: &CastGrantClaims,
    receiver_binding: Option<(&str, i32)>,
) {
    let Some(database) = manager.get_database().await else {
        return;
    };
    let Ok(mut tx) =
        MutationContext::begin_with_auth(database.pool(), TransactionMode::StateMachine, None)
            .await
    else {
        return;
    };
    let Ok(Some(session)) = tx.find_by_id::<CastSession>(&claims.session_id).await else {
        return;
    };
    // Never mutate a reparented session or someone else's playback state.
    if session.user_id.as_deref() != Some(claims.user_id.as_str())
        || session.media_file_id.as_deref() != Some(claims.media_file_id.as_str())
        || session.device_id.as_deref() != Some(claims.receiver_device_id.as_str())
        || session.receiver_address.as_deref() != Some(claims.receiver_address.as_str())
        || session
            .grant_expires_at
            .as_deref()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .is_none_or(|expiry| expiry.timestamp() != claims.expires_at)
        || receiver_binding.is_some_and(|(transport, media)| {
            session.receiver_transport_id.as_deref() != Some(transport)
                || session.media_session_id != Some(media)
        })
        || session.ended_at.is_some()
    {
        return;
    }
    let result = tx
        .update_by_id::<CastSession>(
            &claims.session_id,
            UpdateCastSessionInput {
                player_state: Some("DISCONNECTED".into()),
                ended_at: Some(Some(chrono::Utc::now().to_rfc3339())),
                last_position: Some(Some(session.current_position)),
                last_error: Some(Some(
                    "Cast access ended or the receiver stopped responding.".into(),
                )),
                ..Default::default()
            },
        )
        .await;
    if result.is_ok() {
        let _ = tx.commit_and_emit().await;
    }
}

#[allow(clippy::too_many_arguments)]
async fn monitor_cast_session(
    manager: Arc<ServicesManager>,
    cast: Arc<crate::services::cast::CastService>,
    session_id: String,
    receiver_address: String,
    receiver_port: u16,
    transport_id: String,
    media_session_id: i32,
    claims: CastGrantClaims,
) {
    let mut failures = 0u8;
    let mut ticker = tokio::time::interval(std::time::Duration::from_secs(5));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        let Ok(operation) = cast
            .lock_session_operations(&receiver_address, receiver_port)
            .await
        else {
            return;
        };
        let Ok((managed, observed)) =
            admit_managed_cast_session(&manager, &cast, &session_id).await
        else {
            break;
        };
        if managed.claims != claims
            || managed.transport_id != transport_id
            || managed.media_session_id != media_session_id
        {
            break;
        }
        match cast
            .session_status(
                &operation,
                &receiver_address,
                receiver_port,
                &transport_id,
                media_session_id,
            )
            .await
        {
            Ok(status) => {
                failures = 0;
                match checkpoint_cast_status_for_caller(
                    &manager,
                    &claims,
                    status,
                    &transport_id,
                    media_session_id,
                    None,
                    Some(&observed),
                )
                .await
                {
                    Ok(true) => {
                        cast.forget_session(&session_id);
                        return;
                    }
                    Ok(false) => {}
                    Err(_) => break,
                }
            }
            Err(_error) => {
                failures = failures.saturating_add(1);
                tracing::warn!(cast_session_id = %session_id, failures,
                    "Cast receiver status poll failed");
                if failures >= 3 {
                    break;
                }
            }
        }
    }
    let Ok(operation) = cast
        .lock_session_operations(&receiver_address, receiver_port)
        .await
    else {
        return;
    };
    // A disabled/ended/rebound/historical session has no remaining hardware
    // authority. In particular never send a late STOP to replacement playback.
    if admit_managed_cast_session(&manager, &cast, &session_id)
        .await
        .is_ok_and(|(managed, _)| {
            managed.claims == claims
                && managed.transport_id == transport_id
                && managed.media_session_id == media_session_id
        })
    {
        let _ = cast
            .stop(
                &operation,
                &receiver_address,
                receiver_port,
                &transport_id,
                media_session_id,
            )
            .await;
    }
    retire_cast_monitor_with_binding(&manager, &claims, Some((&transport_id, media_session_id)))
        .await;
    if cast.managed_session(&session_id).is_some_and(|managed| {
        managed.claims == claims
            && managed.transport_id == transport_id
            && managed.media_session_id == media_session_id
    }) {
        cast.forget_session(&session_id);
    }
}

fn map_settings(node: SettingNode) -> LegacyCastSettings {
    LegacyCastSettings {
        auto_discovery_enabled: node.auto_discovery_enabled,
        discovery_interval_seconds: node.discovery_interval_seconds,
        default_volume: node.default_volume,
        transcode_incompatible: node.transcode_incompatible,
        preferred_quality: node.preferred_quality,
    }
}

pub(crate) async fn persist_discovered_device(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    device: &crate::services::cast::DiscoveredCastDevice,
) -> Result<()> {
    upsert_discovered_device(manager, auth_user, device)
        .await
        .map(drop)
}

pub(crate) async fn prune_stale_discovered_devices(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    retention_days: u64,
) -> Result<usize> {
    let retention_days = i64::try_from(retention_days)
        .map_err(|_| async_graphql::Error::new("Cast discovery retention is too large"))?;
    if retention_days <= 0 {
        return Err(async_graphql::Error::new(
            "Cast discovery retention must be greater than zero",
        ));
    }
    let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days);
    let mut offset = 0usize;
    let page_size = 500usize;
    let mut candidates = Vec::new();

    loop {
        let data = execute_graphql(
            manager,
            auth_user,
            r#"
            query StaleCastDeviceCandidates($page: PageInput) {
                CastDevices: castDevices(page: $page) {
                    Edges: edges {
                        Node: node {
                            Id: id
                            IsFavorite: isFavorite
                            IsManual: isManual
                            DiscoveryOrigin: discoveryOrigin
                            LastSeenAt: lastSeenAt
                        }
                    }
                }
            }
            "#,
            serde_json::json!({
                "page": { "limit": page_size, "offset": offset }
            }),
        )
        .await?;
        let edges = data
            .get("CastDevices")
            .and_then(|value| value.get("Edges"))
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        if edges.is_empty() {
            break;
        }

        for edge in &edges {
            let Some(node) = edge.get("Node") else {
                continue;
            };
            let is_transient = node.get("IsFavorite").and_then(serde_json::Value::as_bool)
                == Some(false)
                && node.get("IsManual").and_then(serde_json::Value::as_bool) == Some(false)
                && node
                    .get("DiscoveryOrigin")
                    .and_then(serde_json::Value::as_str)
                    == Some("NETWORK");
            let stale = node
                .get("LastSeenAt")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .is_some_and(|last_seen| last_seen.with_timezone(&chrono::Utc) < cutoff);
            if is_transient
                && stale
                && let Some(id) = node.get("Id").and_then(serde_json::Value::as_str)
            {
                candidates.push(id.to_string());
            }
        }

        if edges.len() < page_size {
            break;
        }
        offset += page_size;
    }

    for id in &candidates {
        delete_cast_device(manager, auth_user, id).await?;
        tracing::info!(
            cast_device_id = %id,
            retention_days,
            "Removed stale transient Cast discovery record"
        );
    }
    Ok(candidates.len())
}

async fn upsert_discovered_device(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    device: &crate::services::cast::DiscoveredCastDevice,
) -> Result<DeviceNode> {
    let existing =
        query_device_by_address_port(manager, auth_user, &device.address, device.port).await?;
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(existing) = existing {
        let playback_supported = matches!(
            device.device_type,
            CastDeviceType::Chromecast | CastDeviceType::ChromecastAudio
        );
        let updated = update_cast_device(
            manager,
            auth_user,
            &existing.id,
            serde_json::json!({
                "name": device.name,
                "model": device.model,
                "deviceType": device.device_type.as_str(),
                "address": device.address,
                "port": device.port,
                "enabled": existing.enabled.unwrap_or(true),
                "discoveryOrigin": "NETWORK",
                "playbackSupported": playback_supported,
                "lastSeenAt": now.clone(),
                "lastProbeAt": now,
                "lastProbeError": null,
            }),
        )
        .await?;
        return Ok(updated);
    }

    let playback_supported = matches!(
        device.device_type,
        CastDeviceType::Chromecast | CastDeviceType::ChromecastAudio
    );
    create_cast_device(
        manager,
        auth_user,
        serde_json::json!({
            "name": device.name,
            "address": device.address,
            "port": device.port,
            "model": device.model,
            "deviceType": device.device_type.as_str(),
            "isFavorite": false,
            "isManual": false,
            "enabled": true,
            "discoveryOrigin": "NETWORK",
            "playbackSupported": playback_supported,
            "firstSeenAt": now.clone(),
            "lastSeenAt": now.clone(),
            "lastProbeAt": now,
            "lastProbeError": null,
        }),
    )
    .await
}

struct CastPlaybackSettings {
    default_volume: f64,
    default_muted: bool,
    transcode_incompatible: bool,
    preferred_quality: Option<String>,
}

async fn query_cast_playback_settings(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
) -> Result<CastPlaybackSettings> {
    let setting = query_latest_cast_setting(manager, auth_user).await?;
    let volume = setting
        .as_ref()
        .map(|setting| setting.default_volume)
        .unwrap_or(1.0);
    Ok(CastPlaybackSettings {
        default_volume: crate::services::cast::CastService::validate_volume(volume)
            .map(f64::from)
            .unwrap_or(1.0),
        default_muted: false,
        transcode_incompatible: setting
            .as_ref()
            .is_some_and(|setting| setting.transcode_incompatible),
        preferred_quality: setting.and_then(|setting| setting.preferred_quality),
    })
}

async fn query_latest_cast_setting(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
) -> Result<Option<SettingNode>> {
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        query LatestCastSetting($page: PageInput, $orderBy: [CastSettingOrderByInput!]) {
            CastSettings: castSettings(page: $page, orderBy: $orderBy) {
                Edges: edges {
                    Node: node {
                        Id: id
                        AutoDiscoveryEnabled: autoDiscoveryEnabled
                        DiscoveryIntervalSeconds: discoveryIntervalSeconds
                        DefaultVolume: defaultVolume
                        TranscodeIncompatible: transcodeIncompatible
                        PreferredQuality: preferredQuality
                    }
                }
            }
        }
        "#,
        serde_json::json!({
            "orderBy": [{ "updatedAt": "DESC" }],
            "page": { "limit": 1, "offset": 0 }
        }),
    )
    .await?;

    let node = data
        .get("CastSettings")
        .and_then(|v| v.get("Edges"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|edge| edge.get("Node"))
        .cloned();

    Ok(node.and_then(|v| serde_json::from_value(v).ok()))
}

async fn query_media_file(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    media_file_id: &str,
) -> Result<Option<MediaFileNode>> {
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        query MediaFileById($id: String!) {
            MediaFile: mediaFile(id: $id) {
                Path: path
                Duration: duration
                ContentType: contentType
                LibraryId: libraryId
                Container: container
                VideoCodec: videoCodec
                AudioCodec: audioCodec
                Height: height
                IsHdr: isHdr
            }
        }
        "#,
        serde_json::json!({ "id": media_file_id }),
    )
    .await?;
    let Some(node) = data.get("MediaFile") else {
        return Ok(None);
    };
    Ok(serde_json::from_value(node.clone()).ok())
}

async fn query_device_by_id(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    device_id: &str,
) -> Result<Option<DeviceNode>> {
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        query CastDeviceById($id: String!) {
            CastDevice: castDevice(id: $id) {
                Id: id
                Name: name
                Address: address
                Port: port
                Model: model
                DeviceType: deviceType
                IsFavorite: isFavorite
                IsManual: isManual
                Enabled: enabled
                PlaybackSupported: playbackSupported
                DiscoveryOrigin: discoveryOrigin
                LastSeenAt: lastSeenAt
            }
        }
        "#,
        serde_json::json!({ "id": device_id }),
    )
    .await?;
    let Some(node) = data.get("CastDevice") else {
        return Ok(None);
    };
    Ok(serde_json::from_value(node.clone()).ok())
}

async fn query_device_by_address_port(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    address: &str,
    port: i32,
) -> Result<Option<DeviceNode>> {
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        query CastDeviceByAddress($where: CastDeviceWhereInput, $page: PageInput) {
            CastDevices: castDevices(where: $where, page: $page) {
                Edges: edges {
                    Node: node {
                        Id: id
                        Name: name
                        Address: address
                        Port: port
                        Model: model
                        DeviceType: deviceType
                        IsFavorite: isFavorite
                        IsManual: isManual
                        Enabled: enabled
                        PlaybackSupported: playbackSupported
                        DiscoveryOrigin: discoveryOrigin
                        LastSeenAt: lastSeenAt
                    }
                }
            }
        }
        "#,
        serde_json::json!({
            "where": {
                "address": { "eq": address },
                "port": { "eq": port }
            },
            "page": { "limit": 1, "offset": 0 }
        }),
    )
    .await?;

    let node = data
        .get("CastDevices")
        .and_then(|v| v.get("Edges"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|edge| edge.get("Node"))
        .cloned();

    Ok(node.and_then(|v| serde_json::from_value(v).ok()))
}

async fn query_session_by_id(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    session_id: &str,
) -> Result<Option<SessionNode>> {
    let database = manager
        .get_database()
        .await
        .ok_or_else(|| async_graphql::Error::new("Database service not available"))?;
    let session = CastSession::get(database.pool().pool(), &session_id.to_string())
        .await
        .map_err(|error| async_graphql::Error::new(error.to_string()))?;
    Ok(session
        .filter(|session| {
            auth_user.is_admin() || session.user_id.as_deref() == Some(auth_user.user_id.as_str())
        })
        .map(session_node_from_entity))
}

async fn create_cast_device(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    input: serde_json::Value,
) -> Result<DeviceNode> {
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        mutation CreateCastDeviceMutation($input: CreateCastDeviceInput!) {
            CreateCastDevice: createCastDevice(input: $input) {
                Success: success
                Error: error
                CastDevice: castDevice {
                    Id: id
                    Name: name
                    Address: address
                    Port: port
                    Model: model
                    DeviceType: deviceType
                    IsFavorite: isFavorite
                    IsManual: isManual
                    Enabled: enabled
                    PlaybackSupported: playbackSupported
                    DiscoveryOrigin: discoveryOrigin
                    LastSeenAt: lastSeenAt
                }
            }
        }
        "#,
        serde_json::json!({ "input": input }),
    )
    .await?;

    let success = data
        .get("CreateCastDevice")
        .and_then(|v| v.get("Success"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !success {
        let error = data
            .get("CreateCastDevice")
            .and_then(|v| v.get("Error"))
            .and_then(|v| v.as_str())
            .unwrap_or("Failed to create cast device");
        return Err(async_graphql::Error::new(error.to_string()));
    }

    let node = data
        .get("CreateCastDevice")
        .and_then(|v| v.get("CastDevice"))
        .cloned()
        .ok_or_else(|| async_graphql::Error::new("Missing CastDevice payload"))?;
    serde_json::from_value(node).map_err(|e| async_graphql::Error::new(e.to_string()))
}

async fn delete_cast_device(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    id: &str,
) -> Result<()> {
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        mutation DeleteCastDeviceMutation($id: String!) {
            DeleteCastDevice: deleteCastDevice(id: $id) {
                Success: success
                Error: error
            }
        }
        "#,
        serde_json::json!({ "id": id }),
    )
    .await?;

    let success = data
        .get("DeleteCastDevice")
        .and_then(|v| v.get("Success"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !success {
        let error = data
            .get("DeleteCastDevice")
            .and_then(|v| v.get("Error"))
            .and_then(|v| v.as_str())
            .unwrap_or("Failed to delete cast device");
        return Err(async_graphql::Error::new(error.to_string()));
    }
    Ok(())
}

async fn update_cast_device(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    id: &str,
    input: serde_json::Value,
) -> Result<DeviceNode> {
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        mutation UpdateCastDeviceMutation($id: String!, $input: UpdateCastDeviceInput!) {
            UpdateCastDevice: updateCastDevice(id: $id, input: $input) {
                Success: success
                Error: error
                CastDevice: castDevice {
                    Id: id
                    Name: name
                    Address: address
                    Port: port
                    Model: model
                    DeviceType: deviceType
                    IsFavorite: isFavorite
                    IsManual: isManual
                    Enabled: enabled
                    PlaybackSupported: playbackSupported
                    DiscoveryOrigin: discoveryOrigin
                    LastSeenAt: lastSeenAt
                }
            }
        }
        "#,
        serde_json::json!({ "id": id, "input": input }),
    )
    .await?;

    let success = data
        .get("UpdateCastDevice")
        .and_then(|v| v.get("Success"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !success {
        let error = data
            .get("UpdateCastDevice")
            .and_then(|v| v.get("Error"))
            .and_then(|v| v.as_str())
            .unwrap_or("Failed to update cast device");
        return Err(async_graphql::Error::new(error.to_string()));
    }

    let node = data
        .get("UpdateCastDevice")
        .and_then(|v| v.get("CastDevice"))
        .cloned()
        .ok_or_else(|| async_graphql::Error::new("Missing CastDevice payload"))?;
    serde_json::from_value(node).map_err(|e| async_graphql::Error::new(e.to_string()))
}

async fn create_cast_session(
    manager: &Arc<ServicesManager>,
    args: CreateCastSessionArgs,
) -> Result<SessionNode> {
    let database = manager
        .get_database()
        .await
        .ok_or_else(|| async_graphql::Error::new("Database service not available"))?;
    CastSession::insert(
        database.pool(),
        CreateCastSessionInput {
            user_id: args.user_id,
            device_id: args.device_id,
            media_file_id: args.media_file_id,
            episode_id: args.episode_id,
            stream_url: args.stream_url,
            receiver_address: args.receiver_address,
            grant_expires_at: args.grant_expires_at,
            receiver_transport_id: args.receiver_transport_id,
            receiver_session_id: args.receiver_session_id,
            media_session_id: args.media_session_id,
            last_error: args.last_error,
            playback_decision: args.playback_decision,
            playback_reason: args.playback_reason,
            player_state: args.player_state,
            current_position: args.current_position,
            duration: args.duration,
            volume: args.volume,
            is_muted: args.is_muted,
            started_at: chrono::Utc::now().to_rfc3339(),
            ended_at: None,
            last_position: Some(args.current_position),
        },
    )
    .await
    .map(session_node_from_entity)
    .map_err(|error| async_graphql::Error::new(error.to_string()))
}

async fn update_cast_session(
    manager: &Arc<ServicesManager>,
    id: &str,
    patch: CastSessionPatch,
) -> Result<SessionNode> {
    let database = manager
        .get_database()
        .await
        .ok_or_else(|| async_graphql::Error::new("Database service not available"))?;
    let id = id.to_string();
    let session = CastSession::update_by_id(
        database.pool(),
        &id,
        UpdateCastSessionInput {
            receiver_transport_id: patch.receiver_transport_id,
            receiver_session_id: patch.receiver_session_id,
            media_session_id: patch.media_session_id,
            last_error: patch.last_error,
            playback_decision: patch.playback_decision,
            playback_reason: patch.playback_reason,
            player_state: patch.player_state,
            current_position: patch.current_position,
            duration: patch.duration,
            volume: patch.volume,
            is_muted: patch.is_muted,
            ended_at: patch.ended_at,
            last_position: patch.last_position,
            ..Default::default()
        },
    )
    .await
    .map_err(|error| async_graphql::Error::new(error.to_string()))?
    .ok_or_else(|| async_graphql::Error::new("Cast session not found"))?;
    Ok(session_node_from_entity(session))
}

fn session_node_from_entity(session: CastSession) -> SessionNode {
    SessionNode {
        id: session.id,
        device_id: session.device_id,
        media_file_id: session.media_file_id,
        episode_id: session.episode_id,
        stream_url: session.stream_url,
        last_error: session.last_error,
        playback_decision: session.playback_decision,
        playback_reason: session.playback_reason,
        player_state: session.player_state,
        current_position: session.current_position,
        duration: session.duration,
        volume: session.volume,
        is_muted: session.is_muted,
        started_at: session.started_at,
    }
}

async fn create_cast_setting(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    input: serde_json::Value,
) -> Result<SettingNode> {
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        mutation CreateCastSettingMutation($input: CreateCastSettingInput!) {
            CreateCastSetting: createCastSetting(input: $input) {
                Success: success
                Error: error
                CastSetting: castSetting {
                    Id: id
                    AutoDiscoveryEnabled: autoDiscoveryEnabled
                    DiscoveryIntervalSeconds: discoveryIntervalSeconds
                    DefaultVolume: defaultVolume
                    TranscodeIncompatible: transcodeIncompatible
                    PreferredQuality: preferredQuality
                }
            }
        }
        "#,
        serde_json::json!({ "input": input }),
    )
    .await?;

    let success = data
        .get("CreateCastSetting")
        .and_then(|v| v.get("Success"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !success {
        let error = data
            .get("CreateCastSetting")
            .and_then(|v| v.get("Error"))
            .and_then(|v| v.as_str())
            .unwrap_or("Failed to create cast settings");
        return Err(async_graphql::Error::new(error.to_string()));
    }

    let node = data
        .get("CreateCastSetting")
        .and_then(|v| v.get("CastSetting"))
        .cloned()
        .ok_or_else(|| async_graphql::Error::new("Missing CastSetting payload"))?;
    serde_json::from_value(node).map_err(|e| async_graphql::Error::new(e.to_string()))
}

async fn update_cast_setting(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    id: &str,
    input: serde_json::Value,
) -> Result<SettingNode> {
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        mutation UpdateCastSettingMutation($id: String!, $input: UpdateCastSettingInput!) {
            UpdateCastSetting: updateCastSetting(id: $id, input: $input) {
                Success: success
                Error: error
                CastSetting: castSetting {
                    Id: id
                    AutoDiscoveryEnabled: autoDiscoveryEnabled
                    DiscoveryIntervalSeconds: discoveryIntervalSeconds
                    DefaultVolume: defaultVolume
                    TranscodeIncompatible: transcodeIncompatible
                    PreferredQuality: preferredQuality
                }
            }
        }
        "#,
        serde_json::json!({ "id": id, "input": input }),
    )
    .await?;

    let success = data
        .get("UpdateCastSetting")
        .and_then(|v| v.get("Success"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !success {
        let error = data
            .get("UpdateCastSetting")
            .and_then(|v| v.get("Error"))
            .and_then(|v| v.as_str())
            .unwrap_or("Failed to update cast settings");
        return Err(async_graphql::Error::new(error.to_string()));
    }

    let node = data
        .get("UpdateCastSetting")
        .and_then(|v| v.get("CastSetting"))
        .cloned()
        .ok_or_else(|| async_graphql::Error::new("Missing CastSetting payload"))?;
    serde_json::from_value(node).map_err(|e| async_graphql::Error::new(e.to_string()))
}

async fn execute_graphql(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    query: &str,
    variables: serde_json::Value,
) -> Result<serde_json::Value> {
    let graphql = manager
        .get_graphql()
        .await
        .ok_or_else(|| async_graphql::Error::new("GraphQL service not available"))?;
    let schema = graphql
        .schema()
        .await
        .ok_or_else(|| async_graphql::Error::new("GraphQL schema not initialized"))?;

    let request = Request::new(query)
        .variables(Variables::from_json(variables))
        .data(auth_user.clone())
        .data(auth_user.user_id.clone());
    let response = schema.execute(request).await;
    if !response.errors.is_empty() {
        let message = response
            .errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(async_graphql::Error::new(message));
    }
    serde_json::to_value(response.data).map_err(|e| async_graphql::Error::new(e.to_string()))
}
