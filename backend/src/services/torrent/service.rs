//! Torrent service implementation.
//!
//! Implements [Service](crate::services::manager::Service) and depends on the database service.
//! Provides the librqbit session, progress monitor, and DB sync loops with graceful shutdown.

use std::collections::VecDeque;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use librqbit::api::TorrentIdOrHash;
use librqbit::dht::DhtPersistenceConfig;
use librqbit::limits::LimitsConfig;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, DhtSessionConfig, ListenerOptions, Session,
    SessionOptions,
};
use parking_lot::{Mutex, RwLock};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::graphql::entities::Torrent;
use crate::services::graphql::AuthUser;
use crate::services::graphql::LibrarianSchema;
use crate::services::library_scan::{LibraryScanService, SourceFileImport, SourceProcessSummary};
use crate::services::manager::{Service, ServiceHealth};
use crate::services::torrent::client::{
    TorrentEvent, TorrentFile, TorrentInfo, TorrentServiceConfig, TorrentState, UpnpResult,
    add_torrent_opts, get_info_hash_hex, perform_upnp, progress_ratio, torrent_state_from_stats,
};
use crate::services::torrent::database;
use crate::services::torrent::seeding::{SeedingAction, SeedingFacts, decide_seeding_action};

/// Upper bound on a single `add_torrent` call.
///
/// A magnet has no metadata of its own: librqbit blocks in `resolve_magnet`
/// until a peer sends the info dictionary, with no timeout of its own. Without
/// this bound the `addTorrent` mutation hangs forever on a dead magnet, and —
/// worse — a single unresolvable magnet restored at startup blocked
/// `TorrentService::start`, which `ServicesManager::start_all` awaits in
/// sequence, so the whole backend never finished booting.
const ADD_TORRENT_TIMEOUT: Duration = Duration::from_secs(120);

/// How often the queue/seeding maintenance loop runs.
const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(15);

// -----------------------------------------------------------------------------
// Runtime (session + background tasks with cancellation)
// -----------------------------------------------------------------------------

struct TorrentRuntime {
    session: Arc<Session>,
    config: TorrentServiceConfig,
    db: Database,
    schema: LibrarianSchema,
    event_tx: broadcast::Sender<TorrentEvent>,
    completed: Arc<RwLock<std::collections::HashSet<String>>>,
    /// Info hashes currently undergoing completed-file post-processing. Shared
    /// with the completion handler and the reconciliation sweep so neither runs
    /// twice for the same torrent, and so [`TorrentService::remove`] can refuse
    /// to delete files out from under an in-flight import.
    processing: Arc<Mutex<std::collections::HashSet<String>>>,
    /// FIFO of torrents that were started paused because
    /// `torrent.max_concurrent` was already reached. The maintenance loop
    /// unpauses them as download slots free up; without this they stayed
    /// paused forever.
    download_queue: Arc<Mutex<VecDeque<String>>>,
    /// Set when the configured download directory is unusable. Adds are
    /// refused (rather than silently written somewhere else) and `health`
    /// reports degraded until it is fixed.
    download_dir_error: Option<String>,
    cancel_token: CancellationToken,
    progress_handle: tokio::task::JoinHandle<()>,
    db_sync_handle: tokio::task::JoinHandle<()>,
    completion_handle: tokio::task::JoinHandle<()>,
    reconcile_handle: tokio::task::JoinHandle<()>,
    maintenance_handle: tokio::task::JoinHandle<()>,
    upnp_handle: tokio::task::JoinHandle<()>,
}

impl TorrentRuntime {
    async fn shutdown(self) {
        self.cancel_token.cancel();
        let _ = self.progress_handle.await;
        let _ = self.db_sync_handle.await;
        let _ = self.completion_handle.await;
        let _ = self.reconcile_handle.await;
        let _ = self.maintenance_handle.await;
        let _ = self.upnp_handle.await;
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

#[derive(Debug, Clone, Default)]
pub struct TorrentAddMetadata {
    pub library_id: Option<String>,
    pub source_url: Option<String>,
    pub source_indexer_id: Option<String>,
    pub source_feed_id: Option<String>,
    /// Wanted-target linkage stamped on the created `Torrent` row so the
    /// "downloading" status can be computed before completion. Set by
    /// `jobs::auto_download` at grab time; empty for manual/RSS adds.
    /// See `docs/tier1-features-plan.md` §1 and `Torrent`'s wanted-linkage columns.
    pub episode_id: Option<String>,
    pub movie_id: Option<String>,
    pub track_id: Option<String>,
    pub chapter_id: Option<String>,
    pub show_id: Option<String>,
    pub album_id: Option<String>,
    pub audiobook_id: Option<String>,
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
    ///
    /// Awaits the runtime lock instead of `try_read`: the previous version
    /// returned `None` — which the subscription resolvers turn into a silently
    /// empty stream — whenever the lock happened to be held by a concurrent
    /// `start`/`stop`, so a client could connect and receive nothing forever.
    pub async fn subscribe(&self) -> Option<broadcast::Receiver<TorrentEvent>> {
        self.inner
            .read()
            .await
            .as_ref()
            .map(|r| r.event_tx.subscribe())
    }

    /// Add a torrent from a magnet link.
    pub async fn add_magnet(&self, magnet: &str, user_id: Option<Uuid>) -> Result<TorrentInfo> {
        self.add_magnet_with_metadata(magnet, user_id, TorrentAddMetadata::default())
            .await
    }

    pub async fn add_magnet_with_metadata(
        &self,
        magnet: &str,
        user_id: Option<Uuid>,
        metadata: TorrentAddMetadata,
    ) -> Result<TorrentInfo> {
        let r = self
            .runtime()
            .await
            .ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
        let session = &r.session;
        let config = &r.config;
        let schema = &r.schema;
        let event_tx = &r.event_tx;

        // Refuse rather than write the payload somewhere the operator never
        // configured. See `start` for how this is detected and reported.
        if let Some(ref problem) = r.download_dir_error {
            anyhow::bail!("{problem}");
        }

        let start_paused = should_start_paused(
            config.max_concurrent,
            count_active_downloads(session.as_ref()),
        );

        let add_result = tokio::time::timeout(
            ADD_TORRENT_TIMEOUT,
            session.add_torrent(
                AddTorrent::from_url(magnet),
                Some(add_torrent_opts(start_paused)),
            ),
        )
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "Timed out after {}s adding torrent '{}': metadata could not be resolved (no peers answered, or the .torrent URL did not respond). Check DHT/tracker connectivity.",
                ADD_TORRENT_TIMEOUT.as_secs(),
                magnet
            )
        })?
        .context("Failed to add torrent")?;

        match add_result {
            AddTorrentResponse::Added(id, handle) => {
                let info_hash = get_info_hash_hex(&handle);
                if start_paused {
                    r.download_queue.lock().push_back(info_hash.clone());
                    info!(
                        info_hash = %info_hash,
                        max_concurrent = config.max_concurrent,
                        "Torrent queued (concurrent download limit reached): info_hash={}, max_concurrent={}",
                        info_hash,
                        config.max_concurrent
                    );
                }
                let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
                let stats = handle.stats();

                if let Some(uid) = user_id {
                    let auth_user = AuthUser {
                        user_id: uid.to_string(),
                        email: None,
                        role: Some("admin".to_string()),
                    };
                    if let Err(e) = database::create_torrent(
                        schema,
                        &auth_user,
                        &info_hash,
                        Some(magnet),
                        metadata.library_id.as_deref(),
                        metadata.source_url.as_deref(),
                        metadata.source_indexer_id.as_deref(),
                        metadata.source_feed_id.as_deref(),
                        &name,
                        // The session's real destination for this torrent
                        // (multi-file torrents get their own sub-folder), so
                        // the row matches what the file sync will write.
                        &handle.output_folder().to_string_lossy(),
                        if start_paused {
                            "paused"
                        } else {
                            "downloading"
                        },
                        0.0,
                        stats.total_bytes as i64,
                        0,
                        0,
                        database::WantedLinkage {
                            episode_id: metadata.episode_id.as_deref(),
                            movie_id: metadata.movie_id.as_deref(),
                            track_id: metadata.track_id.as_deref(),
                            chapter_id: metadata.chapter_id.as_deref(),
                            show_id: metadata.show_id.as_deref(),
                            album_id: metadata.album_id.as_deref(),
                            audiobook_id: metadata.audiobook_id.as_deref(),
                        },
                    )
                    .await
                    {
                        error!(error = %e, "Failed to persist torrent to database");
                    }
                }

                let _ = event_tx.send(TorrentEvent::Added {
                    id,
                    name: name.clone(),
                    info_hash: info_hash.clone(),
                });
                get_torrent_info_impl(session, id).await
            }
            AddTorrentResponse::AlreadyManaged(id, _) => get_torrent_info_impl(session, id).await,
            AddTorrentResponse::ListOnly(_) => anyhow::bail!("Torrent was added in list-only mode"),
        }
    }

    /// Manually (re)run completed-file post-processing for a torrent.
    ///
    /// Shares [`process_completed_torrent_core`] with the completion-event handler
    /// and the reconciliation sweep, so all three entrypoints behave identically.
    pub async fn process_completed_torrent(
        &self,
        auth_user: &AuthUser,
        info_hash: &str,
        library_override: Option<String>,
    ) -> Result<SourceProcessSummary> {
        let r = self
            .runtime()
            .await
            .ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
        let scan_service = self
            .manager
            .get_library_scan()
            .await
            .ok_or_else(|| anyhow::anyhow!("library scan service not started"))?;

        match process_completed_torrent_core(
            &r.session,
            &r.db,
            &scan_service,
            auth_user,
            info_hash,
            library_override,
        )
        .await?
        {
            Some(summary) => Ok(summary),
            // Fixes a prior bug where a torrent missing from the active session
            // silently produced an empty "unmatched" summary with no indication
            // of *why* nothing was processed.
            None => {
                let message = format!("Torrent not found in active session: {info_hash}");
                Ok(SourceProcessSummary {
                    success: false,
                    files_processed: 0,
                    files_failed: 0,
                    messages: vec![message.clone()],
                    error: Some(message),
                })
            }
        }
    }

    /// List all torrents.
    pub async fn list_torrents(&self) -> Result<Vec<TorrentInfo>> {
        let r = self
            .runtime()
            .await
            .ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
        let ids: Vec<usize> = r
            .session
            .with_torrents(|iter| iter.map(|(id, _)| id).collect());
        let mut out = Vec::new();
        for id in ids {
            if let Ok(info) = get_torrent_info_impl(&r.session, id).await {
                out.push(info);
            }
        }
        Ok(out)
    }

    /// List only active (initializing/checking/downloading) torrents.
    ///
    /// Matches [`count_active_downloads`], which is what the concurrency limit
    /// is enforced against — previously `activeDownloadCount` excluded
    /// initializing (`Queued`) torrents that the limit *did* count, so the UI
    /// disagreed with the queue.
    pub async fn list_active_downloads(&self) -> Result<Vec<TorrentInfo>> {
        let all = self.list_torrents().await?;
        Ok(all
            .into_iter()
            .filter(|t| {
                matches!(
                    t.state,
                    TorrentState::Queued | TorrentState::Checking | TorrentState::Downloading
                )
            })
            .collect())
    }

    /// Get a single torrent by numeric id.
    pub async fn get_torrent_info(&self, id: usize) -> Result<TorrentInfo> {
        let r = self
            .runtime()
            .await
            .ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
        get_torrent_info_impl(&r.session, id).await
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
        let info_hash = get_info_hash_hex(&handle);
        r.session
            .pause(&handle)
            .await
            .context("Failed to pause torrent")?;
        // An explicit pause takes the torrent out of the start queue, otherwise
        // the maintenance loop would immediately unpause it again.
        r.download_queue.lock().retain(|hash| hash != &info_hash);
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
        let info_hash = get_info_hash_hex(&handle);
        r.session
            .unpause(&handle)
            .await
            .context("Failed to resume torrent")?;
        r.download_queue.lock().retain(|hash| hash != &info_hash);
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
        // Deleting the payload while post-processing is mid-import would race
        // the hardlink/copy into the library and leave a half-imported release.
        // Removing the torrent record only (delete_files = false) is safe.
        if delete_files && r.processing.lock().contains(&info_hash) {
            anyhow::bail!(
                "Cannot delete files for torrent {info_hash}: completed-file post-processing is still running. Retry once the import has finished, or remove without deleting files."
            );
        }
        r.session
            .delete(TorrentIdOrHash::Id(id), delete_files)
            .await?;
        r.download_queue.lock().retain(|hash| hash != &info_hash);
        let auth_user = database::get_default_user_id(&r.db)
            .await
            .ok()
            .flatten()
            .map(|uid| AuthUser {
                user_id: uid.to_string(),
                email: None,
                role: Some("admin".to_string()),
            });
        if let Some(auth_user) = auth_user
            && let Err(e) = database::delete_torrent(&r.db, &r.schema, &auth_user, &info_hash).await
        {
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
        let r = self
            .runtime()
            .await
            .ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
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
        let r = self
            .runtime()
            .await
            .ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
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
        let r = self
            .runtime()
            .await
            .ok_or_else(|| anyhow::anyhow!("torrent service not started"))?;
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
        vec![
            "database".to_string(),
            "graphql".to_string(),
            "library_scan".to_string(),
        ]
    }

    async fn start(&self) -> Result<()> {
        info!(
            service = "torrent",
            "Torrent service starting: loading settings, initializing session, and restoring torrents"
        );
        let previous = self.inner.write().await.take();
        if let Some(arc_r) = previous {
            warn!(
                service = "torrent",
                "Torrent service start requested while already running; shutting down previous runtime first"
            );
            arc_r.cancel_token.cancel();
            if let Ok(runtime) = Arc::try_unwrap(arc_r) {
                runtime.shutdown().await;
            } else {
                warn!(
                    service = "torrent",
                    "Previous torrent runtime still had active references; waiting for cancellation by drop"
                );
            }
        }

        let db_svc = self
            .manager
            .get_database()
            .await
            .ok_or_else(|| anyhow::anyhow!("database service not started"))?;
        let db = db_svc.pool().clone();
        let graphql = self
            .manager
            .get_graphql()
            .await
            .ok_or_else(|| anyhow::anyhow!("graphql service not started"))?;
        let schema = graphql
            .schema()
            .await
            .ok_or_else(|| anyhow::anyhow!("graphql schema not ready"))?;

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
        if let Ok(Some(v)) = database::get_setting::<u32>(&db, "torrent.upload_limit").await {
            config.upload_limit = v;
        }
        if let Ok(Some(v)) = database::get_setting::<u32>(&db, "torrent.download_limit").await {
            config.download_limit = v;
        }
        if let Ok(Some(v)) = database::get_setting::<f64>(&db, "torrent.seed_ratio_limit").await {
            config.seed_ratio_limit = v;
        }
        if let Ok(Some(v)) = database::get_setting::<i64>(&db, "torrent.seed_time_minutes").await {
            config.seed_time_minutes = v;
        }
        if let Ok(Some(v)) = database::get_setting::<bool>(&db, "torrent.remove_after_import").await
        {
            config.remove_after_import = v;
        }

        info!(
            path = %config.download_dir.display(),
            "Using configured torrent download directory: {}",
            config.download_dir.display()
        );

        // Deleting a payload is destructive, so it only happens once an
        // explicit seeding rule has been satisfied. With both rules at 0 the
        // torrent seeds forever and nothing is ever removed — say so rather
        // than leaving the setting silently inert.
        if config.remove_after_import && config.seeding_rules().seeds_indefinitely() {
            warn!(
                service = "torrent",
                "torrent.remove_after_import is enabled but both torrent.seed_ratio_limit and torrent.seed_time_minutes are 0, so seeding never finishes and no torrent will ever be removed. Set one of them to enable cleanup."
            );
        }

        // A misconfigured download directory used to fall back to the system
        // temp dir with only a WARN, so payloads landed somewhere the operator
        // never chose and imports "just didn't happen". Now it is a loud,
        // persisted failure: the service starts (so the rest of the app boots
        // and the setting can be fixed from the UI) but refuses to add
        // torrents until the directory works.
        let effective_download_dir = config.download_dir.clone();
        let download_dir_error = match ensure_writable_dir(&config.download_dir).await {
            Ok(()) => None,
            Err(e) => {
                let message = format!(
                    "Torrent download directory '{}' is not usable: {}. Set `torrent.download_dir` to a writable path; torrents cannot be added until this is fixed.",
                    config.download_dir.display(),
                    e
                );
                error!(
                    service = "torrent",
                    path = %config.download_dir.display(),
                    error = %e,
                    "{}",
                    message
                );
                if let Some(user) = database::get_default_user_id(&db).await.ok().flatten() {
                    let auth_user = AuthUser {
                        user_id: user.to_string(),
                        email: None,
                        role: Some("admin".to_string()),
                    };
                    if let Err(notify_error) = database::create_service_notification(
                        &db,
                        &schema,
                        &auth_user,
                        "Torrent download directory unusable",
                        &message,
                    )
                    .await
                    {
                        warn!(
                            error = %notify_error,
                            "Failed to record a notification for the unusable torrent download directory: error={}",
                            notify_error
                        );
                    }
                }
                Some(message)
            }
        };

        // The session directory only holds resume state, so falling back keeps
        // the service usable — but at ERROR level, because losing it silently
        // means torrents stop being restored across restarts.
        let effective_session_dir = match ensure_writable_dir(&config.session_dir).await {
            Ok(()) => config.session_dir.clone(),
            Err(e) => {
                let temp = std::env::temp_dir().join("librarian-session");
                error!(
                    service = "torrent",
                    configured = %config.session_dir.display(),
                    fallback = %temp.display(),
                    error = %e,
                    "Torrent session directory '{}' is not usable ({}); falling back to '{}'. Resume state will not survive a reboot until `torrent.session_dir` is fixed.",
                    config.session_dir.display(),
                    e,
                    temp.display()
                );
                let _ = tokio::fs::create_dir_all(&temp).await;
                config.session_dir = temp.clone();
                temp
            }
        };

        let dht = if config.enable_dht {
            Some(DhtSessionConfig {
                persistence: Some(DhtPersistenceConfig {
                    config_filename: Some(effective_session_dir.join("dht.json")),
                    ..Default::default()
                }),
                ..Default::default()
            })
        } else {
            None
        };

        let session_opts = SessionOptions {
            dht,
            persistence: Some(librqbit::SessionPersistenceConfig::Json {
                folder: Some(effective_session_dir.clone()),
            }),
            ratelimits: LimitsConfig {
                upload_bps: NonZeroU32::new(config.upload_limit),
                download_bps: NonZeroU32::new(config.download_limit),
            },
            listen: if config.listen_port > 0 {
                Some(ListenerOptions {
                    listen_addr: ([0, 0, 0, 0], config.listen_port).into(),
                    ipv4_only: true,
                    ..Default::default()
                })
            } else {
                None
            },
            ..Default::default()
        };

        let session = Session::new_with_opts(effective_download_dir.clone(), session_opts)
            .await
            .context("Failed to create torrent session")?;

        let (event_tx, _) = broadcast::channel(1024);
        let upnp_result = Arc::new(RwLock::new(None));

        config.download_dir = effective_download_dir;
        config.session_dir = effective_session_dir;

        let auth_user = database::get_default_user_id(&db)
            .await
            .ok()
            .flatten()
            .map(|uid| AuthUser {
                user_id: uid.to_string(),
                email: None,
                role: Some("admin".to_string()),
            });

        // Initial session -> DB sync can be expensive when many torrent files exist.
        // Run it in the background so service startup (and TUI boot) is not blocked.
        if let Some(ref user) = auth_user {
            let session_sync = session.clone();
            let db_sync = db.clone();
            let schema_sync = schema.clone();
            let user_sync = user.clone();
            tokio::spawn(async move {
                tracing::info!("Starting background initial torrent session-to-DB sync");
                if let Err(e) = database::sync_session_to_database(
                    &session_sync,
                    &db_sync,
                    &schema_sync,
                    &user_sync,
                )
                .await
                {
                    warn!(
                        error = %e,
                        "Failed to sync existing session torrents to database during startup: error={}",
                        e
                    );
                } else {
                    tracing::info!("Finished background initial torrent session-to-DB sync");
                }
            });
        } else {
            warn!("No default user found in database; skipping initial torrent session-to-DB sync");
        }
        let active_downloads = count_active_downloads(session.as_ref());
        let download_queue: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        match database::restore_from_database(
            &session,
            &db,
            &schema,
            auth_user.as_ref(),
            config.max_concurrent,
            active_downloads,
            ADD_TORRENT_TIMEOUT,
        )
        .await
        {
            // Torrents restored above the concurrency limit start paused; they
            // go on the queue so the maintenance loop can start them as slots
            // free up, instead of staying paused until someone notices.
            Ok(queued) => {
                if !queued.is_empty() {
                    info!(
                        count = queued.len(),
                        max_concurrent = config.max_concurrent,
                        "Restored torrents queued behind the concurrent download limit: count={}, max_concurrent={}",
                        queued.len(),
                        config.max_concurrent
                    );
                    download_queue.lock().extend(queued);
                }
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Failed to restore torrents from database during startup: error={}",
                    e
                );
            }
        }

        // Seed already-completed torrents so startup doesn't emit Completed
        // events for torrents that were complete before this process started.
        //
        // The session alone is not enough: a restored torrent is still
        // `Initializing` (hash-checking) when `Session::new_with_opts` returns,
        // so its `progress_bytes` reads 0 and it looks unfinished. A second
        // later the progress loop saw it hit 100% and emitted a fresh
        // `Completed` event — re-running the whole import pipeline for every
        // finished torrent on every restart and overwriting its
        // `post_process_status`. So the database is the authority: any row that
        // already reached 100% *and* has a post-process status has been through
        // the pipeline. Rows that finished but never got a status are
        // deliberately left out, so the reconciliation sweep still retries them.
        let mut completed_hashes: std::collections::HashSet<String> =
            session.with_torrents(|iter| {
                iter.filter_map(|(_, handle)| {
                    let stats = handle.stats();
                    let progress = progress_ratio(stats.progress_bytes, stats.total_bytes);
                    if progress >= 1.0 {
                        Some(get_info_hash_hex(handle))
                    } else {
                        None
                    }
                })
                .collect()
            });
        match database::find_completed_torrents(&db).await {
            Ok(rows) => completed_hashes.extend(
                rows.into_iter()
                    .filter(|row| row.post_process_status.is_some())
                    .map(|row| row.info_hash),
            ),
            Err(e) => warn!(
                error = %e,
                "Failed to load already-processed torrents at startup; completed-file post-processing may run again for them: error={}",
                e
            ),
        }
        let completed = Arc::new(RwLock::new(completed_hashes));

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
        let schema_db = schema.clone();
        let db_sync_handle = tokio::spawn(async move {
            db_sync_loop(session_db, db_clone, completed_db, schema_db, token_db).await;
        });

        // Tracks info_hashes currently undergoing completed-file post-processing so the
        // completion-event handler and the reconciliation sweep never process the same
        // torrent concurrently (each removes its own entry when the task finishes).
        let processing: Arc<Mutex<std::collections::HashSet<String>>> =
            Arc::new(Mutex::new(std::collections::HashSet::new()));

        // Completion handler: process torrent files when a torrent finishes
        let token_compl = cancel_token.clone();
        let session_compl = session.clone();
        let db_compl = db.clone();
        let event_rx_compl = event_tx.subscribe();
        let processing_compl = processing.clone();
        let scan_compl = self
            .manager
            .get_library_scan()
            .await
            .ok_or_else(|| anyhow::anyhow!("library scan service not started"))?;
        let completion_handle = tokio::spawn(async move {
            completion_handler_loop(
                session_compl,
                db_compl,
                scan_compl,
                event_rx_compl,
                processing_compl,
                token_compl,
            )
            .await;
        });

        // Reconciliation sweep: safety net for a `Completed` event that was dropped
        // (e.g. broadcast receiver lag) — periodically finds torrents that finished
        // downloading but never got a post-process status and re-enqueues them
        // through the same processing entrypoint as the event handler.
        let token_recon = cancel_token.clone();
        let session_recon = session.clone();
        let db_recon = db.clone();
        let schema_recon = schema.clone();
        let processing_recon = processing.clone();
        let scan_recon = self
            .manager
            .get_library_scan()
            .await
            .ok_or_else(|| anyhow::anyhow!("library scan service not started"))?;
        let reconcile_handle = tokio::spawn(async move {
            reconciliation_sweep_loop(
                session_recon,
                db_recon,
                schema_recon,
                scan_recon,
                processing_recon,
                token_recon,
            )
            .await;
        });

        // UPnP: map the listen port at startup and keep renewing the lease so
        // NAT'd deployments don't silently lose inbound connectivity after ~1h.
        let token_upnp = cancel_token.clone();
        let upnp_loop_result = upnp_result.clone();
        let upnp_port = config.listen_port;
        let upnp_handle = tokio::spawn(async move {
            upnp_renewal_loop(upnp_loop_result, upnp_port, token_upnp).await;
        });

        // Maintenance: start queued torrents as concurrency slots free up, and
        // apply the seeding policy (stop seeding / remove + delete payload) to
        // torrents that have finished.
        let token_maint = cancel_token.clone();
        let session_maint = session.clone();
        let db_maint = db.clone();
        let schema_maint = schema.clone();
        let config_maint = config.clone();
        let queue_maint = download_queue.clone();
        let processing_maint = processing.clone();
        let event_tx_maint = event_tx.clone();
        let completed_maint = completed.clone();
        let maintenance_handle = tokio::spawn(async move {
            maintenance_loop(
                session_maint,
                db_maint,
                schema_maint,
                config_maint,
                queue_maint,
                processing_maint,
                completed_maint,
                event_tx_maint,
                token_maint,
            )
            .await;
        });

        let started_download_dir = config.download_dir.clone();
        let started_session_dir = config.session_dir.clone();
        let started_listen_port = config.listen_port;
        let started_enable_dht = config.enable_dht;
        let started_seed_ratio_limit = config.seed_ratio_limit;
        let started_seed_time_minutes = config.seed_time_minutes;
        let started_remove_after_import = config.remove_after_import;

        let runtime = Arc::new(TorrentRuntime {
            session,
            config,
            db,
            schema,
            event_tx,
            completed,
            processing,
            download_queue,
            download_dir_error,
            cancel_token,
            progress_handle,
            db_sync_handle,
            completion_handle,
            reconcile_handle,
            maintenance_handle,
            upnp_handle,
        });

        *self.inner.write().await = Some(runtime);
        info!(
            service = "torrent",
            download_dir = %started_download_dir.display(),
            session_dir = %started_session_dir.display(),
            listen_port = started_listen_port,
            enable_dht = started_enable_dht,
            seed_ratio_limit = started_seed_ratio_limit,
            seed_time_minutes = started_seed_time_minutes,
            remove_after_import = started_remove_after_import,
            "Torrent service started: download_dir={}, session_dir={}, listen_port={}, enable_dht={}, seed_ratio_limit={}, seed_time_minutes={}, remove_after_import={}",
            started_download_dir.display(),
            started_session_dir.display(),
            started_listen_port,
            started_enable_dht,
            started_seed_ratio_limit,
            started_seed_time_minutes,
            started_remove_after_import
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        info!(
            service = "torrent",
            "Torrent service stopping: canceling background loops and shutting down runtime"
        );
        let previous = self.inner.write().await.take();
        if let Some(arc_r) = previous {
            arc_r.cancel_token.cancel();
            if let Ok(runtime) = Arc::try_unwrap(arc_r) {
                runtime.shutdown().await;
            }
        }
        info!(
            service = "torrent",
            "Torrent service stopped: runtime and background loops shut down"
        );
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        let runtime = self.inner.read().await;
        let Some(runtime) = runtime.as_ref() else {
            return Ok(ServiceHealth::degraded("torrent service not started"));
        };
        if runtime.progress_handle.is_finished()
            || runtime.db_sync_handle.is_finished()
            || runtime.completion_handle.is_finished()
            || runtime.reconcile_handle.is_finished()
            || runtime.maintenance_handle.is_finished()
            || runtime.upnp_handle.is_finished()
        {
            return Ok(ServiceHealth::degraded(
                "one or more torrent background workers exited unexpectedly",
            ));
        }
        if let Some(ref problem) = runtime.download_dir_error {
            return Ok(ServiceHealth::degraded(problem.clone()));
        }
        Ok(ServiceHealth::healthy())
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
                        let progress = progress_ratio(stats.progress_bytes, stats.total_bytes);

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

                        let state = torrent_state_from_stats(&stats.state, progress);

                        let _ = event_tx.send(TorrentEvent::Progress {
                            id,
                            info_hash,
                            progress,
                            download_speed,
                            upload_speed,
                            peers: usize::try_from(peers).unwrap_or_default(),
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
    schema: LibrarianSchema,
    cancel: CancellationToken,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(10));
    let mut auth_user = database::get_default_user_id(&db)
        .await
        .ok()
        .flatten()
        .map(|uid| AuthUser {
            user_id: uid.to_string(),
            email: None,
            role: Some("admin".to_string()),
        });
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
                if auth_user.is_none() {
                    auth_user = database::get_default_user_id(&db)
                        .await
                        .ok()
                        .flatten()
                        .map(|uid| AuthUser {
                            user_id: uid.to_string(),
                            email: None,
                            role: Some("admin".to_string()),
                        });
                }
                if let Some(ref user) = auth_user
                    && let Err(e) =
                        database::sync_session_to_database(&session, &db, &schema, user).await
                    {
                        tracing::trace!(error = %e, "db_sync iteration failed");
                    }
            }
        }
    }
    let _ = completed;
}

/// Listens for `TorrentEvent::Completed` and processes the finished torrent's
/// files (creates media file records and queues ffprobe analysis).
/// This runs once per torrent completion rather than polling every 10 seconds.
///
/// Completion events are *dispatched* here but processed on a separate spawned
/// task (see [`process_completed_torrent_core`]) so this loop keeps draining the
/// broadcast channel instead of blocking on (potentially slow) file
/// matching/copy/analysis work. Previously, processing ran synchronously inline,
/// so a big import could make this receiver lag behind the shared
/// Progress-event channel; a `Completed` event skipped by `RecvError::Lagged`
/// was then lost forever. The `processing` set still dedupes concurrent runs
/// for the same torrent (e.g. a duplicate event, or a race with the
/// reconciliation sweep below).
async fn completion_handler_loop(
    session: Arc<Session>,
    db: Database,
    scan_service: Arc<LibraryScanService>,
    mut event_rx: broadcast::Receiver<TorrentEvent>,
    processing: Arc<Mutex<std::collections::HashSet<String>>>,
    cancel: CancellationToken,
) {
    let mut auth_user: Option<AuthUser> = database::get_default_user_id(&db)
        .await
        .ok()
        .flatten()
        .map(|uid| AuthUser {
            user_id: uid.to_string(),
            email: None,
            role: Some("admin".to_string()),
        });

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            result = event_rx.recv() => {
                let event = match result {
                    Ok(e) => e,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!(
                            skipped = n,
                            "Completion handler lagged behind event stream; skipped {} events",
                            n
                        );
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                };

                let (info_hash, name) = match event {
                    TorrentEvent::Completed { info_hash, name, .. } => (info_hash, name),
                    _ => continue,
                };

                // Ensure we have an auth user
                if auth_user.is_none() {
                    auth_user = database::get_default_user_id(&db)
                        .await
                        .ok()
                        .flatten()
                        .map(|uid| AuthUser {
                            user_id: uid.to_string(),
                            email: None,
                            role: Some("admin".to_string()),
                        });
                }

                let Some(ref user) = auth_user else {
                    warn!(
                        info_hash = %info_hash,
                        torrent_name = %name,
                        "Skipping completed-file processing: no default user available for info_hash={}, name='{}'",
                        info_hash,
                        name
                    );
                    continue;
                };

                if !processing.lock().insert(info_hash.clone()) {
                    info!(
                        info_hash = %info_hash,
                        torrent_name = %name,
                        "Completed-torrent processing already in flight; skipping duplicate trigger: info_hash={}",
                        info_hash
                    );
                    continue;
                }

                info!(
                    info_hash = %info_hash,
                    torrent_name = %name,
                    "Torrent completed, processing files for analysis: info_hash={}, name='{}'",
                    info_hash,
                    name
                );

                let session_task = session.clone();
                let db_task = db.clone();
                let scan_task = scan_service.clone();
                let user_task = user.clone();
                let processing_task = processing.clone();
                let info_hash_task = info_hash.clone();
                let name_task = name.clone();
                tokio::spawn(async move {
                    let outcome = process_completed_torrent_core(
                        &session_task,
                        &db_task,
                        &scan_task,
                        &user_task,
                        &info_hash_task,
                        None,
                    )
                    .await;
                    log_completed_torrent_outcome(&info_hash_task, &name_task, &outcome);
                    processing_task.lock().remove(&info_hash_task);
                });
            }
        }
    }
}

/// Core processing for a single completed torrent: reads file rows from the
/// active session and imports them via the library scan service.
///
/// Shared by the completion-event handler, the manual
/// [`TorrentService::process_completed_torrent`] API, and the reconciliation
/// sweep so all three entrypoints share one implementation. Returns `Ok(None)`
/// when the torrent has no active session handle (nothing to process, caller
/// should not touch post-process status so a later retry stays possible), and
/// `Ok(Some(summary))` once `process_torrent_source_files` has actually run.
async fn process_completed_torrent_core(
    session: &Arc<Session>,
    db: &Database,
    scan_service: &Arc<LibraryScanService>,
    auth_user: &AuthUser,
    info_hash: &str,
    library_override: Option<String>,
) -> Result<Option<SourceProcessSummary>> {
    let Some(rows) = database::completed_torrent_file_rows(session, db, info_hash).await? else {
        return Ok(None);
    };

    let files = rows
        .into_iter()
        .filter(|row| !row.is_excluded)
        .map(|row| SourceFileImport {
            source_path: row.file_path,
            relative_path: row.relative_path,
            file_size: row.file_size,
            downloaded_bytes: row.downloaded_bytes,
            file_index: Some(row.file_index),
        })
        .collect::<Vec<_>>();

    Ok(Some(
        scan_service
            .process_torrent_source_files(auth_user, info_hash, library_override, files)
            .await,
    ))
}

fn log_completed_torrent_outcome(
    info_hash: &str,
    name: &str,
    outcome: &Result<Option<SourceProcessSummary>>,
) {
    match outcome {
        Ok(Some(summary)) if summary.success => info!(
            info_hash = %info_hash,
            torrent_name = %name,
            processed = summary.files_processed,
            "Completed torrent post-processing finished: info_hash={}, name='{}', processed={}",
            info_hash,
            name,
            summary.files_processed
        ),
        Ok(Some(summary)) => warn!(
            info_hash = %info_hash,
            torrent_name = %name,
            processed = summary.files_processed,
            failed = summary.files_failed,
            error = ?summary.error,
            "Completed torrent post-processing did not fully succeed: info_hash={}, name='{}', processed={}, failed={}, error={:?}",
            info_hash,
            name,
            summary.files_processed,
            summary.files_failed,
            summary.error
        ),
        Ok(None) => warn!(
            info_hash = %info_hash,
            torrent_name = %name,
            "Skipping completed-file processing: torrent was not found in active session"
        ),
        Err(e) => warn!(
            error = %e,
            info_hash = %info_hash,
            torrent_name = %name,
            "Failed to read completed torrent file list: info_hash={}, name='{}', error={}",
            info_hash,
            name,
            e
        ),
    }
}

/// Periodic safety net for [`completion_handler_loop`]: finds torrents that
/// finished downloading but whose completed-file post-processing never ran
/// (or never reached a terminal status) — e.g. because their `Completed` event
/// was dropped by a lagging broadcast receiver — and re-enqueues them through
/// [`process_completed_torrent_core`], the same entrypoint the event handler
/// uses. Runs every 5 minutes for the lifetime of the service.
async fn reconciliation_sweep_loop(
    session: Arc<Session>,
    db: Database,
    schema: LibrarianSchema,
    scan_service: Arc<LibraryScanService>,
    processing: Arc<Mutex<std::collections::HashSet<String>>>,
    cancel: CancellationToken,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(5 * 60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
                if let Err(e) =
                    run_reconciliation_sweep(&session, &db, &schema, &scan_service, &processing)
                        .await
                {
                    warn!(
                        error = %e,
                        "Torrent completion reconciliation sweep failed: error={}",
                        e
                    );
                }
            }
        }
    }
}

async fn run_reconciliation_sweep(
    session: &Arc<Session>,
    db: &Database,
    schema: &LibrarianSchema,
    scan_service: &Arc<LibraryScanService>,
    processing: &Arc<Mutex<std::collections::HashSet<String>>>,
) -> Result<()> {
    let Some(user) = database::get_default_user_id(db)
        .await?
        .map(|uid| AuthUser {
            user_id: uid.to_string(),
            email: None,
            role: Some("admin".to_string()),
        })
    else {
        return Ok(());
    };

    reconcile_orphaned_torrents(session, db, schema, &user).await;

    let candidates = database::find_unprocessed_completed_torrents(db).await?;
    if candidates.is_empty() {
        return Ok(());
    }

    info!(
        count = candidates.len(),
        "Reconciliation sweep found completed torrents missing post-processing: count={}",
        candidates.len()
    );

    for torrent in candidates {
        if !processing.lock().insert(torrent.info_hash.clone()) {
            // Already being handled by the event path or a previous sweep tick.
            continue;
        }

        let session_task = session.clone();
        let db_task = db.clone();
        let scan_task = scan_service.clone();
        let user_task = user.clone();
        let processing_task = processing.clone();
        let info_hash_task = torrent.info_hash.clone();
        let name_task = torrent.name.clone();
        tokio::spawn(async move {
            let outcome = process_completed_torrent_core(
                &session_task,
                &db_task,
                &scan_task,
                &user_task,
                &info_hash_task,
                None,
            )
            .await;
            log_completed_torrent_outcome(&info_hash_task, &name_task, &outcome);
            processing_task.lock().remove(&info_hash_task);
        });
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// Download queue + seeding policy
// -----------------------------------------------------------------------------

/// Runs the two policies that need periodic evaluation:
///
/// 1. **Download queue** — torrents started paused because
///    `torrent.max_concurrent` was reached get unpaused as slots free up.
///    Without this they stayed paused indefinitely: nothing ever reconsidered
///    them after the download that filled the last slot finished.
/// 2. **Seeding policy** — completed torrents are stopped (and optionally
///    removed with their payload) once `torrent.seed_ratio_limit` /
///    `torrent.seed_time_minutes` are satisfied. See
///    [`crate::services::torrent::seeding`] for the pure decision logic.
#[allow(clippy::too_many_arguments)]
async fn maintenance_loop(
    session: Arc<Session>,
    db: Database,
    schema: LibrarianSchema,
    config: TorrentServiceConfig,
    download_queue: Arc<Mutex<VecDeque<String>>>,
    processing: Arc<Mutex<std::collections::HashSet<String>>>,
    completed: Arc<RwLock<std::collections::HashSet<String>>>,
    event_tx: broadcast::Sender<TorrentEvent>,
    cancel: CancellationToken,
) {
    let mut interval = tokio::time::interval(MAINTENANCE_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
                start_queued_downloads(&session, &config, &download_queue).await;
                if let Err(e) = enforce_seeding_policy(
                    &session,
                    &db,
                    &schema,
                    &config,
                    &processing,
                    &completed,
                    &event_tx,
                )
                .await
                {
                    warn!(
                        error = %e,
                        "Torrent seeding policy sweep failed: error={}",
                        e
                    );
                }
            }
        }
    }
}

/// Unpause queued torrents, oldest first, while download slots are available.
async fn start_queued_downloads(
    session: &Arc<Session>,
    config: &TorrentServiceConfig,
    download_queue: &Arc<Mutex<VecDeque<String>>>,
) {
    if download_queue.lock().is_empty() {
        return;
    }
    let mut slots = free_download_slots(config.max_concurrent, count_active_downloads(session));

    while slots > 0 {
        let Some(info_hash) = download_queue.lock().pop_front() else {
            break;
        };
        let Some(handle) = database::session_handle(session, &info_hash) else {
            // Removed from the session since it was queued; drop it silently.
            continue;
        };
        let stats = handle.stats();
        if progress_ratio(stats.progress_bytes, stats.total_bytes) >= 1.0 {
            // Finished while queued (e.g. files were already on disk).
            continue;
        }
        match session.unpause(&handle).await {
            Ok(()) => {
                slots -= 1;
                info!(
                    info_hash = %info_hash,
                    torrent_name = %handle.name().unwrap_or_else(|| "Unknown".to_string()),
                    max_concurrent = config.max_concurrent,
                    "Started queued torrent now that a download slot is free: info_hash={}, name='{}', max_concurrent={}",
                    info_hash,
                    handle.name().unwrap_or_else(|| "Unknown".to_string()),
                    config.max_concurrent
                );
            }
            Err(e) => {
                warn!(
                    info_hash = %info_hash,
                    error = %e,
                    "Failed to start queued torrent, leaving it queued: info_hash={}, error={}",
                    info_hash,
                    e
                );
                download_queue.lock().push_back(info_hash);
                break;
            }
        }
    }
}

/// Apply [`decide_seeding_action`] to every torrent that has finished
/// downloading. Entity-layer writes only: the `Torrent`/`TorrentFile` rows are
/// deleted through the generated mutations, and the payload through librqbit.
async fn enforce_seeding_policy(
    session: &Arc<Session>,
    db: &Database,
    schema: &LibrarianSchema,
    config: &TorrentServiceConfig,
    processing: &Arc<Mutex<std::collections::HashSet<String>>>,
    completed: &Arc<RwLock<std::collections::HashSet<String>>>,
    event_tx: &broadcast::Sender<TorrentEvent>,
) -> Result<()> {
    let rules = config.seeding_rules();
    if rules.seeds_indefinitely() {
        return Ok(());
    }

    let Some(auth_user) = database::get_default_user_id(db)
        .await?
        .map(|uid| AuthUser {
            user_id: uid.to_string(),
            email: None,
            role: Some("admin".to_string()),
        })
    else {
        return Ok(());
    };

    let now = Utc::now();
    for torrent in database::find_completed_torrents(db).await? {
        let Some(handle) = database::session_handle(session, &torrent.info_hash) else {
            continue;
        };
        // Never touch a torrent whose files are being imported right now.
        if processing.lock().contains(&torrent.info_hash) {
            continue;
        }

        let stats = handle.stats();
        let facts = SeedingFacts {
            // The row's cumulative total plus whatever the live session has
            // uploaded since the last db sync tick. `stats.uploaded_bytes` on
            // its own is per-session and resets on restart, so it would
            // under-report the share ratio after every restart.
            uploaded_bytes: Torrent::accumulate_uploaded_bytes(
                torrent.uploaded_bytes_total,
                torrent.uploaded_bytes,
                stats.uploaded_bytes as i64,
            ),
            total_bytes: stats.total_bytes as i64,
            completed_at: parse_timestamp(torrent.completed_at.as_deref()),
            post_process_status: torrent.post_process_status.clone(),
            // Private-tracker floors recorded at grab time.
            minimum_ratio: torrent.minimum_ratio,
            minimum_seed_time_minutes: torrent.minimum_seed_time_minutes.map(i64::from),
        };
        let is_paused = matches!(stats.state, librqbit::TorrentStatsState::Paused);
        let ratio = crate::services::torrent::seeding::share_ratio(&facts);

        match decide_seeding_action(&rules, &facts, now, is_paused) {
            SeedingAction::Continue => {}
            SeedingAction::StopSeeding => match session.pause(&handle).await {
                Ok(()) => info!(
                    info_hash = %torrent.info_hash,
                    torrent_name = %torrent.name,
                    ratio = ratio,
                    seed_ratio_limit = rules.ratio_limit,
                    seed_time_minutes = rules.time_minutes,
                    "Stopped seeding '{}' ({}): seeding rules satisfied at ratio {:.3} (limit {}, time limit {} min); files kept",
                    torrent.name,
                    torrent.info_hash,
                    ratio,
                    rules.ratio_limit,
                    rules.time_minutes
                ),
                Err(e) => warn!(
                    info_hash = %torrent.info_hash,
                    torrent_name = %torrent.name,
                    error = %e,
                    "Failed to stop seeding '{}' ({}): error={}",
                    torrent.name,
                    torrent.info_hash,
                    e
                ),
            },
            SeedingAction::RemoveAndDeleteFiles => {
                let id = handle.id();
                // `delete_files = true`: the library copy is a hardlink into
                // the media tree, so unlinking the download payload leaves the
                // imported file intact.
                if let Err(e) = session
                    .delete(TorrentIdOrHash::Id(id), /* delete_files */ true)
                    .await
                {
                    warn!(
                        info_hash = %torrent.info_hash,
                        torrent_name = %torrent.name,
                        error = %e,
                        "Failed to remove imported torrent '{}' ({}) after seeding: error={}",
                        torrent.name,
                        torrent.info_hash,
                        e
                    );
                    continue;
                }
                if let Err(e) =
                    database::delete_torrent(db, schema, &auth_user, &torrent.info_hash).await
                {
                    warn!(
                        info_hash = %torrent.info_hash,
                        error = %e,
                        "Removed imported torrent from the session but failed to delete its database row: info_hash={}, error={}",
                        torrent.info_hash,
                        e
                    );
                }
                completed.write().remove(&torrent.info_hash);
                let _ = event_tx.send(TorrentEvent::Removed {
                    id,
                    info_hash: torrent.info_hash.clone(),
                });
                info!(
                    info_hash = %torrent.info_hash,
                    torrent_name = %torrent.name,
                    ratio = ratio,
                    "Removed imported torrent '{}' ({}) and deleted its download payload: import completed and seeding rules satisfied at ratio {:.3}",
                    torrent.name,
                    torrent.info_hash,
                    ratio
                );
            }
        }
    }

    Ok(())
}

/// Parse an entity timestamp (`%Y-%m-%dT%H:%M:%S%.3fZ` and other RFC 3339
/// shapes) into a UTC instant.
fn parse_timestamp(value: Option<&str>) -> Option<chrono::DateTime<Utc>> {
    let value = value?;
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .map(|naive| naive.and_utc())
        })
}

/// Mark rows the database still thinks are downloading but that the live
/// session has never heard of.
///
/// `restore_from_database` runs at startup, long before this sweep's first
/// tick, so anything still missing here really is orphaned — its resume data
/// was lost or its magnet could not be re-resolved. Without this the UI showed
/// such a torrent stuck at "downloading" with 0 peers indefinitely, which is
/// indistinguishable from "torrent downloading doesn't work".
async fn reconcile_orphaned_torrents(
    session: &Arc<Session>,
    db: &Database,
    schema: &LibrarianSchema,
    auth_user: &AuthUser,
) {
    let active = match database::find_active_torrents(db).await {
        Ok(rows) => rows,
        Err(e) => {
            warn!(
                error = %e,
                "Failed to list in-progress torrents for orphan reconciliation: error={}",
                e
            );
            return;
        }
    };

    for torrent in active {
        if database::session_handle(session, &torrent.info_hash).is_some() {
            continue;
        }
        warn!(
            info_hash = %torrent.info_hash,
            torrent_name = %torrent.name,
            state = %torrent.state,
            progress = torrent.progress,
            "Torrent '{}' ({}) is recorded as '{}' at {:.1}% but is not in the torrent session; marking it as errored so it stops showing as an active download",
            torrent.name,
            torrent.info_hash,
            torrent.state,
            torrent.progress * 100.0
        );
        if let Err(e) =
            database::update_state(db, schema, auth_user, &torrent.info_hash, "error").await
        {
            warn!(
                info_hash = %torrent.info_hash,
                error = %e,
                "Failed to mark orphaned torrent as errored: info_hash={}, error={}",
                torrent.info_hash,
                e
            );
        }
    }
}

/// Keeps the UPnP port mapping alive for the lifetime of the service. Each
/// mapping is leased for 3600s by [`perform_upnp`]; renewing every 40 minutes
/// keeps it from expiring on NAT'd deployments. Only logs on a state change
/// (success after failure, or a fresh failure) so a healthy renewal doesn't
/// spam the log every cycle.
async fn upnp_renewal_loop(
    upnp_result: Arc<RwLock<Option<UpnpResult>>>,
    port: u16,
    cancel: CancellationToken,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(40 * 60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last_success: Option<bool> = None;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = interval.tick() => {
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

                if last_success != Some(result.success) {
                    if result.success {
                        info!(
                            port = port,
                            external_ip = ?result.external_ip,
                            "UPnP port mapping renewed successfully (port={})",
                            port
                        );
                    } else {
                        warn!(
                            port = port,
                            error = ?result.error,
                            "UPnP port mapping renewal failed (port={}): {:?}",
                            port,
                            result.error
                        );
                    }
                }
                last_success = Some(result.success);

                *upnp_result.write() = Some(result);
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Helpers used by runtime and public API
// -----------------------------------------------------------------------------

async fn get_torrent_info_impl(session: &Session, id: usize) -> Result<TorrentInfo> {
    let handle = session
        .get(TorrentIdOrHash::Id(id))
        .context("Torrent not found")?;
    let stats = handle.stats();
    let info_hash = get_info_hash_hex(&handle);
    let name = handle.name().unwrap_or_else(|| "Unknown".to_string());
    let progress = progress_ratio(stats.progress_bytes, stats.total_bytes);
    let state = torrent_state_from_stats(&stats.state, progress);
    let (download_speed, upload_speed, peers) = stats
        .live
        .as_ref()
        .map(|live| {
            let dl = (live.download_speed.mbps * 125000.0) as u64;
            let ul = (live.upload_speed.mbps * 125000.0) as u64;
            (dl, ul, live.snapshot.peer_stats.live)
        })
        .unwrap_or((0, 0, 0));

    let files = get_torrent_files_list(&handle);
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
        peers: usize::try_from(peers).unwrap_or_default(),
        seeds: 0,
        // The session's real output folder for this torrent, not the
        // service-wide download dir: librqbit puts multi-file torrents in a
        // per-torrent sub-folder, so reconstructing the path from the config
        // produced a `save_path` (and file paths) that did not exist on disk
        // whenever the sub-folder name differed from `handle.name()`.
        save_path: handle.output_folder().to_string_lossy().to_string(),
        files,
    })
}

/// Verify a configured directory exists (creating it if needed) and is actually
/// writable. `create_dir_all` alone succeeds on an existing read-only
/// directory, so the probe file is what catches a mounted-but-not-writable
/// download path.
async fn ensure_writable_dir(dir: &Path) -> Result<()> {
    tokio::fs::create_dir_all(dir)
        .await
        .with_context(|| format!("could not create directory '{}'", dir.display()))?;
    let probe = dir.join(format!(".librarian-write-test-{}", Uuid::new_v4()));
    tokio::fs::write(&probe, b"")
        .await
        .with_context(|| format!("directory '{}' is not writable", dir.display()))?;
    let _ = tokio::fs::remove_file(&probe).await;
    Ok(())
}

/// Whether a newly added torrent has to start paused because
/// `torrent.max_concurrent` is already reached. `0` disables the limit.
fn should_start_paused(max_concurrent: usize, active_downloads: usize) -> bool {
    max_concurrent > 0 && active_downloads >= max_concurrent
}

/// How many queued torrents may be started right now. `max_concurrent == 0`
/// means unlimited.
fn free_download_slots(max_concurrent: usize, active_downloads: usize) -> usize {
    if max_concurrent == 0 {
        return usize::MAX;
    }
    max_concurrent.saturating_sub(active_downloads)
}

fn count_active_downloads(session: &Session) -> usize {
    use librqbit::TorrentStatsState;

    session.with_torrents(|iter| {
        iter.filter(|(_, handle)| {
            let stats = handle.stats();
            match stats.state {
                TorrentStatsState::Initializing { .. } => true,
                TorrentStatsState::Live => stats.progress_bytes < stats.total_bytes,
                TorrentStatsState::Paused | TorrentStatsState::Error => false,
            }
        })
        .count()
    })
}

/// Live file list for a torrent, with the paths librqbit actually writes to
/// (`handle.output_folder()` joined with each file's relative name).
fn get_torrent_files_list(handle: &Arc<librqbit::ManagedTorrent>) -> Vec<TorrentFile> {
    let mut files = Vec::new();
    if let Some(metadata) = handle.metadata.load_full() {
        let stats = handle.stats();
        let output_folder = handle.output_folder();
        for (idx, file_info) in metadata.file_infos.iter().enumerate() {
            let file_progress = stats.file_progress.get(idx).copied().unwrap_or(0);
            let size = file_info.len;
            let progress = if size > 0 {
                (file_progress as f64 / size as f64).min(1.0)
            } else {
                0.0
            };
            let path = output_folder
                .join(file_info.relative_filename.to_string_lossy().as_ref())
                .to_string_lossy()
                .to_string();
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

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Concurrency limit (pause-on-max-concurrent + queue drain)
    // -------------------------------------------------------------------------

    #[test]
    fn zero_max_concurrent_means_unlimited() {
        assert!(!should_start_paused(0, 0));
        assert!(!should_start_paused(0, 1_000));
        assert_eq!(free_download_slots(0, 1_000), usize::MAX);
    }

    #[test]
    fn new_torrents_start_paused_only_at_the_limit() {
        assert!(!should_start_paused(3, 0));
        assert!(!should_start_paused(3, 2));
        assert!(should_start_paused(3, 3));
        // Over the limit (e.g. the limit was lowered while downloads ran).
        assert!(should_start_paused(3, 5));
    }

    #[test]
    fn free_slots_track_the_limit_and_never_underflow() {
        assert_eq!(free_download_slots(3, 0), 3);
        assert_eq!(free_download_slots(3, 2), 1);
        assert_eq!(free_download_slots(3, 3), 0);
        // More active downloads than the limit allows (e.g. the limit was
        // lowered at runtime): saturating keeps the maintenance loop from
        // wrapping around and unpausing the whole queue.
        assert_eq!(free_download_slots(3, 9), 0);
    }

    #[test]
    fn a_torrent_paused_at_the_limit_becomes_startable_when_a_slot_frees() {
        let max = 2;
        // Two active downloads: the third add is queued.
        assert!(should_start_paused(max, 2));
        assert_eq!(free_download_slots(max, 2), 0);
        // One finishes; the queued torrent now has a slot.
        assert_eq!(free_download_slots(max, 1), 1);
    }

    // -------------------------------------------------------------------------
    // Timestamp parsing for the seeding clock
    // -------------------------------------------------------------------------

    #[test]
    fn parses_the_entity_timestamp_format() {
        // Exactly what `now_iso_string` in database.rs writes to `completedAt`.
        let parsed = parse_timestamp(Some("2026-09-07T01:03:27.657Z")).expect("should parse");
        assert_eq!(
            parsed.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "2026-09-07T01:03:27.657Z"
        );
    }

    #[test]
    fn parses_a_timestamp_without_a_zone_as_utc() {
        let parsed = parse_timestamp(Some("2026-09-07T01:03:27.657")).expect("should parse");
        assert_eq!(
            parsed.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            "2026-09-07T01:03:27.657Z"
        );
    }

    #[test]
    fn unparseable_or_missing_timestamps_are_none() {
        assert!(parse_timestamp(None).is_none());
        assert!(parse_timestamp(Some("")).is_none());
        assert!(parse_timestamp(Some("not a date")).is_none());
    }

    // -------------------------------------------------------------------------
    // Seeding rules wired from settings
    // -------------------------------------------------------------------------

    #[test]
    fn config_defaults_map_onto_the_seeding_rules() {
        let rules = TorrentServiceConfig::default().seeding_rules();
        assert_eq!(rules.ratio_limit, 1.0);
        assert_eq!(rules.time_minutes, 0);
        assert!(!rules.remove_after_import);
        assert!(!rules.seeds_indefinitely());
    }

    #[test]
    fn a_zeroed_seeding_config_seeds_indefinitely() {
        let config = TorrentServiceConfig {
            seed_ratio_limit: 0.0,
            seed_time_minutes: 0,
            ..Default::default()
        };
        assert!(config.seeding_rules().seeds_indefinitely());
    }

    // -------------------------------------------------------------------------
    // Directory validation (no more silent /tmp fallback)
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn a_creatable_writable_directory_is_accepted() {
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("downloads/nested");
        ensure_writable_dir(&nested)
            .await
            .expect("nested directory should be created and writable");
        assert!(nested.is_dir());
        // The write probe must not leave anything behind.
        assert_eq!(std::fs::read_dir(&nested).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn an_unwritable_directory_is_reported_not_silently_replaced() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let readonly = dir.path().join("readonly");
        std::fs::create_dir(&readonly).expect("create dir");
        std::fs::set_permissions(&readonly, std::fs::Permissions::from_mode(0o500))
            .expect("chmod 0500");

        let result = ensure_writable_dir(&readonly).await;

        // Restore permissions so the tempdir can be cleaned up.
        let _ = std::fs::set_permissions(&readonly, std::fs::Permissions::from_mode(0o700));

        // Running as root bypasses the permission bits entirely; the assertion
        // only makes sense for an unprivileged user.
        if unsafe { libc::geteuid() } != 0 {
            let error = result.expect_err("a read-only directory must be rejected");
            assert!(
                format!("{error:#}").contains("not writable"),
                "unexpected error: {error:#}"
            );
        }
    }

    #[tokio::test]
    async fn a_path_that_is_a_file_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("not-a-directory");
        std::fs::write(&file, b"x").expect("write file");
        let error = ensure_writable_dir(&file)
            .await
            .expect_err("a file must not be accepted as a download directory");
        assert!(
            format!("{error:#}").contains("could not create directory"),
            "unexpected error: {error:#}"
        );
    }
}
