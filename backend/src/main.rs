//! Librarian Backend - Rust-powered media library service
//!
//! This is the main entry point for the Librarian backend API.
//! All operations are exposed via GraphQL at /graphql.

mod api;
mod config;
mod db;
mod graphql;
mod indexer;
mod jobs;
mod media;
mod services;
mod tui;
mod usenet;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use async_graphql::http::GraphiQLSource;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::Router;
use axum::extract::WebSocketUpgrade;
use axum::http::HeaderMap;
use axum::http::header::AUTHORIZATION;
use axum::response::IntoResponse;
use axum::routing::get;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::api::media::{MediaState, media_routes};
use crate::config::Config;
use crate::db::Database;
use crate::graphql::{AuthUser, LibrarianSchema, verify_token};
use crate::graphql::{LibraryChangedEvent, MediaFileUpdatedEvent};
use crate::services::{
    ArtworkService, CastService, CastServiceConfig, DatabaseLoggerConfig, FfmpegService,
    FilesystemService, FilesystemServiceConfig, MediaAnalysisQueue, MetadataServiceConfig,
    ScannerService, StorageClient, TorrentService, TorrentServiceConfig,
    artwork::ensure_artwork_bucket, create_database_layer, create_media_analysis_queue,
    create_metadata_service_with_artwork, create_metrics_collector,
};
use crate::tui::{TuiApp, TuiConfig, create_tui_layer, should_use_tui};

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Database,
    pub schema: LibrarianSchema,
    pub torrent_service: Arc<TorrentService>,
    pub scanner_service: Arc<ScannerService>,
    pub cast_service: Arc<CastService>,
    pub filesystem_service: Arc<FilesystemService>,
    pub analysis_queue: Arc<MediaAnalysisQueue>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration first (before tracing, so we can use the database for logging)
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;
    let config = Arc::new(config);

    // Initialize database connection early so we can use it for logging
    // Uses retry logic to wait for database to become available
    eprintln!("Connecting to database...");
    let db = Database::connect_with_retry(&config.database_url, std::time::Duration::from_secs(30))
        .await;
    eprintln!("Database connected!");

    // Run migrations to ensure schema is up to date
    eprintln!("Running database migrations...");
    if let Err(e) = db.migrate().await {
        eprintln!("Failed to run migrations: {}", e);
        eprintln!("Please run migrations manually: cd backend && sqlx migrate run");
        std::process::exit(1);
    }
    eprintln!("Migrations complete!");

    // Create the database logging layer
    let db_logger_config = DatabaseLoggerConfig {
        min_level: tracing::Level::INFO,
        batch_size: 100,
        flush_interval_ms: 2000,
        broadcast_capacity: 1000,
    };
    let (db_layer, log_broadcast_sender) =
        create_database_layer(db.pool().clone(), db_logger_config);

    // Detect TUI mode
    let use_tui = should_use_tui();

    // Create TUI layer if needed (we need the receiver before init)
    let tui_log_rx = if use_tui {
        let (tui_layer, tui_rx) = create_tui_layer(tracing::Level::INFO);

        // Initialize tracing with TUI layer (no stdout output)
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "librarian=info,tower_http=info,librqbit=info".into()),
            )
            .with(tui_layer)
            .with(db_layer)
            .init();

        Some(tui_rx)
    } else {
        // Initialize tracing with JSON console output (headless mode)
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "librarian=info,tower_http=info,librqbit=info".into()),
            )
            .with(tracing_subscriber::fmt::layer().json())
            .with(db_layer)
            .init();

        None
    };

    // Create metrics collector
    let metrics = create_metrics_collector();

    tracing::info!("Starting Librarian Backend");
    tracing::info!("Configuration loaded");
    tracing::info!("Database connected");

    // Initialize torrent service with database for persistence
    let torrent_config = TorrentServiceConfig {
        download_dir: PathBuf::from(&config.downloads_path),
        session_dir: PathBuf::from(&config.session_path),
        enable_dht: config.torrent_enable_dht,
        listen_port: config.torrent_listen_port,
        max_concurrent: config.torrent_max_concurrent,
    };
    let torrent_service = Arc::new(TorrentService::new(torrent_config, db.clone()).await?);
    tracing::info!("Torrent service initialized with database persistence");

    // Initialize Supabase storage client for artwork caching
    let storage_client = StorageClient::new(
        config.supabase_url.clone(),
        config.supabase_service_key.clone(),
    );

    // Ensure artwork bucket exists
    if let Err(e) = ensure_artwork_bucket(&storage_client).await {
        tracing::warn!(error = %e, "Failed to create artwork bucket - artwork caching may not work");
    }

    let artwork_service = Arc::new(ArtworkService::new(storage_client));
    tracing::info!("Artwork service initialized");

    // Initialize metadata service with artwork caching
    // Check environment variables first, then fall back to database settings
    let tmdb_api_key = match std::env::var("TMDB_API_KEY").ok() {
        Some(key) if !key.is_empty() => Some(key),
        _ => {
            // Try to read from database settings
            match db
                .settings()
                .get_value::<String>("metadata.tmdb_api_key")
                .await
            {
                Ok(Some(key)) if !key.is_empty() => {
                    tracing::info!("Using TMDB API key from database settings");
                    Some(key)
                }
                _ => None,
            }
        }
    };
    let tvdb_api_key = match std::env::var("TVDB_API_KEY").ok() {
        Some(key) if !key.is_empty() => Some(key),
        _ => db
            .settings()
            .get_value::<String>("metadata.tvdb_api_key")
            .await
            .ok()
            .flatten(),
    };
    let openai_api_key = match std::env::var("OPENAI_API_KEY").ok() {
        Some(key) if !key.is_empty() => Some(key),
        _ => db
            .settings()
            .get_value::<String>("metadata.openai_api_key")
            .await
            .ok()
            .flatten(),
    };

    let metadata_config = MetadataServiceConfig {
        tmdb_api_key,
        tvdb_api_key,
        openai_api_key,
    };
    let metadata_service =
        create_metadata_service_with_artwork(db.clone(), metadata_config, artwork_service);
    tracing::info!("Metadata service initialized with artwork caching");

    // Initialize FFmpeg service for media analysis
    let ffmpeg_service = Arc::new(FfmpegService::new());
    if ffmpeg_service.is_available().await {
        tracing::info!("FFmpeg service initialized (ffprobe available)");
    } else {
        tracing::warn!(
            "FFmpeg service initialized but ffprobe not found in PATH - media analysis will fail"
        );
    }

    // Create media file updated broadcast channel for real-time UI updates
    let (media_file_tx, _) = tokio::sync::broadcast::channel::<MediaFileUpdatedEvent>(100);

    // Initialize media analysis queue for FFmpeg metadata extraction
    let analysis_queue = Arc::new(create_media_analysis_queue(
        ffmpeg_service,
        db.clone(),
        None, // subtitle_queue - TODO: wire up when subtitle download is implemented
        Some(media_file_tx.clone()), // Clone so we can also use it in GraphQL schema
    ));
    tracing::info!("Media analysis queue initialized");

    // Create library changed broadcast channel for real-time scan status updates
    let (library_changed_tx, _) = tokio::sync::broadcast::channel::<LibraryChangedEvent>(100);

    // Initialize IndexerManager early so we can pass it to ScannerService for auto-hunt
    let indexer_manager = match db.settings().get_or_create_indexer_encryption_key().await {
        Ok(encryption_key) => {
            match indexer::manager::IndexerManager::new(db.clone(), &encryption_key).await {
                Ok(manager) => {
                    tracing::info!("IndexerManager initialized for auto-hunt");
                    Some(std::sync::Arc::new(manager))
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to initialize IndexerManager - auto-hunt will be disabled");
                    None
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to get indexer encryption key - auto-hunt will be disabled");
            None
        }
    };

    // Initialize scanner service with analysis queue, library broadcast, and auto-hunt services
    let scanner_service = {
        let mut scanner = ScannerService::new(db.clone(), metadata_service.clone())
            .with_analysis_queue(analysis_queue.clone())
            .with_library_changed_tx(library_changed_tx.clone())
            .with_torrent_service(torrent_service.clone());

        // Add IndexerManager if available for post-scan auto-hunt
        if let Some(ref mgr) = indexer_manager {
            scanner = scanner.with_indexer_manager(mgr.clone());
        }

        Arc::new(scanner)
    };
    tracing::info!("Scanner service initialized with FFmpeg analysis and auto-hunt");

    // Initialize cast service for Chromecast/AirPlay support
    let cast_config = CastServiceConfig {
        media_base_url: format!(
            "http://{}:{}",
            config.host.as_deref().unwrap_or("0.0.0.0"),
            config.port
        ),
        auto_discovery: true,
        discovery_interval_secs: 30,
    };
    let cast_service = Arc::new(CastService::new(db.clone(), cast_config));

    // Start mDNS device discovery in background
    if let Err(e) = cast_service.start_discovery().await {
        tracing::warn!(error = %e, "Failed to start cast device discovery");
    }
    tracing::info!("Cast service initialized");

    // Initialize filesystem service for file operations
    let filesystem_config = FilesystemServiceConfig {
        allow_unrestricted: false, // Only allow operations within library paths
    };
    let filesystem_service = FilesystemService::new(db.clone(), filesystem_config);
    tracing::info!("Filesystem service initialized");

    // Build GraphQL schema
    let schema = graphql::build_schema(
        torrent_service.clone(),
        metadata_service.clone(),
        scanner_service.clone(),
        cast_service.clone(),
        filesystem_service.clone(),
        db.clone(),
        analysis_queue.clone(),
        Some(log_broadcast_sender),
        Some(library_changed_tx),
        Some(media_file_tx),
    );
    tracing::info!("GraphQL schema built");

    // Start job scheduler
    let _scheduler = jobs::start_scheduler(
        scanner_service.clone(),
        torrent_service.clone(),
        db.pool().clone(),
        Some(analysis_queue.clone()),
        Some(metadata_service.clone()),
        indexer_manager,
    )
    .await?;
    tracing::info!("Job scheduler started");

    // Start torrent completion handler for immediate processing
    // This subscribes to TorrentEvent::Completed and processes torrents right away
    // instead of waiting for the 1-minute cron job
    let completion_handler = services::TorrentCompletionHandler::new(
        db.clone(),
        torrent_service.clone(),
        services::CompletionHandlerConfig::default(),
    )
    .with_analysis_queue(analysis_queue.clone())
    .with_metadata_service(metadata_service.clone());

    let _completion_handle = completion_handler.start();
    tracing::info!("Torrent completion handler started");

    // Trigger initial schedule sync in the background
    // This ensures the schedule cache is populated on first startup
    let startup_pool = db.pool().clone();
    tokio::spawn(async move {
        tracing::info!("Starting initial TV schedule sync...");
        if let Err(e) = jobs::schedule_sync::sync_schedule(startup_pool).await {
            tracing::warn!(
                "Initial schedule sync failed (will retry on schedule): {}",
                e
            );
        } else {
            tracing::info!("Initial TV schedule sync completed");
        }
    });

    // Process any completed torrents that weren't organized on previous run
    // This catches torrents that completed while the server was down
    let startup_pool2 = db.pool().clone();
    let startup_torrent_service = torrent_service.clone();
    let startup_analysis_queue = analysis_queue.clone();
    let startup_metadata_service = metadata_service.clone();
    tokio::spawn(async move {
        // Short delay to ensure everything is initialized
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // First, process any pending torrents (newly completed)
        tracing::info!("Checking for unprocessed completed torrents...");
        if let Err(e) = jobs::download_monitor::process_completed_torrents(
            startup_pool2.clone(),
            startup_torrent_service.clone(),
            Some(startup_analysis_queue.clone()),
        )
        .await
        {
            tracing::warn!(
                "Startup torrent processing failed (will retry on schedule): {}",
                e
            );
        } else {
            tracing::info!("Startup torrent processing completed");
        }

        // Then, retry matching for previously unmatched torrents
        // (e.g., shows that were added after the torrent completed)
        tracing::info!("Checking for unmatched torrents to retry...");
        if let Err(e) = jobs::download_monitor::process_unmatched_on_startup(
            startup_pool2,
            startup_torrent_service,
            Some(startup_analysis_queue),
        )
        .await
        {
            tracing::warn!("Unmatched torrent processing failed: {}", e);
        }
    });

    // Clone references for TUI before moving into AppState
    let tui_torrent_service = torrent_service.clone();
    let tui_pool = db.pool().clone();

    // Build application state
    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        schema,
        torrent_service,
        scanner_service,
        cast_service,
        filesystem_service,
        analysis_queue,
    };

    // Build media state for streaming routes
    let media_state = MediaState { db };

    // Build router - GraphQL is the primary API
    let app = Router::new()
        // Health endpoints (no auth required)
        .merge(api::health::router())
        // REST API endpoints
        .nest("/api", api::torrents::router())
        .nest("/api", api::filesystem::router())
        // Torznab API for external apps (Sonarr, Radarr, etc.)
        .nest("/api", api::torznab::router())
        // Media streaming endpoints for cast devices and browser playback
        .nest("/api", media_routes().with_state(media_state))
        // GraphQL endpoint (handles all queries, mutations, subscriptions)
        .route("/graphql", get(graphiql).post(graphql_handler))
        // GraphQL WebSocket endpoint for subscriptions
        .route("/graphql/ws", get(graphql_ws_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Listening on {}", addr);
    tracing::info!(
        "GraphQL playground: http://localhost:{}/graphql",
        config.port
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Run with TUI or headless based on detection
    if let Some(tui_rx) = tui_log_rx {
        // TUI mode: run server and TUI in parallel
        let tui_app = TuiApp::new(
            metrics,
            tui_rx,
            tui_torrent_service,
            tui_pool,
            TuiConfig::default(),
        )?;

        tokio::select! {
            result = axum::serve(listener, app) => {
                result?;
            }
            result = tui_app.run() => {
                result?;
                // TUI exited (user pressed q), terminate the process
                std::process::exit(0);
            }
        }
    } else {
        // Headless mode: just run the server
        axum::serve(listener, app).await?;
    }

    Ok(())
}

/// Extract bearer token from Authorization header
fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .filter(|h| h.starts_with("Bearer "))
        .map(|h| h[7..].to_string())
}

/// GraphQL query/mutation handler with auth context
async fn graphql_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    // Extract and verify auth token if present
    let mut request = req.into_inner();

    if let Some(token) = extract_token(&headers)
        && let Ok(user) = verify_token(&token)
    {
        request = request.data(user);
    }

    state.schema.execute(request).await.into()
}

/// GraphiQL interactive playground (only for browsers)
async fn graphiql(headers: HeaderMap) -> impl IntoResponse {
    // Check if this is a browser request (accepts HTML)
    let accepts_html = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/html"))
        .unwrap_or(false);

    if accepts_html {
        axum::response::Html(
            GraphiQLSource::build()
                .endpoint("/graphql")
                .subscription_endpoint("/graphql/ws")
                .finish(),
        )
        .into_response()
    } else {
        // Return a helpful JSON error for non-browser requests
        (
            axum::http::StatusCode::METHOD_NOT_ALLOWED,
            axum::Json(serde_json::json!({
                "error": "GET requests are not supported for GraphQL queries. Use POST with Content-Type: application/json"
            })),
        )
            .into_response()
    }
}

/// GraphQL WebSocket handler for subscriptions with auth
async fn graphql_ws_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: HeaderMap,
    protocol: async_graphql_axum::GraphQLProtocol,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    // Extract auth from headers for initial connection
    let auth_user: Option<AuthUser> =
        extract_token(&headers).and_then(|token| verify_token(&token).ok());

    ws.protocols(["graphql-transport-ws", "graphql-ws"])
        .on_upgrade(move |socket| {
            let mut ws =
                async_graphql_axum::GraphQLWebSocket::new(socket, state.schema.clone(), protocol);

            // Add auth context if available
            if let Some(user) = auth_user {
                let mut data = async_graphql::Data::default();
                data.insert(user);
                ws = ws.with_data(data);
            }

            // Handle connection_init for auth via payload
            ws.on_connection_init(|params| async move {
                // Check for token in connection params (for WebSocket auth)
                if let Some(token) = params
                    .get("Authorization")
                    .or_else(|| params.get("authorization"))
                    .and_then(|v| v.as_str())
                {
                    let token = token.strip_prefix("Bearer ").unwrap_or(token);
                    if let Ok(user) = verify_token(token) {
                        let mut data = async_graphql::Data::default();
                        data.insert(user);
                        return Ok(data);
                    }
                }
                Ok(async_graphql::Data::default())
            })
            .serve()
        })
}
