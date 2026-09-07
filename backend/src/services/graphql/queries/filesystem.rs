//! GraphQL resolvers for filesystem operations (directory browsing).
//! BrowseDirectory is legacy schema surface and still needs the camelCase resolver migration.

use std::path::PathBuf;
use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Result, SimpleObject};
use tokio::fs;

use crate::services::graphql::auth::AuthExt;
use crate::services::graphql::filesystem_network;
use crate::{db::Database, services::manager::ServicesManager};

/// Input for the legacy BrowseDirectory query.
#[derive(Default, InputObject)]
#[graphql(name = "BrowseDirectoryInput")]
pub struct BrowseDirectoryInput {
    /// Path to browse (defaults to root or home).
    #[graphql(name = "path")]
    pub path: Option<String>,
    /// Only show directories.
    #[graphql(name = "dirsOnly")]
    pub dirs_only: bool,
    /// Include hidden entries (files/dirs starting with .).
    #[graphql(name = "showHidden")]
    pub show_hidden: bool,
}

/// A single file or directory entry.
#[derive(SimpleObject)]
#[graphql(name = "BrowseDirectoryEntry")]
pub struct BrowseDirectoryEntry {
    #[graphql(name = "name")]
    pub name: String,
    #[graphql(name = "path")]
    pub path: String,
    #[graphql(name = "isDir")]
    pub is_dir: bool,
    #[graphql(name = "size")]
    pub size: u64,
    #[graphql(name = "sizeFormatted")]
    pub size_formatted: String,
    #[graphql(name = "readable")]
    pub readable: bool,
    #[graphql(name = "writable")]
    pub writable: bool,
    #[graphql(name = "mimeType")]
    pub mime_type: Option<String>,
    #[graphql(name = "modifiedAt")]
    pub modified_at: Option<String>,
}

/// Quick-access path shortcut.
#[derive(SimpleObject)]
#[graphql(name = "BrowseQuickPath")]
pub struct BrowseQuickPath {
    #[graphql(name = "name")]
    pub name: String,
    #[graphql(name = "path")]
    pub path: String,
}

/// Result of browsing a directory.
#[derive(SimpleObject)]
#[graphql(name = "BrowseDirectoryResult")]
pub struct BrowseDirectoryResult {
    #[graphql(name = "currentPath")]
    pub current_path: String,
    #[graphql(name = "parentPath")]
    pub parent_path: Option<String>,
    #[graphql(name = "entries")]
    pub entries: Vec<BrowseDirectoryEntry>,
    #[graphql(name = "quickPaths")]
    pub quick_paths: Vec<BrowseQuickPath>,
    #[graphql(name = "isLibraryPath")]
    pub is_library_path: bool,
    #[graphql(name = "libraryId")]
    pub library_id: Option<String>,
}

/// Runtime filesystem/network capabilities exposed to frontend.
#[derive(SimpleObject)]
#[graphql(name = "FilesystemRuntimeInfo")]
pub struct FilesystemRuntimeInfo {
    #[graphql(name = "platform")]
    pub platform: String,
    #[graphql(name = "supportsUncCredentials")]
    pub supports_unc_credentials: bool,
    #[graphql(name = "supportsSambaMount")]
    pub supports_samba_mount: bool,
    #[graphql(name = "defaultLinuxMountBase")]
    pub default_linux_mount_base: Option<String>,
}

#[derive(InputObject)]
#[graphql(name = "LibraryPathAvailabilityInput")]
pub struct LibraryPathAvailabilityInput {
    #[graphql(name = "paths")]
    pub paths: Vec<String>,
    #[graphql(name = "attemptReconnect")]
    pub attempt_reconnect: Option<bool>,
}

#[derive(SimpleObject)]
#[graphql(name = "LibraryPathAvailability")]
pub struct LibraryPathAvailability {
    #[graphql(name = "path")]
    pub path: String,
    #[graphql(name = "reachable")]
    pub reachable: bool,
    #[graphql(name = "exists")]
    pub exists: bool,
    #[graphql(name = "isDirectory")]
    pub is_directory: bool,
    #[graphql(name = "needsReconnect")]
    pub needs_reconnect: bool,
    #[graphql(name = "reconnectAttempted")]
    pub reconnect_attempted: bool,
    #[graphql(name = "reconnectSucceeded")]
    pub reconnect_succeeded: bool,
    #[graphql(name = "message")]
    pub message: Option<String>,
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;
    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn display_path(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}

#[cfg(windows)]
fn windows_drive_paths() -> Vec<PathBuf> {
    let mut drives = Vec::new();
    for letter in b'A'..=b'Z' {
        let path = format!("{}:\\", letter as char);
        if std::path::Path::new(&path).exists() {
            drives.push(PathBuf::from(path));
        }
    }
    drives
}

fn default_browse_path() -> PathBuf {
    #[cfg(windows)]
    {
        let drives = windows_drive_paths();
        return drives
            .into_iter()
            .next()
            .unwrap_or_else(|| PathBuf::from("C:\\"));
    }

    #[cfg(not(windows))]
    {
        PathBuf::from("/")
    }
}

fn get_quick_paths() -> Vec<BrowseQuickPath> {
    let mut paths: Vec<BrowseQuickPath> = vec![];

    #[cfg(windows)]
    {
        for drive in windows_drive_paths() {
            let name = drive.to_string_lossy().trim_end_matches('\\').to_string();
            paths.push(BrowseQuickPath {
                name,
                path: display_path(&drive),
            });
        }
        if let Some(home) = dirs::home_dir() {
            paths.push(BrowseQuickPath {
                name: "Home".to_string(),
                path: display_path(&home),
            });
        }
        return paths;
    }

    #[cfg(not(windows))]
    {
        if let Some(home) = dirs::home_dir() {
            paths.push(BrowseQuickPath {
                name: "Home".to_string(),
                path: display_path(&home),
            });
        }

        for (path, name) in [
            ("/data", "Data"),
            ("/mnt", "Mounts"),
            ("/media", "Media"),
            ("/home", "Home Directories"),
            ("/var", "Var"),
            ("/tmp", "Temp"),
        ] {
            if std::path::Path::new(path).exists() {
                paths.push(BrowseQuickPath {
                    name: name.to_string(),
                    path: path.to_string(),
                });
            }
        }

        paths.push(BrowseQuickPath {
            name: "Root".to_string(),
            path: "/".to_string(),
        });
    }

    paths
}

#[derive(Default)]
pub struct FilesystemQueries;

#[Object]
impl FilesystemQueries {
    #[graphql(name = "filesystemRuntimeInfo")]
    async fn filesystem_runtime_info(&self, ctx: &Context<'_>) -> Result<FilesystemRuntimeInfo> {
        ctx.require_admin()?;

        Ok(FilesystemRuntimeInfo {
            platform: filesystem_network::current_platform(),
            supports_unc_credentials: filesystem_network::supports_unc_credentials(),
            supports_samba_mount: filesystem_network::supports_samba_mount(),
            default_linux_mount_base: filesystem_network::default_mount_base()
                .map(ToString::to_string),
        })
    }

    #[graphql(name = "libraryPathAvailability")]
    async fn library_path_availability(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: LibraryPathAvailabilityInput,
    ) -> Result<Vec<LibraryPathAvailability>> {
        ctx.require_admin()?;

        let db = ctx.data::<Database>()?;
        let services = ctx.data::<Arc<ServicesManager>>()?;
        let attempt_reconnect = input.attempt_reconnect.unwrap_or(false);

        let mut out = Vec::with_capacity(input.paths.len());
        for path in input.paths {
            let status =
                filesystem_network::check_path_availability(db, services, &path, attempt_reconnect)
                    .await;
            out.push(LibraryPathAvailability {
                path: status.path,
                reachable: status.reachable,
                exists: status.exists,
                is_directory: status.is_directory,
                needs_reconnect: status.needs_reconnect,
                reconnect_attempted: status.reconnect_attempted,
                reconnect_succeeded: status.reconnect_succeeded,
                message: status.message,
            });
        }

        Ok(out)
    }

    /// Browse a directory on the server. Requires authentication.
    #[graphql(name = "browseDirectory")]
    async fn browse_directory(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: Option<BrowseDirectoryInput>,
    ) -> Result<BrowseDirectoryResult> {
        ctx.require_admin()?;

        let path = input.as_ref().and_then(|i| i.path.as_deref());
        let dirs_only = input.as_ref().map(|i| i.dirs_only).unwrap_or(true);
        let show_hidden = input.as_ref().map(|i| i.show_hidden).unwrap_or(false);

        let requested_path = match path {
            Some(p) if !p.is_empty() => PathBuf::from(p),
            _ => default_browse_path(),
        };
        #[cfg(windows)]
        let requested_path = if requested_path == PathBuf::from("/") {
            default_browse_path()
        } else {
            requested_path
        };

        let canonical_path = match requested_path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                let mut path = requested_path;
                while !path.exists() {
                    let Some(parent) = path.parent() else {
                        break;
                    };
                    path = parent.to_path_buf();
                }
                if path.exists() {
                    path.canonicalize().unwrap_or_else(|_| PathBuf::from("/"))
                } else {
                    #[cfg(windows)]
                    return Ok(default_browse_result());
                    #[cfg(not(windows))]
                    PathBuf::from("/")
                }
            }
        };

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
                    if dirs_only && !is_dir {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !show_hidden && name.starts_with('.') {
                        continue;
                    }
                    let readable = fs::metadata(&entry_path).await.is_ok();
                    let writable = if is_dir {
                        fs::metadata(&entry_path)
                            .await
                            .map(|m| !m.permissions().readonly())
                            .unwrap_or(false)
                    } else {
                        false
                    };
                    let size = if is_dir { 0 } else { metadata.len() };
                    let modified_at = metadata.modified().ok().and_then(|t| {
                        t.duration_since(std::time::UNIX_EPOCH).ok().and_then(|d| {
                            chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                                .map(|dt| dt.to_rfc3339())
                        })
                    });
                    entries.push(BrowseDirectoryEntry {
                        name: name.clone(),
                        path: display_path(&entry_path),
                        is_dir,
                        size,
                        size_formatted: format_size(size),
                        readable,
                        writable,
                        mime_type: None,
                        modified_at,
                    });
                }
            }
            Err(e) => {
                return Err(async_graphql::Error::new(format!(
                    "Cannot read directory: {}",
                    e
                )));
            }
        }

        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        let parent_path = canonical_path.parent().map(display_path);

        Ok(BrowseDirectoryResult {
            current_path: display_path(&canonical_path),
            parent_path,
            entries,
            quick_paths: get_quick_paths(),
            is_library_path: false,
            library_id: None,
        })
    }
}

#[cfg(windows)]
fn default_browse_result() -> BrowseDirectoryResult {
    BrowseDirectoryResult {
        current_path: default_browse_path().to_string_lossy().into_owned(),
        parent_path: None,
        entries: vec![],
        quick_paths: get_quick_paths(),
        is_library_path: false,
        library_id: None,
    }
}
