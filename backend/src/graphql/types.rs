//! GraphQL type definitions
//!
//! These types mirror our domain models but are decorated with async-graphql attributes.

use async_graphql::{Enum, InputObject, Object, SimpleObject};
use serde::{Deserialize, Serialize};

use crate::services::{TorrentInfo as ServiceTorrentInfo, TorrentState as ServiceTorrentState};

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

/// Input for creating a library
#[derive(Debug, InputObject)]
pub struct CreateLibraryInput {
    /// Display name
    pub name: String,
    /// Filesystem path
    pub path: String,
    /// Library type
    pub library_type: LibraryType,
    /// Optional icon name
    pub icon: Option<String>,
    /// Optional color theme
    pub color: Option<String>,
    /// Enable auto-scan (default: true)
    pub auto_scan: Option<bool>,
    /// Scan interval in hours (default: 24)
    pub scan_interval_hours: Option<i32>,
}

/// Input for updating a library
#[derive(Debug, InputObject)]
pub struct UpdateLibraryInput {
    /// New display name
    pub name: Option<String>,
    /// New filesystem path
    pub path: Option<String>,
    /// New icon name
    pub icon: Option<String>,
    /// New color theme
    pub color: Option<String>,
    /// Enable/disable auto-scan
    pub auto_scan: Option<bool>,
    /// New scan interval in hours
    pub scan_interval_hours: Option<i32>,
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

/// Cast session for Chromecast/AirPlay
#[derive(Debug, Clone, SimpleObject)]
pub struct CastSession {
    pub session_token: String,
    pub stream_url: String,
    pub expires_at: String,
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

/// Library scan progress event
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
    pub episode_count: i32,
    pub episode_file_count: i32,
    pub size_bytes: i64,
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
}

/// Result of TV show mutation
#[derive(Debug, SimpleObject)]
pub struct TvShowResult {
    pub success: bool,
    pub tv_show: Option<TvShow>,
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
}

/// Episode with show information (for wanted list)
#[derive(Debug, Clone, SimpleObject, Serialize, Deserialize)]
pub struct WantedEpisode {
    pub episode: Episode,
    pub show_name: String,
    pub show_year: Option<i32>,
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
    pub auto_rename: bool,
    pub naming_pattern: Option<String>,
    pub default_quality_profile_id: Option<String>,
    pub auto_add_discovered: bool,
    pub item_count: i32,
    pub total_size_bytes: i64,
    pub show_count: i32,
    pub last_scanned_at: Option<String>,
}

/// Enhanced input for creating a library
#[derive(Debug, InputObject)]
pub struct CreateLibraryFullInput {
    pub name: String,
    pub path: String,
    pub library_type: LibraryType,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub auto_scan: Option<bool>,
    pub scan_interval_minutes: Option<i32>,
    pub watch_for_changes: Option<bool>,
    pub post_download_action: Option<PostDownloadAction>,
    pub auto_rename: Option<bool>,
    pub naming_pattern: Option<String>,
    pub default_quality_profile_id: Option<String>,
    pub auto_add_discovered: Option<bool>,
}

/// Enhanced input for updating a library
#[derive(Debug, InputObject)]
pub struct UpdateLibraryFullInput {
    pub name: Option<String>,
    pub path: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub auto_scan: Option<bool>,
    pub scan_interval_minutes: Option<i32>,
    pub watch_for_changes: Option<bool>,
    pub post_download_action: Option<PostDownloadAction>,
    pub auto_rename: Option<bool>,
    pub naming_pattern: Option<String>,
    pub default_quality_profile_id: Option<String>,
    pub auto_add_discovered: Option<bool>,
}

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
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
