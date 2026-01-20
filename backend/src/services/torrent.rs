//! Native torrent client service using librqbit
//!
//! This module provides a wrapper around librqbit for managing torrent downloads
//! with real-time status updates via broadcast channels and database persistence.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use librqbit::api::TorrentIdOrHash;
use librqbit::dht::PersistentDhtConfig;
use librqbit::{AddTorrent, AddTorrentOptions, AddTorrentResponse, Session, SessionOptions};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::{CreateTorrent, Database, TorrentRepository};

/// Helper to get info hash as hex string from handle
fn get_info_hash_hex<T: AsRef<librqbit::ManagedTorrent>>(handle: &T) -> String {
    // The info_hash() returns Id<20> which has a .0 field with [u8; 20]
    handle
        .as_ref()
        .info_hash()
        .0
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
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
pub struct TorrentFile {
    pub index: usize,
    pub path: String,
    pub size: u64,
    pub progress: f64,
}

/// Detailed peer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerStats {
    pub queued: usize,
    pub connecting: usize,
    pub live: usize,
    pub seen: usize,
    pub dead: usize,
    pub not_needed: usize,
}

/// Detailed torrent information for the info modal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentDetails {
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
    pub save_path: String,
    pub files: Vec<TorrentFile>,
    // Detailed stats
    pub piece_count: u64,
    pub pieces_downloaded: u64,
    pub average_piece_download_ms: Option<u64>,
    pub time_remaining_secs: Option<u64>,
    pub peer_stats: PeerStats,
    // Error info
    pub error: Option<String>,
    pub finished: bool,
}

/// Configuration for the torrent service
#[derive(Debug, Clone)]
pub struct TorrentServiceConfig {
    pub download_dir: PathBuf,
    pub session_dir: PathBuf,
    pub enable_dht: bool,
    pub listen_port: u16,
    pub max_concurrent: usize,
}

impl Default for TorrentServiceConfig {
    fn default() -> Self {
        Self {
            download_dir: PathBuf::from("/data/downloads"),
            session_dir: PathBuf::from("/data/session"),
            enable_dht: true,
            listen_port: 0,
            max_concurrent: 5,
        }
    }
}

/// Native torrent client service
pub struct TorrentService {
    session: Arc<Session>,
    config: TorrentServiceConfig,
    db: Database,
    event_tx: broadcast::Sender<TorrentEvent>,
    completed: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl TorrentService {
    /// Create a new torrent service with database persistence
    ///
    /// This will attempt to load settings from the database first, then fall back to config.
    /// Directories will be created if possible, but the service will start even if they
    /// cannot be created (with a warning).
    pub async fn new(mut config: TorrentServiceConfig, db: Database) -> Result<Self> {
        // Try to load settings from database
        let settings = db.settings();
        if let Ok(Some(dir)) = settings.get_value::<String>("torrent.download_dir").await {
            config.download_dir = PathBuf::from(dir);
        }
        if let Ok(Some(dir)) = settings.get_value::<String>("torrent.session_dir").await {
            config.session_dir = PathBuf::from(dir);
        }
        if let Ok(Some(v)) = settings.get_value::<bool>("torrent.enable_dht").await {
            config.enable_dht = v;
        }
        if let Ok(Some(v)) = settings.get_value::<u16>("torrent.listen_port").await {
            config.listen_port = v;
        }
        if let Ok(Some(v)) = settings.get_value::<usize>("torrent.max_concurrent").await {
            config.max_concurrent = v;
        }

        info!("Using download directory: {}", config.download_dir.display());

        // Try to create directories, but don't fail if we can't
        let download_dir_ok = match tokio::fs::create_dir_all(&config.download_dir).await {
            Ok(_) => {
                debug!("Download directory ready: {}", config.download_dir.display());
                true
            }
            Err(e) => {
                warn!(path = %config.download_dir.display(), error = %e,
                    "Could not create download directory. Please configure a valid path in Settings.");
                false
            }
        };

        let session_dir_ok = match tokio::fs::create_dir_all(&config.session_dir).await {
            Ok(_) => {
                debug!("Session directory ready: {}", config.session_dir.display());
                true
            }
            Err(e) => {
                warn!(path = %config.session_dir.display(), error = %e,
                    "Could not create session directory. Please configure a valid path in Settings.");
                false
            }
        };

        // If directories aren't ready, use a temp directory as fallback
        let effective_download_dir = if download_dir_ok {
            config.download_dir.clone()
        } else {
            let temp_dir = std::env::temp_dir().join("librarian-downloads");
            warn!(path = %temp_dir.display(), "Using temp directory for downloads");
            tokio::fs::create_dir_all(&temp_dir).await.ok();
            temp_dir
        };

        let effective_session_dir = if session_dir_ok {
            config.session_dir.clone()
        } else {
            let temp_dir = std::env::temp_dir().join("librarian-session");
            warn!(path = %temp_dir.display(), "Using temp directory for session");
            tokio::fs::create_dir_all(&temp_dir).await.ok();
            temp_dir
        };

        let dht_config = if config.enable_dht {
            Some(PersistentDhtConfig {
                config_filename: Some(effective_session_dir.join("dht.json")),
                ..Default::default()
            })
        } else {
            None
        };

        let session_opts = SessionOptions {
            disable_dht: !config.enable_dht,
            disable_dht_persistence: !config.enable_dht,
            dht_config,
            persistence: Some(librqbit::SessionPersistenceConfig::Json {
                folder: Some(effective_session_dir.clone()),
            }),
            listen_port_range: if config.listen_port > 0 {
                Some(config.listen_port..config.listen_port + 1)
            } else {
                None
            },
            ..Default::default()
        };

        let session = Session::new_with_opts(effective_download_dir.clone(), session_opts)
            .await
            .context("Failed to create torrent session")?;

        let (event_tx, _) = broadcast::channel(1024);

        // Update config with effective paths
        config.download_dir = effective_download_dir;
        config.session_dir = effective_session_dir;

        let service = Self {
            session,
            config,
            db,
            event_tx,
            completed: Arc::new(RwLock::new(std::collections::HashSet::new())),
        };

        // First, sync any torrents loaded from session files TO the database
        // This ensures the database reflects all torrents the session knows about
        if let Err(e) = service.sync_session_to_database().await {
            warn!(error = %e, "Failed to sync session torrents to database");
        }

        // Then, restore any additional torrents from database that aren't in the session
        if let Err(e) = service.restore_from_database().await {
            warn!(error = %e, "Failed to restore torrents from database");
        }

        // Start background monitors
        service.start_progress_monitor();
        service.start_db_sync();

        info!("Torrent service initialized");
        Ok(service)
    }

    /// Sync all torrents from the librqbit session to the database
    /// This ensures the database reflects the current state of the session
    async fn sync_session_to_database(&self) -> Result<()> {
        let repo = self.db.torrents();

        // Get a fallback user_id for creating new records
        let fallback_user_id = match repo.get_default_user_id().await? {
            Some(uid) => uid,
            None => {
                info!("No user found in database, skipping session sync");
                return Ok(());
            }
        };

        // Get all torrents from the session
        let session_torrents: Vec<(usize, Arc<librqbit::ManagedTorrent>)> = self
            .session
            .with_torrents(|iter| iter.map(|(id, h)| (id, h.clone())).collect());

        info!(
            count = session_torrents.len(),
            "Syncing session torrents to database"
        );

        for (id, handle) in session_torrents {
            let info_hash = get_info_hash_hex(&handle);
            let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
            let stats = handle.stats();
            let progress = stats.progress_bytes as f64 / stats.total_bytes.max(1) as f64;

            let state = {
                use librqbit::TorrentStatsState;
                match &stats.state {
                    TorrentStatsState::Paused => "paused",
                    TorrentStatsState::Error => "error",
                    TorrentStatsState::Live if progress >= 1.0 => "seeding",
                    TorrentStatsState::Live => "downloading",
                    TorrentStatsState::Initializing => "queued",
                }
            };

            if let Err(e) = repo
                .upsert_from_session(
                    &info_hash,
                    &name,
                    state,
                    progress,
                    stats.total_bytes as i64,
                    stats.progress_bytes as i64,
                    stats.uploaded_bytes as i64,
                    &self.config.download_dir.to_string_lossy(),
                    fallback_user_id,
                )
                .await
            {
                warn!("Failed to sync torrent '{}' to database: {}", name, e);
            } else {
                debug!("Synced torrent '{}' to database", name);
            }
        }

        Ok(())
    }

    /// Restore torrents from database on startup
    async fn restore_from_database(&self) -> Result<()> {
        let repo = self.db.torrents();
        let records = repo.list_resumable().await?;

        info!("Restoring {} torrents from database", records.len());

        for record in records {
            if let Some(magnet) = &record.magnet_uri {
                match self
                    .session
                    .add_torrent(
                        AddTorrent::from_url(magnet),
                        Some(AddTorrentOptions::default()),
                    )
                    .await
                {
                    Ok(_) => {
                        info!("Restored torrent '{}'", record.name);
                    }
                    Err(e) => {
                        warn!(info_hash = %record.info_hash, error = %e, "Failed to restore torrent");
                        // Mark as error in database
                        let _ = repo.update_state(&record.info_hash, "error").await;
                    }
                }
            }
        }

        Ok(())
    }

    /// Get the torrent repository
    pub fn repo(&self) -> TorrentRepository {
        self.db.torrents()
    }

    /// Subscribe to torrent events - for GraphQL subscriptions
    #[allow(dead_code)]
    pub fn subscribe(&self) -> broadcast::Receiver<TorrentEvent> {
        self.event_tx.subscribe()
    }

    pub fn event_sender(&self) -> broadcast::Sender<TorrentEvent> {
        self.event_tx.clone()
    }

    /// Add a torrent from a magnet link, persisting to database
    pub async fn add_magnet(&self, magnet: &str, user_id: Option<Uuid>) -> Result<TorrentInfo> {
        debug!("Adding torrent from magnet link");

        let add_result = self
            .session
            .add_torrent(
                AddTorrent::from_url(magnet),
                Some(AddTorrentOptions::default()),
            )
            .await
            .context("Failed to add torrent")?;

        match add_result {
            AddTorrentResponse::Added(id, handle) => {
                let info_hash = get_info_hash_hex(&handle);
                let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
                let stats = handle.stats();

                // Persist to database if user_id provided
                if let Some(uid) = user_id {
                    let repo = self.db.torrents();
                    if let Err(e) = repo
                        .create(CreateTorrent {
                            user_id: uid,
                            info_hash: info_hash.clone(),
                            magnet_uri: Some(magnet.to_string()),
                            name: name.clone(),
                            save_path: self.config.download_dir.to_string_lossy().to_string(),
                            total_bytes: stats.total_bytes as i64,
                        })
                        .await
                    {
                        error!(error = %e, "Failed to persist torrent to database");
                    }
                }

                let _ = self.event_tx.send(TorrentEvent::Added {
                    id,
                    name,
                    info_hash,
                });
                self.get_torrent_info(id).await
            }
            AddTorrentResponse::AlreadyManaged(id, _) => {
                warn!(id = %id, "Torrent already exists");
                self.get_torrent_info(id).await
            }
            AddTorrentResponse::ListOnly(_) => anyhow::bail!("Torrent was added in list-only mode"),
        }
    }

    /// Add a torrent without user context (for internal use)
    pub async fn add_magnet_internal(&self, magnet: &str) -> Result<TorrentInfo> {
        self.add_magnet(magnet, None).await
    }

    /// Add a torrent from .torrent file data with database persistence
    pub async fn add_torrent_file(
        &self,
        data: Vec<u8>,
        user_id: Option<Uuid>,
    ) -> Result<TorrentInfo> {
        debug!("Adding torrent from file data");

        let add_result = self
            .session
            .add_torrent(
                AddTorrent::from_bytes(data),
                Some(AddTorrentOptions::default()),
            )
            .await
            .context("Failed to add torrent file")?;

        match add_result {
            AddTorrentResponse::Added(id, handle) => {
                let info_hash = get_info_hash_hex(&handle);
                let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
                let stats = handle.stats();

                // Persist to database (note: no magnet URI for file uploads, so can't auto-resume)
                if let Some(uid) = user_id {
                    let repo = self.db.torrents();
                    if let Err(e) = repo
                        .create(CreateTorrent {
                            user_id: uid,
                            info_hash: info_hash.clone(),
                            magnet_uri: None, // File uploads don't have a magnet URI
                            name: name.clone(),
                            save_path: self.config.download_dir.to_string_lossy().to_string(),
                            total_bytes: stats.total_bytes as i64,
                        })
                        .await
                    {
                        error!(error = %e, "Failed to persist torrent to database");
                    }
                }

                let _ = self.event_tx.send(TorrentEvent::Added {
                    id,
                    name,
                    info_hash,
                });
                self.get_torrent_info(id).await
            }
            AddTorrentResponse::AlreadyManaged(id, _) => self.get_torrent_info(id).await,
            AddTorrentResponse::ListOnly(_) => anyhow::bail!("Torrent was added in list-only mode"),
        }
    }

    /// Add a torrent from a URL to a .torrent file
    pub async fn add_torrent_url(&self, url: &str, user_id: Option<Uuid>) -> Result<TorrentInfo> {
        debug!("Adding torrent from URL: {}", url);

        // librqbit handles both magnet links and http URLs
        let add_result = self
            .session
            .add_torrent(
                AddTorrent::from_url(url),
                Some(AddTorrentOptions::default()),
            )
            .await
            .context("Failed to add torrent from URL")?;

        match add_result {
            AddTorrentResponse::Added(id, handle) => {
                let info_hash = get_info_hash_hex(&handle);
                let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
                let stats = handle.stats();

                // Persist to database
                if let Some(uid) = user_id {
                    let repo = self.db.torrents();
                    // Store the URL as magnet_uri for potential resumption
                    if let Err(e) = repo
                        .create(CreateTorrent {
                            user_id: uid,
                            info_hash: info_hash.clone(),
                            magnet_uri: Some(url.to_string()),
                            name: name.clone(),
                            save_path: self.config.download_dir.to_string_lossy().to_string(),
                            total_bytes: stats.total_bytes as i64,
                        })
                        .await
                    {
                        error!(error = %e, "Failed to persist torrent to database");
                    }
                }

                let _ = self.event_tx.send(TorrentEvent::Added {
                    id,
                    name,
                    info_hash,
                });
                self.get_torrent_info(id).await
            }
            AddTorrentResponse::AlreadyManaged(id, _) => {
                warn!(id = %id, "Torrent already exists");
                self.get_torrent_info(id).await
            }
            AddTorrentResponse::ListOnly(_) => anyhow::bail!("Torrent was added in list-only mode"),
        }
    }

    /// Add a torrent from raw bytes (for authenticated downloads)
    pub async fn add_torrent_bytes(&self, bytes: &[u8], user_id: Option<Uuid>) -> Result<TorrentInfo> {
        debug!("Adding torrent from bytes ({} bytes)", bytes.len());

        let add_result = self
            .session
            .add_torrent(
                AddTorrent::from_bytes(bytes.to_vec()),
                Some(AddTorrentOptions::default()),
            )
            .await
            .context("Failed to add torrent from bytes")?;

        match add_result {
            AddTorrentResponse::Added(id, handle) => {
                let info_hash = get_info_hash_hex(&handle);
                let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
                let stats = handle.stats();

                // Persist to database
                if let Some(uid) = user_id {
                    let repo = self.db.torrents();
                    if let Err(e) = repo
                        .create(CreateTorrent {
                            user_id: uid,
                            info_hash: info_hash.clone(),
                            magnet_uri: None,
                            name: name.clone(),
                            save_path: self.config.download_dir.to_string_lossy().to_string(),
                            total_bytes: stats.total_bytes as i64,
                        })
                        .await
                    {
                        error!(error = %e, "Failed to persist torrent to database");
                    }
                }

                let _ = self.event_tx.send(TorrentEvent::Added {
                    id,
                    name,
                    info_hash,
                });
                self.get_torrent_info(id).await
            }
            AddTorrentResponse::AlreadyManaged(id, _) => {
                warn!(id = %id, "Torrent already exists");
                self.get_torrent_info(id).await
            }
            AddTorrentResponse::ListOnly(_) => anyhow::bail!("Torrent was added in list-only mode"),
        }
    }

    pub async fn get_torrent_info(&self, id: usize) -> Result<TorrentInfo> {
        let handle = self
            .session
            .get(TorrentIdOrHash::Id(id))
            .context("Torrent not found")?;
        let stats = handle.stats();
        let info_hash = get_info_hash_hex(&handle);
        let name = handle.name().unwrap_or_else(|| "Unknown".to_string());

        let progress = stats.progress_bytes as f64 / stats.total_bytes.max(1) as f64;
        let state = self.map_state(&stats.state, progress);

        let (download_speed, upload_speed, peers) = stats
            .live
            .as_ref()
            .map(|live| {
                let dl = (live.download_speed.mbps * 125000.0) as u64;
                let ul = (live.upload_speed.mbps * 125000.0) as u64;
                let p = live.snapshot.peer_stats.live;
                (dl, ul, p)
            })
            .unwrap_or((0, 0, 0));

        // Get file info
        let files = self.get_torrent_files(&handle);

        Ok(TorrentInfo {
            id,
            info_hash,
            name,
            state,
            progress,
            size: stats.total_bytes,
            downloaded: stats.progress_bytes,
            uploaded: stats.uploaded_bytes,
            download_speed,
            upload_speed,
            peers,
            seeds: 0,
            save_path: self.config.download_dir.to_string_lossy().to_string(),
            files,
        })
    }

    /// Get detailed torrent information including peer stats, file progress, etc.
    pub async fn get_torrent_details(&self, id: usize) -> Result<TorrentDetails> {
        let handle = self
            .session
            .get(TorrentIdOrHash::Id(id))
            .context("Torrent not found")?;
        let stats = handle.stats();
        let info_hash = get_info_hash_hex(&handle);
        let name = handle.name().unwrap_or_else(|| "Unknown".to_string());

        let progress = stats.progress_bytes as f64 / stats.total_bytes.max(1) as f64;
        let state = self.map_state(&stats.state, progress);

        // Extract detailed stats from live stats
        let (download_speed, upload_speed, peer_stats, avg_piece_ms, time_remaining) = stats
            .live
            .as_ref()
            .map(|live| {
                let dl = (live.download_speed.mbps * 125000.0) as u64;
                let ul = (live.upload_speed.mbps * 125000.0) as u64;
                let ps = PeerStats {
                    queued: live.snapshot.peer_stats.queued,
                    connecting: live.snapshot.peer_stats.connecting,
                    live: live.snapshot.peer_stats.live,
                    seen: live.snapshot.peer_stats.seen,
                    dead: live.snapshot.peer_stats.dead,
                    not_needed: live.snapshot.peer_stats.not_needed,
                };
                let avg_ms = live
                    .average_piece_download_time
                    .map(|d| d.as_millis() as u64);
                // Estimate time remaining from download speed if available
                let time_rem = if dl > 0 && stats.total_bytes > stats.progress_bytes {
                    Some((stats.total_bytes - stats.progress_bytes) / dl)
                } else {
                    None
                };
                (dl, ul, ps, avg_ms, time_rem)
            })
            .unwrap_or((
                0,
                0,
                PeerStats {
                    queued: 0,
                    connecting: 0,
                    live: 0,
                    seen: 0,
                    dead: 0,
                    not_needed: 0,
                },
                None,
                None,
            ));

        // Get file info with progress
        let files = self.get_torrent_files_detailed(&handle, &stats);

        // Calculate piece count from file_progress array length
        let piece_count = stats.file_progress.len() as u64;
        let pieces_downloaded = stats.file_progress.iter().filter(|&&b| b > 0).count() as u64;

        Ok(TorrentDetails {
            id,
            info_hash,
            name,
            state,
            progress,
            size: stats.total_bytes,
            downloaded: stats.progress_bytes,
            uploaded: stats.uploaded_bytes,
            download_speed,
            upload_speed,
            save_path: self.config.download_dir.to_string_lossy().to_string(),
            files,
            piece_count,
            pieces_downloaded,
            average_piece_download_ms: avg_piece_ms,
            time_remaining_secs: time_remaining,
            peer_stats,
            error: stats.error.clone(),
            finished: stats.finished,
        })
    }

    /// Get file info with progress from stats
    fn get_torrent_files_detailed(
        &self,
        handle: &Arc<librqbit::ManagedTorrent>,
        stats: &librqbit::TorrentStats,
    ) -> Vec<TorrentFile> {
        let mut files = Vec::new();

        // Try to get file info from torrent metadata using file_infos
        if let Some(metadata) = handle.metadata.load_full() {
            for (idx, file_info) in metadata.file_infos.iter().enumerate() {
                let file_progress = stats.file_progress.get(idx).copied().unwrap_or(0);
                let size = file_info.len;
                let progress = if size > 0 {
                    file_progress as f64 / size as f64
                } else {
                    0.0
                };

                files.push(TorrentFile {
                    index: idx,
                    path: file_info.relative_filename.to_string_lossy().to_string(),
                    size,
                    progress: progress.min(1.0),
                });
            }
        }

        files
    }

    /// Get files for a torrent by info_hash
    pub async fn get_files_for_torrent(&self, info_hash: &str) -> Result<Vec<TorrentFile>> {
        // Find the torrent by info_hash
        let handle = self.session.with_torrents(|iter| {
            for (id, handle) in iter {
                if get_info_hash_hex(&handle) == info_hash {
                    return Some((id, handle.clone()));
                }
            }
            None
        });

        match handle {
            Some((_, h)) => Ok(self.get_torrent_files(&h)),
            None => anyhow::bail!("Torrent not found: {}", info_hash),
        }
    }

    /// Extract file information from a torrent handle
    fn get_torrent_files(&self, handle: &Arc<librqbit::ManagedTorrent>) -> Vec<TorrentFile> {
        let mut files = Vec::new();

        // Get file info from torrent metadata
        if let Some(metadata) = handle.metadata.load_full() {
            let stats = handle.stats();

            for (idx, file_info) in metadata.file_infos.iter().enumerate() {
                let file_progress = stats.file_progress.get(idx).copied().unwrap_or(0);
                let size = file_info.len;
                let progress = if size > 0 {
                    file_progress as f64 / size as f64
                } else {
                    0.0
                };

                // Build the full path: download_dir / torrent_name / relative_filename
                // For single-file torrents, there might not be a folder
                let torrent_name = handle.name().unwrap_or_else(|| "unknown".to_string());
                let relative_path = file_info.relative_filename.to_string_lossy().to_string();

                // Check if there's a torrent folder or if files are directly in download dir
                let full_path = if metadata.file_infos.len() == 1 {
                    // Single file torrent - use the actual filename from metadata
                    // (not the torrent name, which may differ from the actual file)
                    self.config
                        .download_dir
                        .join(&relative_path)
                        .to_string_lossy()
                        .to_string()
                } else {
                    // Multi-file torrent - files are in a folder named after the torrent
                    self.config
                        .download_dir
                        .join(&torrent_name)
                        .join(&relative_path)
                        .to_string_lossy()
                        .to_string()
                };

                files.push(TorrentFile {
                    index: idx,
                    path: full_path,
                    size,
                    progress: progress.min(1.0),
                });
            }
        }

        files
    }

    pub async fn list_torrents(&self) -> Vec<TorrentInfo> {
        let ids: Vec<usize> = self
            .session
            .with_torrents(|iter| iter.map(|(id, _)| id).collect());
        let mut torrents = Vec::new();
        for id in ids {
            if let Ok(info) = self.get_torrent_info(id).await {
                torrents.push(info);
            }
        }
        torrents
    }

    pub async fn pause(&self, id: usize) -> Result<()> {
        let handle = self
            .session
            .get(TorrentIdOrHash::Id(id))
            .context("Torrent not found")?;
        self.session
            .pause(&handle)
            .await
            .context("Failed to pause torrent")?;
        debug!("Torrent {} paused", id);
        Ok(())
    }

    pub async fn resume(&self, id: usize) -> Result<()> {
        let handle = self
            .session
            .get(TorrentIdOrHash::Id(id))
            .context("Torrent not found")?;
        self.session
            .unpause(&handle)
            .await
            .context("Failed to resume torrent")?;
        debug!("Torrent {} resumed", id);
        Ok(())
    }

    pub async fn remove(&self, id: usize, delete_files: bool) -> Result<()> {
        let handle = self
            .session
            .get(TorrentIdOrHash::Id(id))
            .context("Torrent not found")?;
        let info_hash = get_info_hash_hex(&handle);

        // Delete from librqbit
        self.session
            .delete(TorrentIdOrHash::Id(id), delete_files)
            .await?;

        // Delete from database
        let repo = self.db.torrents();
        if let Err(e) = repo.delete(&info_hash).await {
            warn!(error = %e, "Failed to delete torrent from database");
        }

        let _ = self.event_tx.send(TorrentEvent::Removed {
            id,
            info_hash: info_hash.clone(),
        });
        self.completed.write().remove(&info_hash);
        info!("Torrent {} removed{}", id, if delete_files { " (files deleted)" } else { "" });
        Ok(())
    }

    fn map_state(&self, state: &librqbit::TorrentStatsState, progress: f64) -> TorrentState {
        use librqbit::TorrentStatsState;
        match state {
            TorrentStatsState::Paused => TorrentState::Paused,
            TorrentStatsState::Initializing => TorrentState::Queued,
            TorrentStatsState::Live if progress >= 1.0 => TorrentState::Seeding,
            TorrentStatsState::Live => TorrentState::Downloading,
            TorrentStatsState::Error => TorrentState::Error,
        }
    }

    fn start_progress_monitor(&self) {
        let session = self.session.clone();
        let event_tx = self.event_tx.clone();
        let completed = self.completed.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                let ids: Vec<usize> =
                    session.with_torrents(|iter| iter.map(|(id, _)| id).collect());

                for id in ids {
                    if let Some(handle) = session.get(TorrentIdOrHash::Id(id)) {
                        let stats = handle.stats();
                        let info_hash = get_info_hash_hex(&handle);
                        let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
                        let progress =
                            stats.progress_bytes as f64 / stats.total_bytes.max(1) as f64;

                        if progress >= 1.0 && !completed.read().contains(&info_hash) {
                            completed.write().insert(info_hash.clone());
                            let _ = event_tx.send(TorrentEvent::Completed {
                                id,
                                info_hash: info_hash.clone(),
                                name: name.clone(),
                            });
                        }

                        let (download_speed, upload_speed, peers) = stats
                            .live
                            .as_ref()
                            .map(|live| {
                                let dl = (live.download_speed.mbps * 125000.0) as u64;
                                let ul = (live.upload_speed.mbps * 125000.0) as u64;
                                (dl, ul, live.snapshot.peer_stats.live)
                            })
                            .unwrap_or((0, 0, 0));

                        let state = {
                            use librqbit::TorrentStatsState;
                            match &stats.state {
                                TorrentStatsState::Paused => TorrentState::Paused,
                                TorrentStatsState::Error => TorrentState::Error,
                                TorrentStatsState::Live if progress >= 1.0 => TorrentState::Seeding,
                                TorrentStatsState::Live => TorrentState::Downloading,
                                TorrentStatsState::Initializing => TorrentState::Queued,
                            }
                        };

                        let _ = event_tx.send(TorrentEvent::Progress {
                            id,
                            info_hash,
                            progress,
                            download_speed,
                            upload_speed,
                            peers,
                            state,
                        });
                    }
                }
            }
        });
    }

    pub fn session(&self) -> &Arc<Session> {
        &self.session
    }
    pub fn download_dir(&self) -> &PathBuf {
        &self.config.download_dir
    }

    /// Start background task to sync torrent state to database periodically
    fn start_db_sync(&self) {
        let session = self.session.clone();
        let db = self.db.clone();
        let completed = self.completed.clone();
        let download_dir = self.config.download_dir.clone();

        tokio::spawn(async move {
            // Sync to database every 10 seconds (less frequent than progress updates)
            let mut interval = tokio::time::interval(Duration::from_secs(10));

            // Get fallback user_id once for the sync loop
            let fallback_user_id = {
                let repo = db.torrents();
                repo.get_default_user_id().await.ok().flatten()
            };

            loop {
                interval.tick().await;

                let torrents: Vec<(usize, Arc<librqbit::ManagedTorrent>)> =
                    session.with_torrents(|iter| iter.map(|(id, h)| (id, h.clone())).collect());
                let repo = db.torrents();

                for (_id, handle) in torrents {
                    let stats = handle.stats();
                    let info_hash = get_info_hash_hex(&handle);
                    let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
                    let progress = stats.progress_bytes as f64 / stats.total_bytes.max(1) as f64;

                    let state = {
                        use librqbit::TorrentStatsState;
                        match &stats.state {
                            TorrentStatsState::Paused => "paused",
                            TorrentStatsState::Error => "error",
                            TorrentStatsState::Live if progress >= 1.0 => "seeding",
                            TorrentStatsState::Live => "downloading",
                            TorrentStatsState::Initializing => "queued",
                        }
                    };

                    // Use upsert to handle torrents that might not be in the database yet
                    if let Some(uid) = fallback_user_id {
                        if let Err(e) = repo
                            .upsert_from_session(
                                &info_hash,
                                &name,
                                state,
                                progress,
                                stats.total_bytes as i64,
                                stats.progress_bytes as i64,
                                stats.uploaded_bytes as i64,
                                &download_dir.to_string_lossy(),
                                uid,
                            )
                            .await
                        {
                            warn!(error = %e, info_hash = %info_hash, "Failed to sync torrent to database");
                        }
                    } else {
                        // No fallback user, just try to update existing records
                        if let Err(e) = repo
                            .update_progress(
                                &info_hash,
                                state,
                                progress,
                                stats.progress_bytes as i64,
                                stats.uploaded_bytes as i64,
                            )
                            .await
                        {
                            // This is expected if the torrent isn't in the database
                            tracing::trace!(error = %e, info_hash = %info_hash, "Failed to update torrent progress");
                        }
                    }

                    // Mark completed in database
                    if progress >= 1.0
                        && !completed.read().contains(&info_hash)
                        && let Err(e) = repo.mark_completed(&info_hash).await
                    {
                        warn!(error = %e, "Failed to mark torrent as completed in database");
                    }
                }
            }
        });
    }
}
