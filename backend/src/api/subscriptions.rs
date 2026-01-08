//! Subscription management endpoints

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub show_name: String,
    pub tvdb_id: i32,
    pub quality_profile_id: Uuid,
    pub monitored: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub show_name: String,
    pub tvdb_id: i32,
    pub quality_profile_id: Uuid,
}

/// List all subscriptions
async fn list_subscriptions(State(_state): State<AppState>) -> Json<Vec<Subscription>> {
    // TODO: Implement database query
    Json(vec![])
}

/// Create a new subscription
async fn create_subscription(
    State(_state): State<AppState>,
    Json(_body): Json<CreateSubscriptionRequest>,
) -> Json<Subscription> {
    // TODO: Implement subscription creation
    Json(Subscription {
        id: Uuid::new_v4(),
        show_name: String::new(),
        tvdb_id: 0,
        quality_profile_id: Uuid::new_v4(),
        monitored: true,
    })
}

/// Manually search for episodes
async fn search_subscription(
    State(_state): State<AppState>,
    Path(_subscription_id): Path<Uuid>,
) -> Json<serde_json::Value> {
    // TODO: Implement Torznab search
    Json(serde_json::json!({ "results": [] }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/subscriptions",
            get(list_subscriptions).post(create_subscription),
        )
        .route("/subscriptions/{id}/search", post(search_subscription))
}
