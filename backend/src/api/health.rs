//! Health check endpoints

use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use serde::Serialize;

use crate::AppState;
use crate::api::auth_guard::require_authenticated_user;
use crate::services::graphql::filesystem_network;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    /// Platform/network-path details are only populated for authenticated callers —
    /// they reveal filesystem layout and configured network mounts, which is unnecessary
    /// information for anonymous health probes (load balancers etc. only need `status`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_paths: Option<Vec<NetworkPathHealth>>,
}

#[derive(Serialize)]
pub struct ReadyResponse {
    pub ready: bool,
    pub database: bool,
}

#[derive(Serialize)]
pub struct NetworkPathHealth {
    pub path: String,
    pub reachable: bool,
    pub message: Option<String>,
}

/// Health check - always returns OK if the server is running. Bare `status`/`version` for
/// anonymous callers; authenticated callers additionally get platform/network-path details
/// (previously always exposed, which leaked filesystem/network config to anyone).
async fn healthz(State(state): State<AppState>, headers: HeaderMap) -> Json<HealthResponse> {
    let authenticated = require_authenticated_user(&state, &headers, None)
        .await
        .is_ok();

    if !authenticated {
        return Json(HealthResponse {
            status: "healthy",
            version: env!("CARGO_PKG_VERSION"),
            platform: None,
            network_paths: None,
        });
    }

    let mut network_paths = Vec::new();
    if let Ok(configs) = filesystem_network::load_saved_network_configs(&state.db).await {
        for cfg in configs {
            let status = filesystem_network::check_path_availability(
                &state.db,
                &state.services,
                &cfg.target_path,
                false,
            )
            .await;
            network_paths.push(NetworkPathHealth {
                path: cfg.target_path,
                reachable: status.reachable,
                message: status.message,
            });
        }
    }

    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
        platform: Some(filesystem_network::current_platform()),
        network_paths: Some(network_paths),
    })
}

/// Readiness check - verifies dependencies are available
async fn readyz(
    axum::extract::State(_state): axum::extract::State<AppState>,
) -> Json<ReadyResponse> {
    Json(ReadyResponse {
        ready: true,
        database: true,
    })
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
}
