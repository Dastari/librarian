//! Librarian Backend - Rust-powered media library service
//!
//! This is the main entry point for the Librarian backend API.
//! All operations are exposed via GraphQL at /graphql.
//! The HTTP server and GraphQL start regardless of TUI; run with TUI for a dashboard or headless for API-only.

#![recursion_limit = "512"]

mod api;
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
    LoggingServiceConfig, ServicesManager, cast::service::CastServiceConfig,
    library_scan::LibraryScanServiceConfig, sources::service::SourcesServiceConfig,
    torrent::TorrentServiceConfig,
};
use crate::tui::{TuiApp, TuiConfig, create_tui_layer, should_use_tui};
use anyhow::Context;
use std::path::PathBuf;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub use app::{AppState, api_router, build_app};

pub async fn get_db_pool(services: &ServicesManager) -> Option<Database> {
    services.get_database().await.map(|svc| svc.pool().clone())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    raise_fd_limit();
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
    let cast_base_host = config
        .host
        .clone()
        .filter(|h| !h.trim().is_empty() && h != "0.0.0.0")
        .or_else(|| local_ip_address::local_ip().ok().map(|ip| ip.to_string()))
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let cast_media_base_url = format!("http://{}:{}", cast_base_host, config.port);

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
            upload_limit: 0,
            download_limit: 0,
        })
        .add_service(CastServiceConfig {
            media_base_url: cast_media_base_url,
            auto_discovery: true,
            discovery_interval_secs: 30,
            discovery_timeout_ms: 1500,
        })
        .add_service(SourcesServiceConfig::default())
        .add_service(LibraryScanServiceConfig::default())
        .add_service(HttpServerConfig {
            config: config.clone(),
        })
        .add_api_routes("artwork", |_| crate::api::artwork::router())
        .add_api_routes("health", |_| crate::api::health::router())
        .add_api_routes("media", |_| crate::api::media::router())
        .start()
        .await?;

    if let Some(db_service) = services.get_database().await {
        let db = db_service.pool().clone();
        let services_for_reconnect = services.clone();
        tokio::spawn(async move {
            tracing::info!("Starting background reconnect for saved network paths");
            crate::services::graphql::filesystem_network::reconnect_saved_network_paths(
                &db,
                &services_for_reconnect,
            )
            .await;
            tracing::info!("Finished background reconnect for saved network paths");
        });
    }

    if use_tui {
        let torrent_service = services
            .get_torrent()
            .await
            .context("torrent service unavailable for TUI")?;
        let graphql_schema = services
            .get_graphql()
            .await
            .context("graphql service unavailable for TUI")?
            .schema()
            .await
            .context("graphql schema unavailable for TUI")?;
        let db_pool = services
            .get_database()
            .await
            .context("database service unavailable for TUI")?
            .pool()
            .clone();
        let tui = TuiApp::new(
            log_rx.expect("log_rx set when use_tui"),
            graphql_schema,
            torrent_service,
            db_pool,
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

/// Raise the process soft file-descriptor limit to the hard limit (or at least 65536).
///
/// librqbit creates per-peer sockets, each needing ~4 kernel FDs (socket + eventpoll
/// + eventfd + timerfd). The default soft limit of 1024 on many Linux systems is
/// easily exhausted by a handful of active torrents. Raising it early avoids
/// "unable to open database file" (SQLITE_CANTOPEN) errors that appear once FDs run out.
fn raise_fd_limit() {
    #[cfg(unix)]
    {
        use libc::{RLIMIT_NOFILE, getrlimit, rlimit, setrlimit};

        unsafe {
            let mut rl = rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            if getrlimit(RLIMIT_NOFILE, &mut rl) != 0 {
                eprintln!("warning: getrlimit(RLIMIT_NOFILE) failed");
                return;
            }

            let desired: u64 = 65_536;
            let target = desired.min(rl.rlim_max).max(rl.rlim_cur);

            if target > rl.rlim_cur {
                let prev = rl.rlim_cur;
                rl.rlim_cur = target;
                if setrlimit(RLIMIT_NOFILE, &rl) != 0 {
                    eprintln!(
                        "warning: setrlimit(RLIMIT_NOFILE, {}) failed; current soft limit is {}",
                        target, prev
                    );
                } else {
                    eprintln!("Raised file descriptor soft limit: {} -> {}", prev, target);
                }
            }
        }
    }
}
