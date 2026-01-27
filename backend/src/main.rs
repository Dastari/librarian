//! Librarian Backend - Rust-powered media library service
//!
//! This is the main entry point for the Librarian backend API.
//! All operations are exposed via GraphQL at /graphql.
//! The HTTP server and GraphQL start regardless of TUI; run with TUI for a dashboard or headless for API-only.

#![recursion_limit = "512"]

mod app;
mod app_mode;
mod cli;
mod config;
mod db;
mod services;

#[cfg(feature = "embed-frontend")]
mod static_assets;

pub use crate::services::graphql;
mod tui;

use std::sync::Arc;
use std::time::Duration;

use crate::cli::CliOptions;
use crate::config::Config;
use crate::db::Database;
use crate::services::logging::{DbLayerState, OptionalDbLayer};
use crate::services::{
    AuthConfig, DatabaseServiceConfig, GraphqlServiceConfig, HttpServerConfig,
    LoggingServiceConfig, ServicesManager, torrent::TorrentServiceConfig,
};
use std::path::PathBuf;
use crate::tui::{create_tui_layer, should_use_tui, TuiApp, TuiConfig};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub use app::{api_router, build_app, AppState};

pub async fn get_db_pool(services: &ServicesManager) -> Option<Database> {
    services.get_database().await.map(|svc| svc.pool().clone())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let cli = CliOptions::from_args();
    let config = Config::from_env()?;
    let config = Arc::new(config);

    let use_tui = should_use_tui();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "librarian=info,tower_http=info,librqbit=info".into());

    let db_layer_state: DbLayerState = Arc::new(std::sync::Mutex::new(None));
    let optional_db_layer = OptionalDbLayer::new(Arc::clone(&db_layer_state));

    let log_rx = if use_tui {
        let (tui_layer, rx) = create_tui_layer(tracing::Level::INFO);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tui_layer)
            .with(optional_db_layer)
            .init();
        Some(rx)
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .with(optional_db_layer)
            .init();
        None
    };

    install_rustls_crypto_provider();
    tracing::info!("Starting Librarian Backend");
    if cfg!(windows) {
        let run_mode = cli.run_mode_override.unwrap_or(config.run_mode);
        tracing::info!(?run_mode, "Windows run mode selected");
    }

    let logging_config = LoggingServiceConfig {
        db_layer_state: Some(db_layer_state),
        ..LoggingServiceConfig::default()
    };

    let services = ServicesManager::builder()
        .add_service(DatabaseServiceConfig {
            database_url: config.database_url.clone(),
            connect_timeout: Duration::from_secs(30),
        })
        .add_service(logging_config)
        .add_service(AuthConfig::from_env())
        .add_service(GraphqlServiceConfig {
            server_port: config.port,
        })
        .add_service(TorrentServiceConfig {
            download_dir: PathBuf::from(&config.downloads_path),
            session_dir: PathBuf::from(&config.session_path),
            enable_dht: config.torrent_enable_dht,
            listen_port: config.torrent_listen_port,
            max_concurrent: config.torrent_max_concurrent,
        })
        .add_service(HttpServerConfig {
            config: config.clone(),
        })
        .start()
        .await?;

    if use_tui {
        let tui = TuiApp::new(
            log_rx.expect("log_rx set when use_tui"),
            config.port,
            TuiConfig::default(),
        )?;
        tui.run().await?;
    } else {
        tokio::signal::ctrl_c().await?;
    }

    services.stop_all().await?;
    Ok(())
}

fn install_rustls_crypto_provider() {
    if rustls::crypto::ring::default_provider()
        .install_default()
        .is_err()
    {
        tracing::debug!("Rustls crypto provider already configured");
    }
}
