//! Native torrent client service using librqbit.
//!
//! Implements [Service](crate::services::manager::Service) and depends on the database service.
//! Provides the librqbit session, progress monitor, and DB sync loops with graceful shutdown.

mod db_helpers;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use librqbit::api::TorrentIdOrHash;
use librqbit::dht::PersistentDhtConfig;
use librqbit::{AddTorrent, AddTorrentOptions, AddTorrentResponse, Session, SessionOptions};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::services::manager::{Service, ServiceHealth};

use self::db_helpers::*;

fn add_torrent_opts() -> AddTorrentOptions {
    AddTorrentOptions {
        overwrite: true,
        ..Default::default()
    }
}

fn get_info_hash_hex<T: AsRef<librqbit::ManagedTorrent>>(handle: &T) -> String {
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

/// Port test result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PortTestResult {
    pub success: bool,
    pub port_open: bool,
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

/// Detailed peer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PeerStats {
    pub queued: usize,
    pub connecting: usize,
    pub live: usize,
    pub seen: usize,
    pub dead: usize,
    pub not_needed: usize,
}

/// Detailed torrent information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
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
    pub piece_count: u64,
    pub pieces_downloaded: u64,
    pub average_piece_download_ms: Option<u64>,
    pub time_remaining_secs: Option<u64>,
    pub peer_stats: PeerStats,
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

// -----------------------------------------------------------------------------
// Runtime (session + background tasks with cancellation)
// -----------------------------------------------------------------------------

struct TorrentRuntime {
    session: Arc<Session>,
    config: TorrentServiceConfig,
    db: Database,
    event_tx: broadcast::Sender<TorrentEvent>,
    completed: Arc<RwLock<std::collections::HashSet<String>>>,
    upnp_result: Arc<RwLock<Option<UpnpResult>>>,
    cancel_token: CancellationToken,
    progress_handle: tokio::task::JoinHandle<()>,
    db_sync_handle: tokio::task::JoinHandle<()>,
}

impl TorrentRuntime {
    async fn shutdown(self) {
        self.cancel_token.cancel();
        let _ = self.progress_handle.await;
        let _ = self.db_sync_handle.await;
        // session dropped here
    }
}

// -----------------------------------------------------------------------------
// TorrentService (Service impl + optional runtime)
// -----------------------------------------------------------------------------

/// Native torrent client service: depends on database, manages librqbit session and background loops.
pub struct TorrentService {
    manager: Arc<crate::services::manager::ServicesManager>,
    config: TorrentServiceConfig,
    inner: tokio::sync::RwLock<Option<Arc<TorrentRuntime>>>,
}

impl TorrentService {
    /// Create the service. It will not connect or start until [Service::start] is called.
    pub fn new(
        manager: Arc<crate::services::manager::ServicesManager>,
        config: TorrentServiceConfig,
    ) -> Self {
        Self {
            manager,
            config,
            inner: tokio::sync::RwLock::new(None),
        }
    }

    /// Return the current runtime if the service is started.
    async fn runtime(&self) -> Option<Arc<TorrentRuntime>> {
        self.inner.read().await.clone()
    }

    /// Subscribe to torrent events (for GraphQL subscriptions).
    pub fn subscribe(&self) -> Option<broadcast::Receiver<TorrentEvent>> {
        // We need sync access to inner; we use try_read and return None if not started
        self.inner.try_read().ok().and_then(|g| g.as_ref().map(|r| r.event_tx.subscribe()))
    }

    /// Add a torrent from a magnet link.
    pub async fn add_magnet(&self, magnet: &str, user_id: Option<Uuid>) -> Result<TorrentInfo> {
        let r = self
            .runtime()
            .await
            .ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
        let session = &r.session;
        let config = &r.config;
        let db = &r.db;
        let event_tx = &r.event_tx;

        let add_result = session
            .add_torrent(AddTorrent::from_url(magnet), Some(add_torrent_opts()))
            .await
            .context("Failed to add torrent")?;

        match add_result {
            AddTorrentResponse::Added(id, handle) => {
                let info_hash = get_info_hash_hex(&handle);
                let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
                let stats = handle.stats();

                if let Some(uid) = user_id {
                    if let Err(e) =
                        db_helpers::create_torrent(db, uid, &info_hash, Some(magnet), &name, &config.download_dir.to_string_lossy(), stats.total_bytes as i64).await
                    {
                        error!(error = %e, "Failed to persist torrent to database");
                    }
                }

                let _ = event_tx.send(TorrentEvent::Added {
                    id,
                    name: name.clone(),
                    info_hash: info_hash.clone(),
                });
                get_torrent_info_impl(session, config, id).await
            }
            AddTorrentResponse::AlreadyManaged(id, _) => get_torrent_info_impl(session, config, id).await,
            AddTorrentResponse::ListOnly(_) => anyhow::bail!("Torrent was added in list-only mode"),
        }
    }

    /// List all torrents.
    pub async fn list_torrents(&self) -> Result<Vec<TorrentInfo>> {
        let r = self
            .runtime()
            .await
            .ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
        let ids: Vec<usize> =
            r.session
                .with_torrents(|iter| iter.map(|(id, _)| id).collect());
        let mut out = Vec::new();
        for id in ids {
            if let Ok(info) = get_torrent_info_impl(&r.session, &r.config, id).await {
                out.push(info);
            }
        }
        Ok(out)
    }

    /// List only active (downloading/checking) torrents.
    pub async fn list_active_downloads(&self) -> Result<Vec<TorrentInfo>> {
        let all = self.list_torrents().await?;
        Ok(all
            .into_iter()
            .filter(|t| matches!(t.state, TorrentState::Downloading | TorrentState::Checking))
            .collect())
    }

    /// Get a single torrent by numeric id.
    pub async fn get_torrent_info(&self, id: usize) -> Result<TorrentInfo> {
        let r = self
            .runtime()
            .await
            .ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
        get_torrent_info_impl(&r.session, &r.config, id).await
    }

    /// Pause a torrent.
    pub async fn pause(&self, id: usize) -> Result<()> {
        let r = self
            .runtime()
            .await
            .ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
        let handle = r
            .session
            .get(TorrentIdOrHash::Id(id))
            .context("Torrent not found")?;
        r.session.pause(&handle).await.context("Failed to pause torrent")?;
        Ok(())
    }

    /// Resume a paused torrent.
    pub async fn resume(&self, id: usize) -> Result<()> {
        let r = self
            .runtime()
            .await
            .ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
        let handle = r
            .session
            .get(TorrentIdOrHash::Id(id))
            .context("Torrent not found")?;
        r.session.unpause(&handle).await.context("Failed to resume torrent")?;
        Ok(())
    }

    /// Remove a torrent (optionally delete files).
    pub async fn remove(&self, id: usize, delete_files: bool) -> Result<()> {
        let r = self
            .runtime()
            .await
            .ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
        let handle = r
            .session
            .get(TorrentIdOrHash::Id(id))
            .context("Torrent not found")?;
        let info_hash = get_info_hash_hex(&handle);
        r.session
            .delete(TorrentIdOrHash::Id(id), delete_files)
            .await?;
        if let Err(e) = db_helpers::delete_torrent(&r.db, &info_hash).await {
            warn!(error = %e, "Failed to delete torrent from database");
        }
        let _ = r.event_tx.send(TorrentEvent::Removed {
            id,
            info_hash: info_hash.clone(),
        });
        r.completed.write().remove(&info_hash);
        Ok(())
    }

    /// Pause a torrent by info hash (hex string).
    pub async fn pause_by_info_hash(&self, info_hash: &str) -> Result<()> {
        let r = self.runtime().await.ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
        let id = r
            .session
            .with_torrents(|iter| {
                for (id, handle) in iter {
                    if get_info_hash_hex(handle) == info_hash {
                        return Some(id);
                    }
                }
                None
            })
            .context("Torrent not found")?;
        self.pause(id).await
    }

    /// Resume a torrent by info hash (hex string).
    pub async fn resume_by_info_hash(&self, info_hash: &str) -> Result<()> {
        let r = self.runtime().await.ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
        let id = r
            .session
            .with_torrents(|iter| {
                for (id, handle) in iter {
                    if get_info_hash_hex(handle) == info_hash {
                        return Some(id);
                    }
                }
                None
            })
            .context("Torrent not found")?;
        self.resume(id).await
    }

    /// Remove a torrent by info hash (hex string), optionally deleting files.
    pub async fn remove_by_info_hash(&self, info_hash: &str, delete_files: bool) -> Result<()> {
        let r = self.runtime().await.ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
        let id = r
            .session
            .with_torrents(|iter| {
                for (id, handle) in iter {
                    if get_info_hash_hex(handle) == info_hash {
                        return Some(id);
                    }
                }
                None
            })
            .context("Torrent not found")?;
        self.remove(id, delete_files).await
    }
}

#[async_trait]
impl Service for TorrentService {
    fn name(&self) -> &str {
        "torrent"
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["database".to_string()]
    }

    async fn start(&self) -> Result<()> {
        info!(service = "torrent", "Torrent service starting");

        let db_svc = self
            .manager
            .get_database()
            .await
            .ok_or_else(|| anyhow::anyhow!("database service not started"))?;
        let db = db_svc.pool().clone();

        let mut config = self.config.clone();

        // Load settings from app_settings (paths as raw strings, others as JSON)
        if let Ok(Some(dir)) = get_setting_string(&db, "torrent.download_dir").await {
            config.download_dir = PathBuf::from(dir);
        }
        if let Ok(Some(dir)) = get_setting_string(&db, "torrent.session_dir").await {
            config.session_dir = PathBuf::from(dir);
        }
        if let Ok(Some(v)) = get_setting::<bool>(&db, "torrent.enable_dht").await {
            config.enable_dht = v;
        }
        if let Ok(Some(v)) = get_setting::<u16>(&db, "torrent.listen_port").await {
            config.listen_port = v;
        }
        if let Ok(Some(v)) = get_setting::<usize>(&db, "torrent.max_concurrent").await {
            config.max_concurrent = v;
        }

        info!(path = %config.download_dir.display(), "Using download directory");

        let download_dir_ok = tokio::fs::create_dir_all(&config.download_dir).await.is_ok();
        let effective_download_dir = if download_dir_ok {
            config.download_dir.clone()
        } else {
            let temp = std::env::temp_dir().join("librarian-downloads");
            warn!(path = %temp.display(), "Using temp directory for downloads");
            let _ = tokio::fs::create_dir_all(&temp).await;
            config.download_dir = temp.clone();
            temp
        };

        let session_dir_ok = tokio::fs::create_dir_all(&config.session_dir).await.is_ok();
        let effective_session_dir = if session_dir_ok {
            config.session_dir.clone()
        } else {
            let temp = std::env::temp_dir().join("librarian-session");
            warn!(path = %temp.display(), "Using temp directory for session");
            let _ = tokio::fs::create_dir_all(&temp).await;
            config.session_dir = temp.clone();
            temp
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
        let completed = Arc::new(RwLock::new(std::collections::HashSet::new()));
        let upnp_result = Arc::new(RwLock::new(None));

        config.download_dir = effective_download_dir;
        config.session_dir = effective_session_dir;

        // Sync session -> DB and restore from DB (best-effort)
        if let Err(e) = sync_session_to_database(&session, &db, &config).await {
            warn!(error = %e, "Failed to sync session torrents to database");
        }
        if let Err(e) = restore_from_database(&session, &db).await {
            warn!(error = %e, "Failed to restore torrents from database");
        }

        let cancel_token = CancellationToken::new();
        let token_clone = cancel_token.clone();

        let session_p = session.clone();
        let event_tx_p = event_tx.clone();
        let completed_p = completed.clone();
        let progress_handle = tokio::spawn(async move {
            progress_loop(session_p, event_tx_p, completed_p, token_clone).await;
        });

        let token_db = cancel_token.clone();
        let session_db = session.clone();
        let db_clone = db.clone();
        let completed_db = completed.clone();
        let download_dir_db = config.download_dir.clone();
        let db_sync_handle = tokio::spawn(async move {
            db_sync_loop(
                session_db,
                db_clone,
                completed_db,
                download_dir_db,
                token_db,
            )
            .await;
        });

        // UPnP in background (fire-and-forget)
        {
            let upnp = upnp_result.clone();
            let port = config.listen_port;
            tokio::spawn(async move {
                let result = if port == 0 {
                    UpnpResult {
                        success: false,
                        tcp_forwarded: false,
                        udp_forwarded: false,
                        local_ip: None,
                        external_ip: None,
                        error: Some("Listen port is set to 0 (random)".to_string()),
                    }
                } else {
                    tokio::task::spawn_blocking(move || perform_upnp(port))
                        .await
                        .unwrap_or_else(|_| UpnpResult {
                            success: false,
                            tcp_forwarded: false,
                            udp_forwarded: false,
                            local_ip: None,
                            external_ip: None,
                            error: Some("UPnP task panicked".to_string()),
                        })
                };
                *upnp.write() = Some(result);
            });
        }

        let runtime = Arc::new(TorrentRuntime {
            session,
            config,
            db,
            event_tx,
            completed,
            upnp_result,
            cancel_token,
            progress_handle,
            db_sync_handle,
        });

        *self.inner.write().await = Some(runtime);
        info!(service = "torrent", "Torrent service started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        info!(service = "torrent", "Torrent service stopping");
        let previous = self.inner.write().await.take();
        if let Some(arc_r) = previous {
            arc_r.cancel_token.cancel();
            if let Ok(runtime) = Arc::try_unwrap(arc_r) {
                runtime.shutdown().await;
            }
        }
        info!(service = "torrent", "Torrent service stopped");
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        if self.inner.read().await.is_some() {
            Ok(ServiceHealth::healthy())
        } else {
            Ok(ServiceHealth::degraded("torrent service not started"))
        }
    }
}

// -----------------------------------------------------------------------------
// Background loops (with cancellation)
// -----------------------------------------------------------------------------

async fn progress_loop(
    session: Arc<Session>,
    event_tx: broadcast::Sender<TorrentEvent>,
    completed: Arc<RwLock<std::collections::HashSet<String>>>,
    cancel: CancellationToken,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
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
        }
    }
}

async fn db_sync_loop(
    session: Arc<Session>,
    db: Database,
    completed: Arc<RwLock<std::collections::HashSet<String>>>,
    download_dir: PathBuf,
    cancel: CancellationToken,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    let fallback_user_id = get_default_user_id(&db).await.ok().flatten();
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
                if let Err(e) =
                    sync_session_to_database(&session, &db, &TorrentServiceConfig { download_dir: download_dir.clone(), ..Default::default() }).await
                {
                    tracing::trace!(error = %e, "db_sync iteration failed");
                }
            }
        }
    }
    let _ = fallback_user_id;
    let _ = completed;
}

// -----------------------------------------------------------------------------
// Helpers used by runtime and public API
// -----------------------------------------------------------------------------

async fn get_torrent_info_impl(
    session: &Session,
    config: &TorrentServiceConfig,
    id: usize,
) -> Result<TorrentInfo> {
    use librqbit::TorrentStatsState;

    let handle = session
        .get(TorrentIdOrHash::Id(id))
        .context("Torrent not found")?;
    let stats = handle.stats();
    let info_hash = get_info_hash_hex(&handle);
    let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
    let progress = stats.progress_bytes as f64 / stats.total_bytes.max(1) as f64;
    let state = match &stats.state {
        TorrentStatsState::Paused => TorrentState::Paused,
        TorrentStatsState::Error => TorrentState::Error,
        TorrentStatsState::Live if progress >= 1.0 => TorrentState::Seeding,
        TorrentStatsState::Live => TorrentState::Downloading,
        TorrentStatsState::Initializing => TorrentState::Queued,
    };
    let (download_speed, upload_speed, peers) = stats
        .live
        .as_ref()
        .map(|live| {
            let dl = (live.download_speed.mbps * 125000.0) as u64;
            let ul = (live.upload_speed.mbps * 125000.0) as u64;
            (dl, ul, live.snapshot.peer_stats.live)
        })
        .unwrap_or((0, 0, 0));

    let files = get_torrent_files_list(&handle, config);
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
        save_path: config.download_dir.to_string_lossy().to_string(),
        files,
    })
}

fn get_torrent_files_list(
    handle: &Arc<librqbit::ManagedTorrent>,
    config: &TorrentServiceConfig,
) -> Vec<TorrentFile> {
    let mut files = Vec::new();
    if let Some(metadata) = handle.metadata.load_full() {
        let stats = handle.stats();
        for (idx, file_info) in metadata.file_infos.iter().enumerate() {
            let file_progress = stats.file_progress.get(idx).copied().unwrap_or(0);
            let size = file_info.len;
            let progress = if size > 0 {
                (file_progress as f64 / size as f64).min(1.0)
            } else {
                0.0
            };
            let path = if metadata.file_infos.len() == 1 {
                config
                    .download_dir
                    .join(file_info.relative_filename.to_string_lossy().as_ref())
                    .to_string_lossy()
                    .to_string()
            } else {
                let torrent_name = handle.name().unwrap_or_else(|| "unknown".to_string());
                config
                    .download_dir
                    .join(&torrent_name)
                    .join(file_info.relative_filename.to_string_lossy().as_ref())
                    .to_string_lossy()
                    .to_string()
            };
            files.push(TorrentFile {
                index: idx,
                path,
                size,
                progress,
            });
        }
    }
    files
}

fn perform_upnp(port: u16) -> UpnpResult {
    use igd::{search_gateway, PortMappingProtocol, SearchOptions};
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
            }
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
            }
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
            }
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
            }
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
