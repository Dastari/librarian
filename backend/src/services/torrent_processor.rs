//! Unified torrent processing service
//!
//! This module provides a single point of logic for processing completed torrents:
//! - Matching torrent files to library items (TV episodes, movies, music, audiobooks)
//! - Creating media file records
//! - Organizing files into library folder structure
//! - Updating item status
//!
//! This consolidates logic that was previously duplicated in:
//! - `jobs/download_monitor.rs`
//! - `services/organizer.rs`
//!
//! All torrent processing should go through this service to ensure consistent behavior.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::{Database, EpisodeRecord, MediaFileRecord, MovieRecord, TorrentRecord, TvShowRecord};
use crate::services::file_utils::{get_container, is_audio_file, is_video_file};
use crate::services::filename_parser;
use crate::services::organizer::OrganizerService;
use crate::services::queues::{MediaAnalysisJob, MediaAnalysisQueue};
use crate::services::TorrentService;

/// Result of processing a single torrent
#[derive(Debug, Clone)]
pub struct ProcessTorrentResult {
    pub success: bool,
    pub matched: bool,
    pub organized: bool,
    pub files_processed: i32,
    pub files_failed: i32,
    pub messages: Vec<String>,
}

impl Default for ProcessTorrentResult {
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

/// Unified torrent processor service
///
/// Handles all torrent processing logic including:
/// - Matching files to library items
/// - Creating media file records  
/// - Organizing files
/// - Updating status
/// - Queueing files for FFmpeg analysis to get real metadata
/// - Auto-adding movies from TMDB when `auto_add_discovered` is enabled
pub struct TorrentProcessor {
    db: Database,
    organizer: OrganizerService,
    /// Optional media analysis queue for FFmpeg metadata extraction
    analysis_queue: Option<Arc<MediaAnalysisQueue>>,
    /// Optional metadata service for auto-adding movies from TMDB
    metadata_service: Option<Arc<crate::services::MetadataService>>,
}

impl TorrentProcessor {
    pub fn new(db: Database) -> Self {
        let organizer = OrganizerService::new(db.clone());
        Self {
            db,
            organizer,
            analysis_queue: None,
            metadata_service: None,
        }
    }

    /// Create with a media analysis queue for FFmpeg metadata extraction
    pub fn with_analysis_queue(db: Database, queue: Arc<MediaAnalysisQueue>) -> Self {
        let organizer = OrganizerService::new(db.clone());
        Self {
            db,
            organizer,
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
        Self {
            db,
            organizer,
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

    /// Process a completed torrent
    ///
    /// This is the main entry point for processing. It handles:
    /// 1. Getting torrent files
    /// 2. Matching to library items based on torrent linkage or filename parsing
    /// 3. Creating media file records
    /// 4. Organizing files if enabled
    /// 5. Updating item status
    ///
    /// The `force` parameter allows reprocessing even if already marked as completed.
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
                result.messages.push(format!("Torrent not found: {}", info_hash));
                return Ok(result);
            }
        };

        // Check if already processed (unless forcing)
        if !force && torrent.post_process_status.as_deref() == Some("completed") {
            result.messages.push("Torrent already processed".to_string());
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
            result.messages.push("No files found in torrent".to_string());
            self.db
                .torrents()
                .update_post_process_status(info_hash, "completed")
                .await
                .ok();
            result.success = true;
            return Ok(result);
        }

        // Route processing based on what's linked
        let process_result = if torrent.episode_id.is_some() {
            self.process_linked_episode(&torrent, &files).await
        } else if torrent.movie_id.is_some() {
            self.process_linked_movie(&torrent, &files).await
        } else if torrent.album_id.is_some() || torrent.track_id.is_some() {
            self.process_linked_music(&torrent, &files).await
        } else if torrent.audiobook_id.is_some() {
            self.process_linked_audiobook(&torrent, &files).await
        } else if torrent.library_id.is_some() {
            // Has library but no specific item - try to auto-match
            self.process_with_library(&torrent, &files).await
        } else {
            // No library linked - try to match against all user libraries
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

    /// Process a torrent that's already linked to an episode
    async fn process_linked_episode(
        &self,
        torrent: &TorrentRecord,
        files: &[crate::services::torrent::TorrentFile],
    ) -> Result<ProcessTorrentResult> {
        let mut result = ProcessTorrentResult::default();
        result.matched = true;

        let episode_id = torrent.episode_id.unwrap();
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

        for file_info in files {
            if !is_video_file(&file_info.path) {
                continue;
            }

            let file_result = self
                .process_video_file(
                    &file_info.path,
                    file_info.size as i64,
                    &library,
                    Some(&show),
                    Some(&episode),
                )
                .await;

            match file_result {
                Ok(organized) => {
                    result.files_processed += 1;
                    if organized {
                        result.organized = true;
                    }
                }
                Err(e) => {
                    result.files_failed += 1;
                    result.messages.push(format!("Failed to process file: {}", e));
                }
            }
        }

        // Update episode status
        if result.files_processed > 0 {
            self.db
                .episodes()
                .update_status(episode.id, "downloaded")
                .await?;
            self.db.tv_shows().update_stats(show.id).await?;
            result.success = true;
            result
                .messages
                .push(format!("Processed {} - S{:02}E{:02}", show.name, episode.season, episode.episode));
        }

        Ok(result)
    }

    /// Process a torrent linked to a movie
    async fn process_linked_movie(
        &self,
        torrent: &TorrentRecord,
        files: &[crate::services::torrent::TorrentFile],
    ) -> Result<ProcessTorrentResult> {
        let mut result = ProcessTorrentResult::default();
        result.matched = true;

        let movie_id = torrent.movie_id.unwrap();
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

        for file_info in files {
            if !is_video_file(&file_info.path) {
                continue;
            }

            // Create media file record
            let file_path = &file_info.path;
            if self.db.media_files().exists_by_path(file_path).await? {
                debug!(path = %file_path, "Media file already exists");
                result.files_processed += 1;
                continue;
            }

            let media_file = self
                .db
                .media_files()
                .create(crate::db::CreateMediaFile {
                    library_id: library.id,
                    path: file_path.clone(),
                    size_bytes: file_info.size as i64,
                    container: get_container(file_path),
                    video_codec: None,
                    audio_codec: None,
                    width: None,
                    height: None,
                    duration: None,
                    bitrate: None,
                    file_hash: None,
                    episode_id: None,
                    movie_id: Some(movie_id),
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

            // Queue for FFmpeg analysis to get real metadata
            self.queue_for_analysis(&media_file).await;

            result.files_processed += 1;
        }

        // Update movie status
        if result.files_processed > 0 {
            self.db.movies().update_has_file(movie.id, true).await?;
            result.success = true;
            result.messages.push(format!("Processed movie: {}", movie.title));
        }

        Ok(result)
    }

    /// Process a torrent linked to music (album or track)
    async fn process_linked_music(
        &self,
        torrent: &TorrentRecord,
        files: &[crate::services::torrent::TorrentFile],
    ) -> Result<ProcessTorrentResult> {
        let mut result = ProcessTorrentResult::default();
        result.matched = true;

        let library = if let Some(lib_id) = torrent.library_id {
            self.db.libraries().get_by_id(lib_id).await?
        } else {
            None
        };

        let Some(library) = library else {
            result.messages.push("No library linked to music torrent".to_string());
            return Ok(result);
        };

        for file_info in files {
            if !is_audio_file(&file_info.path) {
                continue;
            }

            let file_path = &file_info.path;
            if self.db.media_files().exists_by_path(file_path).await? {
                result.files_processed += 1;
                continue;
            }

            let media_file = self
                .db
                .media_files()
                .create(crate::db::CreateMediaFile {
                    library_id: library.id,
                    path: file_path.clone(),
                    size_bytes: file_info.size as i64,
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

            // Link to album or track
            if let Some(album_id) = torrent.album_id {
                self.db
                    .media_files()
                    .link_to_album(media_file.id, album_id)
                    .await?;
            }
            if let Some(track_id) = torrent.track_id {
                self.db
                    .media_files()
                    .link_to_track(media_file.id, track_id)
                    .await?;
            }

            // Queue for FFmpeg analysis to get real audio metadata (bitrate, codec, duration, etc.)
            self.queue_for_analysis(&media_file).await;

            result.files_processed += 1;
            info!(
                file_id = %media_file.id,
                path = %file_path,
                "Created media file for music"
            );
        }

        if result.files_processed > 0 {
            result.success = true;
            result.messages.push(format!(
                "Processed {} music file(s)",
                result.files_processed
            ));
        }

        Ok(result)
    }

    /// Process a torrent linked to an audiobook
    async fn process_linked_audiobook(
        &self,
        torrent: &TorrentRecord,
        files: &[crate::services::torrent::TorrentFile],
    ) -> Result<ProcessTorrentResult> {
        let mut result = ProcessTorrentResult::default();
        result.matched = true;

        let audiobook_id = torrent.audiobook_id.unwrap();
        let library = if let Some(lib_id) = torrent.library_id {
            self.db.libraries().get_by_id(lib_id).await?
        } else {
            None
        };

        let Some(library) = library else {
            result.messages.push("No library linked to audiobook torrent".to_string());
            return Ok(result);
        };

        for file_info in files {
            if !is_audio_file(&file_info.path) {
                continue;
            }

            let file_path = &file_info.path;
            if self.db.media_files().exists_by_path(file_path).await? {
                result.files_processed += 1;
                continue;
            }

            let media_file = self
                .db
                .media_files()
                .create(crate::db::CreateMediaFile {
                    library_id: library.id,
                    path: file_path.clone(),
                    size_bytes: file_info.size as i64,
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

            // Link to audiobook
            self.db
                .media_files()
                .link_to_audiobook(media_file.id, audiobook_id)
                .await?;

            // Queue for FFmpeg analysis to get real audio metadata
            self.queue_for_analysis(&media_file).await;

            result.files_processed += 1;
            info!(
                file_id = %media_file.id,
                audiobook_id = %audiobook_id,
                "Created media file for audiobook"
            );
        }

        if result.files_processed > 0 {
            result.success = true;
            result.messages.push(format!(
                "Processed {} audiobook file(s)",
                result.files_processed
            ));
        }

        Ok(result)
    }

    /// Process a torrent with a library but no specific item linked
    /// Attempts to auto-match based on filename parsing
    async fn process_with_library(
        &self,
        torrent: &TorrentRecord,
        files: &[crate::services::torrent::TorrentFile],
    ) -> Result<ProcessTorrentResult> {
        let mut result = ProcessTorrentResult::default();

        let library_id = torrent.library_id.unwrap();
        let library = self
            .db
            .libraries()
            .get_by_id(library_id)
            .await?
            .context("Library not found")?;

        // Determine which files to process based on library type
        let is_audio_library = matches!(library.library_type.as_str(), "music" | "audiobooks");

        for file_info in files {
            let file_path = &file_info.path;

            // Filter files based on library type
            let should_process = if is_audio_library {
                is_audio_file(file_path)
            } else {
                is_video_file(file_path)
            };

            if !should_process {
                continue;
            }

            // Skip if already exists
            if self.db.media_files().exists_by_path(file_path).await? {
                debug!(path = %file_path, "Media file already exists, checking if needs matching");
                // Check if existing file needs matching
                if let Some(existing) = self.db.media_files().get_by_path(file_path).await? {
                    if existing.episode_id.is_none() && library.library_type == "tv" {
                        // Try to match existing unlinked file for TV
                        if let Some((show, episode)) = self.try_match_tv_file(file_path, &library).await? {
                            self.db.media_files().link_to_episode(existing.id, episode.id).await?;
                            self.db.episodes().update_status(episode.id, "downloaded").await?;
                            self.db.tv_shows().update_stats(show.id).await?;
                            self.db.torrents().link_to_episode(&torrent.info_hash, episode.id).await?;
                            result.matched = true;
                            result.files_processed += 1;
                            result.messages.push(format!(
                                "Matched existing file to {} S{:02}E{:02}",
                                show.name, episode.season, episode.episode
                            ));
                            
                            // Try to organize if enabled
                            if let Ok(organized) = self.try_organize_file(
                                &existing, &library, &show, &episode
                            ).await {
                                if organized {
                                    result.organized = true;
                                }
                            }
                        } else {
                            result.files_processed += 1;
                            result.messages.push(format!(
                                "File already tracked but unmatched: {}",
                                Path::new(file_path).file_name().and_then(|n| n.to_str()).unwrap_or(file_path)
                            ));
                        }
                    } else if existing.movie_id.is_none() && library.library_type == "movies" {
                        // Try to match existing unlinked file for movies
                        if let Some(movie) = self.try_match_movie_file(file_path, &library).await? {
                            self.db.media_files().link_to_movie(existing.id, movie.id).await?;
                            self.db.movies().update_has_file(movie.id, true).await?;
                            self.db.torrents().link_to_movie(&torrent.info_hash, movie.id).await?;
                            result.matched = true;
                            result.files_processed += 1;
                            result.messages.push(format!(
                                "Matched existing file to {} ({})",
                                movie.title, movie.year.map(|y| y.to_string()).unwrap_or_default()
                            ));
                        } else {
                            result.files_processed += 1;
                            result.messages.push(format!(
                                "File already tracked but no matching movie in library. Add the movie first, then retry: {}",
                                Path::new(file_path).file_name().and_then(|n| n.to_str()).unwrap_or(file_path)
                            ));
                        }
                    } else {
                        // Already matched
                        result.files_processed += 1;
                        result.messages.push(format!(
                            "File already processed: {}",
                            Path::new(file_path).file_name().and_then(|n| n.to_str()).unwrap_or(file_path)
                        ));
                    }
                }
                continue;
            }

            // Try to match based on library type
            match library.library_type.as_str() {
                "tv" => {
                    if let Some((show, episode)) = self.try_match_tv_file(file_path, &library).await? {
                        let file_result = self
                            .process_video_file(
                                file_path,
                                file_info.size as i64,
                                &library,
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
                                // Link torrent to episode
                                self.db
                                    .torrents()
                                    .link_to_episode(&torrent.info_hash, episode.id)
                                    .await?;
                                result.messages.push(format!(
                                    "Matched {} S{:02}E{:02}",
                                    show.name, episode.season, episode.episode
                                ));
                            }
                            Err(e) => {
                                result.files_failed += 1;
                                result.messages.push(format!("Failed to process: {}", e));
                            }
                        }
                    } else {
                        // No match found - create unlinked media file
                        self.create_unlinked_media_file(file_path, file_info.size as i64, &library)
                            .await?;
                        result.files_processed += 1;
                        result.messages.push(format!(
                            "No match found for: {}",
                            Path::new(file_path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(file_path)
                        ));
                    }
                }
                "movies" => {
                    // Try to match movie by title/year from filename
                    if let Some(movie) = self.try_match_movie_file(file_path, &library).await? {
                        match self
                            .process_video_file_for_movie(
                                file_path,
                                file_info.size as i64,
                                &library,
                                &movie,
                            )
                            .await
                        {
                            Ok(true) => {
                                result.matched = true;
                                result.files_processed += 1;
                                // Link torrent to movie
                                self.db
                                    .torrents()
                                    .link_to_movie(&torrent.info_hash, movie.id)
                                    .await?;
                                result.messages.push(format!(
                                    "Matched movie: {} ({})",
                                    movie.title,
                                    movie.year.map(|y| y.to_string()).unwrap_or_default()
                                ));
                            }
                            Ok(false) => {
                                result.files_processed += 1;
                                result.messages.push(format!(
                                    "File already exists for: {}",
                                    movie.title
                                ));
                            }
                            Err(e) => {
                                result.files_failed += 1;
                                result.messages.push(format!("Failed to process: {}", e));
                            }
                        }
                    } else {
                        // No match found - create unlinked media file
                        self.create_unlinked_media_file(file_path, file_info.size as i64, &library)
                            .await?;
                        result.files_processed += 1;
                        result.messages.push(format!(
                            "No movie match for: {}",
                            Path::new(file_path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(file_path)
                        ));
                    }
                }
                "music" | "audiobooks" => {
                    // For audio libraries, create unlinked audio file
                    // TODO: Add music/audiobook matching
                    self.create_unlinked_audio_file(file_path, file_info.size as i64, &library)
                        .await?;
                    result.files_processed += 1;
                }
                _ => {
                    // Unknown library type - just create unlinked file
                    if is_video_file(file_path) {
                        self.create_unlinked_media_file(file_path, file_info.size as i64, &library)
                            .await?;
                    } else if is_audio_file(file_path) {
                        self.create_unlinked_audio_file(file_path, file_info.size as i64, &library)
                            .await?;
                    }
                    result.files_processed += 1;
                }
            }
        }

        result.success = result.files_processed > 0;
        Ok(result)
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
            result.messages.push("No libraries found for user".to_string());
            return Ok(result);
        }

        // Separate libraries by type
        let tv_libraries: Vec<_> = libraries.iter().filter(|l| l.library_type == "tv").collect();
        let movie_libraries: Vec<_> = libraries.iter().filter(|l| l.library_type == "movies").collect();
        let music_libraries: Vec<_> = libraries.iter().filter(|l| l.library_type == "music").collect();
        let audiobook_libraries: Vec<_> = libraries.iter().filter(|l| l.library_type == "audiobooks").collect();

        for file_info in files {
            let file_path = &file_info.path;

            // Skip if already exists (add message for tracking)
            if self.db.media_files().exists_by_path(file_path).await? {
                debug!(path = %file_path, "Media file already exists");
                result.files_processed += 1;
                result.messages.push(format!(
                    "File already tracked: {}",
                    Path::new(file_path).file_name().and_then(|n| n.to_str()).unwrap_or(file_path)
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
                                // Link torrent to library and episode
                                self.db
                                    .torrents()
                                    .link_to_library(&torrent.info_hash, lib.id)
                                    .await?;
                                self.db
                                    .torrents()
                                    .link_to_episode(&torrent.info_hash, episode.id)
                                    .await?;
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
                                    // Link torrent to library and movie
                                    self.db
                                        .torrents()
                                        .link_to_library(&torrent.info_hash, lib.id)
                                        .await?;
                                    self.db
                                        .torrents()
                                        .link_to_movie(&torrent.info_hash, movie.id)
                                        .await?;
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
                                    result.messages.push(format!("Failed to process movie: {}", e));
                                }
                            }
                        }
                    }
                }
            }

            // Process audio files (music and audiobooks)
            if !matched && is_audio_file(file_path) {
                // TODO: Add music and audiobook matching logic
                // For now, just log
                if !music_libraries.is_empty() || !audiobook_libraries.is_empty() {
                    debug!(
                        path = %file_path,
                        music_libraries = music_libraries.len(),
                        audiobook_libraries = audiobook_libraries.len(),
                        "Would try matching against audio libraries (not yet implemented)"
                    );
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
            "Matched file to episode"
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
            (None, _) => true, // No year in filename, accept any
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

    /// Process a video file for a movie - create media record and link to movie
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

        // Check if file already exists
        if self.db.media_files().exists_by_path(file_path).await? {
            debug!(path = %file_path, "Media file already exists for movie");
            return Ok(false);
        }

        // Create media file record
        let media_file = self
            .db
            .media_files()
            .create(crate::db::CreateMediaFile {
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

        info!(
            file_id = %media_file.id,
            movie_id = %movie.id,
            path = %file_path,
            "Created media file record for movie"
        );

        // Queue for FFmpeg analysis to get real metadata
        self.queue_for_analysis(&media_file).await;

        // Update movie status
        self.db.movies().update_has_file(movie.id, true).await?;

        // Calculate size for the movie
        let file_size = std::fs::metadata(file_path)
            .map(|m| m.len() as i64)
            .unwrap_or(size_bytes);
        self.db.movies().update_file_status(movie.id, true, Some(file_size)).await?;

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

        // Create media file record
        let media_file = self
            .db
            .media_files()
            .create(crate::db::CreateMediaFile {
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

        info!(
            file_id = %media_file.id,
            path = %file_path,
            episode = ?episode.map(|e| e.id),
            "Created media file record"
        );

        // Queue for FFmpeg analysis to get real metadata (resolution, codec, HDR, etc.)
        self.queue_for_analysis(&media_file).await;

        // Update episode status if linked
        if let Some(ep) = episode {
            self.db.episodes().update_status(ep.id, "downloaded").await?;
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
                            "File organized successfully"
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
                info!(new_path = %result.new_path, "File organized successfully");
                Ok(true)
            }
            Ok(result) => {
                warn!(error = ?result.error, "Failed to organize file");
                Ok(false)
            }
            Err(e) => {
                error!(error = %e, "Error organizing file");
                Ok(false)
            }
        }
    }

    /// Create an unlinked media file record
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

        let media_file = self
            .db
            .media_files()
            .create(crate::db::CreateMediaFile {
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
            "Created unlinked media file"
        );

        // Queue for FFmpeg analysis to get real metadata
        self.queue_for_analysis(&media_file).await;

        Ok(media_file)
    }

    /// Create an unlinked audio file record (for music/audiobooks)
    async fn create_unlinked_audio_file(
        &self,
        file_path: &str,
        size_bytes: i64,
        library: &crate::db::libraries::LibraryRecord,
    ) -> Result<crate::db::MediaFileRecord> {
        let media_file = self
            .db
            .media_files()
            .create(crate::db::CreateMediaFile {
                library_id: library.id,
                path: file_path.to_string(),
                size_bytes,
                container: get_container(file_path),
                video_codec: None,
                audio_codec: None, // TODO: Parse audio codec from filename or file
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
            "Created unlinked audio file"
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
    ) -> Result<i32> {
        let torrents = self.db.torrents().list_unmatched().await?;

        if torrents.is_empty() {
            debug!("No unmatched torrents to process");
            return Ok(0);
        }

        info!(count = torrents.len(), "Processing unmatched torrents");

        let mut processed = 0;
        for torrent in torrents {
            match self
                .process_torrent(torrent_service, &torrent.info_hash, true)
                .await
            {
                Ok(result) => {
                    if result.matched {
                        processed += 1;
                        info!(
                            info_hash = %torrent.info_hash,
                            name = %torrent.name,
                            messages = ?result.messages,
                            "Successfully matched previously unmatched torrent"
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        info_hash = %torrent.info_hash,
                        error = %e,
                        "Failed to process unmatched torrent"
                    );
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
    ) -> Result<i32> {
        let torrents = self.db.torrents().list_pending_processing().await?;

        if torrents.is_empty() {
            debug!("No pending torrents to process");
            return Ok(0);
        }

        info!(count = torrents.len(), "Processing pending torrents");

        let mut processed = 0;
        for torrent in torrents {
            match self
                .process_torrent(torrent_service, &torrent.info_hash, false)
                .await
            {
                Ok(result) => {
                    if result.success {
                        processed += 1;
                    }
                    if !result.messages.is_empty() {
                        info!(
                            info_hash = %torrent.info_hash,
                            name = %torrent.name,
                            messages = ?result.messages,
                            "Processed torrent"
                        );
                    }
                }
                Err(e) => {
                    error!(
                        info_hash = %torrent.info_hash,
                        error = %e,
                        "Failed to process torrent"
                    );
                }
            }
        }

        Ok(processed)
    }
}
