//! Torrent client types and utilities.
//!
//! Provides public types, configuration, and helper functions for the torrent service.

use std::path::PathBuf;

use librqbit::AddTorrentOptions;
use serde::{Deserialize, Serialize};

pub fn add_torrent_opts(paused: bool) -> AddTorrentOptions {
    AddTorrentOptions {
        overwrite: true,
        paused,
        ..Default::default()
    }
}

/// Map librqbit's session state plus download progress onto the simplified
/// [`TorrentState`] the API and the `torrents.state` column use.
///
/// Single source of truth for this mapping: the progress loop, the live
/// `TorrentInfo` builder and the session→DB sync all call this, so a
/// `LiveTorrent.state` can never disagree with the persisted `Torrent.state`.
pub fn torrent_state_from_stats(
    state: &librqbit::TorrentStatsState,
    progress: f64,
) -> TorrentState {
    use librqbit::TorrentStatsState;
    match state {
        TorrentStatsState::Paused => TorrentState::Paused,
        TorrentStatsState::Error => TorrentState::Error,
        TorrentStatsState::Live if progress >= 1.0 => TorrentState::Seeding,
        TorrentStatsState::Live => TorrentState::Downloading,
        TorrentStatsState::Initializing { .. } => TorrentState::Queued,
    }
}

/// Fraction of the torrent that is on disk, guarding against the
/// zero-total-bytes case a magnet has before its metadata resolves.
pub fn progress_ratio(progress_bytes: u64, total_bytes: u64) -> f64 {
    if total_bytes == 0 {
        return 0.0;
    }
    (progress_bytes as f64 / total_bytes as f64).min(1.0)
}

pub fn get_info_hash_hex<T: AsRef<librqbit::ManagedTorrent>>(handle: &T) -> String {
    handle
        .as_ref()
        .info_hash()
        .0
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

// -----------------------------------------------------------------------------
// Public types (re-exported for GraphQL, jobs, etc.)
// -----------------------------------------------------------------------------

/// UPnP port forwarding result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UpnpResult {
    pub success: bool,
    pub tcp_forwarded: bool,
    pub udp_forwarded: bool,
    pub local_ip: Option<String>,
    pub external_ip: Option<String>,
    pub error: Option<String>,
}

/// Events broadcast when torrent state changes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TorrentEvent {
    Added {
        id: usize,
        name: String,
        info_hash: String,
    },
    Progress {
        id: usize,
        info_hash: String,
        progress: f64,
        download_speed: u64,
        upload_speed: u64,
        peers: usize,
        state: TorrentState,
    },
    Completed {
        id: usize,
        info_hash: String,
        name: String,
    },
    Removed {
        id: usize,
        info_hash: String,
    },
    Error {
        id: usize,
        info_hash: String,
        message: String,
    },
}

/// Simplified torrent state for API
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TorrentState {
    Queued,
    Checking,
    Downloading,
    Seeding,
    Paused,
    Error,
}

impl std::fmt::Display for TorrentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TorrentState::Queued => write!(f, "queued"),
            TorrentState::Checking => write!(f, "checking"),
            TorrentState::Downloading => write!(f, "downloading"),
            TorrentState::Seeding => write!(f, "seeding"),
            TorrentState::Paused => write!(f, "paused"),
            TorrentState::Error => write!(f, "error"),
        }
    }
}

/// Information about a torrent for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TorrentInfo {
    pub id: usize,
    pub info_hash: String,
    pub name: String,
    pub state: TorrentState,
    pub progress: f64,
    pub size: u64,
    pub downloaded: u64,
    pub uploaded: u64,
    pub download_speed: u64,
    pub upload_speed: u64,
    pub peers: usize,
    pub seeds: usize,
    pub save_path: String,
    pub files: Vec<TorrentFile>,
}

/// Information about a file within a torrent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TorrentFile {
    pub index: usize,
    pub path: String,
    pub size: u64,
    pub progress: f64,
}

/// Configuration for the torrent service
#[derive(Debug, Clone)]
pub struct TorrentServiceConfig {
    pub download_dir: PathBuf,
    pub session_dir: PathBuf,
    pub enable_dht: bool,
    pub listen_port: u16,
    pub max_concurrent: usize,
    pub upload_limit: u32,
    pub download_limit: u32,
    /// `torrent.seed_ratio_limit`: stop seeding at this share ratio (0 = never).
    pub seed_ratio_limit: f64,
    /// `torrent.seed_time_minutes`: stop seeding after this long (0 = never).
    pub seed_time_minutes: i64,
    /// `torrent.remove_after_import`: delete the payload once the import
    /// completed and the seeding rules are satisfied.
    pub remove_after_import: bool,
}

impl Default for TorrentServiceConfig {
    fn default() -> Self {
        Self {
            download_dir: PathBuf::from("/data/downloads"),
            session_dir: PathBuf::from("/data/session"),
            enable_dht: true,
            listen_port: 0,
            max_concurrent: 5,
            upload_limit: 0,
            download_limit: 0,
            seed_ratio_limit: 1.0,
            seed_time_minutes: 0,
            remove_after_import: false,
        }
    }
}

impl TorrentServiceConfig {
    /// Seeding rules derived from the `torrent.seed_*` / `torrent.remove_after_import`
    /// settings.
    pub fn seeding_rules(&self) -> super::seeding::SeedingRules {
        super::seeding::SeedingRules {
            ratio_limit: self.seed_ratio_limit,
            time_minutes: self.seed_time_minutes,
            remove_after_import: self.remove_after_import,
        }
    }
}

pub fn perform_upnp(port: u16) -> UpnpResult {
    use igd::{PortMappingProtocol, SearchOptions, search_gateway};
    use std::time::Duration;

    let search_options = SearchOptions {
        timeout: Some(Duration::from_secs(3)),
        ..Default::default()
    };

    let gateway = match search_gateway(search_options) {
        Ok(g) => g,
        Err(e) => {
            return UpnpResult {
                success: false,
                tcp_forwarded: false,
                udp_forwarded: false,
                local_ip: None,
                external_ip: None,
                error: Some(format!("Failed to find UPnP gateway: {}", e)),
            };
        }
    };

    let local_ip = match local_ip_address::local_ip() {
        Ok(ip) => ip,
        Err(e) => {
            return UpnpResult {
                success: false,
                tcp_forwarded: false,
                udp_forwarded: false,
                local_ip: None,
                external_ip: None,
                error: Some(format!("Failed to get local IP: {}", e)),
            };
        }
    };

    let local_ipv4 = match local_ip {
        std::net::IpAddr::V4(ip) => ip,
        std::net::IpAddr::V6(_) => {
            return UpnpResult {
                success: false,
                tcp_forwarded: false,
                udp_forwarded: false,
                local_ip: None,
                external_ip: None,
                error: Some("UPnP requires IPv4".to_string()),
            };
        }
    };

    let external_ip = match gateway.get_external_ip() {
        Ok(ip) => ip,
        Err(e) => {
            return UpnpResult {
                success: false,
                tcp_forwarded: false,
                udp_forwarded: false,
                local_ip: Some(local_ipv4.to_string()),
                external_ip: None,
                error: Some(format!("Failed to get external IP: {}", e)),
            };
        }
    };

    let local_addr = std::net::SocketAddrV4::new(local_ipv4, port);
    let mut tcp_forwarded = false;
    let mut udp_forwarded = false;

    if gateway
        .add_port(
            PortMappingProtocol::TCP,
            port,
            local_addr,
            3600,
            "Librarian Torrent Client",
        )
        .is_ok()
    {
        tcp_forwarded = true;
    }
    if gateway
        .add_port(
            PortMappingProtocol::UDP,
            port,
            local_addr,
            3600,
            "Librarian Torrent Client",
        )
        .is_ok()
    {
        udp_forwarded = true;
    }

    UpnpResult {
        success: tcp_forwarded || udp_forwarded,
        tcp_forwarded,
        udp_forwarded,
        local_ip: Some(local_ipv4.to_string()),
        external_ip: Some(external_ip.to_string()),
        error: if tcp_forwarded || udp_forwarded {
            None
        } else {
            Some("Failed to forward both TCP and UDP".to_string())
        },
    }
}
