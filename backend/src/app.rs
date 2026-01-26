//! Application state and HTTP router construction.
//!
//! Used by [main] and by [HttpServerService](crate::services::http_server::HttpServerService)
//! to build the Axum app.

use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
#[cfg(not(feature = "embed-frontend"))]
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::db::Database;
use crate::graphql::LibrarianSchema;
use crate::services::{GraphqlService, ServicesManager};

/// Shared state for HTTP handlers (GraphQL, API routes).
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Database,
    pub schema: LibrarianSchema,
    pub services: Arc<ServicesManager>,
}

/// Build the API router (e.g. /api/*) by merging all route builders registered
/// with [ServicesManagerBuilder::add_api_routes]. Services and main can register
/// endpoints via the builder so the HTTP server doesn't need to know about them.
pub fn api_router(state: AppState) -> Router<AppState> {
    state.services.build_api_router(state.clone())
}

/// Build the full Axum router: /api, /graphql, /graphql/ws, layers, and fallback.
/// Returns Router<()> (state fully applied) for use with axum::serve.
pub async fn build_app(state: AppState) -> Router<()> {
    let api = api_router(state.clone());
    let app = Router::new()
        .nest("/api", api)
        .merge(GraphqlService::router())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    #[cfg(feature = "embed-frontend")]
    let app = app.fallback(crate::static_assets::embedded_fallback);

    #[cfg(not(feature = "embed-frontend"))]
    let app = app.fallback_service(
        ServeDir::new("./static").not_found_service(ServeFile::new("./static/index.html")),
    );

    app
}
