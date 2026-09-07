//! Application state and HTTP router construction.
//!
//! Used by [main] and by [HttpServerService](crate::services::http_server::HttpServerService)
//! to build the Axum app.

use std::sync::Arc;

use axum::Router;
use axum::http::{HeaderValue, Method, Request, header};
use tower_http::cors::{AllowOrigin, CorsLayer};
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
        .layer(cors_layer(&state.config))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<axum::body::Body>| {
                // Path only, never the query string: Cast stream requests carry a
                // short-lived receiver grant in the query string, and query strings
                // must not end up in logs/traces.
                tracing::info_span!(
                    "request",
                    method = %request.method(),
                    path = %request.uri().path(),
                )
            }),
        )
        .with_state(state);

    #[cfg(feature = "embed-frontend")]
    let app = app.fallback(crate::static_assets::embedded_fallback);

    #[cfg(not(feature = "embed-frontend"))]
    let app = app.fallback_service(
        ServeDir::new("./static").not_found_service(ServeFile::new("./static/index.html")),
    );

    app
}

/// Build the CORS layer from `config.cors_origins`. Uses an explicit
/// allowlist (never mirrors/reflects the request's `Origin`) since this is
/// combined with `allow_credentials(true)`; reflecting an arbitrary origin
/// with credentials enabled lets any site read authenticated responses.
/// Same-origin requests (the production embedded frontend) never need CORS
/// at all — this only matters for the dev server and any operator-configured
/// extra origins.
fn cors_layer(config: &Config) -> CorsLayer {
    let origins: Vec<HeaderValue> = config
        .cors_origins
        .iter()
        .filter_map(|origin| match HeaderValue::from_str(origin) {
            Ok(value) => Some(value),
            Err(err) => {
                tracing::warn!(%origin, %err, "Ignoring invalid CORS origin in LIBRARIAN_CORS_ORIGINS");
                None
            }
        })
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(true)
}
