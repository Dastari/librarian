//! File processor service
//!
//! Source-agnostic file processing service.
//! This is THE ONLY place in the codebase where file copying to libraries happens.
//!
//! Used by:
//! - Torrent completion handlers
//! - Usenet completion handlers (future)
//! - Library scanners (for linking existing files)
//! - Manual processing

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::{
    CreateMediaFile, Database, LibraryRecord, MediaFileRecord, PendingFileMatchRecord,
};
use crate::services::file_utils::get_container;
use crate::services::organizer::{
    apply_audiobook_naming_pattern, apply_movie_naming_pattern, apply_music_naming_pattern,
    apply_naming_pattern, OrganizerService,
};
use crate::services::queues::MediaAnalysisQueue;

/// Result of processing files for a source
#[derive(Debug, Default)]
pub struct ProcessResult {
    pub files_processed: usize,
    pub files_failed: usize,
    pub files_skipped: usize,
    pub messages: Vec<String>,
}

/// Source-agnostic file processor service
///
/// This is THE ONLY place file copying should happen in the codebase.
pub struct FileProcessor {
    db: Database,
    analysis_queue: Option<Arc<MediaAnalysisQueue>>,
}

impl FileProcessor {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            analysis_queue: None,
        }
    }

    pub fn with_analysis_queue(db: Database, queue: Arc<MediaAnalysisQueue>) -> Self {
        Self {
            db,
            analysis_queue: Some(queue),
        }
    }

    // =========================================================================
    // Main Public API
    // =========================================================================

    /// Process all pending matches for a source (torrent, usenet, etc.)
    ///
    /// Copies files to library folders, creates media_file records, and links items.
    pub async fn process_source(
        &self,
        source_type: &str,
        source_id: Uuid,
    ) -> Result<ProcessResult> {
        let mut result = ProcessResult::default();

        // Get all uncopied matches for this source
        let matches = self
            .db
            .pending_file_matches()
            .list_uncopied_by_source(source_type, source_id)
            .await?;

        if matches.is_empty() {
            info!(
                source_type = %source_type,
                source_id = %source_id,
                "No pending matches to process"
            );
            return Ok(result);
        }

        info!(
            source_type = %source_type,
            source_id = %source_id,
            match_count = matches.len(),
            "Processing pending file matches"
        );

        for pending_match in matches {
            match self.process_match(&pending_match).await {
                Ok(media_file) => {
                    result.files_processed += 1;
                    result.messages.push(format!(
                        "Processed: {} -> {}",
                        pending_match.source_path, media_file.path
                    ));
                }
                Err(e) => {
                    result.files_failed += 1;
                    let error_msg = format!(
                        "Failed to process {}: {}",
                        pending_match.source_path, e
                    );
                    result.messages.push(error_msg.clone());
                    
                    // Mark the match as failed
                    self.db
                        .pending_file_matches()
                        .mark_failed(pending_match.id, &error_msg)
                        .await
                        .ok();
                }
            }
        }

        info!(
            "Finished processing {}/{}: {} files processed, {} failed",
            source_type,
            source_id,
            result.files_processed,
            result.files_failed
        );

        Ok(result)
    }

    /// Process a single pending file match
    ///
    /// This is the core processing function that:
    /// 1. Loads the target item and library
    /// 2. Determines the destination path using library naming rules
    /// 3. Copies the file to the destination
    /// 4. Creates/updates the media_file record
    /// 5. Links the item to the media_file
    /// 6. Updates item status to "downloaded"
    /// 7. Queues for FFprobe analysis
    pub async fn process_match(
        &self,
        pending_match: &PendingFileMatchRecord,
    ) -> Result<MediaFileRecord> {
        let source_path = &pending_match.source_path;

        // Verify source file exists
        if !Path::new(source_path).exists() {
            return Err(anyhow::anyhow!("Source file does not exist: {}", source_path));
        }

        // Route based on match target type
        let media_file = if let Some(episode_id) = pending_match.episode_id {
            self.process_episode_match(pending_match, episode_id).await?
        } else if let Some(movie_id) = pending_match.movie_id {
            self.process_movie_match(pending_match, movie_id).await?
        } else if let Some(track_id) = pending_match.track_id {
            self.process_track_match(pending_match, track_id).await?
        } else if let Some(chapter_id) = pending_match.chapter_id {
            self.process_chapter_match(pending_match, chapter_id).await?
        } else {
            return Err(anyhow::anyhow!("No target ID in pending match"));
        };

        // Mark the match as copied
        self.db
            .pending_file_matches()
            .mark_copied(pending_match.id)
            .await?;

        // Link the torrent_file to the media_file (if source is a torrent)
        if pending_match.source_type == "torrent" {
            if let (Some(source_id), Some(file_index)) = (pending_match.source_id, pending_match.source_file_index) {
                if let Err(e) = self
                    .db
                    .torrent_files()
                    .set_media_file_id(source_id, file_index, media_file.id)
                    .await
                {
                    warn!(
                        torrent_id = %source_id,
                        file_index = file_index,
                        media_file_id = %media_file.id,
                        error = %e,
                        "Failed to link torrent file to media file"
                    );
                } else {
                    debug!(
                        torrent_id = %source_id,
                        file_index = file_index,
                        media_file_id = %media_file.id,
                        "Linked torrent file to media file"
                    );
                }
            }
        }

        // Queue for FFprobe analysis
        if let Some(ref queue) = self.analysis_queue {
            if let Err(e) = queue
                .submit(crate::services::queues::MediaAnalysisJob {
                    media_file_id: media_file.id,
                    path: std::path::PathBuf::from(&media_file.path),
                    check_subtitles: false,
                })
                .await
            {
                warn!(
                    media_file_id = %media_file.id,
                    error = %e,
                    "Failed to queue media file for analysis"
                );
            } else {
                debug!(
                    media_file_id = %media_file.id,
                    "Queued media file for FFprobe analysis"
                );
            }
        }

        Ok(media_file)
    }

    /// For library scans: link an existing file in the library
    ///
    /// The file is already in the library folder, so no copying is needed.
    /// Just creates the media_file record and links it to the item.
    pub async fn link_existing_file(
        &self,
        file_path: &str,
        file_size: i64,
        library_id: Uuid,
        target: ProcessTarget,
    ) -> Result<MediaFileRecord> {
        // Create media_file record
        let media_file = self
            .db
            .media_files()
            .upsert(CreateMediaFile {
                library_id,
                path: file_path.to_string(),
                size_bytes: file_size,
                container: get_container(file_path),
                video_codec: None,
                audio_codec: None,
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                file_hash: None,
                episode_id: match &target {
                    ProcessTarget::Episode(id) => Some(*id),
                    _ => None,
                },
                movie_id: match &target {
                    ProcessTarget::Movie(id) => Some(*id),
                    _ => None,
                },
                track_id: match &target {
                    ProcessTarget::Track(id) => Some(*id),
                    _ => None,
                },
                chapter_id: match &target {
                    ProcessTarget::Chapter(id) => Some(*id),
                    _ => None,
                },
                relative_path: None,
                original_name: Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                resolution: None,
                is_hdr: None,
                hdr_type: None,
                ..Default::default()
            })
            .await?;

        // Bidirectional link: Set media_file_id on the content item
        // (media_file already has the content FK set in the upsert above)
        match target {
            ProcessTarget::Episode(id) => {
                // Bidirectional: episode.media_file_id -> media_file
                self.db.episodes().set_media_file(id, media_file.id).await?;
            }
            ProcessTarget::Movie(id) => {
                // Bidirectional: movie.media_file_id -> media_file
                self.db.movies().set_media_file(id, media_file.id).await?;
            }
            ProcessTarget::Track(id) => {
                // Bidirectional: track.media_file_id -> media_file
                self.db.tracks().link_media_file(id, media_file.id).await?;
            }
            ProcessTarget::Chapter(id) => {
                // Bidirectional: chapter.media_file_id -> media_file
                self.db.chapters().link_media_file(id, media_file.id).await?;
            }
        }

        // Queue for FFprobe analysis
        if let Some(ref queue) = self.analysis_queue {
            queue
                .submit(crate::services::queues::MediaAnalysisJob {
                    media_file_id: media_file.id,
                    path: std::path::PathBuf::from(file_path),
                    check_subtitles: false,
                })
                .await
                .ok();
        }

        Ok(media_file)
    }

    // =========================================================================
    // Internal Processing Methods
    // =========================================================================

    async fn process_episode_match(
        &self,
        pending_match: &PendingFileMatchRecord,
        episode_id: Uuid,
    ) -> Result<MediaFileRecord> {
        // Load episode and show
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
            .context("TV show not found")?;
        let library = self
            .db
            .libraries()
            .get_by_id(show.library_id)
            .await?
            .context("Library not found")?;

        // Get naming pattern
        let pattern = self
            .db
            .naming_patterns()
            .get_default_for_type("tv")
            .await?
            .map(|p| p.pattern)
            .unwrap_or_else(|| crate::services::organizer::DEFAULT_NAMING_PATTERN.to_string());

        // Get file extension
        let ext = Path::new(&pending_match.source_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mkv");

        // Generate destination path
        let relative_path = apply_naming_pattern(&pattern, &show, &episode, ext);
        let dest_path = Path::new(&library.path).join(&relative_path);

        // Copy file
        let media_file = self
            .copy_and_create_media_file(
                &pending_match.source_path,
                &dest_path,
                &library,
                Some(episode_id),
                None,
                None,
                None,
                None, // chapter_id
            )
            .await?;

        // Bidirectional link: episode.media_file_id -> media_file
        // (media_file.episode_id is already set in copy_and_create_media_file)
        self.db.episodes().set_media_file(episode_id, media_file.id).await?;

        let file_name = dest_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
        info!(
            "Processed episode: '{}' S{:02}E{:02} -> '{}'",
            show.name,
            episode.season,
            episode.episode,
            file_name
        );

        Ok(media_file)
    }

    async fn process_movie_match(
        &self,
        pending_match: &PendingFileMatchRecord,
        movie_id: Uuid,
    ) -> Result<MediaFileRecord> {
        // Load movie and library
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

        // Get naming pattern
        let pattern = self
            .db
            .naming_patterns()
            .get_default_for_type("movies")
            .await?
            .map(|p| p.pattern)
            .unwrap_or_else(|| crate::services::organizer::DEFAULT_MOVIE_NAMING_PATTERN.to_string());

        // Get original filename and extension
        let original_filename = Path::new(&pending_match.source_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("movie.mkv");
        let ext = Path::new(&pending_match.source_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mkv");

        // Generate destination path
        let relative_path = apply_movie_naming_pattern(&pattern, &movie, original_filename, ext);
        let dest_path = Path::new(&library.path).join(&relative_path);

        // Copy file
        let media_file = self
            .copy_and_create_media_file(
                &pending_match.source_path,
                &dest_path,
                &library,
                None,
                Some(movie_id),
                None,
                None,
                None, // chapter_id
            )
            .await?;

        // Bidirectional link: movie.media_file_id -> media_file
        // (media_file.movie_id is already set in copy_and_create_media_file)
        self.db.movies().set_media_file(movie_id, media_file.id).await?;

        let file_name = dest_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
        info!(
            "Processed movie: '{}' ({}) -> '{}'",
            movie.title,
            movie.year.unwrap_or(0),
            file_name
        );

        Ok(media_file)
    }

    async fn process_track_match(
        &self,
        pending_match: &PendingFileMatchRecord,
        track_id: Uuid,
    ) -> Result<MediaFileRecord> {
        // Load track, album, and library
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
        let artist = self
            .db
            .albums()
            .get_artist_by_id(album.artist_id)
            .await?
            .context("Artist not found")?;
        let library = self
            .db
            .libraries()
            .get_by_id(album.library_id)
            .await?
            .context("Library not found")?;

        // Get naming pattern
        let pattern = self
            .db
            .naming_patterns()
            .get_default_for_type("music")
            .await?
            .map(|p| p.pattern)
            .unwrap_or_else(|| crate::services::organizer::DEFAULT_MUSIC_NAMING_PATTERN.to_string());

        // Get original filename and extension
        let original_filename = Path::new(&pending_match.source_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("track.flac");
        let ext = Path::new(&pending_match.source_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("flac");

        // Generate destination path
        let relative_path = apply_music_naming_pattern(
            &pattern,
            &artist.name,
            &album,
            Some(&track),
            original_filename,
            ext,
        );
        let dest_path = Path::new(&library.path).join(&relative_path);

        // Copy file
        let media_file = self
            .copy_and_create_media_file(
                &pending_match.source_path,
                &dest_path,
                &library,
                None,
                None,
                Some(track_id),
                Some(album.id),
                None, // chapter_id
            )
            .await?;

        // Bidirectional link: track.media_file_id -> media_file
        // (media_file.track_id is already set in copy_and_create_media_file)
        self.db.tracks().link_media_file(track_id, media_file.id).await?;

        // Update album has_files
        self.db.albums().update_has_files(album.id, true).await?;

        let file_name = dest_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
        info!(
            "Processed track: '{}' - '{}' (#{}) -> '{}'",
            album.name,
            track.title,
            track.track_number,
            file_name
        );

        Ok(media_file)
    }

    async fn process_chapter_match(
        &self,
        pending_match: &PendingFileMatchRecord,
        chapter_id: Uuid,
    ) -> Result<MediaFileRecord> {
        // Load chapter and audiobook
        let chapter = self
            .db
            .chapters()
            .get_by_id(chapter_id)
            .await?
            .context("Chapter not found")?;
        let audiobook = self
            .db
            .audiobooks()
            .get_by_id(chapter.audiobook_id)
            .await?
            .context("Audiobook not found")?;
        let author = if let Some(author_id) = audiobook.author_id {
            self.db
                .audiobooks()
                .get_author_by_id(author_id)
                .await?
                .map(|a| a.name)
                .unwrap_or_else(|| "Unknown Author".to_string())
        } else {
            "Unknown Author".to_string()
        };
        let library = self
            .db
            .libraries()
            .get_by_id(audiobook.library_id)
            .await?
            .context("Library not found")?;

        // Get naming pattern
        let pattern = self
            .db
            .naming_patterns()
            .get_default_for_type("audiobooks")
            .await?
            .map(|p| p.pattern)
            .unwrap_or_else(|| {
                crate::services::organizer::DEFAULT_AUDIOBOOK_NAMING_PATTERN.to_string()
            });

        // Get original filename and extension
        let original_filename = Path::new(&pending_match.source_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("chapter.mp3");
        let ext = Path::new(&pending_match.source_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp3");

        // Generate destination path
        let relative_path =
            apply_audiobook_naming_pattern(&pattern, &author, &audiobook, original_filename, ext);
        let dest_path = Path::new(&library.path).join(&relative_path);

        // Copy file with chapter_id set
        let media_file = self
            .copy_and_create_media_file(
                &pending_match.source_path,
                &dest_path,
                &library,
                None,
                None,
                None,
                None, // album_id
                Some(chapter_id),
            )
            .await?;

        // Bidirectional link: chapter.media_file_id -> media_file
        // (media_file.chapter_id is already set in copy_and_create_media_file)
        self.db.chapters().link_media_file(chapter_id, media_file.id).await?;

        let file_name = dest_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
        info!(
            "Processed chapter: '{}' Chapter {} -> '{}'",
            audiobook.title,
            chapter.chapter_number,
            file_name
        );

        Ok(media_file)
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Copy a file to the library and create a media_file record
    ///
    /// Always uses copy (never move) to preserve the source file for seeding.
    async fn copy_and_create_media_file(
        &self,
        source_path: &str,
        dest_path: &Path,
        library: &LibraryRecord,
        episode_id: Option<Uuid>,
        movie_id: Option<Uuid>,
        track_id: Option<Uuid>,
        album_id: Option<Uuid>,
        chapter_id: Option<Uuid>,
    ) -> Result<MediaFileRecord> {
        // Create parent directories
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!("Failed to create directory: {}", parent.display())
            })?;
        }

        // Get source file size
        let metadata = tokio::fs::metadata(source_path)
            .await
            .with_context(|| format!("Failed to get metadata for: {}", source_path))?;
        let file_size = metadata.len() as i64;

        // Copy the file (NEVER move - preserve source for seeding)
        tokio::fs::copy(source_path, dest_path)
            .await
            .with_context(|| {
                format!(
                    "Failed to copy {} to {}",
                    source_path,
                    dest_path.display()
                )
            })?;

        let file_name = Path::new(source_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(source_path);
        info!(
            "Copied '{}' to {} ({} bytes)",
            file_name,
            dest_path.display(),
            file_size
        );

        // Create media_file record
        let dest_path_str = dest_path.to_string_lossy().to_string();
        let media_file = self
            .db
            .media_files()
            .upsert(CreateMediaFile {
                library_id: library.id,
                path: dest_path_str.clone(),
                size_bytes: file_size,
                container: get_container(&dest_path_str),
                video_codec: None,
                audio_codec: None,
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                file_hash: None,
                episode_id,
                movie_id,
                track_id,
                album_id,
                chapter_id,
                relative_path: dest_path
                    .strip_prefix(&library.path)
                    .ok()
                    .and_then(|p| p.to_str())
                    .map(|s| s.to_string()),
                original_name: Path::new(source_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string()),
                resolution: None,
                is_hdr: None,
                hdr_type: None,
                ..Default::default()
            })
            .await?;

        // If it's a track, also link to album
        if let Some(aid) = album_id {
            self.db.media_files().link_to_album(media_file.id, aid).await?;
        }

        Ok(media_file)
    }
}

/// Target for processing (used by link_existing_file)
pub enum ProcessTarget {
    Episode(Uuid),
    Movie(Uuid),
    Track(Uuid),
    Chapter(Uuid),
}
