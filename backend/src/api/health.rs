//! Health check endpoints

use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

#[derive(Serialize)]
pub struct ReadyResponse {
    pub ready: bool,
    pub database: bool,
}

/// Health check - always returns OK if the server is running
async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Readiness check - verifies dependencies are available
async fn readyz(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<ReadyResponse> {
    // Check database connectivity
    let db_ok = sqlx::query("SELECT 1")
        .fetch_one(state.db.pool())
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
