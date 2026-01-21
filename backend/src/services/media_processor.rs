//! Unified media processing service
//!
//! This module provides a single point of logic for processing completed downloads
//! from any source (torrents, usenet, IRC, FTP, manual imports, etc.):
//! - Processing file-level matches (via file match tables)
//! - Creating media file records
//! - Organizing files into library folder structure
//! - Updating item status from 'downloading' to 'downloaded'
//! - Queueing files for FFmpeg analysis
//!
//! ## File-Level Matching Flow
//!
//! When a download is added, the file matcher service analyzes each file
//! and creates entries in the appropriate file matches table. These entries:
//! - Link individual files to specific episodes/movies/tracks/chapters
//! - Mark files that should be skipped (duplicates, samples, wrong quality)
//! - Update item status to 'downloading'
//!
//! When processing completes:
//! 1. `process_download` checks for file matches
//! 2. If matches exist, uses `process_with_file_matches` to process each file
//! 3. If no matches, uses `process_without_library` to auto-match against all libraries
//!
//! All download processing should go through this service to ensure consistent behavior.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::{debug, error, info, warn};

use crate::db::{
    Database, EpisodeRecord, LibraryRecord, MediaFileRecord, MovieRecord, TorrentFileMatchRecord,
    TorrentRecord, TvShowRecord,
};
use crate::services::extractor::ExtractorService;
use crate::services::file_utils::{get_container, is_audio_file, is_video_file};
use crate::services::filename_parser;
use crate::services::organizer::OrganizerService;
// Quality evaluation imports (used for future quality verification)
use crate::services::TorrentService;
use crate::services::queues::{MediaAnalysisJob, MediaAnalysisQueue};

/// Result of processing a single download (torrent, usenet, or any source)
#[derive(Debug, Clone)]
pub struct ProcessDownloadResult {
    pub success: bool,
    pub matched: bool,
    pub organized: bool,
    pub files_processed: i32,
    pub files_failed: i32,
    pub messages: Vec<String>,
}

impl Default for ProcessDownloadResult {
    fn default() -> Self {
        Self {
            success: false,
            matched: false,
            organized: false,
            files_processed: 0,
            files_failed: 0,
            messages: Vec::new(),
        }
    }
}

/// Legacy alias for backwards compatibility
pub type ProcessTorrentResult = ProcessDownloadResult;

/// Unified media processor service
///
/// Handles all download processing logic regardless of source:
/// - Matching files to library items
/// - Creating media file records  
/// - Organizing files
/// - Updating status
/// - Archive extraction (rar, zip, 7z)
/// - Queueing files for FFmpeg analysis to get real metadata
/// - Quality verification and suboptimal detection
/// - Auto-adding movies from TMDB when `auto_add_discovered` is enabled
///
/// Works with torrents, usenet downloads, and any future download sources.
pub struct MediaProcessor {
    db: Database,
    organizer: OrganizerService,
    /// Optional archive extractor for rar/zip/7z files
    extractor: Option<ExtractorService>,
    /// Optional media analysis queue for FFmpeg metadata extraction
    analysis_queue: Option<Arc<MediaAnalysisQueue>>,
    /// Optional metadata service for auto-adding movies from TMDB
    metadata_service: Option<Arc<crate::services::MetadataService>>,
}

impl MediaProcessor {
    pub fn new(db: Database) -> Self {
        let organizer = OrganizerService::new(db.clone());
        // Create extractor with temp directory from env or default
        let temp_dir = std::env::var("TEMP_PATH").unwrap_or_else(|_| "/tmp/librarian".to_string());
        let extractor = ExtractorService::new(PathBuf::from(temp_dir));
        Self {
            db,
            organizer,
            extractor: Some(extractor),
            analysis_queue: None,
            metadata_service: None,
        }
    }

    /// Create with a media analysis queue for FFmpeg metadata extraction
    pub fn with_analysis_queue(db: Database, queue: Arc<MediaAnalysisQueue>) -> Self {
        let organizer = OrganizerService::new(db.clone());
        let temp_dir = std::env::var("TEMP_PATH").unwrap_or_else(|_| "/tmp/librarian".to_string());
        let extractor = ExtractorService::new(PathBuf::from(temp_dir));
        Self {
            db,
            organizer,
            extractor: Some(extractor),
            analysis_queue: Some(queue),
            metadata_service: None,
        }
    }

    /// Create with both analysis queue and metadata service for full functionality
    pub fn with_services(
        db: Database,
        queue: Arc<MediaAnalysisQueue>,
        metadata: Arc<crate::services::MetadataService>,
    ) -> Self {
        let organizer = OrganizerService::new(db.clone());
        let temp_dir = std::env::var("TEMP_PATH").unwrap_or_else(|_| "/tmp/librarian".to_string());
        let extractor = ExtractorService::new(PathBuf::from(temp_dir));
        Self {
            db,
            organizer,
            extractor: Some(extractor),
            analysis_queue: Some(queue),
            metadata_service: Some(metadata),
        }
    }

    /// Queue a media file for FFmpeg analysis
    async fn queue_for_analysis(&self, media_file: &MediaFileRecord) {
        if let Some(ref queue) = self.analysis_queue {
            let job = MediaAnalysisJob {
                media_file_id: media_file.id,
                path: PathBuf::from(&media_file.path),
                check_subtitles: true, // Enable subtitle checking by default
            };
            match queue.submit(job).await {
                Ok(_) => {
                    debug!(
                        media_file_id = %media_file.id,
                        path = %media_file.path,
                        "Queued file for FFmpeg analysis"
                    );
                }
                Err(e) => {
                    warn!(
                        media_file_id = %media_file.id,
                        error = %e,
                        "Failed to queue file for analysis"
                    );
                }
            }
        }
    }

    /// Resolve the post_download_action for a torrent
    ///
    /// Priority:
    /// 1. Indexer setting (if torrent came from an indexer)
    /// 2. RSS feed setting (if torrent came from a feed)
    /// 3. Library setting (fallback)
    async fn resolve_post_download_action(
        &self,
        torrent: &TorrentRecord,
        library: &LibraryRecord,
    ) -> String {
        // Check indexer first
        if let Some(indexer_id) = &torrent.source_indexer_id {
            if let Ok(Some(indexer)) = self.db.indexers().get(*indexer_id).await {
                if let Some(action) = indexer.post_download_action {
                    debug!(
                        "Using post_download_action '{}' from indexer '{}'",
                        action, indexer.name
                    );
                    return action;
                }
            }
        }

        // Check RSS feed
        if let Some(feed_id) = &torrent.source_feed_id {
            if let Ok(Some(feed)) = self.db.rss_feeds().get_by_id(*feed_id).await {
                if let Some(action) = feed.post_download_action {
                    debug!(
                        "Using post_download_action '{}' from RSS feed '{}'",
                        action, feed.name
                    );
                    return action;
                }
            }
        }

        // Fall back to library setting
        library.post_download_action.clone()
    }

    /// Process a completed torrent download
    ///
    /// This is the main entry point for processing torrent downloads. It handles:
    /// 1. Getting torrent files
    /// 2. Extracting archives (rar, zip, 7z)
    /// 3. Matching to library items based on torrent linkage or filename parsing
    /// 4. Creating media file records
    /// 5. Organizing files if enabled
    /// 6. Updating item status
    ///
    /// The `force` parameter allows reprocessing even if already marked as completed.
    ///
    /// Note: For usenet downloads, use `process_usenet_download()` which follows
    /// the same pipeline but works with usenet-specific data structures.
    pub async fn process_torrent(
        &self,
        torrent_service: &Arc<TorrentService>,
        info_hash: &str,
        force: bool,
    ) -> Result<ProcessTorrentResult> {
        let mut result = ProcessTorrentResult::default();

        // Get torrent record
        let torrent = match self.db.torrents().get_by_info_hash(info_hash).await? {
            Some(t) => t,
            None => {
                result
                    .messages
                    .push(format!("Torrent not found: {}", info_hash));
                return Ok(result);
            }
        };

        // Check if already processed (unless forcing)
        if !force && torrent.post_process_status.as_deref() == Some("completed") {
            result
                .messages
                .push("Torrent already processed".to_string());
            result.success = true;
            return Ok(result);
        }

        // Mark as processing
        self.db
            .torrents()
            .update_post_process_status(info_hash, "processing")
            .await
            .ok();

        // Get files from the torrent
        let files = match torrent_service.get_files_for_torrent(info_hash).await {
            Ok(f) => f,
            Err(e) => {
                let msg = format!("Could not get files for torrent: {}", e);
                warn!(info_hash = %info_hash, "{}", msg);
                result.messages.push(msg);
                self.db
                    .torrents()
                    .update_post_process_status(info_hash, "error")
                    .await
                    .ok();
                return Ok(result);
            }
        };

        if files.is_empty() {
            result
                .messages
                .push("No files found in torrent".to_string());
            self.db
                .torrents()
                .update_post_process_status(info_hash, "completed")
                .await
                .ok();
            result.success = true;
            return Ok(result);
        }

        // Check for and extract any archives in the torrent
        let mut extracted_path: Option<PathBuf> = None;
        if let Some(ref extractor) = self.extractor {
            // Check if any files are archives
            let has_archives = files.iter().any(|f| {
                let path = Path::new(&f.path);
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase());
                matches!(ext.as_deref(), Some("rar") | Some("zip") | Some("7z"))
            });

            if has_archives {
                // Get the torrent's download directory
                if let Some(first_file) = files.first() {
                    let file_path = Path::new(&first_file.path);
                    if let Some(parent) = file_path.parent() {
                        if ExtractorService::needs_extraction(parent) {
                            info!("Extracting archives in '{}'", torrent.name);
                            match extractor.extract_archives(parent).await {
                                Ok(path) => {
                                    if path != parent {
                                        info!("Extracted archives to {}", path.display());
                                        extracted_path = Some(path);
                                    }
                                }
                                Err(e) => {
                                    warn!(
                                        "Archive extraction failed for '{}': {}",
                                        torrent.name, e
                                    );
                                    result
                                        .messages
                                        .push(format!("Archive extraction failed: {}", e));
                                    // Continue processing without extraction
                                }
                            }
                        }
                    }
                }
            }
        }

        // When force=true, delete existing media_file records for files in the downloads folder
        // This allows re-organization of files that were previously processed but not organized correctly
        if force {
            let downloads_path =
                std::env::var("DOWNLOADS_PATH").unwrap_or_else(|_| "/data/downloads".to_string());
            let mut deleted_count = 0;

            for file_info in &files {
                if file_info.path.starts_with(&downloads_path) {
                    if let Some(existing) =
                        self.db.media_files().get_by_path(&file_info.path).await?
                    {
                        debug!(
                            "Force reprocess: deleting existing media_file for '{}'",
                            std::path::Path::new(&file_info.path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(&file_info.path)
                        );
                        // Reset has_file flags on linked items based on media_file links
                        if let Some(movie_id) = existing.movie_id {
                            self.db.movies().update_has_file(movie_id, false).await.ok();
                        }
                        if let Some(album_id) = existing.album_id {
                            self.db
                                .albums()
                                .update_has_files(album_id, false)
                                .await
                                .ok();
                        }
                        if let Some(audiobook_id) = existing.audiobook_id {
                            self.db
                                .audiobooks()
                                .update_has_files(audiobook_id, false)
                                .await
                                .ok();
                        }
                        self.db.media_files().delete(existing.id).await.ok();
                        deleted_count += 1;
                    }
                }
            }

            if deleted_count > 0 {
                debug!(
                    "Force reprocess: cleared {} existing media_file records",
                    deleted_count
                );
            }

            // Get existing file matches to reset item statuses before deleting
            let existing_matches = self
                .db
                .torrent_file_matches()
                .list_by_torrent(torrent.id)
                .await
                .unwrap_or_default();

            // Reset associated item statuses from "downloading" to "wanted"
            for fm in &existing_matches {
                if let Some(track_id) = fm.track_id {
                    self.db.tracks().update_status(track_id, "wanted").await.ok();
                }
                if let Some(chapter_id) = fm.chapter_id {
                    sqlx::query("UPDATE chapters SET status = 'wanted' WHERE id = $1 AND status = 'downloading'")
                        .bind(chapter_id)
                        .execute(self.db.pool())
                        .await
                        .ok();
                }
                if let Some(episode_id) = fm.episode_id {
                    self.db.episodes().update_status(episode_id, "wanted").await.ok();
                }
                if let Some(movie_id) = fm.movie_id {
                    sqlx::query("UPDATE movies SET download_status = 'wanted' WHERE id = $1 AND download_status = 'downloading'")
                        .bind(movie_id)
                        .execute(self.db.pool())
                        .await
                        .ok();
                }
            }

            // DELETE all existing file matches so we can re-run matching from scratch
            // This ensures files that previously didn't match get a chance with improved matching logic
            let deleted_count = self
                .db
                .torrent_file_matches()
                .delete_by_torrent(torrent.id)
                .await
                .unwrap_or(0);

            if deleted_count > 0 || !existing_matches.is_empty() {
                info!(
                    "Force reprocess: deleted {} file matches, reset {} item statuses - will re-run matching",
                    deleted_count,
                    existing_matches.len()
                );
            }
        }

        // Check if we have file-level matches
        let file_matches = self
            .db
            .torrent_file_matches()
            .list_unprocessed(torrent.id)
            .await?;

        // Route processing based on available matches
        let process_result = if !file_matches.is_empty() {
            // Use file-level matching flow
            debug!("Using file-level matching ({} matches)", file_matches.len());
            self.process_with_file_matches(&torrent, &files, file_matches)
                .await
        } else {
            // No file matches - try to auto-match against all user libraries
            self.process_without_library(&torrent, &files).await
        };

        match process_result {
            Ok(r) => {
                result = r;
                // Only mark as completed if we actually matched and organized
                let status = if result.matched && result.organized {
                    "completed"
                } else if result.matched {
                    "matched" // Matched but organize disabled
                } else {
                    "unmatched" // No match found - can be retried
                };
                self.db
                    .torrents()
                    .update_post_process_status(info_hash, status)
                    .await
                    .ok();
            }
            Err(e) => {
                result.messages.push(format!("Processing error: {}", e));
                self.db
                    .torrents()
                    .update_post_process_status(info_hash, "error")
                    .await
                    .ok();
            }
        }

        Ok(result)
    }

    // Legacy process_linked_* methods removed - all processing now uses
    // process_with_file_matches or process_without_library

    /// Process a torrent using file-level matches from torrent_file_matches table
    ///
    /// This is the new unified processing flow that uses pre-computed file matches
    /// created when the torrent was added. Each file is processed independently
    /// based on its matched target (episode, movie, track, or chapter).
    async fn process_with_file_matches(
        &self,
        torrent: &TorrentRecord,
        files: &[crate::services::torrent::TorrentFile],
        file_matches: Vec<TorrentFileMatchRecord>,
    ) -> Result<ProcessTorrentResult> {
        let mut result = ProcessTorrentResult::default();
        result.matched = true; // We have pre-matched files

        // Count match types for logging
        let episode_count = file_matches
            .iter()
            .filter(|m| m.episode_id.is_some())
            .count();
        let movie_count = file_matches.iter().filter(|m| m.movie_id.is_some()).count();
        let track_count = file_matches.iter().filter(|m| m.track_id.is_some()).count();
        let chapter_count = file_matches
            .iter()
            .filter(|m| m.chapter_id.is_some())
            .count();

        info!(
            "Organizing '{}': {} matched files ({} episodes, {} movies, {} tracks, {} chapters)",
            torrent.name,
            file_matches.len(),
            episode_count,
            movie_count,
            track_count,
            chapter_count
        );

        // Build a map of file index to file info
        let file_map: std::collections::HashMap<i32, &crate::services::torrent::TorrentFile> =
            files
                .iter()
                .enumerate()
                .map(|(i, f)| (i as i32, f))
                .collect();

        // Track which items need status updates after all files are processed
        let mut episodes_to_update: Vec<uuid::Uuid> = Vec::new();
        let mut movies_to_update: Vec<uuid::Uuid> = Vec::new();
        let mut tracks_to_update: Vec<uuid::Uuid> = Vec::new();
        let mut chapters_to_update: Vec<uuid::Uuid> = Vec::new();

        for file_match in &file_matches {
            // Skip if already processed or marked to skip
            if file_match.processed || file_match.skip_download {
                continue;
            }

            // Get the actual file info
            let file_info = match file_map.get(&file_match.file_index) {
                Some(f) => f,
                None => {
                    warn!("File index {} not found in torrent", file_match.file_index);
                    continue;
                }
            };

            // Get file name for logging
            let file_name = std::path::Path::new(&file_match.file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file_match.file_path);

            // Process based on what the file is matched to
            let process_result = if let Some(episode_id) = file_match.episode_id {
                self.process_matched_episode_file(file_match, file_info, episode_id)
                    .await
            } else if let Some(movie_id) = file_match.movie_id {
                self.process_matched_movie_file(file_match, file_info, movie_id)
                    .await
            } else if let Some(track_id) = file_match.track_id {
                self.process_matched_track_file(file_match, file_info, track_id)
                    .await
            } else if let Some(chapter_id) = file_match.chapter_id {
                self.process_matched_chapter_file(file_match, file_info, chapter_id)
                    .await
            } else {
                // Unmatched file - just mark as processed
                self.db
                    .torrent_file_matches()
                    .mark_processed(
                        file_match.id,
                        crate::db::MarkProcessed {
                            media_file_id: None,
                            error_message: None,
                        },
                    )
                    .await
                    .ok();
                continue;
            };

            match process_result {
                Ok(Some(media_file_id)) => {
                    // Mark the file match as processed
                    self.db
                        .torrent_file_matches()
                        .mark_processed(
                            file_match.id,
                            crate::db::MarkProcessed {
                                media_file_id: Some(media_file_id),
                                error_message: None,
                            },
                        )
                        .await
                        .ok();

                    info!("Organized file '{}'", file_name);

                    result.files_processed += 1;
                    result.organized = true;

                    // Track items for status update
                    if let Some(episode_id) = file_match.episode_id {
                        if !episodes_to_update.contains(&episode_id) {
                            episodes_to_update.push(episode_id);
                        }
                    }
                    if let Some(movie_id) = file_match.movie_id {
                        if !movies_to_update.contains(&movie_id) {
                            movies_to_update.push(movie_id);
                        }
                    }
                    if let Some(track_id) = file_match.track_id {
                        if !tracks_to_update.contains(&track_id) {
                            tracks_to_update.push(track_id);
                        }
                    }
                    if let Some(chapter_id) = file_match.chapter_id {
                        if !chapters_to_update.contains(&chapter_id) {
                            chapters_to_update.push(chapter_id);
                        }
                    }
                }
                Ok(None) => {
                    // File was processed but no media file created (e.g., organize disabled)
                    self.db
                        .torrent_file_matches()
                        .mark_processed(
                            file_match.id,
                            crate::db::MarkProcessed {
                                media_file_id: None,
                                error_message: None,
                            },
                        )
                        .await
                        .ok();
                    result.files_processed += 1;
                }
                Err(e) => {
                    // Mark as processed with error
                    self.db
                        .torrent_file_matches()
                        .mark_processed(
                            file_match.id,
                            crate::db::MarkProcessed {
                                media_file_id: None,
                                error_message: Some(e.to_string()),
                            },
                        )
                        .await
                        .ok();
                    error!("Failed to organize '{}': {}", file_name, e);
                    result.files_failed += 1;
                    result
                        .messages
                        .push(format!("Failed to process {}: {}", file_match.file_path, e));
                }
            }
        }

        // Update item statuses from 'downloading' to 'downloaded'
        for episode_id in &episodes_to_update {
            self.db
                .episodes()
                .update_status(*episode_id, "downloaded")
                .await
                .ok();
            if let Ok(Some(episode)) = self.db.episodes().get_by_id(*episode_id).await {
                self.db
                    .tv_shows()
                    .update_stats(episode.tv_show_id)
                    .await
                    .ok();
            }
        }

        for movie_id in &movies_to_update {
            self.db.movies().update_has_file(*movie_id, true).await.ok();
            sqlx::query("UPDATE movies SET download_status = 'downloaded' WHERE id = $1")
                .bind(movie_id)
                .execute(self.db.pool())
                .await
                .ok();
        }

        for track_id in &tracks_to_update {
            self.db
                .tracks()
                .update_status(*track_id, "downloaded")
                .await
                .ok();
        }

        for chapter_id in &chapters_to_update {
            sqlx::query("UPDATE chapters SET status = 'downloaded' WHERE id = $1")
                .bind(chapter_id)
                .execute(self.db.pool())
                .await
                .ok();
        }

        result.success = result.files_processed > 0 || result.files_failed == 0;

        let items_updated = episodes_to_update.len()
            + movies_to_update.len()
            + tracks_to_update.len()
            + chapters_to_update.len();
        if items_updated > 0 {
            info!(
                "Marked {} items as downloaded for '{}'",
                items_updated, torrent.name
            );
        }

        Ok(result)
    }

    /// Process a single file matched to an episode
    async fn process_matched_episode_file(
        &self,
        _file_match: &TorrentFileMatchRecord,
        file_info: &crate::services::torrent::TorrentFile,
        episode_id: uuid::Uuid,
    ) -> Result<Option<uuid::Uuid>> {
        let episode = self
            .db
            .episodes()
            .get_by_id(episode_id)
            .await?
            .context("Episode not found")?;
        let show = self
            .db
            .tv_shows()
            .get_by_id(episode.tv_show_id)
            .await?
            .context("Show not found")?;
        let library = self
            .db
            .libraries()
            .get_by_id(show.library_id)
            .await?
            .context("Library not found")?;

        // Create media file record
        let parsed = filename_parser::parse_episode(
            Path::new(&file_info.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(""),
        );

        let media_file = self
            .db
            .media_files()
            .upsert(crate::db::CreateMediaFile {
                library_id: library.id,
                path: file_info.path.clone(),
                size_bytes: file_info.size as i64,
                container: get_container(&file_info.path),
                video_codec: parsed.codec.clone(),
                audio_codec: parsed.audio.clone(),
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                file_hash: None,
                episode_id: Some(episode_id),
                movie_id: None,
                relative_path: None,
                original_name: Path::new(&file_info.path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                resolution: parsed.resolution.clone(),
                is_hdr: parsed.hdr.is_some().then_some(true),
                hdr_type: parsed.hdr.clone(),
            })
            .await?;

        // Queue for analysis
        self.queue_for_analysis(&media_file).await;

        // Organize if enabled
        if library.organize_files {
            // Get rename style from library
            let rename_style = match library.rename_style.as_str() {
                "clean" => crate::services::organizer::RenameStyle::Clean,
                "preserve_info" => crate::services::organizer::RenameStyle::PreserveInfo,
                _ => crate::services::organizer::RenameStyle::None,
            };

            match self
                .organizer
                .organize_file(
                    &media_file,
                    &show,
                    &episode,
                    &library.path,
                    rename_style,
                    library.naming_pattern.as_deref(),
                    &library.post_download_action,
                    false,
                )
                .await
            {
                Ok(org_result) if org_result.success => {
                    info!(
                        episode = %format!("{} S{:02}E{:02}", show.name, episode.season, episode.episode),
                        new_path = %org_result.new_path,
                        "Organized {} S{:02}E{:02} → {}",
                        show.name, episode.season, episode.episode, org_result.new_path
                    );
                }
                Ok(org_result) => {
                    warn!(
                        episode = %format!("{} S{:02}E{:02}", show.name, episode.season, episode.episode),
                        error = ?org_result.error,
                        "Failed to organize episode file"
                    );
                }
                Err(e) => {
                    warn!(error = %e, "Error organizing episode file");
                }
            }
        }

        Ok(Some(media_file.id))
    }

    /// Process a single file matched to a movie
    async fn process_matched_movie_file(
        &self,
        _file_match: &TorrentFileMatchRecord,
        file_info: &crate::services::torrent::TorrentFile,
        movie_id: uuid::Uuid,
    ) -> Result<Option<uuid::Uuid>> {
        let movie = self
            .db
            .movies()
            .get_by_id(movie_id)
            .await?
            .context("Movie not found")?;
        let library = self
            .db
            .libraries()
            .get_by_id(movie.library_id)
            .await?
            .context("Library not found")?;

        // Create media file record
        let parsed = filename_parser::parse_movie(
            Path::new(&file_info.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(""),
        );

        let media_file = self
            .db
            .media_files()
            .upsert(crate::db::CreateMediaFile {
                library_id: library.id,
                path: file_info.path.clone(),
                size_bytes: file_info.size as i64,
                container: get_container(&file_info.path),
                video_codec: parsed.codec.clone(),
                audio_codec: parsed.audio.clone(),
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                file_hash: None,
                episode_id: None,
                movie_id: Some(movie_id),
                relative_path: None,
                original_name: Path::new(&file_info.path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                resolution: parsed.resolution.clone(),
                is_hdr: parsed.hdr.is_some().then_some(true),
                hdr_type: parsed.hdr.clone(),
            })
            .await?;

        // Queue for analysis
        self.queue_for_analysis(&media_file).await;

        // Organize if enabled
        if library.organize_files {
            match self
                .organizer
                .organize_movie_file(
                    &media_file,
                    &movie,
                    &library.path,
                    library.naming_pattern.as_deref(),
                    &library.post_download_action,
                    false,
                )
                .await
            {
                Ok(org_result) if org_result.success => {
                    info!(
                        movie = %movie.title,
                        new_path = %org_result.new_path,
                        "Organized movie '{}' → {}",
                        movie.title, org_result.new_path
                    );
                }
                Ok(org_result) => {
                    warn!(
                        movie = %movie.title,
                        error = ?org_result.error,
                        "Failed to organize movie file"
                    );
                }
                Err(e) => {
                    warn!(error = %e, "Error organizing movie file");
                }
            }
        }

        Ok(Some(media_file.id))
    }

    /// Process a single file matched to a track
    async fn process_matched_track_file(
        &self,
        _file_match: &TorrentFileMatchRecord,
        file_info: &crate::services::torrent::TorrentFile,
        track_id: uuid::Uuid,
    ) -> Result<Option<uuid::Uuid>> {
        let track = self
            .db
            .tracks()
            .get_by_id(track_id)
            .await?
            .context("Track not found")?;
        let album = self
            .db
            .albums()
            .get_by_id(track.album_id)
            .await?
            .context("Album not found")?;
        let library = self
            .db
            .libraries()
            .get_by_id(track.library_id)
            .await?
            .context("Library not found")?;

        // Create media file record
        let media_file = self
            .db
            .media_files()
            .upsert(crate::db::CreateMediaFile {
                library_id: library.id,
                path: file_info.path.clone(),
                size_bytes: file_info.size as i64,
                container: get_container(&file_info.path),
                video_codec: None,
                audio_codec: None,
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                file_hash: None,
                episode_id: None,
                movie_id: None,
                relative_path: None,
                original_name: Path::new(&file_info.path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                resolution: None,
                is_hdr: None,
                hdr_type: None,
            })
            .await?;

        // Link to track and album
        self.db
            .media_files()
            .link_to_track(media_file.id, track_id)
            .await?;
        self.db
            .media_files()
            .link_to_album(media_file.id, album.id)
            .await?;

        // Link track to media file
        self.db
            .tracks()
            .link_media_file(track_id, media_file.id)
            .await?;

        // Queue for analysis
        self.queue_for_analysis(&media_file).await;

        // Organize music file if enabled
        if library.organize_files {
            // Get artist name for path
            let artist_name = if let Ok(Some(artist)) = self.db.albums().get_artist_by_id(album.artist_id).await {
                artist.name
            } else {
                "Unknown Artist".to_string()
            };

            match self
                .organizer
                .organize_music_file(
                    &media_file,
                    &artist_name,
                    &album,
                    Some(&track),
                    &library.path,
                    library.naming_pattern.as_deref(),
                    &library.post_download_action,
                    false,
                )
                .await
            {
                Ok(org_result) if org_result.success => {
                    info!(
                        track = %track.title,
                        album = %album.name,
                        artist = %artist_name,
                        new_path = %org_result.new_path,
                        "Organized music track → {}",
                        org_result.new_path
                    );
                }
                Ok(org_result) => {
                    warn!(
                        track = %track.title,
                        album = %album.name,
                        error = ?org_result.error,
                        "Failed to organize music track"
                    );
                }
                Err(e) => {
                    warn!(
                        track = %track.title,
                        album = %album.name,
                        error = %e,
                        "Error organizing music track"
                    );
                }
            }
        }

        Ok(Some(media_file.id))
    }

    /// Process a single file matched to an audiobook chapter
    async fn process_matched_chapter_file(
        &self,
        _file_match: &TorrentFileMatchRecord,
        file_info: &crate::services::torrent::TorrentFile,
        chapter_id: uuid::Uuid,
    ) -> Result<Option<uuid::Uuid>> {
        // Get chapter info
        let chapter: (uuid::Uuid, i32) =
            sqlx::query_as("SELECT audiobook_id, chapter_number FROM chapters WHERE id = $1")
                .bind(chapter_id)
                .fetch_one(self.db.pool())
                .await?;

        let audiobook = self
            .db
            .audiobooks()
            .get_by_id(chapter.0)
            .await?
            .context("Audiobook not found")?;
        let library = self
            .db
            .libraries()
            .get_by_id(audiobook.library_id)
            .await?
            .context("Library not found")?;

        // Create media file record
        let media_file = self
            .db
            .media_files()
            .upsert(crate::db::CreateMediaFile {
                library_id: library.id,
                path: file_info.path.clone(),
                size_bytes: file_info.size as i64,
                container: get_container(&file_info.path),
                video_codec: None,
                audio_codec: None,
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                file_hash: None,
                episode_id: None,
                movie_id: None,
                relative_path: None,
                original_name: Path::new(&file_info.path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                resolution: None,
                is_hdr: None,
                hdr_type: None,
            })
            .await?;

        // Link to audiobook
        self.db
            .media_files()
            .link_to_audiobook(media_file.id, audiobook.id)
            .await?;

        // Update chapter to point to this media file
        sqlx::query("UPDATE chapters SET media_file_id = $2 WHERE id = $1")
            .bind(chapter_id)
            .bind(media_file.id)
            .execute(self.db.pool())
            .await?;

        // Queue for analysis
        self.queue_for_analysis(&media_file).await;

        Ok(Some(media_file.id))
    }

    /// Process a torrent with no library linked
    /// Attempts to match against all user libraries (TV, movies, music, audiobooks)
    async fn process_without_library(
        &self,
        torrent: &TorrentRecord,
        files: &[crate::services::torrent::TorrentFile],
    ) -> Result<ProcessTorrentResult> {
        let mut result = ProcessTorrentResult::default();

        let libraries = self.db.libraries().list_by_user(torrent.user_id).await?;
        if libraries.is_empty() {
            result
                .messages
                .push("No libraries found for user".to_string());
            return Ok(result);
        }

        // Separate libraries by type
        let tv_libraries: Vec<_> = libraries
            .iter()
            .filter(|l| l.library_type == "tv")
            .collect();
        let movie_libraries: Vec<_> = libraries
            .iter()
            .filter(|l| l.library_type == "movies")
            .collect();
        let music_libraries: Vec<_> = libraries
            .iter()
            .filter(|l| l.library_type == "music")
            .collect();
        let audiobook_libraries: Vec<_> = libraries
            .iter()
            .filter(|l| l.library_type == "audiobooks")
            .collect();

        for file_info in files {
            let file_path = &file_info.path;

            // Skip if already exists (add message for tracking)
            if self.db.media_files().exists_by_path(file_path).await? {
                debug!(path = %file_path, "Media file already exists");
                result.files_processed += 1;
                result.messages.push(format!(
                    "File already tracked: {}",
                    Path::new(file_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(file_path)
                ));
                continue;
            }

            let mut matched = false;

            // Process video files (TV shows and movies)
            if is_video_file(file_path) {
                // Try TV libraries first (more specific matching)
                for lib in &tv_libraries {
                    if let Some((show, episode)) = self.try_match_tv_file(file_path, lib).await? {
                        let file_result = self
                            .process_video_file(
                                file_path,
                                file_info.size as i64,
                                lib,
                                Some(&show),
                                Some(&episode),
                            )
                            .await;

                        match file_result {
                            Ok(organized) => {
                                result.matched = true;
                                result.files_processed += 1;
                                if organized {
                                    result.organized = true;
                                }
                                // Note: file-level matching happens via media_file linkage
                                result.messages.push(format!(
                                    "Matched to {} S{:02}E{:02} in library '{}'",
                                    show.name, episode.season, episode.episode, lib.name
                                ));
                                matched = true;
                                break;
                            }
                            Err(e) => {
                                result.messages.push(format!("Failed to process: {}", e));
                            }
                        }
                    }
                }

                // If no TV match, try movie libraries
                if !matched && !movie_libraries.is_empty() {
                    for lib in &movie_libraries {
                        if let Some(movie) = self.try_match_movie_file(file_path, lib).await? {
                            match self
                                .process_video_file_for_movie(
                                    file_path,
                                    file_info.size as i64,
                                    lib,
                                    &movie,
                                )
                                .await
                            {
                                Ok(true) => {
                                    result.matched = true;
                                    result.files_processed += 1;
                                    // Note: file-level matching happens via media_file linkage
                                    result.messages.push(format!(
                                        "Matched to {} ({}) in library '{}'",
                                        movie.title,
                                        movie.year.map(|y| y.to_string()).unwrap_or_default(),
                                        lib.name
                                    ));
                                    matched = true;
                                    break;
                                }
                                Ok(false) => {
                                    // File already exists
                                    result.files_processed += 1;
                                    matched = true;
                                    break;
                                }
                                Err(e) => {
                                    result
                                        .messages
                                        .push(format!("Failed to process movie: {}", e));
                                }
                            }
                        }
                    }
                }
            }

            // Process audio files (music and audiobooks)
            if !matched && is_audio_file(file_path) {
                // Try to match against music libraries first
                for library in &music_libraries {
                    if let Ok(Some((album, artist_name))) =
                        self.try_match_music_file(file_path, library).await
                    {
                        debug!(
                            path = %file_path,
                            album = %album.name,
                            artist = %artist_name,
                            library = %library.name,
                            "Matched audio file to album"
                        );
                        if let Err(e) = self
                            .create_linked_music_file(
                                file_path,
                                file_info.size as i64,
                                library,
                                &album,
                            )
                            .await
                        {
                            result
                                .messages
                                .push(format!("Error creating music file: {}", e));
                        } else {
                            result.files_processed += 1;
                            matched = true;
                        }
                        break;
                    }
                }

                // If not matched as music, try audiobook libraries
                if !matched {
                    for library in &audiobook_libraries {
                        if let Ok(Some((audiobook, author_name))) =
                            self.try_match_audiobook_file(file_path, library).await
                        {
                            debug!(
                                path = %file_path,
                                audiobook = %audiobook.title,
                                author = %author_name,
                                library = %library.name,
                                "Matched audio file to audiobook"
                            );
                            if let Err(e) = self
                                .create_linked_audiobook_file(
                                    file_path,
                                    file_info.size as i64,
                                    library,
                                    &audiobook,
                                )
                                .await
                            {
                                result
                                    .messages
                                    .push(format!("Error creating audiobook file: {}", e));
                            } else {
                                result.files_processed += 1;
                                matched = true;
                            }
                            break;
                        }
                    }
                }
            }

            if !matched {
                result.messages.push(format!(
                    "No match in any library: {}",
                    Path::new(file_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(file_path)
                ));
            }
        }

        result.success = result.files_processed > 0;
        Ok(result)
    }

    /// Try to match a video file to a TV show and episode
    async fn try_match_tv_file(
        &self,
        file_path: &str,
        library: &crate::db::libraries::LibraryRecord,
    ) -> Result<Option<(TvShowRecord, EpisodeRecord)>> {
        let filename = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);

        let parsed = filename_parser::parse_episode(filename);

        let show_name = match &parsed.show_name {
            Some(name) => name,
            None => {
                debug!(filename = %filename, "Could not parse show name");
                return Ok(None);
            }
        };

        let (season, episode) = match (parsed.season, parsed.episode) {
            (Some(s), Some(e)) => (s as i32, e as i32),
            _ => {
                debug!(filename = %filename, "Could not parse season/episode");
                return Ok(None);
            }
        };

        // Find matching show in library
        let show = match self
            .db
            .tv_shows()
            .find_by_name_in_library(library.id, show_name)
            .await
        {
            Ok(Some(s)) => s,
            Ok(None) => {
                debug!(
                    show_name = %show_name,
                    library = %library.name,
                    "Show not found in library"
                );
                return Ok(None);
            }
            Err(e) => {
                warn!(
                    show_name = %show_name,
                    error = %e,
                    "Error finding show in library"
                );
                return Ok(None);
            }
        };

        // Find matching episode
        let ep = match self
            .db
            .episodes()
            .get_by_show_season_episode(show.id, season, episode)
            .await
        {
            Ok(Some(e)) => e,
            Ok(None) => {
                debug!(
                    show = %show.name,
                    season = season,
                    episode = episode,
                    "Episode not found"
                );
                return Ok(None);
            }
            Err(e) => {
                warn!(
                    show = %show.name,
                    season = season,
                    episode = episode,
                    error = %e,
                    "Error finding episode"
                );
                return Ok(None);
            }
        };

        info!(
            show = %show.name,
            season = season,
            episode = episode,
            filename = %filename,
            "Matched '{}' → {} S{:02}E{:02}",
            filename, show.name, season, episode
        );

        Ok(Some((show, ep)))
    }

    /// Try to match a video file to a movie in the library
    async fn try_match_movie_file(
        &self,
        file_path: &str,
        library: &crate::db::libraries::LibraryRecord,
    ) -> Result<Option<MovieRecord>> {
        let filename = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);

        // First, check if this looks like a TV show (has S01E01 notation)
        // If so, don't try to match as a movie - TV matching should handle it
        let episode_parsed = filename_parser::parse_episode(filename);
        if episode_parsed.season.is_some() && episode_parsed.episode.is_some() {
            debug!(
                filename = %filename,
                season = ?episode_parsed.season,
                episode = ?episode_parsed.episode,
                "File looks like TV episode, skipping movie matching"
            );
            return Ok(None);
        }

        // Parse the movie filename
        let parsed = filename_parser::parse_movie(filename);

        let movie_title = match &parsed.show_name {
            Some(title) => title,
            None => {
                debug!(filename = %filename, "Could not parse movie title");
                return Ok(None);
            }
        };

        let year = parsed.year.map(|y| y as i32);

        // Find matching movie in library
        let movie = match self
            .db
            .movies()
            .find_by_title_in_library(library.id, movie_title, year)
            .await
        {
            Ok(Some(m)) => m,
            Ok(None) => {
                debug!(
                    movie_title = %movie_title,
                    year = ?year,
                    library = %library.name,
                    "Movie not found in library, checking auto_add_discovered"
                );

                // If auto_add_discovered is enabled, try to add from TMDB
                if library.auto_add_discovered {
                    if let Some(auto_added) = self
                        .try_auto_add_movie_from_tmdb(movie_title, year, library)
                        .await?
                    {
                        info!(
                            movie = %auto_added.title,
                            year = ?auto_added.year,
                            tmdb_id = ?auto_added.tmdb_id,
                            "Auto-added movie from TMDB"
                        );
                        return Ok(Some(auto_added));
                    }
                }

                return Ok(None);
            }
            Err(e) => {
                warn!(
                    movie_title = %movie_title,
                    error = %e,
                    "Error finding movie in library"
                );
                return Ok(None);
            }
        };

        info!(
            movie = %movie.title,
            year = ?movie.year,
            filename = %filename,
            "Matched file to movie"
        );

        Ok(Some(movie))
    }

    /// Try to auto-add a movie from TMDB when auto_add_discovered is enabled
    async fn try_auto_add_movie_from_tmdb(
        &self,
        title: &str,
        year: Option<i32>,
        library: &crate::db::libraries::LibraryRecord,
    ) -> Result<Option<MovieRecord>> {
        let metadata_service = match &self.metadata_service {
            Some(ms) => ms,
            None => {
                debug!("MetadataService not available, cannot auto-add movie");
                return Ok(None);
            }
        };

        // Search TMDB for the movie
        let search_results = match metadata_service.search_movies(title, year).await {
            Ok(results) => results,
            Err(e) => {
                warn!(title = %title, year = ?year, error = %e, "Failed to search TMDB for movie");
                return Ok(None);
            }
        };

        if search_results.is_empty() {
            debug!(title = %title, year = ?year, "No TMDB results for movie");
            return Ok(None);
        }

        // Take the first result - TMDB typically returns the best match first
        // Only auto-add if we have high confidence (exact year match or high popularity)
        let best_match = &search_results[0];

        // Confidence check: If we have a year, it should match within 1 year
        let year_matches = match (year, best_match.year) {
            (Some(parsed_year), Some(result_year)) => (parsed_year - result_year).abs() <= 1,
            (None, _) => true,        // No year in filename, accept any
            (Some(_), None) => false, // We have year but result doesn't, skip
        };

        if !year_matches {
            debug!(
                title = %title,
                parsed_year = ?year,
                result_year = ?best_match.year,
                "Year mismatch, skipping auto-add"
            );
            return Ok(None);
        }

        // Add the movie to the library
        info!(
            title = %best_match.title,
            tmdb_id = best_match.provider_id,
            year = ?best_match.year,
            library = %library.name,
            "Auto-adding movie from TMDB"
        );

        let movie_record = match metadata_service
            .add_movie_from_provider(crate::services::AddMovieOptions {
                provider: crate::services::MetadataProvider::Tmdb,
                provider_id: best_match.provider_id,
                library_id: library.id,
                user_id: library.user_id,
                monitored: true,
                path: None,
            })
            .await
        {
            Ok(record) => record,
            Err(e) => {
                warn!(
                    title = %best_match.title,
                    tmdb_id = best_match.provider_id,
                    error = %e,
                    "Failed to auto-add movie from TMDB"
                );
                return Ok(None);
            }
        };

        Ok(Some(movie_record))
    }

    /// Try to auto-add an album from MusicBrainz when auto_add_discovered is enabled
    async fn try_auto_add_album(
        &self,
        torrent_name: &str,
        library: &crate::db::libraries::LibraryRecord,
    ) -> Result<Option<crate::db::albums::AlbumRecord>> {
        let metadata_service = match &self.metadata_service {
            Some(ms) => ms,
            None => {
                debug!("MetadataService not available, cannot auto-add album");
                return Ok(None);
            }
        };

        // Parse album info from torrent name
        // Common patterns: "Artist - Album (Year)" or "Artist-Album-YEAR-FORMAT"
        let cleaned = torrent_name.replace('_', " ").replace('.', " ");

        // Try to extract artist and album from the name
        let (artist_name, album_name) = if let Some(pos) = cleaned.find(" - ") {
            let artist = cleaned[..pos].trim().to_string();
            let rest = &cleaned[pos + 3..];
            // Remove year and format info from album name
            let album = rest
                .split(|c: char| c == '(' || c == '[' || c.is_ascii_digit())
                .next()
                .unwrap_or(rest)
                .trim()
                .to_string();
            (artist, album)
        } else if cleaned.contains('-') {
            // Try splitting on first hyphen
            let parts: Vec<&str> = cleaned.splitn(2, '-').collect();
            if parts.len() == 2 {
                let artist = parts[0].trim().to_string();
                // Clean up album part
                let album_part = parts[1].trim();
                let album = album_part
                    .split(|c: char| c.is_ascii_digit())
                    .next()
                    .unwrap_or(album_part)
                    .trim()
                    .replace('-', " ")
                    .trim()
                    .to_string();
                (artist, album)
            } else {
                debug!(torrent = %torrent_name, "Could not parse artist/album from torrent name");
                return Ok(None);
            }
        } else {
            debug!(torrent = %torrent_name, "Could not parse artist/album from torrent name");
            return Ok(None);
        };

        if artist_name.is_empty() || album_name.is_empty() {
            debug!(torrent = %torrent_name, "Empty artist or album name parsed");
            return Ok(None);
        }

        info!(
            artist = %artist_name,
            album = %album_name,
            "Searching MusicBrainz for album"
        );

        // Search MusicBrainz
        let query = format!("{} {}", artist_name, album_name);
        let search_results = match metadata_service.search_albums(&query).await {
            Ok(results) => results,
            Err(e) => {
                warn!(query = %query, error = %e, "Failed to search MusicBrainz for album");
                return Ok(None);
            }
        };

        if search_results.is_empty() {
            // Try album name only
            let search_results = match metadata_service.search_albums(&album_name).await {
                Ok(results) => results,
                Err(e) => {
                    warn!(album = %album_name, error = %e, "Failed to search MusicBrainz for album (retry)");
                    return Ok(None);
                }
            };
            if search_results.is_empty() {
                debug!(artist = %artist_name, album = %album_name, "No MusicBrainz results for album");
                return Ok(None);
            }
        }

        let best_match = &search_results[0];

        info!(
            title = %best_match.title,
            artist = ?best_match.artist_name,
            provider_id = %best_match.provider_id,
            "Auto-adding album from MusicBrainz"
        );

        // Add the album from provider
        let album_record = match metadata_service
            .add_album_from_provider(crate::services::metadata::AddAlbumOptions {
                musicbrainz_id: best_match.provider_id.clone(),
                library_id: library.id,
                user_id: library.user_id,
                monitored: true,
            })
            .await
        {
            Ok(record) => record,
            Err(e) => {
                warn!(
                    title = %best_match.title,
                    provider_id = %best_match.provider_id,
                    error = %e,
                    "Failed to auto-add album from MusicBrainz"
                );
                return Ok(None);
            }
        };

        Ok(Some(album_record))
    }

    /// Try to auto-add an audiobook from Audible when auto_add_discovered is enabled
    async fn try_auto_add_audiobook(
        &self,
        torrent_name: &str,
        library: &crate::db::libraries::LibraryRecord,
    ) -> Result<Option<crate::db::audiobooks::AudiobookRecord>> {
        let metadata_service = match &self.metadata_service {
            Some(ms) => ms,
            None => {
                debug!("MetadataService not available, cannot auto-add audiobook");
                return Ok(None);
            }
        };

        // Parse audiobook info from torrent name
        // Common patterns: "Author - Title" or "Title by Author"
        let cleaned = torrent_name.replace('_', " ").replace('.', " ");

        // Try to extract author and title
        let (author_name, title) = if cleaned.to_lowercase().contains(" by ") {
            let lower = cleaned.to_lowercase();
            if let Some(pos) = lower.find(" by ") {
                let title = cleaned[..pos].trim().to_string();
                let author = cleaned[pos + 4..]
                    .split(|c: char| c == '(' || c == '[')
                    .next()
                    .unwrap_or(&cleaned[pos + 4..])
                    .trim()
                    .to_string();
                (author, title)
            } else {
                return Ok(None);
            }
        } else if let Some(pos) = cleaned.find(" - ") {
            let author = cleaned[..pos].trim().to_string();
            let title = cleaned[pos + 3..]
                .split(|c: char| c == '(' || c == '[')
                .next()
                .unwrap_or(&cleaned[pos + 3..])
                .trim()
                .to_string();
            (author, title)
        } else {
            debug!(torrent = %torrent_name, "Could not parse author/title from audiobook torrent name");
            return Ok(None);
        };

        if title.is_empty() {
            debug!(torrent = %torrent_name, "Empty title parsed from audiobook torrent");
            return Ok(None);
        }

        info!(
            author = %author_name,
            title = %title,
            "Searching Audible for audiobook"
        );

        // Search for the audiobook
        let query = if author_name.is_empty() {
            title.clone()
        } else {
            format!("{} {}", author_name, title)
        };

        let search_results = match metadata_service.search_audiobooks(&query).await {
            Ok(results) => results,
            Err(e) => {
                warn!(query = %query, error = %e, "Failed to search Audible for audiobook");
                return Ok(None);
            }
        };

        if search_results.is_empty() {
            debug!(author = %author_name, title = %title, "No OpenLibrary results for audiobook");
            return Ok(None);
        }

        let best_match = &search_results[0];

        info!(
            title = %best_match.title,
            author = ?best_match.author_name,
            provider_id = %best_match.provider_id,
            "Auto-adding audiobook from OpenLibrary"
        );

        // Add the audiobook from provider
        let audiobook_record = match metadata_service
            .add_audiobook_from_provider(crate::services::metadata::AddAudiobookOptions {
                openlibrary_id: best_match.provider_id.clone(),
                library_id: library.id,
                user_id: library.user_id,
                monitored: true,
            })
            .await
        {
            Ok(record) => record,
            Err(e) => {
                warn!(
                    title = %best_match.title,
                    provider_id = %best_match.provider_id,
                    error = %e,
                    "Failed to auto-add audiobook from OpenLibrary"
                );
                return Ok(None);
            }
        };

        Ok(Some(audiobook_record))
    }

    /// Process a video file for a movie - create media record, organize, and link to movie
    ///
    /// Returns (success, organized) tuple
    async fn process_video_file_for_movie(
        &self,
        file_path: &str,
        size_bytes: i64,
        library: &crate::db::libraries::LibraryRecord,
        movie: &MovieRecord,
    ) -> Result<bool> {
        let parsed = filename_parser::parse_movie(
            Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(""),
        );

        // Use upsert to handle existing records gracefully
        let media_file = self
            .db
            .media_files()
            .upsert(crate::db::CreateMediaFile {
                library_id: library.id,
                path: file_path.to_string(),
                size_bytes,
                container: get_container(file_path),
                video_codec: parsed.codec.clone(),
                audio_codec: parsed.audio.clone(),
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                file_hash: None,
                episode_id: None,
                movie_id: Some(movie.id),
                relative_path: None,
                original_name: Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                resolution: parsed.resolution.clone(),
                is_hdr: parsed.hdr.is_some().then_some(true),
                hdr_type: parsed.hdr.clone(),
            })
            .await?;

        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);
        info!(
            file_id = %media_file.id,
            movie_id = %movie.id,
            path = %file_path,
            "Created media file record: '{}' for movie '{}'",
            file_name, movie.title
        );

        // Queue for FFmpeg analysis to get real metadata
        self.queue_for_analysis(&media_file).await;

        // Organize the file if library has organize_files enabled
        if library.organize_files {
            match self
                .organizer
                .organize_movie_file(
                    &media_file,
                    movie,
                    &library.path,
                    library.naming_pattern.as_deref(),
                    &library.post_download_action,
                    false,
                )
                .await
            {
                Ok(org_result) if org_result.success => {
                    info!(
                        movie = %movie.title,
                        new_path = %org_result.new_path,
                        "Organized movie '{}' → {}",
                        movie.title, org_result.new_path
                    );
                }
                Ok(org_result) => {
                    warn!(
                        movie = %movie.title,
                        error = ?org_result.error,
                        "Failed to organize movie file"
                    );
                }
                Err(e) => {
                    error!(movie = %movie.title, error = %e, "Error organizing movie file");
                }
            }
        }

        // Update movie status
        self.db.movies().update_has_file(movie.id, true).await?;

        // Calculate size for the movie
        let file_size = std::fs::metadata(file_path)
            .map(|m| m.len() as i64)
            .unwrap_or(size_bytes);
        self.db
            .movies()
            .update_file_status(movie.id, true, Some(file_size))
            .await?;

        Ok(true)
    }

    /// Process a video file - create media record and optionally organize
    async fn process_video_file(
        &self,
        file_path: &str,
        size_bytes: i64,
        library: &crate::db::libraries::LibraryRecord,
        show: Option<&TvShowRecord>,
        episode: Option<&EpisodeRecord>,
    ) -> Result<bool> {
        let parsed = filename_parser::parse_episode(
            Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(""),
        );

        // Use upsert to handle existing records gracefully
        let media_file = self
            .db
            .media_files()
            .upsert(crate::db::CreateMediaFile {
                library_id: library.id,
                path: file_path.to_string(),
                size_bytes,
                container: get_container(file_path),
                video_codec: parsed.codec.clone(),
                audio_codec: parsed.audio.clone(),
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                file_hash: None,
                episode_id: episode.map(|e| e.id),
                movie_id: None,
                relative_path: None,
                original_name: Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                resolution: parsed.resolution.clone(),
                is_hdr: parsed.hdr.is_some().then_some(true),
                hdr_type: parsed.hdr.clone(),
            })
            .await?;

        let episode_desc = if let (Some(s), Some(ep)) = (&show, &episode) {
            format!(" for {} S{:02}E{:02}", s.name, ep.season, ep.episode)
        } else {
            String::new()
        };
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);
        info!(
            file_id = %media_file.id,
            path = %file_path,
            episode = ?episode.map(|e| e.id),
            "Created media file record: '{}'{}",
            file_name, episode_desc
        );

        // Queue for FFmpeg analysis to get real metadata (resolution, codec, HDR, etc.)
        self.queue_for_analysis(&media_file).await;

        // Update episode status if linked
        if let Some(ep) = episode {
            self.db
                .episodes()
                .update_status(ep.id, "downloaded")
                .await?;
            if let Some(s) = show {
                self.db.tv_shows().update_stats(s.id).await?;
            }
        }

        // Organize if enabled
        let mut organized = false;
        if let (Some(s), Some(ep)) = (show, episode) {
            let (organize_enabled, rename_style, action) =
                self.organizer.get_full_organize_settings(s).await?;

            if organize_enabled {
                match self
                    .organizer
                    .organize_file(
                        &media_file,
                        s,
                        ep,
                        &library.path,
                        rename_style,
                        library.naming_pattern.as_deref(),
                        &action,
                        false,
                    )
                    .await
                {
                    Ok(org_result) if org_result.success => {
                        info!(
                            new_path = %org_result.new_path,
                            "Organized file → {}",
                            org_result.new_path
                        );
                        organized = true;
                    }
                    Ok(org_result) => {
                        warn!(
                            error = ?org_result.error,
                            "Failed to organize file"
                        );
                    }
                    Err(e) => {
                        error!(error = %e, "Error organizing file");
                    }
                }
            }
        }

        Ok(organized)
    }

    /// Try to organize an existing media file
    async fn try_organize_file(
        &self,
        media_file: &crate::db::MediaFileRecord,
        library: &crate::db::libraries::LibraryRecord,
        show: &TvShowRecord,
        episode: &EpisodeRecord,
    ) -> Result<bool> {
        let (organize_enabled, rename_style, action) =
            self.organizer.get_full_organize_settings(show).await?;

        if !organize_enabled {
            return Ok(false);
        }

        match self
            .organizer
            .organize_file(
                media_file,
                show,
                episode,
                &library.path,
                rename_style,
                library.naming_pattern.as_deref(),
                &action,
                false,
            )
            .await
        {
            Ok(result) if result.success => {
                info!("File organized to {}", result.new_path);
                Ok(true)
            }
            Ok(result) => {
                warn!("Failed to organize file: {:?}", result.error);
                Ok(false)
            }
            Err(e) => {
                error!(error = %e, "Error organizing file");
                Ok(false)
            }
        }
    }

    /// Create or update an unlinked media file record
    async fn create_unlinked_media_file(
        &self,
        file_path: &str,
        size_bytes: i64,
        library: &crate::db::libraries::LibraryRecord,
    ) -> Result<crate::db::MediaFileRecord> {
        let parsed = filename_parser::parse_episode(
            Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(""),
        );

        // Use upsert to handle existing records gracefully
        let media_file = self
            .db
            .media_files()
            .upsert(crate::db::CreateMediaFile {
                library_id: library.id,
                path: file_path.to_string(),
                size_bytes,
                container: get_container(file_path),
                video_codec: parsed.codec,
                audio_codec: parsed.audio,
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                file_hash: None,
                episode_id: None,
                movie_id: None,
                relative_path: None,
                original_name: Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                resolution: parsed.resolution,
                is_hdr: parsed.hdr.is_some().then_some(true),
                hdr_type: parsed.hdr,
            })
            .await?;

        debug!(
            file_id = %media_file.id,
            path = %file_path,
            "Upserted unlinked media file"
        );

        // Queue for FFmpeg analysis to get real metadata
        self.queue_for_analysis(&media_file).await;

        Ok(media_file)
    }

    /// Create or update an unlinked audio file record (for music/audiobooks)
    async fn create_unlinked_audio_file(
        &self,
        file_path: &str,
        size_bytes: i64,
        library: &crate::db::libraries::LibraryRecord,
    ) -> Result<crate::db::MediaFileRecord> {
        // Use upsert to handle existing records gracefully
        let media_file = self
            .db
            .media_files()
            .upsert(crate::db::CreateMediaFile {
                library_id: library.id,
                path: file_path.to_string(),
                size_bytes,
                container: get_container(file_path),
                video_codec: None,
                audio_codec: None,
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                file_hash: None,
                episode_id: None,
                movie_id: None,
                relative_path: None,
                original_name: Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                resolution: None,
                is_hdr: None,
                hdr_type: None,
            })
            .await?;

        debug!(
            file_id = %media_file.id,
            path = %file_path,
            "Upserted unlinked audio file"
        );

        // Queue for FFmpeg analysis to get real audio metadata
        self.queue_for_analysis(&media_file).await;

        Ok(media_file)
    }

    /// Process all unmatched completed torrents
    ///
    /// This is called on startup to retry matching for torrents that:
    /// - Are in 'seeding' state (download complete)
    /// - Have 'unmatched' or NULL post_process_status
    /// - Have library_id set
    pub async fn process_unmatched_torrents(
        &self,
        torrent_service: &Arc<TorrentService>,
        trigger_reason: &str,
    ) -> Result<i32> {
        let torrents = self.db.torrents().list_unmatched().await?;

        if torrents.is_empty() {
            return Ok(0);
        }

        info!(
            "Found {} unmatched torrents to retry (triggered by {})",
            torrents.len(),
            trigger_reason
        );

        let mut processed = 0;
        for torrent in torrents {
            match self
                .process_torrent(torrent_service, &torrent.info_hash, true)
                .await
            {
                Ok(result) => {
                    if result.matched {
                        processed += 1;
                        info!("Matched previously unmatched torrent '{}'", torrent.name);
                    }
                }
                Err(e) => {
                    warn!("Failed to match '{}': {}", torrent.name, e);
                }
            }
        }

        Ok(processed)
    }

    /// Process all pending torrents
    ///
    /// This is called periodically to process torrents that are:
    /// - In 'seeding' state (download complete)
    /// - Have 'pending' or NULL post_process_status
    pub async fn process_pending_torrents(
        &self,
        torrent_service: &Arc<TorrentService>,
        trigger_reason: &str,
    ) -> Result<i32> {
        let torrents = self.db.torrents().list_pending_processing().await?;

        if torrents.is_empty() {
            return Ok(0);
        }

        info!(
            "Found {} pending torrents to process (triggered by {})",
            torrents.len(),
            trigger_reason
        );

        let mut processed = 0;
        for torrent in torrents {
            info!(
                "Processing '{}' (triggered by {})",
                torrent.name, trigger_reason
            );

            match self
                .process_torrent(torrent_service, &torrent.info_hash, false)
                .await
            {
                Ok(result) => {
                    if result.success {
                        processed += 1;
                        if result.files_processed > 0 {
                            info!(
                                "Finished processing '{}': {} files organized",
                                torrent.name, result.files_processed
                            );
                        }
                    }
                    if !result.messages.is_empty() {
                        for msg in &result.messages {
                            info!("'{}': {}", torrent.name, msg);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to process '{}': {}", torrent.name, e);
                }
            }
        }

        Ok(processed)
    }

    /// Process a completed usenet download using the unified pipeline
    ///
    /// Process a completed usenet download using the unified pipeline
    ///
    /// This method provides the same functionality as process_torrent but for
    /// usenet downloads. It:
    /// 1. Gets the download path and scans for media files
    /// 2. Extracts archives if present (rar, zip, 7z)
    /// 3. Matches files to library items (using linked item or auto-matching)
    /// 4. Creates media file records
    /// 5. Organizes files (if enabled for the library)
    /// 6. Updates status
    /// 7. Queues for FFmpeg analysis
    pub async fn process_usenet_download(
        &self,
        download: &crate::db::usenet_downloads::UsenetDownloadRecord,
        _usenet_service: &std::sync::Arc<crate::services::UsenetService>,
        trigger_reason: &str,
    ) -> Result<ProcessDownloadResult> {
        info!(
            "Processing usenet download '{}' (triggered by {})",
            download.nzb_name, trigger_reason
        );
        
        let mut result = ProcessDownloadResult::default();
        
        // Get download path
        let download_path_str = match &download.download_path {
            Some(path) => path.clone(),
            None => {
                result.messages.push("No download path available".to_string());
                return Ok(result);
            }
        };
        let download_path = PathBuf::from(&download_path_str);
        
        // Check if path exists
        if !tokio::fs::try_exists(&download_path).await.unwrap_or(false) {
            result.messages.push(format!("Download path not found: {}", download_path.display()));
            return Ok(result);
        }
        
        // Scan for media files (recursively)
        let mut media_files = Vec::new();
        self.scan_directory_for_media(&download_path, &mut media_files).await?;
        
        // Check for and extract archives first
        if let Some(ref extractor) = self.extractor {
            let has_archives = media_files.iter().any(|f| {
                let ext = Path::new(f)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase());
                matches!(ext.as_deref(), Some("rar") | Some("zip") | Some("7z"))
            });
            
            if has_archives {
                info!("Found archives in usenet download, extracting...");
                match extractor.extract_archives(&download_path).await {
                    Ok(extracted_path) => {
                        info!("Extracted archives to: {}", extracted_path.display());
                        // Re-scan to pick up extracted files
                        media_files.clear();
                        self.scan_directory_for_media(&download_path, &mut media_files).await?;
                        // Also scan the extraction directory
                        self.scan_directory_for_media(&extracted_path, &mut media_files).await?;
                    }
                    Err(e) => {
                        warn!("Archive extraction failed: {}", e);
                    }
                }
            }
        }
        
        // Filter to only actual media files (not archives)
        let media_files: Vec<String> = media_files
            .into_iter()
            .filter(|f| is_video_file(f) || is_audio_file(f))
            .collect();
        
        if media_files.is_empty() {
            result.messages.push("No media files found in download".to_string());
            result.success = true;
            return Ok(result);
        }
        
        info!("Found {} media files to process", media_files.len());
        
        // Get user's libraries for matching
        let libraries = self.db.libraries().list_by_user(download.user_id).await?;
        if libraries.is_empty() {
            result.messages.push("No libraries found for user".to_string());
            return Ok(result);
        }
        
        // If download is linked to a specific library/item, use that
        // Otherwise, auto-match to any library
        let target_library = if let Some(lib_id) = download.library_id {
            libraries.iter().find(|l| l.id == lib_id).cloned()
        } else {
            None
        };
        
        // Process each file
        for file_path in &media_files {
            match self.process_usenet_file(
                file_path,
                download,
                target_library.as_ref(),
                &libraries,
            ).await {
                Ok(processed) => {
                    if processed {
                        result.files_processed += 1;
                        result.organized = true;
                    } else {
                        result.files_failed += 1;
                    }
                }
                Err(e) => {
                    warn!(file = %file_path, error = %e, "Failed to process usenet file");
                    result.files_failed += 1;
                    result.messages.push(format!("Failed to process {}: {}", file_path, e));
                }
            }
        }
        
        result.matched = result.files_processed > 0;
        result.success = result.files_processed > 0 || result.files_failed == 0;
        
        if result.files_processed > 0 {
            result.messages.push(format!(
                "Processed {} files from usenet download '{}'",
                result.files_processed, download.nzb_name
            ));
        }
        
        Ok(result)
    }
    
    /// Recursively scan a directory for media files
    async fn scan_directory_for_media(&self, dir: &Path, files: &mut Vec<String>) -> Result<()> {
        let mut entries = tokio::fs::read_dir(dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                // Recursively scan subdirectories
                Box::pin(self.scan_directory_for_media(&path, files)).await?;
            } else if path.is_file() {
                if let Some(path_str) = path.to_str() {
                    // Include video, audio, and archive files
                    if is_video_file(path_str) || is_audio_file(path_str) 
                       || path_str.ends_with(".rar") || path_str.ends_with(".zip") || path_str.ends_with(".7z") {
                        files.push(path_str.to_string());
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Process a single file from a usenet download
    async fn process_usenet_file(
        &self,
        file_path: &str,
        download: &crate::db::usenet_downloads::UsenetDownloadRecord,
        target_library: Option<&LibraryRecord>,
        all_libraries: &[LibraryRecord],
    ) -> Result<bool> {
        // Skip if file already exists in media_files
        if self.db.media_files().exists_by_path(file_path).await? {
            debug!(path = %file_path, "Media file already exists, skipping");
            return Ok(true);
        }
        
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);
        
        // Determine file type
        let is_video = is_video_file(file_path);
        let is_audio = is_audio_file(file_path);
        
        // Try to match the file to a library item
        if is_video {
            // Try to match to TV episode or movie
            if let Some(library) = target_library {
                // Use the linked library
                if library.library_type == "tv" {
                    return self.process_usenet_tv_file(file_path, file_name, library, download).await;
                } else if library.library_type == "movies" {
                    return self.process_usenet_movie_file(file_path, file_name, library, download).await;
                }
            }
            
            // Auto-match: try TV libraries first, then movies
            let tv_libraries: Vec<_> = all_libraries.iter().filter(|l| l.library_type == "tv").collect();
            for library in tv_libraries {
                if let Ok(true) = self.process_usenet_tv_file(file_path, file_name, library, download).await {
                    return Ok(true);
                }
            }
            
            let movie_libraries: Vec<_> = all_libraries.iter().filter(|l| l.library_type == "movies").collect();
            for library in movie_libraries {
                if let Ok(true) = self.process_usenet_movie_file(file_path, file_name, library, download).await {
                    return Ok(true);
                }
            }
        } else if is_audio {
            // Try to match to music or audiobooks
            if let Some(library) = target_library {
                if library.library_type == "music" {
                    return self.process_usenet_music_file(file_path, library, download).await;
                } else if library.library_type == "audiobooks" {
                    return self.process_usenet_audiobook_file(file_path, library, download).await;
                }
            }
            
            // Auto-match: try music libraries first, then audiobooks
            let music_libraries: Vec<_> = all_libraries.iter().filter(|l| l.library_type == "music").collect();
            for library in music_libraries {
                if let Ok(true) = self.process_usenet_music_file(file_path, library, download).await {
                    return Ok(true);
                }
            }
            
            let audiobook_libraries: Vec<_> = all_libraries.iter().filter(|l| l.library_type == "audiobooks").collect();
            for library in audiobook_libraries {
                if let Ok(true) = self.process_usenet_audiobook_file(file_path, library, download).await {
                    return Ok(true);
                }
            }
        }
        
        debug!(path = %file_path, "Could not match file to any library item");
        Ok(false)
    }
    
    /// Process a TV file from usenet download
    async fn process_usenet_tv_file(
        &self,
        file_path: &str,
        file_name: &str,
        library: &LibraryRecord,
        download: &crate::db::usenet_downloads::UsenetDownloadRecord,
    ) -> Result<bool> {
        // Parse filename to get show/season/episode info
        let parsed = filename_parser::parse_episode(file_name);
        
        let show_name = match &parsed.show_name {
            Some(t) if !t.is_empty() => t.clone(),
            _ => return Ok(false),
        };
        
        let (season, episode) = match (parsed.season, parsed.episode) {
            (Some(s), Some(e)) => (s, e),
            _ => return Ok(false),
        };
        
        // Find matching show in library
        let shows = self.db.tv_shows().list_by_library(library.id).await?;
        let matched_show = shows.iter().find(|s| {
            crate::services::text_utils::show_name_similarity(&s.name, &show_name) > 0.7
        });
        
        let show = match matched_show {
            Some(s) => s.clone(),
            None => {
                debug!(show = %show_name, "No matching show found in library");
                return Ok(false);
            }
        };
        
        // Find matching episode
        let episodes = self.db.episodes().list_by_show(show.id).await?;
        let matched_episode = episodes.iter().find(|ep| {
            ep.season == season as i32 && ep.episode == episode as i32
        });
        
        let episode_record = match matched_episode {
            Some(ep) => ep.clone(),
            None => {
                debug!(show = %show_name, season = season, episode = episode, "Episode not found");
                return Ok(false);
            }
        };
        
        info!(
            show = %show.name,
            season = season,
            episode = episode,
            "Matched usenet file to episode"
        );
        
        // Organize the file if organization is enabled
        // For TV, we build the target path: library_path/Show Name/Season XX/filename
        let final_path = if library.organize_files {
            match self.organize_tv_file_simple(file_path, &show, &episode_record, library).await {
                Ok(new_path) => {
                    info!(from = %file_path, to = %new_path, "Organized episode file");
                    new_path
                }
                Err(e) => {
                    warn!(error = %e, "Failed to organize episode file, using original path");
                    file_path.to_string()
                }
            }
        } else {
            file_path.to_string()
        };
        
        // Create media file record
        let media_file = self.create_media_file_record(
            &final_path,
            library.id,
            Some(episode_record.id),
            None, // movie_id
            None, // album_id  
            None, // audiobook_id
            Some(&download.nzb_name),
        ).await?;
        
        // Queue for FFmpeg analysis
        if let Some(ref queue) = self.analysis_queue {
            let job = MediaAnalysisJob {
                media_file_id: media_file.id,
                path: PathBuf::from(&final_path),
                check_subtitles: true,
            };
            let _ = queue.submit(job).await;
        }
        
        // Update episode status
        self.db.episodes().update_status(episode_record.id, "downloaded").await.ok();
        
        Ok(true)
    }
    
    /// Process a movie file from usenet download
    async fn process_usenet_movie_file(
        &self,
        file_path: &str,
        file_name: &str,
        library: &LibraryRecord,
        download: &crate::db::usenet_downloads::UsenetDownloadRecord,
    ) -> Result<bool> {
        // Parse filename
        let parsed = filename_parser::parse_movie(file_name);
        
        let movie_name = match &parsed.show_name {
            Some(t) if !t.is_empty() => t.clone(),
            _ => return Ok(false),
        };
        
        // Find matching movie in library
        let movies = self.db.movies().list_by_library(library.id).await?;
        let matched_movie = movies.iter().find(|m| {
            crate::services::text_utils::show_name_similarity(&m.title, &movie_name) > 0.7
        });
        
        let movie = match matched_movie {
            Some(m) => m.clone(),
            None => {
                debug!(title = %movie_name, "No matching movie found in library");
                return Ok(false);
            }
        };
        
        info!(title = %movie.title, "Matched usenet file to movie");
        
        // Organize the file if enabled
        let final_path = if library.organize_files {
            match self.organize_movie_file_simple(file_path, &movie, library).await {
                Ok(new_path) => {
                    info!(from = %file_path, to = %new_path, "Organized movie file");
                    new_path
                }
                Err(e) => {
                    warn!(error = %e, "Failed to organize movie file, using original path");
                    file_path.to_string()
                }
            }
        } else {
            file_path.to_string()
        };
        
        // Create media file record
        let media_file = self.create_media_file_record(
            &final_path,
            library.id,
            None, // episode_id
            Some(movie.id),
            None, // album_id
            None, // audiobook_id
            Some(&download.nzb_name),
        ).await?;
        
        // Queue for FFmpeg analysis
        if let Some(ref queue) = self.analysis_queue {
            let job = MediaAnalysisJob {
                media_file_id: media_file.id,
                path: PathBuf::from(&final_path),
                check_subtitles: true,
            };
            let _ = queue.submit(job).await;
        }
        
        // Update movie file status
        self.db.movies().update_file_status(movie.id, true, None).await.ok();
        
        Ok(true)
    }
    
    /// Process a music file from usenet download
    async fn process_usenet_music_file(
        &self,
        file_path: &str,
        library: &LibraryRecord,
        download: &crate::db::usenet_downloads::UsenetDownloadRecord,
    ) -> Result<bool> {
        // Try to match using existing music matching logic
        if let Ok(Some((album, _artist_name))) = self.try_match_music_file(file_path, library).await {
            info!(album = %album.name, "Matched usenet file to album");
            
            // Organize if enabled
            let final_path = if library.organize_files {
                match self.organize_music_file_simple(file_path, &album, library).await {
                    Ok(new_path) => new_path,
                    Err(_) => file_path.to_string(),
                }
            } else {
                file_path.to_string()
            };
            
            // Create media file record
            let media_file = self.create_media_file_record(
                &final_path,
                library.id,
                None,
                None,
                Some(album.id),
                None,
                Some(&download.nzb_name),
            ).await?;
            
            // Queue for analysis
            if let Some(ref queue) = self.analysis_queue {
                let job = MediaAnalysisJob {
                    media_file_id: media_file.id,
                    path: PathBuf::from(&final_path),
                    check_subtitles: true,
                };
                let _ = queue.submit(job).await;
            }
            
            return Ok(true);
        }
        
        Ok(false)
    }
    
    /// Process an audiobook file from usenet download
    async fn process_usenet_audiobook_file(
        &self,
        file_path: &str,
        library: &LibraryRecord,
        download: &crate::db::usenet_downloads::UsenetDownloadRecord,
    ) -> Result<bool> {
        // Try to match using existing audiobook matching logic
        if let Ok(Some((audiobook, _author_name))) = self.try_match_audiobook_file(file_path, library).await {
            info!(title = %audiobook.title, "Matched usenet file to audiobook");
            
            // Organize if enabled
            let final_path = if library.organize_files {
                match self.organize_audiobook_file_simple(file_path, &audiobook, library).await {
                    Ok(new_path) => new_path,
                    Err(_) => file_path.to_string(),
                }
            } else {
                file_path.to_string()
            };
            
            // Create media file record
            let media_file = self.create_media_file_record(
                &final_path,
                library.id,
                None,
                None,
                None,
                Some(audiobook.id),
                Some(&download.nzb_name),
            ).await?;
            
            // Queue for analysis
            if let Some(ref queue) = self.analysis_queue {
                let job = MediaAnalysisJob {
                    media_file_id: media_file.id,
                    path: PathBuf::from(&final_path),
                    check_subtitles: true,
                };
                let _ = queue.submit(job).await;
            }
            
            return Ok(true);
        }
        
        Ok(false)
    }
    
    /// Create a media file record in the database
    async fn create_media_file_record(
        &self,
        file_path: &str,
        library_id: uuid::Uuid,
        episode_id: Option<uuid::Uuid>,
        movie_id: Option<uuid::Uuid>,
        _album_id: Option<uuid::Uuid>,
        _audiobook_id: Option<uuid::Uuid>,
        source_name: Option<&str>,
    ) -> Result<MediaFileRecord> {
        let path_obj = Path::new(file_path);
        let file_name = path_obj.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
        let container = get_container(file_path);
        
        // Get file size
        let size_bytes = tokio::fs::metadata(file_path)
            .await
            .map(|m| m.len() as i64)
            .unwrap_or(0);
        
        let record = self.db.media_files().create(crate::db::media_files::CreateMediaFile {
            library_id,
            episode_id,
            movie_id,
            path: file_path.to_string(),
            size_bytes,
            container,
            original_name: Some(format!("{} ({})", file_name, source_name.unwrap_or("unknown"))),
            ..Default::default()
        }).await?;
        
        Ok(record)
    }
    
    /// Simple file organization for TV episodes
    /// Moves file to: library_path/Show Name/Season XX/filename
    async fn organize_tv_file_simple(
        &self,
        source_path: &str,
        show: &TvShowRecord,
        episode: &EpisodeRecord,
        library: &LibraryRecord,
    ) -> Result<String> {
        use crate::services::file_utils::sanitize_for_filename;
        
        let library_path = &library.path;
        if library_path.is_empty() {
            return Ok(source_path.to_string());
        }
        
        let show_folder = sanitize_for_filename(&show.name);
        let season_folder = format!("Season {:02}", episode.season);
        
        // Build new filename: Show Name - S01E01 - Episode Title.ext
        let source = Path::new(source_path);
        let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("mkv");
        let episode_title = episode.title.as_deref().unwrap_or("Episode");
        let new_filename = format!(
            "{} - S{:02}E{:02} - {}.{}",
            sanitize_for_filename(&show.name),
            episode.season,
            episode.episode,
            sanitize_for_filename(episode_title),
            ext
        );
        
        let target_dir = PathBuf::from(library_path).join(&show_folder).join(&season_folder);
        let target_path = target_dir.join(&new_filename);
        
        // Create directories
        tokio::fs::create_dir_all(&target_dir).await?;
        
        // Move the file (try rename first, fallback to copy+delete for cross-device)
        if tokio::fs::rename(source_path, &target_path).await.is_err() {
            // Cross-device move: copy then delete
            tokio::fs::copy(source_path, &target_path).await?;
            tokio::fs::remove_file(source_path).await?;
        }
        
        Ok(target_path.to_string_lossy().to_string())
    }
    
    /// Simple file organization for movies
    /// Moves file to: library_path/Movie Name (Year)/filename
    async fn organize_movie_file_simple(
        &self,
        source_path: &str,
        movie: &MovieRecord,
        library: &LibraryRecord,
    ) -> Result<String> {
        use crate::services::file_utils::sanitize_for_filename;
        
        let library_path = &library.path;
        if library_path.is_empty() {
            return Ok(source_path.to_string());
        }
        
        // Build folder name: Movie Title (Year)
        let year = movie.year.map(|y| format!(" ({})", y)).unwrap_or_default();
        let movie_folder = format!("{}{}", sanitize_for_filename(&movie.title), year);
        
        // Build new filename
        let source = Path::new(source_path);
        let ext = source.extension().and_then(|e| e.to_str()).unwrap_or("mkv");
        let new_filename = format!("{}{}.{}", sanitize_for_filename(&movie.title), year, ext);
        
        let target_dir = PathBuf::from(library_path).join(&movie_folder);
        let target_path = target_dir.join(&new_filename);
        
        // Create directories
        tokio::fs::create_dir_all(&target_dir).await?;
        
        // Move the file (try rename first, fallback to copy+delete for cross-device)
        if tokio::fs::rename(source_path, &target_path).await.is_err() {
            tokio::fs::copy(source_path, &target_path).await?;
            tokio::fs::remove_file(source_path).await?;
        }
        
        Ok(target_path.to_string_lossy().to_string())
    }
    
    /// Simple file organization for music
    /// Moves file to: library_path/Artist/Album/filename
    async fn organize_music_file_simple(
        &self,
        source_path: &str,
        album: &crate::db::AlbumRecord,
        library: &LibraryRecord,
    ) -> Result<String> {
        use crate::services::file_utils::sanitize_for_filename;
        
        let library_path = &library.path;
        if library_path.is_empty() {
            return Ok(source_path.to_string());
        }
        
        // Get artist name
        let artist_name = if let Some(artist) = self.db.albums().get_artist_by_id(album.artist_id).await? {
            artist.name
        } else {
            "Unknown Artist".to_string()
        };
        
        let artist_folder = sanitize_for_filename(&artist_name);
        let album_folder = sanitize_for_filename(&album.name);
        
        let source = Path::new(source_path);
        let filename = source.file_name().and_then(|n| n.to_str()).unwrap_or("track.mp3");
        
        let target_dir = PathBuf::from(library_path).join(&artist_folder).join(&album_folder);
        let target_path = target_dir.join(filename);
        
        // Create directories
        tokio::fs::create_dir_all(&target_dir).await?;
        
        // Move the file (try rename first, fallback to copy+delete for cross-device)
        if tokio::fs::rename(source_path, &target_path).await.is_err() {
            tokio::fs::copy(source_path, &target_path).await?;
            tokio::fs::remove_file(source_path).await?;
        }
        
        Ok(target_path.to_string_lossy().to_string())
    }
    
    /// Simple file organization for audiobooks
    /// Moves file to: library_path/Author/Book Title/filename
    async fn organize_audiobook_file_simple(
        &self,
        source_path: &str,
        audiobook: &crate::db::AudiobookRecord,
        library: &LibraryRecord,
    ) -> Result<String> {
        use crate::services::file_utils::sanitize_for_filename;
        
        let library_path = &library.path;
        if library_path.is_empty() {
            return Ok(source_path.to_string());
        }
        
        // Get author name
        let author_name = if let Some(author_id) = audiobook.author_id {
            if let Ok(Some(author)) = self.db.audiobooks().get_author_by_id(author_id).await {
                author.name
            } else {
                "Unknown Author".to_string()
            }
        } else {
            "Unknown Author".to_string()
        };
        
        let author_folder = sanitize_for_filename(&author_name);
        let book_folder = sanitize_for_filename(&audiobook.title);
        
        let source = Path::new(source_path);
        let filename = source.file_name().and_then(|n| n.to_str()).unwrap_or("chapter.mp3");
        
        let target_dir = PathBuf::from(library_path).join(&author_folder).join(&book_folder);
        let target_path = target_dir.join(filename);
        
        // Create directories
        tokio::fs::create_dir_all(&target_dir).await?;
        
        // Move the file (try rename first, fallback to copy+delete for cross-device)
        if tokio::fs::rename(source_path, &target_path).await.is_err() {
            tokio::fs::copy(source_path, &target_path).await?;
            tokio::fs::remove_file(source_path).await?;
        }
        
        Ok(target_path.to_string_lossy().to_string())
    }

    /// Try to match an audio file to a music album by reading ID3 tags
    async fn try_match_music_file(
        &self,
        file_path: &str,
        library: &crate::db::libraries::LibraryRecord,
    ) -> Result<Option<(crate::db::AlbumRecord, String)>> {
        // Read ID3 tags from the file
        let meta = match Self::read_audio_metadata_static(file_path) {
            Some(m) => m,
            None => return Ok(None),
        };

        // We need at least artist and album to match
        let artist_name = match meta.artist {
            Some(a) if !a.is_empty() => a,
            _ => return Ok(None),
        };
        let album_name = match meta.album {
            Some(a) if !a.is_empty() => a,
            _ => return Ok(None),
        };

        // Try to find the album in the library by name
        let albums = self.db.albums().list_by_library(library.id).await?;
        for album in albums {
            // Case-insensitive match
            if album.name.to_lowercase() == album_name.to_lowercase() {
                // Verify artist matches (get artist name)
                if let Some(artist) = self.db.albums().get_artist_by_id(album.artist_id).await? {
                    if artist.name.to_lowercase() == artist_name.to_lowercase() {
                        return Ok(Some((album, artist.name)));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Try to match an audio file to an audiobook by reading metadata or folder structure
    async fn try_match_audiobook_file(
        &self,
        file_path: &str,
        library: &crate::db::libraries::LibraryRecord,
    ) -> Result<Option<(crate::db::AudiobookRecord, String)>> {
        // Try to read ID3 tags for album name (often audiobook title)
        let meta = Self::read_audio_metadata_static(file_path);

        // Get folder name as fallback
        let folder_name = Path::new(file_path)
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Try to match by album tag (audiobook title) or folder name
        let search_name = meta
            .as_ref()
            .and_then(|m| m.album.clone())
            .unwrap_or_else(|| folder_name.to_string());

        if search_name.is_empty() {
            return Ok(None);
        }

        // Try to find the audiobook in the library
        let audiobooks = self.db.audiobooks().list_by_library(library.id).await?;
        for audiobook in audiobooks {
            // Case-insensitive partial match (audiobooks often have varied naming)
            if audiobook
                .title
                .to_lowercase()
                .contains(&search_name.to_lowercase())
                || search_name
                    .to_lowercase()
                    .contains(&audiobook.title.to_lowercase())
            {
                // Get author name
                let author_name = if let Some(author_id) = audiobook.author_id {
                    self.db
                        .audiobooks()
                        .get_author_by_id(author_id)
                        .await?
                        .map(|a| a.name)
                        .unwrap_or_else(|| "Unknown Author".to_string())
                } else {
                    "Unknown Author".to_string()
                };
                return Ok(Some((audiobook, author_name)));
            }
        }

        Ok(None)
    }

    /// Create or update a media file linked to a music album
    async fn create_linked_music_file(
        &self,
        file_path: &str,
        size_bytes: i64,
        library: &crate::db::libraries::LibraryRecord,
        album: &crate::db::AlbumRecord,
    ) -> Result<crate::db::MediaFileRecord> {
        // Use upsert to handle existing records gracefully
        let media_file = self
            .db
            .media_files()
            .upsert(crate::db::CreateMediaFile {
                library_id: library.id,
                path: file_path.to_string(),
                size_bytes,
                container: get_container(file_path),
                video_codec: None,
                audio_codec: None,
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                file_hash: None,
                episode_id: None,
                movie_id: None,
                relative_path: None,
                original_name: Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                resolution: None,
                is_hdr: None,
                hdr_type: None,
            })
            .await?;

        // Link to album (will update if already linked)
        self.db
            .media_files()
            .link_to_album(media_file.id, album.id)
            .await?;

        // Try to match to a specific track within the album
        let tracks = self.db.tracks().list_by_album(album.id).await?;
        if let Some(track_id) = match_track_to_file(
            Path::new(file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(file_path),
            &tracks,
        ) {
            // Link to the matched track
            self.db
                .media_files()
                .link_to_track(media_file.id, track_id)
                .await?;
            self.db
                .tracks()
                .link_media_file(track_id, media_file.id)
                .await?;
            self.db.tracks().update_status(track_id, "downloaded").await?;
            
            if let Some(track) = tracks.iter().find(|t| t.id == track_id) {
                info!(
                    file_path = %file_path,
                    track_title = %track.title,
                    track_number = track.track_number,
                    "Matched and linked file to track"
                );
            }
        }

        // Update album has_files
        self.db.albums().update_has_files(album.id, true).await?;

        // Queue for analysis
        self.queue_for_analysis(&media_file).await;

        debug!(
            file_id = %media_file.id,
            album_id = %album.id,
            path = %file_path,
            "Upserted linked music file"
        );

        Ok(media_file)
    }

    /// Create or update a media file linked to an audiobook
    async fn create_linked_audiobook_file(
        &self,
        file_path: &str,
        size_bytes: i64,
        library: &crate::db::libraries::LibraryRecord,
        audiobook: &crate::db::AudiobookRecord,
    ) -> Result<crate::db::MediaFileRecord> {
        // Use upsert to handle existing records gracefully
        let media_file = self
            .db
            .media_files()
            .upsert(crate::db::CreateMediaFile {
                library_id: library.id,
                path: file_path.to_string(),
                size_bytes,
                container: get_container(file_path),
                video_codec: None,
                audio_codec: None,
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                file_hash: None,
                episode_id: None,
                movie_id: None,
                relative_path: None,
                original_name: Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                resolution: None,
                is_hdr: None,
                hdr_type: None,
            })
            .await?;

        // Link to audiobook (will update if already linked)
        self.db
            .media_files()
            .link_to_audiobook(media_file.id, audiobook.id)
            .await?;

        // Update audiobook has_files
        self.db
            .audiobooks()
            .update_has_files(audiobook.id, true)
            .await?;

        // Queue for analysis
        self.queue_for_analysis(&media_file).await;

        debug!(
            file_id = %media_file.id,
            audiobook_id = %audiobook.id,
            path = %file_path,
            "Upserted linked audiobook file"
        );

        Ok(media_file)
    }

    /// Read audio metadata from a file (static method for use in matching)
    fn read_audio_metadata_static(path: &str) -> Option<AudioMetadata> {
        use lofty::prelude::*;
        use lofty::probe::Probe;

        let tagged_file = Probe::open(path).ok()?.read().ok()?;
        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag())?;

        Some(AudioMetadata {
            artist: tag.artist().map(|s| s.to_string()),
            album: tag.album().map(|s| s.to_string()),
            title: tag.title().map(|s| s.to_string()),
            track_number: tag.track(),
            disc_number: tag.disk(),
            year: tag.year(),
            genre: tag.genre().map(|s| s.to_string()),
        })
    }
}

/// Audio metadata from ID3/FLAC/etc tags
#[derive(Debug, Clone)]
struct AudioMetadata {
    artist: Option<String>,
    album: Option<String>,
    title: Option<String>,
    track_number: Option<u32>,
    disc_number: Option<u32>,
    year: Option<u32>,
    genre: Option<String>,
}

/// Match a music filename to a track in the album track list
///
/// Tries multiple matching strategies:
/// 1. Extract track number from filename (e.g., "01 - Song Title.flac", "Track01.mp3")
/// 2. Match by title similarity
fn match_track_to_file(filename: &str, tracks: &[crate::db::TrackRecord]) -> Option<uuid::Uuid> {
    if tracks.is_empty() {
        return None;
    }

    // Clean up filename - remove extension
    let name_without_ext = std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);

    // Try to extract track number from filename
    // Common patterns:
    // "01 - Song Title"
    // "01. Song Title"
    // "01_Song Title"
    // "Track 01 - Song Title"
    // "(01) Song Title"
    // "1-01 Song Title" (disc-track)

    let parsed = parse_track_number_from_filename(name_without_ext);

    if let Some((disc_num, track_num)) = parsed {
        // Try exact disc+track match first
        if let Some(track) = tracks
            .iter()
            .find(|t| t.disc_number == disc_num && t.track_number == track_num)
        {
            return Some(track.id);
        }

        // If no disc match, just try track number on disc 1
        if disc_num == 1 {
            if let Some(track) = tracks.iter().find(|t| t.track_number == track_num) {
                return Some(track.id);
            }
        }
    }

    // Fallback: Try matching by title similarity
    let clean_name = clean_title_for_matching(name_without_ext);

    let best_match = tracks
        .iter()
        .filter_map(|t| {
            let clean_track = clean_title_for_matching(&t.title);
            let score = title_similarity(&clean_name, &clean_track);
            if score >= 0.8 { Some((t, score)) } else { None }
        })
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    best_match.map(|(t, _)| t.id)
}

/// Parse track number (and optionally disc number) from filename
fn parse_track_number_from_filename(filename: &str) -> Option<(i32, i32)> {
    use regex::Regex;

    // Match patterns like "1-05" or "2-01" (disc-track)
    if let Ok(disc_track_re) = Regex::new(r"^(\d{1,2})[.-](\d{1,2})") {
        if let Some(caps) = disc_track_re.captures(filename) {
            if let (Some(disc_str), Some(track_str)) = (caps.get(1), caps.get(2)) {
                if let (Ok(disc), Ok(track)) = (
                    disc_str.as_str().parse::<i32>(),
                    track_str.as_str().parse::<i32>(),
                ) {
                    // Validate reasonable ranges
                    if disc >= 1 && disc <= 20 && track >= 1 && track <= 99 {
                        return Some((disc, track));
                    }
                }
            }
        }
    }

    // Match patterns like "01 -", "01.", "01_", "(01)", "[01]", "Track 01"
    if let Ok(track_re) = Regex::new(r"(?i)(?:^|track\s*)(\d{1,3})(?:\s*[-._)\]]|\s)") {
        if let Some(caps) = track_re.captures(filename) {
            if let Some(track_str) = caps.get(1) {
                if let Ok(track) = track_str.as_str().parse::<i32>() {
                    if track >= 1 && track <= 999 {
                        return Some((1, track));
                    }
                }
            }
        }
    }

    // Last resort: find any leading number
    let leading_num: String = filename
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if !leading_num.is_empty() {
        if let Ok(track) = leading_num.parse::<i32>() {
            if track >= 1 && track <= 999 {
                return Some((1, track));
            }
        }
    }

    None
}

/// Clean a title for fuzzy matching
fn clean_title_for_matching(title: &str) -> String {
    // Convert to lowercase and remove punctuation
    let mut result = String::new();
    for c in title.chars() {
        if c.is_alphanumeric() {
            result.push(c.to_ascii_lowercase());
        } else if c.is_whitespace() {
            result.push(' ');
        }
    }

    // Normalize whitespace
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Calculate title similarity using rapidfuzz
/// 
/// Uses the main show_name_similarity implementation which combines:
/// - Normalized Levenshtein distance
/// - Partial ratio (substring matching)  
/// - Token sort ratio (word order invariant)
fn title_similarity(a: &str, b: &str) -> f64 {
    crate::services::filename_parser::show_name_similarity(a, b)
}
