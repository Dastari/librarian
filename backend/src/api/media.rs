//! Media item endpoints

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize)]
pub struct MediaItem {
    pub id: Uuid,
    pub title: String,
    pub media_type: String,
    pub year: Option<i32>,
    pub overview: Option<String>,
    pub runtime: Option<i32>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StreamInfo {
    pub playlist_url: String,
    pub direct_play_supported: bool,
}

/// Get media item details
async fn get_media(
    State(_state): State<AppState>,
    Path(_media_id): Path<Uuid>,
) -> Json<Option<MediaItem>> {
    // TODO: Implement database query
    Json(None)
}

/// Get HLS stream information
async fn get_stream(
    State(_state): State<AppState>,
    Path(_media_id): Path<Uuid>,
) -> Json<StreamInfo> {
    // TODO: Implement stream URL generation
    Json(StreamInfo {
        playlist_url: String::new(),
        direct_play_supported: true,
    })
}

/// Get cast session token
async fn get_cast_session(
    State(_state): State<AppState>,
    Path(_media_id): Path<Uuid>,
) -> Json<serde_json::Value> {
    // TODO: Implement cast session
    Json(serde_json::json!({ "session_token": "" }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/media/{id}", get(get_media))
        .route("/media/{id}/stream/hls", get(get_stream))
        .route("/media/{id}/cast/session", get(get_cast_session))
}
