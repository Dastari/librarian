//! GraphQL resolvers for filesystem operations (directory browsing).
//! Exposes BrowseDirectory with PascalCase types for the codegen frontend.

use std::path::PathBuf;

use async_graphql::{Context, InputObject, Object, Result, SimpleObject};
use tokio::fs;

use crate::services::graphql::auth::AuthUser;

/// Input for the BrowseDirectory query (PascalCase for GraphQL).
#[derive(Default, InputObject)]
#[graphql(name = "BrowseDirectoryInput")]
pub struct BrowseDirectoryInput {
    /// Path to browse (defaults to root or home).
    #[graphql(name = "Path")]
    pub path: Option<String>,
    /// Only show directories.
    #[graphql(name = "DirsOnly")]
    pub dirs_only: bool,
    /// Include hidden entries (files/dirs starting with .).
    #[graphql(name = "ShowHidden")]
    pub show_hidden: bool,
}

/// A single file or directory entry (PascalCase for GraphQL).
#[derive(SimpleObject)]
#[graphql(name = "BrowseDirectoryEntry")]
pub struct BrowseDirectoryEntry {
    #[graphql(name = "Name")]
    pub name: String,
    #[graphql(name = "Path")]
    pub path: String,
    #[graphql(name = "IsDir")]
    pub is_dir: bool,
    #[graphql(name = "Size")]
    pub size: u64,
    #[graphql(name = "SizeFormatted")]
    pub size_formatted: String,
    #[graphql(name = "Readable")]
    pub readable: bool,
    #[graphql(name = "Writable")]
    pub writable: bool,
    #[graphql(name = "MimeType")]
    pub mime_type: Option<String>,
    #[graphql(name = "ModifiedAt")]
    pub modified_at: Option<String>,
}

/// Quick-access path shortcut (PascalCase for GraphQL).
#[derive(SimpleObject)]
#[graphql(name = "BrowseQuickPath")]
pub struct BrowseQuickPath {
    #[graphql(name = "Name")]
    pub name: String,
    #[graphql(name = "Path")]
    pub path: String,
}

/// Result of browsing a directory (PascalCase for GraphQL).
#[derive(SimpleObject)]
#[graphql(name = "BrowseDirectoryResult")]
pub struct BrowseDirectoryResult {
    #[graphql(name = "CurrentPath")]
    pub current_path: String,
    #[graphql(name = "ParentPath")]
    pub parent_path: Option<String>,
    #[graphql(name = "Entries")]
    pub entries: Vec<BrowseDirectoryEntry>,
    #[graphql(name = "QuickPaths")]
    pub quick_paths: Vec<BrowseQuickPath>,
    #[graphql(name = "IsLibraryPath")]
    pub is_library_path: bool,
    #[graphql(name = "LibraryId")]
    pub library_id: Option<String>,
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
            let name = drive
                .to_string_lossy()
                .trim_end_matches('\\')
                .to_string();
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
    /// Browse a directory on the server. Requires authentication.
    #[graphql(name = "BrowseDirectory")]
    async fn browse_directory(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: Option<BrowseDirectoryInput>,
    ) -> Result<BrowseDirectoryResult> {
        let _user = ctx
            .data_opt::<AuthUser>()
            .ok_or_else(|| async_graphql::Error::new("Authentication required to browse directories"))?;

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
                while !path.exists() && path.parent().is_some() {
                    path = path.parent().unwrap().to_path_buf();
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
                        t.duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .and_then(|d| {
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

        let parent_path = canonical_path
            .parent()
            .map(|p| display_path(p));

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
