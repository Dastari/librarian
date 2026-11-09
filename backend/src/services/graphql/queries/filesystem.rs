//! GraphQL resolvers for filesystem operations (directory browsing).
//! BrowseDirectory is legacy schema surface and still needs the camelCase resolver migration.

use std::path::PathBuf;
use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Result, SimpleObject};
use tokio::fs;

use crate::services::graphql::auth::AuthExt;
use crate::services::graphql::filesystem_network;
use crate::services::library_storage::{
    EntryKind, SmbLocation, StoragePath, is_smb_uri, library_storage,
};
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
    #[graphql(
        name = "attemptReconnect",
        deprecation = "Queries are read-only; use reconnectNetworkPath instead"
    )]
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

/// Whether a saved sign-in exists for a share, and for whom.
#[derive(SimpleObject)]
#[graphql(name = "ShareSignIn")]
pub struct ShareSignIn {
    /// The share's normalised `smb://host[:port]/share` address.
    #[graphql(name = "share")]
    pub share: String,
    #[graphql(name = "saved")]
    pub saved: bool,
    #[graphql(name = "username")]
    pub username: Option<String>,
}

/// What moving a library to a folder its files were copied to would find. Nothing is changed.
#[derive(SimpleObject)]
#[graphql(name = "LibraryMovePlan", rename_fields = "camelCase")]
pub struct LibraryMovePlanGql {
    /// The library's current folder.
    pub old_path: String,
    /// The new folder, as it would be saved.
    pub new_path: String,
    /// Files the library holds.
    pub file_count: i32,
    /// On the new folder with the same size.
    pub matched: i32,
    /// Not on the new folder at all.
    pub missing: i32,
    /// The first of them, as paths below the library's folder.
    pub missing_files: Vec<String>,
    /// On the new folder, but at a different size (a copy that did not finish).
    pub changed: i32,
    pub changed_files: Vec<String>,
    /// Media files on the new folder that the library does not hold; the next scan finds them.
    pub extra: i32,
    pub extra_files: Vec<String>,
    /// Why the move cannot go ahead at all.
    pub blocked: Option<String>,
    /// Every file is on the new folder at its size, and nothing blocks the move.
    pub can_apply: bool,
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
        // Availability is a read. Mount/reconnect is the explicit admin command.
        let _legacy_attempt_reconnect = input.attempt_reconnect;
        if input.paths.len() > 100 {
            return Err(async_graphql::Error::new(
                "At most 100 paths may be checked at once",
            ));
        }

        let mut out = Vec::with_capacity(input.paths.len());
        for path in input.paths {
            let status =
                filesystem_network::check_path_availability(db, services, &path, false).await;
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

    /// Whether a sign-in is saved for a share (`smb://host[:port]/share`), and its user name. The
    /// password is never returned.
    #[graphql(name = "shareSignIn")]
    async fn share_sign_in(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "share")] share: String,
    ) -> Result<ShareSignIn> {
        ctx.require_admin()?;
        let db = ctx.data::<Database>()?;
        let services = ctx.data::<Arc<ServicesManager>>()?;
        let location = SmbLocation::parse(&share)
            .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        let username = filesystem_network::saved_share_username(db, services, &location).await;
        Ok(ShareSignIn {
            share: location.share_root().to_uri(),
            saved: username.is_some(),
            username,
        })
    }

    /// Compare a library's files with a folder they were copied to, before moving the library
    /// there. Read only.
    #[graphql(name = "libraryMovePlan")]
    async fn library_move_plan(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "libraryId")] library_id: String,
        #[graphql(name = "newPath")] new_path: String,
    ) -> Result<LibraryMovePlanGql> {
        ctx.require_admin()?;
        let services = ctx.data::<Arc<ServicesManager>>()?;
        let scan = services
            .get_library_scan()
            .await
            .ok_or_else(|| async_graphql::Error::new("Library scan service not available"))?;
        let plan = scan
            .plan_library_move(&library_id, &new_path)
            .await
            .map_err(|error| async_graphql::Error::new(format!("{error:#}")))?;
        let count = |value: usize| i32::try_from(value).unwrap_or(i32::MAX);
        Ok(LibraryMovePlanGql {
            can_apply: plan.is_complete(),
            old_path: plan.old_path,
            new_path: plan.new_path,
            file_count: count(plan.file_count),
            matched: count(plan.matched),
            missing: count(plan.missing),
            missing_files: plan.missing_files,
            changed: count(plan.changed),
            changed_files: plan.changed_files,
            extra: count(plan.extra),
            extra_files: plan.extra_files,
            blocked: plan.blocked,
        })
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

        if let Some(remote) = path.filter(|path| is_smb_uri(path)) {
            return browse_share(remote, dirs_only, show_hidden).await;
        }

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

/// Browse a folder on a share through the storage layer. The listing cannot climb above the
/// share's root.
async fn browse_share(
    path: &str,
    dirs_only: bool,
    show_hidden: bool,
) -> Result<BrowseDirectoryResult> {
    let location =
        SmbLocation::parse(path).map_err(|error| async_graphql::Error::new(error.to_string()))?;
    let storage = library_storage();
    // A folder that is not there (a library typed ahead of its folder) shows the nearest one that is.
    let mut here = location.storage_path();
    let listing = loop {
        match storage.list_dir(&here).await {
            Ok(listing) => break listing,
            Err(error) if error.is_not_found() && here.parent().is_some() => {
                here = here.parent().expect("checked above");
            }
            Err(error) => {
                return Err(async_graphql::Error::new(format!(
                    "Cannot read directory: {error}"
                )));
            }
        }
    };
    let mut entries = Vec::new();
    for entry in listing {
        let is_dir = entry.kind == EntryKind::Directory;
        if (dirs_only && !is_dir) || (!show_hidden && entry.name.starts_with('.')) {
            continue;
        }
        let size = if is_dir {
            0
        } else {
            entry.meta.as_ref().map_or(0, |meta| meta.size)
        };
        let modified_at = entry
            .meta
            .as_ref()
            .and_then(|meta| meta.modified)
            .map(|time| chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339());
        entries.push(BrowseDirectoryEntry {
            path: here.join(&entry.name).into_string(),
            name: entry.name,
            is_dir,
            size,
            size_formatted: format_size(size),
            readable: true,
            writable: is_dir,
            mime_type: None,
            modified_at,
        });
    }
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    let share_root = StoragePath::new(location.share_root().to_uri());
    Ok(BrowseDirectoryResult {
        current_path: here.as_str().to_string(),
        parent_path: here.parent().map(StoragePath::into_string),
        entries,
        quick_paths: vec![BrowseQuickPath {
            name: format!("{} (share)", location.share),
            path: share_root.into_string(),
        }],
        is_library_path: false,
        library_id: None,
    })
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
