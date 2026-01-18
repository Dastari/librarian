//! Filesystem browsing API for server-side directory selection

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct BrowseQuery {
    /// Path to browse (defaults to root or home)
    path: Option<String>,
    /// Only show directories
    #[serde(default)]
    dirs_only: bool,
}

#[derive(Debug, Serialize)]
pub struct FileEntry {
    /// File/directory name
    pub name: String,
    /// Full path
    pub path: String,
    /// Is this a directory?
    pub is_dir: bool,
    /// File size in bytes (0 for directories)
    pub size: u64,
    /// Is this path readable?
    pub readable: bool,
    /// Is this path writable?
    pub writable: bool,
}

#[derive(Debug, Serialize)]
pub struct BrowseResponse {
    /// Current path being browsed
    pub current_path: String,
    /// Parent path (null if at root)
    pub parent_path: Option<String>,
    /// List of entries in the directory
    pub entries: Vec<FileEntry>,
    /// Common quick-access paths
    pub quick_paths: Vec<QuickPath>,
}

#[derive(Debug, Serialize)]
pub struct QuickPath {
    pub name: String,
    pub path: String,
}

/// Get quick access paths for the system
fn get_quick_paths() -> Vec<QuickPath> {
    let mut paths = vec![];

    // Home directory
    if let Some(home) = dirs::home_dir() {
        paths.push(QuickPath {
            name: "Home".to_string(),
            path: home.to_string_lossy().to_string(),
        });
    }

    // Common data directories
    let common_paths = [
        ("/data", "Data"),
        ("/mnt", "Mounts"),
        ("/media", "Media"),
        ("/home", "Home Directories"),
        ("/var", "Var"),
        ("/tmp", "Temp"),
    ];

    for (path, name) in common_paths {
        if std::path::Path::new(path).exists() {
            paths.push(QuickPath {
                name: name.to_string(),
                path: path.to_string(),
            });
        }
    }

    // Root
    paths.push(QuickPath {
        name: "Root".to_string(),
        path: "/".to_string(),
    });

    paths
}

/// Browse a directory on the server
async fn browse_directory(
    State(_state): State<AppState>,
    Query(query): Query<BrowseQuery>,
) -> Result<Json<BrowseResponse>, (StatusCode, String)> {
    // Determine the path to browse (default to root)
    let requested_path = match &query.path {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => PathBuf::from("/"),
    };

    // Canonicalize the path to resolve symlinks and ..
    // If the path doesn't exist, fall back to nearest existing parent or root
    let canonical_path = match requested_path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // Path doesn't exist - try to find the nearest existing parent
            let mut path = requested_path.clone();
            while !path.exists() && path.parent().is_some() {
                path = path.parent().unwrap().to_path_buf();
            }
            // If we found an existing parent, use it; otherwise fall back to root
            if path.exists() {
                path.canonicalize().unwrap_or_else(|_| PathBuf::from("/"))
            } else {
                PathBuf::from("/")
            }
        }
    };

    // Read directory contents
    let mut entries = Vec::new();

    match fs::read_dir(&canonical_path).await {
        Ok(mut dir) => {
            while let Ok(Some(entry)) = dir.next_entry().await {
                let entry_path = entry.path();
                let metadata = match entry.metadata().await {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let is_dir = metadata.is_dir();

                // Skip files if dirs_only is set
                if query.dirs_only && !is_dir {
                    continue;
                }

                // Skip hidden files (starting with .)
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }

                // Check permissions
                let readable = fs::metadata(&entry_path).await.is_ok();
                let writable = if is_dir {
                    // Try to check if we can write to the directory
                    fs::metadata(&entry_path)
                        .await
                        .map(|m| !m.permissions().readonly())
                        .unwrap_or(false)
                } else {
                    false
                };

                entries.push(FileEntry {
                    name,
                    path: entry_path.to_string_lossy().to_string(),
                    is_dir,
                    size: if is_dir { 0 } else { metadata.len() },
                    readable,
                    writable,
                });
            }
        }
        Err(e) => {
            return Err((
                StatusCode::FORBIDDEN,
                format!("Cannot read directory: {}", e),
            ));
        }
    }

    // Sort: directories first, then by name
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    // Get parent path
    let parent_path = canonical_path
        .parent()
        .map(|p| p.to_string_lossy().to_string());

    Ok(Json(BrowseResponse {
        current_path: canonical_path.to_string_lossy().to_string(),
        parent_path,
        entries,
        quick_paths: get_quick_paths(),
    }))
}

/// Create a directory on the server
#[derive(Debug, Deserialize)]
pub struct CreateDirRequest {
    path: String,
}

#[derive(Debug, Serialize)]
pub struct CreateDirResponse {
    success: bool,
    path: Option<String>,
    error: Option<String>,
}

async fn create_directory(
    State(_state): State<AppState>,
    Json(body): Json<CreateDirRequest>,
) -> Json<CreateDirResponse> {
    let path = PathBuf::from(&body.path);

    match fs::create_dir_all(&path).await {
        Ok(_) => Json(CreateDirResponse {
            success: true,
            path: Some(path.to_string_lossy().to_string()),
            error: None,
        }),
        Err(e) => Json(CreateDirResponse {
            success: false,
            path: None,
            error: Some(e.to_string()),
        }),
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/filesystem/browse", get(browse_directory))
        .route("/filesystem/mkdir", axum::routing::post(create_directory))
}
