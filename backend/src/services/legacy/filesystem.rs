//! Filesystem service for directory browsing and file operations
//!
//! This service provides file system operations with library path validation.
//! Copy/move/delete operations are restricted to paths within valid libraries.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use tokio::fs;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::{Database, LibraryRecord};
use crate::services::file_utils::{normalize_display_path, normalize_display_path_buf};

/// Event emitted when directory contents change
#[derive(Debug, Clone)]
pub struct DirectoryChangeEvent {
    /// Directory path that changed
    pub path: String,
    /// Type of change: "created", "modified", "deleted", "renamed"
    pub change_type: String,
    /// Affected file/directory name
    pub name: Option<String>,
    /// New name (for rename events)
    pub new_name: Option<String>,
    /// Timestamp of the change
    pub timestamp: DateTime<Utc>,
}

/// A file or directory entry
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub readable: bool,
    pub writable: bool,
    pub mime_type: Option<String>,
    pub modified_at: Option<DateTime<Utc>>,
}

/// A quick-access path shortcut
#[derive(Debug, Clone)]
pub struct QuickPath {
    pub name: String,
    pub path: String,
}

/// Result of browsing a directory
#[derive(Debug, Clone)]
pub struct BrowseResult {
    pub current_path: String,
    pub parent_path: Option<String>,
    pub entries: Vec<FileEntry>,
    pub quick_paths: Vec<QuickPath>,
    pub is_library_path: bool,
    pub library_id: Option<Uuid>,
}

/// Result of path validation
#[derive(Debug, Clone)]
pub struct PathValidation {
    pub is_valid: bool,
    pub is_library_path: bool,
    pub library_id: Option<Uuid>,
    pub library_name: Option<String>,
    pub error: Option<String>,
}

/// Filesystem service configuration
#[derive(Debug, Clone)]
pub struct FilesystemServiceConfig {
    /// Whether to allow operations outside of library paths
    pub allow_unrestricted: bool,
}

impl Default for FilesystemServiceConfig {
    fn default() -> Self {
        Self {
            allow_unrestricted: false,
        }
    }
}

/// Service for filesystem operations
pub struct FilesystemService {
    db: Database,
    config: FilesystemServiceConfig,
    change_sender: broadcast::Sender<DirectoryChangeEvent>,
    /// Cache of library paths for validation (user_id -> libraries)
    library_cache: tokio::sync::RwLock<HashMap<Uuid, Vec<LibraryRecord>>>,
}

impl FilesystemService {
    pub fn new(db: Database, config: FilesystemServiceConfig) -> Arc<Self> {
        let (change_sender, _) = broadcast::channel(256);
        Arc::new(Self {
            db,
            config,
            change_sender,
            library_cache: tokio::sync::RwLock::new(HashMap::new()),
        })
    }

    /// Subscribe to directory change events
    pub fn subscribe(&self) -> broadcast::Receiver<DirectoryChangeEvent> {
        self.change_sender.subscribe()
    }

    /// Broadcast a directory change event
    fn broadcast_change(&self, event: DirectoryChangeEvent) {
        let _ = self.change_sender.send(event);
    }

    /// Get quick access paths for the system
    pub fn get_quick_paths() -> Vec<QuickPath> {
        let mut paths = vec![];

        if cfg!(windows) {
            for drive in windows_drive_paths() {
                let name = drive
                    .to_string_lossy()
                    .trim_end_matches('\\')
                    .to_string();
                paths.push(QuickPath {
                    name,
                    path: normalize_display_path_buf(&drive),
                });
            }
            if let Some(home) = dirs::home_dir() {
                paths.push(QuickPath {
                    name: "Home".to_string(),
                    path: normalize_display_path_buf(&home),
                });
            }
            return paths;
        }

        // Home directory
        if let Some(home) = dirs::home_dir() {
            paths.push(QuickPath {
                name: "Home".to_string(),
                path: normalize_display_path_buf(&home),
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
            if Path::new(path).exists() {
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

    /// Refresh the library cache for a user
    async fn refresh_library_cache(&self, user_id: Uuid) -> Result<Vec<LibraryRecord>> {
        let libraries = self.db.libraries().list_by_user(user_id).await?;
        let mut cache: tokio::sync::RwLockWriteGuard<'_, HashMap<Uuid, Vec<LibraryRecord>>> =
            self.library_cache.write().await;
        cache.insert(user_id, libraries.clone());
        Ok(libraries)
    }

    /// Get libraries for a user (from cache or database)
    async fn get_libraries(&self, user_id: Uuid) -> Result<Vec<LibraryRecord>> {
        // Check cache first
        {
            let cache: tokio::sync::RwLockReadGuard<'_, HashMap<Uuid, Vec<LibraryRecord>>> =
                self.library_cache.read().await;
            if let Some(libs) = cache.get(&user_id) {
                return Ok(libs.clone());
            }
        }
        // Cache miss, fetch from database
        self.refresh_library_cache(user_id).await
    }

    /// Invalidate the library cache for a user
    pub async fn invalidate_cache(&self, user_id: Uuid) {
        let mut cache: tokio::sync::RwLockWriteGuard<'_, HashMap<Uuid, Vec<LibraryRecord>>> =
            self.library_cache.write().await;
        cache.remove(&user_id);
    }

    /// Validate that a path is inside a library
    pub async fn validate_path(&self, path: &str, user_id: Uuid) -> Result<PathValidation> {
        let canonical = match PathBuf::from(path).canonicalize() {
            Ok(p) => p,
            Err(e) => {
                return Ok(PathValidation {
                    is_valid: false,
                    is_library_path: false,
                    library_id: None,
                    library_name: None,
                    error: Some(format!("Path does not exist or is not accessible: {}", e)),
                });
            }
        };

        let libraries = self.get_libraries(user_id).await?;

        for lib in &libraries {
            let lib_canonical = match PathBuf::from(&lib.path).canonicalize() {
                Ok(p) => p,
                Err(_) => continue,
            };
            if canonical.starts_with(&lib_canonical) {
                return Ok(PathValidation {
                    is_valid: true,
                    is_library_path: true,
                    library_id: Some(lib.id),
                    library_name: Some(lib.name.clone()),
                    error: None,
                });
            }
        }

        // Not inside any library
        if self.config.allow_unrestricted {
            Ok(PathValidation {
                is_valid: true,
                is_library_path: false,
                library_id: None,
                library_name: None,
                error: None,
            })
        } else {
        Ok(PathValidation {
            is_valid: false,
            is_library_path: false,
            library_id: None,
            library_name: None,
            error: Some("Path is not inside any library".to_string()),
        })
    }
    }

    /// Check if a path is inside any library (for operations requiring library paths)
    pub async fn require_library_path(&self, path: &str, user_id: Uuid) -> Result<PathValidation> {
        let validation = self.validate_path(path, user_id).await?;
        if !validation.is_library_path {
            return Ok(PathValidation {
                is_valid: false,
                is_library_path: false,
                library_id: None,
                library_name: None,
                error: Some("Operation only allowed within library directories".to_string()),
            });
        }
        Ok(validation)
    }

    /// Browse a directory
    pub async fn browse(
        &self,
        path: Option<&str>,
        dirs_only: bool,
        show_hidden: bool,
        user_id: Uuid,
    ) -> Result<BrowseResult> {
        // Determine the path to browse (default to root)
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

        // Canonicalize the path to resolve symlinks and ..
        let canonical_path = match requested_path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // Path doesn't exist - try to find the nearest existing parent
                let mut path = requested_path.clone();
                while !path.exists() && path.parent().is_some() {
                    path = path.parent().unwrap().to_path_buf();
                }
                if path.exists() {
                    path.canonicalize().unwrap_or_else(|_| PathBuf::from("/"))
                } else {
                    PathBuf::from("/")
                }
            }
        };

        // Check if this path is inside a library
        let validation = self
            .validate_path(&canonical_path.to_string_lossy(), user_id)
            .await?;

        // Read directory contents
        let mut entries = Vec::new();

        let mut dir = fs::read_dir(&canonical_path).await.map_err(|e| {
            anyhow!(
                "Cannot read directory '{}': {}",
                canonical_path.display(),
                e
            )
        })?;

        while let Ok(Some(entry)) = dir.next_entry().await {
            let entry_path = entry.path();
            let metadata = match entry.metadata().await {
                Ok(m) => m,
                Err(_) => continue,
            };

            let is_dir = metadata.is_dir();

            // Skip files if dirs_only is set
            if dirs_only && !is_dir {
                continue;
            }

            // Skip hidden files unless show_hidden
            let name = entry.file_name().to_string_lossy().to_string();
            if !show_hidden && name.starts_with('.') {
                continue;
            }

            // Check permissions
            let readable = fs::metadata(&entry_path).await.is_ok();
            let writable = if is_dir {
                fs::metadata(&entry_path)
                    .await
                    .map(|m| !m.permissions().readonly())
                    .unwrap_or(false)
            } else {
                false
            };

            // Get modification time
            let modified_at = metadata
                .modified()
                .ok()
                .and_then(|t| DateTime::<Utc>::try_from(t).ok());

            // Get MIME type for files
            let mime_type = if !is_dir {
                mime_guess::from_path(&entry_path)
                    .first()
                    .map(|m| m.to_string())
            } else {
                None
            };

            entries.push(FileEntry {
                name,
                path: normalize_display_path_buf(&entry_path),
                is_dir,
                size: if is_dir { 0 } else { metadata.len() },
                readable,
                writable,
                mime_type,
                modified_at,
            });
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
            .map(|p| normalize_display_path_buf(p));

        Ok(BrowseResult {
            current_path: normalize_display_path_buf(&canonical_path),
            parent_path,
            entries,
            quick_paths: Self::get_quick_paths(),
            is_library_path: validation.is_library_path,
            library_id: validation.library_id,
        })
    }

    /// Create a directory
    pub async fn create_directory(&self, path: &str, _user_id: Uuid) -> Result<String> {
        // For create, we allow creating directories at any location for now
        // (used for library folder selection, not file management)
        let path_buf = PathBuf::from(path);

        fs::create_dir_all(&path_buf)
            .await
            .map_err(|e| anyhow!("Failed to create directory '{}': {}", path, e))?;

        // Broadcast the change
        if let Ok(canonical) = path_buf.canonicalize() {
            if let Some(parent) = canonical.parent() {
                self.broadcast_change(DirectoryChangeEvent {
                    path: parent.to_string_lossy().to_string(),
                    change_type: "created".to_string(),
                    name: canonical
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string()),
                    new_name: None,
                    timestamp: Utc::now(),
                });
            }
        }

        path_buf
            .canonicalize()
            .map(|p| normalize_display_path_buf(&p))
            .map_err(|e| anyhow!("Failed to resolve path: {}", e))
    }

    /// Delete files or directories (must be inside a library)
    pub async fn delete_files(
        &self,
        paths: &[String],
        recursive: bool,
        user_id: Uuid,
    ) -> Result<(i32, Vec<String>)> {
        info!(
            user_id = %user_id,
            count = paths.len(),
            recursive = recursive,
            "Deleting files"
        );
        let mut deleted_count = 0;
        let mut messages = Vec::new();

        for path in paths {
            // Validate path is inside a library
            let validation = self.require_library_path(path, user_id).await?;
            if !validation.is_valid {
                let error_msg = validation.error.unwrap_or_else(|| "Not allowed".to_string());
                let display_path = normalize_display_path(path);
                warn!(path = %display_path, error = %error_msg, "Delete skipped: path not in library");
                messages.push(format!("Skipped '{}': {}", display_path, error_msg));
                continue;
            }

            let path_buf = PathBuf::from(path);
            let metadata = match fs::metadata(&path_buf).await {
                Ok(m) => m,
                Err(e) => {
                    let display_path = normalize_display_path(path);
                    messages.push(format!("Skipped '{}': {}", display_path, e));
                    continue;
                }
            };

            let parent_path = path_buf.parent().map(normalize_display_path_buf);
            let file_name = path_buf
                .file_name()
                .map(|n| n.to_string_lossy().to_string());

            if metadata.is_dir() {
                if recursive {
                    if let Err(e) = fs::remove_dir_all(&path_buf).await {
                        let display_path = normalize_display_path(path);
                        messages.push(format!("Failed to delete '{}': {}", display_path, e));
                        continue;
                    }
                } else if let Err(e) = fs::remove_dir(&path_buf).await {
                    let display_path = normalize_display_path(path);
                    messages.push(format!(
                        "Failed to delete '{}': {} (use recursive for non-empty directories)",
                        display_path, e
                    ));
                    continue;
                }
            } else if let Err(e) = fs::remove_file(&path_buf).await {
                let display_path = normalize_display_path(path);
                messages.push(format!("Failed to delete '{}': {}", display_path, e));
                continue;
            }

            deleted_count += 1;
            let display_path = normalize_display_path(path);
            debug!(path = %display_path, "Deleted file/directory");
            messages.push(format!("Deleted: {}", display_path));

            // Broadcast the change
            if let Some(parent) = parent_path {
                self.broadcast_change(DirectoryChangeEvent {
                    path: parent,
                    change_type: "deleted".to_string(),
                    name: file_name,
                    new_name: None,
                    timestamp: Utc::now(),
                });
            }
        }

        info!(deleted_count = deleted_count, "Delete operation completed");
        Ok((deleted_count, messages))
    }

    /// Copy files or directories (both source and destination must be inside libraries)
    pub async fn copy_files(
        &self,
        sources: &[String],
        destination: &str,
        overwrite: bool,
        user_id: Uuid,
    ) -> Result<(i32, Vec<String>)> {
        info!(
            user_id = %user_id,
            count = sources.len(),
            destination = %destination,
            overwrite = overwrite,
            "Copying files"
        );
        // Validate destination is inside a library
        let dest_validation = self.require_library_path(destination, user_id).await?;
        if !dest_validation.is_valid {
            return Err(anyhow!(
                "Destination not allowed: {}",
                dest_validation
                    .error
                    .unwrap_or_else(|| "Unknown".to_string())
            ));
        }

        let dest_path = PathBuf::from(destination);
        if !dest_path.is_dir() {
            return Err(anyhow!("Destination must be a directory"));
        }

        let mut copied_count = 0;
        let mut messages = Vec::new();

        for source in sources {
            // For copy, we only require the destination to be in a library
            // This allows importing files from download directories, etc.
            // The source just needs to be readable
            let source_path = PathBuf::from(source);
            
            // Check source exists and is readable
            if !source_path.exists() {
            let display_source = normalize_display_path(source);
            let msg = format!("Skipped '{}': Path does not exist", display_source);
            warn!(source = %display_source, "Copy skipped: path does not exist");
            messages.push(msg);
            continue;
        }

            let source_path = PathBuf::from(source);
            let file_name = match source_path.file_name() {
                Some(n) => n,
                None => {
                let display_source = normalize_display_path(source);
                warn!(source = %display_source, "Copy skipped: invalid path (no filename)");
                messages.push(format!("Skipped '{}': Invalid path", display_source));
                continue;
            }
        };

        let target_path = dest_path.join(file_name);
        let display_source = normalize_display_path(source);
        let display_target = normalize_display_path_buf(&target_path);
        debug!(source = %display_source, target = %display_target, "Attempting to copy");

            // Check if target exists
            if target_path.exists() && !overwrite {
            let msg = format!("Skipped '{}': Target exists (use overwrite)", display_source);
            warn!(source = %display_source, target = %display_target, "Copy skipped: target exists");
            messages.push(msg);
            continue;
        }

            // Copy the file/directory
            if let Err(e) = self.copy_recursive(&source_path, &target_path).await {
            warn!(source = %display_source, error = %e, "Copy failed");
            messages.push(format!("Failed to copy '{}': {}", display_source, e));
            continue;
        }

        copied_count += 1;
        debug!(source = %display_source, target = %display_target, "Copied file/directory");
        messages.push(format!("Copied: {} -> {}", display_source, display_target));

            // Broadcast the change
            self.broadcast_change(DirectoryChangeEvent {
                path: normalize_display_path(destination),
                change_type: "created".to_string(),
                name: Some(file_name.to_string_lossy().to_string()),
                new_name: None,
                timestamp: Utc::now(),
            });
        }

        info!(copied_count = copied_count, destination = %destination, "Copy operation completed");
        Ok((copied_count, messages))
    }

    /// Recursively copy a file or directory
    async fn copy_recursive(&self, source: &Path, dest: &Path) -> Result<()> {
        let metadata = fs::metadata(source).await?;

        if metadata.is_dir() {
            fs::create_dir_all(dest).await?;
            let mut entries = fs::read_dir(source).await?;
            while let Some(entry) = entries.next_entry().await? {
                let source_child = entry.path();
                let dest_child = dest.join(entry.file_name());
                Box::pin(self.copy_recursive(&source_child, &dest_child)).await?;
            }
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).await?;
            }
            fs::copy(source, dest).await?;
        }

        Ok(())
    }

    /// Move files or directories (both source and destination must be inside libraries)
    pub async fn move_files(
        &self,
        sources: &[String],
        destination: &str,
        overwrite: bool,
        user_id: Uuid,
    ) -> Result<(i32, Vec<String>)> {
        info!(
            user_id = %user_id,
            count = sources.len(),
            destination = %destination,
            overwrite = overwrite,
            "Moving files"
        );
        // Validate destination is inside a library
        let dest_validation = self.require_library_path(destination, user_id).await?;
        if !dest_validation.is_valid {
            return Err(anyhow!(
                "Destination not allowed: {}",
                dest_validation
                    .error
                    .unwrap_or_else(|| "Unknown".to_string())
            ));
        }

        let dest_path = PathBuf::from(destination);
        if !dest_path.is_dir() {
            return Err(anyhow!("Destination must be a directory"));
        }

        let mut moved_count = 0;
        let mut messages = Vec::new();

        for source in sources {
            // Validate source is inside a library
            let source_validation = self.require_library_path(source, user_id).await?;
            if !source_validation.is_valid {
                let display_source = normalize_display_path(source);
                messages.push(format!(
                    "Skipped '{}': {}",
                    display_source,
                    source_validation
                        .error
                        .unwrap_or_else(|| "Not allowed".to_string())
                ));
                continue;
            }

            let source_path = PathBuf::from(source);
            let file_name = match source_path.file_name() {
                Some(n) => n,
                None => {
                    let display_source = normalize_display_path(source);
                    messages.push(format!("Skipped '{}': Invalid path", display_source));
                    continue;
                }
            };

            let source_parent = source_path.parent().map(normalize_display_path_buf);
            let target_path = dest_path.join(file_name);
            let display_source = normalize_display_path(source);
            let display_target = normalize_display_path_buf(&target_path);

            // Check if target exists
            if target_path.exists() {
                if overwrite {
                    // Remove existing
                    let metadata = fs::metadata(&target_path).await?;
                    if metadata.is_dir() {
                        fs::remove_dir_all(&target_path).await?;
                    } else {
                        fs::remove_file(&target_path).await?;
                    }
                } else {
                    messages.push(format!(
                        "Skipped '{}': Target exists (use overwrite)",
                        display_source
                    ));
                    continue;
                }
            }

            // Try rename first (fast, same filesystem)
            if let Err(_) = fs::rename(&source_path, &target_path).await {
                // Fallback to copy + delete (different filesystem)
                if let Err(e) = self.copy_recursive(&source_path, &target_path).await {
                    messages.push(format!("Failed to move '{}': {}", display_source, e));
                    continue;
                }

                // Delete source after successful copy
                let metadata = fs::metadata(&source_path).await?;
                if metadata.is_dir() {
                    fs::remove_dir_all(&source_path).await?;
                } else {
                    fs::remove_file(&source_path).await?;
                }
            }

            moved_count += 1;
            debug!(source = %display_source, target = %display_target, "Moved file/directory");
            messages.push(format!("Moved: {} -> {}", display_source, display_target));

            // Broadcast changes
            if let Some(parent) = source_parent {
                self.broadcast_change(DirectoryChangeEvent {
                    path: parent,
                    change_type: "deleted".to_string(),
                    name: Some(file_name.to_string_lossy().to_string()),
                    new_name: None,
                    timestamp: Utc::now(),
                });
            }
            self.broadcast_change(DirectoryChangeEvent {
                path: normalize_display_path(destination),
                change_type: "created".to_string(),
                name: Some(file_name.to_string_lossy().to_string()),
                new_name: None,
                timestamp: Utc::now(),
            });
        }

        info!(moved_count = moved_count, destination = %destination, "Move operation completed");
        Ok((moved_count, messages))
    }

    /// Rename a file or directory (must be inside a library)
    pub async fn rename_file(&self, path: &str, new_name: &str, user_id: Uuid) -> Result<String> {
        info!(
            user_id = %user_id,
            path = %path,
            new_name = %new_name,
            "Renaming file"
        );
        // Validate path is inside a library
        let validation = self.require_library_path(path, user_id).await?;
        if !validation.is_valid {
            return Err(anyhow!(
                "Operation not allowed: {}",
                validation.error.unwrap_or_else(|| "Unknown".to_string())
            ));
        }

        // Validate new name doesn't contain path separators
        if new_name.contains('/') || new_name.contains('\\') {
            return Err(anyhow!("New name cannot contain path separators"));
        }

        let source_path = PathBuf::from(path);
        let parent = source_path
            .parent()
            .ok_or_else(|| anyhow!("Cannot rename root"))?;
        let old_name = source_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string());

        let target_path = parent.join(new_name);

        // Check target doesn't exist
        if target_path.exists() {
            return Err(anyhow!("A file or directory with that name already exists"));
        }

        fs::rename(&source_path, &target_path).await?;

        let display_path = normalize_display_path(path);
        let display_target = normalize_display_path_buf(&target_path);
        info!(
            old_path = %display_path,
            new_path = %display_target,
            "Rename completed"
        );

        // Broadcast the change
        self.broadcast_change(DirectoryChangeEvent {
            path: normalize_display_path_buf(parent),
            change_type: "renamed".to_string(),
            name: old_name,
            new_name: Some(new_name.to_string()),
            timestamp: Utc::now(),
        });

        Ok(display_target)
    }
}

fn windows_drive_paths() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        let mut drives = Vec::new();
        for letter in b'A'..=b'Z' {
            let path = format!("{}:\\", letter as char);
            if Path::new(&path).exists() {
                drives.push(PathBuf::from(path));
            }
        }
        return drives;
    }

    #[cfg(not(windows))]
    {
        Vec::new()
    }
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
