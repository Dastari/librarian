use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use async_trait::async_trait;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use parking_lot::RwLock;
use rust_cast::CastDevice as RustCastDevice;
use rust_cast::channels::media::{Media, StreamType};
use rust_cast::channels::receiver::CastDeviceApp;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as TokioRwLock;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::services::manager::{Service, ServiceHealth};

const CHROMECAST_SERVICE_TYPE: &str = "_googlecast._tcp.local.";
const DLNA_MULTICAST_ADDR: &str = "239.255.255.250:1900";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CastDeviceType {
    Chromecast,
    ChromecastAudio,
    DlnaRenderer,
    Unknown,
}

impl CastDeviceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Chromecast => "CHROMECAST",
            Self::ChromecastAudio => "CHROMECAST_AUDIO",
            Self::DlnaRenderer => "DLNA_RENDERER",
            Self::Unknown => "UNKNOWN",
        }
    }

    pub fn from_model(model: Option<&str>) -> Self {
        let Some(model) = model else {
            return Self::Unknown;
        };
        let lower = model.to_ascii_lowercase();
        if lower.contains("chromecast audio") {
            Self::ChromecastAudio
        } else if lower.contains("chromecast") {
            Self::Chromecast
        } else if lower.contains("dlna") || lower.contains("upnp") {
            Self::DlnaRenderer
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredCastDevice {
    pub name: String,
    pub address: String,
    pub port: i32,
    pub model: Option<String>,
    pub device_type: CastDeviceType,
    pub last_seen_at: String,
}

#[derive(Debug, Clone)]
pub struct CastServiceConfig {
    pub media_base_url: String,
    pub auto_discovery: bool,
    pub discovery_interval_secs: u64,
    pub discovery_timeout_ms: u64,
}

impl Default for CastServiceConfig {
    fn default() -> Self {
        Self {
            media_base_url: "http://127.0.0.1:3001".to_string(),
            auto_discovery: true,
            discovery_interval_secs: 30,
            discovery_timeout_ms: 1500,
        }
    }
}

pub struct CastService {
    config: CastServiceConfig,
    discovered_devices: Arc<RwLock<HashMap<String, DiscoveredCastDevice>>>,
    discovery_task: TokioRwLock<Option<JoinHandle<()>>>,
}

impl CastService {
    pub fn new(config: CastServiceConfig) -> Self {
        Self {
            config,
            discovered_devices: Arc::new(RwLock::new(HashMap::new())),
            discovery_task: TokioRwLock::new(None),
        }
    }

    pub fn media_base_url(&self) -> &str {
        &self.config.media_base_url
    }

    pub async fn discover_now(&self) -> Result<Vec<DiscoveredCastDevice>> {
        let devices = Self::discover_once_blocking(self.config.discovery_timeout_ms).await?;
        {
            let mut guard = self.discovered_devices.write();
            for device in &devices {
                let key = format!("{}:{}", device.address, device.port);
                guard.insert(key, device.clone());
            }
        }
        Ok(devices)
    }

    pub async fn discovered_devices(&self) -> Vec<DiscoveredCastDevice> {
        self.discovered_devices.read().values().cloned().collect()
    }

    pub fn infer_content_type(path: &str, content_type: Option<&str>) -> String {
        if let Some(ct) = content_type {
            let trimmed = ct.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }

        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

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

    pub async fn cast_media(
        &self,
        address: &str,
        port: u16,
        stream_url: &str,
        content_type: &str,
        start_position: f64,
    ) -> Result<Option<f64>> {
        let address = address.to_string();
        let stream_url = stream_url.to_string();
        let content_type = content_type.to_string();
        tokio::task::spawn_blocking(move || {
            Self::cast_media_blocking(&address, port, &stream_url, &content_type, start_position)
        })
        .await?
    }

    pub async fn play(&self, address: &str, port: u16) -> Result<()> {
        let address = address.to_string();
        tokio::task::spawn_blocking(move || Self::control_playback_blocking(&address, port, "play"))
            .await?
    }

    pub async fn pause(&self, address: &str, port: u16) -> Result<()> {
        let address = address.to_string();
        tokio::task::spawn_blocking(move || {
            Self::control_playback_blocking(&address, port, "pause")
        })
        .await?
    }

    pub async fn stop(&self, address: &str, port: u16) -> Result<()> {
        let address = address.to_string();
        tokio::task::spawn_blocking(move || Self::control_playback_blocking(&address, port, "stop"))
            .await?
    }

    pub async fn seek(&self, address: &str, port: u16, position: f64) -> Result<()> {
        let address = address.to_string();
        tokio::task::spawn_blocking(move || Self::control_seek_blocking(&address, port, position))
            .await?
    }

    pub async fn set_volume(&self, address: &str, port: u16, volume: f32) -> Result<()> {
        let address = address.to_string();
        tokio::task::spawn_blocking(move || {
            Self::control_volume_blocking(&address, port, volume.clamp(0.0, 1.0))
        })
        .await?
    }

    pub async fn set_muted(&self, address: &str, port: u16, muted: bool) -> Result<()> {
        let address = address.to_string();
        tokio::task::spawn_blocking(move || Self::control_mute_blocking(&address, port, muted))
            .await?
    }

    async fn discover_once_blocking(timeout_ms: u64) -> Result<Vec<DiscoveredCastDevice>> {
        let mut out = Vec::new();
        let mdns = Self::discover_chromecast_mdns(timeout_ms).await?;
        out.extend(mdns);
        let dlna = Self::discover_dlna_ssdp(timeout_ms).await?;
        out.extend(dlna);
        Ok(out)
    }

    async fn discover_chromecast_mdns(timeout_ms: u64) -> Result<Vec<DiscoveredCastDevice>> {
        tokio::task::spawn_blocking(move || -> Result<Vec<DiscoveredCastDevice>> {
            let mut devices = Vec::new();
            let daemon = ServiceDaemon::new().context("Failed to create mDNS daemon")?;
            let receiver = daemon
                .browse(CHROMECAST_SERVICE_TYPE)
                .context("Failed to browse Chromecast mDNS service")?;

            let until = Instant::now() + Duration::from_millis(timeout_ms);
            while Instant::now() < until {
                let remaining = until.saturating_duration_since(Instant::now());
                let wait = remaining.min(Duration::from_millis(250));
                match receiver.recv_timeout(wait) {
                    Ok(ServiceEvent::ServiceResolved(info)) => {
                        let model = info
                            .get_properties()
                            .get("md")
                            .map(|v| v.val_str().to_string());
                        let name = info
                            .get_properties()
                            .get("fn")
                            .map(|v| v.val_str().to_string())
                            .unwrap_or_else(|| info.get_fullname().to_string());
                        let device_type = CastDeviceType::from_model(model.as_deref());
                        for address in info.get_addresses() {
                            devices.push(DiscoveredCastDevice {
                                name: name.clone(),
                                address: address.to_string(),
                                port: info.get_port() as i32,
                                model: model.clone(),
                                device_type,
                                last_seen_at: chrono::Utc::now().to_rfc3339(),
                            });
                        }
                    }
                    Ok(_) => {}
                    Err(flume::RecvTimeoutError::Timeout) => {}
                    Err(flume::RecvTimeoutError::Disconnected) => break,
                }
            }

            Ok(devices)
        })
        .await?
    }

    async fn discover_dlna_ssdp(timeout_ms: u64) -> Result<Vec<DiscoveredCastDevice>> {
        tokio::task::spawn_blocking(move || -> Result<Vec<DiscoveredCastDevice>> {
            let mut devices = Vec::new();
            let socket = UdpSocket::bind("0.0.0.0:0").context("Failed to bind SSDP socket")?;
            socket
                .set_read_timeout(Some(Duration::from_millis(200)))
                .context("Failed to set SSDP timeout")?;

            let search = concat!(
                "M-SEARCH * HTTP/1.1\r\n",
                "HOST: 239.255.255.250:1900\r\n",
                "MAN: \"ssdp:discover\"\r\n",
                "MX: 1\r\n",
                "ST: urn:schemas-upnp-org:device:MediaRenderer:1\r\n",
                "\r\n"
            );
            socket
                .send_to(search.as_bytes(), DLNA_MULTICAST_ADDR)
                .context("Failed to send SSDP discovery")?;

            let until = Instant::now() + Duration::from_millis(timeout_ms);
            let mut buf = [0u8; 4096];
            while Instant::now() < until {
                match socket.recv_from(&mut buf) {
                    Ok((len, src)) => {
                        let response = String::from_utf8_lossy(&buf[..len]).to_string();
                        if !response.to_ascii_lowercase().contains("mediarenderer") {
                            continue;
                        }
                        let model = extract_ssdp_header(&response, "SERVER");
                        let name = extract_ssdp_header(&response, "USN")
                            .or_else(|| extract_ssdp_header(&response, "SERVER"))
                            .unwrap_or_else(|| format!("DLNA Renderer ({})", src.ip()));
                        let port = src.port() as i32;
                        devices.push(DiscoveredCastDevice {
                            name,
                            address: src.ip().to_string(),
                            port,
                            model,
                            device_type: CastDeviceType::DlnaRenderer,
                            last_seen_at: chrono::Utc::now().to_rfc3339(),
                        });
                    }
                    Err(err)
                        if err.kind() == std::io::ErrorKind::WouldBlock
                            || err.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(err) => return Err(err).context("Failed reading SSDP response"),
                }
            }

            Ok(devices)
        })
        .await?
    }

    fn cast_media_blocking(
        address: &str,
        port: u16,
        stream_url: &str,
        content_type: &str,
        start_position: f64,
    ) -> Result<Option<f64>> {
        let device = RustCastDevice::connect_without_host_verification(address, port)
            .context("Failed to connect to cast device")?;

        device
            .connection
            .connect("receiver-0")
            .context("Failed to connect to receiver")?;

        let app = device
            .receiver
            .launch_app(&CastDeviceApp::DefaultMediaReceiver)
            .context("Failed to launch media receiver")?;
        let transport_id = app.transport_id.clone();

        device
            .connection
            .connect(&transport_id)
            .context("Failed to connect to media app")?;

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
            .context("Failed to load media on cast device")?;

        if start_position > 0.0 {
            std::thread::sleep(Duration::from_millis(500));
            if let Ok(status) = device.media.get_status(&transport_id, None) {
                if let Some(entry) = status.entries.first() {
                    let _ = device.media.seek(
                        &transport_id,
                        entry.media_session_id,
                        Some(start_position as f32),
                        None,
                    );
                }
            }
        }

        let duration = device
            .media
            .get_status(&transport_id, None)
            .ok()
            .and_then(|s| {
                s.entries
                    .first()
                    .and_then(|e| e.media.as_ref())
                    .and_then(|m| m.duration)
                    .map(|d| d as f64)
            });
        Ok(duration)
    }

    fn control_playback_blocking(address: &str, port: u16, command: &str) -> Result<()> {
        let device = RustCastDevice::connect_without_host_verification(address, port)
            .context("Failed to connect to cast device")?;
        device.connection.connect("receiver-0")?;

        let status = device.receiver.get_status()?;
        let app = status
            .applications
            .first()
            .context("No running cast application")?;
        let transport_id = &app.transport_id;
        device.connection.connect(transport_id)?;

        let media_status = device.media.get_status(transport_id, None)?;
        let media_session_id = media_status
            .entries
            .first()
            .map(|e| e.media_session_id)
            .context("No active cast media session")?;

        match command {
            "play" => {
                let _ = device.media.play(transport_id, media_session_id)?;
            }
            "pause" => {
                let _ = device.media.pause(transport_id, media_session_id)?;
            }
            "stop" => {
                let _ = device.media.stop(transport_id, media_session_id)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn control_seek_blocking(address: &str, port: u16, position: f64) -> Result<()> {
        let device = RustCastDevice::connect_without_host_verification(address, port)
            .context("Failed to connect to cast device")?;
        device.connection.connect("receiver-0")?;
        let status = device.receiver.get_status()?;
        let app = status
            .applications
            .first()
            .context("No running cast application")?;
        let transport_id = &app.transport_id;
        device.connection.connect(transport_id)?;
        let media_status = device.media.get_status(transport_id, None)?;
        let media_session_id = media_status
            .entries
            .first()
            .map(|e| e.media_session_id)
            .context("No active cast media session")?;
        device
            .media
            .seek(transport_id, media_session_id, Some(position as f32), None)?;
        Ok(())
    }

    fn control_volume_blocking(address: &str, port: u16, volume: f32) -> Result<()> {
        let device = RustCastDevice::connect_without_host_verification(address, port)
            .context("Failed to connect to cast device")?;
        device.connection.connect("receiver-0")?;
        device.receiver.set_volume(volume)?;
        Ok(())
    }

    fn control_mute_blocking(address: &str, port: u16, muted: bool) -> Result<()> {
        let device = RustCastDevice::connect_without_host_verification(address, port)
            .context("Failed to connect to cast device")?;
        device.connection.connect("receiver-0")?;
        // Send both current level and muted state; some receivers ignore muted-only updates.
        let current_level = device.receiver.get_status()?.volume.level.unwrap_or(1.0);
        device.receiver.set_volume((current_level, muted))?;
        Ok(())
    }
}

#[async_trait]
impl Service for CastService {
    fn name(&self) -> &str {
        "cast"
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["graphql".to_string()]
    }

    async fn start(&self) -> Result<()> {
        if !self.config.auto_discovery {
            info!("Cast service started (auto discovery disabled)");
            return Ok(());
        }

        let mut guard = self.discovery_task.write().await;
        if guard.is_some() {
            return Ok(());
        }

        let discovered_devices = Arc::clone(&self.discovered_devices);
        let interval_secs = self.config.discovery_interval_secs.max(5);
        let timeout_ms = self.config.discovery_timeout_ms.max(250);
        *guard = Some(tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
            loop {
                ticker.tick().await;
                match CastService::discover_once_blocking(timeout_ms).await {
                    Ok(found) => {
                        let mut map = discovered_devices.write();
                        for device in found {
                            let key = format!("{}:{}", device.address, device.port);
                            map.insert(key, device);
                        }
                    }
                    Err(err) => {
                        warn!(error = %err, "Cast auto-discovery scan failed");
                    }
                }
            }
        }));

        info!("Cast service started (auto discovery enabled)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        if let Some(handle) = self.discovery_task.write().await.take() {
            handle.abort();
        }
        info!("Cast service stopped");
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        let devices = self.discovered_devices.read().len();
        Ok(ServiceHealth::healthy().with_message(format!("{} discovered device(s)", devices)))
    }
}

trait ServiceHealthExt {
    fn with_message(self, message: String) -> ServiceHealth;
}

impl ServiceHealthExt for ServiceHealth {
    fn with_message(mut self, message: String) -> ServiceHealth {
        self.message = Some(message);
        self
    }
}

fn extract_ssdp_header(response: &str, header: &str) -> Option<String> {
    let needle = format!("{}:", header);
    for line in response.lines() {
        if line
            .to_ascii_lowercase()
            .starts_with(&needle.to_ascii_lowercase())
        {
            return line
                .split_once(':')
                .map(|(_, value)| value.trim().to_string());
        }
    }
    None
}
