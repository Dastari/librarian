use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Request, Result, SimpleObject, Variables};
use serde::Deserialize;

use super::super::auth::{AuthExt, AuthUser};
use crate::services::ServicesManager;

#[derive(SimpleObject, Clone, Debug)]
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
    pub last_seen_at: Option<String>,
}

#[derive(SimpleObject, Clone, Debug)]
pub struct LegacyCastSession {
    pub id: String,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub media_file_id: Option<String>,
    pub episode_id: Option<String>,
    pub stream_url: String,
    pub player_state: String,
    pub current_time: f64,
    pub duration: Option<f64>,
    pub volume: f64,
    pub is_muted: bool,
    pub started_at: String,
}

#[derive(SimpleObject, Clone, Debug)]
pub struct CastSessionOperationResult {
    pub success: bool,
    pub session: Option<LegacyCastSession>,
    pub error: Option<String>,
}

#[derive(SimpleObject, Clone, Debug)]
pub struct CastDeviceOperationResult {
    pub success: bool,
    pub device: Option<LegacyCastDevice>,
    pub error: Option<String>,
}

#[derive(SimpleObject, Clone, Debug)]
pub struct LegacyCastSettings {
    pub auto_discovery_enabled: bool,
    pub discovery_interval_seconds: i32,
    pub default_volume: f64,
    pub transcode_incompatible: bool,
    pub preferred_quality: Option<String>,
}

#[derive(SimpleObject, Clone, Debug)]
pub struct CastSettingsOperationResult {
    pub success: bool,
    pub settings: Option<LegacyCastSettings>,
    pub error: Option<String>,
}

#[derive(SimpleObject, Clone, Debug)]
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
    #[graphql(name = "DiscoverCastDevices")]
    async fn discover_cast_devices(&self, ctx: &Context<'_>) -> Result<Vec<LegacyCastDevice>> {
        let auth_user = ctx.auth_user()?.clone();
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
            let node = upsert_discovered_device(manager, &auth_user, &device).await?;
            out.push(map_device(node));
        }

        Ok(out)
    }

    #[graphql(name = "CastMedia")]
    async fn cast_media(
        &self,
        ctx: &Context<'_>,
        input: CastMediaInput,
    ) -> Result<CastSessionOperationResult> {
        let auth_user = ctx.auth_user()?.clone();
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let cast = manager
            .get_cast()
            .await
            .ok_or_else(|| async_graphql::Error::new("Cast service not available"))?;

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

        end_active_sessions_for_device(manager, &auth_user, &input.device_id).await?;

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

        let stream_url = format!(
            "{}/api/media/{}/stream",
            cast.media_base_url(),
            input.media_file_id
        );
        let content_type = crate::services::cast::CastService::infer_content_type(
            &media_file.path,
            media_file.content_type.as_deref(),
        );
        let start_position = input.start_position.unwrap_or(0.0);

        let duration = match cast
            .cast_media(
                &device.address,
                device.port as u16,
                &stream_url,
                &content_type,
                start_position,
            )
            .await
        {
            Ok(duration) => duration,
            Err(err) => {
                return Ok(CastSessionOperationResult {
                    success: false,
                    session: None,
                    error: Some(format!("Failed to cast media: {}", err)),
                });
            }
        };

        let (default_volume, default_muted) =
            query_cast_settings_defaults(manager, &auth_user).await?;
        let session = create_cast_session(
            manager,
            &auth_user,
            CreateCastSessionArgs {
                device_id: Some(input.device_id.clone()),
                media_file_id: Some(input.media_file_id.clone()),
                episode_id: input.episode_id.clone(),
                stream_url: stream_url.clone(),
                player_state: "PLAYING".to_string(),
                current_position: start_position,
                duration,
                volume: default_volume,
                is_muted: default_muted,
            },
        )
        .await?;

        Ok(CastSessionOperationResult {
            success: true,
            session: Some(map_session(session, Some(device.name))),
            error: None,
        })
    }

    #[graphql(name = "CastPlay")]
    async fn cast_play(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<CastSessionOperationResult> {
        update_cast_session_state(ctx, &session_id, SessionCommand::Play).await
    }

    #[graphql(name = "CastPause")]
    async fn cast_pause(
        &self,
        ctx: &Context<'_>,
        session_id: String,
    ) -> Result<CastSessionOperationResult> {
        update_cast_session_state(ctx, &session_id, SessionCommand::Pause).await
    }

    #[graphql(name = "CastStop")]
    async fn cast_stop(&self, ctx: &Context<'_>, session_id: String) -> Result<CastActionResult> {
        let auth_user = ctx.auth_user()?.clone();
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let cast = manager
            .get_cast()
            .await
            .ok_or_else(|| async_graphql::Error::new("Cast service not available"))?;

        let Some(session) = query_session_by_id(manager, &auth_user, &session_id).await? else {
            return Ok(CastActionResult {
                success: false,
                error: Some("Cast session not found".to_string()),
            });
        };
        let Some(device_id) = session.device_id.clone() else {
            return Ok(CastActionResult {
                success: false,
                error: Some("Cast session has no device".to_string()),
            });
        };
        let Some(device) = query_device_by_id(manager, &auth_user, &device_id).await? else {
            return Ok(CastActionResult {
                success: false,
                error: Some("Cast device not found".to_string()),
            });
        };

        if let Err(err) = cast.stop(&device.address, device.port as u16).await {
            return Ok(CastActionResult {
                success: false,
                error: Some(format!("Failed to stop cast: {}", err)),
            });
        }

        let now = chrono::Utc::now().to_rfc3339();
        let _ = update_cast_session(
            manager,
            &auth_user,
            &session_id,
            serde_json::json!({
                "PlayerState": "IDLE",
                "EndedAt": now,
                "LastPosition": session.current_position,
            }),
        )
        .await?;

        Ok(CastActionResult {
            success: true,
            error: None,
        })
    }

    #[graphql(name = "CastSeek")]
    async fn cast_seek(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        position: f64,
    ) -> Result<CastSessionOperationResult> {
        update_cast_session_state(ctx, &session_id, SessionCommand::Seek(position)).await
    }

    #[graphql(name = "CastSetVolume")]
    async fn cast_set_volume(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        volume: f64,
    ) -> Result<CastSessionOperationResult> {
        update_cast_session_state(ctx, &session_id, SessionCommand::SetVolume(volume)).await
    }

    #[graphql(name = "CastSetMuted")]
    async fn cast_set_muted(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        muted: bool,
    ) -> Result<CastSessionOperationResult> {
        update_cast_session_state(ctx, &session_id, SessionCommand::SetMuted(muted)).await
    }

    #[graphql(name = "AddCastDevice")]
    async fn add_cast_device(
        &self,
        ctx: &Context<'_>,
        input: AddCastDeviceInput,
    ) -> Result<CastDeviceOperationResult> {
        let auth_user = ctx.auth_user()?.clone();
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let name = input
            .name
            .clone()
            .unwrap_or_else(|| format!("Cast Device ({})", input.address));
        let node = create_cast_device(
            manager,
            &auth_user,
            serde_json::json!({
                "Name": name,
                "Address": input.address,
                "Port": input.port.unwrap_or(8009),
                "Model": null,
                "DeviceType": "CHROMECAST",
                "IsFavorite": false,
                "IsManual": true,
                "LastSeenAt": null,
            }),
        )
        .await?;

        Ok(CastDeviceOperationResult {
            success: true,
            device: Some(map_device(node)),
            error: None,
        })
    }

    #[graphql(name = "UpdateCastDevice")]
    async fn update_cast_device(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateCastDeviceInput,
    ) -> Result<CastDeviceOperationResult> {
        let auth_user = ctx.auth_user()?.clone();
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let Some(existing) = query_device_by_id(manager, &auth_user, &id).await? else {
            return Ok(CastDeviceOperationResult {
                success: false,
                device: None,
                error: Some("Cast device not found".to_string()),
            });
        };

        let node = update_cast_device(
            manager,
            &auth_user,
            &id,
            serde_json::json!({
                "Name": input.name.unwrap_or(existing.name),
                "Address": input.address.unwrap_or(existing.address),
                "Port": input.port.unwrap_or(existing.port),
                "Model": input.model.or(existing.model),
                "DeviceType": input.device_type.unwrap_or(existing.device_type),
                "IsFavorite": input.is_favorite.unwrap_or(existing.is_favorite),
                "IsManual": input.is_manual.unwrap_or(existing.is_manual),
            }),
        )
        .await?;

        Ok(CastDeviceOperationResult {
            success: true,
            device: Some(map_device(node)),
            error: None,
        })
    }

    #[graphql(name = "RemoveCastDevice")]
    async fn remove_cast_device(&self, ctx: &Context<'_>, id: String) -> Result<CastActionResult> {
        let auth_user = ctx.auth_user()?.clone();
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        delete_cast_device(manager, &auth_user, &id).await?;
        Ok(CastActionResult {
            success: true,
            error: None,
        })
    }

    #[graphql(name = "UpdateCastSettings")]
    async fn update_cast_settings(
        &self,
        ctx: &Context<'_>,
        input: UpdateCastSettingsInput,
    ) -> Result<CastSettingsOperationResult> {
        let auth_user = ctx.auth_user()?.clone();
        let manager = ctx.data::<Arc<ServicesManager>>()?;

        let existing = query_latest_cast_setting(manager, &auth_user).await?;
        let node = match existing {
            Some(setting) => {
                update_cast_setting(
                    manager,
                    &auth_user,
                    &setting.id,
                    serde_json::json!({
                        "AutoDiscoveryEnabled": input.auto_discovery_enabled.unwrap_or(setting.auto_discovery_enabled),
                        "DiscoveryIntervalSeconds": input.discovery_interval_seconds.unwrap_or(setting.discovery_interval_seconds),
                        "DefaultVolume": input.default_volume.unwrap_or(setting.default_volume),
                        "TranscodeIncompatible": input.transcode_incompatible.unwrap_or(setting.transcode_incompatible),
                        "PreferredQuality": input.preferred_quality.or(setting.preferred_quality),
                    }),
                )
                .await?
            }
            None => {
                create_cast_setting(
                    manager,
                    &auth_user,
                    serde_json::json!({
                        "AutoDiscoveryEnabled": input.auto_discovery_enabled.unwrap_or(true),
                        "DiscoveryIntervalSeconds": input.discovery_interval_seconds.unwrap_or(30),
                        "DefaultVolume": input.default_volume.unwrap_or(1.0),
                        "TranscodeIncompatible": input.transcode_incompatible.unwrap_or(false),
                        "PreferredQuality": input.preferred_quality,
                    }),
                )
                .await?
            }
        };

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
    #[serde(rename = "StreamUrl")]
    stream_url: String,
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
    #[serde(rename = "ContentType")]
    content_type: Option<String>,
}

#[derive(Clone, Debug)]
struct CreateCastSessionArgs {
    device_id: Option<String>,
    media_file_id: Option<String>,
    episode_id: Option<String>,
    stream_url: String,
    player_state: String,
    current_position: f64,
    duration: Option<f64>,
    volume: f64,
    is_muted: bool,
}

enum SessionCommand {
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
    let auth_user = ctx.auth_user()?.clone();
    let manager = ctx.data::<Arc<ServicesManager>>()?;
    let cast = manager
        .get_cast()
        .await
        .ok_or_else(|| async_graphql::Error::new("Cast service not available"))?;

    let Some(session) = query_session_by_id(manager, &auth_user, session_id).await? else {
        return Ok(CastSessionOperationResult {
            success: false,
            session: None,
            error: Some("Cast session not found".to_string()),
        });
    };
    let Some(device_id) = session.device_id.clone() else {
        return Ok(CastSessionOperationResult {
            success: false,
            session: None,
            error: Some("Cast session has no device".to_string()),
        });
    };
    let Some(device) = query_device_by_id(manager, &auth_user, &device_id).await? else {
        return Ok(CastSessionOperationResult {
            success: false,
            session: None,
            error: Some("Cast device not found".to_string()),
        });
    };

    let update_input = match command {
        SessionCommand::Play => {
            cast.play(&device.address, device.port as u16)
                .await
                .map_err(|e| async_graphql::Error::new(format!("Failed to play cast: {}", e)))?;
            serde_json::json!({ "PlayerState": "PLAYING" })
        }
        SessionCommand::Pause => {
            cast.pause(&device.address, device.port as u16)
                .await
                .map_err(|e| async_graphql::Error::new(format!("Failed to pause cast: {}", e)))?;
            serde_json::json!({ "PlayerState": "PAUSED" })
        }
        SessionCommand::Seek(position) => {
            cast.seek(&device.address, device.port as u16, position)
                .await
                .map_err(|e| async_graphql::Error::new(format!("Failed to seek cast: {}", e)))?;
            serde_json::json!({ "CurrentPosition": position, "LastPosition": position })
        }
        SessionCommand::SetVolume(volume) => {
            cast.set_volume(&device.address, device.port as u16, volume as f32)
                .await
                .map_err(|e| {
                    async_graphql::Error::new(format!("Failed to set cast volume: {}", e))
                })?;
            serde_json::json!({ "Volume": volume })
        }
        SessionCommand::SetMuted(muted) => {
            cast.set_muted(&device.address, device.port as u16, muted)
                .await
                .map_err(|e| async_graphql::Error::new(format!("Failed to mute cast: {}", e)))?;
            serde_json::json!({ "IsMuted": muted })
        }
    };

    let updated = update_cast_session(manager, &auth_user, session_id, update_input).await?;
    Ok(CastSessionOperationResult {
        success: true,
        session: Some(map_session(updated, Some(device.name))),
        error: None,
    })
}

fn map_device(node: DeviceNode) -> LegacyCastDevice {
    LegacyCastDevice {
        id: node.id,
        name: node.name,
        address: node.address,
        port: node.port,
        model: node.model,
        device_type: node.device_type,
        is_favorite: node.is_favorite,
        is_manual: node.is_manual,
        is_connected: false,
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
        stream_url: node.stream_url,
        player_state: node.player_state,
        current_time: node.current_position,
        duration: node.duration,
        volume: node.volume,
        is_muted: node.is_muted,
        started_at: node.started_at,
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

async fn upsert_discovered_device(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    device: &crate::services::cast::DiscoveredCastDevice,
) -> Result<DeviceNode> {
    let existing =
        query_device_by_address_port(manager, auth_user, &device.address, device.port).await?;
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(existing) = existing {
        let updated = update_cast_device(
            manager,
            auth_user,
            &existing.id,
            serde_json::json!({
                "Name": device.name,
                "Model": device.model,
                "DeviceType": device.device_type.as_str(),
                "Address": device.address,
                "Port": device.port,
                "LastSeenAt": now,
            }),
        )
        .await?;
        return Ok(updated);
    }

    create_cast_device(
        manager,
        auth_user,
        serde_json::json!({
            "Name": device.name,
            "Address": device.address,
            "Port": device.port,
            "Model": device.model,
            "DeviceType": device.device_type.as_str(),
            "IsFavorite": false,
            "IsManual": false,
            "LastSeenAt": now,
        }),
    )
    .await
}

async fn end_active_sessions_for_device(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    device_id: &str,
) -> Result<()> {
    let sessions = query_active_sessions_for_device(manager, auth_user, device_id).await?;
    for session in sessions {
        let now = chrono::Utc::now().to_rfc3339();
        let _ = update_cast_session(
            manager,
            auth_user,
            &session.id,
            serde_json::json!({
                "PlayerState": "IDLE",
                "EndedAt": now,
                "LastPosition": session.current_position,
            }),
        )
        .await?;
    }
    Ok(())
}

async fn query_cast_settings_defaults(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
) -> Result<(f64, bool)> {
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        query CastSettingDefaults($Page: PageInput) {
            CastSettings(Page: $Page, OrderBy: [{ UpdatedAt: Desc }]) {
                Edges {
                    Node {
                        DefaultVolume
                    }
                }
            }
        }
        "#,
        serde_json::json!({ "Page": { "Limit": 1, "Offset": 0 } }),
    )
    .await?;

    let volume = data
        .get("CastSettings")
        .and_then(|v| v.get("Edges"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|edge| edge.get("Node"))
        .and_then(|node| node.get("DefaultVolume"))
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    Ok((volume, false))
}

async fn query_latest_cast_setting(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
) -> Result<Option<SettingNode>> {
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        query LatestCastSetting($Page: PageInput, $OrderBy: [CastSettingOrderByInput]) {
            CastSettings(Page: $Page, OrderBy: $OrderBy) {
                Edges {
                    Node {
                        Id
                        AutoDiscoveryEnabled
                        DiscoveryIntervalSeconds
                        DefaultVolume
                        TranscodeIncompatible
                        PreferredQuality
                    }
                }
            }
        }
        "#,
        serde_json::json!({
            "OrderBy": [{ "UpdatedAt": "Desc" }],
            "Page": { "Limit": 1, "Offset": 0 }
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
        query MediaFileById($Id: String!) {
            MediaFile(Id: $Id) {
                Path
                ContentType
            }
        }
        "#,
        serde_json::json!({ "Id": media_file_id }),
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
        query CastDeviceById($Id: String!) {
            CastDevice(Id: $Id) {
                Id
                Name
                Address
                Port
                Model
                DeviceType
                IsFavorite
                IsManual
                LastSeenAt
            }
        }
        "#,
        serde_json::json!({ "Id": device_id }),
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
        query CastDeviceByAddress($Where: CastDeviceWhereInput, $Page: PageInput) {
            CastDevices(Where: $Where, Page: $Page) {
                Edges {
                    Node {
                        Id
                        Name
                        Address
                        Port
                        Model
                        DeviceType
                        IsFavorite
                        IsManual
                        LastSeenAt
                    }
                }
            }
        }
        "#,
        serde_json::json!({
            "Where": {
                "Address": { "Eq": address },
                "Port": { "Eq": port }
            },
            "Page": { "Limit": 1, "Offset": 0 }
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
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        query CastSessionById($Id: String!) {
            CastSession(Id: $Id) {
                Id
                DeviceId
                MediaFileId
                EpisodeId
                StreamUrl
                PlayerState
                CurrentPosition
                Duration
                Volume
                IsMuted
                StartedAt
            }
        }
        "#,
        serde_json::json!({ "Id": session_id }),
    )
    .await?;
    let Some(node) = data.get("CastSession") else {
        return Ok(None);
    };
    Ok(serde_json::from_value(node.clone()).ok())
}

async fn query_active_sessions_for_device(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    device_id: &str,
) -> Result<Vec<SessionNode>> {
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        query ActiveSessionsForDevice($Where: CastSessionWhereInput, $Page: PageInput, $OrderBy: [CastSessionOrderByInput]) {
            CastSessions(Where: $Where, Page: $Page, OrderBy: $OrderBy) {
                Edges {
                    Node {
                        Id
                        DeviceId
                        MediaFileId
                        EpisodeId
                        StreamUrl
                        PlayerState
                        CurrentPosition
                        Duration
                        Volume
                        IsMuted
                        StartedAt
                    }
                }
            }
        }
        "#,
        serde_json::json!({
            "Where": {
                "DeviceId": { "Eq": device_id },
                "EndedAt": { "IsNull": true }
            },
            "OrderBy": [{ "StartedAt": "Desc" }],
            "Page": { "Limit": 20, "Offset": 0 }
        }),
    )
    .await?;

    let mut sessions = Vec::new();
    if let Some(edges) = data
        .get("CastSessions")
        .and_then(|v| v.get("Edges"))
        .and_then(|v| v.as_array())
    {
        for edge in edges {
            if let Some(node) = edge.get("Node").cloned() {
                if let Ok(session) = serde_json::from_value::<SessionNode>(node) {
                    sessions.push(session);
                }
            }
        }
    }
    Ok(sessions)
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
        mutation CreateCastDeviceMutation($Input: CreateCastDeviceInput!) {
            CreateCastDevice(Input: $Input) {
                Success
                Error
                CastDevice {
                    Id
                    Name
                    Address
                    Port
                    Model
                    DeviceType
                    IsFavorite
                    IsManual
                    LastSeenAt
                }
            }
        }
        "#,
        serde_json::json!({ "Input": input }),
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
        mutation DeleteCastDeviceMutation($Id: String!) {
            DeleteCastDevice(Id: $Id) {
                Success
                Error
            }
        }
        "#,
        serde_json::json!({ "Id": id }),
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
        mutation UpdateCastDeviceMutation($Id: String!, $Input: UpdateCastDeviceInput!) {
            UpdateCastDevice(Id: $Id, Input: $Input) {
                Success
                Error
                CastDevice {
                    Id
                    Name
                    Address
                    Port
                    Model
                    DeviceType
                    IsFavorite
                    IsManual
                    LastSeenAt
                }
            }
        }
        "#,
        serde_json::json!({ "Id": id, "Input": input }),
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
    auth_user: &AuthUser,
    args: CreateCastSessionArgs,
) -> Result<SessionNode> {
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        mutation CreateCastSessionMutation($Input: CreateCastSessionInput!) {
            CreateCastSession(Input: $Input) {
                Success
                Error
                CastSession {
                    Id
                    DeviceId
                    MediaFileId
                    EpisodeId
                    StreamUrl
                    PlayerState
                    CurrentPosition
                    Duration
                    Volume
                    IsMuted
                    StartedAt
                }
            }
        }
        "#,
        serde_json::json!({
            "Input": {
                "DeviceId": args.device_id,
                "MediaFileId": args.media_file_id,
                "EpisodeId": args.episode_id,
                "StreamUrl": args.stream_url,
                "PlayerState": args.player_state,
                "CurrentPosition": args.current_position,
                "Duration": args.duration,
                "Volume": args.volume,
                "IsMuted": args.is_muted,
                "StartedAt": chrono::Utc::now().to_rfc3339(),
            }
        }),
    )
    .await?;

    let success = data
        .get("CreateCastSession")
        .and_then(|v| v.get("Success"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !success {
        let error = data
            .get("CreateCastSession")
            .and_then(|v| v.get("Error"))
            .and_then(|v| v.as_str())
            .unwrap_or("Failed to create cast session");
        return Err(async_graphql::Error::new(error.to_string()));
    }

    let node = data
        .get("CreateCastSession")
        .and_then(|v| v.get("CastSession"))
        .cloned()
        .ok_or_else(|| async_graphql::Error::new("Missing CastSession payload"))?;
    serde_json::from_value(node).map_err(|e| async_graphql::Error::new(e.to_string()))
}

async fn update_cast_session(
    manager: &Arc<ServicesManager>,
    auth_user: &AuthUser,
    id: &str,
    input: serde_json::Value,
) -> Result<SessionNode> {
    let data = execute_graphql(
        manager,
        auth_user,
        r#"
        mutation UpdateCastSessionMutation($Id: String!, $Input: UpdateCastSessionInput!) {
            UpdateCastSession(Id: $Id, Input: $Input) {
                Success
                Error
                CastSession {
                    Id
                    DeviceId
                    MediaFileId
                    EpisodeId
                    StreamUrl
                    PlayerState
                    CurrentPosition
                    Duration
                    Volume
                    IsMuted
                    StartedAt
                }
            }
        }
        "#,
        serde_json::json!({ "Id": id, "Input": input }),
    )
    .await?;

    let success = data
        .get("UpdateCastSession")
        .and_then(|v| v.get("Success"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !success {
        let error = data
            .get("UpdateCastSession")
            .and_then(|v| v.get("Error"))
            .and_then(|v| v.as_str())
            .unwrap_or("Failed to update cast session");
        return Err(async_graphql::Error::new(error.to_string()));
    }

    let node = data
        .get("UpdateCastSession")
        .and_then(|v| v.get("CastSession"))
        .cloned()
        .ok_or_else(|| async_graphql::Error::new("Missing CastSession payload"))?;
    serde_json::from_value(node).map_err(|e| async_graphql::Error::new(e.to_string()))
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
        mutation CreateCastSettingMutation($Input: CreateCastSettingInput!) {
            CreateCastSetting(Input: $Input) {
                Success
                Error
                CastSetting {
                    Id
                    AutoDiscoveryEnabled
                    DiscoveryIntervalSeconds
                    DefaultVolume
                    TranscodeIncompatible
                    PreferredQuality
                }
            }
        }
        "#,
        serde_json::json!({ "Input": input }),
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
        mutation UpdateCastSettingMutation($Id: String!, $Input: UpdateCastSettingInput!) {
            UpdateCastSetting(Id: $Id, Input: $Input) {
                Success
                Error
                CastSetting {
                    Id
                    AutoDiscoveryEnabled
                    DiscoveryIntervalSeconds
                    DefaultVolume
                    TranscodeIncompatible
                    PreferredQuality
                }
            }
        }
        "#,
        serde_json::json!({ "Id": id, "Input": input }),
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
        .data(auth_user.clone());
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
