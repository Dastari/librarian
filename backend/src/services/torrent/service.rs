//! Torrent service implementation.
//!
//! Implements [Service](crate::services::manager::Service) and depends on the database service.
//! Provides the librqbit session, progress monitor, and DB sync loops with graceful shutdown.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use librqbit::api::TorrentIdOrHash;
use librqbit::dht::PersistentDhtConfig;
use librqbit::{AddTorrent, AddTorrentOptions, AddTorrentResponse, Session, SessionOptions};
use parking_lot::RwLock;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::services::manager::{Service, ServiceHealth};
use crate::services::torrent::client::{add_torrent_opts, get_info_hash_hex, perform_upnp, TorrentEvent, TorrentFile, TorrentInfo, TorrentServiceConfig, TorrentState, UpnpResult};
use crate::services::torrent::database;

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
                        database::create_torrent(db, uid, &info_hash, Some(magnet), &name, &config.download_dir.to_string_lossy(), stats.total_bytes as i64).await
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
        if let Err(e) = database::delete_torrent(&r.db, &info_hash).await {
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
        if let Ok(Some(dir)) = database::get_setting_string(&db, "torrent.download_dir").await {
            config.download_dir = PathBuf::from(dir);
        }
        if let Ok(Some(dir)) = database::get_setting_string(&db, "torrent.session_dir").await {
            config.session_dir = PathBuf::from(dir);
        }
        if let Ok(Some(v)) = database::get_setting::<bool>(&db, "torrent.enable_dht").await {
            config.enable_dht = v;
        }
        if let Ok(Some(v)) = database::get_setting::<u16>(&db, "torrent.listen_port").await {
            config.listen_port = v;
        }
        if let Ok(Some(v)) = database::get_setting::<usize>(&db, "torrent.max_concurrent").await {
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
        if let Err(e) = database::sync_session_to_database(&session, &db, &config).await {
            warn!(error = %e, "Failed to sync session torrents to database");
        }
        if let Err(e) = database::restore_from_database(&session, &db).await {
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
                } as UpnpResult;
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
    let fallback_user_id = database::get_default_user_id(&db).await.ok().flatten();
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
                if let Err(e) =
                    database::sync_session_to_database(&session, &db, &TorrentServiceConfig { download_dir: download_dir.clone(), ..Default::default() }).await
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
