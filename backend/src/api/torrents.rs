//! Torrent management REST endpoints
//!
//! These provide a REST API alternative to GraphQL for torrent operations.
//! For real-time updates, use the GraphQL subscriptions at /graphql/ws.

use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::graphql::verify_token;
use crate::services::TorrentInfo as ServiceTorrentInfo;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct TorrentResponse {
    pub id: usize,
    pub info_hash: String,
    pub name: String,
    pub state: String,
    pub progress: f64,
    pub size: u64,
    pub downloaded: u64,
    pub uploaded: u64,
    pub download_speed: u64,
    pub upload_speed: u64,
    pub peers: usize,
    pub save_path: String,
}

impl From<ServiceTorrentInfo> for TorrentResponse {
    fn from(info: ServiceTorrentInfo) -> Self {
        Self {
            id: info.id,
            info_hash: info.info_hash,
            name: info.name,
            state: info.state.to_string(),
            progress: info.progress,
            size: info.size,
            downloaded: info.downloaded,
            uploaded: info.uploaded,
            download_speed: info.download_speed,
            upload_speed: info.upload_speed,
            peers: info.peers,
            save_path: info.save_path,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AddTorrentRequest {
    /// Magnet link
    pub magnet: Option<String>,
    /// URL to .torrent file
    pub url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AddTorrentResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub torrent: Option<TorrentResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ActionResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RemoveTorrentQuery {
    #[serde(default)]
    pub delete_files: bool,
}

/// List all torrents
async fn list_torrents(State(state): State<AppState>) -> Json<Vec<TorrentResponse>> {
    let torrents = state.torrent_service.list_torrents().await;
    Json(torrents.into_iter().map(|t| t.into()).collect())
}

/// Get a specific torrent by ID
async fn get_torrent(
    State(state): State<AppState>,
    Path(id): Path<usize>,
) -> Result<Json<TorrentResponse>, StatusCode> {
    match state.torrent_service.get_torrent_info(id).await {
        Ok(info) => Ok(Json(info.into())),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// Add a new torrent (deprecated - use GraphQL instead)
async fn add_torrent(
    State(state): State<AppState>,
    Json(body): Json<AddTorrentRequest>,
) -> (StatusCode, Json<AddTorrentResponse>) {
    // Note: No user_id available in REST API - use GraphQL for full functionality
    let result = if let Some(magnet) = body.magnet {
        state.torrent_service.add_magnet(&magnet, None).await
    } else if let Some(url) = body.url {
        state.torrent_service.add_magnet(&url, None).await
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(AddTorrentResponse {
                success: false,
                torrent: None,
                error: Some("Either 'magnet' or 'url' must be provided".to_string()),
            }),
        );
    };

    match result {
        Ok(info) => (
            StatusCode::CREATED,
            Json(AddTorrentResponse {
                success: true,
                torrent: Some(info.into()),
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AddTorrentResponse {
                success: false,
                torrent: None,
                error: Some(e.to_string()),
            }),
        ),
    }
}

/// Pause a torrent
async fn pause_torrent(
    State(state): State<AppState>,
    Path(id): Path<usize>,
) -> (StatusCode, Json<ActionResponse>) {
    match state.torrent_service.pause(id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ActionResponse {
                success: true,
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ActionResponse {
                success: false,
                error: Some(e.to_string()),
            }),
        ),
    }
}

/// Resume a torrent
async fn resume_torrent(
    State(state): State<AppState>,
    Path(id): Path<usize>,
) -> (StatusCode, Json<ActionResponse>) {
    match state.torrent_service.resume(id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ActionResponse {
                success: true,
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ActionResponse {
                success: false,
                error: Some(e.to_string()),
            }),
        ),
    }
}

/// Remove a torrent
async fn remove_torrent(
    State(state): State<AppState>,
    Path(id): Path<usize>,
    Query(query): Query<RemoveTorrentQuery>,
) -> (StatusCode, Json<ActionResponse>) {
    match state.torrent_service.remove(id, query.delete_files).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ActionResponse {
                success: true,
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ActionResponse {
                success: false,
                error: Some(e.to_string()),
            }),
        ),
    }
}

/// Upload a .torrent file
async fn upload_torrent_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> (StatusCode, Json<AddTorrentResponse>) {
    // Extract and verify auth token
    let user_id: Option<Uuid> = if let Some(auth_header) = headers.get(AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            let token = auth_str.trim_start_matches("Bearer ").trim();
            match verify_token(token) {
                Ok(user) => Uuid::parse_str(&user.user_id).ok(),
                Err(_) => None,
            }
        } else {
            None
        }
    } else {
        None
    };

    // Read the multipart file
    let mut file_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "file" || field_name == "torrent" {
            match field.bytes().await {
                Ok(bytes) => {
                    file_data = Some(bytes.to_vec());
                    break;
                }
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(AddTorrentResponse {
                            success: false,
                            torrent: None,
                            error: Some(format!("Failed to read file: {}", e)),
                        }),
                    );
                }
            }
        }
    }

    let data = match file_data {
        Some(d) => d,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(AddTorrentResponse {
                    success: false,
                    torrent: None,
                    error: Some("No torrent file provided. Use field name 'file' or 'torrent'.".to_string()),
                }),
            );
        }
    };

    // Add the torrent
    match state.torrent_service.add_torrent_file(data, user_id).await {
        Ok(info) => (
            StatusCode::CREATED,
            Json(AddTorrentResponse {
                success: true,
                torrent: Some(info.into()),
                error: None,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AddTorrentResponse {
                success: false,
                torrent: None,
                error: Some(e.to_string()),
            }),
        ),
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/torrents", get(list_torrents).post(add_torrent))
        .route("/torrents/upload", post(upload_torrent_file))
        .route("/torrents/{id}", get(get_torrent).delete(remove_torrent))
        .route("/torrents/{id}/pause", post(pause_torrent))
        .route("/torrents/{id}/resume", post(resume_torrent))
}
