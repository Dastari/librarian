//! Global services manager for long-running and background services.
//!
//! Services register with the manager and are started/stopped/restarted together.
//! Start order respects [dependencies](Service::dependencies); a service is only
//! started after all of its dependencies.
//!
//! **HTTP route registration:** Services (or main) can register `/api/*` route
//! builders via [add_api_routes](ServicesManagerBuilder::add_api_routes). The
//! [HttpServerService](crate::services::http_server::HttpServerService) builds
//! the app by calling [build_api_router](ServicesManager::build_api_router)
//! which merges all registered route builders. Use this so any service can
//! contribute endpoints without the HTTP service needing to know about them.
//!
//! See `docs/design.md` (Background Services section) for how to implement and register services.

use crate::jobs::schedule_sync::{ScheduleSyncConfig, ScheduleSyncService};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use axum::Router;
use parking_lot::RwLock as ParkingRwLock;
use serde::Serialize;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::app::AppState;
use crate::jobs::download_monitor::{AutoDownloadService, AutoDownloadServiceConfig};

use crate::services::auth::{AuthConfig, AuthService};
use crate::services::backup::{BackupService, BackupServiceConfig};
use crate::services::cast::{CastService, CastServiceConfig};
use crate::services::database::{DatabaseService, DatabaseServiceConfig};
use crate::services::graphql::{GraphqlService, GraphqlServiceConfig};
use crate::services::http_server::{HttpServerConfig, HttpServerService};
use crate::services::library_scan::{LibraryScanService, LibraryScanServiceConfig};
use crate::services::logging::{LoggingService, LoggingServiceConfig};
use crate::services::sources::service::{SourcesService, SourcesServiceConfig};
use crate::services::storage::{ObjectStorageService, ObjectStorageServiceConfig};
use crate::services::torrent::{TorrentService, TorrentServiceConfig};
use crate::services::transcode::{TranscodeService, TranscodeServiceConfig};

/// Health status of a service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Result of a service health check.
#[derive(Debug, Clone, Serialize)]
pub struct ServiceHealth {
    pub status: HealthStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ServiceHealth {
    pub fn healthy() -> Self {
        Self {
            status: HealthStatus::Healthy,
            message: None,
        }
    }

    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Degraded,
            message: Some(message.into()),
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            message: Some(message.into()),
        }
    }
}

/// A service that can be started, stopped, restarted, and health-checked by the manager.
///
/// **Required:**
/// - Implement [start](Service::start), [stop](Service::stop), [restart](Service::restart), and [health](Service::health).
/// - Use the [tracing] crate for all lifecycle and operational logging. Log at appropriate
///   levels (e.g. `info` for start/stop/restart, `debug` for periodic work, `warn`/`error`
///   for failures) and include the service name or a consistent target so logs are
///   filterable (e.g. `tracing::info!(service = %self.name(), "Started")`).
///
/// **Dependencies:** Return the list of service names that must be started before this
/// service via [dependencies](Service::dependencies). The manager will start in dependency
/// order and will not start a service until its dependencies are started.
///
/// Implement this for components that:
/// - Run background tasks (e.g. torrent progress monitor, cast mDNS discovery)
/// - Hold a long-lived session or connection (e.g. database pool)
/// - Run on timers or event loops
///
/// Stateless utilities (e.g. metadata client, ffmpeg wrapper) do not need
/// to implement `Service`; they are constructed and passed where needed.
#[async_trait]
pub trait Service: Send + Sync + 'static {
    /// Unique name for logging and lookup (e.g. "torrent", "cast", "database").
    fn name(&self) -> &str;

    /// Names of services that must be started before this one. Start order is
    /// computed from this; cycles are an error.
    fn dependencies(&self) -> Vec<String> {
        Vec::new()
    }

    /// Start background tasks or connections. Idempotent allowed.
    /// Log start and any errors using [tracing].
    async fn start(&self) -> Result<()>;

    /// Stop background tasks and release resources. Idempotent allowed.
    /// Log stop and any errors using [tracing].
    async fn stop(&self) -> Result<()>;

    /// Restart the service: stop then start. Override if you need atomic or
    /// non-sequential restart behaviour. Log restart and errors using [tracing].
    async fn restart(&self) -> Result<()> {
        self.stop().await?;
        self.start().await
    }

    /// Report current health. Used by the manager and by health endpoints.
    /// Default returns [ServiceHealth::healthy].
    async fn health(&self) -> Result<ServiceHealth> {
        Ok(ServiceHealth::healthy())
    }

    /// Whether this service exposes HTTP routes (e.g. `/api/artwork`).
    /// Used by the app to decide which route modules to merge; route
    /// wiring is done in `main`, not in the manager.
    fn provides_routes(&self) -> bool {
        false
    }
}

/// Pending registration for the builder.
pub struct ServiceRegistration(ServiceRegistrationKind);

enum ServiceRegistrationKind {
    Auth(AuthConfig),
    Database(DatabaseServiceConfig),
    Storage(ObjectStorageServiceConfig),
    Backup(BackupServiceConfig),
    Logging(LoggingServiceConfig),
    Graphql(GraphqlServiceConfig),
    Http(HttpServerConfig),
    Torrent(TorrentServiceConfig),
    Cast(CastServiceConfig),
    Sources(SourcesServiceConfig),
    LibraryScan(LibraryScanServiceConfig),
    AutoDownload(AutoDownloadServiceConfig),
    ScheduleSync(ScheduleSyncConfig),
    Transcode(TranscodeServiceConfig),
    Service(Arc<dyn Service>),
}

type ApiRouteBuilder = Box<dyn Fn(AppState) -> Router<AppState> + Send + Sync>;
type ApiRouteRegistration = (String, ApiRouteBuilder);

/// Types that can be added to a [ServicesManagerBuilder] via [add_service](ServicesManagerBuilder::add_service).
///
/// Implemented for [AuthConfig], [DatabaseServiceConfig], [LoggingServiceConfig], [GraphqlServiceConfig], and [Arc]\[[dyn](Service)\].
pub trait IntoServiceRegistration {
    #[doc(hidden)]
    fn into_registration(self) -> ServiceRegistration;
}

impl IntoServiceRegistration for GraphqlServiceConfig {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::Graphql(self))
    }
}

impl IntoServiceRegistration for AuthConfig {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::Auth(self))
    }
}

impl IntoServiceRegistration for DatabaseServiceConfig {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::Database(self))
    }
}

impl IntoServiceRegistration for ObjectStorageServiceConfig {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::Storage(self))
    }
}

impl IntoServiceRegistration for BackupServiceConfig {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::Backup(self))
    }
}

impl IntoServiceRegistration for LoggingServiceConfig {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::Logging(self))
    }
}

impl IntoServiceRegistration for HttpServerConfig {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::Http(self))
    }
}

impl IntoServiceRegistration for TorrentServiceConfig {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::Torrent(self))
    }
}

impl IntoServiceRegistration for CastServiceConfig {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::Cast(self))
    }
}

impl IntoServiceRegistration for SourcesServiceConfig {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::Sources(self))
    }
}

impl IntoServiceRegistration for LibraryScanServiceConfig {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::LibraryScan(self))
    }
}

impl IntoServiceRegistration for ScheduleSyncConfig {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::ScheduleSync(self))
    }
}

impl IntoServiceRegistration for AutoDownloadServiceConfig {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::AutoDownload(self))
    }
}

impl IntoServiceRegistration for TranscodeServiceConfig {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::Transcode(self))
    }
}

impl IntoServiceRegistration for Arc<dyn Service> {
    fn into_registration(self) -> ServiceRegistration {
        ServiceRegistration(ServiceRegistrationKind::Service(self))
    }
}

/// Builder for [ServicesManager]: add services with configs, then [build](ServicesManagerBuilder::build) or [start](ServicesManagerBuilder::start).
///
/// Use like Bevy plugins: create a builder, add services (with dependencies and configs), then call `start()`.
///
/// # Example
///
/// ```ignore
/// let services = ServicesManager::builder()
///     .add_service(DatabaseServiceConfig {
///         database_url: config.database_url.clone(),
///         connect_timeout: Duration::from_secs(30),
///     })
///     .add_service(LoggingServiceConfig::default())
///     .add_service(Arc::new(MyService::new(...)))
///     .start()
///     .await?;
/// ```
pub struct ServicesManagerBuilder {
    registrations: Vec<ServiceRegistration>,
    /// Route builders for /api/*; merged in order when the HTTP app is built.
    api_route_registrations: Vec<ApiRouteRegistration>,
}

impl ServicesManagerBuilder {
    pub fn new() -> Self {
        Self {
            registrations: Vec::new(),
            api_route_registrations: Vec::new(),
        }
    }

    /// Register a route builder for `/api/*`. All registered builders are merged
    /// in order when the HTTP server builds the app. Use this so services (or
    /// main) can contribute REST endpoints without the HTTP service knowing
    /// about them. `name` is for logging; `builder` receives [AppState](crate::app::AppState)
    /// and returns a [Router](axum::Router) to merge under `/api`.
    pub fn add_api_routes<N, F>(mut self, name: N, builder: F) -> Self
    where
        N: Into<String>,
        F: Fn(AppState) -> Router<AppState> + Send + Sync + 'static,
    {
        self.api_route_registrations
            .push((name.into(), Box::new(builder)));
        self
    }

    /// Add a service: a config (e.g. [DatabaseServiceConfig], [LoggingServiceConfig], [AuthConfig], [GraphqlServiceConfig])
    /// or a pre-built [Arc]\[[dyn](Service)\]. Config-based services are instantiated when [build](Self::build) or
    /// [start](Self::start) is called. Add in dependency order (e.g. database before logging).
    pub fn add_service<T: IntoServiceRegistration>(mut self, t: T) -> Self {
        self.registrations.push(t.into_registration());
        self
    }

    /// Build the manager and register all services. Does not start them.
    pub async fn build(self) -> Result<Arc<ServicesManager>> {
        let manager = Arc::new(ServicesManager::new());
        for (name, builder) in self.api_route_registrations {
            manager.register_api_routes(name, builder);
        }
        for reg in self.registrations {
            match reg.0 {
                ServiceRegistrationKind::Auth(config) => {
                    let auth_svc = Arc::new(AuthService::new(manager.clone(), config));
                    manager.register_auth(auth_svc).await;
                }
                ServiceRegistrationKind::Database(config) => {
                    let db_svc = Arc::new(
                        DatabaseService::from_config(config, Some(Arc::downgrade(&manager)))
                            .await
                            .context("Failed to create database service from config")?,
                    );
                    manager.register_database(db_svc).await;
                }
                ServiceRegistrationKind::Storage(config) => {
                    let storage_svc = Arc::new(
                        ObjectStorageService::new(config)
                            .context("Failed to create object storage service")?,
                    );
                    manager.register_storage(storage_svc).await;
                }
                ServiceRegistrationKind::Backup(config) => {
                    let db = manager
                        .get_database_unchecked()
                        .await
                        .context("Backup service requires database service")?
                        .pool()
                        .clone();
                    let storage = manager
                        .get_storage_unchecked()
                        .await
                        .context("Backup service requires storage service")?;
                    let backup_svc = Arc::new(BackupService::new(db, storage, config));
                    manager.register_backup(backup_svc).await;
                }
                ServiceRegistrationKind::Logging(config) => {
                    let logging_svc = Arc::new(LoggingService::new(manager.clone(), config));
                    manager.register_logging(logging_svc).await;
                }
                ServiceRegistrationKind::Graphql(config) => {
                    let graphql_svc =
                        Arc::new(GraphqlService::new(manager.clone(), config.server_port));
                    manager.register_graphql(graphql_svc).await;
                }
                ServiceRegistrationKind::Http(config) => {
                    let http_svc = Arc::new(HttpServerService::new(manager.clone(), config.config));
                    manager.register(http_svc).await;
                }
                ServiceRegistrationKind::Torrent(config) => {
                    let torrent_svc = Arc::new(TorrentService::new(manager.clone(), config));
                    manager.register_torrent(torrent_svc).await;
                }
                ServiceRegistrationKind::Cast(config) => {
                    let cast_svc = Arc::new(CastService::new(Arc::downgrade(&manager), config));
                    manager.register_cast(cast_svc).await;
                }
                ServiceRegistrationKind::Sources(_config) => {
                    let sources_svc = Arc::new(SourcesService::new(manager.clone(), _config));
                    manager.register_sources(sources_svc).await;
                }
                ServiceRegistrationKind::LibraryScan(config) => {
                    let library_scan_svc =
                        Arc::new(LibraryScanService::new(manager.clone(), config));
                    manager.register_library_scan(library_scan_svc).await;
                }
                ServiceRegistrationKind::ScheduleSync(config) => {
                    manager
                        .register(Arc::new(ScheduleSyncService::new(manager.clone(), config)))
                        .await;
                }
                ServiceRegistrationKind::AutoDownload(config) => {
                    let auto_download_svc =
                        Arc::new(AutoDownloadService::new(manager.clone(), config));
                    manager.register_auto_download(auto_download_svc).await;
                }
                ServiceRegistrationKind::Transcode(config) => {
                    let transcode_svc = Arc::new(TranscodeService::new(manager.clone(), config));
                    manager.register_transcode(transcode_svc).await;
                }
                ServiceRegistrationKind::Service(svc) => {
                    manager.register(svc).await;
                }
            }
        }
        Ok(manager)
    }

    /// Build the manager, register all services, and start them in dependency order.
    /// Returns the started [ServicesManager].
    pub async fn start(self) -> Result<Arc<ServicesManager>> {
        let manager = self.build().await?;
        manager.start_all().await?;
        manager.start_supervisor();
        Ok(manager)
    }
}

impl Default for ServicesManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Global registry and lifecycle controller for services.
pub struct ServicesManager {
    services: RwLock<HashMap<String, Arc<dyn Service>>>,
    started: RwLock<HashSet<String>>,
    auth: RwLock<Option<Arc<AuthService>>>,
    database: RwLock<Option<Arc<DatabaseService>>>,
    storage: RwLock<Option<Arc<ObjectStorageService>>>,
    backup: RwLock<Option<Arc<BackupService>>>,
    logging: RwLock<Option<Arc<LoggingService>>>,
    graphql: RwLock<Option<Arc<GraphqlService>>>,
    torrent: RwLock<Option<Arc<TorrentService>>>,
    cast: RwLock<Option<Arc<CastService>>>,
    sources: RwLock<Option<Arc<SourcesService>>>,
    library_scan: RwLock<Option<Arc<LibraryScanService>>>,
    auto_download: RwLock<Option<Arc<AutoDownloadService>>>,
    transcode: RwLock<Option<Arc<TranscodeService>>>,
    /// Route builders for /api/*; used by [build_api_router]. ParkingRwLock so registration and build are sync.
    api_route_builders: ParkingRwLock<Vec<ApiRouteRegistration>>,
    supervisor: ParkingRwLock<Option<ServiceSupervisor>>,
}

struct ServiceSupervisor {
    cancel: CancellationToken,
    handle: JoinHandle<()>,
}

const SUPERVISED_BACKGROUND_SERVICES: &[&str] = &[
    "logging",
    "torrent",
    "cast",
    "library_scan",
    "auto_download",
    "transcode",
];
const SUPERVISOR_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(15);
const SUPERVISOR_MAX_CONSECUTIVE_RESTARTS: u8 = 3;

impl Default for ServicesManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ServicesManager {
    pub fn new() -> Self {
        Self {
            services: RwLock::new(HashMap::new()),
            started: RwLock::new(HashSet::new()),
            auth: RwLock::new(None),
            database: RwLock::new(None),
            storage: RwLock::new(None),
            backup: RwLock::new(None),
            logging: RwLock::new(None),
            graphql: RwLock::new(None),
            torrent: RwLock::new(None),
            cast: RwLock::new(None),
            sources: RwLock::new(None),
            library_scan: RwLock::new(None),
            auto_download: RwLock::new(None),
            transcode: RwLock::new(None),
            api_route_builders: ParkingRwLock::new(Vec::new()),
            supervisor: ParkingRwLock::new(None),
        }
    }

    /// Start a bounded health supervisor for background services.
    ///
    /// Only health messages that explicitly identify an unexpected worker exit
    /// are restartable. Environmental degradation (for example unavailable
    /// ffmpeg or a failed discovery attempt) remains visible without creating a
    /// restart storm.
    pub fn start_supervisor(self: &Arc<Self>) {
        if self.supervisor.read().is_some() {
            return;
        }

        let cancel = CancellationToken::new();
        let worker_cancel = cancel.clone();
        let manager = Arc::downgrade(self);
        let handle = tokio::spawn(async move {
            let mut consecutive_restarts: HashMap<String, u8> = HashMap::new();
            loop {
                tokio::select! {
                    _ = worker_cancel.cancelled() => break,
                    _ = tokio::time::sleep(SUPERVISOR_CHECK_INTERVAL) => {}
                }

                let Some(manager) = manager.upgrade() else {
                    break;
                };
                for name in SUPERVISED_BACKGROUND_SERVICES {
                    let service = {
                        let services = manager.services.read().await;
                        services.get(*name).cloned()
                    };
                    let Some(service) = service else {
                        continue;
                    };

                    let health = match service.health().await {
                        Ok(health) => health,
                        Err(error) => {
                            warn!(
                                service = *name,
                                error = %error,
                                "Background service health check failed"
                            );
                            continue;
                        }
                    };

                    if !health_requires_restart(&health) {
                        consecutive_restarts.remove(*name);
                        continue;
                    }

                    let attempts = consecutive_restarts.entry((*name).to_string()).or_default();
                    if *attempts >= SUPERVISOR_MAX_CONSECUTIVE_RESTARTS {
                        warn!(
                            service = *name,
                            attempts = *attempts,
                            message = ?health.message,
                            "Background service restart budget exhausted"
                        );
                        continue;
                    }

                    let attempt = *attempts + 1;
                    let backoff = std::time::Duration::from_secs(1_u64 << (*attempts).min(4));
                    tokio::select! {
                        _ = worker_cancel.cancelled() => return,
                        _ = tokio::time::sleep(backoff) => {}
                    }

                    match manager.restart_one(name).await {
                        Ok(()) => {
                            *attempts = attempt;
                            warn!(
                                service = *name,
                                attempt,
                                max_attempts = SUPERVISOR_MAX_CONSECUTIVE_RESTARTS,
                                "Restarted background service after an unexpected worker exit"
                            );
                        }
                        Err(error) => {
                            *attempts = attempt;
                            warn!(
                                service = *name,
                                attempt,
                                max_attempts = SUPERVISOR_MAX_CONSECUTIVE_RESTARTS,
                                error = %error,
                                "Background service restart failed"
                            );
                        }
                    }
                }
            }
        });
        *self.supervisor.write() = Some(ServiceSupervisor { cancel, handle });
        info!(
            services = ?SUPERVISED_BACKGROUND_SERVICES,
            max_consecutive_restarts = SUPERVISOR_MAX_CONSECUTIVE_RESTARTS,
            "Background service supervisor started"
        );
    }

    async fn stop_supervisor(&self) {
        let supervisor = self.supervisor.write().take();
        if let Some(supervisor) = supervisor {
            supervisor.cancel.cancel();
            let _ = supervisor.handle.await;
            info!("Background service supervisor stopped");
        }
    }

    /// Register a route builder for `/api/*`. Called by the builder when
    /// [add_api_routes](ServicesManagerBuilder::add_api_routes) was used.
    pub fn register_api_routes(
        &self,
        name: String,
        builder: Box<dyn Fn(AppState) -> Router<AppState> + Send + Sync>,
    ) {
        self.api_route_builders.write().push((name, builder));
    }

    /// Build the merged `/api` router from all registered route builders.
    /// Used by [crate::app::api_router] when the HTTP server builds the app.
    pub fn build_api_router(&self, state: AppState) -> Router<AppState> {
        let builders = self.api_route_builders.read();
        let mut api = Router::new();
        for (name, f) in builders.iter() {
            api = api.merge(f(state.clone()));
            tracing::debug!(api_routes = %name, "Merged API route builder");
        }
        api
    }

    /// Create a builder to add services with configs and then build/start, Bevy-plugin style.
    pub fn builder() -> ServicesManagerBuilder {
        ServicesManagerBuilder::new()
    }

    /// Compute start order from dependencies (topological order). Returns an error on unknown deps or cycles.
    async fn start_order(&self) -> Result<Vec<String>> {
        let guard = self.services.read().await;
        let names: HashSet<String> = guard.keys().cloned().collect();
        let mut deps: HashMap<String, Vec<String>> = HashMap::new();
        for (name, svc) in guard.iter() {
            let d = svc.dependencies();
            for dep in &d {
                if !names.contains(dep) {
                    anyhow::bail!(
                        "Service {} depends on {} which is not registered",
                        name,
                        dep
                    );
                }
            }
            deps.insert(name.clone(), d);
        }
        drop(guard);

        // Kahn's algorithm: start order = topological order (dependencies first).
        let mut in_degree: HashMap<String, usize> = deps
            .iter()
            .map(|(name, d)| (name.clone(), d.len()))
            .collect();
        let mut dependent_on: HashMap<String, Vec<String>> =
            names.iter().map(|n| (n.clone(), Vec::new())).collect();
        for (name, d) in &deps {
            for dep in d {
                dependent_on
                    .get_mut(dep)
                    .ok_or_else(|| anyhow::anyhow!("Service dependency graph lost node {dep}"))?
                    .push(name.clone());
            }
        }
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(n, _)| n.clone())
            .collect();
        let mut order = Vec::with_capacity(names.len());
        while let Some(n) = queue.pop() {
            order.push(n.clone());
            for s in dependent_on.get(&n).unwrap_or(&vec![]) {
                let deg = in_degree
                    .get_mut(s)
                    .ok_or_else(|| anyhow::anyhow!("Service dependency graph lost node {s}"))?;
                *deg -= 1;
                if *deg == 0 {
                    queue.push(s.clone());
                }
            }
        }
        if order.len() != names.len() {
            anyhow::bail!("Service dependency cycle detected");
        }
        Ok(order)
    }

    /// Register a service. Does not start it. If a service with the same name
    /// exists, it is replaced (the previous instance is not stopped).
    pub async fn register(&self, service: Arc<dyn Service>) {
        let name = service.name().to_string();
        if name == "database" {
            warn!(
                "Service registration mismatch for '{}': use register_database() so get_database() returns typed handle",
                name
            );
        }
        let mut guard = self.services.write().await;
        if guard.insert(name.clone(), service).is_some() {
            warn!(service = %name, "Service '{}' reregistered, overwriting previous", name);
        } else {
            info!(service = %name, "Service '{}' registered", name);
        }
    }

    /// Register the database service. This is the only way to make [get_database] return a value.
    /// The database service is the single owner of the connection pool; all other code should
    /// obtain the pool via [get_database](ServicesManager::get_database) and handle [None] when
    /// the database is stopped or unavailable.
    pub async fn register_database(&self, service: Arc<DatabaseService>) {
        let name = service.name().to_string();
        *self.database.write().await = Some(service.clone());
        let mut guard = self.services.write().await;
        if guard.insert(name.clone(), service).is_some() {
            warn!(service = %name, "Service '{}' reregistered, overwriting previous", name);
        } else {
            info!(service = %name, "Service '{}' registered", name);
        }
    }

    /// Register the auth service so [get_auth](ServicesManager::get_auth) works.
    pub async fn register_auth(&self, service: Arc<AuthService>) {
        let name = service.name().to_string();
        *self.auth.write().await = Some(service.clone());
        let mut guard = self.services.write().await;
        if guard.insert(name.clone(), service).is_some() {
            warn!(service = %name, "Service '{}' reregistered, overwriting previous", name);
        } else {
            info!(service = %name, "Service '{}' registered", name);
        }
    }

    /// Return the auth service if it is registered and currently **started**.
    pub async fn get_auth(&self) -> Option<Arc<AuthService>> {
        if !self.started.read().await.contains("auth") {
            return None;
        }
        self.auth.read().await.clone()
    }

    /// Return the database service if it is registered and currently **started**.
    /// Returns [None] when the database service is not registered, or when it is stopped,
    /// so callers can avoid using the pool when the database is unavailable (and e.g. return
    /// 503 or skip DB-dependent work).
    pub async fn get_database(&self) -> Option<Arc<DatabaseService>> {
        if !self.started.read().await.contains("database") {
            return None;
        }
        self.database.read().await.clone()
    }

    /// Return the database service if registered, regardless of started state.
    /// Prefer [get_database](ServicesManager::get_database) for normal use so callers see
    /// [None] when the database is stopped.
    pub async fn get_database_unchecked(&self) -> Option<Arc<DatabaseService>> {
        self.database.read().await.clone()
    }

    /// Register the object storage service so [get_storage](ServicesManager::get_storage) works.
    pub async fn register_storage(&self, service: Arc<ObjectStorageService>) {
        let name = service.name().to_string();
        *self.storage.write().await = Some(service.clone());
        let mut guard = self.services.write().await;
        if guard.insert(name.clone(), service).is_some() {
            warn!(service = %name, "Service '{}' reregistered, overwriting previous", name);
        } else {
            info!(service = %name, "Service '{}' registered", name);
        }
    }

    /// Return the object storage service if it is registered and currently started.
    pub async fn get_storage(&self) -> Option<Arc<ObjectStorageService>> {
        if !self.started.read().await.contains("storage") {
            return None;
        }
        self.storage.read().await.clone()
    }

    /// Return the object storage service regardless of started state.
    pub async fn get_storage_unchecked(&self) -> Option<Arc<ObjectStorageService>> {
        self.storage.read().await.clone()
    }

    /// Register the backup service so [get_backup](ServicesManager::get_backup) works.
    pub async fn register_backup(&self, service: Arc<BackupService>) {
        let name = service.name().to_string();
        *self.backup.write().await = Some(service.clone());
        let mut guard = self.services.write().await;
        if guard.insert(name.clone(), service).is_some() {
            warn!(service = %name, "Service '{}' reregistered, overwriting previous", name);
        } else {
            info!(service = %name, "Service '{}' registered", name);
        }
    }

    /// Return the backup service if it is registered and currently started.
    pub async fn get_backup(&self) -> Option<Arc<BackupService>> {
        if !self.started.read().await.contains("backup") {
            return None;
        }
        self.backup.read().await.clone()
    }

    /// Register the logging service so [get_logging](ServicesManager::get_logging) works.
    pub async fn register_logging(&self, service: Arc<LoggingService>) {
        let name = service.name().to_string();
        *self.logging.write().await = Some(service.clone());
        let mut guard = self.services.write().await;
        if guard.insert(name.clone(), service).is_some() {
            warn!(service = %name, "Service '{}' reregistered, overwriting previous", name);
        } else {
            info!(service = %name, "Service '{}' registered", name);
        }
    }

    /// Return the logging service if it is registered and currently **started**.
    /// Use [LoggingService::tracing_layer] to get the layer for tracing_subscriber.
    pub async fn get_logging(&self) -> Option<Arc<LoggingService>> {
        if !self.started.read().await.contains("logging") {
            return None;
        }
        self.logging.read().await.clone()
    }

    /// Register the GraphQL service so [get_graphql](ServicesManager::get_graphql) works.
    pub async fn register_graphql(&self, service: Arc<GraphqlService>) {
        let name = service.name().to_string();
        *self.graphql.write().await = Some(service.clone());
        let mut guard = self.services.write().await;
        if guard.insert(name.clone(), service).is_some() {
            warn!(service = %name, "Service '{}' reregistered, overwriting previous", name);
        } else {
            info!(service = %name, "Service '{}' registered", name);
        }
    }

    /// Return the GraphQL service if it is registered and currently **started**.
    /// Use [GraphqlService::schema] and [GraphqlService::router] to build app state and routes.
    pub async fn get_graphql(&self) -> Option<Arc<GraphqlService>> {
        if !self.started.read().await.contains("graphql") {
            return None;
        }
        self.graphql.read().await.clone()
    }

    /// Register the torrent service so [get_torrent](ServicesManager::get_torrent) works.
    pub async fn register_torrent(&self, service: Arc<TorrentService>) {
        let name = service.name().to_string();
        *self.torrent.write().await = Some(service.clone());
        let mut guard = self.services.write().await;
        if guard.insert(name.clone(), service).is_some() {
            warn!(service = %name, "Service '{}' reregistered, overwriting previous", name);
        } else {
            info!(service = %name, "Service '{}' registered", name);
        }
    }

    /// Return the torrent service if it is registered and currently **started**.
    pub async fn get_torrent(&self) -> Option<Arc<TorrentService>> {
        if !self.started.read().await.contains("torrent") {
            return None;
        }
        self.torrent.read().await.clone()
    }

    /// Register the cast service so [get_cast](ServicesManager::get_cast) works.
    pub async fn register_cast(&self, service: Arc<CastService>) {
        let name = service.name().to_string();
        *self.cast.write().await = Some(service.clone());
        let mut guard = self.services.write().await;
        if guard.insert(name.clone(), service).is_some() {
            warn!(service = %name, "Service '{}' reregistered, overwriting previous", name);
        } else {
            info!(service = %name, "Service '{}' registered", name);
        }
    }

    /// Return the cast service if it is registered and currently **started**.
    pub async fn get_cast(&self) -> Option<Arc<CastService>> {
        if !self.started.read().await.contains("cast") {
            return None;
        }
        self.cast.read().await.clone()
    }

    /// Register the sources service so [get_sources](ServicesManager::get_sources) works.
    pub async fn register_sources(&self, service: Arc<SourcesService>) {
        let name = service.name().to_string();
        *self.sources.write().await = Some(service.clone());
        let mut guard = self.services.write().await;
        if guard.insert(name.clone(), service).is_some() {
            warn!(service = %name, "Service '{}' reregistered, overwriting previous", name);
        } else {
            info!(service = %name, "Service '{}' registered", name);
        }
    }

    /// Return the sources service if it is registered and currently **started**.
    pub async fn get_sources(&self) -> Option<Arc<SourcesService>> {
        if !self.started.read().await.contains("sources") {
            return None;
        }
        self.sources.read().await.clone()
    }

    /// Register the library scan service so [get_library_scan](ServicesManager::get_library_scan) works.
    pub async fn register_library_scan(&self, service: Arc<LibraryScanService>) {
        let name = service.name().to_string();
        *self.library_scan.write().await = Some(service.clone());
        let mut guard = self.services.write().await;
        if guard.insert(name.clone(), service).is_some() {
            warn!(service = %name, "Service '{}' reregistered, overwriting previous", name);
        } else {
            info!(service = %name, "Service '{}' registered", name);
        }
    }

    /// Return the library scan service if it is registered and currently **started**.
    pub async fn get_library_scan(&self) -> Option<Arc<LibraryScanService>> {
        if !self.started.read().await.contains("library_scan") {
            return None;
        }
        self.library_scan.read().await.clone()
    }

    /// Return the library scan service regardless of started state.
    pub async fn get_library_scan_unchecked(&self) -> Option<Arc<LibraryScanService>> {
        self.library_scan.read().await.clone()
    }

    /// Register the auto-download service so [get_auto_download](ServicesManager::get_auto_download) works.
    pub async fn register_auto_download(&self, service: Arc<AutoDownloadService>) {
        let name = service.name().to_string();
        *self.auto_download.write().await = Some(service.clone());
        let mut guard = self.services.write().await;
        if guard.insert(name.clone(), service).is_some() {
            warn!(service = %name, "Service '{}' reregistered, overwriting previous", name);
        } else {
            info!(service = %name, "Service '{}' registered", name);
        }
    }

    /// Return the auto-download service if it is registered and currently **started**.
    pub async fn get_auto_download(&self) -> Option<Arc<AutoDownloadService>> {
        if !self.started.read().await.contains("auto_download") {
            return None;
        }
        self.auto_download.read().await.clone()
    }

    /// Return the auto-download service regardless of started state.
    pub async fn get_auto_download_unchecked(&self) -> Option<Arc<AutoDownloadService>> {
        self.auto_download.read().await.clone()
    }

    /// Register the transcode service so [get_transcode](ServicesManager::get_transcode) works.
    pub async fn register_transcode(&self, service: Arc<TranscodeService>) {
        let name = service.name().to_string();
        *self.transcode.write().await = Some(service.clone());
        let mut guard = self.services.write().await;
        if guard.insert(name.clone(), service).is_some() {
            warn!(service = %name, "Service '{}' reregistered, overwriting previous", name);
        } else {
            info!(service = %name, "Service '{}' registered", name);
        }
    }

    /// Return the transcode service if it is registered and currently **started**.
    pub async fn get_transcode(&self) -> Option<Arc<TranscodeService>> {
        if !self.started.read().await.contains("transcode") {
            return None;
        }
        self.transcode.read().await.clone()
    }

    /// Return the transcode service regardless of started state.
    pub async fn get_transcode_unchecked(&self) -> Option<Arc<TranscodeService>> {
        self.transcode.read().await.clone()
    }

    /// Unregister a service by name. Does not stop it; call [stop_one](ServicesManager::stop_one)
    /// before unregistering if it is running. Removes it from the started set.
    /// Returns the previous service if present. Clears typed handles for "auth", "database", "logging", "graphql".
    pub async fn unregister(&self, name: &str) -> Option<Arc<dyn Service>> {
        self.started.write().await.remove(name);
        if name == "auth" {
            *self.auth.write().await = None;
        }
        if name == "database" {
            *self.database.write().await = None;
        }
        if name == "storage" {
            *self.storage.write().await = None;
        }
        if name == "backup" {
            *self.backup.write().await = None;
        }
        if name == "logging" {
            *self.logging.write().await = None;
        }
        if name == "graphql" {
            *self.graphql.write().await = None;
        }
        if name == "torrent" {
            *self.torrent.write().await = None;
        }
        if name == "cast" {
            *self.cast.write().await = None;
        }
        if name == "sources" {
            *self.sources.write().await = None;
        }
        if name == "library_scan" {
            *self.library_scan.write().await = None;
        }
        if name == "auto_download" {
            *self.auto_download.write().await = None;
        }
        if name == "transcode" {
            *self.transcode.write().await = None;
        }
        let mut guard = self.services.write().await;
        let out = guard.remove(name);
        if out.is_some() {
            info!(service = %name, "Service '{}' unregistered", name);
        }
        out
    }

    /// Start all registered services in dependency order (dependencies first).
    /// A service is only started after all of its [dependencies](Service::dependencies) are started.
    /// Returns an error on unknown dependency, cycle, or if any start fails.
    pub async fn start_all(&self) -> Result<()> {
        let order = self.start_order().await?;
        for name in &order {
            let svc = {
                let g = self.services.read().await;
                g.get(name).cloned()
            };
            if let Some(s) = svc {
                if let Err(e) = s.start().await {
                    warn!(service = %name, error = %e, "Service '{}' start failed", name);
                    return Err(e).context(format!("failed to start service {}", name));
                }
                self.started.write().await.insert(name.clone());
                info!(service = %name, "Service '{}' started", name);
            }
        }
        Ok(())
    }

    /// Stop all registered services in reverse dependency order (dependents first).
    pub async fn stop_all(&self) -> Result<()> {
        self.stop_supervisor().await;
        let order = self.start_order().await?;
        for name in order.into_iter().rev() {
            let svc = {
                let g = self.services.read().await;
                g.get(&name).cloned()
            };
            if let Some(s) = svc {
                if let Err(e) = s.stop().await {
                    warn!(service = %name, error = %e, "Service '{}' stop failed", name);
                } else {
                    info!(service = %name, "Service '{}' stopped", name);
                }
                self.started.write().await.remove(&name);
            }
        }
        Ok(())
    }

    /// Restart a single service by name. Returns an error if the service is not
    /// registered or if restart fails. Dependencies are not restarted.
    pub async fn restart_one(&self, name: &str) -> Result<()> {
        let svc = {
            let guard = self.services.read().await;
            guard.get(name).cloned()
        };
        match svc {
            Some(s) => {
                s.stop().await?;
                self.started.write().await.remove(name);
                s.start().await?;
                self.started.write().await.insert(name.to_string());
                info!(service = %name, "Service '{}' restarted", name);
                Ok(())
            }
            None => {
                anyhow::bail!("Service not found: {}", name)
            }
        }
    }

    /// Restart all registered services: stop all in reverse order, then start all.
    pub async fn restart_all(self: &Arc<Self>) -> Result<()> {
        self.stop_all().await?;
        self.start_all().await?;
        self.start_supervisor();
        Ok(())
    }

    /// Stop a single service by name. Logs a warning if the service is not registered.
    /// Returns whether the service was found and stopped successfully.
    /// Does not stop dependents; stop them first or use [stop_all](ServicesManager::stop_all) for ordered shutdown.
    pub async fn stop_one(&self, name: &str) -> bool {
        let svc = {
            let guard = self.services.read().await;
            guard.get(name).cloned()
        };
        if let Some(s) = svc {
            match s.stop().await {
                Ok(()) => {
                    self.started.write().await.remove(name);
                    info!(service = %name, "Service '{}' stopped", name);
                    true
                }
                Err(e) => {
                    warn!(service = %name, error = %e, "Service '{}' stop failed", name);
                    false
                }
            }
        } else {
            warn!(service = %name, "Service '{}' not found, cannot stop", name);
            false
        }
    }

    /// Start a single service by name. All of its [dependencies](Service::dependencies) must
    /// already be started (e.g. via a prior [start_all](ServicesManager::start_all)), otherwise
    /// returns an error. Returns an error if the service is not registered or if start fails.
    pub async fn start_one(&self, name: &str) -> Result<()> {
        let (svc, deps) = {
            let guard = self.services.read().await;
            let s = guard.get(name).cloned();
            let d = s.as_ref().map(|s| s.dependencies()).unwrap_or_default();
            (s, d)
        };
        let svc = svc.ok_or_else(|| anyhow::anyhow!("Service not found: {}", name))?;
        let started = self.started.read().await;
        for dep in &deps {
            if !started.contains(dep) {
                anyhow::bail!("Cannot start {}: dependency {} is not started", name, dep);
            }
        }
        drop(started);
        svc.start().await?;
        self.started.write().await.insert(name.to_string());
        info!(service = %name, "Service '{}' started", name);
        Ok(())
    }

    /// Health check for one service. Returns an error if the service is not registered
    /// or if the health check fails (e.g. runtime error).
    pub async fn health_one(&self, name: &str) -> Result<ServiceHealth> {
        let svc = {
            let guard = self.services.read().await;
            guard.get(name).cloned()
        };
        match svc {
            Some(s) => s.health().await,
            None => anyhow::bail!("Service not found: {}", name),
        }
    }

    /// Health check for all registered services. Returns a map of name -> health.
    /// Services that return an error from [health](Service::health) are reported as
    /// [Unhealthy](HealthStatus::Unhealthy) with the error message.
    pub async fn health_all(&self) -> HashMap<String, ServiceHealth> {
        let guard = self.services.read().await;
        let names: Vec<String> = guard.keys().cloned().collect();
        drop(guard);
        let mut out = HashMap::new();
        for name in names {
            let svc = {
                let g = self.services.read().await;
                g.get(&name).cloned()
            };
            if let Some(s) = svc {
                let h = match s.health().await {
                    Ok(h) => h,
                    Err(e) => ServiceHealth::unhealthy(e.to_string()),
                };
                out.insert(name, h);
            }
        }
        out
    }

    /// Return whether the given service is currently started (tracked by the manager).
    pub async fn is_started(&self, name: &str) -> bool {
        self.started.read().await.contains(name)
    }

    /// Return names of services that expose HTTP routes.
    pub async fn services_with_routes(&self) -> Vec<String> {
        let guard = self.services.read().await;
        guard
            .values()
            .filter(|s| s.provides_routes())
            .map(|s| s.name().to_string())
            .collect()
    }

    /// Get a registered service by name for downcast or use in app state.
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Service>> {
        let guard = self.services.read().await;
        guard.get(name).cloned()
    }

    /// List all registered service names.
    pub async fn names(&self) -> Vec<String> {
        let guard = self.services.read().await;
        guard.keys().cloned().collect()
    }
}

fn health_requires_restart(health: &ServiceHealth) -> bool {
    health.message.as_deref().is_some_and(|message| {
        message.contains("exited unexpectedly") || message.contains("exhausting restarts")
    })
}

#[cfg(test)]
mod supervisor_tests {
    use super::*;

    #[test]
    fn only_worker_exit_health_is_restartable() {
        assert!(health_requires_restart(&ServiceHealth::degraded(
            "one or more workers exited unexpectedly"
        )));
        assert!(health_requires_restart(&ServiceHealth::degraded(
            "workers stopped after exhausting restarts"
        )));
        assert!(!health_requires_restart(&ServiceHealth::degraded(
            "ffmpeg is unavailable"
        )));
        assert!(!health_requires_restart(&ServiceHealth::healthy()));
    }
}
