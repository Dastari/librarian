//! Chromecast/Google Cast service
//!
//! This module provides device discovery via mDNS and media casting
//! functionality using the rust_cast library for CASTV2 protocol communication.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use parking_lot::RwLock;
use rust_cast::channels::media::{Media, StreamType};
use rust_cast::channels::receiver::CastDeviceApp;
use rust_cast::CastDevice as RustCastDevice;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::{
    CastDeviceRecord, CastSessionRecord, CreateCastDevice, CreateCastSession, Database,
    UpdateCastSession,
};

/// Service name for Chromecast mDNS discovery
const CHROMECAST_SERVICE_TYPE: &str = "_googlecast._tcp.local.";

/// Default Chromecast port
const DEFAULT_CAST_PORT: u16 = 8009;

/// Cast device types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CastDeviceType {
    Chromecast,
    ChromecastAudio,
    GoogleHome,
    GoogleNestHub,
    AndroidTv,
    Unknown,
}

impl CastDeviceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Chromecast => "chromecast",
            Self::ChromecastAudio => "chromecast_audio",
            Self::GoogleHome => "google_home",
            Self::GoogleNestHub => "google_nest_hub",
            Self::AndroidTv => "android_tv",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_model(model: &str) -> Self {
        let model_lower = model.to_lowercase();
        if model_lower.contains("chromecast audio") {
            Self::ChromecastAudio
        } else if model_lower.contains("chromecast") {
            Self::Chromecast
        } else if model_lower.contains("nest hub") || model_lower.contains("home hub") {
            Self::GoogleNestHub
        } else if model_lower.contains("google home") || model_lower.contains("nest") {
            Self::GoogleHome
        } else if model_lower.contains("android tv") || model_lower.contains("shield") {
            Self::AndroidTv
        } else {
            Self::Unknown
        }
    }
}

impl std::fmt::Display for CastDeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Player state for casting sessions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CastPlayerState {
    Idle,
    Buffering,
    Playing,
    Paused,
}

impl CastPlayerState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Buffering => "buffering",
            Self::Playing => "playing",
            Self::Paused => "paused",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "buffering" => Self::Buffering,
            "playing" => Self::Playing,
            "paused" => Self::Paused,
            _ => Self::Idle,
        }
    }
}

impl std::fmt::Display for CastPlayerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A discovered cast device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredCastDevice {
    pub name: String,
    pub address: IpAddr,
    pub port: u16,
    pub model: Option<String>,
    pub device_type: CastDeviceType,
}

/// Cast session update event for subscriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastSessionEvent {
    pub session_id: Uuid,
    pub device_id: Uuid,
    pub player_state: CastPlayerState,
    pub current_position: f64,
    pub duration: Option<f64>,
    pub volume: f32,
    pub is_muted: bool,
}

/// Cast devices changed event for subscriptions
#[derive(Debug, Clone)]
pub struct CastDevicesEvent {
    pub devices: Vec<CastDeviceRecord>,
}

/// Active connection to a cast device
struct ActiveConnection {
    device_id: Uuid,
    session_id: Option<Uuid>,
    transport_id: String,
}

/// Cast service configuration
#[derive(Debug, Clone)]
pub struct CastServiceConfig {
    /// Base URL for media streaming (e.g., "http://192.168.1.100:3001")
    pub media_base_url: String,
    /// Enable automatic device discovery
    pub auto_discovery: bool,
    /// Discovery interval in seconds
    pub discovery_interval_secs: u64,
}

impl Default for CastServiceConfig {
    fn default() -> Self {
        Self {
            media_base_url: String::new(),
            auto_discovery: true,
            discovery_interval_secs: 30,
        }
    }
}

/// Cast service for managing Chromecast devices and sessions
pub struct CastService {
    db: Database,
    config: CastServiceConfig,
    /// Discovered devices from mDNS (address -> device info)
    discovered_devices: Arc<RwLock<HashMap<IpAddr, DiscoveredCastDevice>>>,
    /// Active connections to devices
    connections: Arc<RwLock<HashMap<Uuid, ActiveConnection>>>,
    /// Broadcast channel for session updates
    session_tx: broadcast::Sender<CastSessionEvent>,
    /// Broadcast channel for device changes
    devices_tx: broadcast::Sender<CastDevicesEvent>,
}

impl CastService {
    /// Create a new cast service
    pub fn new(db: Database, config: CastServiceConfig) -> Self {
        let (session_tx, _) = broadcast::channel(100);
        let (devices_tx, _) = broadcast::channel(100);

        Self {
            db,
            config,
            discovered_devices: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
            session_tx,
            devices_tx,
        }
    }

    /// Subscribe to session update events
    pub fn subscribe_sessions(&self) -> broadcast::Receiver<CastSessionEvent> {
        self.session_tx.subscribe()
    }

    /// Subscribe to device change events
    pub fn subscribe_devices(&self) -> broadcast::Receiver<CastDevicesEvent> {
        self.devices_tx.subscribe()
    }

    /// Start mDNS device discovery
    pub async fn start_discovery(&self) -> Result<()> {
        info!("Starting Chromecast device discovery via mDNS");

        let discovered = self.discovered_devices.clone();
        let db = self.db.clone();
        let devices_tx = self.devices_tx.clone();

        // Spawn discovery task
        tokio::task::spawn_blocking(move || {
            let mdns = match ServiceDaemon::new() {
                Ok(mdns) => mdns,
                Err(e) => {
                    error!("Failed to create mDNS daemon: {}", e);
                    return;
                }
            };

            let receiver = match mdns.browse(CHROMECAST_SERVICE_TYPE) {
                Ok(receiver) => receiver,
                Err(e) => {
                    error!("Failed to browse for Chromecast devices: {}", e);
                    return;
                }
            };

            info!("mDNS discovery started, listening for Chromecast devices");

            loop {
                match receiver.recv_timeout(Duration::from_secs(5)) {
                    Ok(event) => {
                        match event {
                            ServiceEvent::ServiceResolved(info) => {
                                // Extract device info from mDNS TXT records
                                let name = info
                                    .get_properties()
                                    .get("fn")
                                    .map(|v| v.val_str().to_string())
                                    .unwrap_or_else(|| info.get_fullname().to_string());

                                let model = info
                                    .get_properties()
                                    .get("md")
                                    .map(|v| v.val_str().to_string());

                                let device_type = model
                                    .as_ref()
                                    .map(|m| CastDeviceType::from_model(m))
                                    .unwrap_or(CastDeviceType::Unknown);

                                for addr in info.get_addresses() {
                                    let device = DiscoveredCastDevice {
                                        name: name.clone(),
                                        address: *addr,
                                        port: info.get_port(),
                                        model: model.clone(),
                                        device_type,
                                    };

                                    debug!(
                                        "Discovered Chromecast: {} at {}:{}",
                                        device.name, device.address, device.port
                                    );

                                    // Store in discovered devices map
                                    discovered.write().insert(*addr, device.clone());

                                    // Save to database (upsert)
                                    let db_clone = db.clone();
                                    let devices_tx_clone = devices_tx.clone();
                                    let device_clone = device.clone();
                                    
                                    tokio::spawn(async move {
                                        if let Err(e) = Self::save_discovered_device(&db_clone, &device_clone).await {
                                            warn!("Failed to save discovered device: {}", e);
                                        }
                                        
                                        // Broadcast device change
                                        if let Ok(devices) = db_clone.cast().list_devices().await {
                                            let _ = devices_tx_clone.send(CastDevicesEvent { devices });
                                        }
                                    });
                                }
                            }
                            ServiceEvent::ServiceRemoved(_, fullname) => {
                                debug!("Chromecast removed: {}", fullname);
                            }
                            _ => {}
                        }
                    }
                    Err(flume::RecvTimeoutError::Timeout) => {
                        // Continue listening
                    }
                    Err(flume::RecvTimeoutError::Disconnected) => {
                        warn!("mDNS receiver disconnected, stopping discovery");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Save a discovered device to the database
    async fn save_discovered_device(db: &Database, device: &DiscoveredCastDevice) -> Result<()> {
        let input = CreateCastDevice {
            name: device.name.clone(),
            address: device.address.to_string(),
            port: device.port as i32,
            model: device.model.clone(),
            device_type: device.device_type.to_string(),
            is_manual: false,
        };
        db.cast().upsert_device(input).await?;
        Ok(())
    }

    /// Manually add a cast device by IP address
    pub async fn add_device_manual(
        &self,
        address: IpAddr,
        port: Option<u16>,
        name: Option<String>,
    ) -> Result<CastDeviceRecord> {
        let port = port.unwrap_or(DEFAULT_CAST_PORT);
        let name = name.unwrap_or_else(|| format!("Cast Device ({})", address));

        // Try to connect to get device info
        let model = match self.probe_device(address, port).await {
            Ok(info) => info,
            Err(e) => {
                warn!("Could not probe device at {}:{}: {}", address, port, e);
                None
            }
        };

        let device_type = model
            .as_ref()
            .map(|m| CastDeviceType::from_model(m))
            .unwrap_or(CastDeviceType::Unknown);

        let input = CreateCastDevice {
            name,
            address: address.to_string(),
            port: port as i32,
            model,
            device_type: device_type.to_string(),
            is_manual: true,
        };

        let device = self.db.cast().create_device(input).await?;
        info!("Added manual cast device: {} at {}:{}", device.name, address, port);

        // Broadcast device change
        if let Ok(devices) = self.db.cast().list_devices().await {
            let _ = self.devices_tx.send(CastDevicesEvent { devices });
        }

        Ok(device)
    }

    /// Probe a device to get its model info
    async fn probe_device(&self, address: IpAddr, port: u16) -> Result<Option<String>> {
        let addr_str = address.to_string();
        
        tokio::task::spawn_blocking(move || {
            let device = RustCastDevice::connect_without_host_verification(&addr_str, port)
                .context("Failed to connect to cast device")?;
            
            // Get receiver status to determine device model
            let status = device.receiver.get_status()
                .context("Failed to get receiver status")?;
            
            // The status contains volume info but not model directly
            // Model info comes from mDNS TXT records, so return None here
            drop(status);
            Ok(None)
        })
        .await?
    }

    /// Get all cast devices (from database)
    pub async fn get_devices(&self) -> Result<Vec<CastDeviceRecord>> {
        self.db.cast().list_devices().await
    }

    /// Get a cast device by ID
    pub async fn get_device(&self, id: Uuid) -> Result<Option<CastDeviceRecord>> {
        self.db.cast().get_device(id).await
    }

    /// Remove a cast device
    pub async fn remove_device(&self, id: Uuid) -> Result<bool> {
        // End any active sessions for this device
        self.db.cast().end_sessions_for_device(id).await?;
        
        // Remove from connections
        self.connections.write().remove(&id);
        
        let result = self.db.cast().delete_device(id).await?;
        
        // Broadcast device change
        if let Ok(devices) = self.db.cast().list_devices().await {
            let _ = self.devices_tx.send(CastDevicesEvent { devices });
        }
        
        Ok(result)
    }

    /// Update a cast device (name, favorite status)
    pub async fn update_device(
        &self,
        id: Uuid,
        name: Option<String>,
        is_favorite: Option<bool>,
    ) -> Result<Option<CastDeviceRecord>> {
        let input = crate::db::UpdateCastDevice { name, is_favorite };
        self.db.cast().update_device(id, input).await
    }

    /// Get active cast sessions
    pub async fn get_active_sessions(&self) -> Result<Vec<CastSessionRecord>> {
        self.db.cast().list_active_sessions().await
    }

    /// Get a cast session by ID
    pub async fn get_session(&self, id: Uuid) -> Result<Option<CastSessionRecord>> {
        self.db.cast().get_session(id).await
    }

    /// Cast media to a device
    pub async fn cast_media(
        &self,
        device_id: Uuid,
        media_file_id: Uuid,
        episode_id: Option<Uuid>,
        start_position: Option<f64>,
    ) -> Result<CastSessionRecord> {
        let device = self
            .db
            .cast()
            .get_device(device_id)
            .await?
            .context("Cast device not found")?;

        let media_file = self
            .db
            .media_files()
            .get_by_id(media_file_id)
            .await?
            .context("Media file not found")?;

        // Generate stream URL
        let stream_url = format!(
            "{}/api/media/{}/stream",
            self.config.media_base_url,
            media_file_id
        );

        // End any existing session on this device
        self.db.cast().end_sessions_for_device(device_id).await?;

        // Create new session in database
        let session = self
            .db
            .cast()
            .create_session(CreateCastSession {
                device_id,
                media_file_id: Some(media_file_id),
                episode_id,
                stream_url: stream_url.clone(),
            })
            .await?;

        // Connect to device and start casting
        let addr = device.address.to_string();
        let port = device.port as u16;
        let session_id = session.id;
        let db = self.db.clone();
        let session_tx = self.session_tx.clone();
        let start_pos = start_position.unwrap_or(0.0);

        // Determine media type from file
        let content_type = Self::get_content_type(&media_file.path);

        tokio::task::spawn_blocking(move || {
            match Self::cast_media_blocking(&addr, port, &stream_url, &content_type, start_pos) {
                Ok((transport_id, duration)) => {
                    info!("Started casting to {} (session: {})", addr, session_id);
                    
                    // Update session with duration and playing state
                    let db_clone = db.clone();
                    let session_id_clone = session_id;
                    tokio::spawn(async move {
                        let input = UpdateCastSession {
                            player_state: Some("playing".to_string()),
                            duration: duration,
                            current_position: Some(start_pos),
                            ..Default::default()
                        };
                        if let Err(e) = db_clone.cast().update_session(session_id_clone, input).await {
                            error!("Failed to update session: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to cast media: {}", e);
                    // End the session on error
                    let db_clone = db.clone();
                    tokio::spawn(async move {
                        let _ = db_clone.cast().end_session(session_id).await;
                    });
                }
            }
        });

        Ok(session)
    }

    /// Cast media (blocking, runs in spawn_blocking)
    fn cast_media_blocking(
        addr: &str,
        port: u16,
        stream_url: &str,
        content_type: &str,
        start_position: f64,
    ) -> Result<(String, Option<f64>)> {
        let device = RustCastDevice::connect_without_host_verification(addr, port)
            .context("Failed to connect to cast device")?;

        // Connect to receiver
        device
            .connection
            .connect("receiver-0")
            .context("Failed to connect to receiver")?;

        // Launch default media receiver
        let app = device
            .receiver
            .launch_app(&CastDeviceApp::DefaultMediaReceiver)
            .context("Failed to launch media receiver")?;

        let transport_id = app.transport_id.clone();

        // Connect to the media app
        device
            .connection
            .connect(&transport_id)
            .context("Failed to connect to media app")?;

        // Load media
        let media = Media {
            content_id: stream_url.to_string(),
            content_type: content_type.to_string(),
            stream_type: StreamType::Buffered,
            duration: None,
            metadata: None,
        };

        device
            .media
            .load(&transport_id, &app.session_id, &media)
            .context("Failed to load media")?;

        // Seek to start position if specified
        if start_position > 0.0 {
            // Wait a moment for media to load
            std::thread::sleep(Duration::from_millis(500));
            // Get media session ID from status
            if let Ok(status) = device.media.get_status(&transport_id, None) {
                if let Some(entry) = status.entries.first() {
                    let _ = device.media.seek(&transport_id, entry.media_session_id, Some(start_position as f32), None);
                }
            }
        }

        // Get media status to get duration
        let duration = device.media.get_status(&transport_id, None)
            .ok()
            .and_then(|s| {
                s.entries.first()
                    .and_then(|e| e.media.as_ref())
                    .and_then(|m| m.duration)
                    .map(|d| d as f64)
            });

        Ok((transport_id, duration))
    }

    /// Get content type from file path
    fn get_content_type(path: &str) -> String {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "mp4" | "m4v" => "video/mp4",
            "mkv" => "video/x-matroska",
            "webm" => "video/webm",
            "avi" => "video/x-msvideo",
            "mov" => "video/quicktime",
            "wmv" => "video/x-ms-wmv",
            "mp3" => "audio/mpeg",
            "flac" => "audio/flac",
            "aac" => "audio/aac",
            "m4a" => "audio/mp4",
            _ => "video/mp4",
        }
        .to_string()
    }

    /// Play the current media (resume)
    pub async fn play(&self, session_id: Uuid) -> Result<CastSessionRecord> {
        let session = self
            .db
            .cast()
            .get_session(session_id)
            .await?
            .context("Session not found")?;

        let device = self
            .db
            .cast()
            .get_device(session.device_id.context("No device for session")?)
            .await?
            .context("Device not found")?;

        let addr = device.address.to_string();
        let port = device.port as u16;

        tokio::task::spawn_blocking(move || {
            Self::control_playback_blocking(&addr, port, PlaybackCommand::Play)
        })
        .await??;

        let input = UpdateCastSession {
            player_state: Some("playing".to_string()),
            ..Default::default()
        };
        
        let updated = self
            .db
            .cast()
            .update_session(session_id, input)
            .await?
            .context("Failed to update session")?;

        self.broadcast_session_update(&updated);
        Ok(updated)
    }

    /// Pause the current media
    pub async fn pause(&self, session_id: Uuid) -> Result<CastSessionRecord> {
        let session = self
            .db
            .cast()
            .get_session(session_id)
            .await?
            .context("Session not found")?;

        let device = self
            .db
            .cast()
            .get_device(session.device_id.context("No device for session")?)
            .await?
            .context("Device not found")?;

        let addr = device.address.to_string();
        let port = device.port as u16;

        tokio::task::spawn_blocking(move || {
            Self::control_playback_blocking(&addr, port, PlaybackCommand::Pause)
        })
        .await??;

        let input = UpdateCastSession {
            player_state: Some("paused".to_string()),
            ..Default::default()
        };
        
        let updated = self
            .db
            .cast()
            .update_session(session_id, input)
            .await?
            .context("Failed to update session")?;

        self.broadcast_session_update(&updated);
        Ok(updated)
    }

    /// Stop casting and end the session
    pub async fn stop(&self, session_id: Uuid) -> Result<()> {
        let session = self
            .db
            .cast()
            .get_session(session_id)
            .await?
            .context("Session not found")?;

        if let Some(device_id) = session.device_id {
            if let Ok(Some(device)) = self.db.cast().get_device(device_id).await {
                let addr = device.address.to_string();
                let port = device.port as u16;

                let _ = tokio::task::spawn_blocking(move || {
                    Self::control_playback_blocking(&addr, port, PlaybackCommand::Stop)
                })
                .await;
            }
        }

        self.db.cast().end_session(session_id).await?;
        info!("Stopped cast session: {}", session_id);
        Ok(())
    }

    /// Seek to a position in the media
    pub async fn seek(&self, session_id: Uuid, position: f64) -> Result<CastSessionRecord> {
        let session = self
            .db
            .cast()
            .get_session(session_id)
            .await?
            .context("Session not found")?;

        let device = self
            .db
            .cast()
            .get_device(session.device_id.context("No device for session")?)
            .await?
            .context("Device not found")?;

        let addr = device.address.to_string();
        let port = device.port as u16;

        tokio::task::spawn_blocking(move || {
            Self::control_playback_blocking(&addr, port, PlaybackCommand::Seek(position))
        })
        .await??;

        let input = UpdateCastSession {
            current_position: Some(position),
            ..Default::default()
        };
        
        let updated = self
            .db
            .cast()
            .update_session(session_id, input)
            .await?
            .context("Failed to update session")?;

        self.broadcast_session_update(&updated);
        Ok(updated)
    }

    /// Set volume level (0.0 - 1.0)
    pub async fn set_volume(&self, session_id: Uuid, volume: f32) -> Result<CastSessionRecord> {
        let session = self
            .db
            .cast()
            .get_session(session_id)
            .await?
            .context("Session not found")?;

        let device = self
            .db
            .cast()
            .get_device(session.device_id.context("No device for session")?)
            .await?
            .context("Device not found")?;

        let addr = device.address.to_string();
        let port = device.port as u16;
        let vol = volume.clamp(0.0, 1.0);

        tokio::task::spawn_blocking(move || {
            Self::control_volume_blocking(&addr, port, vol, None)
        })
        .await??;

        let input = UpdateCastSession {
            volume: Some(vol),
            ..Default::default()
        };
        
        let updated = self
            .db
            .cast()
            .update_session(session_id, input)
            .await?
            .context("Failed to update session")?;

        self.broadcast_session_update(&updated);
        Ok(updated)
    }

    /// Set mute state
    pub async fn set_muted(&self, session_id: Uuid, muted: bool) -> Result<CastSessionRecord> {
        let session = self
            .db
            .cast()
            .get_session(session_id)
            .await?
            .context("Session not found")?;

        let device = self
            .db
            .cast()
            .get_device(session.device_id.context("No device for session")?)
            .await?
            .context("Device not found")?;

        let addr = device.address.to_string();
        let port = device.port as u16;

        tokio::task::spawn_blocking(move || {
            Self::control_volume_blocking(&addr, port, 0.0, Some(muted))
        })
        .await??;

        let input = UpdateCastSession {
            is_muted: Some(muted),
            ..Default::default()
        };
        
        let updated = self
            .db
            .cast()
            .update_session(session_id, input)
            .await?
            .context("Failed to update session")?;

        self.broadcast_session_update(&updated);
        Ok(updated)
    }

    /// Broadcast a session update event
    fn broadcast_session_update(&self, session: &CastSessionRecord) {
        if let Some(device_id) = session.device_id {
            let event = CastSessionEvent {
                session_id: session.id,
                device_id,
                player_state: CastPlayerState::from_str(&session.player_state),
                current_position: session.current_position,
                duration: session.duration,
                volume: session.volume,
                is_muted: session.is_muted,
            };
            let _ = self.session_tx.send(event);
        }
    }

    /// Control playback (blocking)
    fn control_playback_blocking(addr: &str, port: u16, command: PlaybackCommand) -> Result<()> {
        let device = RustCastDevice::connect_without_host_verification(addr, port)
            .context("Failed to connect to cast device")?;

        // Connect to receiver
        device.connection.connect("receiver-0")?;
        
        let status = device.receiver.get_status()?;
        let app = status
            .applications
            .first()
            .context("No running application")?;

        let transport_id = &app.transport_id;

        device.connection.connect(transport_id)?;

        // Get media status to find media_session_id
        let media_status = device.media.get_status(transport_id, None)?;
        let media_session_id = media_status
            .entries
            .first()
            .map(|e| e.media_session_id)
            .context("No media session")?;

        match command {
            PlaybackCommand::Play => {
                device.media.play(transport_id, media_session_id)?;
            }
            PlaybackCommand::Pause => {
                device.media.pause(transport_id, media_session_id)?;
            }
            PlaybackCommand::Stop => {
                device.media.stop(transport_id, media_session_id)?;
            }
            PlaybackCommand::Seek(position) => {
                device.media.seek(transport_id, media_session_id, Some(position as f32), None)?;
            }
        }

        Ok(())
    }

    /// Control volume (blocking)
    fn control_volume_blocking(addr: &str, port: u16, volume: f32, muted: Option<bool>) -> Result<()> {
        let device = RustCastDevice::connect_without_host_verification(addr, port)
            .context("Failed to connect to cast device")?;

        device.connection.connect("receiver-0")?;

        if let Some(muted) = muted {
            device.receiver.set_volume(muted)?;
        } else {
            device.receiver.set_volume(volume)?;
        }

        Ok(())
    }

    /// Get cast settings
    pub async fn get_settings(&self) -> Result<Option<crate::db::CastSettingsRecord>> {
        self.db.cast().get_settings().await
    }

    /// Update cast settings
    pub async fn update_settings(
        &self,
        input: crate::db::UpdateCastSettings,
    ) -> Result<crate::db::CastSettingsRecord> {
        self.db.cast().update_settings(input).await
    }
}

/// Playback command enum
enum PlaybackCommand {
    Play,
    Pause,
    Stop,
    Seek(f64),
}
