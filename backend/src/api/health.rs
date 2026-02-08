//! Health check endpoints

use axum::{Json, Router, routing::get};
use serde::Serialize;

use crate::AppState;
use crate::services::graphql::filesystem_network;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub platform: String,
    pub network_paths: Vec<NetworkPathHealth>,
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

/// Health check - always returns OK if the server is running
async fn healthz(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<HealthResponse> {
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
        platform: filesystem_network::current_platform(),
        network_paths,
    })
}

/// Readiness check - verifies dependencies are available
async fn readyz(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<ReadyResponse> {
    // Check database connectivity
    let db_ok = sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    Json(ReadyResponse {
        ready: db_ok,
        database: db_ok,
    })
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
}
