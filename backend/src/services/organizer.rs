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
    pub fn generate_organized_path(
        &self,
        library_path: &str,
        show: &TvShowRecord,
        episode: &EpisodeRecord,
        original_filename: &str,
        rename_style: RenameStyle,
    ) -> PathBuf {
        let ext = Path::new(original_filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mkv");

        // Create show folder name: "Show Name (Year)" or just "Show Name"
        let show_folder = match show.year {
            Some(year) => format!("{} ({})", sanitize_filename(&show.name), year),
            None => sanitize_filename(&show.name),
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
                    .map(|t| format!(" - {}", sanitize_filename(t)))
                    .unwrap_or_default();
                format!(
                    "{} - S{:02}E{:02}{}.{}",
                    sanitize_filename(&show.name),
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
                    .map(|t| format!(" - {}", sanitize_filename(t)))
                    .unwrap_or_default();

                if quality_info.is_empty() {
                    format!(
                        "{} - S{:02}E{:02}{}.{}",
                        sanitize_filename(&show.name),
                        episode.season,
                        episode.episode,
                        episode_title,
                        ext
                    )
                } else {
                    format!(
                        "{} - S{:02}E{:02}{} [{}].{}",
                        sanitize_filename(&show.name),
                        episode.season,
                        episode.episode,
                        episode_title,
                        quality_info,
                        ext
                    )
                }
            }
        };

        PathBuf::from(library_path)
            .join(&show_folder)
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
        action: &str,
        dry_run: bool,
    ) -> Result<OrganizeResult> {
        let original_path = file.path.clone();
        let original_filename = Path::new(&original_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.mkv");

        let new_path = self.generate_organized_path(
            library_path,
            show,
            episode,
            original_filename,
            rename_style,
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

        // Perform file operation based on action
        let operation_result = match action {
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
                action = %action,
                error = %e,
                "Failed to organize file"
            );
            return Ok(OrganizeResult {
                file_id: file.id,
                original_path,
                new_path: new_path_str,
                success: false,
                error: Some(format!("Failed to {} file: {}", action, e)),
            });
        }

        // Update database
        self.db
            .media_files()
            .mark_organized(file.id, &new_path_str, &original_path)
            .await?;

        info!(
            file_id = %file.id,
            action = %action,
            original = %original_path,
            new = %new_path_str,
            "Successfully organized file"
        );

        Ok(OrganizeResult {
            file_id: file.id,
            original_path,
            new_path: new_path_str,
            success: true,
            error: None,
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
                    &action,
                    false,
                )
                .await?;
            results.push(result);
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
            Some(year) => format!("{} ({})", sanitize_filename(&show.name), year),
            None => sanitize_filename(&show.name),
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

        info!(show_id = %show_id, path = %show_path.display(), "Created show folder structure");

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

    /// Organize files from a completed torrent
    ///
    /// This method:
    /// 1. Gets the torrent files from the torrent service
    /// 2. Parses filenames to identify show/episode
    /// 3. Matches to existing shows or creates new entries
    /// 4. Copies/moves/symlinks files based on library settings
    /// 5. Creates folder structure as needed
    ///
    /// Returns details about what was organized
    pub async fn organize_torrent(
        &self,
        torrent_info_hash: &str,
        torrent_files: Vec<TorrentFileForOrganize>,
        user_id: Uuid,
        library_id: Option<Uuid>,
    ) -> Result<OrganizeTorrentResult> {
        use crate::services::filename_parser;
        use std::path::Path;

        let mut organized_count = 0;
        let mut failed_count = 0;
        let mut messages = Vec::new();

        // Get the library (if specified, use it; otherwise find first TV library for user)
        let library = if let Some(lib_id) = library_id {
            self.db.libraries().get_by_id(lib_id).await?
        } else {
            // Find user's first TV library
            let libraries = self.db.libraries().list_by_user(user_id).await?;
            libraries.into_iter().find(|l| l.library_type == "tv")
        };

        let library = match library {
            Some(l) => l,
            None => {
                return Ok(OrganizeTorrentResult {
                    success: false,
                    organized_count: 0,
                    failed_count: 0,
                    messages: vec!["No TV library found to organize into".to_string()],
                });
            }
        };

        // Get the post_download_action from library
        let action = &library.post_download_action;

        // Log what files we received
        info!(
            file_count = torrent_files.len(),
            library_name = %library.name,
            "Starting torrent organization"
        );

        if torrent_files.is_empty() {
            return Ok(OrganizeTorrentResult {
                success: false,
                organized_count: 0,
                failed_count: 0,
                messages: vec!["No files found in torrent".to_string()],
            });
        }

        // Add all file paths to messages for debugging
        messages.push(format!("Found {} file(s) in torrent", torrent_files.len()));
        for file_info in &torrent_files {
            let filename = std::path::Path::new(&file_info.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file_info.path);
            messages.push(format!("  - {}", filename));
            debug!(path = %file_info.path, size = file_info.size, "Processing file");
        }

        for file_info in torrent_files {
            // Skip non-video files
            if !is_video_file(&file_info.path) {
                debug!(path = %file_info.path, "Skipping non-video file");
                messages.push(format!(
                    "Skipped non-video: {}",
                    Path::new(&file_info.path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&file_info.path)
                ));
                continue;
            }

            let filename = Path::new(&file_info.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file_info.path);

            // Check if file exists
            let source_path = Path::new(&file_info.path);
            if !source_path.exists() {
                messages.push(format!("File not found: {}", filename));
                failed_count += 1;
                continue;
            }

            // Parse the filename to extract show/episode info
            let parsed = filename_parser::parse_episode(filename);

            let show_name = match &parsed.show_name {
                Some(name) => name.clone(),
                None => {
                    messages.push(format!("Could not parse show name from: {}", filename));
                    failed_count += 1;
                    continue;
                }
            };

            let (season, episode) = match (parsed.season, parsed.episode) {
                (Some(s), Some(e)) => (s as i32, e as i32),
                _ => {
                    messages.push(format!("Could not parse season/episode from: {}", filename));
                    failed_count += 1;
                    continue;
                }
            };

            // Try to find an existing show in this library
            let existing_show = self
                .db
                .tv_shows()
                .find_by_name_in_library(library.id, &show_name)
                .await?;

            let show = match existing_show {
                Some(s) => s,
                None => {
                    messages.push(format!(
                        "No matching show '{}' found in library. Add the show first or enable auto-add.",
                        show_name
                    ));
                    failed_count += 1;
                    continue;
                }
            };

            // Find the episode
            let episode_record = self
                .db
                .episodes()
                .get_by_show_season_episode(show.id, season, episode)
                .await?;

            let ep = match episode_record {
                Some(e) => e,
                None => {
                    messages.push(format!(
                        "Episode S{:02}E{:02} not found for show '{}'",
                        season, episode, show.name
                    ));
                    failed_count += 1;
                    continue;
                }
            };

            // Get organize settings for this show
            let (organize_enabled, rename_style) = self.get_show_organize_settings(&show).await?;

            if !organize_enabled {
                messages.push(format!(
                    "Organization disabled for show '{}', skipping {}",
                    show.name, filename
                ));
                continue;
            }

            // Generate the target path
            let target_path =
                self.generate_organized_path(&library.path, &show, &ep, filename, rename_style);

            // Create parent directories
            if let Some(parent) = target_path.parent()
                && let Err(e) = tokio::fs::create_dir_all(parent).await
            {
                messages.push(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ));
                failed_count += 1;
                continue;
            }

            let source_path = Path::new(&file_info.path);
            let target_path_str = target_path.to_string_lossy().to_string();

            // Perform the file operation based on post_download_action
            let operation_result = match action.as_str() {
                "move" => {
                    // Try rename first (same filesystem), fall back to copy+delete
                    match tokio::fs::rename(source_path, &target_path).await {
                        Ok(_) => Ok(()),
                        Err(_) => {
                            // Cross-filesystem: copy then delete
                            tokio::fs::copy(source_path, &target_path).await?;
                            tokio::fs::remove_file(source_path).await?;
                            Ok(())
                        }
                    }
                }
                "hardlink" | "symlink" => {
                    // Create hard link (or fall back to symlink on some systems)
                    #[cfg(unix)]
                    {
                        match tokio::fs::hard_link(source_path, &target_path).await {
                            Ok(_) => Ok(()),
                            Err(_) => {
                                // Fall back to symlink
                                tokio::fs::symlink(source_path, &target_path).await
                            }
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        // Windows: just copy
                        tokio::fs::copy(source_path, &target_path).await.map(|_| ())
                    }
                }
                _ => {
                    // Default: copy
                    tokio::fs::copy(source_path, &target_path).await.map(|_| ())
                }
            };

            match operation_result {
                Ok(_) => {
                    // Create or update media file record
                    let existing_file = self.db.media_files().get_by_path(&target_path_str).await?;

                    if existing_file.is_none() {
                        let size = tokio::fs::metadata(&target_path)
                            .await
                            .map(|m| m.len() as i64)
                            .unwrap_or(file_info.size as i64);

                        self.db
                            .media_files()
                            .create(crate::db::CreateMediaFile {
                                library_id: library.id,
                                path: target_path_str.clone(),
                                size_bytes: size,
                                container: target_path
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .map(|s| s.to_lowercase()),
                                video_codec: parsed.codec.clone(),
                                audio_codec: parsed.audio.clone(),
                                width: None,
                                height: None,
                                duration: None,
                                bitrate: None,
                                file_hash: None,
                                episode_id: Some(ep.id),
                                relative_path: target_path
                                    .strip_prefix(&library.path)
                                    .ok()
                                    .map(|p| p.to_string_lossy().to_string()),
                                original_name: Some(filename.to_string()),
                                resolution: parsed.resolution.clone(),
                                is_hdr: parsed.hdr.is_some().then_some(true),
                                hdr_type: parsed.hdr.clone(),
                            })
                            .await?;
                    }

                    // Mark episode as downloaded
                    self.db
                        .episodes()
                        .update_status(ep.id, "downloaded")
                        .await?;

                    // Update torrent record to link to episode if not already linked
                    self.db
                        .torrents()
                        .link_to_episode(torrent_info_hash, ep.id)
                        .await
                        .ok();

                    // Update show stats
                    self.db.tv_shows().update_stats(show.id).await?;

                    organized_count += 1;
                    messages.push(format!(
                        "Organized: {} -> {}",
                        filename,
                        target_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("?")
                    ));
                }
                Err(e) => {
                    messages.push(format!("Failed to {} file {}: {}", action, filename, e));
                    failed_count += 1;
                }
            }
        }

        // Mark torrent as processed
        self.db
            .torrents()
            .mark_processed(torrent_info_hash)
            .await
            .ok();

        Ok(OrganizeTorrentResult {
            success: organized_count > 0 || failed_count == 0,
            organized_count,
            failed_count,
            messages,
        })
    }
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

/// Check if a file is a video file
fn is_video_file(path: &str) -> bool {
    let video_extensions = [
        ".mkv", ".mp4", ".avi", ".mov", ".wmv", ".flv", ".webm", ".m4v", ".ts", ".m2ts",
    ];
    let lower = path.to_lowercase();
    video_extensions.iter().any(|ext| lower.ends_with(ext))
}

/// Sanitize a string for use as a filename
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Show: Name"), "Show_ Name");
        assert_eq!(sanitize_filename("What/If?"), "What_If_");
        assert_eq!(sanitize_filename("Normal Name"), "Normal Name");
    }

    #[test]
    fn test_extract_quality_info() {
        assert_eq!(
            extract_quality_info("Show.S01E01.1080p.HEVC.x265-GroupName"),
            "1080p HEVC GroupName"
        );
        assert_eq!(
            extract_quality_info("Show.S01E01.720p.x264-FLEET"),
            "720p x264 FLEET"
        );
    }
}
