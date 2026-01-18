//! GraphQL type definitions
//!
//! These types mirror our domain models but are decorated with async-graphql attributes.

use async_graphql::{Enum, InputObject, Object, SimpleObject};
use serde::{Deserialize, Serialize};

use crate::services::{
    PeerStats as ServicePeerStats, TorrentDetails as ServiceTorrentDetails,
    TorrentInfo as ServiceTorrentInfo, TorrentState as ServiceTorrentState,
};

/// Torrent download state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum TorrentState {
    Queued,
    Checking,
    Downloading,
    Seeding,
    Paused,
    Error,
}

impl From<ServiceTorrentState> for TorrentState {
    fn from(state: ServiceTorrentState) -> Self {
        match state {
            ServiceTorrentState::Queued => TorrentState::Queued,
            ServiceTorrentState::Checking => TorrentState::Checking,
            ServiceTorrentState::Downloading => TorrentState::Downloading,
            ServiceTorrentState::Seeding => TorrentState::Seeding,
            ServiceTorrentState::Paused => TorrentState::Paused,
            ServiceTorrentState::Error => TorrentState::Error,
        }
    }
}

/// A file within a torrent
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct TorrentFile {
    /// File index within the torrent
    pub index: i32,
    /// File path relative to torrent root
    pub path: String,
    /// File size in bytes
    pub size: i64,
    /// Download progress (0.0 - 1.0)
    pub progress: f64,
}

/// A torrent download
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Torrent {
    /// Unique ID within the session
    pub id: i32,
    /// Info hash (hex string)
    pub info_hash: String,
    /// Torrent name
    pub name: String,
    /// Current state
    pub state: TorrentState,
    /// Download progress (0.0 - 1.0)
    pub progress: f64,
    /// Total size in bytes
    pub size: i64,
    /// Downloaded bytes
    pub downloaded: i64,
    /// Uploaded bytes
    pub uploaded: i64,
    /// Download speed in bytes/second
    pub download_speed: i64,
    /// Upload speed in bytes/second
    pub upload_speed: i64,
    /// Number of connected peers
    pub peers: i32,
    /// Number of seeds
    pub seeds: i32,
    /// Save path
    pub save_path: String,
    /// Files in the torrent
    pub files: Vec<TorrentFile>,
    /// When the torrent was added (ISO 8601 timestamp)
    #[serde(default)]
    pub added_at: Option<String>,
}

#[Object]
impl Torrent {
    async fn id(&self) -> i32 {
        self.id
    }

    async fn info_hash(&self) -> &str {
        &self.info_hash
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn state(&self) -> TorrentState {
        self.state
    }

    async fn progress(&self) -> f64 {
        self.progress
    }

    /// Progress as a percentage (0-100)
    async fn progress_percent(&self) -> f64 {
        self.progress * 100.0
    }

    async fn size(&self) -> i64 {
        self.size
    }

    /// Size formatted as human-readable string
    async fn size_formatted(&self) -> String {
        format_bytes(self.size as u64)
    }

    async fn downloaded(&self) -> i64 {
        self.downloaded
    }

    async fn uploaded(&self) -> i64 {
        self.uploaded
    }

    async fn download_speed(&self) -> i64 {
        self.download_speed
    }

    /// Download speed formatted as human-readable string
    async fn download_speed_formatted(&self) -> String {
        format!("{}/s", format_bytes(self.download_speed as u64))
    }

    async fn upload_speed(&self) -> i64 {
        self.upload_speed
    }

    /// Upload speed formatted as human-readable string
    async fn upload_speed_formatted(&self) -> String {
        format!("{}/s", format_bytes(self.upload_speed as u64))
    }

    async fn peers(&self) -> i32 {
        self.peers
    }

    async fn seeds(&self) -> i32 {
        self.seeds
    }

    async fn save_path(&self) -> &str {
        &self.save_path
    }

    async fn files(&self) -> &[TorrentFile] {
        &self.files
    }

    /// Ratio of uploaded to downloaded
    async fn ratio(&self) -> f64 {
        if self.downloaded > 0 {
            self.uploaded as f64 / self.downloaded as f64
        } else {
            0.0
        }
    }

    /// Estimated time to completion in seconds
    async fn eta(&self) -> Option<i64> {
        if self.download_speed > 0 && self.progress < 1.0 {
            let remaining = self.size - self.downloaded;
            Some(remaining / self.download_speed)
        } else {
            None
        }
    }

    /// When the torrent was added (ISO 8601 timestamp)
    async fn added_at(&self) -> Option<&str> {
        self.added_at.as_deref()
    }
}

impl From<ServiceTorrentInfo> for Torrent {
    fn from(info: ServiceTorrentInfo) -> Self {
        Self {
            id: info.id as i32,
            info_hash: info.info_hash,
            name: info.name,
            state: info.state.into(),
            progress: info.progress,
            size: info.size as i64,
            downloaded: info.downloaded as i64,
            uploaded: info.uploaded as i64,
            download_speed: info.download_speed as i64,
            upload_speed: info.upload_speed as i64,
            peers: info.peers as i32,
            seeds: info.seeds as i32,
            save_path: info.save_path,
            files: info
                .files
                .into_iter()
                .map(|f| TorrentFile {
                    index: f.index as i32,
                    path: f.path,
                    size: f.size as i64,
                    progress: f.progress,
                })
                .collect(),
            added_at: None, // Will be populated from database
        }
    }
}

/// Input for adding a torrent
#[derive(Debug, InputObject)]
pub struct AddTorrentInput {
    /// Magnet link
    pub magnet: Option<String>,
    /// URL to a .torrent file
    pub url: Option<String>,
    /// Optional display name for the torrent
    pub name: Option<String>,
    /// Optional library ID to associate the torrent with
    pub library_id: Option<String>,
    /// Optional episode ID to link the download to
    pub episode_id: Option<String>,
    /// Optional movie ID to link the download to
    pub movie_id: Option<String>,
    /// Optional indexer ID (for authenticated .torrent downloads)
    pub indexer_id: Option<String>,
}

/// Result of adding a torrent
#[derive(Debug, SimpleObject)]
pub struct AddTorrentResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// The added torrent (if successful)
    pub torrent: Option<Torrent>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Result of a torrent action (pause, resume, remove)
#[derive(Debug, SimpleObject)]
pub struct TorrentActionResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Result of organizing a torrent
#[derive(Debug, SimpleObject)]
pub struct OrganizeTorrentResult {
    /// Whether the operation succeeded overall
    pub success: bool,
    /// Number of files successfully organized
    pub organized_count: i32,
    /// Number of files that failed to organize
    pub failed_count: i32,
    /// Detailed messages about what happened
    pub messages: Vec<String>,
}

/// Detailed peer statistics for torrent info modal
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct PeerStats {
    /// Peers queued for connection
    pub queued: i32,
    /// Peers currently connecting
    pub connecting: i32,
    /// Active/live peers
    pub live: i32,
    /// Total peers seen
    pub seen: i32,
    /// Dead/disconnected peers
    pub dead: i32,
    /// Peers not needed (complete)
    pub not_needed: i32,
}

/// Detailed torrent information for the info modal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentDetails {
    /// Unique ID within the session
    pub id: i32,
    /// Info hash (hex string)
    pub info_hash: String,
    /// Torrent name
    pub name: String,
    /// Current state
    pub state: TorrentState,
    /// Download progress (0.0 - 1.0)
    pub progress: f64,
    /// Total size in bytes
    pub size: i64,
    /// Downloaded bytes
    pub downloaded: i64,
    /// Uploaded bytes
    pub uploaded: i64,
    /// Download speed in bytes/second
    pub download_speed: i64,
    /// Upload speed in bytes/second
    pub upload_speed: i64,
    /// Save path
    pub save_path: String,
    /// Files in the torrent
    pub files: Vec<TorrentFile>,
    /// Number of pieces in the torrent
    pub piece_count: i64,
    /// Number of pieces downloaded
    pub pieces_downloaded: i64,
    /// Average time to download a piece (ms)
    pub average_piece_download_ms: Option<i64>,
    /// Estimated time remaining (seconds)
    pub time_remaining_secs: Option<i64>,
    /// Detailed peer statistics
    pub peer_stats: PeerStats,
    /// Error message if in error state
    pub error: Option<String>,
    /// Whether download is complete
    pub finished: bool,
}

#[Object]
impl TorrentDetails {
    async fn id(&self) -> i32 {
        self.id
    }
    async fn info_hash(&self) -> &str {
        &self.info_hash
    }
    async fn name(&self) -> &str {
        &self.name
    }
    async fn state(&self) -> TorrentState {
        self.state
    }
    async fn progress(&self) -> f64 {
        self.progress
    }
    async fn progress_percent(&self) -> f64 {
        self.progress * 100.0
    }
    async fn size(&self) -> i64 {
        self.size
    }
    async fn size_formatted(&self) -> String {
        format_bytes(self.size as u64)
    }
    async fn downloaded(&self) -> i64 {
        self.downloaded
    }
    async fn downloaded_formatted(&self) -> String {
        format_bytes(self.downloaded as u64)
    }
    async fn uploaded(&self) -> i64 {
        self.uploaded
    }
    async fn uploaded_formatted(&self) -> String {
        format_bytes(self.uploaded as u64)
    }
    async fn download_speed(&self) -> i64 {
        self.download_speed
    }
    async fn download_speed_formatted(&self) -> String {
        format!("{}/s", format_bytes(self.download_speed as u64))
    }
    async fn upload_speed(&self) -> i64 {
        self.upload_speed
    }
    async fn upload_speed_formatted(&self) -> String {
        format!("{}/s", format_bytes(self.upload_speed as u64))
    }
    async fn save_path(&self) -> &str {
        &self.save_path
    }
    async fn files(&self) -> &[TorrentFile] {
        &self.files
    }
    async fn piece_count(&self) -> i64 {
        self.piece_count
    }
    async fn pieces_downloaded(&self) -> i64 {
        self.pieces_downloaded
    }
    async fn average_piece_download_ms(&self) -> Option<i64> {
        self.average_piece_download_ms
    }
    async fn time_remaining_secs(&self) -> Option<i64> {
        self.time_remaining_secs
    }
    async fn peer_stats(&self) -> &PeerStats {
        &self.peer_stats
    }
    async fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    async fn finished(&self) -> bool {
        self.finished
    }

    /// Ratio of uploaded to downloaded
    async fn ratio(&self) -> f64 {
        if self.downloaded > 0 {
            self.uploaded as f64 / self.downloaded as f64
        } else {
            0.0
        }
    }

    /// Time remaining formatted as human readable
    async fn time_remaining_formatted(&self) -> Option<String> {
        self.time_remaining_secs.map(|secs| {
            if secs >= 3600 {
                format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
            } else if secs >= 60 {
                format!("{}m {}s", secs / 60, secs % 60)
            } else {
                format!("{}s", secs)
            }
        })
    }
}

impl From<ServicePeerStats> for PeerStats {
    fn from(ps: ServicePeerStats) -> Self {
        Self {
            queued: ps.queued as i32,
            connecting: ps.connecting as i32,
            live: ps.live as i32,
            seen: ps.seen as i32,
            dead: ps.dead as i32,
            not_needed: ps.not_needed as i32,
        }
    }
}

impl From<ServiceTorrentDetails> for TorrentDetails {
    fn from(d: ServiceTorrentDetails) -> Self {
        Self {
            id: d.id as i32,
            info_hash: d.info_hash,
            name: d.name,
            state: d.state.into(),
            progress: d.progress,
            size: d.size as i64,
            downloaded: d.downloaded as i64,
            uploaded: d.uploaded as i64,
            download_speed: d.download_speed as i64,
            upload_speed: d.upload_speed as i64,
            save_path: d.save_path,
            files: d
                .files
                .into_iter()
                .map(|f| TorrentFile {
                    index: f.index as i32,
                    path: f.path,
                    size: f.size as i64,
                    progress: f.progress,
                })
                .collect(),
            piece_count: d.piece_count as i64,
            pieces_downloaded: d.pieces_downloaded as i64,
            average_piece_download_ms: d.average_piece_download_ms.map(|v| v as i64),
            time_remaining_secs: d.time_remaining_secs.map(|v| v as i64),
            peer_stats: d.peer_stats.into(),
            error: d.error,
            finished: d.finished,
        }
    }
}

// ============================================================================
// Library Types
// ============================================================================

/// Library type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum LibraryType {
    Movies,
    Tv,
    Music,
    Audiobooks,
    Other,
}

/// A media library
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct Library {
    /// Unique ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Filesystem path
    pub path: String,
    /// Library type
    pub library_type: LibraryType,
    /// Icon name
    pub icon: String,
    /// Color theme
    pub color: String,
    /// Whether auto-scan is enabled
    pub auto_scan: bool,
    /// Scan interval in hours
    pub scan_interval_hours: i32,
    /// Number of items in the library
    pub item_count: i32,
    /// Total size in bytes
    pub total_size_bytes: i64,
    /// Last scan timestamp (ISO 8601)
    pub last_scanned_at: Option<String>,
}

/// Result of a library mutation
#[derive(Debug, SimpleObject)]
pub struct LibraryResult {
    pub success: bool,
    pub library: Option<Library>,
    pub error: Option<String>,
}

/// Library scan status
#[derive(Debug, SimpleObject)]
pub struct ScanStatus {
    pub library_id: String,
    pub status: String,
    pub message: Option<String>,
}

/// Result of library consolidation
#[derive(Debug, SimpleObject)]
pub struct ConsolidateLibraryResult {
    pub success: bool,
    pub folders_removed: i32,
    pub files_moved: i32,
    pub messages: Vec<String>,
}

/// Type of library change event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum LibraryChangeType {
    /// Library was created
    Created,
    /// Library was updated
    Updated,
    /// Library was deleted
    Deleted,
}

/// Event when a library changes
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct LibraryChangedEvent {
    /// Type of change
    pub change_type: LibraryChangeType,
    /// Library ID
    pub library_id: String,
    /// Library name (None if deleted)
    pub library_name: Option<String>,
    /// The full library object (None if deleted)
    pub library: Option<Library>,
}

// ============================================================================
// Media Types
// ============================================================================

/// Media item type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum MediaType {
    Movie,
    Episode,
    Song,
    Audiobook,
    Other,
}

/// A media item (movie, episode, etc.)
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct MediaItem {
    /// Unique ID
    pub id: String,
    /// Library ID this belongs to
    pub library_id: String,
    /// Title
    pub title: String,
    /// Media type
    pub media_type: MediaType,
    /// Year released
    pub year: Option<i32>,
    /// File path
    pub path: String,
    /// Duration in seconds
    pub duration_seconds: Option<i64>,
    /// Duration formatted (HH:MM:SS)
    pub duration_formatted: Option<String>,
    /// Video resolution (e.g., "1920x1080")
    pub resolution: Option<String>,
    /// Video codec
    pub video_codec: Option<String>,
    /// Audio codec
    pub audio_codec: Option<String>,
    /// File size in bytes
    pub file_size_bytes: i64,
    /// Poster URL
    pub poster_url: Option<String>,
    /// Backdrop URL
    pub backdrop_url: Option<String>,
    /// Overview/description
    pub overview: Option<String>,
    /// TMDB ID
    pub tmdb_id: Option<i32>,
    /// TVDB ID
    pub tvdb_id: Option<i32>,
    /// IMDB ID
    pub imdb_id: Option<String>,
}

/// Stream information for playback
#[derive(Debug, Clone, SimpleObject)]
pub struct StreamInfo {
    /// HLS playlist URL
    pub playlist_url: String,
    /// Whether direct play is supported
    pub direct_play_supported: bool,
    /// Direct play URL (if supported)
    pub direct_url: Option<String>,
    /// Supported subtitles
    pub subtitles: Vec<SubtitleTrack>,
    /// Audio tracks
    pub audio_tracks: Vec<AudioTrack>,
}

/// Subtitle track info
#[derive(Debug, Clone, SimpleObject)]
pub struct SubtitleTrack {
    pub index: i32,
    pub language: String,
    pub label: String,
    pub url: String,
}

/// Audio track info
#[derive(Debug, Clone, SimpleObject)]
pub struct AudioTrack {
    pub index: i32,
    pub language: String,
    pub label: String,
    pub codec: String,
    pub channels: i32,
}

// ============================================================================
// Cast Types (Chromecast / AirPlay)
// ============================================================================

/// Cast device type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum CastDeviceType {
    Chromecast,
    ChromecastAudio,
    GoogleHome,
    GoogleNestHub,
    AndroidTv,
    Unknown,
}

impl From<&str> for CastDeviceType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "chromecast" => Self::Chromecast,
            "chromecast_audio" => Self::ChromecastAudio,
            "google_home" => Self::GoogleHome,
            "google_nest_hub" => Self::GoogleNestHub,
            "android_tv" => Self::AndroidTv,
            _ => Self::Unknown,
        }
    }
}

/// Cast player state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum CastPlayerState {
    Idle,
    Buffering,
    Playing,
    Paused,
}

impl From<&str> for CastPlayerState {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "buffering" => Self::Buffering,
            "playing" => Self::Playing,
            "paused" => Self::Paused,
            _ => Self::Idle,
        }
    }
}

/// A discovered/saved cast device
#[derive(Debug, Clone, SimpleObject)]
pub struct CastDevice {
    /// Unique ID
    pub id: String,
    /// Device name (friendly name)
    pub name: String,
    /// IP address
    pub address: String,
    /// Port number
    pub port: i32,
    /// Device model
    pub model: Option<String>,
    /// Device type
    pub device_type: CastDeviceType,
    /// Whether this is a favorite device
    pub is_favorite: bool,
    /// Whether this was manually added
    pub is_manual: bool,
    /// Whether the device is currently connected
    pub is_connected: bool,
    /// Last time the device was seen on the network
    pub last_seen_at: Option<String>,
}

impl CastDevice {
    pub fn from_record(record: crate::db::CastDeviceRecord, is_connected: bool) -> Self {
        Self {
            id: record.id.to_string(),
            name: record.name,
            address: record.address.to_string(),
            port: record.port,
            model: record.model,
            device_type: CastDeviceType::from(record.device_type.as_str()),
            is_favorite: record.is_favorite,
            is_manual: record.is_manual,
            is_connected,
            last_seen_at: record.last_seen_at.map(|t| {
                t.format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default()
            }),
        }
    }
}

/// An active cast session
#[derive(Debug, Clone, SimpleObject)]
pub struct CastSession {
    /// Session ID
    pub id: String,
    /// Device ID
    pub device_id: Option<String>,
    /// Device name (for display)
    pub device_name: Option<String>,
    /// Media file ID being cast
    pub media_file_id: Option<String>,
    /// Episode ID if applicable
    pub episode_id: Option<String>,
    /// Stream URL
    pub stream_url: String,
    /// Current player state
    pub player_state: CastPlayerState,
    /// Current playback position in seconds
    #[graphql(name = "currentTime")]
    pub current_position: f64,
    /// Total duration in seconds
    pub duration: Option<f64>,
    /// Volume level (0.0 - 1.0)
    pub volume: f32,
    /// Whether audio is muted
    pub is_muted: bool,
    /// When the session started
    pub started_at: String,
}

impl CastSession {
    pub fn from_record(record: crate::db::CastSessionRecord, device_name: Option<String>) -> Self {
        Self {
            id: record.id.to_string(),
            device_id: record.device_id.map(|id| id.to_string()),
            device_name,
            media_file_id: record.media_file_id.map(|id| id.to_string()),
            episode_id: record.episode_id.map(|id| id.to_string()),
            stream_url: record.stream_url,
            player_state: CastPlayerState::from(record.player_state.as_str()),
            current_position: record.current_position,
            duration: record.duration,
            volume: record.volume,
            is_muted: record.is_muted,
            started_at: record
                .started_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        }
    }
}

/// Cast settings (global configuration)
#[derive(Debug, Clone, SimpleObject)]
pub struct CastSettings {
    /// Whether auto-discovery is enabled
    pub auto_discovery_enabled: bool,
    /// Discovery interval in seconds
    pub discovery_interval_seconds: i32,
    /// Default volume level (0.0 - 1.0)
    pub default_volume: f32,
    /// Whether to auto-transcode incompatible files
    pub transcode_incompatible: bool,
    /// Preferred quality for transcoding
    pub preferred_quality: Option<String>,
}

impl CastSettings {
    pub fn from_record(record: crate::db::CastSettingsRecord) -> Self {
        Self {
            auto_discovery_enabled: record.auto_discovery_enabled,
            discovery_interval_seconds: record.discovery_interval_seconds,
            default_volume: record.default_volume,
            transcode_incompatible: record.transcode_incompatible,
            preferred_quality: record.preferred_quality,
        }
    }
}

/// Input for adding a cast device manually
#[derive(Debug, InputObject)]
pub struct AddCastDeviceInput {
    /// IP address of the device
    pub address: String,
    /// Port number (default: 8009)
    pub port: Option<i32>,
    /// Friendly name for the device
    pub name: Option<String>,
}

/// Input for updating a cast device
#[derive(Debug, InputObject)]
pub struct UpdateCastDeviceInput {
    /// New name
    pub name: Option<String>,
    /// Mark as favorite
    pub is_favorite: Option<bool>,
}

/// Input for casting media to a device
#[derive(Debug, InputObject)]
pub struct CastMediaInput {
    /// Device ID to cast to
    pub device_id: String,
    /// Media file ID to cast
    pub media_file_id: String,
    /// Episode ID (optional, for tracking)
    pub episode_id: Option<String>,
    /// Start position in seconds
    pub start_position: Option<f64>,
}

/// Input for updating cast settings
#[derive(Debug, InputObject)]
pub struct UpdateCastSettingsInput {
    /// Enable/disable auto-discovery
    pub auto_discovery_enabled: Option<bool>,
    /// Discovery interval in seconds
    pub discovery_interval_seconds: Option<i32>,
    /// Default volume level
    pub default_volume: Option<f32>,
    /// Auto-transcode incompatible files
    pub transcode_incompatible: Option<bool>,
    /// Preferred quality for transcoding
    pub preferred_quality: Option<String>,
}

/// Result of a cast device mutation
#[derive(Debug, SimpleObject)]
pub struct CastDeviceResult {
    pub success: bool,
    pub device: Option<CastDevice>,
    pub error: Option<String>,
}

/// Result of a cast session mutation
#[derive(Debug, SimpleObject)]
pub struct CastSessionResult {
    pub success: bool,
    pub session: Option<CastSession>,
    pub error: Option<String>,
}

/// Result of cast settings mutation
#[derive(Debug, SimpleObject)]
pub struct CastSettingsResult {
    pub success: bool,
    pub settings: Option<CastSettings>,
    pub error: Option<String>,
}

// ============================================================================
// Subscription Types
// ============================================================================

/// A subscription to a TV show for auto-downloading
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct Subscription {
    /// Unique ID
    pub id: String,
    /// Show name
    pub name: String,
    /// TVDB ID
    pub tvdb_id: Option<i32>,
    /// TMDB ID
    pub tmdb_id: Option<i32>,
    /// Quality profile ID
    pub quality_profile_id: String,
    /// Whether actively monitoring
    pub monitored: bool,
    /// Last checked timestamp
    pub last_checked_at: Option<String>,
    /// Number of episodes downloaded
    pub episode_count: i32,
}

/// Input for creating a subscription
#[derive(Debug, InputObject)]
pub struct CreateSubscriptionInput {
    /// Show name
    pub name: String,
    /// TVDB ID
    pub tvdb_id: Option<i32>,
    /// TMDB ID
    pub tmdb_id: Option<i32>,
    /// Quality profile ID
    pub quality_profile_id: String,
    /// Enable monitoring (default: true)
    pub monitored: Option<bool>,
}

/// Input for updating a subscription
#[derive(Debug, InputObject)]
pub struct UpdateSubscriptionInput {
    /// New quality profile ID
    pub quality_profile_id: Option<String>,
    /// Enable/disable monitoring
    pub monitored: Option<bool>,
}

/// Result of a subscription mutation
#[derive(Debug, SimpleObject)]
pub struct SubscriptionResult {
    pub success: bool,
    pub subscription: Option<Subscription>,
    pub error: Option<String>,
}

/// Search result from Torznab indexer
#[derive(Debug, Clone, SimpleObject)]
pub struct SearchResult {
    pub title: String,
    pub indexer: String,
    pub size_bytes: i64,
    pub size_formatted: String,
    pub seeders: i32,
    pub leechers: i32,
    pub magnet_url: Option<String>,
    pub download_url: Option<String>,
    pub info_url: Option<String>,
    pub published_at: Option<String>,
}

// ============================================================================
// User Types
// ============================================================================

/// Current authenticated user
#[derive(Debug, Clone, SimpleObject)]
pub struct User {
    /// User ID
    pub id: String,
    /// Email address
    pub email: Option<String>,
    /// User role
    pub role: Option<String>,
}

/// User preferences/settings
#[derive(Debug, Clone, SimpleObject)]
pub struct UserPreferences {
    /// Preferred theme (light/dark/system)
    pub theme: String,
    /// Default quality profile for downloads
    pub default_quality_profile: Option<String>,
    /// Enable notifications
    pub notifications_enabled: bool,
}

/// Input for updating user preferences
#[derive(Debug, InputObject)]
pub struct UpdatePreferencesInput {
    pub theme: Option<String>,
    pub default_quality_profile: Option<String>,
    pub notifications_enabled: Option<bool>,
}

// ============================================================================
// Generic Result Types
// ============================================================================

/// Generic mutation result
#[derive(Debug, SimpleObject)]
pub struct MutationResult {
    pub success: bool,
    pub error: Option<String>,
}

/// Real-time torrent progress update
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct TorrentProgress {
    /// Torrent ID
    pub id: i32,
    /// Info hash
    pub info_hash: String,
    /// Download progress (0.0 - 1.0)
    pub progress: f64,
    /// Download speed in bytes/second
    pub download_speed: i64,
    /// Upload speed in bytes/second
    pub upload_speed: i64,
    /// Number of connected peers
    pub peers: i32,
    /// Current state
    pub state: TorrentState,
}

/// Event when a torrent is added
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct TorrentAddedEvent {
    /// Torrent ID
    pub id: i32,
    /// Torrent name
    pub name: String,
    /// Info hash
    pub info_hash: String,
}

/// Event when a torrent completes
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct TorrentCompletedEvent {
    /// Torrent ID
    pub id: i32,
    /// Torrent name
    pub name: String,
    /// Info hash
    pub info_hash: String,
}

/// Event when a torrent is removed
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct TorrentRemovedEvent {
    /// Torrent ID
    pub id: i32,
    /// Info hash
    pub info_hash: String,
}

/// Library scan progress event (for future subscription use)
#[allow(dead_code)]
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct LibraryScanProgress {
    /// Library ID
    pub library_id: String,
    /// Total files to scan
    pub total_files: i32,
    /// Files scanned so far
    pub scanned_files: i32,
    /// Current file being scanned
    pub current_file: Option<String>,
    /// Whether scan is complete
    pub complete: bool,
}

// ============================================================================
// Settings Types
// ============================================================================

/// An application setting
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct AppSetting {
    /// Setting key
    pub key: String,
    /// Setting value as JSON
    pub value: serde_json::Value,
    /// Human-readable description
    pub description: Option<String>,
    /// Setting category
    pub category: String,
}

/// Torrent client settings
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct TorrentSettings {
    /// Download directory path
    pub download_dir: String,
    /// Session directory path
    pub session_dir: String,
    /// Enable DHT for peer discovery
    pub enable_dht: bool,
    /// Listen port (0 = random)
    pub listen_port: i32,
    /// Maximum concurrent downloads
    pub max_concurrent: i32,
    /// Upload speed limit in bytes/sec (0 = unlimited)
    pub upload_limit: i64,
    /// Download speed limit in bytes/sec (0 = unlimited)
    pub download_limit: i64,
}

/// Input for updating torrent settings
#[derive(Debug, InputObject)]
pub struct UpdateTorrentSettingsInput {
    /// Download directory path
    pub download_dir: Option<String>,
    /// Session directory path
    pub session_dir: Option<String>,
    /// Enable DHT for peer discovery
    pub enable_dht: Option<bool>,
    /// Listen port (0 = random)
    pub listen_port: Option<i32>,
    /// Maximum concurrent downloads
    pub max_concurrent: Option<i32>,
    /// Upload speed limit in bytes/sec (0 = unlimited)
    pub upload_limit: Option<i64>,
    /// Download speed limit in bytes/sec (0 = unlimited)
    pub download_limit: Option<i64>,
}

/// Result of updating settings
#[derive(Debug, SimpleObject)]
pub struct SettingsResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Result of refreshing the schedule cache
#[derive(Debug, SimpleObject)]
pub struct RefreshScheduleResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Number of schedule entries updated
    pub entries_updated: i32,
    /// Error message if failed
    pub error: Option<String>,
}

// ============================================================================
// TV Show Types
// ============================================================================

/// TV Show status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum TvShowStatus {
    Continuing,
    Ended,
    Upcoming,
    Cancelled,
    Unknown,
}

/// Monitor type for shows
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum MonitorType {
    /// Monitor all episodes (past and future)
    All,
    /// Only monitor future episodes
    Future,
    /// Don't monitor (track but don't download)
    None,
}

/// A TV show in a library
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct TvShow {
    pub id: String,
    pub library_id: String,
    pub name: String,
    pub sort_name: Option<String>,
    pub year: Option<i32>,
    pub status: TvShowStatus,
    pub tvmaze_id: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub tvdb_id: Option<i32>,
    pub imdb_id: Option<String>,
    pub overview: Option<String>,
    pub network: Option<String>,
    pub runtime: Option<i32>,
    pub genres: Vec<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub monitored: bool,
    pub monitor_type: MonitorType,
    pub quality_profile_id: Option<String>,
    pub path: Option<String>,
    /// Override library auto-download setting (null = inherit)
    pub auto_download_override: Option<bool>,
    /// Whether to backfill existing episodes when added
    pub backfill_existing: bool,
    /// Override library organize_files setting (null = inherit)
    pub organize_files_override: Option<bool>,
    /// Override library rename_style setting (null = inherit)
    pub rename_style_override: Option<String>,
    /// Override library auto_hunt setting (null = inherit)
    pub auto_hunt_override: Option<bool>,
    pub episode_count: i32,
    pub episode_file_count: i32,
    pub size_bytes: i64,
    // Quality override settings (null = inherit from library)
    /// Override allowed resolutions (null = inherit)
    pub allowed_resolutions_override: Option<Vec<String>>,
    /// Override allowed video codecs (null = inherit)
    pub allowed_video_codecs_override: Option<Vec<String>>,
    /// Override allowed audio formats (null = inherit)
    pub allowed_audio_formats_override: Option<Vec<String>>,
    /// Override HDR requirement (null = inherit)
    pub require_hdr_override: Option<bool>,
    /// Override allowed HDR types (null = inherit)
    pub allowed_hdr_types_override: Option<Vec<String>>,
    /// Override allowed sources (null = inherit)
    pub allowed_sources_override: Option<Vec<String>>,
    /// Override release group blacklist (null = inherit)
    pub release_group_blacklist_override: Option<Vec<String>>,
    /// Override release group whitelist (null = inherit)
    pub release_group_whitelist_override: Option<Vec<String>>,
}

impl TvShow {
    /// Convert a TvShowRecord from the database to a TvShow GraphQL type
    pub fn from_record(r: crate::db::TvShowRecord) -> Self {
        Self {
            id: r.id.to_string(),
            library_id: r.library_id.to_string(),
            name: r.name,
            sort_name: r.sort_name,
            year: r.year,
            status: match r.status.as_str() {
                "Running" | "Continuing" => TvShowStatus::Continuing,
                "Ended" => TvShowStatus::Ended,
                "Upcoming" | "To Be Determined" | "In Development" => TvShowStatus::Upcoming,
                "Cancelled" => TvShowStatus::Cancelled,
                _ => TvShowStatus::Unknown,
            },
            tvmaze_id: r.tvmaze_id,
            tmdb_id: r.tmdb_id,
            tvdb_id: r.tvdb_id,
            imdb_id: r.imdb_id,
            overview: r.overview,
            network: r.network,
            runtime: r.runtime,
            genres: r.genres,
            poster_url: r.poster_url,
            backdrop_url: r.backdrop_url,
            monitored: r.monitored,
            monitor_type: match r.monitor_type.as_str() {
                "future" => MonitorType::Future,
                "none" => MonitorType::None,
                _ => MonitorType::All,
            },
            quality_profile_id: r.quality_profile_id.map(|id| id.to_string()),
            path: r.path,
            auto_download_override: r.auto_download_override,
            backfill_existing: r.backfill_existing,
            organize_files_override: r.organize_files_override,
            rename_style_override: r.rename_style_override,
            auto_hunt_override: r.auto_hunt_override,
            episode_count: r.episode_count.unwrap_or(0),
            episode_file_count: r.episode_file_count.unwrap_or(0),
            size_bytes: r.size_bytes.unwrap_or(0),
            allowed_resolutions_override: r.allowed_resolutions_override,
            allowed_video_codecs_override: r.allowed_video_codecs_override,
            allowed_audio_formats_override: r.allowed_audio_formats_override,
            require_hdr_override: r.require_hdr_override,
            allowed_hdr_types_override: r.allowed_hdr_types_override,
            allowed_sources_override: r.allowed_sources_override,
            release_group_blacklist_override: r.release_group_blacklist_override,
            release_group_whitelist_override: r.release_group_whitelist_override,
        }
    }
}

impl From<crate::db::TvShowRecord> for TvShow {
    fn from(record: crate::db::TvShowRecord) -> Self {
        TvShow::from_record(record)
    }
}

/// TV show search result from metadata providers
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct TvShowSearchResult {
    pub provider: String,
    pub provider_id: i32,
    pub name: String,
    pub year: Option<i32>,
    pub status: Option<String>,
    pub network: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub tvdb_id: Option<i32>,
    pub imdb_id: Option<String>,
    pub score: f64,
}

/// Input for adding a TV show to a library
#[derive(Debug, InputObject)]
pub struct AddTvShowInput {
    /// Metadata provider to use
    pub provider: String,
    /// Provider-specific ID (e.g., TVMaze ID)
    pub provider_id: i32,
    /// Monitor type
    pub monitor_type: Option<MonitorType>,
    /// Quality profile to use (null = library default)
    pub quality_profile_id: Option<String>,
    /// Custom path within the library
    pub path: Option<String>,
}

/// Input for updating a TV show
#[derive(Debug, InputObject)]
pub struct UpdateTvShowInput {
    pub monitored: Option<bool>,
    pub monitor_type: Option<MonitorType>,
    pub quality_profile_id: Option<String>,
    pub path: Option<String>,
    /// Override library auto-download setting (null = inherit, Some(true/false) = override)
    pub auto_download_override: Option<Option<bool>>,
    /// Whether to backfill existing episodes
    pub backfill_existing: Option<bool>,
    /// Override library organize_files setting (null = inherit)
    pub organize_files_override: Option<Option<bool>>,
    /// Override library rename_style setting (null = inherit, Some("none"/"clean"/"preserve_info"))
    pub rename_style_override: Option<Option<String>>,
    /// Override library auto_hunt setting (null = inherit, Some(true/false) = override)
    pub auto_hunt_override: Option<Option<bool>>,
    // Quality override settings (null = inherit, Some([]) = override with any)
    /// Override allowed resolutions (null = inherit)
    pub allowed_resolutions_override: Option<Option<Vec<String>>>,
    /// Override allowed video codecs (null = inherit)
    pub allowed_video_codecs_override: Option<Option<Vec<String>>>,
    /// Override allowed audio formats (null = inherit)
    pub allowed_audio_formats_override: Option<Option<Vec<String>>>,
    /// Override HDR requirement (null = inherit)
    pub require_hdr_override: Option<Option<bool>>,
    /// Override allowed HDR types (null = inherit)
    pub allowed_hdr_types_override: Option<Option<Vec<String>>>,
    /// Override allowed sources (null = inherit)
    pub allowed_sources_override: Option<Option<Vec<String>>>,
    /// Override release group blacklist (null = inherit)
    pub release_group_blacklist_override: Option<Option<Vec<String>>>,
    /// Override release group whitelist (null = inherit)
    pub release_group_whitelist_override: Option<Option<Vec<String>>>,
}

/// Result of TV show mutation
#[derive(Debug, SimpleObject)]
pub struct TvShowResult {
    pub success: bool,
    pub tv_show: Option<TvShow>,
    pub error: Option<String>,
}

// ============================================================================
// Movie Types
// ============================================================================

/// Movie status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize, Default)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum MovieStatus {
    /// Movie has been released
    Released,
    /// Movie is upcoming
    Upcoming,
    /// Movie has been announced
    Announced,
    /// Movie is in production
    InProduction,
    /// Status unknown
    #[default]
    Unknown,
}

impl From<&str> for MovieStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "released" => MovieStatus::Released,
            "upcoming" => MovieStatus::Upcoming,
            "announced" => MovieStatus::Announced,
            "in_production" => MovieStatus::InProduction,
            _ => MovieStatus::Unknown,
        }
    }
}

/// A movie in a library
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct Movie {
    pub id: String,
    pub library_id: String,
    pub title: String,
    pub sort_title: Option<String>,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub imdb_id: Option<String>,
    pub status: MovieStatus,
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub runtime: Option<i32>,
    pub genres: Vec<String>,
    pub director: Option<String>,
    pub cast_names: Vec<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub monitored: bool,
    /// Whether a file exists for this movie
    pub has_file: bool,
    pub size_bytes: i64,
    pub path: Option<String>,
    /// Collection info
    pub collection_id: Option<i32>,
    pub collection_name: Option<String>,
    pub collection_poster_url: Option<String>,
    /// Ratings
    pub tmdb_rating: Option<f64>,
    pub tmdb_vote_count: Option<i32>,
    pub certification: Option<String>,
    pub release_date: Option<String>,
    // Quality override settings (null = inherit from library)
    pub allowed_resolutions_override: Option<Vec<String>>,
    pub allowed_video_codecs_override: Option<Vec<String>>,
    pub allowed_audio_formats_override: Option<Vec<String>>,
    pub require_hdr_override: Option<bool>,
    pub allowed_hdr_types_override: Option<Vec<String>>,
    pub allowed_sources_override: Option<Vec<String>>,
    pub release_group_blacklist_override: Option<Vec<String>>,
    pub release_group_whitelist_override: Option<Vec<String>>,
}

/// Movie search result from TMDB
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct MovieSearchResult {
    pub provider: String,
    pub provider_id: i32,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub imdb_id: Option<String>,
    pub vote_average: Option<f64>,
    pub popularity: Option<f64>,
}

/// Input for adding a movie to a library
#[derive(Debug, InputObject)]
pub struct AddMovieInput {
    /// TMDB movie ID
    pub tmdb_id: i32,
    /// Whether to monitor for releases
    pub monitored: Option<bool>,
    /// Custom path within the library
    pub path: Option<String>,
}

/// Input for updating a movie
#[derive(Debug, InputObject)]
pub struct UpdateMovieInput {
    pub monitored: Option<bool>,
    pub path: Option<String>,
    // Quality override settings (null = inherit, Some([]) = override with any)
    pub allowed_resolutions_override: Option<Option<Vec<String>>>,
    pub allowed_video_codecs_override: Option<Option<Vec<String>>>,
    pub allowed_audio_formats_override: Option<Option<Vec<String>>>,
    pub require_hdr_override: Option<Option<bool>>,
    pub allowed_hdr_types_override: Option<Option<Vec<String>>>,
    pub allowed_sources_override: Option<Option<Vec<String>>>,
    pub release_group_blacklist_override: Option<Option<Vec<String>>>,
    pub release_group_whitelist_override: Option<Option<Vec<String>>>,
}

/// Result of movie mutation
#[derive(Debug, SimpleObject)]
pub struct MovieResult {
    pub success: bool,
    pub movie: Option<Movie>,
    pub error: Option<String>,
}

// ============================================================================
// Episode Types
// ============================================================================

/// Episode status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum EpisodeStatus {
    /// Episode hasn't aired yet
    Missing,
    /// Episode should be downloaded
    Wanted,
    /// Episode found in RSS feed, ready to download
    Available,
    /// Currently downloading
    Downloading,
    /// Episode is downloaded
    Downloaded,
    /// Manually ignored
    Ignored,
}

/// An episode of a TV show
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub tv_show_id: String,
    pub season: i32,
    pub episode: i32,
    pub absolute_number: Option<i32>,
    pub title: Option<String>,
    pub overview: Option<String>,
    pub air_date: Option<String>,
    pub runtime: Option<i32>,
    pub status: EpisodeStatus,
    pub tvmaze_id: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub tvdb_id: Option<i32>,
    /// URL/magnet link to download this episode (when status is 'available')
    pub torrent_link: Option<String>,
    /// When the torrent link was found in RSS
    pub torrent_link_added_at: Option<String>,
    /// Media file ID if episode has been downloaded (for playback)
    pub media_file_id: Option<String>,
}

/// Episode with show information (for future wanted list feature)
#[allow(dead_code)]
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct WantedEpisode {
    pub episode: Episode,
    pub show_name: String,
    pub show_year: Option<i32>,
}

/// Result of downloading an episode
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct DownloadEpisodeResult {
    pub success: bool,
    pub episode: Option<Episode>,
    pub error: Option<String>,
}

// ============================================================================
// Quality Profile Types
// ============================================================================

/// A quality profile for downloads
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct QualityProfile {
    pub id: String,
    pub name: String,
    pub preferred_resolution: Option<String>,
    pub min_resolution: Option<String>,
    pub preferred_codec: Option<String>,
    pub preferred_audio: Option<String>,
    pub require_hdr: bool,
    pub hdr_types: Vec<String>,
    pub preferred_language: Option<String>,
    pub max_size_gb: Option<f64>,
    pub min_seeders: Option<i32>,
    pub release_group_whitelist: Vec<String>,
    pub release_group_blacklist: Vec<String>,
    pub upgrade_until: Option<String>,
}

/// Input for creating a quality profile
#[derive(Debug, InputObject)]
pub struct CreateQualityProfileInput {
    pub name: String,
    pub preferred_resolution: Option<String>,
    pub min_resolution: Option<String>,
    pub preferred_codec: Option<String>,
    pub preferred_audio: Option<String>,
    pub require_hdr: Option<bool>,
    pub hdr_types: Option<Vec<String>>,
    pub preferred_language: Option<String>,
    pub max_size_gb: Option<f64>,
    pub min_seeders: Option<i32>,
    pub release_group_whitelist: Option<Vec<String>>,
    pub release_group_blacklist: Option<Vec<String>>,
    pub upgrade_until: Option<String>,
}

/// Input for updating a quality profile
#[derive(Debug, InputObject)]
pub struct UpdateQualityProfileInput {
    pub name: Option<String>,
    pub preferred_resolution: Option<String>,
    pub min_resolution: Option<String>,
    pub preferred_codec: Option<String>,
    pub preferred_audio: Option<String>,
    pub require_hdr: Option<bool>,
    pub hdr_types: Option<Vec<String>>,
    pub preferred_language: Option<String>,
    pub max_size_gb: Option<f64>,
    pub min_seeders: Option<i32>,
    pub release_group_whitelist: Option<Vec<String>>,
    pub release_group_blacklist: Option<Vec<String>>,
    pub upgrade_until: Option<String>,
}

/// Result of quality profile mutation
#[derive(Debug, SimpleObject)]
pub struct QualityProfileResult {
    pub success: bool,
    pub quality_profile: Option<QualityProfile>,
    pub error: Option<String>,
}

// ============================================================================
// Naming Pattern Types
// ============================================================================

/// A file naming pattern preset
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct NamingPattern {
    /// Unique identifier
    pub id: String,
    /// Display name for the pattern
    pub name: String,
    /// The actual pattern string (e.g., "{show}/Season {season:02}/...")
    pub pattern: String,
    /// Human-readable description/example
    pub description: Option<String>,
    /// Whether this is the default pattern for new libraries of this type
    pub is_default: bool,
    /// Whether this is a built-in system pattern (cannot be deleted)
    pub is_system: bool,
    /// Library type this pattern is for (tv, movies, music, audiobooks, other)
    pub library_type: Option<String>,
}

impl NamingPattern {
    pub fn from_record(record: crate::db::NamingPatternRecord) -> Self {
        Self {
            id: record.id.to_string(),
            name: record.name,
            pattern: record.pattern,
            description: record.description,
            is_default: record.is_default,
            is_system: record.is_system,
            library_type: record.library_type,
        }
    }
}

/// Input for creating a custom naming pattern
#[derive(Debug, InputObject)]
pub struct CreateNamingPatternInput {
    /// Display name for the pattern
    pub name: String,
    /// The pattern string
    pub pattern: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Library type this pattern is for (tv, movies, music, audiobooks, other)
    pub library_type: Option<String>,
}

/// Result of naming pattern mutation
#[derive(Debug, SimpleObject)]
pub struct NamingPatternResult {
    pub success: bool,
    pub naming_pattern: Option<NamingPattern>,
    pub error: Option<String>,
}

// ============================================================================
// RSS Feed Types
// ============================================================================

/// An RSS feed for torrent releases
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct RssFeed {
    pub id: String,
    pub library_id: Option<String>,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub poll_interval_minutes: i32,
    pub last_polled_at: Option<String>,
    pub last_successful_at: Option<String>,
    pub last_error: Option<String>,
    pub consecutive_failures: i32,
}

/// Input for creating an RSS feed
#[derive(Debug, InputObject)]
pub struct CreateRssFeedInput {
    pub library_id: Option<String>,
    pub name: String,
    pub url: String,
    pub enabled: Option<bool>,
    pub poll_interval_minutes: Option<i32>,
}

/// Input for updating an RSS feed
#[derive(Debug, InputObject)]
pub struct UpdateRssFeedInput {
    pub library_id: Option<String>,
    pub name: Option<String>,
    pub url: Option<String>,
    pub enabled: Option<bool>,
    pub poll_interval_minutes: Option<i32>,
}

/// Result of RSS feed mutation
#[derive(Debug, SimpleObject)]
pub struct RssFeedResult {
    pub success: bool,
    pub rss_feed: Option<RssFeed>,
    pub error: Option<String>,
}

/// Result of testing an RSS feed
#[derive(Debug, SimpleObject)]
pub struct RssFeedTestResult {
    pub success: bool,
    pub item_count: i32,
    pub sample_items: Vec<RssItem>,
    pub error: Option<String>,
}

/// An RSS feed item
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct RssItem {
    pub title: String,
    pub link: String,
    pub pub_date: Option<String>,
    pub description: Option<String>,
    pub parsed_show_name: Option<String>,
    pub parsed_season: Option<i32>,
    pub parsed_episode: Option<i32>,
    pub parsed_resolution: Option<String>,
    pub parsed_codec: Option<String>,
}

// ============================================================================
// Parse and Identify Types
// ============================================================================

/// Parsed episode information from a filename
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct ParsedEpisodeInfo {
    pub original_title: String,
    pub show_name: Option<String>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub year: Option<i32>,
    pub date: Option<String>,
    pub resolution: Option<String>,
    pub source: Option<String>,
    pub codec: Option<String>,
    pub hdr: Option<String>,
    pub audio: Option<String>,
    pub release_group: Option<String>,
    pub is_proper: bool,
    pub is_repack: bool,
}

/// Result of parsing and identifying media
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct ParseAndIdentifyMediaResult {
    pub parsed: ParsedEpisodeInfo,
    pub matches: Vec<TvShowSearchResult>,
}

// ============================================================================
// Enhanced Library Types
// ============================================================================

/// Post-download action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum PostDownloadAction {
    /// Copy files (preserves seeding)
    Copy,
    /// Move files (stops seeding)
    Move,
    /// Hardlink files (same disk only)
    Hardlink,
}

/// Enhanced library with additional fields
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct LibraryFull {
    pub id: String,
    pub name: String,
    pub path: String,
    pub library_type: LibraryType,
    pub icon: String,
    pub color: String,
    pub auto_scan: bool,
    pub scan_interval_minutes: i32,
    pub watch_for_changes: bool,
    pub post_download_action: PostDownloadAction,
    pub organize_files: bool,
    pub rename_style: String,
    pub naming_pattern: Option<String>,
    pub default_quality_profile_id: Option<String>,
    pub auto_add_discovered: bool,
    pub auto_download: bool,
    /// Automatically hunt for missing episodes using indexers
    pub auto_hunt: bool,
    /// Whether a scan is currently in progress
    pub scanning: bool,
    pub item_count: i32,
    pub total_size_bytes: i64,
    pub show_count: i32,
    pub last_scanned_at: Option<String>,
    // Inline quality settings (empty = any)
    /// Allowed resolutions: 2160p, 1080p, 720p, 480p. Empty = any.
    pub allowed_resolutions: Vec<String>,
    /// Allowed video codecs: hevc, h264, av1, xvid. Empty = any.
    pub allowed_video_codecs: Vec<String>,
    /// Allowed audio formats: atmos, truehd, dtshd, dts, dd51, aac. Empty = any.
    pub allowed_audio_formats: Vec<String>,
    /// If true, only accept releases with HDR.
    pub require_hdr: bool,
    /// Allowed HDR types: hdr10, hdr10plus, dolbyvision, hlg. Empty with require_hdr=true = any HDR.
    pub allowed_hdr_types: Vec<String>,
    /// Allowed sources: webdl, webrip, bluray, hdtv. Empty = any.
    pub allowed_sources: Vec<String>,
    /// Blacklisted release groups.
    pub release_group_blacklist: Vec<String>,
    /// Whitelisted release groups (if set, only allow these).
    pub release_group_whitelist: Vec<String>,
}

impl LibraryFull {
    /// Convert a LibraryRecord and LibraryStats from the database to a LibraryFull GraphQL type
    pub fn from_record_with_stats(
        r: crate::db::libraries::LibraryRecord,
        stats: crate::db::LibraryStats,
    ) -> Self {
        Self {
            id: r.id.to_string(),
            name: r.name,
            path: r.path,
            library_type: match r.library_type.as_str() {
                "movies" => LibraryType::Movies,
                "tv" => LibraryType::Tv,
                "music" => LibraryType::Music,
                "audiobooks" => LibraryType::Audiobooks,
                _ => LibraryType::Other,
            },
            icon: r.icon.unwrap_or_else(|| "folder".to_string()),
            color: r.color.unwrap_or_else(|| "slate".to_string()),
            auto_scan: r.auto_scan,
            scan_interval_minutes: r.scan_interval_minutes,
            watch_for_changes: r.watch_for_changes,
            post_download_action: match r.post_download_action.as_str() {
                "copy" => PostDownloadAction::Copy,
                "hardlink" => PostDownloadAction::Hardlink,
                _ => PostDownloadAction::Move,
            },
            organize_files: r.organize_files,
            rename_style: r.rename_style,
            naming_pattern: r.naming_pattern,
            default_quality_profile_id: r.default_quality_profile_id.map(|id| id.to_string()),
            auto_add_discovered: r.auto_add_discovered,
            auto_download: r.auto_download,
            auto_hunt: r.auto_hunt,
            scanning: r.scanning,
            item_count: stats.file_count.unwrap_or(0) as i32,
            total_size_bytes: stats.total_size_bytes.unwrap_or(0),
            show_count: stats.show_count.unwrap_or(0) as i32,
            last_scanned_at: r.last_scanned_at.map(|dt| dt.to_rfc3339()),
            allowed_resolutions: r.allowed_resolutions,
            allowed_video_codecs: r.allowed_video_codecs,
            allowed_audio_formats: r.allowed_audio_formats,
            require_hdr: r.require_hdr,
            allowed_hdr_types: r.allowed_hdr_types,
            allowed_sources: r.allowed_sources,
            release_group_blacklist: r.release_group_blacklist,
            release_group_whitelist: r.release_group_whitelist,
        }
    }
}

/// Input for creating a library
#[derive(Debug, InputObject)]
pub struct CreateLibraryInput {
    pub name: String,
    pub path: String,
    pub library_type: LibraryType,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub auto_scan: Option<bool>,
    pub scan_interval_minutes: Option<i32>,
    pub watch_for_changes: Option<bool>,
    pub post_download_action: Option<PostDownloadAction>,
    /// Organize files into show/season folders (default: true)
    pub organize_files: Option<bool>,
    /// How to rename files: none, clean, preserve_info (default: none)
    pub rename_style: Option<String>,
    pub naming_pattern: Option<String>,
    pub default_quality_profile_id: Option<String>,
    pub auto_add_discovered: Option<bool>,
    /// Enable auto-download of available episodes (default: true)
    pub auto_download: Option<bool>,
    /// Automatically hunt for missing episodes using indexers (default: false)
    pub auto_hunt: Option<bool>,
    // Inline quality settings
    /// Allowed resolutions (empty = any)
    pub allowed_resolutions: Option<Vec<String>>,
    /// Allowed video codecs (empty = any)
    pub allowed_video_codecs: Option<Vec<String>>,
    /// Allowed audio formats (empty = any)
    pub allowed_audio_formats: Option<Vec<String>>,
    /// Require HDR content
    pub require_hdr: Option<bool>,
    /// Allowed HDR types (empty with require_hdr = any HDR)
    pub allowed_hdr_types: Option<Vec<String>>,
    /// Allowed sources (empty = any)
    pub allowed_sources: Option<Vec<String>>,
    /// Blacklisted release groups
    pub release_group_blacklist: Option<Vec<String>>,
    /// Whitelisted release groups
    pub release_group_whitelist: Option<Vec<String>>,
}

/// Input for updating a library
#[derive(Debug, InputObject)]
pub struct UpdateLibraryInput {
    pub name: Option<String>,
    pub path: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub auto_scan: Option<bool>,
    pub scan_interval_minutes: Option<i32>,
    pub watch_for_changes: Option<bool>,
    pub post_download_action: Option<PostDownloadAction>,
    /// Organize files into show/season folders
    pub organize_files: Option<bool>,
    /// How to rename files: none, clean, preserve_info
    pub rename_style: Option<String>,
    pub naming_pattern: Option<String>,
    pub default_quality_profile_id: Option<String>,
    pub auto_add_discovered: Option<bool>,
    /// Enable auto-download of available episodes
    pub auto_download: Option<bool>,
    /// Automatically hunt for missing episodes using indexers
    pub auto_hunt: Option<bool>,
    // Inline quality settings
    /// Allowed resolutions (empty = any)
    pub allowed_resolutions: Option<Vec<String>>,
    /// Allowed video codecs (empty = any)
    pub allowed_video_codecs: Option<Vec<String>>,
    /// Allowed audio formats (empty = any)
    pub allowed_audio_formats: Option<Vec<String>>,
    /// Require HDR content
    pub require_hdr: Option<bool>,
    /// Allowed HDR types (empty with require_hdr = any HDR)
    pub allowed_hdr_types: Option<Vec<String>>,
    /// Allowed sources (empty = any)
    pub allowed_sources: Option<Vec<String>>,
    /// Blacklisted release groups
    pub release_group_blacklist: Option<Vec<String>>,
    /// Whitelisted release groups
    pub release_group_whitelist: Option<Vec<String>>,
}

// ============================================================================
// Log Types
// ============================================================================

/// Log level enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<&str> for LogLevel {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "TRACE" => LogLevel::Trace,
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" => LogLevel::Warn,
            "ERROR" => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }
}

/// An application log entry
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct LogEntry {
    /// Unique ID
    pub id: String,
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Log level
    pub level: LogLevel,
    /// Source module/target (e.g., librarian_backend::jobs::auto_download)
    pub target: String,
    /// Log message
    pub message: String,
    /// Structured fields as JSON
    pub fields: Option<serde_json::Value>,
    /// Span name if within a span
    pub span_name: Option<String>,
}

/// Paginated log result
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct PaginatedLogResult {
    /// Log entries
    pub logs: Vec<LogEntry>,
    /// Total count matching the filter
    pub total_count: i64,
    /// Whether there are more results
    pub has_more: bool,
    /// Cursor for next page (timestamp of last item)
    pub next_cursor: Option<String>,
}

/// Input for filtering logs
#[derive(Debug, InputObject)]
pub struct LogFilterInput {
    /// Filter by log levels
    pub levels: Option<Vec<LogLevel>>,
    /// Filter by target/source (prefix match)
    pub target: Option<String>,
    /// Keyword search in message
    pub keyword: Option<String>,
    /// From timestamp (ISO 8601)
    pub from_timestamp: Option<String>,
    /// To timestamp (ISO 8601)
    pub to_timestamp: Option<String>,
}

/// Log statistics by level
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct LogStats {
    /// Count of TRACE logs
    pub trace_count: i64,
    /// Count of DEBUG logs
    pub debug_count: i64,
    /// Count of INFO logs
    pub info_count: i64,
    /// Count of WARN logs
    pub warn_count: i64,
    /// Count of ERROR logs
    pub error_count: i64,
    /// Total log count
    pub total_count: i64,
}

/// Result of clearing logs
#[derive(Debug, SimpleObject)]
pub struct ClearLogsResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Number of logs deleted
    pub deleted_count: i64,
    /// Error message if failed
    pub error: Option<String>,
}

/// Real-time log event for subscriptions
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct LogEventSubscription {
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Log level
    pub level: LogLevel,
    /// Source module/target
    pub target: String,
    /// Log message
    pub message: String,
    /// Structured fields as JSON
    pub fields: Option<serde_json::Value>,
    /// Span name if within a span
    pub span_name: Option<String>,
}

// ============================================================================
// Playback Session Types
// ============================================================================

/// A user's playback session (what they're currently watching)
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct PlaybackSession {
    /// Session ID
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Episode being played
    pub episode_id: Option<String>,
    /// Media file being played
    pub media_file_id: Option<String>,
    /// TV show ID
    pub tv_show_id: Option<String>,
    /// Current playback position in seconds
    pub current_position: f64,
    /// Total duration in seconds
    pub duration: Option<f64>,
    /// Volume level (0.0 - 1.0)
    pub volume: f32,
    /// Whether audio is muted
    pub is_muted: bool,
    /// Whether currently playing (vs paused)
    pub is_playing: bool,
    /// When playback started
    pub started_at: String,
    /// When position was last updated
    pub last_updated_at: String,
}

impl PlaybackSession {
    pub fn from_record(record: crate::db::PlaybackSessionRecord) -> Self {
        Self {
            id: record.id.to_string(),
            user_id: record.user_id.to_string(),
            episode_id: record.episode_id.map(|id| id.to_string()),
            media_file_id: record.media_file_id.map(|id| id.to_string()),
            tv_show_id: record.tv_show_id.map(|id| id.to_string()),
            current_position: record.current_position,
            duration: record.duration,
            volume: record.volume,
            is_muted: record.is_muted,
            is_playing: record.is_playing,
            started_at: record.started_at.to_string(),
            last_updated_at: record.last_updated_at.to_string(),
        }
    }
}

/// Input for starting/updating playback
#[derive(Debug, InputObject)]
pub struct StartPlaybackInput {
    /// Episode ID to play
    pub episode_id: String,
    /// Media file ID
    pub media_file_id: String,
    /// TV show ID
    pub tv_show_id: String,
    /// Starting position in seconds
    pub start_position: Option<f64>,
    /// Duration in seconds
    pub duration: Option<f64>,
}

/// Input for updating playback position
#[derive(Debug, InputObject)]
pub struct UpdatePlaybackInput {
    /// Current position in seconds
    pub current_position: Option<f64>,
    /// Duration in seconds (if known)
    pub duration: Option<f64>,
    /// Volume level
    pub volume: Option<f32>,
    /// Whether muted
    pub is_muted: Option<bool>,
    /// Whether playing
    pub is_playing: Option<bool>,
}

/// Result of playback operations
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct PlaybackResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// The playback session
    pub session: Option<PlaybackSession>,
    /// Error message if failed
    pub error: Option<String>,
}

// ============================================================================
// Upcoming Episode Types (for home page)
// ============================================================================

/// An upcoming episode with embedded show information
/// Used for displaying upcoming TV schedule on the home page
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct UpcomingEpisode {
    /// Episode TVMaze ID
    pub tvmaze_id: i32,
    /// Episode name/title
    pub name: String,
    /// Season number
    pub season: i32,
    /// Episode number
    pub episode: i32,
    /// Air date (YYYY-MM-DD)
    pub air_date: String,
    /// Air time (HH:MM)
    pub air_time: Option<String>,
    /// Full air timestamp (ISO 8601)
    pub air_stamp: Option<String>,
    /// Episode runtime in minutes
    pub runtime: Option<i32>,
    /// Episode overview/summary
    pub summary: Option<String>,
    /// Episode image URL
    pub episode_image_url: Option<String>,
    /// Show information
    pub show: UpcomingEpisodeShow,
}

/// Show information embedded in upcoming episode
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct UpcomingEpisodeShow {
    /// TVMaze show ID
    pub tvmaze_id: i32,
    /// Show name
    pub name: String,
    /// Network name (e.g., "HBO", "Netflix")
    pub network: Option<String>,
    /// Show poster URL
    pub poster_url: Option<String>,
    /// Show genres
    pub genres: Vec<String>,
}

/// An upcoming episode from the user's library
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct LibraryUpcomingEpisode {
    /// Episode ID in our database
    pub id: String,
    /// Episode TVMaze ID (if available)
    pub tvmaze_id: Option<i32>,
    /// Episode name/title
    pub name: Option<String>,
    /// Season number
    pub season: i32,
    /// Episode number
    pub episode: i32,
    /// Air date (YYYY-MM-DD)
    pub air_date: String,
    /// Episode status
    pub status: EpisodeStatus,
    /// Show information from our database
    pub show: LibraryUpcomingShow,
}

/// Show information for library upcoming episodes
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct LibraryUpcomingShow {
    /// Show ID in our database
    pub id: String,
    /// Show name
    pub name: String,
    /// Show year
    pub year: Option<i32>,
    /// Network name
    pub network: Option<String>,
    /// Show poster URL
    pub poster_url: Option<String>,
    /// Library ID this show belongs to
    pub library_id: String,
}

// ============================================================================
// Helpers
// ============================================================================

/// Format bytes as human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

// ============================================================================
// Media File Types
// ============================================================================

/// A media file in a library
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct MediaFile {
    /// Unique identifier
    pub id: String,
    /// Library ID
    pub library_id: String,
    /// Full file path
    pub path: String,
    /// Relative path within library
    pub relative_path: Option<String>,
    /// Original filename
    pub original_name: Option<String>,
    /// File size in bytes
    pub size_bytes: i64,
    /// Human-readable file size
    pub size_formatted: String,
    /// Container format (mkv, mp4, etc.)
    pub container: Option<String>,
    /// Video codec
    pub video_codec: Option<String>,
    /// Audio codec
    pub audio_codec: Option<String>,
    /// Video resolution (1080p, 4K, etc.)
    pub resolution: Option<String>,
    /// Whether the file is HDR
    pub is_hdr: Option<bool>,
    /// HDR type (HDR10, Dolby Vision, etc.)
    pub hdr_type: Option<String>,
    /// Video width
    pub width: Option<i32>,
    /// Video height
    pub height: Option<i32>,
    /// Duration in seconds
    pub duration: Option<i32>,
    /// Bitrate in kbps
    pub bitrate: Option<i32>,
    /// Episode ID if matched
    pub episode_id: Option<String>,
    /// Whether the file has been organized
    pub organized: bool,
    /// Organization status: pending, organized, skipped, conflicted, error
    pub organize_status: Option<String>,
    /// Error or conflict details when organization fails
    pub organize_error: Option<String>,
    /// When the file was added
    pub added_at: String,
}

impl MediaFile {
    pub fn from_record(record: crate::db::MediaFileRecord) -> Self {
        Self {
            id: record.id.to_string(),
            library_id: record.library_id.to_string(),
            path: record.path,
            relative_path: record.relative_path,
            original_name: record.original_name,
            size_bytes: record.size_bytes,
            size_formatted: format_bytes(record.size_bytes as u64),
            container: record.container,
            video_codec: record.video_codec,
            audio_codec: record.audio_codec,
            resolution: record.resolution,
            is_hdr: record.is_hdr,
            hdr_type: record.hdr_type,
            width: record.width,
            height: record.height,
            duration: record.duration,
            bitrate: record.bitrate,
            episode_id: record.episode_id.map(|id| id.to_string()),
            organized: record.organized,
            organize_status: record.organize_status,
            organize_error: record.organize_error,
            added_at: record.added_at.to_rfc3339(),
        }
    }
}

// ============================================================================
// Media Analysis Types (FFmpeg)
// ============================================================================

/// Source type for subtitle tracks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum SubtitleSourceType {
    /// Embedded in the media container
    Embedded,
    /// External file alongside the media
    External,
    /// Downloaded from a subtitle service
    Downloaded,
}

impl From<&str> for SubtitleSourceType {
    fn from(s: &str) -> Self {
        match s {
            "embedded" => Self::Embedded,
            "external" => Self::External,
            "downloaded" => Self::Downloaded,
            _ => Self::Embedded,
        }
    }
}

/// Subtitle record from the database
#[derive(Debug, Clone, SimpleObject)]
pub struct Subtitle {
    /// Unique identifier
    pub id: String,
    /// Media file this subtitle belongs to
    pub media_file_id: String,
    /// Source type (embedded, external, downloaded)
    pub source_type: SubtitleSourceType,
    /// Stream index for embedded subtitles
    pub stream_index: Option<i32>,
    /// File path for external/downloaded subtitles
    pub file_path: Option<String>,
    /// Codec/format (srt, ass, pgs, etc.)
    pub codec: Option<String>,
    /// Language code (ISO 639-1)
    pub language: Option<String>,
    /// Stream title
    pub title: Option<String>,
    /// Whether this is the default track
    pub is_default: bool,
    /// Whether this is a forced subtitle track
    pub is_forced: bool,
    /// Whether this is for hearing impaired (SDH)
    pub is_hearing_impaired: bool,
    /// OpenSubtitles file ID if downloaded
    pub opensubtitles_id: Option<String>,
    /// When the subtitle was downloaded
    pub downloaded_at: Option<String>,
}

impl Subtitle {
    pub fn from_record(record: crate::db::SubtitleRecord) -> Self {
        Self {
            id: record.id.to_string(),
            media_file_id: record.media_file_id.to_string(),
            source_type: SubtitleSourceType::from(record.source_type.as_str()),
            stream_index: record.stream_index,
            file_path: record.file_path,
            codec: record.codec,
            language: record.language,
            title: record.title,
            is_default: record.is_default,
            is_forced: record.is_forced,
            is_hearing_impaired: record.is_hearing_impaired,
            opensubtitles_id: record.opensubtitles_id,
            downloaded_at: record.downloaded_at.map(|d| d.to_rfc3339()),
        }
    }
}

/// Video stream information from FFmpeg analysis
#[derive(Debug, Clone, SimpleObject)]
pub struct VideoStreamInfo {
    /// Unique identifier
    pub id: String,
    /// Stream index
    pub stream_index: i32,
    /// Codec name (h264, hevc, av1, etc.)
    pub codec: String,
    /// Codec long name
    pub codec_long_name: Option<String>,
    /// Width in pixels
    pub width: i32,
    /// Height in pixels
    pub height: i32,
    /// Display aspect ratio
    pub aspect_ratio: Option<String>,
    /// Frame rate
    pub frame_rate: Option<String>,
    /// Bitrate in bps
    pub bitrate: Option<i64>,
    /// Pixel format
    pub pixel_format: Option<String>,
    /// HDR type (HDR10, Dolby Vision, HLG)
    pub hdr_type: Option<String>,
    /// Bit depth (8, 10, 12)
    pub bit_depth: Option<i32>,
    /// Language code
    pub language: Option<String>,
    /// Stream title
    pub title: Option<String>,
    /// Whether this is the default track
    pub is_default: bool,
}

impl VideoStreamInfo {
    pub fn from_record(record: crate::db::VideoStreamRecord) -> Self {
        Self {
            id: record.id.to_string(),
            stream_index: record.stream_index,
            codec: record.codec,
            codec_long_name: record.codec_long_name,
            width: record.width,
            height: record.height,
            aspect_ratio: record.aspect_ratio,
            frame_rate: record.frame_rate,
            bitrate: record.bitrate,
            pixel_format: record.pixel_format,
            hdr_type: record.hdr_type,
            bit_depth: record.bit_depth,
            language: record.language,
            title: record.title,
            is_default: record.is_default,
        }
    }
}

/// Audio stream information from FFmpeg analysis
#[derive(Debug, Clone, SimpleObject)]
pub struct AudioStreamInfo {
    /// Unique identifier
    pub id: String,
    /// Stream index
    pub stream_index: i32,
    /// Codec name (aac, ac3, dts, flac, truehd, etc.)
    pub codec: String,
    /// Codec long name
    pub codec_long_name: Option<String>,
    /// Number of channels
    pub channels: i32,
    /// Channel layout (stereo, 5.1, 7.1, etc.)
    pub channel_layout: Option<String>,
    /// Sample rate in Hz
    pub sample_rate: Option<i32>,
    /// Bitrate in bps
    pub bitrate: Option<i64>,
    /// Bit depth
    pub bit_depth: Option<i32>,
    /// Language code
    pub language: Option<String>,
    /// Stream title
    pub title: Option<String>,
    /// Whether this is the default track
    pub is_default: bool,
    /// Whether this is a commentary track
    pub is_commentary: bool,
}

impl AudioStreamInfo {
    pub fn from_record(record: crate::db::AudioStreamRecord) -> Self {
        Self {
            id: record.id.to_string(),
            stream_index: record.stream_index,
            codec: record.codec,
            codec_long_name: record.codec_long_name,
            channels: record.channels,
            channel_layout: record.channel_layout,
            sample_rate: record.sample_rate,
            bitrate: record.bitrate,
            bit_depth: record.bit_depth,
            language: record.language,
            title: record.title,
            is_default: record.is_default,
            is_commentary: record.is_commentary,
        }
    }
}

/// Chapter information from media file
#[derive(Debug, Clone, SimpleObject)]
pub struct ChapterInfo {
    /// Unique identifier
    pub id: String,
    /// Chapter index (0-based)
    pub chapter_index: i32,
    /// Start time in seconds
    pub start_secs: f64,
    /// End time in seconds
    pub end_secs: f64,
    /// Chapter title
    pub title: Option<String>,
}

impl ChapterInfo {
    pub fn from_record(record: crate::db::ChapterRecord) -> Self {
        Self {
            id: record.id.to_string(),
            chapter_index: record.chapter_index,
            start_secs: record.start_secs,
            end_secs: record.end_secs,
            title: record.title,
        }
    }
}

/// Detailed media file info including all streams
#[derive(Debug, Clone)]
pub struct MediaFileDetails {
    /// Basic file info
    pub file: MediaFile,
    /// Video streams
    pub video_streams: Vec<VideoStreamInfo>,
    /// Audio streams
    pub audio_streams: Vec<AudioStreamInfo>,
    /// Subtitle tracks
    pub subtitles: Vec<Subtitle>,
    /// Chapters
    pub chapters: Vec<ChapterInfo>,
}

#[Object]
impl MediaFileDetails {
    async fn id(&self) -> &str {
        &self.file.id
    }

    async fn file(&self) -> &MediaFile {
        &self.file
    }

    async fn video_streams(&self) -> &Vec<VideoStreamInfo> {
        &self.video_streams
    }

    async fn audio_streams(&self) -> &Vec<AudioStreamInfo> {
        &self.audio_streams
    }

    async fn subtitles(&self) -> &Vec<Subtitle> {
        &self.subtitles
    }

    async fn chapters(&self) -> &Vec<ChapterInfo> {
        &self.chapters
    }
}

/// Subtitle settings for a library or show
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct SubtitleSettings {
    /// Whether to auto-download missing subtitles
    pub auto_download: bool,
    /// Preferred subtitle languages (ISO 639-1 codes)
    pub languages: Vec<String>,
}

/// Input for updating subtitle settings
#[derive(Debug, Clone, InputObject)]
pub struct SubtitleSettingsInput {
    /// Whether to auto-download missing subtitles (null = inherit from parent)
    pub auto_download: Option<bool>,
    /// Preferred subtitle languages (null = inherit from parent)
    pub languages: Option<Vec<String>>,
}

/// OpenSubtitles search result
#[derive(Debug, Clone, SimpleObject)]
pub struct SubtitleSearchResult {
    /// Subtitle file ID (for download)
    pub file_id: String,
    /// Language code
    pub language: Option<String>,
    /// Release name
    pub release: Option<String>,
    /// Download count (popularity)
    pub download_count: Option<i64>,
    /// Whether it's for hearing impaired
    pub hearing_impaired: bool,
    /// File name
    pub file_name: Option<String>,
    /// Rating
    pub rating: Option<f64>,
}

/// Input for searching subtitles
#[derive(Debug, Clone, InputObject)]
pub struct SearchSubtitlesInput {
    /// Media file ID to search subtitles for
    pub media_file_id: String,
    /// Languages to search for (ISO 639-1 codes)
    pub languages: Vec<String>,
    /// Optional: IMDB ID for better matching
    pub imdb_id: Option<String>,
}

/// Result of analyzing a media file
#[derive(Debug, Clone, SimpleObject)]
pub struct AnalyzeMediaFileResult {
    pub success: bool,
    pub error: Option<String>,
    /// Number of video streams found
    pub video_stream_count: Option<i32>,
    /// Number of audio streams found
    pub audio_stream_count: Option<i32>,
    /// Number of subtitle streams found
    pub subtitle_stream_count: Option<i32>,
    /// Number of chapters found
    pub chapter_count: Option<i32>,
}

// ============================================================================
// Indexer Types
// ============================================================================

/// Type of indexer implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum IndexerImplementationType {
    /// Native Rust implementation
    Native,
    /// YAML-based Cardigann definition
    Cardigann,
    /// RSS/Atom feed
    Feed,
    /// Newznab-compatible
    Newznab,
}

/// Type of tracker
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum, Serialize, Deserialize)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum TrackerTypeEnum {
    /// Private tracker requiring invitation
    Private,
    /// Public tracker accessible to anyone
    Public,
    /// Semi-private tracker with open registration
    SemiPrivate,
}

/// An indexer configuration
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct IndexerConfig {
    /// Unique identifier
    pub id: String,
    /// Indexer type (e.g., "iptorrents", "cardigann")
    pub indexer_type: String,
    /// Display name
    pub name: String,
    /// Whether the indexer is enabled
    pub enabled: bool,
    /// Priority (higher = searched first)
    pub priority: i32,
    /// Site URL
    pub site_url: Option<String>,
    /// Whether the indexer is healthy (no recent errors)
    pub is_healthy: bool,
    /// Last error message if any
    pub last_error: Option<String>,
    /// Error count
    pub error_count: i32,
    /// Last successful search timestamp
    pub last_success_at: Option<String>,
    /// When the indexer was created
    pub created_at: String,
    /// When the indexer was last updated
    pub updated_at: String,
    /// Capabilities
    pub capabilities: IndexerCapabilities,
}

/// Indexer capabilities
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct IndexerCapabilities {
    /// Whether general search is available
    pub supports_search: bool,
    /// Whether TV search is available
    pub supports_tv_search: bool,
    /// Whether movie search is available
    pub supports_movie_search: bool,
    /// Whether music search is available
    pub supports_music_search: bool,
    /// Whether book search is available
    pub supports_book_search: bool,
    /// Whether IMDB search is available
    pub supports_imdb_search: bool,
    /// Whether TVDB search is available
    pub supports_tvdb_search: bool,
}

/// Information about an available indexer type
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct IndexerTypeInfo {
    /// Unique identifier (e.g., "iptorrents")
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Tracker type
    pub tracker_type: String,
    /// Language code
    pub language: String,
    /// Primary site URL
    pub site_link: String,
    /// Required credential types
    pub required_credentials: Vec<String>,
    /// Whether this is a native implementation
    pub is_native: bool,
}

/// A setting definition for an indexer
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct IndexerSettingDefinition {
    /// Setting key
    pub key: String,
    /// Display label
    pub label: String,
    /// Setting type (text, password, checkbox, select)
    pub setting_type: String,
    /// Default value
    pub default_value: Option<String>,
    /// Options for select type
    pub options: Option<Vec<IndexerSettingOption>>,
}

/// An option for a select setting
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct IndexerSettingOption {
    /// Option value
    pub value: String,
    /// Display label
    pub label: String,
}

/// Input for creating an indexer
#[derive(Debug, Clone, InputObject)]
pub struct CreateIndexerInput {
    /// Indexer type (e.g., "iptorrents")
    pub indexer_type: String,
    /// Display name
    pub name: String,
    /// Site URL override
    pub site_url: Option<String>,
    /// Credentials (cookie, api_key, etc.)
    pub credentials: Vec<IndexerCredentialInput>,
    /// Settings (freeleech, sort, etc.)
    pub settings: Vec<IndexerSettingInput>,
}

/// Input for updating an indexer
#[derive(Debug, Clone, InputObject)]
pub struct UpdateIndexerInput {
    /// Display name
    pub name: Option<String>,
    /// Whether the indexer is enabled
    pub enabled: Option<bool>,
    /// Priority (higher = searched first)
    pub priority: Option<i32>,
    /// Site URL override
    pub site_url: Option<String>,
    /// Updated credentials
    pub credentials: Option<Vec<IndexerCredentialInput>>,
    /// Updated settings
    pub settings: Option<Vec<IndexerSettingInput>>,
}

/// Input for a credential
#[derive(Debug, Clone, InputObject)]
pub struct IndexerCredentialInput {
    /// Credential type (cookie, api_key, user_agent, etc.)
    pub credential_type: String,
    /// Credential value (will be encrypted)
    pub value: String,
}

/// Input for a setting
#[derive(Debug, Clone, InputObject)]
pub struct IndexerSettingInput {
    /// Setting key
    pub key: String,
    /// Setting value
    pub value: String,
}

/// Input for searching indexers
#[derive(Debug, Clone, InputObject)]
pub struct IndexerSearchInput {
    /// Search query
    pub query: String,
    /// Specific indexer IDs to search (or all if empty)
    pub indexer_ids: Option<Vec<String>>,
    /// Torznab category IDs
    pub categories: Option<Vec<i32>>,
    /// Season number (for TV search)
    pub season: Option<i32>,
    /// Episode number (for TV search)
    pub episode: Option<String>,
    /// IMDB ID (e.g., "tt1234567")
    pub imdb_id: Option<String>,
    /// Result limit
    pub limit: Option<i32>,
}

/// Result of an indexer mutation
#[derive(Debug, Clone, SimpleObject)]
pub struct IndexerResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// The indexer if successful
    pub indexer: Option<IndexerConfig>,
}

/// Result of testing an indexer
#[derive(Debug, Clone, SimpleObject)]
pub struct IndexerTestResult {
    /// Whether the test succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Number of releases found in test query
    pub releases_found: Option<i32>,
    /// Time taken in milliseconds
    pub elapsed_ms: Option<i64>,
}

/// Result of an indexer search
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct IndexerSearchResultSet {
    /// Results from each indexer
    pub indexers: Vec<IndexerSearchResultItem>,
    /// Total releases found
    pub total_releases: i32,
    /// Total time taken in milliseconds
    pub total_elapsed_ms: i64,
}

/// Search results from a single indexer
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct IndexerSearchResultItem {
    /// Indexer ID
    pub indexer_id: String,
    /// Indexer name
    pub indexer_name: String,
    /// Releases found
    pub releases: Vec<TorrentRelease>,
    /// Time taken in milliseconds
    pub elapsed_ms: i64,
    /// Whether results came from cache
    pub from_cache: bool,
    /// Error message if search failed
    pub error: Option<String>,
}

/// A torrent release from an indexer search
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct TorrentRelease {
    /// Release title
    pub title: String,
    /// Unique identifier
    pub guid: String,
    /// Download link
    pub link: Option<String>,
    /// Magnet URI
    pub magnet_uri: Option<String>,
    /// Info hash
    pub info_hash: Option<String>,
    /// Details page URL
    pub details: Option<String>,
    /// Publication date (ISO 8601)
    pub publish_date: String,
    /// Torznab category IDs
    pub categories: Vec<i32>,
    /// File size in bytes
    pub size: Option<i64>,
    /// Human-readable size
    pub size_formatted: Option<String>,
    /// Number of seeders
    pub seeders: Option<i32>,
    /// Number of leechers
    pub leechers: Option<i32>,
    /// Number of peers (seeders + leechers)
    pub peers: Option<i32>,
    /// Number of downloads/grabs
    pub grabs: Option<i32>,
    /// Whether this is freeleech
    pub is_freeleech: bool,
    /// IMDB ID
    pub imdb_id: Option<String>,
    /// Poster/cover image URL
    pub poster: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Indexer ID that found this release
    pub indexer_id: Option<String>,
    /// Indexer name
    pub indexer_name: Option<String>,
}

// =============================================================================
// Security Settings Types
// =============================================================================

/// Security settings for the application
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct SecuritySettings {
    /// Whether the indexer encryption key is set
    pub encryption_key_set: bool,
    /// Masked version of the encryption key (first/last 4 chars)
    pub encryption_key_preview: Option<String>,
    /// When the encryption key was last changed
    pub encryption_key_last_modified: Option<String>,
}

/// Result of updating security settings
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct SecuritySettingsResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// The updated settings
    pub settings: Option<SecuritySettings>,
}

/// Input for generating a new encryption key
#[derive(Debug, Clone, InputObject)]
pub struct GenerateEncryptionKeyInput {
    /// Confirm that you understand this will invalidate existing credentials
    pub confirm_invalidation: bool,
}

// =============================================================================
// Filesystem Types
// =============================================================================

/// A file or directory entry
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct FileEntry {
    /// File/directory name
    pub name: String,
    /// Full path
    pub path: String,
    /// Is this a directory?
    pub is_dir: bool,
    /// File size in bytes (0 for directories)
    pub size: i64,
    /// Human-readable file size
    pub size_formatted: String,
    /// Is this path readable?
    pub readable: bool,
    /// Is this path writable?
    pub writable: bool,
    /// MIME type (for files)
    pub mime_type: Option<String>,
    /// Last modified timestamp (ISO 8601)
    pub modified_at: Option<String>,
}

/// A quick-access path shortcut
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct QuickPath {
    /// Display name
    pub name: String,
    /// Full path
    pub path: String,
}

/// Result of browsing a directory
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct BrowseDirectoryResult {
    /// Current path being browsed
    pub current_path: String,
    /// Parent path (null if at root)
    pub parent_path: Option<String>,
    /// List of entries in the directory
    pub entries: Vec<FileEntry>,
    /// Common quick-access paths
    pub quick_paths: Vec<QuickPath>,
    /// Whether this path is inside a library
    pub is_library_path: bool,
    /// Library ID if path is inside a library
    pub library_id: Option<String>,
}

/// Input for browsing a directory
#[derive(Debug, Clone, InputObject)]
pub struct BrowseDirectoryInput {
    /// Path to browse (defaults to root or home)
    pub path: Option<String>,
    /// Only show directories
    pub dirs_only: Option<bool>,
    /// Show hidden files (starting with .)
    pub show_hidden: Option<bool>,
}

/// Result of a file operation
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct FileOperationResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Number of items affected
    pub affected_count: i32,
    /// Detailed messages about what happened
    pub messages: Vec<String>,
    /// Updated path (for create operations)
    pub path: Option<String>,
}

/// Input for creating a directory
#[derive(Debug, Clone, InputObject)]
pub struct CreateDirectoryInput {
    /// Full path of the directory to create
    pub path: String,
}

/// Input for deleting files or directories
#[derive(Debug, Clone, InputObject)]
pub struct DeleteFilesInput {
    /// Paths to delete
    pub paths: Vec<String>,
    /// Whether to allow deleting non-empty directories
    pub recursive: Option<bool>,
}

/// Input for copying files or directories
#[derive(Debug, Clone, InputObject)]
pub struct CopyFilesInput {
    /// Source paths to copy
    pub sources: Vec<String>,
    /// Destination directory
    pub destination: String,
    /// Whether to overwrite existing files
    pub overwrite: Option<bool>,
}

/// Input for moving files or directories
#[derive(Debug, Clone, InputObject)]
pub struct MoveFilesInput {
    /// Source paths to move
    pub sources: Vec<String>,
    /// Destination directory
    pub destination: String,
    /// Whether to overwrite existing files
    pub overwrite: Option<bool>,
}

/// Input for renaming a file or directory
#[derive(Debug, Clone, InputObject)]
pub struct RenameFileInput {
    /// Current path of the file/directory
    pub path: String,
    /// New name (not full path, just the name)
    pub new_name: String,
}

/// Event emitted when directory contents change
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct DirectoryChangeEvent {
    /// Directory path that changed
    pub path: String,
    /// Type of change: "created", "modified", "deleted", "renamed"
    pub change_type: String,
    /// Affected file/directory name
    pub name: Option<String>,
    /// New name (for rename events)
    pub new_name: Option<String>,
    /// Timestamp of the change (ISO 8601)
    pub timestamp: String,
}

/// Path validation result
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct PathValidationResult {
    /// Whether the path is valid
    pub is_valid: bool,
    /// Whether the path is inside a library
    pub is_library_path: bool,
    /// Library ID if path is inside a library
    pub library_id: Option<String>,
    /// Library name if path is inside a library
    pub library_name: Option<String>,
    /// Error message if path is invalid
    pub error: Option<String>,
}
