use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use async_trait::async_trait;
use mdns_sd::{DaemonStatus, ServiceDaemon, ServiceEvent};
use parking_lot::RwLock;
use rust_cast::CastDevice as RustCastDevice;
use rust_cast::channels::media::{Media, StreamType};
use rust_cast::channels::receiver::CastDeviceApp;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as TokioMutex, RwLock as TokioRwLock, Semaphore};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};
use url::Host;

use super::{CastGrantClaims, CastGrantSigner};
use crate::services::manager::{Service, ServiceHealth, ServicesManager};

const CHROMECAST_SERVICE_TYPE: &str = "_googlecast._tcp.local.";
const DLNA_MULTICAST_ADDR: &str = "239.255.255.250:1900";
const CAST_COMMAND_TIMEOUT: Duration = Duration::from_secs(15);
const CAST_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const CAST_IO_TIMEOUT: Duration = Duration::from_secs(10);

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

#[derive(Clone, Debug)]
pub struct CastLaunchResult {
    pub duration: Option<f64>,
    pub receiver_transport_id: String,
    pub receiver_session_id: String,
    pub media_session_id: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CastPlaybackMode {
    Direct,
    Remux,
    Transcode,
    Unsupported,
}

impl CastPlaybackMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "DIRECT",
            Self::Remux => "REMUX",
            Self::Transcode => "TRANSCODE",
            Self::Unsupported => "UNSUPPORTED",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CastPlaybackDecision {
    pub mode: CastPlaybackMode,
    pub reason: &'static str,
}

#[derive(Clone, Debug)]
pub struct CastRemoteStatus {
    pub player_state: String,
    pub current_position: f64,
    pub duration: Option<f64>,
    pub volume: f64,
    pub is_muted: bool,
}

#[derive(Clone)]
pub struct CastServiceConfig {
    pub media_base_url: String,
    pub auto_discovery: bool,
    pub discovery_interval_secs: u64,
    pub discovery_timeout_ms: u64,
    pub discovery_retention_days: u64,
    pub grant_secret: Vec<u8>,
}

impl std::fmt::Debug for CastServiceConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CastServiceConfig")
            .field("media_base_url", &self.media_base_url)
            .field("auto_discovery", &self.auto_discovery)
            .field("discovery_interval_secs", &self.discovery_interval_secs)
            .field("discovery_timeout_ms", &self.discovery_timeout_ms)
            .field("discovery_retention_days", &self.discovery_retention_days)
            .field("grant_secret", &"[REDACTED]")
            .finish()
    }
}

impl Default for CastServiceConfig {
    fn default() -> Self {
        Self {
            media_base_url: "http://127.0.0.1:3001".to_string(),
            auto_discovery: true,
            discovery_interval_secs: 30,
            discovery_timeout_ms: 1500,
            discovery_retention_days: 30,
            grant_secret: b"test-only-cast-grant-secret".to_vec(),
        }
    }
}

pub struct CastService {
    manager: Weak<ServicesManager>,
    config: CastServiceConfig,
    runtime_settings: TokioRwLock<CastRuntimeSettings>,
    discovered_devices: Arc<RwLock<HashMap<String, DiscoveredCastDevice>>>,
    discovery_task: TokioRwLock<Option<JoinHandle<()>>>,
    mdns_daemon: Arc<TokioRwLock<Option<ServiceDaemon>>>,
    mdns_discovery_lock: Arc<TokioMutex<()>>,
    grant_signer: CastGrantSigner,
    command_slots: Arc<Semaphore>,
    device_command_locks: Arc<TokioRwLock<HashMap<String, Arc<TokioMutex<()>>>>>,
    discovery_status: Arc<RwLock<DiscoveryStatus>>,
    session_monitors: TokioMutex<HashMap<String, JoinHandle<()>>>,
}

#[derive(Clone, Copy, Debug)]
struct CastRuntimeSettings {
    auto_discovery: bool,
    discovery_interval_secs: u64,
    discovery_timeout_ms: u64,
}

#[derive(Clone, Debug, Default)]
struct DiscoveryStatus {
    last_success_at: Option<String>,
    last_error: Option<String>,
}

impl CastService {
    pub fn new(manager: Weak<ServicesManager>, config: CastServiceConfig) -> Self {
        let grant_signer = CastGrantSigner::derive(&config.grant_secret);
        let runtime_settings = CastRuntimeSettings {
            auto_discovery: config.auto_discovery,
            discovery_interval_secs: config.discovery_interval_secs,
            discovery_timeout_ms: config.discovery_timeout_ms,
        };
        Self {
            manager,
            config,
            runtime_settings: TokioRwLock::new(runtime_settings),
            discovered_devices: Arc::new(RwLock::new(HashMap::new())),
            discovery_task: TokioRwLock::new(None),
            mdns_daemon: Arc::new(TokioRwLock::new(None)),
            mdns_discovery_lock: Arc::new(TokioMutex::new(())),
            grant_signer,
            command_slots: Arc::new(Semaphore::new(4)),
            device_command_locks: Arc::new(TokioRwLock::new(HashMap::new())),
            discovery_status: Arc::new(RwLock::new(DiscoveryStatus::default())),
            session_monitors: TokioMutex::new(HashMap::new()),
        }
    }

    pub fn media_base_url(&self) -> &str {
        &self.config.media_base_url
    }

    pub fn validate_advertised_media_url(&self) -> Result<()> {
        let url = reqwest::Url::parse(&self.config.media_base_url)
            .context("Advertised Cast media URL is invalid")?;
        if !matches!(url.scheme(), "http" | "https") {
            anyhow::bail!("Advertised Cast media URL must use HTTP or HTTPS");
        }
        match url
            .host()
            .context("Advertised Cast media URL has no host")?
        {
            Host::Ipv4(address) if address.is_loopback() || address.is_unspecified() => {
                anyhow::bail!(
                    "Advertised Cast media URL points at a loopback/unspecified address; configure LIBRARIAN_ADVERTISED_MEDIA_URL"
                )
            }
            Host::Ipv6(address) if address.is_loopback() || address.is_unspecified() => {
                anyhow::bail!(
                    "Advertised Cast media URL points at a loopback/unspecified address; configure LIBRARIAN_ADVERTISED_MEDIA_URL"
                )
            }
            _ => Ok(()),
        }
    }

    pub fn mint_grant(&self, claims: &CastGrantClaims) -> Result<String> {
        self.grant_signer.sign(claims)
    }

    pub fn verify_grant(&self, grant: &str, now_unix: i64) -> Result<CastGrantClaims> {
        self.grant_signer.verify(grant, now_unix)
    }

    pub fn validate_target(address: &str, port: i32) -> Result<(IpAddr, u16)> {
        let port = u16::try_from(port)
            .ok()
            .filter(|port| *port > 0)
            .context("Cast port must be between 1 and 65535")?;
        let address: IpAddr = address
            .trim()
            .parse()
            .context("Cast address must be a literal LAN IP address")?;
        let allowed = match address {
            IpAddr::V4(address) => address.is_private() || address.is_link_local(),
            IpAddr::V6(address) => {
                address.is_unicast_link_local() || (address.segments()[0] & 0xfe00) == 0xfc00
            }
        };
        if !allowed || address.is_loopback() || address.is_unspecified() || address.is_multicast() {
            anyhow::bail!("Cast destination is outside the allowed LAN address ranges");
        }
        Ok((address, port))
    }

    pub fn validate_position(position: f64) -> Result<f64> {
        if !position.is_finite() || position < 0.0 {
            anyhow::bail!("Cast playback position must be a finite non-negative number");
        }
        Ok(position)
    }

    pub fn validate_volume(volume: f64) -> Result<f32> {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            anyhow::bail!("Cast volume must be a finite number between 0 and 1");
        }
        Ok(volume as f32)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn decide_playback(
        device_type: CastDeviceType,
        model: Option<&str>,
        container: Option<&str>,
        video_codec: Option<&str>,
        audio_codec: Option<&str>,
        source_height: Option<i32>,
        is_hdr: bool,
        transcode_incompatible: bool,
        preferred_quality: Option<&str>,
    ) -> CastPlaybackDecision {
        if device_type == CastDeviceType::ChromecastAudio && video_codec.is_some() {
            return CastPlaybackDecision {
                mode: CastPlaybackMode::Unsupported,
                reason: "audio-only receiver cannot play a video stream",
            };
        }

        let requested_height = preferred_quality.and_then(|quality| {
            match quality.trim().to_ascii_lowercase().as_str() {
                "2160p" => Some(2160),
                "1080p" => Some(1080),
                "720p" => Some(720),
                "480p" => Some(480),
                _ => None,
            }
        });
        if requested_height
            .zip(source_height)
            .is_some_and(|(target, source)| source > target)
        {
            return if transcode_incompatible {
                CastPlaybackDecision {
                    mode: CastPlaybackMode::Transcode,
                    reason: "source exceeds the configured Cast quality limit",
                }
            } else {
                CastPlaybackDecision {
                    mode: CastPlaybackMode::Unsupported,
                    reason: "quality reduction requires Cast transcoding, which is disabled",
                }
            };
        }

        let container = container.map(str::to_ascii_lowercase);
        let video = video_codec.map(str::to_ascii_lowercase);
        let audio = audio_codec.map(str::to_ascii_lowercase);
        let enhanced_model = model.is_some_and(|value| {
            let value = value.to_ascii_lowercase();
            value.contains("ultra")
                || value.contains("google tv")
                || value.contains("chromecast hd")
        });
        let video_direct = match video.as_deref() {
            None => true,
            Some(value)
                if value.contains("h264")
                    || value.contains("avc")
                    || value.contains("vp8")
                    || value.contains("vp9") =>
            {
                true
            }
            Some(value) if value.contains("hevc") || value.contains("h265") => {
                enhanced_model && !is_hdr
            }
            Some(_) => false,
        };
        let audio_direct = match audio.as_deref() {
            None => true,
            Some(value)
                if ["aac", "mp3", "opus", "vorbis", "flac"]
                    .iter()
                    .any(|codec| value.contains(codec)) =>
            {
                true
            }
            Some(_) => false,
        };
        // FFmpeg reports WebM as the shared `matroska,webm` demuxer. Treating
        // the mere presence of `webm` as direct-play capable would therefore
        // incorrectly direct-play H.264/AAC MKV files. The WebM family is
        // direct only when its codecs also conform to WebM.
        let container_direct = container.as_deref().is_some_and(|value| {
            let has = |expected: &str| {
                value
                    .split(',')
                    .any(|part| part.trim().eq_ignore_ascii_case(expected))
            };
            let mp4_family = ["mp4", "mov", "m4a"].iter().any(|name| has(name));
            let audio_family = ["mp3", "flac", "wav", "ogg"].iter().any(|name| has(name));
            let webm_video = video.as_deref().is_none_or(|codec| {
                codec.contains("vp8") || codec.contains("vp9") || codec.contains("av1")
            });
            let webm_audio = audio
                .as_deref()
                .is_none_or(|codec| codec.contains("opus") || codec.contains("vorbis"));
            mp4_family || audio_family || (has("webm") && webm_video && webm_audio)
        });
        let analysis_known = container.is_some() && (video.is_some() || audio.is_some());

        if analysis_known && container_direct && video_direct && audio_direct {
            CastPlaybackDecision {
                mode: CastPlaybackMode::Direct,
                reason: "container and codecs match the receiver capability profile",
            }
        } else if analysis_known && video_direct && audio_direct {
            CastPlaybackDecision {
                mode: CastPlaybackMode::Remux,
                reason: "codecs are compatible but the container requires HLS remuxing",
            }
        } else if transcode_incompatible {
            CastPlaybackDecision {
                mode: CastPlaybackMode::Transcode,
                reason: if analysis_known {
                    "one or more codecs require a conservative HLS transcode"
                } else {
                    "media analysis is incomplete; conservative Cast transcode selected"
                },
            }
        } else {
            CastPlaybackDecision {
                mode: CastPlaybackMode::Unsupported,
                reason: if analysis_known {
                    "receiver compatibility requires transcoding, which is disabled"
                } else {
                    "media analysis is incomplete and Cast transcoding is disabled"
                },
            }
        }
    }

    pub async fn discover_now(&self) -> Result<Vec<DiscoveredCastDevice>> {
        let timeout_ms = self.runtime_settings.read().await.discovery_timeout_ms;
        let devices = match self.discover_once(timeout_ms).await {
            Ok(devices) => {
                let mut status = self.discovery_status.write();
                status.last_success_at = Some(chrono::Utc::now().to_rfc3339());
                status.last_error = None;
                devices
            }
            Err(error) => {
                self.discovery_status.write().last_error =
                    Some("Cast discovery scan failed".to_string());
                return Err(error);
            }
        };
        {
            let mut guard = self.discovered_devices.write();
            for device in &devices {
                let key = format!("{}:{}", device.address, device.port);
                guard.insert(key, device.clone());
            }
        }
        Ok(devices)
    }

    pub async fn apply_runtime_settings(
        &self,
        auto_discovery: bool,
        discovery_interval_secs: u64,
    ) -> Result<()> {
        if !(5..=86_400).contains(&discovery_interval_secs) {
            anyhow::bail!("Cast discovery interval must be between 5 and 86400 seconds");
        }
        {
            let mut settings = self.runtime_settings.write().await;
            settings.auto_discovery = auto_discovery;
            settings.discovery_interval_secs = discovery_interval_secs;
        }
        if let Some(handle) = self.discovery_task.write().await.take() {
            handle.abort();
        }
        if auto_discovery {
            self.start_discovery_loop().await;
        }
        Ok(())
    }

    async fn start_discovery_loop(&self) {
        let settings = *self.runtime_settings.read().await;
        if !settings.auto_discovery {
            return;
        }
        let mut guard = self.discovery_task.write().await;
        if guard.is_some() {
            return;
        }
        let discovered_devices = Arc::clone(&self.discovered_devices);
        let mdns_daemon = Arc::clone(&self.mdns_daemon);
        let mdns_discovery_lock = Arc::clone(&self.mdns_discovery_lock);
        let discovery_status = Arc::clone(&self.discovery_status);
        let manager = self.manager.clone();
        let discovery_retention_days = self.config.discovery_retention_days;
        *guard = Some(tokio::spawn(async move {
            let mut ticker =
                tokio::time::interval(Duration::from_secs(settings.discovery_interval_secs.max(5)));
            loop {
                ticker.tick().await;
                match CastService::discover_once_with(
                    Arc::clone(&mdns_daemon),
                    Arc::clone(&mdns_discovery_lock),
                    settings.discovery_timeout_ms.max(250),
                )
                .await
                {
                    Ok(found) => {
                        {
                            let mut status = discovery_status.write();
                            status.last_success_at = Some(chrono::Utc::now().to_rfc3339());
                            status.last_error = None;
                        }
                        persist_discovered_devices(&manager, &found, discovery_retention_days)
                            .await;
                        let mut map = discovered_devices.write();
                        for device in found {
                            let key = format!("{}:{}", device.address, device.port);
                            map.insert(key, device);
                        }
                    }
                    Err(err) => {
                        discovery_status.write().last_error =
                            Some("Cast auto-discovery scan failed".to_string());
                        warn!(error = %err, "Cast auto-discovery scan failed");
                    }
                }
            }
        }));
    }

    pub async fn discovered_devices(&self) -> Vec<DiscoveredCastDevice> {
        self.discovered_devices.read().values().cloned().collect()
    }

    pub fn infer_content_type(path: &str, content_type: Option<&str>) -> String {
        if let Some(ct) = content_type {
            let trimmed = ct.trim();
            if Self::is_mime_content_type(trimmed) {
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

    fn is_mime_content_type(value: &str) -> bool {
        let essence = value.split(';').next().unwrap_or_default().trim();
        let Some((media_type, subtype)) = essence.split_once('/') else {
            return false;
        };
        let is_token = |part: &str| {
            !part.is_empty()
                && part.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric()
                        || matches!(
                            byte,
                            b'!' | b'#'
                                | b'$'
                                | b'%'
                                | b'&'
                                | b'\''
                                | b'*'
                                | b'+'
                                | b'-'
                                | b'.'
                                | b'^'
                                | b'_'
                                | b'`'
                                | b'|'
                                | b'~'
                        )
                })
        };
        is_token(media_type) && is_token(subtype)
    }

    pub async fn cast_media(
        &self,
        address: &str,
        port: u16,
        stream_url: &str,
        content_type: &str,
        start_position: f64,
    ) -> Result<CastLaunchResult> {
        Self::validate_position(start_position)?;
        let stream_url = stream_url.to_string();
        let content_type = content_type.to_string();
        self.run_device_command(address, port, move |address, port| {
            Self::cast_media_blocking(&address, port, &stream_url, &content_type, start_position)
        })
        .await
    }

    pub async fn play(
        &self,
        address: &str,
        port: u16,
        transport_id: &str,
        media_session_id: i32,
    ) -> Result<()> {
        let transport_id = transport_id.to_string();
        self.run_device_command(address, port, move |address, port| {
            Self::control_playback_blocking(&address, port, &transport_id, media_session_id, "play")
        })
        .await
    }

    pub async fn pause(
        &self,
        address: &str,
        port: u16,
        transport_id: &str,
        media_session_id: i32,
    ) -> Result<()> {
        let transport_id = transport_id.to_string();
        self.run_device_command(address, port, move |address, port| {
            Self::control_playback_blocking(
                &address,
                port,
                &transport_id,
                media_session_id,
                "pause",
            )
        })
        .await
    }

    pub async fn stop(
        &self,
        address: &str,
        port: u16,
        transport_id: &str,
        media_session_id: i32,
    ) -> Result<()> {
        let transport_id = transport_id.to_string();
        self.run_device_command(address, port, move |address, port| {
            Self::control_playback_blocking(&address, port, &transport_id, media_session_id, "stop")
        })
        .await
    }

    pub async fn seek(
        &self,
        address: &str,
        port: u16,
        transport_id: &str,
        media_session_id: i32,
        position: f64,
    ) -> Result<()> {
        Self::validate_position(position)?;
        let transport_id = transport_id.to_string();
        self.run_device_command(address, port, move |address, port| {
            Self::control_seek_blocking(&address, port, &transport_id, media_session_id, position)
        })
        .await
    }

    pub async fn set_volume(&self, address: &str, port: u16, volume: f32) -> Result<()> {
        let volume = Self::validate_volume(volume as f64)?;
        self.run_device_command(address, port, move |address, port| {
            Self::control_volume_blocking(&address, port, volume)
        })
        .await
    }

    pub async fn set_muted(&self, address: &str, port: u16, muted: bool) -> Result<()> {
        self.run_device_command(address, port, move |address, port| {
            Self::control_mute_blocking(&address, port, muted)
        })
        .await
    }

    pub async fn session_status(
        &self,
        address: &str,
        port: u16,
        transport_id: &str,
        media_session_id: i32,
    ) -> Result<CastRemoteStatus> {
        let transport_id = transport_id.to_string();
        self.run_device_command(address, port, move |address, port| {
            Self::session_status_blocking(&address, port, &transport_id, media_session_id)
        })
        .await
    }

    pub async fn install_session_monitor(
        &self,
        session_id: String,
        monitor: impl std::future::Future<Output = ()> + Send + 'static,
    ) {
        let mut monitors = self.session_monitors.lock().await;
        if let Some(existing) = monitors.remove(&session_id) {
            existing.abort();
        }
        monitors.insert(session_id, tokio::spawn(monitor));
    }

    async fn run_device_command<T, F>(&self, address: &str, port: u16, command: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(String, u16) -> Result<T> + Send + 'static,
    {
        let (address, port) = Self::validate_target(address, i32::from(port))?;
        let address = address.to_string();
        let key = format!("{address}:{port}");
        let device_lock = {
            let locks = self.device_command_locks.read().await;
            locks.get(&key).cloned()
        };
        let device_lock = match device_lock {
            Some(lock) => lock,
            None => {
                let mut locks = self.device_command_locks.write().await;
                Arc::clone(
                    locks
                        .entry(key)
                        .or_insert_with(|| Arc::new(TokioMutex::new(()))),
                )
            }
        };

        let permit = Arc::clone(&self.command_slots)
            .acquire_owned()
            .await
            .context("Cast command executor is unavailable")?;
        let device_guard = device_lock.lock_owned().await;
        let task = tokio::task::spawn_blocking(move || {
            // These guards intentionally live inside the blocking task. If the
            // async wait times out, the detached operation continues to consume
            // its global slot and per-device lock until it really exits.
            let _permit = permit;
            let _device_guard = device_guard;
            command(address, port)
        });

        match tokio::time::timeout(CAST_COMMAND_TIMEOUT, task).await {
            Ok(joined) => joined.context("Cast command worker failed")?,
            Err(_) => anyhow::bail!(
                "Cast command exceeded the {} second deadline",
                CAST_COMMAND_TIMEOUT.as_secs()
            ),
        }
    }

    async fn discover_once(&self, timeout_ms: u64) -> Result<Vec<DiscoveredCastDevice>> {
        Self::discover_once_with(
            Arc::clone(&self.mdns_daemon),
            Arc::clone(&self.mdns_discovery_lock),
            timeout_ms,
        )
        .await
    }

    async fn discover_once_with(
        mdns_daemon: Arc<TokioRwLock<Option<ServiceDaemon>>>,
        mdns_discovery_lock: Arc<TokioMutex<()>>,
        timeout_ms: u64,
    ) -> Result<Vec<DiscoveredCastDevice>> {
        let mut out = Vec::new();
        let mdns =
            Self::discover_chromecast_mdns(mdns_daemon, mdns_discovery_lock, timeout_ms).await?;
        out.extend(mdns);
        let dlna = Self::discover_dlna_ssdp(timeout_ms).await?;
        out.extend(dlna);
        Ok(out)
    }

    async fn discover_chromecast_mdns(
        mdns_daemon: Arc<TokioRwLock<Option<ServiceDaemon>>>,
        mdns_discovery_lock: Arc<TokioMutex<()>>,
        timeout_ms: u64,
    ) -> Result<Vec<DiscoveredCastDevice>> {
        let _discovery_guard = mdns_discovery_lock.lock().await;
        let daemon = Self::get_or_create_mdns_daemon(&mdns_daemon).await?;
        let result = Self::browse_chromecast_mdns(daemon, timeout_ms).await;
        if result.is_err() {
            Self::reset_mdns_daemon(&mdns_daemon).await;
        }
        result
    }

    async fn get_or_create_mdns_daemon(
        mdns_daemon: &Arc<TokioRwLock<Option<ServiceDaemon>>>,
    ) -> Result<ServiceDaemon> {
        if let Some(daemon) = mdns_daemon.read().await.as_ref().cloned() {
            return Ok(daemon);
        }

        let mut guard = mdns_daemon.write().await;
        if let Some(daemon) = guard.as_ref().cloned() {
            return Ok(daemon);
        }

        let daemon =
            ServiceDaemon::new().context("Failed to create Chromecast mDNS discovery daemon")?;
        *guard = Some(daemon.clone());
        debug!(
            service_type = CHROMECAST_SERVICE_TYPE,
            "Created Chromecast mDNS discovery daemon"
        );
        Ok(daemon)
    }

    async fn browse_chromecast_mdns(
        daemon: ServiceDaemon,
        timeout_ms: u64,
    ) -> Result<Vec<DiscoveredCastDevice>> {
        tokio::task::spawn_blocking(move || -> Result<Vec<DiscoveredCastDevice>> {
            let mut devices = Vec::new();
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

            if let Err(err) = daemon.stop_browse(CHROMECAST_SERVICE_TYPE) {
                warn!(
                    service_type = CHROMECAST_SERVICE_TYPE,
                    error = %err,
                    "Failed to stop Chromecast mDNS browse after discovery scan"
                );
            }

            Ok(devices)
        })
        .await?
    }

    async fn reset_mdns_daemon(mdns_daemon: &Arc<TokioRwLock<Option<ServiceDaemon>>>) {
        let daemon = {
            let mut guard = mdns_daemon.write().await;
            guard.take()
        };
        if let Some(daemon) = daemon {
            Self::shutdown_mdns_daemon(daemon).await;
        }
    }

    async fn shutdown_mdns_daemon(daemon: ServiceDaemon) {
        match tokio::task::spawn_blocking(move || -> Result<()> {
            let receiver = daemon
                .shutdown()
                .context("Failed to request Chromecast mDNS daemon shutdown")?;
            match receiver.recv_timeout(Duration::from_secs(2)) {
                Ok(DaemonStatus::Shutdown) => {
                    debug!("Chromecast mDNS discovery daemon shut down");
                }
                Ok(status) => {
                    warn!(
                        status = ?status,
                        "Chromecast mDNS daemon returned unexpected shutdown status"
                    );
                }
                Err(err) => {
                    warn!(
                        error = %err,
                        "Timed out waiting for Chromecast mDNS daemon shutdown"
                    );
                }
            }
            Ok(())
        })
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                warn!(
                    error = %err,
                    "Failed to shut down Chromecast mDNS discovery daemon"
                );
            }
            Err(err) => {
                warn!(
                    error = %err,
                    "Failed to join Chromecast mDNS daemon shutdown task"
                );
            }
        }
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
                        let Some((address, port)) =
                            validated_ssdp_location_endpoint(&response, src.ip())
                        else {
                            warn!(
                                source_address = %src.ip(),
                                "Ignoring DLNA response without a safe same-device LOCATION"
                            );
                            continue;
                        };
                        devices.push(DiscoveredCastDevice {
                            name,
                            address: address.to_string(),
                            port: i32::from(port),
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
    ) -> Result<CastLaunchResult> {
        let device = RustCastDevice::connect_without_host_verification_with_timeouts(
            address,
            port,
            CAST_CONNECT_TIMEOUT,
            CAST_IO_TIMEOUT,
        )
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

        let status = device
            .media
            .load(&transport_id, &app.session_id, &media)
            .context("Failed to load media on cast device")?;
        let media_session_id = status
            .entries
            .first()
            .map(|entry| entry.media_session_id)
            .context("Cast receiver did not return a media session identifier")?;

        if start_position > 0.0 {
            device
                .media
                .seek(
                    &transport_id,
                    media_session_id,
                    Some(start_position as f32),
                    None,
                )
                .context("Failed to seek newly launched Cast media")?;
        }

        let duration = status
            .entries
            .first()
            .and_then(|entry| entry.media.as_ref())
            .and_then(|media| media.duration)
            .map(f64::from)
            .or_else(|| {
                device
                    .media
                    .get_status(&transport_id, None)
                    .ok()
                    .and_then(|s| {
                        s.entries
                            .first()
                            .and_then(|e| e.media.as_ref())
                            .and_then(|m| m.duration)
                            .map(|d| d as f64)
                    })
            });
        Ok(CastLaunchResult {
            duration,
            receiver_transport_id: transport_id,
            receiver_session_id: app.session_id,
            media_session_id,
        })
    }

    fn control_playback_blocking(
        address: &str,
        port: u16,
        transport_id: &str,
        media_session_id: i32,
        command: &str,
    ) -> Result<()> {
        let device = RustCastDevice::connect_without_host_verification_with_timeouts(
            address,
            port,
            CAST_CONNECT_TIMEOUT,
            CAST_IO_TIMEOUT,
        )
        .context("Failed to connect to cast device")?;
        device.connection.connect("receiver-0")?;
        device.connection.connect(transport_id)?;

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

    fn control_seek_blocking(
        address: &str,
        port: u16,
        transport_id: &str,
        media_session_id: i32,
        position: f64,
    ) -> Result<()> {
        let device = RustCastDevice::connect_without_host_verification_with_timeouts(
            address,
            port,
            CAST_CONNECT_TIMEOUT,
            CAST_IO_TIMEOUT,
        )
        .context("Failed to connect to cast device")?;
        device.connection.connect("receiver-0")?;
        device.connection.connect(transport_id)?;
        device
            .media
            .seek(transport_id, media_session_id, Some(position as f32), None)?;
        Ok(())
    }

    fn control_volume_blocking(address: &str, port: u16, volume: f32) -> Result<()> {
        let device = RustCastDevice::connect_without_host_verification_with_timeouts(
            address,
            port,
            CAST_CONNECT_TIMEOUT,
            CAST_IO_TIMEOUT,
        )
        .context("Failed to connect to cast device")?;
        device.connection.connect("receiver-0")?;
        device.receiver.set_volume(volume)?;
        Ok(())
    }

    fn control_mute_blocking(address: &str, port: u16, muted: bool) -> Result<()> {
        let device = RustCastDevice::connect_without_host_verification_with_timeouts(
            address,
            port,
            CAST_CONNECT_TIMEOUT,
            CAST_IO_TIMEOUT,
        )
        .context("Failed to connect to cast device")?;
        device.connection.connect("receiver-0")?;
        // Send both current level and muted state; some receivers ignore muted-only updates.
        let current_level = device.receiver.get_status()?.volume.level.unwrap_or(1.0);
        device.receiver.set_volume((current_level, muted))?;
        Ok(())
    }

    fn session_status_blocking(
        address: &str,
        port: u16,
        transport_id: &str,
        media_session_id: i32,
    ) -> Result<CastRemoteStatus> {
        let device = RustCastDevice::connect_without_host_verification_with_timeouts(
            address,
            port,
            CAST_CONNECT_TIMEOUT,
            CAST_IO_TIMEOUT,
        )
        .context("Failed to connect to cast device")?;
        device.connection.connect("receiver-0")?;
        device.connection.connect(transport_id)?;
        let media = device
            .media
            .get_status(transport_id, Some(media_session_id))?;
        let entry = media
            .entries
            .iter()
            .find(|entry| entry.media_session_id == media_session_id)
            .context("The receiver no longer has this media session")?;
        let receiver = device.receiver.get_status()?;
        Ok(CastRemoteStatus {
            player_state: entry.player_state.to_string(),
            current_position: entry.current_time.map(f64::from).unwrap_or(0.0),
            duration: entry
                .media
                .as_ref()
                .and_then(|media| media.duration)
                .map(f64::from),
            volume: receiver.volume.level.map(f64::from).unwrap_or(1.0),
            is_muted: receiver.volume.muted.unwrap_or(false),
        })
    }
}

async fn persist_discovered_devices(
    manager: &Weak<ServicesManager>,
    devices: &[DiscoveredCastDevice],
    retention_days: u64,
) {
    let Some(manager) = manager.upgrade() else {
        return;
    };
    let actor = crate::graphql::AuthUser::system_admin();
    for device in devices {
        if let Err(error) =
            crate::graphql::entities::cast::persist_discovered_device(&manager, &actor, device)
                .await
        {
            warn!(
                cast_address = %device.address,
                cast_port = device.port,
                error = ?error,
                "Failed to persist automatically discovered Cast device"
            );
        }
    }
    if let Err(error) = crate::graphql::entities::cast::prune_stale_discovered_devices(
        &manager,
        &actor,
        retention_days,
    )
    .await
    {
        warn!(
            error = ?error,
            retention_days,
            "Failed to prune stale automatically discovered Cast devices"
        );
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
        let settings = *self.runtime_settings.read().await;
        if !(5..=86_400).contains(&settings.discovery_interval_secs) {
            anyhow::bail!("Cast discovery interval must be between 5 and 86400 seconds");
        }
        if !(250..=30_000).contains(&settings.discovery_timeout_ms) {
            anyhow::bail!("Cast discovery timeout must be between 250 and 30000 milliseconds");
        }
        if !(1..=3_650).contains(&self.config.discovery_retention_days) {
            anyhow::bail!("Cast discovery retention must be between 1 and 3650 days");
        }
        if !settings.auto_discovery {
            info!("Cast service started (auto discovery disabled)");
            return Ok(());
        }

        self.start_discovery_loop().await;

        info!("Cast service started (auto discovery enabled)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        for (_, monitor) in self.session_monitors.lock().await.drain() {
            monitor.abort();
        }
        if let Some(handle) = self.discovery_task.write().await.take() {
            handle.abort();
        }
        let _discovery_guard = self.mdns_discovery_lock.lock().await;
        Self::reset_mdns_daemon(&self.mdns_daemon).await;
        info!("Cast service stopped");
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        let settings = *self.runtime_settings.read().await;
        if settings.auto_discovery
            && self
                .discovery_task
                .read()
                .await
                .as_ref()
                .is_none_or(JoinHandle::is_finished)
        {
            return Ok(ServiceHealth::degraded(
                "Cast discovery worker exited unexpectedly",
            ));
        }
        let devices = self.discovered_devices.read().len();
        let status = self.discovery_status.read().clone();
        let message = format!(
            "{} discovered device(s); advertised media origin {}; last successful discovery {}",
            devices,
            self.config.media_base_url,
            status.last_success_at.as_deref().unwrap_or("never")
        );
        Ok(if let Some(error) = status.last_error {
            ServiceHealth::degraded(format!("{message}; {error}"))
        } else {
            ServiceHealth::healthy().with_message(message)
        })
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

fn validated_ssdp_location_endpoint(response: &str, source_ip: IpAddr) -> Option<(IpAddr, u16)> {
    let location = extract_ssdp_header(response, "LOCATION")?;
    let url = reqwest::Url::parse(&location).ok()?;
    if url.scheme() != "http" {
        return None;
    }
    let location_ip = match url.host()? {
        Host::Ipv4(address) => IpAddr::V4(address),
        Host::Ipv6(address) => IpAddr::V6(address),
        Host::Domain(_) => return None,
    };
    // SSDP is unauthenticated. Keep the descriptor endpoint on the packet
    // source so discovery cannot nominate an arbitrary LAN target.
    if location_ip != source_ip {
        return None;
    }
    let port = url.port_or_known_default()?;
    CastService::validate_target(&location_ip.to_string(), i32::from(port)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_content_type_uses_valid_mime_value() {
        assert_eq!(
            CastService::infer_content_type("movie.mkv", Some(" video/mp4 ")),
            "video/mp4"
        );
    }

    #[test]
    fn infer_content_type_replaces_generic_media_kind() {
        assert_eq!(
            CastService::infer_content_type("movie.mkv", Some("video")),
            "video/x-matroska"
        );
        assert_eq!(
            CastService::infer_content_type("song.flac", Some("audio")),
            "audio/flac"
        );
    }

    #[test]
    fn receiver_target_validation_rejects_unsafe_addresses_and_ports() {
        for address in [
            "127.0.0.1",
            "0.0.0.0",
            "8.8.8.8",
            "224.0.0.1",
            "::1",
            "::",
            "ff02::1",
            "example.com",
        ] {
            assert!(
                CastService::validate_target(address, 8009).is_err(),
                "{address} must not be accepted as a Cast receiver target"
            );
        }

        for port in [-1, 0, 65_536, i32::MAX] {
            assert!(
                CastService::validate_target("192.168.1.10", port).is_err(),
                "port {port} must not wrap or be accepted"
            );
        }
    }

    #[test]
    fn receiver_target_validation_accepts_only_lan_ip_literals() {
        assert_eq!(
            CastService::validate_target("192.168.1.10", 8009)
                .expect("private IPv4 should be accepted")
                .1,
            8009
        );
        assert!(CastService::validate_target("169.254.10.20", 8009).is_ok());
        assert!(CastService::validate_target("fd00::10", 8009).is_ok());
        assert!(CastService::validate_target("fe80::10", 8009).is_ok());
    }

    #[test]
    fn playback_number_validation_rejects_non_finite_and_out_of_range_values() {
        for position in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(CastService::validate_position(position).is_err());
        }
        assert_eq!(
            CastService::validate_position(0.0).expect("zero position should be accepted"),
            0.0
        );

        for volume in [-0.1, 1.1, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(CastService::validate_volume(volume).is_err());
        }
        assert_eq!(
            CastService::validate_volume(1.0).expect("maximum volume should be accepted"),
            1.0
        );
    }

    #[test]
    fn cast_playback_decision_covers_direct_remux_transcode_and_unsupported() {
        let direct = CastService::decide_playback(
            CastDeviceType::Chromecast,
            Some("Chromecast"),
            Some("mov,mp4,m4a"),
            Some("h264"),
            Some("aac"),
            Some(1080),
            false,
            false,
            Some("original"),
        );
        assert_eq!(direct.mode, CastPlaybackMode::Direct);

        let remux = CastService::decide_playback(
            CastDeviceType::Chromecast,
            Some("Chromecast"),
            Some("matroska,webm"),
            Some("h264"),
            Some("aac"),
            Some(1080),
            false,
            false,
            Some("original"),
        );
        assert_eq!(remux.mode, CastPlaybackMode::Remux);

        let transcode = CastService::decide_playback(
            CastDeviceType::Chromecast,
            None,
            Some("matroska,webm"),
            Some("hevc"),
            Some("dts"),
            Some(2160),
            true,
            true,
            Some("1080p"),
        );
        assert_eq!(transcode.mode, CastPlaybackMode::Transcode);

        let unsupported = CastService::decide_playback(
            CastDeviceType::Chromecast,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            Some("original"),
        );
        assert_eq!(unsupported.mode, CastPlaybackMode::Unsupported);
    }

    #[test]
    fn cast_audio_receiver_rejects_video() {
        let decision = CastService::decide_playback(
            CastDeviceType::ChromecastAudio,
            Some("Chromecast Audio"),
            Some("mp4"),
            Some("h264"),
            Some("aac"),
            Some(1080),
            false,
            true,
            Some("original"),
        );
        assert_eq!(decision.mode, CastPlaybackMode::Unsupported);
    }

    #[test]
    fn ssdp_uses_safe_location_endpoint_instead_of_udp_source_port() {
        let response = concat!(
            "HTTP/1.1 200 OK\r\n",
            "ST: urn:schemas-upnp-org:device:MediaRenderer:1\r\n",
            "LOCATION: http://192.168.1.25:1400/xml/device.xml\r\n",
            "\r\n"
        );
        assert_eq!(
            validated_ssdp_location_endpoint(response, "192.168.1.25".parse().expect("test IP")),
            Some(("192.168.1.25".parse().expect("test IP"), 1400))
        );
        assert!(
            validated_ssdp_location_endpoint(response, "192.168.1.26".parse().expect("test IP"))
                .is_none()
        );
    }

    #[cfg(target_os = "linux")]
    fn mdns_daemon_thread_count() -> usize {
        std::fs::read_dir("/proc/self/task")
            .ok()
            .into_iter()
            .flat_map(|entries| entries.filter_map(std::result::Result::ok))
            .filter_map(|entry| std::fs::read_to_string(entry.path().join("comm")).ok())
            .filter(|name| name.trim() == "mDNS_daemon")
            .count()
    }

    #[cfg(target_os = "linux")]
    fn wait_for_mdns_daemon_thread_count(target: usize) -> usize {
        let until = Instant::now() + Duration::from_secs(2);
        let mut observed = mdns_daemon_thread_count();
        while observed != target && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(25));
            observed = mdns_daemon_thread_count();
        }
        observed
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn cast_service_reuses_one_mdns_daemon_thread() {
        let mdns_daemon = Arc::new(TokioRwLock::new(None));
        let before = mdns_daemon_thread_count();

        let _first = CastService::get_or_create_mdns_daemon(&mdns_daemon)
            .await
            .expect("first mDNS daemon should be created");
        let after_first = wait_for_mdns_daemon_thread_count(before + 1);
        assert_eq!(
            after_first,
            before + 1,
            "creating the cast mDNS daemon should add exactly one daemon thread"
        );

        let _second = CastService::get_or_create_mdns_daemon(&mdns_daemon)
            .await
            .expect("existing mDNS daemon should be reused");
        std::thread::sleep(Duration::from_millis(50));
        let after_second = mdns_daemon_thread_count();
        assert_eq!(
            after_second, after_first,
            "reusing the cast mDNS daemon must not spawn another daemon thread"
        );

        CastService::reset_mdns_daemon(&mdns_daemon).await;
        let after_shutdown = wait_for_mdns_daemon_thread_count(before);
        assert_eq!(
            after_shutdown, before,
            "stopping the cast service should shut down the mDNS daemon thread"
        );
    }
}
