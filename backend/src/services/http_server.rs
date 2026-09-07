//! HTTP server service: binds the Axum app and runs it in a background task.
//!
//! Depends on the GraphQL service (and transitively database, auth). Start order is
//! ensured by the service manager; this service builds [AppState](crate::app::AppState)
//! and the router in [start](Service::start) and runs the server until [stop](Service::stop).

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::sync::oneshot;
use tracing::info;

use crate::app::{AppState, build_app};
use crate::config::Config;
use crate::services::manager::{Service, ServiceHealth};

/// How long to wait for in-flight connections (e.g. GraphQL subscriptions held open by
/// websockets) to drain after graceful shutdown is signaled, before giving up and letting
/// the server task be abandoned.
const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

/// Configuration for the HTTP server service (port and app config).
#[derive(Clone)]
pub struct HttpServerConfig {
    pub config: Arc<Config>,
}

/// HTTP server service: binds and serves the Axum app in a background task.
pub struct HttpServerService {
    manager: Arc<crate::services::ServicesManager>,
    config: Arc<Config>,
    /// JoinHandle for the server task; set in start(), taken in stop().
    join_handle: parking_lot::RwLock<Option<tokio::task::JoinHandle<Result<()>>>>,
    /// Send to trigger graceful server shutdown; set in start(), taken in stop().
    shutdown_tx: parking_lot::RwLock<Option<oneshot::Sender<()>>>,
}

impl HttpServerService {
    /// Create the service. Register with the manager (e.g. via builder) and call
    /// [start_all](crate::services::ServicesManager::start_all); [start](Service::start)
    /// will build the app and spawn the server task.
    pub fn new(manager: Arc<crate::services::ServicesManager>, config: Arc<Config>) -> Self {
        Self {
            manager,
            config,
            join_handle: parking_lot::RwLock::new(None),
            shutdown_tx: parking_lot::RwLock::new(None),
        }
    }

    /// Resolve the address to bind to from `config.host`. Falls back to `0.0.0.0`
    /// (previous hardcoded behavior) when host is unset/empty, or if set but not a
    /// parseable IP address (logged as a warning rather than failing startup).
    fn bind_ip(&self) -> IpAddr {
        const FALLBACK: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

        match self.config.host.as_deref().map(str::trim) {
            Some(host) if !host.is_empty() => host.parse::<IpAddr>().unwrap_or_else(|err| {
                tracing::warn!(
                    host = %host,
                    error = %err,
                    "Invalid HOST value; falling back to 0.0.0.0"
                );
                FALLBACK
            }),
            _ => FALLBACK,
        }
    }
}

#[async_trait]
impl Service for HttpServerService {
    fn name(&self) -> &str {
        "http"
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["graphql".to_string()]
    }

    async fn start(&self) -> Result<()> {
        info!(service = "http", "HTTP server service starting");

        let db = self
            .manager
            .get_database()
            .await
            .map(|s| s.pool().clone())
            .ok_or_else(|| anyhow::anyhow!("database service not available"))?;
        let gql = self
            .manager
            .get_graphql()
            .await
            .ok_or_else(|| anyhow::anyhow!("graphql service not available"))?;
        let schema = gql
            .schema()
            .await
            .ok_or_else(|| anyhow::anyhow!("graphql schema not built"))?;

        let state = AppState {
            config: self.config.clone(),
            db,
            schema,
            services: self.manager.clone(),
        };

        let app = build_app(state).await;
        let bind_ip = self.bind_ip();
        let addr = SocketAddr::new(bind_ip, self.config.port);
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .context("HTTP server: bind failed")?;

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        // `with_graceful_shutdown` lets in-flight requests (and axum::serve's own
        // connection-close handshake) complete instead of abandoning them mid-request
        // the moment shutdown is signaled.
        let serve_fut = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });
        let join = tokio::spawn(async move { serve_fut.await.context("axum::serve") });

        *self.join_handle.write() = Some(join);
        *self.shutdown_tx.write() = Some(shutdown_tx);

        info!(service = "http", "HTTP server service started");
        info!(
            service = "http",
            "Listening on http://{}; GraphQL: http://localhost:{}/graphql", addr, self.config.port
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let tx = self.shutdown_tx.write().take();
        let handle = self.join_handle.write().take();
        if let Some(tx) = tx {
            let _ = tx.send(()); // signal with_graceful_shutdown to start draining
        }
        if let Some(h) = handle {
            let abort_handle = h.abort_handle();
            if tokio::time::timeout(GRACEFUL_SHUTDOWN_TIMEOUT, h)
                .await
                .is_err()
            {
                tracing::warn!(
                    service = "http",
                    timeout_secs = GRACEFUL_SHUTDOWN_TIMEOUT.as_secs(),
                    "HTTP server graceful shutdown timed out (likely a held-open websocket/subscription); \
                     abandoning remaining in-flight connections"
                );
                abort_handle.abort();
            }
        }
        info!(service = "http", "HTTP server service stopped");
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        if self.join_handle.read().is_some() {
            Ok(ServiceHealth::healthy())
        } else {
            Ok(ServiceHealth::unhealthy("server task not running"))
        }
    }

    fn provides_routes(&self) -> bool {
        true
    }
}
