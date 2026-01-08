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
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::{CreateTorrent, Database, TorrentRepository};

/// Helper to get info hash as hex string from handle
fn get_info_hash_hex<T: AsRef<librqbit::ManagedTorrent>>(handle: &T) -> String {
    // The info_hash() returns Id<20> which has a .0 field with [u8; 20]
    handle.as_ref().info_hash().0.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Events broadcast when torrent state changes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TorrentEvent {
    Added { id: usize, name: String, info_hash: String },
    Progress { id: usize, info_hash: String, progress: f64, download_speed: u64, upload_speed: u64, peers: usize, state: TorrentState },
    Completed { id: usize, info_hash: String, name: String },
    Removed { id: usize, info_hash: String },
    Error { id: usize, info_hash: String, message: String },
}

/// Simplified torrent state for API
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TorrentState {
    Queued, Checking, Downloading, Seeding, Paused, Error,
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

        info!(download_dir = %config.download_dir.display(), "Using download directory from settings");

        // Try to create directories, but don't fail if we can't
        let download_dir_ok = match tokio::fs::create_dir_all(&config.download_dir).await {
            Ok(_) => {
                info!(path = %config.download_dir.display(), "Download directory ready");
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
                info!(path = %config.session_dir.display(), "Session directory ready");
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

        // Restore torrents from database
        if let Err(e) = service.restore_from_database().await {
            warn!(error = %e, "Failed to restore torrents from database");
        }

        // Start background monitors
        service.start_progress_monitor();
        service.start_db_sync();

        info!("Torrent service initialized");
        Ok(service)
    }

    /// Restore torrents from database on startup
    async fn restore_from_database(&self) -> Result<()> {
        let repo = self.db.torrents();
        let records = repo.list_resumable().await?;

        info!(count = records.len(), "Restoring torrents from database");

        for record in records {
            if let Some(magnet) = &record.magnet_uri {
                match self.session
                    .add_torrent(AddTorrent::from_url(magnet), Some(AddTorrentOptions::default()))
                    .await
                {
                    Ok(_) => {
                        info!(info_hash = %record.info_hash, name = %record.name, "Restored torrent");
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

    pub fn subscribe(&self) -> broadcast::Receiver<TorrentEvent> {
        self.event_tx.subscribe()
    }

    pub fn event_sender(&self) -> broadcast::Sender<TorrentEvent> {
        self.event_tx.clone()
    }

    /// Add a torrent from a magnet link, persisting to database
    pub async fn add_magnet(&self, magnet: &str, user_id: Option<Uuid>) -> Result<TorrentInfo> {
        info!(magnet = %magnet, "Adding torrent from magnet");

        let add_result = self.session
            .add_torrent(AddTorrent::from_url(magnet), Some(AddTorrentOptions::default()))
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
                    if let Err(e) = repo.create(CreateTorrent {
                        user_id: uid,
                        info_hash: info_hash.clone(),
                        magnet_uri: Some(magnet.to_string()),
                        name: name.clone(),
                        save_path: self.config.download_dir.to_string_lossy().to_string(),
                        total_bytes: stats.total_bytes as i64,
                    }).await {
                        error!(error = %e, "Failed to persist torrent to database");
                    }
                }

                let _ = self.event_tx.send(TorrentEvent::Added { id: id.into(), name, info_hash });
                self.get_torrent_info(id.into()).await
            }
            AddTorrentResponse::AlreadyManaged(id, _) => {
                warn!(id = %id, "Torrent already exists");
                self.get_torrent_info(id.into()).await
            }
            AddTorrentResponse::ListOnly(_) => anyhow::bail!("Torrent was added in list-only mode"),
        }
    }

    /// Add a torrent without user context (for internal use)
    pub async fn add_magnet_internal(&self, magnet: &str) -> Result<TorrentInfo> {
        self.add_magnet(magnet, None).await
    }

    /// Add a torrent from .torrent file data with database persistence
    pub async fn add_torrent_file(&self, data: Vec<u8>, user_id: Option<Uuid>) -> Result<TorrentInfo> {
        info!("Adding torrent from file data");

        let add_result = self.session
            .add_torrent(AddTorrent::from_bytes(data), Some(AddTorrentOptions::default()))
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
                    if let Err(e) = repo.create(CreateTorrent {
                        user_id: uid,
                        info_hash: info_hash.clone(),
                        magnet_uri: None, // File uploads don't have a magnet URI
                        name: name.clone(),
                        save_path: self.config.download_dir.to_string_lossy().to_string(),
                        total_bytes: stats.total_bytes as i64,
                    }).await {
                        error!(error = %e, "Failed to persist torrent to database");
                    }
                }

                let _ = self.event_tx.send(TorrentEvent::Added { id: id.into(), name, info_hash });
                self.get_torrent_info(id.into()).await
            }
            AddTorrentResponse::AlreadyManaged(id, _) => self.get_torrent_info(id.into()).await,
            AddTorrentResponse::ListOnly(_) => anyhow::bail!("Torrent was added in list-only mode"),
        }
    }

    /// Add a torrent from a URL to a .torrent file
    pub async fn add_torrent_url(&self, url: &str, user_id: Option<Uuid>) -> Result<TorrentInfo> {
        info!(url = %url, "Adding torrent from URL");

        // librqbit handles both magnet links and http URLs
        let add_result = self.session
            .add_torrent(AddTorrent::from_url(url), Some(AddTorrentOptions::default()))
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
                    if let Err(e) = repo.create(CreateTorrent {
                        user_id: uid,
                        info_hash: info_hash.clone(),
                        magnet_uri: Some(url.to_string()),
                        name: name.clone(),
                        save_path: self.config.download_dir.to_string_lossy().to_string(),
                        total_bytes: stats.total_bytes as i64,
                    }).await {
                        error!(error = %e, "Failed to persist torrent to database");
                    }
                }

                let _ = self.event_tx.send(TorrentEvent::Added { id: id.into(), name, info_hash });
                self.get_torrent_info(id.into()).await
            }
            AddTorrentResponse::AlreadyManaged(id, _) => {
                warn!(id = %id, "Torrent already exists");
                self.get_torrent_info(id.into()).await
            }
            AddTorrentResponse::ListOnly(_) => anyhow::bail!("Torrent was added in list-only mode"),
        }
    }

    pub async fn get_torrent_info(&self, id: usize) -> Result<TorrentInfo> {
        let handle = self.session.get(TorrentIdOrHash::Id(id.into())).context("Torrent not found")?;
        let stats = handle.stats();
        let info_hash = get_info_hash_hex(&handle);
        let name = handle.name().unwrap_or_else(|| "Unknown".to_string());

        let progress = stats.progress_bytes as f64 / stats.total_bytes.max(1) as f64;
        let state = self.map_state(&stats.state, progress);

        let (download_speed, upload_speed, peers) = stats.live.as_ref()
            .map(|live| {
                let dl = (live.download_speed.mbps * 125000.0) as u64;
                let ul = (live.upload_speed.mbps * 125000.0) as u64;
                let p = live.snapshot.peer_stats.live;
                (dl, ul, p)
            })
            .unwrap_or((0, 0, 0));

        Ok(TorrentInfo {
            id, info_hash, name, state, progress,
            size: stats.total_bytes, downloaded: stats.progress_bytes, uploaded: stats.uploaded_bytes,
            download_speed, upload_speed, peers, seeds: 0,
            save_path: self.config.download_dir.to_string_lossy().to_string(),
            files: vec![],
        })
    }

    pub async fn list_torrents(&self) -> Vec<TorrentInfo> {
        let ids: Vec<usize> = self.session.with_torrents(|iter| iter.map(|(id, _)| id).collect());
        let mut torrents = Vec::new();
        for id in ids {
            if let Ok(info) = self.get_torrent_info(id).await {
                torrents.push(info);
            }
        }
        torrents
    }

    pub async fn pause(&self, id: usize) -> Result<()> {
        let handle = self.session.get(TorrentIdOrHash::Id(id.into())).context("Torrent not found")?;
        self.session.pause(&handle).await.context("Failed to pause torrent")?;
        info!(id = %id, "Torrent paused");
        Ok(())
    }

    pub async fn resume(&self, id: usize) -> Result<()> {
        let handle = self.session.get(TorrentIdOrHash::Id(id.into())).context("Torrent not found")?;
        self.session.unpause(&handle).await.context("Failed to resume torrent")?;
        info!(id = %id, "Torrent resumed");
        Ok(())
    }

    pub async fn remove(&self, id: usize, delete_files: bool) -> Result<()> {
        let handle = self.session.get(TorrentIdOrHash::Id(id.into())).context("Torrent not found")?;
        let info_hash = get_info_hash_hex(&handle);
        
        // Delete from librqbit
        self.session.delete(TorrentIdOrHash::Id(id.into()), delete_files).await?;
        
        // Delete from database
        let repo = self.db.torrents();
        if let Err(e) = repo.delete(&info_hash).await {
            warn!(error = %e, "Failed to delete torrent from database");
        }
        
        let _ = self.event_tx.send(TorrentEvent::Removed { id, info_hash: info_hash.clone() });
        self.completed.write().remove(&info_hash);
        info!(id = %id, delete_files = %delete_files, "Torrent removed");
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
                let ids: Vec<usize> = session.with_torrents(|iter| iter.map(|(id, _)| id).collect());

                for id in ids {
                    if let Some(handle) = session.get(TorrentIdOrHash::Id(id.into())) {
                        let stats = handle.stats();
                        let info_hash = get_info_hash_hex(&handle);
                        let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
                        let progress = stats.progress_bytes as f64 / stats.total_bytes.max(1) as f64;

                        if progress >= 1.0 && !completed.read().contains(&info_hash) {
                            completed.write().insert(info_hash.clone());
                            let _ = event_tx.send(TorrentEvent::Completed { id, info_hash: info_hash.clone(), name: name.clone() });
                        }

                        let (download_speed, upload_speed, peers) = stats.live.as_ref()
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

                        let _ = event_tx.send(TorrentEvent::Progress { id, info_hash, progress, download_speed, upload_speed, peers, state });
                    }
                }
            }
        });
    }

    pub fn session(&self) -> &Arc<Session> { &self.session }
    pub fn download_dir(&self) -> &PathBuf { &self.config.download_dir }

    /// Start background task to sync torrent state to database periodically
    fn start_db_sync(&self) {
        let session = self.session.clone();
        let db = self.db.clone();
        let completed = self.completed.clone();

        tokio::spawn(async move {
            // Sync to database every 10 seconds (less frequent than progress updates)
            let mut interval = tokio::time::interval(Duration::from_secs(10));

            loop {
                interval.tick().await;

                let ids: Vec<usize> = session.with_torrents(|iter| iter.map(|(id, _)| id).collect());
                let repo = db.torrents();

                for id in ids {
                    if let Some(handle) = session.get(TorrentIdOrHash::Id(id.into())) {
                        let stats = handle.stats();
                        let info_hash = get_info_hash_hex(&handle);
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

                        // Update database
                        if let Err(e) = repo.update_progress(
                            &info_hash,
                            state,
                            progress,
                            stats.progress_bytes as i64,
                            stats.uploaded_bytes as i64,
                        ).await {
                            warn!(error = %e, info_hash = %info_hash, "Failed to sync torrent to database");
                        }

                        // Mark completed in database
                        if progress >= 1.0 && !completed.read().contains(&info_hash) {
                            if let Err(e) = repo.mark_completed(&info_hash).await {
                                warn!(error = %e, "Failed to mark torrent as completed in database");
                            }
                        }
                    }
                }
            }
        });
    }
}
