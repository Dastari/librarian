//! HTTP server service: binds the Axum app and runs it in a background task.
//!
//! Depends on the GraphQL service (and transitively database, auth). Start order is
//! ensured by the service manager; this service builds [AppState](crate::app::AppState)
//! and the router in [start](Service::start) and runs the server until [stop](Service::stop).

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::sync::broadcast;
use tracing::info;

use crate::app::{build_app, AppState};
use crate::config::Config;
use crate::services::manager::{Service, ServiceHealth};

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
    /// Send to trigger server shutdown; set in start(), taken in stop().
    shutdown_tx: parking_lot::RwLock<Option<broadcast::Sender<()>>>,
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
        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .context("HTTP server: bind failed")?;

        let (shutdown_tx, _) = broadcast::channel::<()>(1);
        let mut shutdown_rx = shutdown_tx.subscribe();

        let serve_fut = axum::serve(listener, app);
        let join = tokio::spawn(async move {
            tokio::select! {
                result = serve_fut => result.context("axum::serve"),
                _ = shutdown_rx.recv() => Ok(()),
            }
        });

        *self.join_handle.write() = Some(join);
        *self.shutdown_tx.write() = Some(shutdown_tx);

        info!(service = "http", "HTTP server service started");
        info!(
            service = "http",
            "Listening on http://{}; GraphQL: http://localhost:{}/graphql",
            addr,
            self.config.port
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let tx = self.shutdown_tx.write().take();
        let handle = self.join_handle.write().take();
        drop(tx); // send (or drop) to unblock the server task's recv
        if let Some(h) = handle {
            let _ = h.await;
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
