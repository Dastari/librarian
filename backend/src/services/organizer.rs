//! Media file organization service
//!
//! Handles organizing media files into proper folder structures:
//! - Creates show folders (e.g., "Show Name (2024)")
//! - Creates season folders (e.g., "Season 01")
//! - Moves files to appropriate locations
//! - Optionally renames files based on rename_style setting

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

use super::file_utils::{is_video_file, sanitize_for_filename};
use crate::db::libraries::LibraryRecord;
use crate::db::{Database, EpisodeRecord, MediaFileRecord, TvShowRecord};

/// Rename style options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameStyle {
    /// Keep original filename
    None,
    /// Clean format: "Show Name - S01E01 - Episode Title.ext"
    Clean,
    /// Preserve info: "Show Name - S01E01 - Episode Title [1080p HEVC Group].ext"
    PreserveInfo,
}

impl RenameStyle {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "clean" => Self::Clean,
            "preserve_info" => Self::PreserveInfo,
            _ => Self::None,
        }
    }

    /// Convert to string representation - for future serialization
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Clean => "clean",
            Self::PreserveInfo => "preserve_info",
        }
    }
}

/// Result of organizing a file
#[derive(Debug)]
pub struct OrganizeResult {
    pub file_id: Uuid,
    /// Original file path (for rollback purposes)
    #[allow(dead_code)]
    pub original_path: String,
    pub new_path: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Media file organizer service
pub struct OrganizerService {
    db: Database,
}

impl OrganizerService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Get effective organize settings for a show (considering overrides)
    ///
    /// Returns (organize_files, rename_style, post_download_action)
    /// - organize_files: Whether to organize files into folders
    /// - rename_style: How to rename files (None, Clean, PreserveInfo)
    /// - post_download_action: What to do with files (copy, move, hardlink)
    pub async fn get_show_organize_settings(
        &self,
        show: &TvShowRecord,
    ) -> Result<(bool, RenameStyle)> {
        let (organize_files, rename_style, _action) = self.get_full_organize_settings(show).await?;
        Ok((organize_files, rename_style))
    }

    /// Get full organize settings for a show including post_download_action
    ///
    /// Returns (organize_files, rename_style, post_download_action)
    pub async fn get_full_organize_settings(
        &self,
        show: &TvShowRecord,
    ) -> Result<(bool, RenameStyle, String)> {
        let library = self
            .db
            .libraries()
            .get_by_id(show.library_id)
            .await?
            .context("Library not found")?;

        // Use show override if set, otherwise use library setting
        let organize_files = show
            .organize_files_override
            .unwrap_or(library.organize_files);

        let rename_style = match &show.rename_style_override {
            Some(style) => RenameStyle::from_str(style),
            None => RenameStyle::from_str(&library.rename_style),
        };

        // post_download_action comes from library (no show override currently)
        let post_download_action = library.post_download_action.clone();

        Ok((organize_files, rename_style, post_download_action))
    }

    /// Generate the organized path for a media file
    ///
    /// If `naming_pattern` is provided, it will be used to generate the path.
    /// Otherwise, falls back to the legacy `rename_style` behavior.
    ///
    /// IMPORTANT: If `show.path` is set (i.e., the show was added via the UI with a specific
    /// folder structure), we use that path as the base. This ensures files are organized
    /// into the existing folder structure (e.g., "Star Trek Deep Space Nine (1993)/Season 1/")
    /// rather than generating a new folder name from the show name.
    pub fn generate_organized_path(
        &self,
        library_path: &str,
        show: &TvShowRecord,
        episode: &EpisodeRecord,
        original_filename: &str,
        rename_style: RenameStyle,
        naming_pattern: Option<&str>,
    ) -> PathBuf {
        let ext = Path::new(original_filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mkv");

        // If naming pattern is provided, use it
        if let Some(pattern) = naming_pattern {
            if !pattern.is_empty() {
                let relative_path = apply_naming_pattern(pattern, show, episode, ext);
                return PathBuf::from(library_path).join(relative_path);
            }
        }

        // Determine the show folder path:
        // 1. If show.path is set, use it (respects existing folder structure)
        // 2. Otherwise, generate from show name and year
        let show_path = if let Some(ref path) = show.path {
            // Use the existing show path that was set when the show was added
            debug!(
                show = %show.name,
                existing_path = %path,
                "Using existing show path for organization"
            );
            PathBuf::from(path)
        } else {
            // Legacy behavior: generate show folder name
            let show_folder = match show.year {
                Some(year) => format!("{} ({})", sanitize_for_filename(&show.name), year),
                None => sanitize_for_filename(&show.name),
            };
            debug!(
                show = %show.name,
                generated_folder = %show_folder,
                "No show.path set, generating folder from name/year"
            );
            PathBuf::from(library_path).join(&show_folder)
        };

        // Create season folder name: "Season 01"
        let season_folder = format!("Season {:02}", episode.season);

        // Generate filename based on rename style
        let filename = match rename_style {
            RenameStyle::None => original_filename.to_string(),
            RenameStyle::Clean => {
                let episode_title = episode
                    .title
                    .as_ref()
                    .map(|t| format!(" - {}", sanitize_for_filename(t)))
                    .unwrap_or_default();
                format!(
                    "{} - S{:02}E{:02}{}.{}",
                    sanitize_for_filename(&show.name),
                    episode.season,
                    episode.episode,
                    episode_title,
                    ext
                )
            }
            RenameStyle::PreserveInfo => {
                // Extract quality info from original filename
                let quality_info = extract_quality_info(original_filename);
                let episode_title = episode
                    .title
                    .as_ref()
                    .map(|t| format!(" - {}", sanitize_for_filename(t)))
                    .unwrap_or_default();

                if quality_info.is_empty() {
                    format!(
                        "{} - S{:02}E{:02}{}.{}",
                        sanitize_for_filename(&show.name),
                        episode.season,
                        episode.episode,
                        episode_title,
                        ext
                    )
                } else {
                    format!(
                        "{} - S{:02}E{:02}{} [{}].{}",
                        sanitize_for_filename(&show.name),
                        episode.season,
                        episode.episode,
                        episode_title,
                        quality_info,
                        ext
                    )
                }
            }
        };

        show_path
            .join(&season_folder)
            .join(&filename)
    }

    /// Organize a single media file
    ///
    /// `action` specifies how to handle the file:
    /// - "copy": Copy file to new location, keep original (for seeding)
    /// - "move": Move file to new location, delete original
    /// - "hardlink": Create hard link to new location, keep original
    pub async fn organize_file(
        &self,
        file: &MediaFileRecord,
        show: &TvShowRecord,
        episode: &EpisodeRecord,
        library_path: &str,
        rename_style: RenameStyle,
        naming_pattern: Option<&str>,
        action: &str,
        dry_run: bool,
    ) -> Result<OrganizeResult> {
        let original_path = file.path.clone();
        let original_filename = Path::new(&original_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.mkv");

        // Get pattern from library, or fetch default from database for TV
        let effective_pattern = match naming_pattern {
            Some(p) if !p.is_empty() => Some(p.to_string()),
            _ => {
                // Fetch default from database
                let pattern = self
                    .db
                    .naming_patterns()
                    .get_default_pattern_for_type("tv")
                    .await?;
                Some(pattern)
            }
        };

        let new_path = self.generate_organized_path(
            library_path,
            show,
            episode,
            original_filename,
            rename_style,
            effective_pattern.as_deref(),
        );

        let new_path_str = new_path.to_string_lossy().to_string();

        // Skip if already at the correct location
        if original_path == new_path_str {
            debug!(
                file_id = %file.id,
                path = %original_path,
                "File already at correct location"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: true,
                error: None,
            });
        }

        if dry_run {
            info!(
                file_id = %file.id,
                original = %original_path,
                new = %new_path_str,
                action = %action,
                "[DRY RUN] Would organize file"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: true,
                error: None,
            });
        }

        // Create parent directories
        if let Some(parent) = new_path.parent()
            && let Err(e) = tokio::fs::create_dir_all(parent).await
        {
            error!(
                file_id = %file.id,
                path = %parent.display(),
                error = %e,
                "Failed to create directory"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: false,
                error: Some(format!("Failed to create directory: {}", e)),
            });
        }

        let source_path = Path::new(&original_path);

        // Check for conflicts - if target exists with different size, it's a conflict
        if new_path.exists() {
            let target_size = tokio::fs::metadata(&new_path)
                .await
                .map(|m| m.len() as i64)
                .unwrap_or(0);
            let source_size = file.size_bytes;

            if target_size != source_size {
                // Different file - conflict!
                // Move the existing file to the conflicts folder instead of failing
                let conflicts_folder = self.get_conflicts_folder(library_path).await;

                if let Some(conflict_path) =
                    self.move_to_conflicts(&new_path, &conflicts_folder).await?
                {
                    info!(
                        file_id = %file.id,
                        original_target = %new_path_str,
                        conflict_path = %conflict_path,
                        "Moved conflicting file to conflicts folder"
                    );
                    // Continue with normal organization - target is now clear
                } else {
                    // Couldn't move to conflicts, return error
                    let error_msg = format!(
                        "Target file exists with different size (source: {} bytes, target: {} bytes)",
                        source_size, target_size
                    );
                    warn!(
                        file_id = %file.id,
                        source_path = %original_path,
                        target_path = %new_path_str,
                        source_size = source_size,
                        target_size = target_size,
                        "File conflict detected and could not move to conflicts folder"
                    );

                    // Mark as conflicted in database
                    self.db
                        .media_files()
                        .mark_conflicted(file.id, &error_msg)
                        .await?;

                    return Ok(OrganizeResult {
                        file_id: file.id,
                        original_path,
                        new_path: new_path_str,
                        success: false,
                        error: Some(error_msg),
                    });
                }
            } else {
                // Same size - assume it's the same file
                // Check if another record already has this path (to avoid duplicate key violation)
                if let Some(existing) = self.db.media_files().get_by_path(&new_path_str).await? {
                    if existing.id != file.id {
                        // Another record already has this path - this file is a duplicate
                        // Delete the source file from disk AND the database record
                        info!(
                            file_id = %file.id,
                            existing_file_id = %existing.id,
                            source_path = %original_path,
                            target_path = %new_path_str,
                            "Duplicate file detected, deleting source file and DB record"
                        );

                        // Delete the source file from disk (if it's different from target)
                        if original_path != new_path_str && source_path.exists() {
                            if let Err(e) = tokio::fs::remove_file(source_path).await {
                                warn!(
                                    path = %original_path,
                                    error = %e,
                                    "Failed to delete duplicate source file"
                                );
                            } else {
                                debug!(
                                    "Deleted duplicate source file: {}",
                                    std::path::Path::new(&original_path)
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or(&original_path)
                                );
                            }
                        }

                        self.db.media_files().delete(file.id).await?;

                        return Ok(OrganizeResult {
                            file_id: file.id,
                            original_path,
                            new_path: new_path_str,
                            success: true,
                            error: None,
                        });
                    }
                }

                // Target exists with same size, but no other record has this path
                // This means the file is already at the correct location
                // Just update the database record
                info!(
                    file_id = %file.id,
                    path = %new_path_str,
                    "Target file already exists with same size, marking as organized"
                );

                self.db
                    .media_files()
                    .mark_organized(file.id, &new_path_str, &original_path)
                    .await?;

                return Ok(OrganizeResult {
                    file_id: file.id,
                    original_path,
                    new_path: new_path_str,
                    success: true,
                    error: None,
                });
            }
        }

        // Determine the effective action:
        // - If source is already in the library folder, use "move" (rename in-place)
        // - If source is in downloads folder, use the specified action (hardlink/copy to preserve seeding)
        let source_in_library = source_path.starts_with(library_path);
        let effective_action = if source_in_library {
            // File is already in library - just rename it, don't create duplicates
            "move"
        } else {
            // File is in downloads - use the specified action to preserve seeding capability
            action
        };

        if source_in_library && effective_action != action {
            debug!(
                file_id = %file.id,
                original_action = %action,
                effective_action = %effective_action,
                "File is already in library, using move instead of {}", action
            );
        }

        // Perform file operation based on effective action
        let operation_result = match effective_action {
            "move" => {
                // Try rename first (same filesystem), fall back to copy+delete
                match tokio::fs::rename(source_path, &new_path).await {
                    Ok(_) => Ok(()),
                    Err(_) => {
                        // Cross-filesystem: copy then delete
                        tokio::fs::copy(source_path, &new_path).await?;
                        tokio::fs::remove_file(source_path).await?;
                        Ok(())
                    }
                }
            }
            "hardlink" => {
                // Create hard link (keeps original for seeding)
                #[cfg(unix)]
                {
                    match tokio::fs::hard_link(source_path, &new_path).await {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            warn!(
                                file_id = %file.id,
                                error = %e,
                                "Hardlink failed, falling back to copy"
                            );
                            // Fall back to copy if hardlink fails
                            tokio::fs::copy(source_path, &new_path).await.map(|_| ())
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    // Windows: just copy
                    tokio::fs::copy(source_path, &new_path).await.map(|_| ())
                }
            }
            _ => {
                // Default: copy (preserves original for seeding)
                tokio::fs::copy(source_path, &new_path).await.map(|_| ())
            }
        };

        if let Err(e) = operation_result {
            error!(
                file_id = %file.id,
                action = %effective_action,
                error = %e,
                "Failed to organize file"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: false,
                error: Some(format!("Failed to {} file: {}", effective_action, e)),
            });
        }

        // Check for duplicate path in database before updating
        if let Some(existing) = self.db.media_files().get_by_path(&new_path_str).await? {
            if existing.id != file.id {
                // Another record has this path - delete this duplicate
                info!(
                    file_id = %file.id,
                    existing_file_id = %existing.id,
                    path = %new_path_str,
                    "Duplicate file detected after organize, deleting duplicate record"
                );
                self.db.media_files().delete(file.id).await?;

                return Ok(OrganizeResult {
                    file_id: file.id,
                    original_path,
                    new_path: new_path_str,
                    success: true,
                    error: None,
                });
            }
        }

        // Update database
        self.db
            .media_files()
            .mark_organized(file.id, &new_path_str, &original_path)
            .await?;

        let original_name = std::path::Path::new(&original_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&original_path);
        info!(
            file_id = %file.id,
            action = %effective_action,
            original = %original_path,
            new = %new_path_str,
            "Organized '{}' ({}) → {}",
            original_name, effective_action, new_path_str
        );

        Ok(OrganizeResult {
            file_id: file.id,
            original_path,
            new_path: new_path_str,
            success: true,
            error: None,
        })
    }

    /// Generate the organized path for a movie file
    ///
    /// Uses the library's naming_pattern if set, otherwise falls back to DEFAULT_MOVIE_NAMING_PATTERN.
    pub fn generate_movie_organized_path(
        &self,
        library_path: &str,
        movie: &crate::db::MovieRecord,
        original_filename: &str,
        naming_pattern: Option<&str>,
    ) -> PathBuf {
        let ext = Path::new(original_filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mkv");

        // Use provided pattern or default movie pattern
        let pattern = naming_pattern.unwrap_or(DEFAULT_MOVIE_NAMING_PATTERN);

        let relative_path = apply_movie_naming_pattern(pattern, movie, original_filename, ext);
        PathBuf::from(library_path).join(relative_path)
    }

    /// Organize a movie file
    ///
    /// `action` specifies how to handle the file:
    /// - "copy": Copy file to new location, keep original (for seeding)
    /// - "move": Move file to new location, delete original
    /// - "hardlink": Create hard link to new location, keep original
    pub async fn organize_movie_file(
        &self,
        file: &MediaFileRecord,
        movie: &crate::db::MovieRecord,
        library_path: &str,
        naming_pattern: Option<&str>,
        action: &str,
        dry_run: bool,
    ) -> Result<OrganizeResult> {
        let original_path = file.path.clone();
        let original_filename = Path::new(&original_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.mkv");

        // Get pattern from library, or fetch default from database
        let effective_pattern = match naming_pattern {
            Some(p) => p.to_string(),
            None => {
                self.db
                    .naming_patterns()
                    .get_default_pattern_for_type("movies")
                    .await?
            }
        };

        let new_path = self.generate_movie_organized_path(
            library_path,
            movie,
            original_filename,
            Some(&effective_pattern),
        );

        let new_path_str = new_path.to_string_lossy().to_string();

        // Skip if already at the correct location
        if original_path == new_path_str {
            debug!(
                file_id = %file.id,
                path = %original_path,
                "Movie file already at correct location"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: true,
                error: None,
            });
        }

        if dry_run {
            info!(
                file_id = %file.id,
                original = %original_path,
                new = %new_path_str,
                action = %action,
                "[DRY RUN] Would organize movie file"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: true,
                error: None,
            });
        }

        // Create parent directories
        if let Some(parent) = new_path.parent()
            && let Err(e) = tokio::fs::create_dir_all(parent).await
        {
            error!(
                file_id = %file.id,
                path = %parent.display(),
                error = %e,
                "Failed to create directory for movie"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: false,
                error: Some(format!("Failed to create directory: {}", e)),
            });
        }

        let source_path = Path::new(&original_path);

        // Check for conflicts - if target exists with different size, it's a conflict
        if new_path.exists() {
            let target_size = tokio::fs::metadata(&new_path)
                .await
                .map(|m| m.len() as i64)
                .unwrap_or(0);
            let source_size = file.size_bytes;

            if target_size != source_size {
                // Different file - conflict!
                // Move the existing file to the conflicts folder instead of failing
                let conflicts_folder = self.get_conflicts_folder(library_path).await;

                if let Some(conflict_path) =
                    self.move_to_conflicts(&new_path, &conflicts_folder).await?
                {
                    info!(
                        file_id = %file.id,
                        original_target = %new_path_str,
                        conflict_path = %conflict_path,
                        movie = %movie.title,
                        "Moved conflicting movie file to conflicts folder"
                    );
                    // Continue with normal organization - target is now clear
                } else {
                    // Couldn't move to conflicts, return error
                    let error_msg = format!(
                        "Target file exists with different size (source: {} bytes, target: {} bytes)",
                        source_size, target_size
                    );
                    warn!(
                        file_id = %file.id,
                        source_path = %original_path,
                        target_path = %new_path_str,
                        source_size = source_size,
                        target_size = target_size,
                        "Movie file conflict detected and could not move to conflicts folder"
                    );

                    // Mark as conflicted in database
                    self.db
                        .media_files()
                        .mark_conflicted(file.id, &error_msg)
                        .await?;

                    return Ok(OrganizeResult {
                        file_id: file.id,
                        original_path,
                        new_path: new_path_str,
                        success: false,
                        error: Some(error_msg),
                    });
                }
            } else {
                // Same size - assume it's the same file
                // Check if another record already has this path (to avoid duplicate key violation)
                if let Some(existing) = self.db.media_files().get_by_path(&new_path_str).await? {
                    if existing.id != file.id {
                        // Another record already has this path - this file is a duplicate
                        // Delete the source file from disk AND the database record
                        info!(
                            file_id = %file.id,
                            existing_file_id = %existing.id,
                            source_path = %original_path,
                            target_path = %new_path_str,
                            movie = %movie.title,
                            "Duplicate movie file detected, deleting source file and DB record"
                        );

                        // Delete the source file from disk (if it's different from target)
                        if original_path != new_path_str && source_path.exists() {
                            if let Err(e) = tokio::fs::remove_file(source_path).await {
                                warn!(
                                    path = %original_path,
                                    error = %e,
                                    "Failed to delete duplicate source movie file"
                                );
                            } else {
                                debug!("Deleted duplicate source movie file");
                            }
                        }

                        self.db.media_files().delete(file.id).await?;

                        return Ok(OrganizeResult {
                            file_id: file.id,
                            original_path,
                            new_path: new_path_str,
                            success: true,
                            error: None,
                        });
                    }
                }

                // No other record has this path, just update the database
                info!(
                    file_id = %file.id,
                    path = %new_path_str,
                    movie = %movie.title,
                    "Movie target file already exists with same size, marking as organized"
                );

                self.db
                    .media_files()
                    .mark_organized(file.id, &new_path_str, &original_path)
                    .await?;

                return Ok(OrganizeResult {
                    file_id: file.id,
                    original_path,
                    new_path: new_path_str,
                    success: true,
                    error: None,
                });
            }
        }

        // Determine the effective action:
        // - If source is already in the library folder, use "move" (rename in-place)
        // - If source is in downloads folder, use the specified action
        let source_in_library = source_path.starts_with(library_path);
        let effective_action = if source_in_library { "move" } else { action };

        if source_in_library && effective_action != action {
            debug!(
                file_id = %file.id,
                original_action = %action,
                effective_action = %effective_action,
                "Movie file is already in library, using move instead of {}", action
            );
        }

        // Perform file operation based on effective action
        let operation_result = match effective_action {
            "move" => {
                match tokio::fs::rename(source_path, &new_path).await {
                    Ok(_) => Ok(()),
                    Err(_) => {
                        // Cross-filesystem: copy then delete
                        tokio::fs::copy(source_path, &new_path).await?;
                        tokio::fs::remove_file(source_path).await?;
                        Ok(())
                    }
                }
            }
            "hardlink" => {
                #[cfg(unix)]
                {
                    match tokio::fs::hard_link(source_path, &new_path).await {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            warn!(
                                file_id = %file.id,
                                error = %e,
                                "Hardlink failed for movie, falling back to copy"
                            );
                            tokio::fs::copy(source_path, &new_path).await.map(|_| ())
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    tokio::fs::copy(source_path, &new_path).await.map(|_| ())
                }
            }
            _ => {
                // Default: copy
                tokio::fs::copy(source_path, &new_path).await.map(|_| ())
            }
        };

        if let Err(e) = operation_result {
            error!(
                file_id = %file.id,
                action = %effective_action,
                error = %e,
                movie = %movie.title,
                source = %original_path,
                target = %new_path_str,
                "Failed to organize movie file"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: false,
                error: Some(format!("Failed to {} file: {}", effective_action, e)),
            });
        }

        // Check for duplicate path in database before updating
        if let Some(existing) = self.db.media_files().get_by_path(&new_path_str).await? {
            if existing.id != file.id {
                // Another record has this path - delete this duplicate
                info!(
                    file_id = %file.id,
                    existing_file_id = %existing.id,
                    path = %new_path_str,
                    movie = %movie.title,
                    "Duplicate movie file detected after organize, deleting duplicate record"
                );
                self.db.media_files().delete(file.id).await?;

                return Ok(OrganizeResult {
                    file_id: file.id,
                    original_path,
                    new_path: new_path_str,
                    success: true,
                    error: None,
                });
            }
        }

        // Update database
        if let Err(e) = self
            .db
            .media_files()
            .mark_organized(file.id, &new_path_str, &original_path)
            .await
        {
            error!(
                file_id = %file.id,
                movie = %movie.title,
                error = %e,
                source = %original_path,
                target = %new_path_str,
                "Failed to update database after organizing movie file"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: false,
                error: Some(format!("Database error: {}", e)),
            });
        }

        info!(
            file_id = %file.id,
            movie = %movie.title,
            action = %effective_action,
            original = %original_path,
            new = %new_path_str,
            "Organized movie '{}' ({}) → {}",
            movie.title, effective_action, new_path_str
        );

        Ok(OrganizeResult {
            file_id: file.id,
            original_path,
            new_path: new_path_str,
            success: true,
            error: None,
        })
    }

    /// Generate the organized path for a music album file
    pub fn generate_music_organized_path(
        &self,
        library_path: &str,
        artist_name: &str,
        album: &crate::db::AlbumRecord,
        track: Option<&crate::db::TrackRecord>,
        original_filename: &str,
        naming_pattern: Option<&str>,
    ) -> PathBuf {
        let ext = Path::new(original_filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp3");

        let pattern = naming_pattern.unwrap_or(DEFAULT_MUSIC_NAMING_PATTERN);
        let relative_path =
            apply_music_naming_pattern(pattern, artist_name, album, track, original_filename, ext);
        PathBuf::from(library_path).join(relative_path)
    }

    /// Organize a music file (album track)
    pub async fn organize_music_file(
        &self,
        file: &MediaFileRecord,
        artist_name: &str,
        album: &crate::db::AlbumRecord,
        track: Option<&crate::db::TrackRecord>,
        library_path: &str,
        naming_pattern: Option<&str>,
        action: &str,
        dry_run: bool,
    ) -> Result<OrganizeResult> {
        let original_path = file.path.clone();
        let original_filename = Path::new(&original_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.mp3");

        // Get pattern from library, or fetch default from database
        let effective_pattern = match naming_pattern {
            Some(p) => p.to_string(),
            None => {
                self.db
                    .naming_patterns()
                    .get_default_pattern_for_type("music")
                    .await?
            }
        };

        let new_path = self.generate_music_organized_path(
            library_path,
            artist_name,
            album,
            track,
            original_filename,
            Some(&effective_pattern),
        );

        let new_path_str = new_path.to_string_lossy().to_string();

        // Skip if already at the correct location
        if original_path == new_path_str {
            debug!(
                file_id = %file.id,
                path = %original_path,
                "Music file already at correct location"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: true,
                error: None,
            });
        }

        if dry_run {
            info!(
                file_id = %file.id,
                original = %original_path,
                new = %new_path_str,
                action = %action,
                "[DRY RUN] Would organize music file"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: true,
                error: None,
            });
        }

        // Create parent directories
        if let Some(parent) = new_path.parent()
            && let Err(e) = tokio::fs::create_dir_all(parent).await
        {
            error!(
                file_id = %file.id,
                path = %parent.display(),
                error = %e,
                "Failed to create directory for music"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: false,
                error: Some(format!("Failed to create directory: {}", e)),
            });
        }

        let source_path = Path::new(&original_path);

        // Check if target exists
        if new_path.exists() {
            // Check if another record already has this path (to avoid duplicate key violation)
            if let Some(existing) = self.db.media_files().get_by_path(&new_path_str).await? {
                if existing.id != file.id {
                    // Another record already has this path - this file is a duplicate
                    info!(
                        file_id = %file.id,
                        existing_file_id = %existing.id,
                        source_path = %original_path,
                        target_path = %new_path_str,
                        artist = %artist_name,
                        album = %album.name,
                        "Duplicate music file detected, deleting source file and DB record"
                    );

                    // Delete the source file from disk (if it's different from target)
                    if original_path != new_path_str && source_path.exists() {
                        if let Err(e) = tokio::fs::remove_file(source_path).await {
                            warn!(
                                path = %original_path,
                                error = %e,
                                "Failed to delete duplicate source music file"
                            );
                        } else {
                            debug!("Deleted duplicate source music file");
                        }
                    }

                    self.db.media_files().delete(file.id).await?;

                    return Ok(OrganizeResult {
                        file_id: file.id,
                        original_path,
                        new_path: new_path_str,
                        success: true,
                        error: None,
                    });
                }
            }

            info!(
                file_id = %file.id,
                path = %new_path_str,
                artist = %artist_name,
                album = %album.name,
                "Music target file already exists, marking as organized"
            );

            if let Err(e) = self
                .db
                .media_files()
                .mark_organized(file.id, &new_path_str, &original_path)
                .await
            {
                error!(
                    file_id = %file.id,
                    error = %e,
                    artist = %artist_name,
                    album = %album.name,
                    source = %original_path,
                    target = %new_path_str,
                    "Failed to update database after organizing music file"
                );
                return Ok(OrganizeResult {
                    file_id: file.id,
                    original_path,
                    new_path: new_path_str,
                    success: false,
                    error: Some(format!("Database error: {}", e)),
                });
            }

            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: true,
                error: None,
            });
        }

        // Determine effective action
        let source_in_library = source_path.starts_with(library_path);
        let effective_action = if source_in_library { "move" } else { action };

        // Perform file operation
        let operation_result = match effective_action {
            "move" => match tokio::fs::rename(source_path, &new_path).await {
                Ok(_) => Ok(()),
                Err(_) => {
                    tokio::fs::copy(source_path, &new_path).await?;
                    tokio::fs::remove_file(source_path).await?;
                    Ok(())
                }
            },
            "hardlink" => {
                #[cfg(unix)]
                {
                    match tokio::fs::hard_link(source_path, &new_path).await {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            warn!(file_id = %file.id, error = %e, "Hardlink failed for music, falling back to copy");
                            tokio::fs::copy(source_path, &new_path).await.map(|_| ())
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    tokio::fs::copy(source_path, &new_path).await.map(|_| ())
                }
            }
            _ => tokio::fs::copy(source_path, &new_path).await.map(|_| ()),
        };

        if let Err(e) = operation_result {
            error!(
                file_id = %file.id,
                action = %effective_action,
                error = %e,
                artist = %artist_name,
                album = %album.name,
                source = %original_path,
                target = %new_path_str,
                "Failed to organize music file"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: false,
                error: Some(format!("Failed to {} file: {}", effective_action, e)),
            });
        }

        // Check for duplicate path in database before updating
        if let Some(existing) = self.db.media_files().get_by_path(&new_path_str).await? {
            if existing.id != file.id {
                // Another record has this path - delete this duplicate
                info!(
                    file_id = %file.id,
                    existing_file_id = %existing.id,
                    path = %new_path_str,
                    artist = %artist_name,
                    album = %album.name,
                    "Duplicate music file detected after organize, deleting duplicate record"
                );
                self.db.media_files().delete(file.id).await?;

                return Ok(OrganizeResult {
                    file_id: file.id,
                    original_path,
                    new_path: new_path_str,
                    success: true,
                    error: None,
                });
            }
        }

        // Update database
        if let Err(e) = self
            .db
            .media_files()
            .mark_organized(file.id, &new_path_str, &original_path)
            .await
        {
            error!(
                file_id = %file.id,
                error = %e,
                artist = %artist_name,
                album = %album.name,
                source = %original_path,
                target = %new_path_str,
                "Failed to update database after organizing music file"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: false,
                error: Some(format!("Database error: {}", e)),
            });
        }

        info!(
            file_id = %file.id,
            artist = %artist_name,
            album = %album.name,
            action = %effective_action,
            new = %new_path_str,
            "Organized music: {} - {} ({}) → {}",
            artist_name, album.name, effective_action, new_path_str
        );

        Ok(OrganizeResult {
            file_id: file.id,
            original_path,
            new_path: new_path_str,
            success: true,
            error: None,
        })
    }

    /// Generate the organized path for an audiobook file
    pub fn generate_audiobook_organized_path(
        &self,
        library_path: &str,
        author_name: &str,
        audiobook: &crate::db::AudiobookRecord,
        original_filename: &str,
        naming_pattern: Option<&str>,
    ) -> PathBuf {
        let ext = Path::new(original_filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("m4b");

        let pattern = naming_pattern.unwrap_or(DEFAULT_AUDIOBOOK_NAMING_PATTERN);
        let relative_path =
            apply_audiobook_naming_pattern(pattern, author_name, audiobook, original_filename, ext);
        PathBuf::from(library_path).join(relative_path)
    }

    /// Organize an audiobook file
    pub async fn organize_audiobook_file(
        &self,
        file: &MediaFileRecord,
        author_name: &str,
        audiobook: &crate::db::AudiobookRecord,
        library_path: &str,
        naming_pattern: Option<&str>,
        action: &str,
        dry_run: bool,
    ) -> Result<OrganizeResult> {
        let original_path = file.path.clone();
        let original_filename = Path::new(&original_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.m4b");

        // Get pattern from library, or fetch default from database
        let effective_pattern = match naming_pattern {
            Some(p) => p.to_string(),
            None => {
                self.db
                    .naming_patterns()
                    .get_default_pattern_for_type("audiobooks")
                    .await?
            }
        };

        let new_path = self.generate_audiobook_organized_path(
            library_path,
            author_name,
            audiobook,
            original_filename,
            Some(&effective_pattern),
        );

        let new_path_str = new_path.to_string_lossy().to_string();

        // Skip if already at the correct location
        if original_path == new_path_str {
            debug!(
                file_id = %file.id,
                path = %original_path,
                "Audiobook file already at correct location"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: true,
                error: None,
            });
        }

        if dry_run {
            info!(
                file_id = %file.id,
                original = %original_path,
                new = %new_path_str,
                action = %action,
                "[DRY RUN] Would organize audiobook file"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: true,
                error: None,
            });
        }

        // Create parent directories
        if let Some(parent) = new_path.parent()
            && let Err(e) = tokio::fs::create_dir_all(parent).await
        {
            error!(
                file_id = %file.id,
                path = %parent.display(),
                error = %e,
                "Failed to create directory for audiobook"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: false,
                error: Some(format!("Failed to create directory: {}", e)),
            });
        }

        let source_path = Path::new(&original_path);

        // Check if target exists
        if new_path.exists() {
            // Check if another record already has this path (to avoid duplicate key violation)
            if let Some(existing) = self.db.media_files().get_by_path(&new_path_str).await? {
                if existing.id != file.id {
                    // Another record already has this path - this file is a duplicate
                    info!(
                        file_id = %file.id,
                        existing_file_id = %existing.id,
                        source_path = %original_path,
                        target_path = %new_path_str,
                        author = %author_name,
                        audiobook = %audiobook.title,
                        "Duplicate audiobook file detected, deleting source file and DB record"
                    );

                    // Delete the source file from disk (if it's different from target)
                    if original_path != new_path_str && source_path.exists() {
                        if let Err(e) = tokio::fs::remove_file(source_path).await {
                            warn!(
                                path = %original_path,
                                error = %e,
                                "Failed to delete duplicate source audiobook file"
                            );
                        } else {
                            debug!("Deleted duplicate source audiobook file");
                        }
                    }

                    self.db.media_files().delete(file.id).await?;

                    return Ok(OrganizeResult {
                        file_id: file.id,
                        original_path,
                        new_path: new_path_str,
                        success: true,
                        error: None,
                    });
                }
            }

            info!(
                file_id = %file.id,
                path = %new_path_str,
                author = %author_name,
                audiobook = %audiobook.title,
                "Audiobook target file already exists, marking as organized"
            );

            if let Err(e) = self
                .db
                .media_files()
                .mark_organized(file.id, &new_path_str, &original_path)
                .await
            {
                error!(
                    file_id = %file.id,
                    error = %e,
                    author = %author_name,
                    audiobook = %audiobook.title,
                    source = %original_path,
                    target = %new_path_str,
                    "Failed to update database after organizing audiobook file"
                );
                return Ok(OrganizeResult {
                    file_id: file.id,
                    original_path,
                    new_path: new_path_str,
                    success: false,
                    error: Some(format!("Database error: {}", e)),
                });
            }

            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: true,
                error: None,
            });
        }

        // Determine effective action
        let source_in_library = source_path.starts_with(library_path);
        let effective_action = if source_in_library { "move" } else { action };

        // Perform file operation
        let operation_result = match effective_action {
            "move" => match tokio::fs::rename(source_path, &new_path).await {
                Ok(_) => Ok(()),
                Err(_) => {
                    tokio::fs::copy(source_path, &new_path).await?;
                    tokio::fs::remove_file(source_path).await?;
                    Ok(())
                }
            },
            "hardlink" => {
                #[cfg(unix)]
                {
                    match tokio::fs::hard_link(source_path, &new_path).await {
                        Ok(_) => Ok(()),
                        Err(e) => {
                            warn!(file_id = %file.id, error = %e, "Hardlink failed for audiobook, falling back to copy");
                            tokio::fs::copy(source_path, &new_path).await.map(|_| ())
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    tokio::fs::copy(source_path, &new_path).await.map(|_| ())
                }
            }
            _ => tokio::fs::copy(source_path, &new_path).await.map(|_| ()),
        };

        if let Err(e) = operation_result {
            error!(
                file_id = %file.id,
                action = %effective_action,
                error = %e,
                author = %author_name,
                audiobook = %audiobook.title,
                source = %original_path,
                target = %new_path_str,
                "Failed to organize audiobook file"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: false,
                error: Some(format!("Failed to {} file: {}", effective_action, e)),
            });
        }

        // Check for duplicate path in database before updating
        if let Some(existing) = self.db.media_files().get_by_path(&new_path_str).await? {
            if existing.id != file.id {
                // Another record has this path - delete this duplicate
                info!(
                    file_id = %file.id,
                    existing_file_id = %existing.id,
                    path = %new_path_str,
                    author = %author_name,
                    audiobook = %audiobook.title,
                    "Duplicate audiobook file detected after organize, deleting duplicate record"
                );
                self.db.media_files().delete(file.id).await?;

                return Ok(OrganizeResult {
                    file_id: file.id,
                    original_path,
                    new_path: new_path_str,
                    success: true,
                    error: None,
                });
            }
        }

        // Update database
        if let Err(e) = self
            .db
            .media_files()
            .mark_organized(file.id, &new_path_str, &original_path)
            .await
        {
            error!(
                file_id = %file.id,
                error = %e,
                author = %author_name,
                audiobook = %audiobook.title,
                source = %original_path,
                target = %new_path_str,
                "Failed to update database after organizing audiobook file"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: false,
                error: Some(format!("Database error: {}", e)),
            });
        }

        info!(
            file_id = %file.id,
            author = %author_name,
            audiobook = %audiobook.title,
            action = %effective_action,
            new = %new_path_str,
            "Organized audiobook: {} by {} ({}) → {}",
            audiobook.title, author_name, effective_action, new_path_str
        );

        Ok(OrganizeResult {
            file_id: file.id,
            original_path,
            new_path: new_path_str,
            success: true,
            error: None,
        })
    }

    /// Remove duplicate files for the same episode
    ///
    /// For each episode that has multiple media files:
    /// 1. Keep the file that is already at the canonical path (if any)
    /// 2. Otherwise, keep the highest quality file (by resolution/codec)
    /// 3. Delete the other files from both disk and database
    ///
    /// Returns the number of duplicates removed
    pub async fn deduplicate_library(&self, library_id: Uuid) -> Result<DeduplicationResult> {
        let library = self
            .db
            .libraries()
            .get_by_id(library_id)
            .await?
            .context("Library not found")?;

        let mut duplicates_removed = 0;
        let mut files_deleted = 0;
        let mut messages = Vec::new();

        // Get all shows in this library
        let shows = self.db.tv_shows().list_by_library(library_id).await?;

        for show in shows {
            let episodes = self.db.episodes().list_by_show(show.id).await?;

            for episode in episodes {
                // Get all media files linked to this episode
                let files = self.db.media_files().list_by_episode(episode.id).await?;

                if files.len() <= 1 {
                    continue; // No duplicates
                }

                // Generate the canonical path for this episode
                let (organize_files, rename_style) = self.get_show_organize_settings(&show).await?;

                // Find the "best" file to keep
                // Priority: 1. Already at canonical path, 2. Highest resolution, 3. Best codec
                let canonical_path = if organize_files {
                    let dummy_filename = "file.mkv";
                    let path = self.generate_organized_path(
                        &library.path,
                        &show,
                        &episode,
                        dummy_filename,
                        rename_style,
                        library.naming_pattern.as_deref(),
                    );
                    // Get the parent directory (without filename)
                    path.parent().map(|p| p.to_path_buf())
                } else {
                    None
                };

                // Score each file
                let scored_files: Vec<_> = files
                    .iter()
                    .map(|f| {
                        let mut score = 0i32;

                        // Bonus for being at canonical location
                        if let Some(ref canon) = canonical_path {
                            if Path::new(&f.path).parent() == Some(canon.as_path()) {
                                score += 10000;
                            }
                        }

                        // Bonus for being organized already
                        if f.organized {
                            score += 5000;
                        }

                        // Resolution score
                        score += match f.resolution.as_deref() {
                            Some("2160p") => 4000,
                            Some("1080p") => 3000,
                            Some("720p") => 2000,
                            Some("480p") => 1000,
                            _ => 0,
                        };

                        // Codec score
                        let codec = f.video_codec.as_deref().unwrap_or("").to_lowercase();
                        score += if codec.contains("hevc") || codec.contains("h265") {
                            300
                        } else if codec.contains("av1") {
                            200
                        } else if codec.contains("h264") || codec.contains("avc") {
                            100
                        } else {
                            0
                        };

                        // Larger file is probably better quality
                        score += (f.size_bytes / 1_000_000) as i32; // MB as points

                        (f, score)
                    })
                    .collect();

                // Sort by score descending
                let mut sorted: Vec<_> = scored_files;
                sorted.sort_by(|a, b| b.1.cmp(&a.1));

                // Keep the first (best) file, delete the rest
                let best_file = sorted[0].0;
                let duplicates: Vec<_> = sorted.iter().skip(1).map(|(f, _)| *f).collect();

                for dup in duplicates {
                    info!(
                        episode = %format!("S{:02}E{:02}", episode.season, episode.episode),
                        show = %show.name,
                        keeping = %best_file.path,
                        deleting = %dup.path,
                        "Removing duplicate file"
                    );

                    // Delete the file from disk
                    let dup_path = Path::new(&dup.path);
                    if dup_path.exists() {
                        if let Err(e) = tokio::fs::remove_file(dup_path).await {
                            warn!(
                                path = %dup.path,
                                error = %e,
                                "Failed to delete duplicate file from disk"
                            );
                            messages.push(format!("Failed to delete {}: {}", dup.path, e));
                        } else {
                            files_deleted += 1;
                            messages.push(format!("Deleted duplicate: {}", dup.path));
                        }
                    }

                    // Delete the database record
                    if let Err(e) = self.db.media_files().delete(dup.id).await {
                        warn!(
                            file_id = %dup.id,
                            error = %e,
                            "Failed to delete duplicate record from database"
                        );
                    }

                    duplicates_removed += 1;
                }
            }
        }

        if duplicates_removed > 0 {
            info!(
                library_id = %library_id,
                duplicates_removed = duplicates_removed,
                files_deleted = files_deleted,
                "Deduplication complete"
            );
        }

        Ok(DeduplicationResult {
            duplicates_removed,
            files_deleted,
            messages,
        })
    }

    /// Clean up orphan files - files on disk that aren't tracked in the database
    ///
    /// This is useful after hardlinking, where the original file remains but the
    /// database record is updated to point to the new location.
    pub async fn cleanup_orphan_files(&self, library_id: Uuid) -> Result<CleanupResult> {
        let library = self
            .db
            .libraries()
            .get_by_id(library_id)
            .await?
            .context("Library not found")?;

        let library_path = Path::new(&library.path);
        if !library_path.exists() {
            return Ok(CleanupResult {
                folders_removed: 0,
                messages: vec!["Library path does not exist".to_string()],
            });
        }

        let mut files_deleted = 0;
        let mut messages = Vec::new();

        // Get all tracked files in this library
        let tracked_files = self.db.media_files().list_by_library(library_id).await?;
        let tracked_paths: std::collections::HashSet<String> =
            tracked_files.iter().map(|f| f.path.clone()).collect();

        // Get extensions for this library type
        let valid_extensions =
            crate::services::scanner::get_extensions_for_library_type(&library.library_type);

        // Walk the library and find orphan files
        for entry in WalkDir::new(library_path)
            .follow_links(false) // Don't follow symlinks
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase());

            // Skip non-media files
            if !ext
                .as_ref()
                .map(|e| valid_extensions.contains(&e.as_str()))
                .unwrap_or(false)
            {
                continue;
            }

            let path_str = path.to_string_lossy().to_string();

            // If this file is tracked in the database, skip it
            if tracked_paths.contains(&path_str) {
                continue;
            }

            // This file is not tracked - check if it's a hardlink
            if let Ok(metadata) = std::fs::metadata(path) {
                use std::os::unix::fs::MetadataExt;
                let nlink = metadata.nlink();

                if nlink > 1 {
                    // This is a hardlink - another file with the same content exists
                    // Safe to delete this orphan
                    info!(
                        path = %path_str,
                        "Deleting orphan hardlink (not in database, has other hardlinks)"
                    );
                    if let Err(e) = tokio::fs::remove_file(path).await {
                        warn!(path = %path_str, error = %e, "Failed to delete orphan hardlink");
                        messages.push(format!("Failed to delete orphan: {}", path_str));
                    } else {
                        files_deleted += 1;
                        messages.push(format!("Deleted orphan hardlink: {}", path_str));
                    }
                } else {
                    // Single link - this file is unique, don't auto-delete
                    // Log it for review
                    debug!(
                        path = %path_str,
                        "Found orphan file (not in database), but it's not a hardlink - skipping"
                    );
                }
            }
        }

        if files_deleted > 0 {
            info!(
                library_id = %library_id,
                files_deleted = files_deleted,
                "Orphan file cleanup complete"
            );
        }

        Ok(CleanupResult {
            folders_removed: files_deleted, // Reusing the field for files
            messages,
        })
    }

    /// Organize all unorganized files in a library
    pub async fn organize_library(&self, library_id: Uuid) -> Result<Vec<OrganizeResult>> {
        let library = self
            .db
            .libraries()
            .get_by_id(library_id)
            .await?
            .context("Library not found")?;

        if !library.organize_files {
            debug!(library_id = %library_id, "Library has organize_files disabled");
            return Ok(vec![]);
        }

        let unorganized_files = self
            .db
            .media_files()
            .list_unorganized_by_library(library_id)
            .await?;

        let mut results = Vec::new();

        for file in unorganized_files {
            // Get the episode this file is linked to
            let episode_id = match file.episode_id {
                Some(id) => id,
                None => {
                    debug!(file_id = %file.id, "File not linked to an episode, skipping");
                    continue;
                }
            };

            let episode = match self.db.episodes().get_by_id(episode_id).await? {
                Some(ep) => ep,
                None => {
                    warn!(file_id = %file.id, episode_id = %episode_id, "Episode not found");
                    continue;
                }
            };

            let show = match self.db.tv_shows().get_by_id(episode.tv_show_id).await? {
                Some(s) => s,
                None => {
                    warn!(file_id = %file.id, show_id = %episode.tv_show_id, "Show not found");
                    continue;
                }
            };

            let (organize_files, rename_style, action) =
                self.get_full_organize_settings(&show).await?;

            if !organize_files {
                debug!(file_id = %file.id, show_id = %show.id, "Show has organize_files disabled");
                continue;
            }

            let result = self
                .organize_file(
                    &file,
                    &show,
                    &episode,
                    &library.path,
                    rename_style,
                    library.naming_pattern.as_deref(),
                    &action,
                    false,
                )
                .await?;
            results.push(result);
        }

        // Also check organized files that might need renaming (wrong naming pattern)
        let organized_files = self
            .db
            .media_files()
            .list_by_library(library_id)
            .await?
            .into_iter()
            .filter(|f| f.organized && f.episode_id.is_some())
            .collect::<Vec<_>>();

        for file in organized_files {
            let episode_id = file.episode_id.unwrap();

            let episode = match self.db.episodes().get_by_id(episode_id).await? {
                Some(ep) => ep,
                None => continue,
            };

            let show = match self.db.tv_shows().get_by_id(episode.tv_show_id).await? {
                Some(s) => s,
                None => continue,
            };

            let (organize_files, rename_style, action) =
                self.get_full_organize_settings(&show).await?;

            if !organize_files || rename_style == RenameStyle::None {
                continue;
            }

            // Generate what the path should be
            let expected_path = self.generate_organized_path(
                &library.path,
                &show,
                &episode,
                Path::new(&file.path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file.mkv"),
                rename_style,
                library.naming_pattern.as_deref(),
            );

            let expected_path_str = expected_path.to_string_lossy().to_string();

            // Check if file is already at the expected path (allowing for different extensions)
            let current_stem = Path::new(&file.path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let expected_stem = expected_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            // If the stems don't match, this file needs renaming
            if current_stem != expected_stem {
                debug!(
                    file_id = %file.id,
                    current = %file.path,
                    expected = %expected_path_str,
                    "File needs renaming to match naming pattern"
                );

                // Mark as unorganized so it gets processed
                if let Err(e) = self.db.media_files().mark_unorganized(file.id).await {
                    warn!(file_id = %file.id, error = %e, "Failed to mark file as unorganized");
                    continue;
                }

                let result = self
                    .organize_file(
                        &file,
                        &show,
                        &episode,
                        &library.path,
                        rename_style,
                        library.naming_pattern.as_deref(),
                        &action,
                        false,
                    )
                    .await?;
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Create show and season folder structure for a TV show
    pub async fn create_show_folders(&self, show_id: Uuid) -> Result<PathBuf> {
        let show = self
            .db
            .tv_shows()
            .get_by_id(show_id)
            .await?
            .context("Show not found")?;

        let library = self
            .db
            .libraries()
            .get_by_id(show.library_id)
            .await?
            .context("Library not found")?;

        // Create show folder name
        let show_folder = match show.year {
            Some(year) => format!("{} ({})", sanitize_for_filename(&show.name), year),
            None => sanitize_for_filename(&show.name),
        };

        let show_path = PathBuf::from(&library.path).join(&show_folder);

        // Create show directory
        tokio::fs::create_dir_all(&show_path).await?;

        // Get all episodes for this show to determine what seasons exist
        let episodes = self.db.episodes().list_by_show(show_id).await?;

        // Create season folders for all unique seasons
        let mut seasons: Vec<i32> = episodes.iter().map(|e| e.season).collect();
        seasons.sort();
        seasons.dedup();

        for season in seasons {
            let season_folder = format!("Season {:02}", season);
            let season_path = show_path.join(&season_folder);
            tokio::fs::create_dir_all(&season_path).await?;
            debug!(show_id = %show_id, season = season, path = %season_path.display(), "Created season folder");
        }

        debug!("Created show folder structure at {}", show_path.display());

        // Update show path in database if not set
        if show.path.is_none() {
            let show_path_str = show_path.to_string_lossy().to_string();
            self.db
                .tv_shows()
                .update(
                    show_id,
                    crate::db::UpdateTvShow {
                        path: Some(show_path_str),
                        ..Default::default()
                    },
                )
                .await?;
        }

        Ok(show_path)
    }

    // Note: Torrent processing is now handled by FileProcessor::process_source()

    /// Consolidate a library by:
    /// 1. Finding duplicate show folders (e.g., "Show" and "Show (2024)")
    /// 2. Moving files from old folders to the correct folder structure
    /// 3. Removing empty folders
    /// 4. Updating database paths
    pub async fn consolidate_library(&self, library_id: Uuid) -> Result<ConsolidateResult> {
        let library = self
            .db
            .libraries()
            .get_by_id(library_id)
            .await?
            .context("Library not found")?;

        if library.library_type != "tv" {
            return Ok(ConsolidateResult {
                success: true,
                folders_removed: 0,
                files_moved: 0,
                messages: vec![
                    "Consolidation is only supported for TV libraries currently".to_string(),
                ],
            });
        }

        let library_path = Path::new(&library.path);
        let mut folders_removed = 0;
        let mut files_moved = 0;
        let mut messages = Vec::new();

        // Get all shows in this library
        let shows = self.db.tv_shows().list_by_library(library_id).await?;

        for show in &shows {
            // Determine the correct folder name for this show
            let correct_folder_name = match show.year {
                Some(year) => format!("{} ({})", sanitize_for_filename(&show.name), year),
                None => sanitize_for_filename(&show.name),
            };
            let correct_folder_path = library_path.join(&correct_folder_name);

            // Look for other folders that might contain this show's files
            // (old naming conventions, without year, etc.)
            let show_name_sanitized = sanitize_for_filename(&show.name);
            let show_name_lower = show_name_sanitized.to_lowercase();

            // Read library directory to find potential duplicate folders
            let mut entries = match tokio::fs::read_dir(library_path).await {
                Ok(e) => e,
                Err(e) => {
                    messages.push(format!("Could not read library directory: {}", e));
                    continue;
                }
            };

            while let Ok(Some(entry)) = entries.next_entry().await {
                let entry_path = entry.path();
                if !entry_path.is_dir() {
                    continue;
                }

                let folder_name = entry_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                // Skip if this IS the correct folder
                if folder_name == correct_folder_name {
                    continue;
                }

                // Check if this folder might be for the same show
                // (starts with show name, possibly missing year)
                let folder_lower = folder_name.to_lowercase();
                let is_potential_match = folder_lower == show_name_lower
                    || folder_lower.starts_with(&format!("{} (", show_name_lower))
                    || folder_lower.starts_with(&format!("{}_", show_name_lower));

                if !is_potential_match {
                    continue;
                }

                info!(
                    show = %show.name,
                    old_folder = folder_name,
                    correct_folder = %correct_folder_name,
                    "Found potential duplicate folder"
                );

                // Move files from old folder to correct folder
                let moved = self
                    .move_folder_contents(&entry_path, &correct_folder_path, &mut messages)
                    .await?;
                files_moved += moved;

                // Try to remove the old folder if empty
                if self.remove_empty_folder(&entry_path, &mut messages).await? {
                    folders_removed += 1;
                }
            }
        }

        // Also scan for orphaned files in the library root and move them
        let moved_from_root = self
            .organize_root_files(&library, &shows, &mut messages)
            .await?;
        files_moved += moved_from_root;

        // Update media file paths in database
        let updated = self.update_media_file_paths(library_id).await?;
        if updated > 0 {
            messages.push(format!("Updated {} media file paths in database", updated));
        }

        // Clean up empty folders (library-type aware)
        let cleanup_result = self.cleanup_empty_folders(library_id).await?;
        folders_removed += cleanup_result.folders_removed;
        messages.extend(cleanup_result.messages);

        Ok(ConsolidateResult {
            success: true,
            folders_removed,
            files_moved,
            messages,
        })
    }

    /// Move contents of one folder to another, preserving season structure
    async fn move_folder_contents(
        &self,
        source: &Path,
        dest: &Path,
        messages: &mut Vec<String>,
    ) -> Result<i32> {
        let mut moved_count = 0;

        // Create destination if it doesn't exist
        if !dest.exists() {
            tokio::fs::create_dir_all(dest).await?;
        }

        let mut entries = tokio::fs::read_dir(source).await?;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let entry_path = entry.path();
            let entry_name = entry_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            let dest_path = dest.join(entry_name);

            if entry_path.is_dir() {
                // Recursively move directory contents (e.g., Season folders)
                let sub_moved =
                    Box::pin(self.move_folder_contents(&entry_path, &dest_path, messages)).await?;
                moved_count += sub_moved;

                // Try to remove the now-empty source directory
                self.remove_empty_folder(&entry_path, messages).await.ok();
            } else if entry_path.is_file() {
                // Move file
                if dest_path.exists() {
                    // Destination already exists - delete the source (it's a duplicate)
                    match tokio::fs::remove_file(&entry_path).await {
                        Ok(_) => {
                            messages.push(format!(
                                "Deleted duplicate: {} (already at {})",
                                entry_path.display(),
                                dest_path.display()
                            ));
                            moved_count += 1; // Count as "handled"
                        }
                        Err(e) => {
                            messages
                                .push(format!("Failed to delete duplicate {}: {}", entry_name, e));
                        }
                    }
                    continue;
                }

                match tokio::fs::rename(&entry_path, &dest_path).await {
                    Ok(_) => {
                        messages.push(format!(
                            "Moved: {} -> {}",
                            entry_path.display(),
                            dest_path.display()
                        ));
                        moved_count += 1;
                    }
                    Err(e) => {
                        // If rename fails (cross-device), try copy + delete
                        match tokio::fs::copy(&entry_path, &dest_path).await {
                            Ok(_) => {
                                tokio::fs::remove_file(&entry_path).await.ok();
                                messages.push(format!(
                                    "Moved: {} -> {}",
                                    entry_path.display(),
                                    dest_path.display()
                                ));
                                moved_count += 1;
                            }
                            Err(copy_err) => {
                                messages.push(format!(
                                    "Failed to move {}: rename={}, copy={}",
                                    entry_name, e, copy_err
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(moved_count)
    }

    /// Remove an empty folder (and any empty parent folders up to library root)
    async fn remove_empty_folder(&self, path: &Path, messages: &mut Vec<String>) -> Result<bool> {
        // Check if folder is empty
        let mut entries = tokio::fs::read_dir(path).await?;
        if entries.next_entry().await?.is_some() {
            return Ok(false); // Not empty
        }

        match tokio::fs::remove_dir(path).await {
            Ok(_) => {
                messages.push(format!("Removed empty folder: {}", path.display()));
                Ok(true)
            }
            Err(e) => {
                messages.push(format!("Could not remove folder {}: {}", path.display(), e));
                Ok(false)
            }
        }
    }

    /// Get the conflicts folder path for a library
    async fn get_conflicts_folder(&self, library_path: &str) -> PathBuf {
        // TODO: In the future, read from library.conflicts_folder field
        // For now, default to "_conflicts" subfolder
        Path::new(library_path).join("_conflicts")
    }

    /// Move a file to the conflicts folder
    ///
    /// Returns the new path if successful, None if failed
    async fn move_to_conflicts(
        &self,
        file_path: &Path,
        conflicts_folder: &Path,
    ) -> Result<Option<String>> {
        // Create conflicts folder if it doesn't exist
        if let Err(e) = tokio::fs::create_dir_all(conflicts_folder).await {
            warn!(
                conflicts_folder = %conflicts_folder.display(),
                error = %e,
                "Failed to create conflicts folder"
            );
            return Ok(None);
        }

        // Generate unique filename to avoid conflicts within the conflicts folder
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let new_name = format!("{}_{}", timestamp, file_name);
        let new_path = conflicts_folder.join(&new_name);

        // Move the file
        match tokio::fs::rename(file_path, &new_path).await {
            Ok(_) => {
                info!(
                    original_path = %file_path.display(),
                    conflict_path = %new_path.display(),
                    "Moved conflicting file to conflicts folder"
                );
                Ok(Some(new_path.to_string_lossy().to_string()))
            }
            Err(e) => {
                // Try copy + delete if rename fails (cross-filesystem)
                if let Err(copy_err) = tokio::fs::copy(file_path, &new_path).await {
                    warn!(
                        original_path = %file_path.display(),
                        conflict_path = %new_path.display(),
                        rename_error = %e,
                        copy_error = %copy_err,
                        "Failed to move conflicting file to conflicts folder"
                    );
                    return Ok(None);
                }
                // Successfully copied, now delete original
                if let Err(del_err) = tokio::fs::remove_file(file_path).await {
                    warn!(
                        original_path = %file_path.display(),
                        error = %del_err,
                        "Failed to delete original after copying to conflicts"
                    );
                    // File was copied though, so return success
                }
                info!(
                    original_path = %file_path.display(),
                    conflict_path = %new_path.display(),
                    "Copied conflicting file to conflicts folder (cross-filesystem)"
                );
                Ok(Some(new_path.to_string_lossy().to_string()))
            }
        }
    }

    /// Organize any loose video files in the library root
    async fn organize_root_files(
        &self,
        library: &LibraryRecord,
        shows: &[TvShowRecord],
        messages: &mut Vec<String>,
    ) -> Result<i32> {
        let library_path = Path::new(&library.path);
        let mut moved_count = 0;

        let mut entries = match tokio::fs::read_dir(library_path).await {
            Ok(e) => e,
            Err(_) => return Ok(0),
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let entry_path = entry.path();
            if !entry_path.is_file() {
                continue;
            }

            let filename = entry_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            if !is_video_file(filename) {
                continue;
            }

            // Parse the filename
            let parsed = crate::services::filename_parser::parse_episode(filename);

            let Some(ref show_name) = parsed.show_name else {
                continue;
            };
            let Some(season) = parsed.season else {
                continue;
            };

            // Find matching show
            let show_name_lower = show_name.to_lowercase();
            let matching_show = shows.iter().find(|s| {
                s.name.to_lowercase() == show_name_lower
                    || sanitize_for_filename(&s.name).to_lowercase() == show_name_lower
            });

            let Some(show) = matching_show else {
                continue;
            };

            // Determine target path
            let show_folder = match show.year {
                Some(year) => format!("{} ({})", sanitize_for_filename(&show.name), year),
                None => sanitize_for_filename(&show.name),
            };
            let season_folder = format!("Season {:02}", season);
            let target_dir = library_path.join(&show_folder).join(&season_folder);
            let target_path = target_dir.join(filename);

            if target_path.exists() {
                continue;
            }

            // Create target directory and move file
            tokio::fs::create_dir_all(&target_dir).await.ok();

            match tokio::fs::rename(&entry_path, &target_path).await {
                Ok(_) => {
                    messages.push(format!(
                        "Moved root file: {} -> {}/{}/{}",
                        filename, show_folder, season_folder, filename
                    ));
                    moved_count += 1;
                }
                Err(_) => {
                    // Try copy + delete
                    if tokio::fs::copy(&entry_path, &target_path).await.is_ok() {
                        tokio::fs::remove_file(&entry_path).await.ok();
                        messages.push(format!(
                            "Moved root file: {} -> {}/{}/{}",
                            filename, show_folder, season_folder, filename
                        ));
                        moved_count += 1;
                    }
                }
            }
        }

        Ok(moved_count)
    }

    /// Update media file paths in database to match actual file locations
    async fn update_media_file_paths(&self, library_id: Uuid) -> Result<i32> {
        let library = self.db.libraries().get_by_id(library_id).await?;
        let Some(lib) = library else {
            return Ok(0);
        };

        let media_files = self.db.media_files().list_by_library(library_id).await?;
        let mut updated_count = 0;

        for file in media_files {
            let current_path = Path::new(&file.path);

            // Check if file exists at current path
            if current_path.exists() {
                continue;
            }

            // Try to find the file by name in the library
            let filename = current_path.file_name().and_then(|n| n.to_str());

            let Some(name) = filename else {
                continue;
            };

            // Search for file in library
            if let Some(new_path) = find_file_in_directory(&lib.path, name).await {
                // Update database
                if self
                    .db
                    .media_files()
                    .update_path(file.id, &new_path)
                    .await
                    .is_ok()
                {
                    updated_count += 1;
                }
            }
        }

        Ok(updated_count)
    }

    /// Clean up empty folders in a library
    ///
    /// This method removes folders that contain no files (recursively).
    /// For TV libraries, it protects:
    /// - Show folders that are registered in the database
    /// - Season folders for registered shows (even if empty)
    ///
    /// Returns the number of folders removed
    pub async fn cleanup_empty_folders(&self, library_id: Uuid) -> Result<CleanupResult> {
        let library = self
            .db
            .libraries()
            .get_by_id(library_id)
            .await?
            .context("Library not found")?;

        let library_path = Path::new(&library.path);
        if !library_path.exists() {
            return Ok(CleanupResult {
                folders_removed: 0,
                messages: vec!["Library path does not exist".to_string()],
            });
        }

        match library.library_type.as_str() {
            "tv" => {
                self.cleanup_empty_folders_tv(library_id, library_path)
                    .await
            }
            "movies" => self.cleanup_empty_folders_movies(library_path).await,
            _ => {
                // For other library types, do generic cleanup
                self.cleanup_empty_folders_generic(library_path).await
            }
        }
    }

    /// Clean up empty folders for TV libraries
    ///
    /// Protects show and season folders for registered shows
    async fn cleanup_empty_folders_tv(
        &self,
        library_id: Uuid,
        library_path: &Path,
    ) -> Result<CleanupResult> {
        use std::collections::HashSet;

        let mut messages = Vec::new();
        let mut folders_removed = 0;

        // Build set of protected paths (show folders and their season folders)
        let mut protected_paths: HashSet<PathBuf> = HashSet::new();

        let shows = self.db.tv_shows().list_by_library(library_id).await?;
        for show in &shows {
            // Generate the expected show folder path
            let show_folder = match show.year {
                Some(year) => format!("{} ({})", sanitize_for_filename(&show.name), year),
                None => sanitize_for_filename(&show.name),
            };
            let show_path = library_path.join(&show_folder);
            protected_paths.insert(show_path.clone());

            // Also protect all season folders for this show
            let episodes = self.db.episodes().list_by_show(show.id).await?;
            let mut seasons: Vec<i32> = episodes.iter().map(|e| e.season).collect();
            seasons.sort();
            seasons.dedup();

            for season in seasons {
                let season_folder = format!("Season {:02}", season);
                protected_paths.insert(show_path.join(&season_folder));
            }

            // Also protect the show's actual path if different from expected
            if let Some(ref path) = show.path {
                let actual_path = PathBuf::from(path);
                protected_paths.insert(actual_path.clone());

                // And its season folders
                if actual_path.exists() {
                    if let Ok(mut entries) = tokio::fs::read_dir(&actual_path).await {
                        while let Ok(Some(entry)) = entries.next_entry().await {
                            let entry_path = entry.path();
                            if entry_path.is_dir() {
                                let name = entry_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("");
                                // Protect Season folders
                                if name.to_lowercase().starts_with("season") {
                                    protected_paths.insert(entry_path);
                                }
                            }
                        }
                    }
                }
            }
        }

        debug!(
            protected_count = protected_paths.len(),
            "Built protected paths set for TV library cleanup"
        );

        // Now walk the library and find empty folders to remove
        // We need to process depth-first (deepest folders first)
        let folders_to_check = collect_all_folders(library_path).await?;

        // Sort by depth (deepest first) so we remove children before parents
        let mut sorted_folders: Vec<_> = folders_to_check.into_iter().collect();
        sorted_folders.sort_by(|a, b| {
            let depth_a = a.components().count();
            let depth_b = b.components().count();
            depth_b.cmp(&depth_a) // Deepest first
        });

        for folder in sorted_folders {
            // Skip the library root itself
            if folder == library_path {
                continue;
            }

            // Skip protected paths
            if protected_paths.contains(&folder) {
                debug!(path = %folder.display(), "Skipping protected folder");
                continue;
            }

            // Check if folder is empty (no files, may have empty subdirs)
            if is_folder_empty_of_files(&folder).await? {
                // Try to remove the folder (will fail if not empty, which is fine)
                match tokio::fs::remove_dir(&folder).await {
                    Ok(_) => {
                        debug!("Removed empty folder: {}", folder.display());
                        messages.push(format!("Removed: {}", folder.display()));
                        folders_removed += 1;
                    }
                    Err(e) => {
                        // If it fails, the folder probably has subdirs that weren't empty
                        debug!(path = %folder.display(), error = %e, "Could not remove folder");
                    }
                }
            }
        }

        Ok(CleanupResult {
            folders_removed,
            messages,
        })
    }

    /// Clean up empty folders for Movie libraries
    ///
    /// For movies, we typically have a simpler structure:
    /// - Movie Name (Year)/movie files
    /// Remove any folders that are completely empty
    async fn cleanup_empty_folders_movies(&self, library_path: &Path) -> Result<CleanupResult> {
        // For now, use generic cleanup for movies
        // In the future, we could protect registered movie folders
        self.cleanup_empty_folders_generic(library_path).await
    }

    /// Generic empty folder cleanup
    ///
    /// Removes all folders that contain no files (recursively)
    async fn cleanup_empty_folders_generic(&self, library_path: &Path) -> Result<CleanupResult> {
        let mut messages = Vec::new();
        let mut folders_removed = 0;

        // Collect all folders depth-first
        let folders_to_check = collect_all_folders(library_path).await?;

        // Sort by depth (deepest first)
        let mut sorted_folders: Vec<_> = folders_to_check.into_iter().collect();
        sorted_folders.sort_by(|a, b| {
            let depth_a = a.components().count();
            let depth_b = b.components().count();
            depth_b.cmp(&depth_a)
        });

        for folder in sorted_folders {
            if folder == library_path {
                continue;
            }

            if is_folder_empty_of_files(&folder).await? {
                match tokio::fs::remove_dir(&folder).await {
                    Ok(_) => {
                        debug!("Removed empty folder: {}", folder.display());
                        messages.push(format!("Removed: {}", folder.display()));
                        folders_removed += 1;
                    }
                    Err(e) => {
                        debug!(path = %folder.display(), error = %e, "Could not remove folder");
                    }
                }
            }
        }

        Ok(CleanupResult {
            folders_removed,
            messages,
        })
    }
}

/// Result of library consolidation
#[derive(Debug, Clone)]
pub struct ConsolidateResult {
    pub success: bool,
    pub folders_removed: i32,
    pub files_moved: i32,
    pub messages: Vec<String>,
}

/// Result of empty folder cleanup
#[derive(Debug, Clone)]
pub struct CleanupResult {
    pub folders_removed: i32,
    pub messages: Vec<String>,
}

/// Result of deduplication
#[derive(Debug, Clone)]
pub struct DeduplicationResult {
    pub duplicates_removed: i32,
    pub files_deleted: i32,
    pub messages: Vec<String>,
}

/// Collect all folders in a directory tree
async fn collect_all_folders(root: &Path) -> Result<Vec<PathBuf>> {
    use walkdir::WalkDir;

    let mut folders = Vec::new();

    for entry in WalkDir::new(root)
        .min_depth(1) // Skip the root itself
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() {
            folders.push(entry.path().to_path_buf());
        }
    }

    Ok(folders)
}

/// Check if a folder is empty of files (may contain empty subdirectories)
///
/// Returns true if the folder and all its subdirectories contain no files
async fn is_folder_empty_of_files(path: &Path) -> Result<bool> {
    use walkdir::WalkDir;

    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Find a file by name within a directory tree
async fn find_file_in_directory(dir: &str, filename: &str) -> Option<String> {
    use walkdir::WalkDir;

    for entry in WalkDir::new(dir)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(name) = entry.file_name().to_str() {
                if name == filename {
                    return Some(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

/// File information for organizing
#[derive(Debug, Clone)]
pub struct TorrentFileForOrganize {
    pub path: String,
    pub size: u64,
}

/// Result of organizing a torrent
#[derive(Debug, Clone)]
pub struct OrganizeTorrentResult {
    pub success: bool,
    pub organized_count: i32,
    pub failed_count: i32,
    pub messages: Vec<String>,
}

/// Extract quality information from a filename
fn extract_quality_info(filename: &str) -> String {
    let mut parts = Vec::new();

    // Resolution
    let resolutions = ["2160p", "1080p", "720p", "480p", "4K", "UHD"];
    for res in resolutions {
        if filename.to_lowercase().contains(&res.to_lowercase()) {
            parts.push(res.to_string());
            break;
        }
    }

    // Video codec
    let codecs = ["HEVC", "x265", "H.265", "x264", "H.264", "AV1", "VP9"];
    for codec in codecs {
        if filename.to_lowercase().contains(&codec.to_lowercase()) {
            parts.push(codec.to_string());
            break;
        }
    }

    // HDR
    let hdr_types = ["DV", "Dolby Vision", "HDR10+", "HDR10", "HDR"];
    for hdr in hdr_types {
        if filename.to_lowercase().contains(&hdr.to_lowercase()) {
            parts.push(hdr.to_string());
            break;
        }
    }

    // Release group (typically at end after dash or in brackets)
    if let Some(group) = extract_release_group(filename) {
        parts.push(group);
    }

    parts.join(" ")
}

/// Extract release group from filename
fn extract_release_group(filename: &str) -> Option<String> {
    // Remove extension
    let name = filename
        .rsplit_once('.')
        .map(|(n, _)| n)
        .unwrap_or(filename);

    // Look for common patterns: -GroupName or [GroupName] at the end
    if let Some(idx) = name.rfind('-') {
        let group = &name[idx + 1..];
        // Filter out common non-group patterns
        let lower = group.to_lowercase();
        if !lower.contains("1080")
            && !lower.contains("720")
            && !lower.contains("x264")
            && !lower.contains("x265")
            && !lower.contains("hevc")
            && group.len() < 20
            && !group.contains(' ')
        {
            return Some(group.to_string());
        }
    }

    None
}

/// Apply a naming pattern to generate a file path
///
/// Supported variables:
/// - `{show}` - TV show name
/// - `{season}` - Season number (raw)
/// - `{season:02}` - Season number zero-padded to 2 digits
/// - `{episode}` - Episode number (raw)
/// - `{episode:02}` - Episode number zero-padded to 2 digits
/// - `{title}` - Episode title
/// - `{ext}` - File extension (without dot)
/// - `{year}` - Show premiere year
///
/// Example pattern: `{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}`
pub fn apply_naming_pattern(
    pattern: &str,
    show: &TvShowRecord,
    episode: &EpisodeRecord,
    extension: &str,
) -> PathBuf {
    use regex::Regex;

    let mut result = pattern.to_string();

    // Replace {show} with sanitized show name
    result = result.replace("{show}", &sanitize_for_filename(&show.name));

    // Replace {year} with show year
    let year_str = show.year.map(|y| y.to_string()).unwrap_or_default();
    result = result.replace("{year}", &year_str);

    // Replace {title} with sanitized episode title
    let title = episode
        .title
        .as_ref()
        .map(|t| sanitize_for_filename(t))
        .unwrap_or_else(|| format!("Episode {}", episode.episode));
    result = result.replace("{title}", &title);

    // Replace {ext} with extension (without leading dot)
    let ext = extension.trim_start_matches('.');
    result = result.replace("{ext}", ext);

    // Replace season with format specifier: {season:02} -> zero-padded, {season} -> raw
    let season_re = Regex::new(r"\{season(?::(\d+))?\}").unwrap();
    result = season_re
        .replace_all(&result, |caps: &regex::Captures| {
            if let Some(width) = caps.get(1) {
                let w: usize = width.as_str().parse().unwrap_or(2);
                format!("{:0>width$}", episode.season, width = w)
            } else {
                episode.season.to_string()
            }
        })
        .to_string();

    // Replace episode with format specifier: {episode:02} -> zero-padded, {episode} -> raw
    let episode_re = Regex::new(r"\{episode(?::(\d+))?\}").unwrap();
    result = episode_re
        .replace_all(&result, |caps: &regex::Captures| {
            if let Some(width) = caps.get(1) {
                let w: usize = width.as_str().parse().unwrap_or(2);
                format!("{:0>width$}", episode.episode, width = w)
            } else {
                episode.episode.to_string()
            }
        })
        .to_string();

    PathBuf::from(result)
}

/// Apply a naming pattern to generate a movie file path
///
/// Supported variables:
/// - `{title}` - Movie title (sanitized for filesystem)
/// - `{year}` - Release year
/// - `{ext}` - File extension (without dot)
/// - `{quality}` - Quality info extracted from original filename
/// - `{original}` - Original filename without extension
pub fn apply_movie_naming_pattern(
    pattern: &str,
    movie: &crate::db::MovieRecord,
    original_filename: &str,
    extension: &str,
) -> PathBuf {
    let mut result = pattern.to_string();

    // Replace {title} with sanitized movie title
    result = result.replace("{title}", &sanitize_for_filename(&movie.title));

    // Replace {year} with movie year
    let year_str = movie
        .year
        .map(|y| y.to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    result = result.replace("{year}", &year_str);

    // Replace {ext} with extension (without leading dot)
    let ext = extension.trim_start_matches('.');
    result = result.replace("{ext}", ext);

    // Replace {quality} with extracted quality info
    let quality_info = extract_quality_info(original_filename);
    result = result.replace("{quality}", &quality_info);

    // Replace {original} with original filename without extension
    let original_stem = Path::new(original_filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(original_filename);
    result = result.replace("{original}", original_stem);

    PathBuf::from(result)
}

/// Apply a naming pattern to generate a music album file path
///
/// Supported variables:
/// - `{artist}` - Artist name
/// - `{album}` - Album name
/// - `{year}` - Release year
/// - `{track}` - Track number (from TrackRecord or parsed from filename)
/// - `{title}` - Track title (from TrackRecord or parsed from filename)
/// - `{ext}` - File extension (without dot)
/// - `{original}` - Original filename without extension
pub fn apply_music_naming_pattern(
    pattern: &str,
    artist_name: &str,
    album: &crate::db::AlbumRecord,
    track: Option<&crate::db::TrackRecord>,
    original_filename: &str,
    extension: &str,
) -> PathBuf {
    use regex::Regex;

    let mut result = pattern.to_string();

    // Replace {artist} with sanitized artist name
    result = result.replace("{artist}", &sanitize_for_filename(artist_name));

    // Replace {album} with sanitized album name
    result = result.replace("{album}", &sanitize_for_filename(&album.name));

    // Replace {year} with album year
    let year_str = album
        .year
        .map(|y| y.to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    result = result.replace("{year}", &year_str);

    // Replace {ext} with extension (without leading dot)
    let ext = extension.trim_start_matches('.');
    result = result.replace("{ext}", ext);

    // Replace {original} with original filename without extension
    let original_stem = Path::new(original_filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(original_filename);
    result = result.replace("{original}", original_stem);

    // Get track number and title from TrackRecord if available, otherwise parse from filename
    let (track_num, track_title): (i32, String) = if let Some(t) = track {
        // Use actual metadata from database (like TV shows do with EpisodeRecord)
        (t.track_number, sanitize_for_filename(&t.title))
    } else {
        // Fallback: Try to extract track number and title from filename
        // Common patterns: "01 - Track Title.mp3", "01. Track Title.mp3", "01 Track Title.mp3"
        let track_re = Regex::new(r"^(\d+)[.\-\s]+(.+)$").unwrap();
        if let Some(caps) = track_re.captures(original_stem) {
            let num: i32 = caps
                .get(1)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let title = caps.get(2).map(|m| m.as_str()).unwrap_or(original_stem);
            (num, sanitize_for_filename(title))
        } else {
            (0, sanitize_for_filename(original_stem))
        }
    };

    // Replace track with format specifier: {track:02} -> zero-padded, {track} -> raw
    let track_fmt_re = Regex::new(r"\{track(?::(\d+))?\}").unwrap();
    result = track_fmt_re
        .replace_all(&result, |caps: &regex::Captures| {
            if let Some(width) = caps.get(1) {
                let w: usize = width.as_str().parse().unwrap_or(2);
                format!("{:0>width$}", track_num, width = w)
            } else {
                track_num.to_string()
            }
        })
        .to_string();

    // Replace {title} with track title (from database or parsed from filename)
    result = result.replace("{title}", &track_title);

    PathBuf::from(result)
}

/// Apply a naming pattern to generate an audiobook file path
///
/// Supported variables:
/// - `{author}` - Author name
/// - `{title}` - Audiobook title
/// - `{series}` - Series name (empty if none)
/// - `{series_position}` - Position in series (empty if none)
/// - `{narrator}` - Primary narrator (first in list)
/// - `{ext}` - File extension (without dot)
/// - `{original}` - Original filename without extension
pub fn apply_audiobook_naming_pattern(
    pattern: &str,
    author_name: &str,
    audiobook: &crate::db::AudiobookRecord,
    original_filename: &str,
    extension: &str,
) -> PathBuf {
    let mut result = pattern.to_string();

    // Replace {author} with sanitized author name
    result = result.replace("{author}", &sanitize_for_filename(author_name));

    // Replace {title} with sanitized audiobook title
    result = result.replace("{title}", &sanitize_for_filename(&audiobook.title));

    // Replace {series} with series name (or empty)
    let series_name = audiobook
        .series_name
        .as_ref()
        .map(|s| sanitize_for_filename(s))
        .unwrap_or_default();
    result = result.replace("{series}", &series_name);

    // Replace {series_position} with position
    let series_pos = audiobook
        .series_position
        .map(|p| p.to_string())
        .unwrap_or_default();
    result = result.replace("{series_position}", &series_pos);

    // Replace {narrator} with first narrator
    let narrator = audiobook
        .narrators
        .first()
        .map(|n| sanitize_for_filename(n))
        .unwrap_or_default();
    result = result.replace("{narrator}", &narrator);

    // Replace {ext} with extension (without leading dot)
    let ext = extension.trim_start_matches('.');
    result = result.replace("{ext}", ext);

    // Replace {original} with original filename without extension
    let original_stem = Path::new(original_filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(original_filename);
    result = result.replace("{original}", original_stem);

    PathBuf::from(result)
}

/// Default naming pattern used when library doesn't have one set (TV shows)
pub const DEFAULT_NAMING_PATTERN: &str =
    "{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}";

/// Default naming pattern for movies
/// Supported variables:
/// - `{title}` - Movie title
/// - `{year}` - Release year
/// - `{ext}` - File extension (without dot)
/// - `{quality}` - Quality info (resolution, codec, etc.)
/// - `{original}` - Original filename (without extension)
pub const DEFAULT_MOVIE_NAMING_PATTERN: &str = "{title} ({year})/{title} ({year}).{ext}";

/// Default naming pattern for music albums
/// Supported variables:
/// - `{artist}` - Artist name
/// - `{album}` - Album name
/// - `{year}` - Release year
/// - `{track}` - Track number (zero-padded)
/// - `{title}` - Track title
/// - `{ext}` - File extension (without dot)
/// - `{original}` - Original filename (without extension)
pub const DEFAULT_MUSIC_NAMING_PATTERN: &str =
    "{artist}/{album} ({year})/{track:02} - {title}.{ext}";

/// Default naming pattern for audiobooks
/// Supported variables:
/// - `{author}` - Author name
/// - `{title}` - Audiobook title
/// - `{series}` - Series name (if any)
/// - `{series_position}` - Position in series
/// - `{narrator}` - Primary narrator
/// - `{ext}` - File extension (without dot)
/// - `{original}` - Original filename (without extension)
pub const DEFAULT_AUDIOBOOK_NAMING_PATTERN: &str = "{author}/{title}/{original}.{ext}";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_for_filename() {
        // sanitize_filename crate removes invalid characters
        let result = sanitize_for_filename("Show: Name");
        assert!(!result.contains(':'), "Should not contain colon");

        let result = sanitize_for_filename("What/If?");
        assert!(!result.contains('/'), "Should not contain slash");
        assert!(!result.contains('?'), "Should not contain question mark");

        // Normal names should be unchanged
        assert_eq!(sanitize_for_filename("Normal Name"), "Normal Name");
    }

    #[test]
    fn test_extract_quality_info() {
        // extract_quality_info returns resolution + codec (group may or may not be included)
        let result1 = extract_quality_info("Show.S01E01.1080p.HEVC.x265-GroupName");
        assert!(
            result1.contains("1080p"),
            "Should contain 1080p: {}",
            result1
        );
        assert!(result1.contains("HEVC"), "Should contain HEVC: {}", result1);

        let result2 = extract_quality_info("Show.S01E01.720p.x264-FLEET");
        assert!(result2.contains("720p"), "Should contain 720p: {}", result2);
    }

    #[test]
    fn test_apply_naming_pattern() {
        use crate::db::{EpisodeRecord, TvShowRecord};
        use uuid::Uuid;

        let show = TvShowRecord {
            id: Uuid::new_v4(),
            library_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "Breaking Bad".to_string(),
            sort_name: None,
            year: Some(2008),
            status: "Ended".to_string(),
            tvmaze_id: None,
            tmdb_id: None,
            tvdb_id: None,
            imdb_id: None,
            overview: None,
            network: None,
            runtime: None,
            genres: vec![],
            poster_url: None,
            backdrop_url: None,
            monitored: true,
            monitor_type: "all".to_string(),
            path: None,
            auto_download_override: None,
            backfill_existing: false,
            organize_files_override: None,
            rename_style_override: None,
            auto_hunt_override: None,
            episode_count: None,
            episode_file_count: None,
            size_bytes: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            allowed_resolutions_override: None,
            allowed_video_codecs_override: None,
            allowed_audio_formats_override: None,
            require_hdr_override: None,
            allowed_hdr_types_override: None,
            allowed_sources_override: None,
            release_group_blacklist_override: None,
            release_group_whitelist_override: None,
        };

        let episode = EpisodeRecord {
            id: Uuid::new_v4(),
            tv_show_id: show.id,
            season: 1,
            episode: 5,
            absolute_number: None,
            title: Some("Gray Matter".to_string()),
            overview: None,
            air_date: None,
            runtime: None,
            tvmaze_id: None,
            tmdb_id: None,
            tvdb_id: None,
            status: "downloaded".to_string(),
            torrent_link: None,
            torrent_link_added_at: None,
            matched_rss_item_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let result = apply_naming_pattern(DEFAULT_NAMING_PATTERN, &show, &episode, "mkv");
        assert_eq!(
            result.to_string_lossy(),
            "Breaking Bad/Season 01/Breaking Bad - S01E05 - Gray Matter.mkv"
        );

        // Test with no episode title
        let episode_no_title = EpisodeRecord {
            title: None,
            ..episode.clone()
        };
        let result = apply_naming_pattern(DEFAULT_NAMING_PATTERN, &show, &episode_no_title, "mkv");
        assert_eq!(
            result.to_string_lossy(),
            "Breaking Bad/Season 01/Breaking Bad - S01E05 - Episode 5.mkv"
        );
    }

    // =========================================================================
    // Naming Pattern Tests with Various Shows
    // =========================================================================

    #[test]
    fn test_apply_naming_pattern_ds9() {
        use crate::db::{EpisodeRecord, TvShowRecord};
        use uuid::Uuid;

        let show = TvShowRecord {
            id: Uuid::new_v4(),
            library_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "Star Trek: Deep Space Nine".to_string(),
            sort_name: None,
            year: Some(1993),
            status: "Ended".to_string(),
            tvmaze_id: None,
            tmdb_id: None,
            tvdb_id: None,
            imdb_id: None,
            overview: None,
            network: None,
            runtime: None,
            genres: vec![],
            poster_url: None,
            backdrop_url: None,
            monitored: true,
            monitor_type: "all".to_string(),
            path: None,
            auto_download_override: None,
            backfill_existing: false,
            organize_files_override: None,
            rename_style_override: None,
            auto_hunt_override: None,
            episode_count: None,
            episode_file_count: None,
            size_bytes: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            allowed_resolutions_override: None,
            allowed_video_codecs_override: None,
            allowed_audio_formats_override: None,
            require_hdr_override: None,
            allowed_hdr_types_override: None,
            allowed_sources_override: None,
            release_group_blacklist_override: None,
            release_group_whitelist_override: None,
        };

        let episode = EpisodeRecord {
            id: Uuid::new_v4(),
            tv_show_id: show.id,
            season: 1,
            episode: 9,
            absolute_number: None,
            title: Some("The Passenger".to_string()),
            overview: None,
            air_date: None,
            runtime: None,
            tvmaze_id: None,
            tmdb_id: None,
            tvdb_id: None,
            status: "wanted".to_string(),
            torrent_link: None,
            torrent_link_added_at: None,
            matched_rss_item_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let result = apply_naming_pattern(DEFAULT_NAMING_PATTERN, &show, &episode, "mkv");
        // Colon should be sanitized
        let path_str = result.to_string_lossy();
        assert!(
            !path_str.contains(':'),
            "Path should not contain colons: {}",
            path_str
        );
        assert!(path_str.contains("Deep Space Nine"));
        assert!(path_str.contains("S01E09"));
        assert!(path_str.contains("The Passenger"));
    }

    #[test]
    fn test_apply_naming_pattern_with_special_characters() {
        use crate::db::{EpisodeRecord, TvShowRecord};
        use uuid::Uuid;

        let show = TvShowRecord {
            id: Uuid::new_v4(),
            library_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "What If...?".to_string(), // Has special characters
            sort_name: None,
            year: Some(2021),
            status: "Running".to_string(),
            tvmaze_id: None,
            tmdb_id: None,
            tvdb_id: None,
            imdb_id: None,
            overview: None,
            network: None,
            runtime: None,
            genres: vec![],
            poster_url: None,
            backdrop_url: None,
            monitored: true,
            monitor_type: "all".to_string(),
            path: None,
            auto_download_override: None,
            backfill_existing: false,
            organize_files_override: None,
            rename_style_override: None,
            auto_hunt_override: None,
            episode_count: None,
            episode_file_count: None,
            size_bytes: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            allowed_resolutions_override: None,
            allowed_video_codecs_override: None,
            allowed_audio_formats_override: None,
            require_hdr_override: None,
            allowed_hdr_types_override: None,
            allowed_sources_override: None,
            release_group_blacklist_override: None,
            release_group_whitelist_override: None,
        };

        let episode = EpisodeRecord {
            id: Uuid::new_v4(),
            tv_show_id: show.id,
            season: 1,
            episode: 1,
            absolute_number: None,
            title: Some("What If... Captain Carter Were the First Avenger?".to_string()),
            overview: None,
            air_date: None,
            runtime: None,
            tvmaze_id: None,
            tmdb_id: None,
            tvdb_id: None,
            status: "wanted".to_string(),
            torrent_link: None,
            torrent_link_added_at: None,
            matched_rss_item_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let result = apply_naming_pattern(DEFAULT_NAMING_PATTERN, &show, &episode, "mkv");
        let path_str = result.to_string_lossy();

        // Should not contain filesystem-invalid characters
        assert!(
            !path_str.contains('?'),
            "Path should not contain question marks: {}",
            path_str
        );
        assert!(
            !path_str.contains('/') || path_str.matches('/').count() <= 2,
            "Should only have expected path separators"
        );
    }

    // =========================================================================
    // Movie Naming Pattern Tests
    // =========================================================================

    #[test]
    fn test_apply_movie_naming_pattern() {
        use crate::db::MovieRecord;
        use uuid::Uuid;

        let movie = MovieRecord {
            id: Uuid::new_v4(),
            library_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: "Jack Ryan: Shadow Recruit".to_string(),
            sort_title: None,
            original_title: None,
            year: Some(2014),
            tmdb_id: Some(137094),
            imdb_id: Some("tt1205537".to_string()),
            overview: None,
            tagline: None,
            runtime: Some(105),
            genres: vec!["Action".to_string(), "Thriller".to_string()],
            production_countries: vec![],
            spoken_languages: vec![],
            director: Some("Kenneth Branagh".to_string()),
            cast_names: vec![],
            tmdb_rating: None,
            tmdb_vote_count: None,
            poster_url: None,
            backdrop_url: None,
            collection_id: None,
            collection_name: None,
            collection_poster_url: None,
            release_date: None,
            certification: None,
            status: None,
            monitored: true,
            has_file: false,
            size_bytes: None,
            path: None,
            download_status: "missing".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            allowed_resolutions_override: None,
            allowed_video_codecs_override: None,
            allowed_audio_formats_override: None,
            require_hdr_override: None,
            allowed_hdr_types_override: None,
            allowed_sources_override: None,
            release_group_blacklist_override: None,
            release_group_whitelist_override: None,
        };

        let result = apply_movie_naming_pattern(
            DEFAULT_MOVIE_NAMING_PATTERN,
            &movie,
            "Jack.Ryan.Shadow.Recruit.2014.1080p.BluRay.mkv",
            "mkv",
        );
        let path_str = result.to_string_lossy();

        // Should contain title and year
        assert!(
            path_str.contains("Jack Ryan"),
            "Should contain movie title: {}",
            path_str
        );
        assert!(
            path_str.contains("2014"),
            "Should contain year: {}",
            path_str
        );
        // Colon should be sanitized
        assert!(
            !path_str.contains(':'),
            "Should not contain colons: {}",
            path_str
        );
    }

    #[test]
    fn test_apply_movie_naming_pattern_no_year() {
        use crate::db::MovieRecord;
        use uuid::Uuid;

        let movie = MovieRecord {
            id: Uuid::new_v4(),
            library_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            title: "Unknown Movie".to_string(),
            year: None, // No year
            sort_title: None,
            original_title: None,
            tmdb_id: None,
            imdb_id: None,
            overview: None,
            tagline: None,
            runtime: None,
            genres: vec![],
            production_countries: vec![],
            spoken_languages: vec![],
            director: None,
            cast_names: vec![],
            tmdb_rating: None,
            tmdb_vote_count: None,
            poster_url: None,
            backdrop_url: None,
            collection_id: None,
            collection_name: None,
            collection_poster_url: None,
            release_date: None,
            certification: None,
            status: None,
            monitored: true,
            has_file: false,
            size_bytes: None,
            path: None,
            download_status: "missing".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            allowed_resolutions_override: None,
            allowed_video_codecs_override: None,
            allowed_audio_formats_override: None,
            require_hdr_override: None,
            allowed_hdr_types_override: None,
            allowed_sources_override: None,
            release_group_blacklist_override: None,
            release_group_whitelist_override: None,
        };

        let result = apply_movie_naming_pattern(
            DEFAULT_MOVIE_NAMING_PATTERN,
            &movie,
            "Unknown.Movie.1080p.BluRay.mkv",
            "mkv",
        );
        let path_str = result.to_string_lossy();

        // Should handle missing year gracefully
        assert!(
            path_str.contains("Unknown Movie"),
            "Should contain movie title: {}",
            path_str
        );
    }

    // =========================================================================
    // Path Safety Tests
    // =========================================================================

    #[test]
    fn test_sanitize_for_filename_comprehensive() {
        // Test various problematic characters
        assert!(!sanitize_for_filename("File: Name").contains(':'));
        assert!(!sanitize_for_filename("File/Name").contains('/'));
        assert!(!sanitize_for_filename("File\\Name").contains('\\'));
        assert!(!sanitize_for_filename("File?Name").contains('?'));
        assert!(!sanitize_for_filename("File*Name").contains('*'));
        assert!(!sanitize_for_filename("File<>Name").contains('<'));
        assert!(!sanitize_for_filename("File<>Name").contains('>'));
        assert!(!sanitize_for_filename("File|Name").contains('|'));
        assert!(!sanitize_for_filename("File\"Name").contains('"'));

        // Normal names should be unchanged
        assert_eq!(sanitize_for_filename("Normal Name"), "Normal Name");
        assert_eq!(
            sanitize_for_filename("Name-With-Dashes"),
            "Name-With-Dashes"
        );
        assert_eq!(sanitize_for_filename("Name.With.Dots"), "Name.With.Dots");
    }

    // =========================================================================
    // Quality Info Extraction Tests
    // =========================================================================

    #[test]
    fn test_extract_quality_info_comprehensive() {
        // Standard scene releases - test that key info is extracted
        let result1 = extract_quality_info("Show.S01E01.1080p.HEVC.x265-GroupName");
        assert!(
            result1.contains("1080p"),
            "Should contain 1080p: {}",
            result1
        );

        let result2 = extract_quality_info("Show.S01E01.720p.x264-FLEET");
        assert!(result2.contains("720p"), "Should contain 720p: {}", result2);

        // 4K HDR
        let result_4k = extract_quality_info("Show.S01E01.2160p.HDR.DV.HEVC-GROUP");
        assert!(
            result_4k.contains("2160p") || result_4k.contains("4K"),
            "Should contain 4K info: {}",
            result_4k
        );

        // Web-DL sources
        let result_web =
            extract_quality_info("Fallout.2024.S01E01.1080p.AMZN.WEB-DL.DDP5.1.H.264-NTb");
        assert!(
            result_web.contains("1080p"),
            "Should contain 1080p: {}",
            result_web
        );
    }

    #[test]
    fn test_extract_quality_info_real_examples() {
        // From the RSS feed - test that key quality info is present
        let examples = vec![
            ("Chicago Fire S14E08 1080p WEB h264-ETHEL", "1080p"),
            (
                "The Daily Show 2026 01 07 Stephen J Dubner 720p HEVC x265-MeGusta",
                "720p",
            ),
            (
                "Old Dog New Tricks S01E05 2025 2160p NF WEB-DL DDP5 1 Atmos HDR H 265-HHWEB",
                "2160p",
            ),
            // Note: XviD detection depends on parser implementation
        ];

        for (filename, expected_res) in examples {
            let result = extract_quality_info(filename);
            assert!(
                result.to_lowercase().contains(&expected_res.to_lowercase()),
                "Expected '{}' to contain '{}', got '{}'",
                filename,
                expected_res,
                result
            );
        }
    }
}
