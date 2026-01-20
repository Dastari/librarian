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

        // When force=true, delete existing media_file records for files in the downloads folder
        // This allows re-organization of files that were previously processed but not organized correctly
        if force {
            let downloads_path = std::env::var("DOWNLOADS_PATH").unwrap_or_else(|_| "/data/downloads".to_string());
            let mut deleted_count = 0;
            
            for file_info in &files {
                if file_info.path.starts_with(&downloads_path) {
                    if let Some(existing) = self.db.media_files().get_by_path(&file_info.path).await? {
                        info!(
                            path = %file_info.path,
                            media_file_id = %existing.id,
                            "Force reprocess: deleting existing media_file record in downloads folder"
                        );
                        // Reset has_file flags on linked items
                        // (Episodes derive status from media_files, so no update needed)
                        if let Some(movie_id) = existing.movie_id {
                            self.db.movies().update_has_file(movie_id, false).await.ok();
                        }
                        self.db.media_files().delete(existing.id).await.ok();
                        deleted_count += 1;
                    }
                }
            }
            
            // For albums/audiobooks, reset has_files flag on the linked item if we deleted any files
            if deleted_count > 0 {
                if let Some(album_id) = torrent.album_id {
                    self.db.albums().update_has_files(album_id, false).await.ok();
                }
                if let Some(audiobook_id) = torrent.audiobook_id {
                    self.db.audiobooks().update_has_files(audiobook_id, false).await.ok();
                }
                info!(
                    info_hash = %info_hash,
                    deleted_count = deleted_count,
                    "Force reprocess: cleared existing media_file records"
                );
            }
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

            let parsed = filename_parser::parse_movie(
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
                    path: file_path.clone(),
                    size_bytes: file_info.size as i64,
                    container: get_container(file_path),
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
                    original_name: Path::new(file_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string()),
                    resolution: parsed.resolution.clone(),
                    is_hdr: parsed.hdr.is_some().then_some(true),
                    hdr_type: parsed.hdr.clone(),
                })
                .await?;

            // Queue for FFmpeg analysis to get real metadata
            self.queue_for_analysis(&media_file).await;

            // Organize the file if library has organize_files enabled
            if library.organize_files {
                info!(
                    movie = %movie.title,
                    library_path = %library.path,
                    source_path = %file_path,
                    post_download_action = %library.post_download_action,
                    "Organizing movie file"
                );
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
                            original_path = %org_result.original_path,
                            new_path = %org_result.new_path,
                            "Movie file organized successfully"
                        );
                        result.organized = true;
                    }
                    Ok(org_result) => {
                        warn!(
                            movie = %movie.title,
                            original_path = %org_result.original_path,
                            new_path = %org_result.new_path,
                            error = ?org_result.error,
                            "Failed to organize movie file"
                        );
                    }
                    Err(e) => {
                        error!(movie = %movie.title, error = %e, "Error organizing movie file");
                    }
                }
            } else {
                debug!(
                    movie = %movie.title,
                    library_name = %library.name,
                    "Library has organize_files disabled, file will remain in downloads"
                );
            }

            result.files_processed += 1;
        }

        // Update movie status
        if result.files_processed > 0 {
            self.db.movies().update_has_file(movie.id, true).await?;
            
            // Calculate total size from all files
            let movie_files = self.db.media_files().list_by_movie(movie.id).await?;
            let total_size: i64 = movie_files.iter().map(|f| f.size_bytes).sum();
            self.db.movies().update_file_status(movie.id, true, Some(total_size)).await?;
            
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

        // Get album and artist info for organization
        let album = if let Some(album_id) = torrent.album_id {
            self.db.albums().get_by_id(album_id).await?
        } else {
            None
        };

        let artist_name = if let Some(ref album) = album {
            // Get artist name from album's artist_id
            self.db
                .albums()
                .get_artist_by_id(album.artist_id)
                .await?
                .map(|a| a.name)
                .unwrap_or_else(|| "Unknown Artist".to_string())
        } else {
            "Unknown Artist".to_string()
        };

        // Get tracks for the album if we have an album linked
        let album_tracks = if let Some(ref album) = album {
            self.db.tracks().list_by_album(album.id).await.unwrap_or_default()
        } else {
            Vec::new()
        };

        let mut tracks_matched = 0;

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

            // Link to album
            if let Some(album_id) = torrent.album_id {
                self.db
                    .media_files()
                    .link_to_album(media_file.id, album_id)
                    .await?;
            }

            // Try to match this file to a track
            // If the torrent has a specific track_id, use that
            // Otherwise, try to match by filename analysis
            let matched_track_id = if let Some(track_id) = torrent.track_id {
                Some(track_id)
            } else if !album_tracks.is_empty() {
                // Try to match by track number in filename
                let filename = Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                match_track_to_file(filename, &album_tracks)
            } else {
                None
            };

            // Link to track if we found a match
            if let Some(track_id) = matched_track_id {
                if let Err(e) = self.db.tracks().link_media_file(track_id, media_file.id).await {
                    warn!(error = %e, track_id = %track_id, "Failed to link media file to track");
                } else {
                    tracks_matched += 1;
                    debug!(track_id = %track_id, file = %file_path, "Linked file to track");
                }
            }

            // Queue for FFmpeg analysis to get real audio metadata (bitrate, codec, duration, etc.)
            self.queue_for_analysis(&media_file).await;

            // Organize the file if library has organize_files enabled and we have album info
            if library.organize_files {
                if let Some(ref album) = album {
                    match self
                        .organizer
                        .organize_music_file(
                            &media_file,
                            &artist_name,
                            album,
                            &library.path,
                            library.naming_pattern.as_deref(),
                            &library.post_download_action,
                            false,
                        )
                        .await
                    {
                        Ok(org_result) if org_result.success => {
                            info!(
                                album = %album.name,
                                artist = %artist_name,
                                new_path = %org_result.new_path,
                                "Music file organized successfully"
                            );
                            result.organized = true;
                        }
                        Ok(org_result) => {
                            warn!(
                                album = %album.name,
                                error = ?org_result.error,
                                "Failed to organize music file"
                            );
                        }
                        Err(e) => {
                            error!(album = %album.name, error = %e, "Error organizing music file");
                        }
                    }
                }
            }

            result.files_processed += 1;
            info!(
                file_id = %media_file.id,
                path = %file_path,
                "Created media file for music"
            );
        }

        if result.files_processed > 0 {
            // Update album has_files status if we have an album
            if let Some(ref album) = album {
                self.db.albums().update_has_files(album.id, true).await?;
            }
            
            result.success = true;
            result.messages.push(format!(
                "Processed {} music file(s), matched {} track(s)",
                result.files_processed,
                tracks_matched
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

        // Get audiobook and author info for organization
        let audiobook = self.db.audiobooks().get_by_id(audiobook_id).await?;
        
        let author_name = if let Some(ref ab) = audiobook {
            self.db
                .audiobooks()
                .get_author_by_id(ab.author_id)
                .await?
                .map(|a| a.name)
                .unwrap_or_else(|| "Unknown Author".to_string())
        } else {
            "Unknown Author".to_string()
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

            // Organize the file if library has organize_files enabled and we have audiobook info
            if library.organize_files {
                if let Some(ref ab) = audiobook {
                    match self
                        .organizer
                        .organize_audiobook_file(
                            &media_file,
                            &author_name,
                            ab,
                            &library.path,
                            library.naming_pattern.as_deref(),
                            &library.post_download_action,
                            false,
                        )
                        .await
                    {
                        Ok(org_result) if org_result.success => {
                            info!(
                                audiobook = %ab.title,
                                author = %author_name,
                                new_path = %org_result.new_path,
                                "Audiobook file organized successfully"
                            );
                            result.organized = true;
                        }
                        Ok(org_result) => {
                            warn!(
                                audiobook = %ab.title,
                                error = ?org_result.error,
                                "Failed to organize audiobook file"
                            );
                        }
                        Err(e) => {
                            error!(audiobook = %ab.title, error = %e, "Error organizing audiobook file");
                        }
                    }
                }
            }

            result.files_processed += 1;
            info!(
                file_id = %media_file.id,
                audiobook_id = %audiobook_id,
                "Created media file for audiobook"
            );
        }

        if result.files_processed > 0 {
            // Update audiobook has_files status
            if let Some(ref ab) = audiobook {
                self.db.audiobooks().update_has_files(ab.id, true).await?;
            }
            
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
                "music" => {
                    // For music libraries, try to auto-add album from the torrent name
                    if library.auto_add_discovered {
                        if let Some(album) = self.try_auto_add_album(&torrent.name, &library).await? {
                            // Link torrent to the album
                            self.db.torrents().link_to_album(&torrent.info_hash, album.id).await?;
                            
                            // Now process as linked music - this will organize all files
                            let mut updated_torrent = torrent.clone();
                            updated_torrent.album_id = Some(album.id);
                            
                            let music_result = self.process_linked_music(&updated_torrent, files).await?;
                            result.matched = music_result.matched;
                            result.organized = music_result.organized;
                            result.files_processed += music_result.files_processed;
                            result.files_failed += music_result.files_failed;
                            result.messages.extend(music_result.messages);
                            result.messages.push(format!(
                                "Auto-added album: {} by {}",
                                album.name,
                                self.db.albums().get_artist_by_id(album.artist_id).await?
                                    .map(|a| a.name).unwrap_or_else(|| "Unknown Artist".to_string())
                            ));
                            // Break out of file loop since we processed all files
                            break;
                        }
                    }
                    // Fallback: create unlinked audio file
                    self.create_unlinked_audio_file(file_path, file_info.size as i64, &library)
                        .await?;
                    result.files_processed += 1;
                    result.messages.push(format!(
                        "No album match found for: {}. Enable 'Auto-add discovered' or add the album first.",
                        Path::new(file_path).file_name().and_then(|n| n.to_str()).unwrap_or(file_path)
                    ));
                }
                "audiobooks" => {
                    // For audiobook libraries, try to auto-add from the torrent name
                    if library.auto_add_discovered {
                        if let Some(audiobook) = self.try_auto_add_audiobook(&torrent.name, &library).await? {
                            // Link torrent to the audiobook
                            self.db.torrents().link_to_audiobook(&torrent.info_hash, audiobook.id).await?;
                            
                            // Now process as linked audiobook
                            let mut updated_torrent = torrent.clone();
                            updated_torrent.audiobook_id = Some(audiobook.id);
                            
                            let audiobook_result = self.process_linked_audiobook(&updated_torrent, files).await?;
                            result.matched = audiobook_result.matched;
                            result.organized = audiobook_result.organized;
                            result.files_processed += audiobook_result.files_processed;
                            result.files_failed += audiobook_result.files_failed;
                            result.messages.extend(audiobook_result.messages);
                            result.messages.push(format!(
                                "Auto-added audiobook: {}",
                                audiobook.title
                            ));
                            break;
                        }
                    }
                    // Fallback: create unlinked audio file
                    self.create_unlinked_audio_file(file_path, file_info.size as i64, &library)
                        .await?;
                    result.files_processed += 1;
                    result.messages.push(format!(
                        "No audiobook match found for: {}. Enable 'Auto-add discovered' or add the audiobook first.",
                        Path::new(file_path).file_name().and_then(|n| n.to_str()).unwrap_or(file_path)
                    ));
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
                // Try to match against music libraries first
                for library in &music_libraries {
                    if let Ok(Some((album, artist_name))) = self.try_match_music_file(file_path, library).await {
                        debug!(
                            path = %file_path,
                            album = %album.name,
                            artist = %artist_name,
                            library = %library.name,
                            "Matched audio file to album"
                        );
                        if let Err(e) = self.create_linked_music_file(file_path, file_info.size as i64, library, &album).await {
                            result.messages.push(format!("Error creating music file: {}", e));
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
                        if let Ok(Some((audiobook, author_name))) = self.try_match_audiobook_file(file_path, library).await {
                            debug!(
                                path = %file_path,
                                audiobook = %audiobook.title,
                                author = %author_name,
                                library = %library.name,
                                "Matched audio file to audiobook"
                            );
                            if let Err(e) = self.create_linked_audiobook_file(file_path, file_info.size as i64, library, &audiobook).await {
                                result.messages.push(format!("Error creating audiobook file: {}", e));
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
        let cleaned = torrent_name
            .replace('_', " ")
            .replace('.', " ");
        
        // Try to extract artist and album from the name
        let (artist_name, album_name) = if let Some(pos) = cleaned.find(" - ") {
            let artist = cleaned[..pos].trim().to_string();
            let rest = &cleaned[pos + 3..];
            // Remove year and format info from album name
            let album = rest.split(|c: char| c == '(' || c == '[' || c.is_ascii_digit())
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
        let cleaned = torrent_name
            .replace('_', " ")
            .replace('.', " ");
        
        // Try to extract author and title
        let (author_name, title) = if cleaned.to_lowercase().contains(" by ") {
            let lower = cleaned.to_lowercase();
            if let Some(pos) = lower.find(" by ") {
                let title = cleaned[..pos].trim().to_string();
                let author = cleaned[pos + 4..].split(|c: char| c == '(' || c == '[')
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
                        "Movie file organized successfully"
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
            if audiobook.title.to_lowercase().contains(&search_name.to_lowercase())
                || search_name.to_lowercase().contains(&audiobook.title.to_lowercase())
            {
                // Get author name
                if let Some(author) = self.db.audiobooks().get_author_by_id(audiobook.author_id).await? {
                    return Ok(Some((audiobook, author.name)));
                }
            }
        }

        Ok(None)
    }

    /// Create a media file linked to a music album
    async fn create_linked_music_file(
        &self,
        file_path: &str,
        size_bytes: i64,
        library: &crate::db::libraries::LibraryRecord,
        album: &crate::db::AlbumRecord,
    ) -> Result<crate::db::MediaFileRecord> {
        let media_file = self.db.media_files().create(crate::db::CreateMediaFile {
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
        }).await?;

        // Link to album
        self.db.media_files().link_to_album(media_file.id, album.id).await?;

        // Update album has_files
        self.db.albums().update_has_files(album.id, true).await?;

        // Queue for analysis
        self.queue_for_analysis(&media_file).await;

        debug!(
            file_id = %media_file.id,
            album_id = %album.id,
            path = %file_path,
            "Created linked music file"
        );

        Ok(media_file)
    }

    /// Create a media file linked to an audiobook
    async fn create_linked_audiobook_file(
        &self,
        file_path: &str,
        size_bytes: i64,
        library: &crate::db::libraries::LibraryRecord,
        audiobook: &crate::db::AudiobookRecord,
    ) -> Result<crate::db::MediaFileRecord> {
        let media_file = self.db.media_files().create(crate::db::CreateMediaFile {
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
        }).await?;

        // Link to audiobook
        self.db.media_files().link_to_audiobook(media_file.id, audiobook.id).await?;

        // Update audiobook has_files
        self.db.audiobooks().update_has_files(audiobook.id, true).await?;

        // Queue for analysis
        self.queue_for_analysis(&media_file).await;

        debug!(
            file_id = %media_file.id,
            audiobook_id = %audiobook.id,
            path = %file_path,
            "Created linked audiobook file"
        );

        Ok(media_file)
    }

    /// Read audio metadata from a file (static method for use in matching)
    fn read_audio_metadata_static(path: &str) -> Option<AudioMetadata> {
        use lofty::prelude::*;
        use lofty::probe::Probe;

        let tagged_file = Probe::open(path).ok()?.read().ok()?;
        let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag())?;

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
fn match_track_to_file(
    filename: &str,
    tracks: &[crate::db::TrackRecord],
) -> Option<uuid::Uuid> {
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
        if let Some(track) = tracks.iter().find(|t| {
            t.disc_number == disc_num && t.track_number == track_num
        }) {
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
            if score >= 0.8 {
                Some((t, score))
            } else {
                None
            }
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
    let leading_num: String = filename.chars().take_while(|c| c.is_ascii_digit()).collect();
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

/// Simple string similarity using longest common subsequence ratio
fn title_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    // LCS DP table
    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 1..=m {
        for j in 1..=n {
            if a_chars[i - 1] == b_chars[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    let lcs_len = dp[m][n] as f64;
    let max_len = m.max(n) as f64;

    lcs_len / max_len
}
