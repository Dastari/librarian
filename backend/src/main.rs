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
mod jobs;
mod platform;
mod services;

#[cfg(feature = "embed-frontend")]
mod static_assets;

pub use crate::services::graphql;
mod tui;

use std::sync::Arc;
use std::time::Duration;

use crate::cli::{CliOptions, CredentialCommand, SchemaCommand};
use crate::config::Config;
use crate::db::Database;
use crate::jobs::AutoDownloadServiceConfig;
use crate::services::logging::{DbLayerState, OptionalDbLayer};
use crate::services::{
    AuthConfig, BackupServiceConfig, DatabaseService, DatabaseServiceConfig, GraphqlServiceConfig,
    HttpServerConfig, LoggingServiceConfig, ObjectStorageServiceConfig, ServicesManager,
    cast::service::CastServiceConfig, library_scan::LibraryScanServiceConfig,
    sources::service::SourcesServiceConfig, torrent::TorrentServiceConfig,
    transcode::TranscodeServiceConfig,
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

fn main() -> anyhow::Result<()> {
    let worker_threads = runtime_thread_count(
        "LIBRARIAN_TOKIO_WORKER_THREADS",
        default_tokio_worker_threads(),
        1,
        64,
    );
    let max_blocking_threads =
        runtime_thread_count("LIBRARIAN_TOKIO_MAX_BLOCKING_THREADS", 32, 1, 128);

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .max_blocking_threads(max_blocking_threads)
        .thread_name("librarian-runtime")
        .enable_all()
        .thread_stack_size(8 * 1024 * 1024)
        .build()
        .context("failed to build Tokio runtime")?
        .block_on(async_main())
}

fn default_tokio_worker_threads() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(4)
        .clamp(2, 8)
}

fn runtime_thread_count(env_key: &str, default: usize, min: usize, max: usize) -> usize {
    std::env::var(env_key)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
        .clamp(min, max)
}

async fn async_main() -> anyhow::Result<()> {
    raise_fd_limit();
    dotenvy::dotenv().ok();
    let cli = CliOptions::from_args()?;
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
    if !crate::config::dev_mode() && !config.secure_cookies && config.trusted_proxies.is_empty() {
        tracing::warn!(
            "Production auth cookies are not configured as Secure. Set \
             LIBRARIAN_SECURE_COOKIES=true for HTTPS deployments, or configure \
             LIBRARIAN_TRUSTED_PROXIES with the exact reverse-proxy IP/CIDR."
        );
    }
    if !config.trusted_proxies.is_empty() {
        tracing::info!(
            trusted_proxy_count = config.trusted_proxies.len(),
            "Forwarded scheme headers will only be accepted from configured trusted proxies"
        );
    }
    if let Some(command) = cli.schema_command {
        return run_schema_command(config.as_ref(), command).await;
    }
    if let Some(command) = cli.credential_command {
        return run_credential_command(config.as_ref(), command).await;
    }
    let run_mode = cli.run_mode_override.unwrap_or(config.run_mode);
    crate::platform::windows::ensure_mode_supported(run_mode)?;
    tracing::info!(?run_mode, "Application run mode selected");

    let logging_config = LoggingServiceConfig {
        db_layer_state: Some(db_layer_state),
        ..LoggingServiceConfig::default()
    };
    let cast_media_base_url = if let Some(url) = config.advertised_media_url.clone() {
        url
    } else {
        let cast_base_host = config
            .host
            .clone()
            .filter(|h| !h.trim().is_empty() && h != "0.0.0.0" && h != "::")
            .or_else(|| local_ip_address::local_ip().ok().map(|ip| ip.to_string()))
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let cast_base_host = cast_base_host
            .parse::<std::net::IpAddr>()
            .map(|address| match address {
                std::net::IpAddr::V4(_) => address.to_string(),
                std::net::IpAddr::V6(_) => format!("[{address}]"),
            })
            .unwrap_or(cast_base_host);
        tracing::warn!(
            cast_base_host,
            "LIBRARIAN_ADVERTISED_MEDIA_URL is unset; Cast media URL was auto-detected and may be wrong on multi-interface hosts"
        );
        format!("http://{}:{}", cast_base_host, config.port)
    };
    let auth_config = AuthConfig::from_env()?;

    let services = ServicesManager::builder()
        .add_service(DatabaseServiceConfig {
            database_url: config.database_url.clone(),
            connect_timeout: Duration::from_secs(30),
        })
        .add_service(ObjectStorageServiceConfig {
            backend: config.storage_backend.clone(),
            local_path: PathBuf::from(&config.storage_path),
        })
        .add_service(logging_config)
        .add_service(auth_config.clone())
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
            // Seeding policy is settings-driven; these are only the
            // pre-`app_settings` defaults (see `bootstrap_defaults.rs`).
            ..TorrentServiceConfig::default()
        })
        .add_service(CastServiceConfig {
            media_base_url: cast_media_base_url,
            auto_discovery: config.cast_auto_discovery,
            discovery_interval_secs: config.cast_discovery_interval_secs,
            discovery_timeout_ms: config.cast_discovery_timeout_ms,
            discovery_retention_days: config.cast_discovery_retention_days,
            grant_secret: auth_config.jwt_secret.as_bytes().to_vec(),
        })
        .add_service(SourcesServiceConfig::from_env()?)
        .add_service(LibraryScanServiceConfig::default())
        .add_service(AutoDownloadServiceConfig::default())
        .add_service(crate::jobs::schedule_sync::ScheduleSyncConfig::from_env()?)
        .add_service(TranscodeServiceConfig {
            cache_path: PathBuf::from(&config.cache_path),
            ffmpeg_path: config
                .ffmpeg_path
                .clone()
                .unwrap_or_else(|| "ffmpeg".to_string()),
            ..TranscodeServiceConfig::default()
        })
        .add_service(BackupServiceConfig {
            repository_path: PathBuf::from(&config.backup_path),
        })
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
            .pool()
            .clone();
        let tui_log_rx = log_rx.context("TUI log channel was not initialized")?;
        let tui = TuiApp::new(
            tui_log_rx,
            graphql_schema,
            torrent_service,
            db_pool,
            config.port,
            TuiConfig::default(),
        )?;
        tui.run().await?;
    } else {
        wait_for_shutdown_signal().await?;
    }

    services.stop_all().await?;
    Ok(())
}

async fn run_schema_command(config: &Config, command: SchemaCommand) -> anyhow::Result<()> {
    let database = DatabaseService::from_config(
        DatabaseServiceConfig {
            database_url: config.database_url.clone(),
            connect_timeout: Duration::from_secs(30),
        },
        None,
    )
    .await
    .context("Failed to connect for schema administration")?;

    match command {
        SchemaCommand::Plan => {
            if let Some(plan) = crate::services::database::plan_schema(database.pool()).await? {
                println!("{}", crate::services::database::format_schema_plan(&plan));
                println!(
                    "\nNo changes were applied. Create and verify a full backup before using \
                     `librarian schema apply` for a non-additive plan."
                );
            } else {
                println!("Database schema is current; no migration plan is pending.");
            }
        }
        SchemaCommand::Apply {
            plan_hash,
            backup_snapshot_id,
        } => {
            let plan = crate::services::database::plan_schema(database.pool())
                .await?
                .context("Database schema is current; no migration plan is pending")?;
            let source_schema_hash = plan
                .source_schema_hash
                .as_deref()
                .context("Schema plan does not contain a source schema hash")?;
            let verified_backup = crate::services::backup::verify_schema_migration_backup(
                PathBuf::from(&config.backup_path),
                &backup_snapshot_id,
                source_schema_hash,
            )
            .await?;
            let report = crate::services::database::apply_reviewed_schema_plan(
                database.pool(),
                &plan_hash,
                &verified_backup,
            )
            .await?
            .context("Database schema became current before the reviewed plan was applied")?;
            println!(
                "Applied schema migration {} using verified backup {} ({} statements).",
                report.version,
                verified_backup.snapshot_id(),
                report.statements_applied
            );
        }
    }

    database.pool().pool().close().await;
    Ok(())
}

async fn run_credential_command(config: &Config, command: CredentialCommand) -> anyhow::Result<()> {
    let database = DatabaseService::from_config(
        DatabaseServiceConfig {
            database_url: config.database_url.clone(),
            connect_timeout: Duration::from_secs(30),
        },
        None,
    )
    .await
    .context("Failed to connect for source credential administration")?;
    let source_config = SourcesServiceConfig::from_env()?;

    let (new_key_file, backup_snapshot_id) = match &command {
        CredentialCommand::Plan { new_key_file } => (new_key_file, None),
        CredentialCommand::Rotate {
            new_key_file,
            backup_snapshot_id,
        } => (new_key_file, Some(backup_snapshot_id)),
    };
    let new_key =
        crate::services::sources::service::read_key_file(PathBuf::from(new_key_file).as_path())?;
    let plan = crate::services::sources::service::plan_source_credential_rotation(
        database.pool(),
        source_config.credential_key(),
        &new_key,
    )
    .await?;

    println!(
        "Source credential rotation plan: {} credential rows ({} current envelopes, {} legacy \
         envelopes, {} plaintext rows).",
        plan.credential_rows, plan.current_envelopes, plan.legacy_envelopes, plan.plaintext_rows
    );

    if let Some(snapshot_id) = backup_snapshot_id {
        let verified_backup = crate::services::backup::verify_full_backup(
            PathBuf::from(&config.backup_path),
            snapshot_id,
        )
        .await?;
        let report = crate::services::sources::service::rotate_source_credentials(
            database.pool(),
            source_config.credential_key(),
            &new_key,
            &verified_backup,
        )
        .await?;
        println!(
            "Rotated {} source credential rows using verified backup {}. Legacy database key \
             removed: {}. Update the active external key configuration before normal startup.",
            report.rows_rotated,
            verified_backup.snapshot_id(),
            report.legacy_database_key_removed
        );
    } else {
        println!(
            "No changes were applied. Create and verify a full backup, then run `librarian \
             credentials rotate` with the same new key file."
        );
    }

    database.pool().pool().close().await;
    Ok(())
}

/// Wait for a shutdown signal: Ctrl+C on all platforms, plus SIGTERM on Unix
/// (the signal Docker/systemd/`kill` send by default) so `stop_all()` always runs
/// instead of the process being killed out from under it.
#[cfg(unix)]
async fn wait_for_shutdown_signal() -> anyhow::Result<()> {
    use tokio::signal::unix::{SignalKind, signal};

    let mut sigterm = signal(SignalKind::terminate())?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result?,
        _ = sigterm.recv() => {
            tracing::info!("Received SIGTERM, shutting down gracefully");
        }
    }
    Ok(())
}

#[cfg(not(unix))]
async fn wait_for_shutdown_signal() -> anyhow::Result<()> {
    tokio::signal::ctrl_c().await?;
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
/// librqbit creates per-peer sockets, each needing ~4 kernel FDs:
///
/// - socket
/// - eventpoll
/// - eventfd
/// - timerfd
///
/// The default soft limit of 1024 on many Linux systems is easily exhausted by
/// a handful of active torrents. Raising it early avoids "unable to open
/// database file" (SQLITE_CANTOPEN) errors that appear once FDs run out.
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
