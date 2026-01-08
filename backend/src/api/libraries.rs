//! Library management endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

/// Library types supported by the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LibraryType {
    Movies,
    Tv,
    Music,
    Audiobooks,
    Other,
}

impl LibraryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LibraryType::Movies => "movies",
            LibraryType::Tv => "tv",
            LibraryType::Music => "music",
            LibraryType::Audiobooks => "audiobooks",
            LibraryType::Other => "other",
        }
    }

    pub fn default_icon(&self) -> &'static str {
        match self {
            LibraryType::Movies => "film",
            LibraryType::Tv => "tv",
            LibraryType::Music => "music",
            LibraryType::Audiobooks => "headphones",
            LibraryType::Other => "folder",
        }
    }

    pub fn default_color(&self) -> &'static str {
        match self {
            LibraryType::Movies => "purple",
            LibraryType::Tv => "blue",
            LibraryType::Music => "green",
            LibraryType::Audiobooks => "orange",
            LibraryType::Other => "slate",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    pub library_type: LibraryType,
    pub icon: String,
    pub color: String,
    pub auto_scan: bool,
    pub scan_interval_hours: i32,
    pub last_scanned_at: Option<String>,
    pub file_count: Option<i64>,
    pub total_size_bytes: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateLibraryRequest {
    pub name: String,
    pub path: String,
    pub library_type: LibraryType,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default = "default_auto_scan")]
    pub auto_scan: bool,
    #[serde(default = "default_scan_interval")]
    pub scan_interval_hours: i32,
}

fn default_auto_scan() -> bool { true }
fn default_scan_interval() -> i32 { 24 }

#[derive(Debug, Deserialize)]
pub struct UpdateLibraryRequest {
    pub name: Option<String>,
    pub path: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub auto_scan: Option<bool>,
    pub scan_interval_hours: Option<i32>,
}

/// List all libraries for the current user
async fn list_libraries(State(_state): State<AppState>) -> Json<Vec<Library>> {
    // TODO: Query from database with user_id from JWT
    // For now, return example libraries
    Json(vec![
        Library {
            id: Uuid::new_v4(),
            name: "Movies".to_string(),
            path: "/data/media/Movies".to_string(),
            library_type: LibraryType::Movies,
            icon: "film".to_string(),
            color: "purple".to_string(),
            auto_scan: true,
            scan_interval_hours: 24,
            last_scanned_at: None,
            file_count: Some(0),
            total_size_bytes: Some(0),
        },
        Library {
            id: Uuid::new_v4(),
            name: "TV Shows".to_string(),
            path: "/data/media/TV".to_string(),
            library_type: LibraryType::Tv,
            icon: "tv".to_string(),
            color: "blue".to_string(),
            auto_scan: true,
            scan_interval_hours: 6,
            last_scanned_at: None,
            file_count: Some(0),
            total_size_bytes: Some(0),
        },
    ])
}

/// Get a single library by ID
async fn get_library(
    State(_state): State<AppState>,
    Path(library_id): Path<Uuid>,
) -> Result<Json<Library>, StatusCode> {
    // TODO: Query from database
    Ok(Json(Library {
        id: library_id,
        name: "Movies".to_string(),
        path: "/data/media/Movies".to_string(),
        library_type: LibraryType::Movies,
        icon: "film".to_string(),
        color: "purple".to_string(),
        auto_scan: true,
        scan_interval_hours: 24,
        last_scanned_at: None,
        file_count: None,
        total_size_bytes: None,
    }))
}

/// Create a new library
async fn create_library(
    State(_state): State<AppState>,
    Json(body): Json<CreateLibraryRequest>,
) -> Result<Json<Library>, StatusCode> {
    // TODO: Insert into database with user_id from JWT
    let library = Library {
        id: Uuid::new_v4(),
        name: body.name,
        path: body.path,
        library_type: body.library_type.clone(),
        icon: body.icon.unwrap_or_else(|| body.library_type.default_icon().to_string()),
        color: body.color.unwrap_or_else(|| body.library_type.default_color().to_string()),
        auto_scan: body.auto_scan,
        scan_interval_hours: body.scan_interval_hours,
        last_scanned_at: None,
        file_count: Some(0),
        total_size_bytes: Some(0),
    };

    Ok(Json(library))
}

/// Update an existing library
async fn update_library(
    State(_state): State<AppState>,
    Path(_library_id): Path<Uuid>,
    Json(_body): Json<UpdateLibraryRequest>,
) -> Result<Json<Library>, StatusCode> {
    // TODO: Update in database
    Err(StatusCode::NOT_IMPLEMENTED)
}

/// Delete a library
async fn delete_library(
    State(_state): State<AppState>,
    Path(_library_id): Path<Uuid>,
) -> StatusCode {
    // TODO: Delete from database (soft delete or cascade?)
    StatusCode::NO_CONTENT
}

/// Trigger a library scan
async fn scan_library(
    State(_state): State<AppState>,
    Path(library_id): Path<Uuid>,
) -> Json<serde_json::Value> {
    // TODO: Enqueue scan job
    tracing::info!("Scan requested for library: {}", library_id);
    Json(serde_json::json!({ 
        "status": "scan_queued",
        "library_id": library_id.to_string()
    }))
}

/// Get library statistics
async fn get_library_stats(
    State(_state): State<AppState>,
    Path(library_id): Path<Uuid>,
) -> Json<serde_json::Value> {
    // TODO: Query stats from database
    Json(serde_json::json!({
        "library_id": library_id.to_string(),
        "file_count": 0,
        "total_size_bytes": 0,
        "movie_count": 0,
        "episode_count": 0,
        "last_scanned_at": null
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/libraries", get(list_libraries).post(create_library))
        .route("/libraries/{id}", get(get_library).patch(update_library).delete(delete_library))
        .route("/libraries/{id}/scan", post(scan_library))
        .route("/libraries/{id}/stats", get(get_library_stats))
}
