//! Librarian Backend - Rust-powered media library service
//!
//! This is the main entry point for the Librarian backend API.
//! All operations are exposed via GraphQL at /graphql.

mod api;
mod config;
mod db;
mod graphql;
mod jobs;
mod media;
mod services;
mod torrent;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::WebSocketUpgrade;
use axum::http::header::AUTHORIZATION;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use async_graphql::http::GraphiQLSource;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::db::Database;
use crate::graphql::{verify_token, AuthUser, LibrarianSchema};
use crate::services::{TorrentService, TorrentServiceConfig};

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Database,
    pub schema: LibrarianSchema,
    pub torrent_service: Arc<TorrentService>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "librarian_backend=debug,tower_http=debug,librqbit=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    tracing::info!("Starting Librarian Backend");

    // Load configuration
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;
    let config = Arc::new(config);

    tracing::info!("Configuration loaded");

    // Initialize database connection
    let db = Database::connect(&config.database_url).await?;
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

    // Build GraphQL schema
    let schema = graphql::build_schema(torrent_service.clone(), db.clone());
    tracing::info!("GraphQL schema built");

    // Build application state
    let state = AppState {
        config: config.clone(),
        db,
        schema,
        torrent_service,
    };

    // Build router - GraphQL is the primary API
    let app = Router::new()
        // Health endpoints (no auth required)
        .merge(api::health::router())
        // REST API endpoints
        .nest("/api", api::torrents::router())
        .nest("/api", api::filesystem::router())
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
    tracing::info!("GraphQL playground: http://localhost:{}/graphql", config.port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

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

    if let Some(token) = extract_token(&headers) {
        if let Ok(user) = verify_token(&token) {
            request = request.data(user);
        }
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
    let auth_user: Option<AuthUser> = extract_token(&headers)
        .and_then(|token| verify_token(&token).ok());

    ws.protocols(["graphql-transport-ws", "graphql-ws"])
        .on_upgrade(move |socket| {
            let mut ws = async_graphql_axum::GraphQLWebSocket::new(
                socket,
                state.schema.clone(),
                protocol,
            );

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
