//! Torrent file upload endpoint
//!
//! This provides a REST endpoint for uploading .torrent files.
//! File uploads don't work well with GraphQL (multipart form data),
//! so this is the one torrent operation that remains as REST.
//!
//! All other torrent operations (add magnet, pause, resume, remove, list)
//! are handled via GraphQL at /graphql.

use axum::{
    Json, Router,
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    routing::post,
};
use serde::Serialize;
use uuid::Uuid;

use crate::AppState;
use crate::graphql::verify_token;
use crate::services::TorrentInfo as ServiceTorrentInfo;

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

#[derive(Debug, Serialize)]
pub struct AddTorrentResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub torrent: Option<TorrentResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Upload a .torrent file
///
/// This is the only REST endpoint for torrents - file uploads don't work well with GraphQL.
/// Use GraphQL mutations for all other torrent operations (add magnet, pause, resume, remove).
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
                    error: Some(
                        "No torrent file provided. Use field name 'file' or 'torrent'.".to_string(),
                    ),
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
    Router::new().route("/torrents/upload", post(upload_torrent_file))
}
