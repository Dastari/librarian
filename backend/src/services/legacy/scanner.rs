//! Library scanner service
//!
//! Walks library directories to discover media files, parse filenames,
//! identify TV shows, and update the database.
//!
//! After scanning, if the library has `organize_files` enabled, the scanner
//! will automatically organize files into the proper folder structure
//! (Show Name/Season XX/) and optionally rename them based on the rename_style.
//!
//! ## Concurrency and Rate Limiting
//!
//! The scanner uses bounded concurrency to prevent overwhelming:
//! - External APIs (TVMaze metadata lookups)
//! - File system operations
//! - Database connections
//!
//! Shows are processed in parallel with a configurable concurrency limit
//! (default: 3 concurrent metadata fetches).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::sync::{Semaphore, broadcast};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

use super::file_matcher::{FileMatcher, FileInfo, FileMatchTarget};
use super::file_processor::{FileProcessor, ProcessTarget};
use super::filename_parser::{self, ParsedEpisode};
use super::metadata::{
    AddAlbumOptions, AddAudiobookOptions, AddMovieOptions, AddTvShowOptions, MetadataProvider,
    MetadataService,
};
use super::organizer::OrganizerService;
use super::queues::{MediaAnalysisJob, MediaAnalysisQueue};
use crate::db::{CreateEpisode, CreateMediaFile, Database};

/// Configuration for scanner concurrency
#[derive(Debug, Clone)]
pub struct ScannerConfig {
    /// Maximum concurrent metadata fetches (default: 3)
    pub max_concurrent_metadata: usize,
    /// Maximum concurrent file processing (default: 10)
    pub max_concurrent_files: usize,
    /// Delay between metadata fetch batches (ms)
    pub metadata_batch_delay_ms: u64,
    /// Chunk size for file processing
    pub file_chunk_size: usize,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_metadata: 3,
            max_concurrent_files: 10,
            metadata_batch_delay_ms: 200,
            file_chunk_size: 50,
        }
    }
}

/// Video file extensions we recognize (for TV and Movies)
const VIDEO_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "m4v", "mov", "wmv", "flv", "webm", "mpeg", "mpg", "ts", "m2ts",
];

/// Audio file extensions for Music libraries
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "m4a", "aac", "ogg", "opus", "wav", "wma", "aiff", "alac", "ape", "dsf", "dff",
];

/// Audiobook file extensions
const AUDIOBOOK_EXTENSIONS: &[&str] = &["mp3", "m4a", "m4b", "aac", "ogg", "opus", "flac"];

/// Get file extensions for a library type
pub fn get_extensions_for_library_type(library_type: &str) -> &'static [&'static str] {
    match library_type {
        "movies" | "tv" => VIDEO_EXTENSIONS,
        "music" => AUDIO_EXTENSIONS,
        "audiobooks" => AUDIOBOOK_EXTENSIONS,
        _ => VIDEO_EXTENSIONS, // Default to video for "other"
    }
}

/// Scanner progress event
#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub library_id: Uuid,
    pub library_name: String,
    pub total_files: i32,
    pub scanned_files: i32,
    pub current_file: Option<String>,
    pub is_complete: bool,
    pub new_files: i32,
    pub removed_files: i32,
    pub shows_added: i32,
    pub episodes_linked: i32,
}

/// Discovered file with parsed info
#[derive(Debug, Clone)]
struct DiscoveredFile {
    path: String,
    size: u64,
    filename: String,
    parsed: ParsedEpisode,
    relative_path: Option<String>,
}

/// Audio file metadata extracted from ID3 tags
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

/// Scanner service for discovering media files
use crate::services::graphql::{Library as GqlLibrary, LibraryChangeType, LibraryChangedEvent};
use crate::indexer::manager::IndexerManager;
use crate::services::TorrentService;

pub struct ScannerService {
    db: Database,
    metadata_service: Arc<MetadataService>,
    progress_tx: broadcast::Sender<ScanProgress>,
    config: ScannerConfig,
    /// Semaphore to limit concurrent metadata fetches
    metadata_semaphore: Arc<Semaphore>,
    /// Optional queue for FFmpeg analysis of discovered files
    analysis_queue: Option<Arc<MediaAnalysisQueue>>,
    /// Optional broadcast sender for library changed events
    library_changed_tx: Option<broadcast::Sender<LibraryChangedEvent>>,
    /// Optional TorrentService for auto-hunt after scans
    torrent_service: Option<Arc<TorrentService>>,
    /// Optional IndexerManager for auto-hunt after scans
    indexer_manager: Option<Arc<IndexerManager>>,
    /// Optional notification service for user alerts
    notification_service: Option<Arc<super::NotificationService>>,
    /// Track if we've already notified about missing TMDB key (to avoid spam)
    tmdb_key_notified: std::sync::atomic::AtomicBool,
}

impl ScannerService {
    /// Create a new scanner service with default config
    pub fn new(db: Database, metadata_service: Arc<MetadataService>) -> Self {
        Self::with_config(db, metadata_service, ScannerConfig::default())
    }

    /// Create a new scanner service with custom config
    pub fn with_config(
        db: Database,
        metadata_service: Arc<MetadataService>,
        config: ScannerConfig,
    ) -> Self {
        let (progress_tx, _) = broadcast::channel(100);
        let metadata_semaphore = Arc::new(Semaphore::new(config.max_concurrent_metadata));
        Self {
            db,
            metadata_service,
            progress_tx,
            config,
            metadata_semaphore,
            analysis_queue: None,
            notification_service: None,
            tmdb_key_notified: std::sync::atomic::AtomicBool::new(false),
            library_changed_tx: None,
            torrent_service: None,
            indexer_manager: None,
        }
    }

    /// Set the library changed event broadcast channel
    pub fn with_library_changed_tx(mut self, tx: broadcast::Sender<LibraryChangedEvent>) -> Self {
        self.library_changed_tx = Some(tx);
        self
    }

    /// Set the media analysis queue for FFmpeg metadata extraction
    pub fn with_analysis_queue(mut self, queue: Arc<MediaAnalysisQueue>) -> Self {
        self.analysis_queue = Some(queue);
        self
    }

    /// Set the torrent service for auto-hunt after scans
    pub fn with_torrent_service(mut self, service: Arc<TorrentService>) -> Self {
        self.torrent_service = Some(service);
        self
    }

    /// Set the indexer manager for auto-hunt after scans
    pub fn with_indexer_manager(mut self, manager: Arc<IndexerManager>) -> Self {
        self.indexer_manager = Some(manager);
        self
    }

    /// Set the notification service for user alerts
    pub fn with_notification_service(mut self, service: Arc<super::NotificationService>) -> Self {
        self.notification_service = Some(service);
        self
    }

    /// Subscribe to scan progress updates - for GraphQL subscriptions
    #[allow(dead_code)]
    pub fn subscribe(&self) -> broadcast::Receiver<ScanProgress> {
        self.progress_tx.subscribe()
    }

    /// Broadcast a library changed event (for scan start/stop)
    async fn broadcast_library_changed(&self, library_id: Uuid) {
        if let Some(tx) = &self.library_changed_tx {
            // Fetch the updated library to include in the event
            if let Ok(Some(lib)) = self.db.libraries().get_by_id(library_id).await {
                let _ = tx.send(LibraryChangedEvent {
                    change_type: LibraryChangeType::Updated,
                    library_id: library_id.to_string(),
                    library_name: Some(lib.name.clone()),
                    library: Some(GqlLibrary::from_db(lib)),
                });
            }
        }
    }

    /// Scan a specific library
    pub async fn scan_library(&self, library_id: Uuid) -> Result<ScanProgress> {
        debug!(library_id = %library_id, "scan_library called");
        
        // Get library info
        let library = self
            .db
            .libraries()
            .get_by_id(library_id)
            .await
            .context("Failed to query library from database")?
            .context("Library not found")?;

        debug!(
            library_id = %library_id,
            name = %library.name,
            path = %library.path,
            library_type = %library.library_type,
            scanning = library.scanning,
            "Retrieved library info"
        );

        // Check if already scanning
        if library.scanning {
            warn!(library_id = %library_id, name = %library.name, "Scan already in progress, skipping");
            return Ok(ScanProgress {
                library_id,
                library_name: library.name,
                total_files: 0,
                scanned_files: 0,
                current_file: None,
                is_complete: true,
                new_files: 0,
                removed_files: 0,
                shows_added: 0,
                episodes_linked: 0,
            });
        }

        // Set scanning state to true
        self.db.libraries().set_scanning(library_id, true).await?;
        self.broadcast_library_changed(library_id).await;

        info!(
            "Beginning scan of library '{}' ({})",
            library.name, library.library_type
        );

        let library_path = Path::new(&library.path);
        if !library_path.exists() {
            warn!(
                "Library path does not exist for '{}': {}",
                library.name, library.path
            );
            // Set scanning back to false before returning
            let _ = self.db.libraries().set_scanning(library_id, false).await;
            self.broadcast_library_changed(library_id).await;
            return Ok(ScanProgress {
                library_id,
                library_name: library.name,
                total_files: 0,
                scanned_files: 0,
                current_file: None,
                is_complete: true,
                new_files: 0,
                removed_files: 0,
                shows_added: 0,
                episodes_linked: 0,
            });
        }

        // For TV libraries, run consolidation first to merge duplicate folders
        // and delete duplicate files before we discover files
        if library.library_type == "tv" && library.organize_files {
            debug!("Running pre-scan consolidation for '{}'", library.name);
            let organizer = OrganizerService::new(self.db.clone());
            match organizer.consolidate_library(library_id).await {
                Ok(result) => {
                    if result.files_moved > 0 || result.folders_removed > 0 {
                        info!(
                            "Pre-scan consolidation for '{}': {} files moved, {} folders removed",
                            library.name, result.files_moved, result.folders_removed
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        "Pre-scan consolidation failed for '{}': {}",
                        library.name, e
                    );
                }
            }
        }

        // Get extensions for this library type
        let valid_extensions = get_extensions_for_library_type(&library.library_type);

        // First pass: collect all media files
        let mut video_files: Vec<DiscoveredFile> = Vec::new();

        for entry in WalkDir::new(library_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|e| e.to_str())
                && valid_extensions.contains(&ext.to_lowercase().as_str())
            {
                let path_str = path.to_string_lossy().to_string();
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Parse based on library type
                let parsed = match library.library_type.as_str() {
                    "tv" => filename_parser::parse_episode(&filename),
                    "movies" => filename_parser::parse_movie(&filename),
                    _ => filename_parser::parse_episode(&filename), // Fallback
                };

                let relative_path = path
                    .strip_prefix(library_path)
                    .map(|p| p.to_string_lossy().to_string())
                    .ok();

                video_files.push(DiscoveredFile {
                    path: path_str,
                    size,
                    filename,
                    parsed,
                    relative_path,
                });
            }
        }

        let total_files = video_files.len() as i32;
        info!(
            "Found {} media files to scan in '{}'",
            total_files, library.name
        );

        // Initialize progress
        let mut progress = ScanProgress {
            library_id,
            library_name: library.name.clone(),
            total_files,
            scanned_files: 0,
            current_file: None,
            is_complete: false,
            new_files: 0,
            removed_files: 0,
            shows_added: 0,
            episodes_linked: 0,
        };
        let _ = self.progress_tx.send(progress.clone());

        // Check library type and auto_add setting
        let library_type = library.library_type.as_str();
        let is_tv_library = library_type == "tv";
        let is_movie_library = library_type == "movies";
        let auto_add = library.auto_add_discovered;

        // Process files based on library type and auto_add setting
        if is_tv_library && auto_add {
            // TV library with auto-add: group by show and match metadata
            progress = self
                .process_tv_library_with_auto_add(
                    library_id,
                    library.user_id,
                    video_files,
                    progress,
                )
                .await?;
        } else if is_movie_library && auto_add {
            // Movie library with auto-add: group by title+year and match TMDB
            progress = self
                .process_movie_library_with_auto_add(
                    library_id,
                    library.user_id,
                    video_files,
                    progress,
                )
                .await?;
        } else if library_type == "music" && auto_add {
            // Music library with auto-add: parse ID3 tags, match MusicBrainz
            progress = self
                .process_music_library_with_auto_add(
                    library_id,
                    library.user_id,
                    video_files, // Contains audio files for music libraries
                    progress,
                )
                .await?;
        } else if library_type == "audiobooks" && auto_add {
            // Audiobook library with auto-add: parse audio files, match OpenLibrary
            progress = self
                .process_audiobook_library_with_auto_add(
                    library_id,
                    library.user_id,
                    video_files, // Contains audio files for audiobook libraries
                    progress,
                )
                .await?;
        } else {
            // Simple processing - just add files without metadata matching
            progress = self
                .process_files_simple(library_id, &library.path, video_files, progress)
                .await?;
        }

        // Note: File removal is handled in process_* methods
        // The scan tracks new files added; removed files would need a separate cleanup job

        // Update library last_scanned_at
        self.db.libraries().update_last_scanned(library_id).await?;

        // Auto-organize files if the library has organize_files enabled
        let is_music_library = library_type == "music";
        let is_audiobook_library = library_type == "audiobooks";
        if library.organize_files
            && (is_tv_library || is_movie_library || is_music_library || is_audiobook_library)
        {
            info!("Running automatic file organization for '{}'", library.name);
            if let Err(e) = self.organize_library_files(library_id).await {
                error!("Failed to organize '{}': {}", library.name, e);
            }
        }

        // Set scanning state back to false
        self.db.libraries().set_scanning(library_id, false).await?;
        self.broadcast_library_changed(library_id).await;

        // Send final progress
        progress.is_complete = true;
        progress.current_file = None;
        let _ = self.progress_tx.send(progress.clone());

        info!(
            library_id = %library_id,
            total = progress.total_files,
            new = progress.new_files,
            removed = progress.removed_files,
            shows_added = progress.shows_added,
            episodes_linked = progress.episodes_linked,
            "Library scan completed for '{}': {} files ({} new, {} removed)",
            library.name, progress.total_files, progress.new_files, progress.removed_files
        );

        // Trigger auto-hunt for this library if enabled (library-level or show-level overrides)
        // Check if library has auto_hunt enabled OR any shows with auto_hunt_override = true
        let should_auto_hunt = if library.auto_hunt {
            true
        } else if library.library_type.to_lowercase() == "tv" {
            // Check for any TV shows with auto_hunt_override = true
            let has_override: bool = {
                use crate::db::sqlite_helpers::uuid_to_str;
                let result: Option<i32> = sqlx::query_scalar(
                    "SELECT 1 FROM tv_shows WHERE library_id = ?1 AND auto_hunt_override = 1 AND monitored = 1 LIMIT 1",
                )
                .bind(uuid_to_str(library_id))
                .fetch_optional(self.db.pool())
                .await
                .unwrap_or(None);
                result.is_some()
            };

            has_override
        } else {
            false
        };

        if should_auto_hunt {
            if let (Some(torrent_svc), Some(indexer_mgr)) =
                (&self.torrent_service, &self.indexer_manager)
            {
                info!(
                    "Triggering auto-hunt for '{}'{}",
                    library.name,
                    if !library.auto_hunt {
                        " (show-level override)"
                    } else {
                        ""
                    }
                );

                let pool = self.db.pool().clone();
                let torrent_svc = torrent_svc.clone();
                let indexer_mgr = indexer_mgr.clone();
                let library_name_clone = library.name.clone();

                // Run auto-hunt in background to not block scan completion
                tokio::spawn(async move {
                    match crate::jobs::auto_hunt::run_auto_hunt_for_library(
                        pool,
                        library_id,
                        torrent_svc,
                        indexer_mgr,
                    )
                    .await
                    {
                        Ok(result) => {
                            info!(
                                library_id = %library_id,
                                searched = result.searched,
                                matched = result.matched,
                                downloaded = result.downloaded,
                                "Post-scan auto-hunt completed for '{}': {} searched, {} matched, {} downloaded",
                                library_name_clone, result.searched, result.matched, result.downloaded
                            );
                        }
                        Err(e) => {
                            error!(
                                library_id = %library_id,
                                error = %e,
                                "Post-scan auto-hunt failed"
                            );
                        }
                    }
                });
            } else {
                debug!(
                    library_id = %library_id,
                    "Auto-hunt enabled but TorrentService or IndexerManager not available"
                );
            }
        }

        Ok(progress)
    }

    /// Process TV library with auto-add discovered shows
    ///
    /// Uses bounded concurrency to prevent overwhelming external APIs:
    /// - Shows are processed in parallel (limited by metadata_semaphore)
    /// - Files within each show are processed sequentially to maintain order
    async fn process_tv_library_with_auto_add(
        &self,
        library_id: Uuid,
        user_id: Uuid,
        files: Vec<DiscoveredFile>,
        mut progress: ScanProgress,
    ) -> Result<ScanProgress> {
        // Group files by parsed show name
        let mut files_by_show: HashMap<String, Vec<DiscoveredFile>> = HashMap::new();
        let mut unlinked_files: Vec<DiscoveredFile> = Vec::new();

        for file in files {
            if let Some(ref show_name) = file.parsed.show_name {
                let normalized = show_name.to_lowercase();
                files_by_show.entry(normalized).or_default().push(file);
            } else {
                // No show name parsed, add without linking
                unlinked_files.push(file);
            }
        }

        // Process unlinked files first (no API calls needed)
        for file in unlinked_files {
            self.add_unlinked_file(library_id, &file, &mut progress)
                .await?;
        }

        let show_count = files_by_show.len();
        info!(
            show_groups = show_count,
            max_concurrent = self.config.max_concurrent_metadata,
            "Processing show groups with bounded concurrency"
        );

        // Use atomic counters for thread-safe progress tracking
        let shows_added = Arc::new(AtomicI32::new(0));
        let scanned_files = Arc::new(AtomicI32::new(progress.scanned_files));
        let episodes_linked = Arc::new(AtomicI32::new(progress.episodes_linked));
        let new_files = Arc::new(AtomicI32::new(progress.new_files));

        // Collect show groups into a vec for parallel processing
        let show_groups: Vec<(String, Vec<DiscoveredFile>)> = files_by_show.into_iter().collect();

        // Process shows in chunks to avoid overwhelming the system
        let chunk_size = self.config.max_concurrent_metadata;
        let mut processed_shows = 0;

        for chunk in show_groups.chunks(chunk_size) {
            let mut handles = Vec::with_capacity(chunk.len());

            for (_normalized_name, show_files) in chunk {
                if show_files.is_empty() {
                    continue;
                }

                // Clone values for the spawned task
                let library_id = library_id;
                let user_id = user_id;
                let db = self.db.clone();
                let metadata_service = self.metadata_service.clone();
                let semaphore = self.metadata_semaphore.clone();
                let progress_tx = self.progress_tx.clone();
                let shows_added = shows_added.clone();
                let scanned_files = scanned_files.clone();
                let episodes_linked = episodes_linked.clone();
                let new_files = new_files.clone();
                let show_files = show_files.clone();
                let library_name = progress.library_name.clone();
                let total_files = progress.total_files;
                let analysis_queue = self.analysis_queue.clone();

                let handle = tokio::spawn(async move {
                    // Acquire semaphore permit for metadata operations
                    let _permit = semaphore.acquire().await.expect("Semaphore closed");

                    let show_name = show_files[0].parsed.show_name.clone().unwrap_or_default();
                    let year = show_files[0].parsed.year;

                    info!(
                        "Processing {} files for show '{}'",
                        show_files.len(),
                        show_name
                    );

                    // Try to find or create the TV show
                    let tv_show_id = match Self::find_or_create_tv_show_static(
                        &db,
                        &metadata_service,
                        library_id,
                        user_id,
                        &show_name,
                        year,
                    )
                    .await
                    {
                        Ok(Some((id, is_new))) => {
                            if is_new {
                                shows_added.fetch_add(1, Ordering::SeqCst);
                            }
                            Some(id)
                        }
                        Ok(None) => {
                            warn!(show_name = %show_name, "Could not find show in metadata providers");
                            None
                        }
                        Err(e) => {
                            error!(show_name = %show_name, error = %e, "Error finding/creating show");
                            None
                        }
                    };

                    // Process files for this show
                    for file in show_files {
                        let current_scanned = scanned_files.fetch_add(1, Ordering::SeqCst) + 1;

                        // Send progress update every 10 files
                        if current_scanned % 10 == 0 {
                            let _ = progress_tx.send(ScanProgress {
                                library_id,
                                library_name: library_name.clone(),
                                total_files,
                                scanned_files: current_scanned,
                                current_file: Some(file.path.clone()),
                                is_complete: false,
                                new_files: new_files.load(Ordering::SeqCst),
                                removed_files: 0,
                                shows_added: shows_added.load(Ordering::SeqCst),
                                episodes_linked: episodes_linked.load(Ordering::SeqCst),
                            });
                        }

                        // Process file based on whether we have a show
                        match tv_show_id {
                            Some(show_id) => {
                                if let Err(e) = Self::process_file_for_show(
                                    &db,
                                    library_id,
                                    show_id,
                                    &file,
                                    &episodes_linked,
                                    &new_files,
                                    analysis_queue.as_ref(),
                                )
                                .await
                                {
                                    warn!(path = %file.path, error = %e, "Failed to process file");
                                }
                            }
                            None => {
                                // Add as unlinked file
                                if let Err(e) = Self::create_unlinked_media_file_static(
                                    &db,
                                    library_id,
                                    &file,
                                    &new_files,
                                    analysis_queue.as_ref(),
                                )
                                .await
                                {
                                    warn!(path = %file.path, error = %e, "Failed to create unlinked file");
                                }
                            }
                        }
                    }

                    // Update show stats if we have a show
                    if let Some(show_id) = tv_show_id
                        && let Err(e) = db.tv_shows().update_stats(show_id).await
                    {
                        warn!(tv_show_id = %show_id, error = %e, "Failed to update show stats");
                    }
                });

                handles.push(handle);
            }

            // Wait for all shows in this chunk to complete
            for handle in handles {
                if let Err(e) = handle.await {
                    error!(error = %e, "Show processing task panicked");
                }
            }

            processed_shows += chunk.len();
            info!(
                "Processed {}/{} shows in '{}'",
                processed_shows, show_count, progress.library_name
            );

            // Small delay between chunks
            if self.config.metadata_batch_delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.config.metadata_batch_delay_ms))
                    .await;
            }
        }

        // Update progress with final counts
        progress.shows_added = shows_added.load(Ordering::SeqCst);
        progress.scanned_files = scanned_files.load(Ordering::SeqCst);
        progress.episodes_linked = episodes_linked.load(Ordering::SeqCst);
        progress.new_files = new_files.load(Ordering::SeqCst);

        Ok(progress)
    }

    /// Process movie library with auto-add discovered movies
    ///
    /// Groups discovered files by parsed title+year, searches TMDB for matches,
    /// creates movie records, and links media files to them.
    async fn process_movie_library_with_auto_add(
        &self,
        library_id: Uuid,
        user_id: Uuid,
        files: Vec<DiscoveredFile>,
        mut progress: ScanProgress,
    ) -> Result<ScanProgress> {
        // Group files by parsed movie title+year
        // Key is (normalized_title, year) tuple
        let mut files_by_movie: HashMap<(String, Option<u32>), Vec<DiscoveredFile>> =
            HashMap::new();
        let mut unlinked_files: Vec<DiscoveredFile> = Vec::new();

        for file in files {
            if let Some(ref title) = file.parsed.show_name {
                // Use title + year as the grouping key
                let normalized = title.to_lowercase();
                let key = (normalized, file.parsed.year);
                files_by_movie.entry(key).or_default().push(file);
            } else {
                // No title parsed, add without linking
                unlinked_files.push(file);
            }
        }

        // Process unlinked files first (no API calls needed)
        for file in unlinked_files {
            self.add_unlinked_file(library_id, &file, &mut progress)
                .await?;
        }

        let movie_count = files_by_movie.len();
        info!(
            movie_groups = movie_count,
            max_concurrent = self.config.max_concurrent_metadata,
            "Processing movie groups with bounded concurrency"
        );

        // Use atomic counters for thread-safe progress tracking
        let movies_added = Arc::new(AtomicI32::new(0));
        let scanned_files = Arc::new(AtomicI32::new(progress.scanned_files));
        let files_linked = Arc::new(AtomicI32::new(0));
        let new_files = Arc::new(AtomicI32::new(progress.new_files));
        // Track if we've notified about TMDB key (to avoid spam)
        let tmdb_notified = Arc::new(std::sync::atomic::AtomicBool::new(false));

        // Collect movie groups into a vec for parallel processing
        let movie_groups: Vec<((String, Option<u32>), Vec<DiscoveredFile>)> =
            files_by_movie.into_iter().collect();

        // Process movies in chunks to avoid overwhelming the system
        let chunk_size = self.config.max_concurrent_metadata;
        let mut processed_movies = 0;

        for chunk in movie_groups.chunks(chunk_size) {
            let mut handles = Vec::with_capacity(chunk.len());

            for ((_normalized_title, year), movie_files) in chunk {
                if movie_files.is_empty() {
                    continue;
                }

                // Clone values for the spawned task
                let library_id = library_id;
                let user_id = user_id;
                let db = self.db.clone();
                let metadata_service = self.metadata_service.clone();
                let semaphore = self.metadata_semaphore.clone();
                let progress_tx = self.progress_tx.clone();
                let movies_added = movies_added.clone();
                let scanned_files = scanned_files.clone();
                let files_linked = files_linked.clone();
                let new_files = new_files.clone();
                let movie_files = movie_files.clone();
                let library_name = progress.library_name.clone();
                let total_files = progress.total_files;
                let analysis_queue = self.analysis_queue.clone();
                let year = *year;
                let notification_service = self.notification_service.clone();
                let tmdb_key_notified = tmdb_notified.clone();

                let handle = tokio::spawn(async move {
                    // Acquire semaphore permit for metadata operations
                    let _permit = semaphore.acquire().await.expect("Semaphore closed");

                    let title = movie_files[0].parsed.show_name.clone().unwrap_or_default();

                    info!(
                        "Processing {} files for movie '{}'{}",
                        movie_files.len(),
                        title,
                        year.map(|y| format!(" ({})", y)).unwrap_or_default()
                    );

                    // Try to find or create the movie
                    let movie_id = match Self::find_or_create_movie_static(
                        &db,
                        &metadata_service,
                        library_id,
                        user_id,
                        &title,
                        year.map(|y| y as i32),
                    )
                    .await
                    {
                        Ok(Some((id, is_new))) => {
                            if is_new {
                                movies_added.fetch_add(1, Ordering::SeqCst);
                            }
                            Some(id)
                        }
                        Ok(None) => {
                            warn!(title = %title, year = ?year, "Could not find movie in TMDB");
                            None
                        }
                        Err(e) => {
                            let error_str = e.to_string();
                            error!(
                                title = %title, 
                                year = ?year, 
                                error = %e, 
                                "Error fetching metadata for '{}': {}", title, e
                            );
                            
                            // Create notification for TMDB API key issues (only once per scan)
                            // Use create_system_warning to notify all users since this is a system config issue
                            let is_tmdb_error = error_str.to_lowercase().contains("tmdb api key");
                            if is_tmdb_error {
                                let was_notified = tmdb_key_notified.swap(true, Ordering::SeqCst);
                                if !was_notified {
                                    if let Some(notif_svc) = &notification_service {
                                        info!(movie = %title, "Creating TMDB API key notification for all users");
                                        match notif_svc
                                            .create_system_warning(
                                                "TMDB API key not configured".to_string(),
                                                format!(
                                                    "Could not fetch metadata for '{}'. TMDB API key is not configured. \
                                                    Add your TMDB API key in Settings → Metadata to enable movie metadata, \
                                                    posters, and search.",
                                                    title
                                                ),
                                                crate::db::NotificationCategory::Configuration,
                                            )
                                            .await
                                        {
                                            Ok(records) => info!(
                                                count = records.len(),
                                                "TMDB API key notification created for all users"
                                            ),
                                            Err(e) => error!(error = %e, "Failed to create TMDB notification"),
                                        }
                                    } else {
                                        error!("Notification service not configured on scanner - this is a bug");
                                    }
                                }
                            }
                            
                            None
                        }
                    };

                    // Calculate total size before consuming movie_files
                    let total_size: i64 = movie_files.iter().map(|f| f.size as i64).sum();

                    // Process files for this movie
                    for file in movie_files {
                        let current_scanned = scanned_files.fetch_add(1, Ordering::SeqCst) + 1;

                        // Send progress update every 10 files
                        if current_scanned % 10 == 0 {
                            let _ = progress_tx.send(ScanProgress {
                                library_id,
                                library_name: library_name.clone(),
                                total_files,
                                scanned_files: current_scanned,
                                current_file: Some(file.path.clone()),
                                is_complete: false,
                                new_files: new_files.load(Ordering::SeqCst),
                                removed_files: 0,
                                shows_added: movies_added.load(Ordering::SeqCst),
                                episodes_linked: files_linked.load(Ordering::SeqCst),
                            });
                        }

                        // Process file based on whether we have a movie
                        match movie_id {
                            Some(movie_id) => {
                                if let Err(e) = Self::process_file_for_movie(
                                    &db,
                                    library_id,
                                    movie_id,
                                    &file,
                                    &files_linked,
                                    &new_files,
                                    analysis_queue.as_ref(),
                                )
                                .await
                                {
                                    warn!(path = %file.path, error = %e, "Failed to process file for movie");
                                }
                            }
                            None => {
                                // Add as unlinked file
                                if let Err(e) = Self::create_unlinked_media_file_static(
                                    &db,
                                    library_id,
                                    &file,
                                    &new_files,
                                    analysis_queue.as_ref(),
                                )
                                .await
                                {
                                    warn!(path = %file.path, error = %e, "Failed to create unlinked file");
                                }
                            }
                        }
                    }

                    // Movie status is now computed from media_file_id - no stats update needed
                    let _ = movie_id; // Suppress unused variable warning
                    let _ = total_size; // Suppress unused variable warning
                });

                handles.push(handle);
            }

            // Wait for all movies in this chunk to complete
            for handle in handles {
                if let Err(e) = handle.await {
                    error!(error = %e, "Movie processing task panicked");
                }
            }

            processed_movies += chunk.len();
            info!(
                "Processed {}/{} movies in '{}'",
                processed_movies, movie_count, progress.library_name
            );

            // Small delay between chunks
            if self.config.metadata_batch_delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.config.metadata_batch_delay_ms))
                    .await;
            }
        }

        // Update progress with final counts (reuse shows_added for movies_added)
        progress.shows_added = movies_added.load(Ordering::SeqCst);
        progress.scanned_files = scanned_files.load(Ordering::SeqCst);
        progress.episodes_linked = files_linked.load(Ordering::SeqCst);
        progress.new_files = new_files.load(Ordering::SeqCst);

        Ok(progress)
    }

    /// Static version of find_or_create_movie for use in spawned tasks
    async fn find_or_create_movie_static(
        db: &Database,
        metadata_service: &Arc<MetadataService>,
        library_id: Uuid,
        user_id: Uuid,
        title: &str,
        year: Option<i32>,
    ) -> Result<Option<(Uuid, bool)>> {
        let movies_repo = db.movies();

        // Search for the movie using metadata service
        let search_results = metadata_service.search_movies(title, year).await?;

        if search_results.is_empty() {
            // Try without year
            let retry_results = metadata_service.search_movies(title, None).await?;
            if retry_results.is_empty() {
                return Ok(None);
            }
            // Use first result from retry
            let best_match = &retry_results[0];

            // Check if we already have this movie in the library
            if let Some(existing) = movies_repo
                .get_by_tmdb_id(library_id, best_match.provider_id as i32)
                .await?
            {
                debug!("Movie '{}' already exists in library", existing.title);
                return Ok(Some((existing.id, false)));
            }

            // Add the movie from provider
            let movie = metadata_service
                .add_movie_from_provider(AddMovieOptions {
                    provider: MetadataProvider::Tmdb,
                    provider_id: best_match.provider_id,
                    library_id,
                    user_id,
                    monitored: true,
                })
                .await?;

            return Ok(Some((movie.id, true)));
        }

        // Get the best match
        let best_match = &search_results[0];

        // Check if we already have this movie in the library
        if let Some(existing) = movies_repo
            .get_by_tmdb_id(library_id, best_match.provider_id as i32)
            .await?
        {
            debug!("Movie '{}' already exists in library", existing.title);
            return Ok(Some((existing.id, false)));
        }

        // Add the movie from provider
        let movie = metadata_service
            .add_movie_from_provider(AddMovieOptions {
                provider: MetadataProvider::Tmdb,
                provider_id: best_match.provider_id,
                library_id,
                user_id,
                monitored: true,
            })
            .await?;

        Ok(Some((movie.id, true)))
    }

    /// Process a single file for a movie
    ///
    /// Uses FileMatcher to verify/find the movie match, then FileProcessor
    /// to create media_file and set bidirectional links.
    async fn process_file_for_movie(
        db: &Database,
        library_id: Uuid,
        movie_id: Uuid,
        file: &DiscoveredFile,
        files_linked: &Arc<AtomicI32>,
        new_files: &Arc<AtomicI32>,
        analysis_queue: Option<&Arc<MediaAnalysisQueue>>,
    ) -> Result<()> {
        // Check if file already exists and is properly linked
        let media_files_repo = db.media_files();
        if let Some(existing_file) = media_files_repo.get_by_path(&file.path).await? {
            if existing_file.movie_id.is_some() {
                // File already linked - but check if it needs analysis
                if existing_file.ffprobe_analyzed_at.is_none() {
                    // File was never analyzed - queue it now
                    if let Some(queue) = analysis_queue {
                        debug!(
                            path = %file.path,
                            media_file_id = %existing_file.id,
                            "File already linked but not analyzed (ffprobe_analyzed_at is null), queueing for analysis"
                        );
                        let job = MediaAnalysisJob {
                            media_file_id: existing_file.id,
                            path: std::path::PathBuf::from(&file.path),
                            check_subtitles: true,
                        };
                        if let Err(e) = queue.submit(job).await {
                            warn!(
                                media_file_id = %existing_file.id,
                                error = %e,
                                "Failed to queue existing file for analysis"
                            );
                        }
                    }
                }
                return Ok(());
            }
            // File exists but not linked - will be handled by FileProcessor below
            debug!(
                path = %file.path,
                "File exists in database but not linked to movie, will link now"
            );
        }

        // Get the library to use with FileMatcher
        let library = db.libraries().get_by_id(library_id).await?
            .context("Library not found")?;

        // Use FileMatcher to verify this is a valid video file (detect samples, etc.)
        let file_matcher = FileMatcher::new(db.clone());
        let file_info = FileInfo {
            path: file.path.clone(),
            size: file.size as i64,
            file_index: None,
            source_name: None,
        };

        let filename = std::path::Path::new(&file.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file.path);

        // Match video file against the library
        let matches = file_matcher.match_video_file(&file_info, filename, &[library]).await?;

        // Check if this is a sample file (FileMatcher detects samples)
        let is_sample = matches.iter().any(|m| matches!(m.match_target, FileMatchTarget::Sample));
        if is_sample {
            debug!(path = %file.path, "File detected as sample by FileMatcher, skipping");
            return Ok(());
        }

        // Try to find a match for the expected movie, or use the passed movie_id as fallback
        let target_movie_id = matches
            .iter()
            .find_map(|m| {
                if let FileMatchTarget::Movie { movie_id: matched_id, .. } = &m.match_target {
                    if matched_id == &movie_id {
                        Some(*matched_id)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap_or(movie_id);

        // Use FileProcessor to create media_file and set bidirectional links
        let file_processor = if let Some(queue) = analysis_queue {
            FileProcessor::with_analysis_queue(db.clone(), queue.clone())
        } else {
            FileProcessor::new(db.clone())
        };

        match file_processor
            .link_existing_file(
                &file.path,
                file.size as i64,
                library_id,
                ProcessTarget::Movie(target_movie_id),
            )
            .await
        {
            Ok(_media_file) => {
                files_linked.fetch_add(1, Ordering::SeqCst);
                new_files.fetch_add(1, Ordering::SeqCst);
                debug!(
                    path = %file.path,
                    movie_id = %target_movie_id,
                    "Linked file to movie via FileMatcher + FileProcessor"
                );
            }
            Err(e) => {
                error!(path = %file.path, error = %e, "Failed to link file to movie");
            }
        }

        Ok(())
    }

    /// Process music library with auto-add discovered albums
    ///
    /// Parses audio files for ID3 tags, groups by artist/album,
    /// searches MusicBrainz for matches, creates records, and links files.
    async fn process_music_library_with_auto_add(
        &self,
        library_id: Uuid,
        user_id: Uuid,
        files: Vec<DiscoveredFile>,
        mut progress: ScanProgress,
    ) -> Result<ScanProgress> {
        // Parse ID3 tags and group files by album
        // Key is (artist_name, album_name) tuple
        let mut files_by_album: HashMap<(String, String), Vec<(DiscoveredFile, AudioMetadata)>> =
            HashMap::new();
        let mut unlinked_files: Vec<DiscoveredFile> = Vec::new();

        for file in files {
            // Try to read ID3 tags
            match Self::read_audio_metadata(&file.path) {
                Some(meta) if meta.artist.is_some() && meta.album.is_some() => {
                    let key = (
                        meta.artist.clone().unwrap().to_lowercase(),
                        meta.album.clone().unwrap().to_lowercase(),
                    );
                    files_by_album.entry(key).or_default().push((file, meta));
                }
                _ => {
                    // No tags or incomplete, add as unlinked
                    unlinked_files.push(file);
                }
            }
        }

        // Process unlinked files first (no API calls needed)
        for file in unlinked_files {
            self.add_unlinked_file(library_id, &file, &mut progress)
                .await?;
        }

        let album_count = files_by_album.len();
        info!(
            album_groups = album_count,
            max_concurrent = self.config.max_concurrent_metadata,
            "Processing album groups with bounded concurrency"
        );

        // Use atomic counters for thread-safe progress tracking
        let albums_added = Arc::new(AtomicI32::new(0));
        let scanned_files = Arc::new(AtomicI32::new(progress.scanned_files));
        let files_linked = Arc::new(AtomicI32::new(0));
        let new_files = Arc::new(AtomicI32::new(progress.new_files));

        // Collect album groups into a vec for parallel processing
        let album_groups: Vec<((String, String), Vec<(DiscoveredFile, AudioMetadata)>)> =
            files_by_album.into_iter().collect();

        // Process albums in chunks to avoid overwhelming the system
        let chunk_size = self.config.max_concurrent_metadata;
        let mut processed_albums = 0;

        for chunk in album_groups.chunks(chunk_size) {
            let mut handles = Vec::with_capacity(chunk.len());

            for ((_artist_key, _album_key), album_files) in chunk {
                if album_files.is_empty() {
                    continue;
                }

                // Get the first file's metadata for the search
                let first_meta = &album_files[0].1;
                let artist_name = first_meta.artist.clone().unwrap_or_default();
                let album_name = first_meta.album.clone().unwrap_or_default();

                // Clone values for the spawned task
                let library_id = library_id;
                let user_id = user_id;
                let db = self.db.clone();
                let metadata_service = self.metadata_service.clone();
                let semaphore = self.metadata_semaphore.clone();
                let progress_tx = self.progress_tx.clone();
                let albums_added = albums_added.clone();
                let scanned_files = scanned_files.clone();
                let files_linked = files_linked.clone();
                let new_files = new_files.clone();
                let album_files: Vec<_> = album_files.clone();
                let library_name = progress.library_name.clone();
                let total_files = progress.total_files;
                let analysis_queue = self.analysis_queue.clone();

                let handle = tokio::spawn(async move {
                    // Acquire semaphore permit for metadata operations
                    let _permit = semaphore.acquire().await.expect("Semaphore closed");

                    info!(
                        artist = %artist_name,
                        album = %album_name,
                        file_count = album_files.len(),
                        "Processing album group"
                    );

                    // Try to find or create the album
                    let album_id = match Self::find_or_create_album_static(
                        &db,
                        &metadata_service,
                        library_id,
                        user_id,
                        &artist_name,
                        &album_name,
                    )
                    .await
                    {
                        Ok(Some((id, is_new))) => {
                            if is_new {
                                albums_added.fetch_add(1, Ordering::SeqCst);
                            }
                            Some(id)
                        }
                        Ok(None) => {
                            warn!(
                                artist = %artist_name,
                                album = %album_name,
                                "Could not find album in MusicBrainz"
                            );
                            None
                        }
                        Err(e) => {
                            error!(
                                artist = %artist_name,
                                album = %album_name,
                                error = %e,
                                "Error finding/creating album"
                            );
                            None
                        }
                    };

                    // Process files for this album
                    for (file, meta) in &album_files {
                        let current_scanned = scanned_files.fetch_add(1, Ordering::SeqCst) + 1;

                        // Send progress update every 10 files
                        if current_scanned % 10 == 0 {
                            let _ = progress_tx.send(ScanProgress {
                                library_id,
                                library_name: library_name.clone(),
                                total_files,
                                scanned_files: current_scanned,
                                current_file: Some(file.path.clone()),
                                is_complete: false,
                                new_files: new_files.load(Ordering::SeqCst),
                                removed_files: 0,
                                shows_added: albums_added.load(Ordering::SeqCst),
                                episodes_linked: files_linked.load(Ordering::SeqCst),
                            });
                        }

                        // Check if file already exists in database
                        let media_files_repo = db.media_files();
                        if let Ok(Some(existing_file)) = media_files_repo.get_by_path(&file.path).await {
                            // File exists - verify its album link using embedded metadata
                            let file_name = std::path::Path::new(&file.path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(&file.path);

                            // Get linked album/track names for better logging
                            let linked_to = if let Some(album_id) = existing_file.album_id {
                                if let Ok(Some(album)) = db.albums().get_by_id(album_id).await {
                                    if let Some(track_id) = existing_file.track_id {
                                        if let Ok(Some(track)) = db.tracks().get_by_id(track_id).await {
                                            format!("\"{}\" - \"{}\"", album.name, track.title)
                                        } else {
                                            format!("\"{}\"", album.name)
                                        }
                                    } else {
                                        format!("\"{}\"", album.name)
                                    }
                                } else {
                                    "unknown album".to_string()
                                }
                            } else {
                                "unlinked".to_string()
                            };

                            info!("Verifying: {} (linked to: {})", file_name, linked_to);

                            // Skip verification/auto-correction for manually matched files
                            // Manual matches should never be overwritten by automatic matching
                            if existing_file.match_type.as_deref() == Some("manual") {
                                debug!("Skipping verification for manually matched file: {}", file_name);
                            } else {
                                match Self::verify_music_file_link(&db, &file.path, &existing_file).await {
                                    Ok(true) => {
                                        debug!("Verified OK: {}", file_name);
                                    }
                                    Ok(false) => {
                                        // Mismatch detected - try to fix
                                        match Self::try_fix_music_file_link(&db, library_id, &file.path, &existing_file).await {
                                            Ok(true) => {
                                                info!("Auto-corrected: {} -> new album link", file_name);
                                            }
                                            Ok(false) => {
                                                warn!("Could not auto-correct: {} - manual review needed", file_name);
                                            }
                                            Err(e) => {
                                                warn!("Error fixing {}: {}", file_name, e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Error verifying {}: {}", file_name, e);
                                    }
                                }
                            }
                        } else if let Some(album_id) = album_id {
                            // New file with known album - use FileProcessor to link to track
                            if let Err(e) = Self::process_file_for_album(
                                &db,
                                library_id,
                                album_id,
                                file,
                                meta,
                                &files_linked,
                                &new_files,
                                analysis_queue.as_ref(),
                            )
                            .await
                            {
                                warn!(path = %file.path, error = %e, "Failed to process file for album");
                            }
                        } else {
                            // No album found - create unlinked media file
                            if let Err(e) = Self::create_unlinked_media_file_static(
                                &db,
                                library_id,
                                file,
                                &new_files,
                                analysis_queue.as_ref(),
                            )
                            .await
                            {
                                warn!(path = %file.path, error = %e, "Failed to create unlinked media file");
                            }
                        }
                    }

                    // Update album stats if we have an album
                    if let Some(album_id) = album_id {
                        // Calculate total size from all files
                        let total_size: i64 = album_files.iter().map(|(f, _)| f.size as i64).sum();
                        if let Err(e) = db.albums().update_has_files(album_id, true).await {
                            warn!(album_id = %album_id, error = %e, "Failed to update album has_files");
                        }
                        // Update size_bytes would require adding that method
                        let _ = total_size; // Suppress unused warning
                    }
                });

                handles.push(handle);
            }

            // Wait for all albums in this chunk to complete
            for handle in handles {
                if let Err(e) = handle.await {
                    error!(error = %e, "Album processing task panicked");
                }
            }

            processed_albums += chunk.len();
            info!(
                "Processed {}/{} albums in '{}'",
                processed_albums, album_count, progress.library_name
            );

            // Small delay between chunks
            if self.config.metadata_batch_delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.config.metadata_batch_delay_ms))
                    .await;
            }
        }

        // Update progress with final counts (reuse shows_added for albums_added)
        progress.shows_added = albums_added.load(Ordering::SeqCst);
        progress.scanned_files = scanned_files.load(Ordering::SeqCst);
        progress.episodes_linked = files_linked.load(Ordering::SeqCst);
        progress.new_files = new_files.load(Ordering::SeqCst);

        Ok(progress)
    }

    /// Process audiobook library with auto-add discovered audiobooks
    ///
    /// Parses audio files, groups by folder structure (assumed one audiobook per folder),
    /// searches OpenLibrary for matches, creates records, and links files.
    async fn process_audiobook_library_with_auto_add(
        &self,
        library_id: Uuid,
        user_id: Uuid,
        files: Vec<DiscoveredFile>,
        mut progress: ScanProgress,
    ) -> Result<ScanProgress> {
        use std::path::Path;

        // Group files by parent folder (each folder is treated as one audiobook)
        // This is common for audiobook organization where chapters are separate files
        let mut files_by_folder: HashMap<String, Vec<DiscoveredFile>> = HashMap::new();

        for file in files {
            // Get the parent folder name as the key
            let path = Path::new(&file.path);
            let folder_key = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .map(|s| s.to_lowercase())
                .unwrap_or_else(|| "unknown".to_string());

            files_by_folder.entry(folder_key).or_default().push(file);
        }

        let folder_count = files_by_folder.len();
        info!(
            audiobook_folders = folder_count,
            max_concurrent = self.config.max_concurrent_metadata,
            "Processing audiobook folders with bounded concurrency"
        );

        // Use atomic counters for thread-safe progress tracking
        let audiobooks_added = Arc::new(AtomicI32::new(0));
        let scanned_files = Arc::new(AtomicI32::new(progress.scanned_files));
        let files_linked = Arc::new(AtomicI32::new(0));
        let new_files = Arc::new(AtomicI32::new(progress.new_files));

        // Collect folder groups into a vec for parallel processing
        let folder_groups: Vec<(String, Vec<DiscoveredFile>)> =
            files_by_folder.into_iter().collect();

        // Process folders in chunks to avoid overwhelming the system
        let chunk_size = self.config.max_concurrent_metadata;
        let mut processed_folders = 0;

        for chunk in folder_groups.chunks(chunk_size) {
            let mut handles = Vec::with_capacity(chunk.len());

            for (folder_name, audiobook_files) in chunk {
                if audiobook_files.is_empty() {
                    continue;
                }

                // Try to extract book title/author from folder name or file tags
                // Common patterns: "Author - Title", "Title (Author)", "Author/Title"
                let search_query = Self::extract_audiobook_info(folder_name, audiobook_files);

                // Clone values for the spawned task
                let library_id = library_id;
                let user_id = user_id;
                let db = self.db.clone();
                let metadata_service = self.metadata_service.clone();
                let semaphore = self.metadata_semaphore.clone();
                let progress_tx = self.progress_tx.clone();
                let audiobooks_added = audiobooks_added.clone();
                let scanned_files = scanned_files.clone();
                let files_linked = files_linked.clone();
                let new_files = new_files.clone();
                let audiobook_files: Vec<_> = audiobook_files.clone();
                let library_name = progress.library_name.clone();
                let total_files = progress.total_files;
                let analysis_queue = self.analysis_queue.clone();
                let search_query = search_query.clone();

                let handle = tokio::spawn(async move {
                    // Acquire semaphore permit for metadata operations
                    let _permit = semaphore.acquire().await.expect("Semaphore closed");

                    info!(
                        search_query = %search_query,
                        file_count = audiobook_files.len(),
                        "Processing audiobook folder"
                    );

                    // Try to find or create the audiobook
                    let audiobook_id = match Self::find_or_create_audiobook_static(
                        &db,
                        &metadata_service,
                        library_id,
                        user_id,
                        &search_query,
                    )
                    .await
                    {
                        Ok(Some((id, is_new))) => {
                            if is_new {
                                audiobooks_added.fetch_add(1, Ordering::SeqCst);
                            }
                            Some(id)
                        }
                        Ok(None) => {
                            warn!(
                                search_query = %search_query,
                                "Could not find audiobook in OpenLibrary"
                            );
                            None
                        }
                        Err(e) => {
                            error!(
                                search_query = %search_query,
                                error = %e,
                                "Error finding/creating audiobook"
                            );
                            None
                        }
                    };

                    // Process files for this audiobook
                    // Sort files by name for consistent chapter ordering
                    let mut sorted_files = audiobook_files.clone();
                    sorted_files.sort_by(|a, b| a.path.cmp(&b.path));

                    for (idx, file) in sorted_files.iter().enumerate() {
                        let current_scanned = scanned_files.fetch_add(1, Ordering::SeqCst) + 1;
                        let chapter_number = (idx + 1) as i32;

                        // Send progress update every 10 files
                        if current_scanned % 10 == 0 {
                            let _ = progress_tx.send(ScanProgress {
                                library_id,
                                library_name: library_name.clone(),
                                total_files,
                                scanned_files: current_scanned,
                                current_file: Some(file.path.clone()),
                                is_complete: false,
                                new_files: new_files.load(Ordering::SeqCst),
                                removed_files: 0,
                                shows_added: audiobooks_added.load(Ordering::SeqCst),
                                episodes_linked: files_linked.load(Ordering::SeqCst),
                            });
                        }

                        if let Some(audiobook_id) = audiobook_id {
                            // Use FileProcessor to link to chapter
                            if let Err(e) = Self::process_file_for_audiobook(
                                &db,
                                library_id,
                                audiobook_id,
                                file,
                                chapter_number,
                                &files_linked,
                                &new_files,
                                analysis_queue.as_ref(),
                            )
                            .await
                            {
                                warn!(path = %file.path, error = %e, "Failed to process file for audiobook");
                            }
                        } else {
                            // No audiobook found - create unlinked media file
                            if let Err(e) = Self::create_unlinked_media_file_static(
                                &db,
                                library_id,
                                file,
                                &new_files,
                                analysis_queue.as_ref(),
                            )
                            .await
                            {
                                warn!(path = %file.path, error = %e, "Failed to create unlinked media file");
                            }
                        }
                    }

                    // Update audiobook stats if we have an audiobook
                    if let Some(audiobook_id) = audiobook_id {
                        // Calculate total size from all files
                        let total_size: i64 = audiobook_files.iter().map(|f| f.size as i64).sum();
                        if let Err(e) = db.audiobooks().update_has_files(audiobook_id, true).await {
                            warn!(audiobook_id = %audiobook_id, error = %e, "Failed to update audiobook has_files");
                        }
                        let _ = total_size; // Suppress unused warning
                    }
                });

                handles.push(handle);
            }

            // Wait for all audiobooks in this chunk to complete
            for handle in handles {
                if let Err(e) = handle.await {
                    error!(error = %e, "Audiobook processing task panicked");
                }
            }

            processed_folders += chunk.len();
            info!(
                "Processed {}/{} audiobooks in '{}'",
                processed_folders, folder_count, progress.library_name
            );

            // Small delay between chunks
            if self.config.metadata_batch_delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.config.metadata_batch_delay_ms))
                    .await;
            }
        }

        // Update progress with final counts (reuse shows_added for audiobooks_added)
        progress.shows_added = audiobooks_added.load(Ordering::SeqCst);
        progress.scanned_files = scanned_files.load(Ordering::SeqCst);
        progress.episodes_linked = files_linked.load(Ordering::SeqCst);
        progress.new_files = new_files.load(Ordering::SeqCst);

        Ok(progress)
    }

    /// Extract audiobook info from folder name and files for searching
    fn extract_audiobook_info(folder_name: &str, files: &[DiscoveredFile]) -> String {
        // Try to read ID3 tags from first file for album/artist info
        if let Some(file) = files.first() {
            if let Some(meta) = Self::read_audio_metadata(&file.path) {
                // If we have album name, use it (often the audiobook title)
                if let Some(album) = meta.album {
                    // If we also have artist, include it (often the author)
                    if let Some(artist) = meta.artist {
                        return format!("{} {}", artist, album);
                    }
                    return album;
                }
                // If we have title, use it
                if let Some(title) = meta.title {
                    return title;
                }
            }
        }

        // Fall back to folder name, cleaning up common patterns
        let cleaned = folder_name
            .replace("_", " ")
            .replace("-", " ")
            .replace("  ", " ")
            .trim()
            .to_string();

        cleaned
    }

    /// Find or create an audiobook from OpenLibrary (static method for async context)
    async fn find_or_create_audiobook_static(
        db: &Database,
        metadata_service: &Arc<crate::services::metadata::MetadataService>,
        library_id: Uuid,
        user_id: Uuid,
        search_query: &str,
    ) -> Result<Option<(Uuid, bool)>> {
        // Search OpenLibrary
        let search_results = metadata_service.search_audiobooks(search_query).await?;

        let best_match = search_results.into_iter().next();

        let Some(result) = best_match else {
            return Ok(None);
        };

        // Check if audiobook already exists
        if let Some(existing) = db
            .audiobooks()
            .get_by_openlibrary_id(library_id, &result.provider_id)
            .await?
        {
            return Ok(Some((existing.id, false)));
        }

        // Add the audiobook from OpenLibrary
        let audiobook = metadata_service
            .add_audiobook_from_provider(AddAudiobookOptions {
                openlibrary_id: result.provider_id,
                library_id,
                user_id,
                monitored: true,
            })
            .await?;

        Ok(Some((audiobook.id, true)))
    }

    /// Read audio metadata (ID3 tags) from a file with verbose logging
    fn read_audio_metadata(path: &str) -> Option<AudioMetadata> {
        use lofty::prelude::*;
        use lofty::probe::Probe;
        use std::path::Path;

        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);

        let tagged_file = match Probe::open(path) {
            Ok(probe) => match probe.read() {
                Ok(tf) => tf,
                Err(e) => {
                    debug!("Failed to read tags from {}: {}", file_name, e);
                    return None;
                }
            },
            Err(e) => {
                debug!("Failed to open {} for tag reading: {}", file_name, e);
                return None;
            }
        };

        let tag = match tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
            Some(t) => t,
            None => {
                info!("No embedded tags: {}", file_name);
                return None;
            }
        };

        let metadata = AudioMetadata {
            artist: tag.artist().map(|s| s.to_string()),
            album: tag.album().map(|s| s.to_string()),
            title: tag.title().map(|s| s.to_string()),
            track_number: tag.track(),
            disc_number: tag.disk(),
            year: tag.year(),
            genre: tag.genre().map(|s| s.to_string()),
        };

        // Verbose logging of found metadata - inline readable format
        info!(
            "Read tags: {} | Artist: {} | Album: {} | Title: {} | Track: {}",
            file_name,
            metadata.artist.as_deref().unwrap_or("-"),
            metadata.album.as_deref().unwrap_or("-"),
            metadata.title.as_deref().unwrap_or("-"),
            metadata.track_number.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string())
        );

        Some(metadata)
    }

    /// Verify an existing music file's album link against its embedded metadata
    /// Returns true if the file is correctly linked, false if there's a mismatch
    /// 
    /// Now checks artist (most important), album, and track title.
    async fn verify_music_file_link(
        db: &Database,
        file_path: &str,
        existing_file: &crate::db::MediaFileRecord,
    ) -> Result<bool> {
        use super::filename_parser;

        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);

        // Read embedded metadata
        let metadata = match Self::read_audio_metadata(file_path) {
            Some(m) => m,
            None => {
                debug!("No metadata to verify: {}", file_name);
                return Ok(true); // Can't verify without metadata
            }
        };

        // Get the linked album if any
        let album_id = match existing_file.album_id {
            Some(id) => id,
            None => {
                let meta_album = metadata.album.as_deref().unwrap_or("unknown");
                info!("Unlinked file has metadata: {} (album: \"{}\")", file_name, meta_album);
                return Ok(false); // Mismatch - has metadata but no link
            }
        };

        // Get the album record
        let album = match db.albums().get_by_id(album_id).await? {
            Some(a) => a,
            None => {
                warn!("Linked album not found: {} -> album_id {}", file_name, album_id);
                return Ok(false);
            }
        };

        // Get the artist for this album
        let artist = db.albums().get_artist_by_id(album.artist_id).await?;
        let db_artist_name = artist.map(|a| a.name).unwrap_or_default();

        // CRITICAL: Check artist first - this is the most important differentiator
        // "Appetite for Destruction" by Guns N' Roses vs Xzibit are DIFFERENT albums
        if let Some(meta_artist) = metadata.artist.as_deref() {
            let artist_similarity = filename_parser::show_name_similarity(&db_artist_name, meta_artist);
            
            if artist_similarity < 0.5 {
                warn!(
                    "ARTIST MISMATCH: {} linked to \"{}\" by \"{}\" but tags say artist \"{}\" (similarity: {:.0}%)",
                    file_name, album.name, db_artist_name, meta_artist, artist_similarity * 100.0
                );
                return Ok(false);
            }
        }

        // Compare metadata album with linked album
        let meta_album = metadata.album.as_deref().unwrap_or("");
        let album_similarity = filename_parser::show_name_similarity(&album.name, meta_album);

        if album_similarity < 0.6 {
            warn!(
                "ALBUM MISMATCH: {} linked to \"{}\" but tags say \"{}\" (similarity: {:.0}%)",
                file_name, album.name, meta_album, album_similarity * 100.0
            );
            return Ok(false);
        }

        // Optional: Also check track title if linked to a track
        if let Some(track_id) = existing_file.track_id {
            if let Some(track) = db.tracks().get_by_id(track_id).await? {
                let meta_title = metadata.title.as_deref().unwrap_or("");
                let title_similarity = filename_parser::show_name_similarity(&track.title, meta_title);
                
                if title_similarity < 0.5 {
                    warn!(
                        "TRACK MISMATCH: {} linked to track \"{}\" but tags say \"{}\" (similarity: {:.0}%)",
                        file_name, track.title, meta_title, title_similarity * 100.0
                    );
                    return Ok(false);
                }
            }
        }

        info!(
            "Verified: {} -> \"{}\" by \"{}\" (album: {:.0}%, artist: match)",
            file_name, album.name, db_artist_name, album_similarity * 100.0
        );

        // CRITICAL: Clean up bidirectional relationship inconsistencies
        // If this file's track_id is correct, ensure the correct track has media_file_id pointing here
        // AND clear any OTHER tracks that incorrectly have media_file_id pointing to this file
        // Note: Status is derived from media_file_id presence, so we only update media_file_id
        if let Some(correct_track_id) = existing_file.track_id {
            use crate::db::sqlite_helpers::uuid_to_str;
            
            // 1. Clear any OTHER tracks that incorrectly point to this file
            let cleared = sqlx::query(
                "UPDATE tracks SET media_file_id = NULL WHERE media_file_id = ?1 AND id != ?2"
            )
            .bind(uuid_to_str(existing_file.id))
            .bind(uuid_to_str(correct_track_id))
            .execute(db.pool())
            .await?;

            if cleared.rows_affected() > 0 {
                info!(
                    "Cleared {} stale track->file references for '{}'",
                    cleared.rows_affected(), file_name
                );
            }

            // 2. Ensure the correct track has media_file_id pointing to this file
            sqlx::query(
                "UPDATE tracks SET media_file_id = ?1 WHERE id = ?2"
            )
            .bind(uuid_to_str(existing_file.id))
            .bind(uuid_to_str(correct_track_id))
            .execute(db.pool())
            .await?;
        }

        Ok(true)
    }

    /// Try to find and fix a mismatched music file by re-matching based on metadata
    async fn try_fix_music_file_link(
        db: &Database,
        library_id: Uuid,
        file_path: &str,
        existing_file: &crate::db::MediaFileRecord,
    ) -> Result<bool> {
        use super::file_matcher::{FileMatcher, FileMatchTarget};
        use crate::db::EmbeddedMetadata;

        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);

        // Never auto-correct manually matched files
        if existing_file.match_type.as_deref() == Some("manual") {
            debug!(
                file = %file_name,
                "Skipping auto-correction for manually matched file"
            );
            return Ok(false);
        }

        // Read embedded metadata
        let metadata = match Self::read_audio_metadata(file_path) {
            Some(m) if m.album.is_some() => m,
            _ => return Ok(false),
        };

        let meta_album = metadata.album.as_deref().unwrap_or("");
        let meta_artist = metadata.artist.as_deref().unwrap_or("");

        // Store the extracted metadata in the database first
        let embedded = EmbeddedMetadata {
            artist: metadata.artist.clone(),
            album: metadata.album.clone(),
            title: metadata.title.clone(),
            track_number: metadata.track_number.map(|n| n as i32),
            disc_number: None,
            year: metadata.year.map(|y| y as i32),
            genre: metadata.genre.clone(),
            show_name: None,
            season: None,
            episode: None,
            cover_art_base64: None, // Album art extracted by queues processor
            cover_art_mime: None,
            lyrics: None, // Lyrics extracted by queues processor
            ..Default::default()
        };

        db.media_files()
            .update_embedded_metadata(existing_file.id, &embedded)
            .await?;

        // Get library for FileMatcher
        let library = match db.libraries().get_by_id(library_id).await? {
            Some(lib) => lib,
            None => return Ok(false),
        };

        // Use FileMatcher to find the correct match
        let matcher = FileMatcher::new(db.clone());
        
        // Re-fetch the media file with updated metadata
        let updated_file = match db.media_files().get_by_id(existing_file.id).await? {
            Some(f) => f,
            None => return Ok(false),
        };

        match matcher.match_media_file(&updated_file, &library).await {
            Ok(result) => {
                if result.match_target.is_matched() {
                    // Extract the new album/track IDs from the match
                    if let FileMatchTarget::Track { track_id, album_id, title, .. } = &result.match_target {
                        // Check if this is different from current link
                        let different_album = existing_file.album_id.map(|id| id != *album_id).unwrap_or(true);
                        let different_track = existing_file.track_id.map(|id| id != *track_id).unwrap_or(true);

                        if different_album || different_track {
                            info!(
                                file = %file_name,
                                meta_album = %meta_album,
                                meta_artist = %meta_artist,
                                new_album_id = %album_id,
                                new_track_id = %track_id,
                                new_track_title = %title,
                                confidence = result.confidence,
                                "[FIX] Auto-correcting via FileMatcher"
                            );

                            // Update the OLD track: clear media_file_id
                            // Note: Status is derived from media_file_id presence
                            if let Some(old_track_id) = existing_file.track_id {
                                if old_track_id != *track_id {
                                    use crate::db::sqlite_helpers::uuid_to_str;
                                    sqlx::query("UPDATE tracks SET media_file_id = NULL WHERE id = ?1")
                                        .bind(uuid_to_str(old_track_id))
                                        .execute(db.pool())
                                        .await?;
                                    debug!(
                                        "Unlinked old track {}: media_file_id=NULL",
                                        old_track_id
                                    );
                                }
                            }

                            // Update the media file link
                            db.media_files()
                                .update_match(
                                    existing_file.id,
                                    None, // episode_id
                                    None, // movie_id
                                    Some(*track_id),
                                    Some(*album_id),
                                    None, // audiobook_id
                                    Some("track"),
                                )
                                .await?;

                            // Update the NEW track: set media_file_id
                            // Note: Status is derived from media_file_id presence
                            {
                                use crate::db::sqlite_helpers::uuid_to_str;
                                sqlx::query("UPDATE tracks SET media_file_id = ?1 WHERE id = ?2")
                                    .bind(uuid_to_str(existing_file.id))
                                    .bind(uuid_to_str(*track_id))
                                    .execute(db.pool())
                                    .await?;
                            }
                            debug!(
                                "Linked new track {}: media_file_id={}",
                                track_id, existing_file.id
                            );

                            return Ok(true);
                        }
                    }
                }
                
                warn!(
                    file = %file_name,
                    meta_album = %meta_album,
                    "[FIX] FileMatcher could not find matching album in library"
                );
                Ok(false)
            }
            Err(e) => {
                warn!(
                    file = %file_name,
                    error = %e,
                    "[FIX] FileMatcher error"
                );
                Ok(false)
            }
        }
    }

    /// Static version of find_or_create_album for use in spawned tasks
    async fn find_or_create_album_static(
        db: &Database,
        metadata_service: &Arc<MetadataService>,
        library_id: Uuid,
        user_id: Uuid,
        artist_name: &str,
        album_name: &str,
    ) -> Result<Option<(Uuid, bool)>> {
        let albums_repo = db.albums();

        // Build search query combining artist and album
        let query = format!("{} {}", artist_name, album_name);

        // Search for the album using metadata service
        let search_results = metadata_service.search_albums(&query).await?;

        if search_results.is_empty() {
            // Try album name only
            let retry_results = metadata_service.search_albums(album_name).await?;
            if retry_results.is_empty() {
                return Ok(None);
            }
            // Use first result from retry
            let best_match = &retry_results[0];

            // Check if we already have this album in the library
            if let Some(existing) = albums_repo
                .get_by_musicbrainz_id(library_id, best_match.provider_id)
                .await?
            {
                debug!("Album '{}' already exists in library", existing.name);
                return Ok(Some((existing.id, false)));
            }

            // Add the album from provider
            let album = metadata_service
                .add_album_from_provider(AddAlbumOptions {
                    musicbrainz_id: best_match.provider_id,
                    library_id,
                    user_id,
                    monitored: true,
                })
                .await?;

            return Ok(Some((album.id, true)));
        }

        // Get the best match
        let best_match = &search_results[0];

        // Check if we already have this album in the library
        if let Some(existing) = albums_repo
            .get_by_musicbrainz_id(library_id, best_match.provider_id)
            .await?
        {
            debug!("Album '{}' already exists in library", existing.name);
            return Ok(Some((existing.id, false)));
        }

        // Add the album from provider
        let album = metadata_service
            .add_album_from_provider(AddAlbumOptions {
                musicbrainz_id: best_match.provider_id,
                library_id,
                user_id,
                monitored: true,
            })
            .await?;

        Ok(Some((album.id, true)))
    }

    /// Static version of find_or_create_tv_show for use in spawned tasks
    async fn find_or_create_tv_show_static(
        db: &Database,
        metadata_service: &Arc<MetadataService>,
        library_id: Uuid,
        user_id: Uuid,
        show_name: &str,
        year: Option<u32>,
    ) -> Result<Option<(Uuid, bool)>> {
        let tv_shows_repo = db.tv_shows();

        // Build search query
        let mut query = show_name.to_string();
        if let Some(y) = year {
            query = format!("{} {}", query, y);
        }

        // Search for the show using metadata service
        let mut search_results = metadata_service.search_shows(&query).await?;

        if search_results.is_empty() {
            // Try without year
            search_results = metadata_service.search_shows(show_name).await?;
            if search_results.is_empty() {
                return Ok(None);
            }
        }

        // Get the best match
        let best_match = &search_results[0];

        // Check if we already have this show in the library
        if best_match.provider == MetadataProvider::TvMaze
            && let Some(existing) = tv_shows_repo
                .get_by_tvmaze_id(library_id, best_match.provider_id as i32)
                .await?
        {
            debug!("Show '{}' already exists in library", existing.name);
            return Ok(Some((existing.id, false)));
        }

        // Use the unified add_tv_show_from_provider method
        let tv_show = metadata_service
            .add_tv_show_from_provider(AddTvShowOptions {
                provider: best_match.provider,
                provider_id: best_match.provider_id,
                library_id,
                user_id,
                monitored: true,
                monitor_type: "all".to_string(),
                path: None,
            })
            .await?;

        Ok(Some((tv_show.id, true)))
    }

    /// Process a single file for a show
    ///
    /// Uses FileMatcher to find the matching episode, then FileProcessor to create
    /// media_file and set bidirectional links.
    async fn process_file_for_show(
        db: &Database,
        library_id: Uuid,
        tv_show_id: Uuid,
        file: &DiscoveredFile,
        episodes_linked: &Arc<AtomicI32>,
        new_files: &Arc<AtomicI32>,
        analysis_queue: Option<&Arc<MediaAnalysisQueue>>,
    ) -> Result<()> {
        // Check if file already exists and is properly linked
        let media_files_repo = db.media_files();
        if let Some(existing_file) = media_files_repo.get_by_path(&file.path).await? {
            if existing_file.episode_id.is_some() {
                // File already linked - but check if it needs analysis
                if existing_file.ffprobe_analyzed_at.is_none() {
                    // File was never analyzed - queue it now
                    if let Some(queue) = analysis_queue {
                        debug!(
                            path = %file.path,
                            media_file_id = %existing_file.id,
                            "File already linked but not analyzed (ffprobe_analyzed_at is null), queueing for analysis"
                        );
                        let job = MediaAnalysisJob {
                            media_file_id: existing_file.id,
                            path: std::path::PathBuf::from(&file.path),
                            check_subtitles: true,
                        };
                        if let Err(e) = queue.submit(job).await {
                            warn!(
                                media_file_id = %existing_file.id,
                                error = %e,
                                "Failed to queue existing file for analysis"
                            );
                        }
                    }
                }
                return Ok(());
            }
            // File exists but not linked - will be handled by FileProcessor below
            debug!(
                path = %file.path,
                "File exists in database but not linked to episode, will link now"
            );
        }

        // Get the library to use with FileMatcher
        let library = db.libraries().get_by_id(library_id).await?
            .context("Library not found")?;

        // Use FileMatcher to find the matching episode
        let file_matcher = FileMatcher::new(db.clone());
        let file_info = FileInfo {
            path: file.path.clone(),
            size: file.size as i64,
            file_index: None,
            source_name: None,
        };

        let filename = std::path::Path::new(&file.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file.path);

        // Match video file against the library
        let matches = file_matcher.match_video_file(&file_info, filename, &[library]).await?;

        // Find a match for this specific show
        let episode_match = matches.into_iter().find(|m| {
            if let FileMatchTarget::Episode { show_id, .. } = &m.match_target {
                show_id == &tv_show_id
            } else {
                false
            }
        });

        if let Some(result) = episode_match {
            if let FileMatchTarget::Episode { episode_id, season, episode, .. } = result.match_target {
                // Use FileProcessor to create media_file and set bidirectional links
                let file_processor = if let Some(queue) = analysis_queue {
                    FileProcessor::with_analysis_queue(db.clone(), queue.clone())
                } else {
                    FileProcessor::new(db.clone())
                };

                match file_processor
                    .link_existing_file(
                        &file.path,
                        file.size as i64,
                        library_id,
                        ProcessTarget::Episode(episode_id),
                    )
                    .await
                {
                    Ok(_media_file) => {
                        episodes_linked.fetch_add(1, Ordering::SeqCst);
                        new_files.fetch_add(1, Ordering::SeqCst);
                        debug!(
                            path = %file.path,
                            episode_id = %episode_id,
                            season = season,
                            episode = episode,
                            "Linked file to episode via FileMatcher + FileProcessor"
                        );
                    }
                    Err(e) => {
                        error!(path = %file.path, error = %e, "Failed to link file to episode");
                    }
                }
            }
        } else {
            // FileMatcher couldn't find a match - try creating a placeholder episode
            // based on parsed filename info, then link
            if let (Some(season), Some(episode)) = (file.parsed.season, file.parsed.episode) {
                let episodes_repo = db.episodes();
                let episode_id = if let Some(ep) = episodes_repo
                    .get_by_show_season_episode(tv_show_id, season as i32, episode as i32)
                    .await?
                {
                    Some(ep.id)
                } else {
                    // Create placeholder episode
                    match episodes_repo
                        .create(CreateEpisode {
                            tv_show_id,
                            season: season as i32,
                            episode: episode as i32,
                            absolute_number: None,
                            title: None,
                            overview: None,
                            air_date: None,
                            runtime: None,
                            tvmaze_id: None,
                            tmdb_id: None,
                            tvdb_id: None,
                        })
                        .await
                    {
                        Ok(ep) => Some(ep.id),
                        Err(e) => {
                            warn!(error = %e, "Failed to create placeholder episode");
                            None
                        }
                    }
                };

                if let Some(ep_id) = episode_id {
                    let file_processor = if let Some(queue) = analysis_queue {
                        FileProcessor::with_analysis_queue(db.clone(), queue.clone())
                    } else {
                        FileProcessor::new(db.clone())
                    };

                    match file_processor
                        .link_existing_file(
                            &file.path,
                            file.size as i64,
                            library_id,
                            ProcessTarget::Episode(ep_id),
                        )
                        .await
                    {
                        Ok(_media_file) => {
                            episodes_linked.fetch_add(1, Ordering::SeqCst);
                            new_files.fetch_add(1, Ordering::SeqCst);
                            debug!(
                                path = %file.path,
                                episode_id = %ep_id,
                                "Linked file to placeholder episode via FileProcessor"
                            );
                        }
                        Err(e) => {
                            error!(path = %file.path, error = %e, "Failed to link file to episode");
                        }
                    }
                } else {
                    // No episode to link to - create unlinked media file
                    Self::create_unlinked_media_file_static(db, library_id, file, new_files, analysis_queue)
                        .await?;
                }
            } else {
                // No season/episode parsed - create unlinked media file
                Self::create_unlinked_media_file_static(db, library_id, file, new_files, analysis_queue)
                    .await?;
            }
        }

        Ok(())
    }

    /// Create an unlinked media file record
    ///
    /// Used for files that could not be matched to any content item.
    async fn create_unlinked_media_file_static(
        db: &Database,
        library_id: Uuid,
        file: &DiscoveredFile,
        new_files: &Arc<AtomicI32>,
        analysis_queue: Option<&Arc<MediaAnalysisQueue>>,
    ) -> Result<()> {
        let create_input = CreateMediaFile {
            library_id,
            path: file.path.clone(),
            size_bytes: file.size as i64,
            container: Path::new(&file.path)
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase()),
            video_codec: file.parsed.codec.clone(),
            audio_codec: file.parsed.audio.clone(),
            width: None,
            height: None,
            duration: None,
            bitrate: None,
            file_hash: None,
            episode_id: None,
            movie_id: None,
            track_id: None,
            album_id: None,
            audiobook_id: None,
            chapter_id: None,
            relative_path: file.relative_path.clone(),
            original_name: Some(file.filename.clone()),
            resolution: file.parsed.resolution.clone(),
            is_hdr: file.parsed.hdr.is_some().then_some(true),
            hdr_type: file.parsed.hdr.clone(),
        };

        let media_files_repo = db.media_files();
        match media_files_repo.upsert(create_input).await {
            Ok(media_file) => {
                new_files.fetch_add(1, Ordering::SeqCst);
                debug!(path = %file.path, "Added unlinked media file");

                // Queue for FFmpeg analysis to get real metadata
                if let Some(queue) = analysis_queue {
                    let job = MediaAnalysisJob {
                        media_file_id: media_file.id,
                        path: std::path::PathBuf::from(&file.path),
                        check_subtitles: true,
                    };
                    if let Err(e) = queue.submit(job).await {
                        warn!(
                            media_file_id = %media_file.id,
                            error = %e,
                            "Failed to queue file for FFmpeg analysis"
                        );
                    } else {
                        debug!(
                            media_file_id = %media_file.id,
                            path = %file.path,
                            "Queued file for FFmpeg analysis"
                        );
                    }
                }
            }
            Err(e) => {
                error!(path = %file.path, error = %e, "Failed to create media file record");
            }
        }

        Ok(())
    }

    /// Process a single file for an album (music)
    ///
    /// Uses FileMatcher to find the matching track, then FileProcessor
    /// to create media_file and set bidirectional links.
    async fn process_file_for_album(
        db: &Database,
        library_id: Uuid,
        album_id: Uuid,
        file: &DiscoveredFile,
        meta: &AudioMetadata,
        files_linked: &Arc<AtomicI32>,
        new_files: &Arc<AtomicI32>,
        analysis_queue: Option<&Arc<MediaAnalysisQueue>>,
    ) -> Result<()> {
        // Check if file already exists and is properly linked
        let media_files_repo = db.media_files();
        if let Some(existing_file) = media_files_repo.get_by_path(&file.path).await? {
            if existing_file.track_id.is_some() {
                // File already linked - but check if it needs analysis
                if existing_file.ffprobe_analyzed_at.is_none() {
                    // File was never analyzed - queue it now
                    if let Some(queue) = analysis_queue {
                        debug!(
                            path = %file.path,
                            media_file_id = %existing_file.id,
                            "File already linked but not analyzed (ffprobe_analyzed_at is null), queueing for analysis"
                        );
                        let job = MediaAnalysisJob {
                            media_file_id: existing_file.id,
                            path: std::path::PathBuf::from(&file.path),
                            check_subtitles: false,
                        };
                        if let Err(e) = queue.submit(job).await {
                            warn!(
                                media_file_id = %existing_file.id,
                                error = %e,
                                "Failed to queue existing file for analysis"
                            );
                        }
                    }
                }
                return Ok(());
            }
            // File exists but not linked - will be handled by FileProcessor below
            debug!(
                path = %file.path,
                "File exists in database but not linked to track, will link now"
            );
        }

        // Get the library to use with FileMatcher
        let library = db.libraries().get_by_id(library_id).await?
            .context("Library not found")?;

        // Use FileMatcher to find the matching track
        let file_matcher = FileMatcher::new(db.clone());
        let file_info = FileInfo {
            path: file.path.clone(),
            size: file.size as i64,
            file_index: None,
            source_name: None,
        };

        let filename = std::path::Path::new(&file.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file.path);

        // Match audio file against the library
        let matches = file_matcher.match_audio_file(&file_info, filename, &[library]).await?;

        // Find a match for this specific album
        let track_match = matches.into_iter().find(|m| {
            if let FileMatchTarget::Track { album_id: matched_album_id, .. } = &m.match_target {
                matched_album_id == &album_id
            } else {
                false
            }
        });

        if let Some(result) = track_match {
            if let FileMatchTarget::Track { track_id, title, track_number, .. } = result.match_target {
                // Use FileProcessor to create media_file and set bidirectional links
                let file_processor = if let Some(queue) = analysis_queue {
                    FileProcessor::with_analysis_queue(db.clone(), queue.clone())
                } else {
                    FileProcessor::new(db.clone())
                };

                match file_processor
                    .link_existing_file(
                        &file.path,
                        file.size as i64,
                        library_id,
                        ProcessTarget::Track(track_id),
                    )
                    .await
                {
                    Ok(_media_file) => {
                        files_linked.fetch_add(1, Ordering::SeqCst);
                        new_files.fetch_add(1, Ordering::SeqCst);
                        debug!(
                            path = %file.path,
                            track_id = %track_id,
                            title = %title,
                            track_number = track_number,
                            "Linked file to track via FileMatcher + FileProcessor"
                        );
                    }
                    Err(e) => {
                        error!(path = %file.path, error = %e, "Failed to link file to track");
                    }
                }
            }
        } else {
            // FileMatcher couldn't find a match - try fallback by track number
            let track_id = if let Some(track_num) = meta.track_number {
                let tracks = db.tracks().list_by_album(album_id).await?;
                tracks
                    .iter()
                    .find(|t| t.track_number == track_num as i32)
                    .map(|t| t.id)
            } else {
                None
            };

            if let Some(track_id) = track_id {
                let file_processor = if let Some(queue) = analysis_queue {
                    FileProcessor::with_analysis_queue(db.clone(), queue.clone())
                } else {
                    FileProcessor::new(db.clone())
                };

                match file_processor
                    .link_existing_file(
                        &file.path,
                        file.size as i64,
                        library_id,
                        ProcessTarget::Track(track_id),
                    )
                    .await
                {
                    Ok(_media_file) => {
                        files_linked.fetch_add(1, Ordering::SeqCst);
                        new_files.fetch_add(1, Ordering::SeqCst);
                        debug!(
                            path = %file.path,
                            track_id = %track_id,
                            "Linked file to track via track number fallback"
                        );
                    }
                    Err(e) => {
                        error!(path = %file.path, error = %e, "Failed to link file to track");
                    }
                }
            } else {
                // No matching track found - create unlinked media file
                Self::create_unlinked_media_file_static(db, library_id, file, new_files, analysis_queue)
                    .await?;
            }
        }

        Ok(())
    }

    /// Process a single file for an audiobook
    ///
    /// Uses FileProcessor to create media_file and set bidirectional links to chapter.
    async fn process_file_for_audiobook(
        db: &Database,
        library_id: Uuid,
        audiobook_id: Uuid,
        file: &DiscoveredFile,
        chapter_number: i32,
        files_linked: &Arc<AtomicI32>,
        new_files: &Arc<AtomicI32>,
        analysis_queue: Option<&Arc<MediaAnalysisQueue>>,
    ) -> Result<()> {
        // Check if file already exists and is properly linked
        let media_files_repo = db.media_files();
        if let Some(existing_file) = media_files_repo.get_by_path(&file.path).await? {
            if existing_file.chapter_id.is_some() {
                // File already linked - but check if it needs analysis
                if existing_file.ffprobe_analyzed_at.is_none() {
                    // File was never analyzed - queue it now
                    if let Some(queue) = analysis_queue {
                        debug!(
                            path = %file.path,
                            media_file_id = %existing_file.id,
                            "File already linked but not analyzed (ffprobe_analyzed_at is null), queueing for analysis"
                        );
                        let job = MediaAnalysisJob {
                            media_file_id: existing_file.id,
                            path: std::path::PathBuf::from(&file.path),
                            check_subtitles: false,
                        };
                        if let Err(e) = queue.submit(job).await {
                            warn!(
                                media_file_id = %existing_file.id,
                                error = %e,
                                "Failed to queue existing file for analysis"
                            );
                        }
                    }
                }
                return Ok(());
            }
            // File exists but not linked - will be handled by FileProcessor below
            debug!(
                path = %file.path,
                "File exists in database but not linked to chapter, will link now"
            );
        }

        // Get the library to use with FileMatcher
        let library = db.libraries().get_by_id(library_id).await?
            .context("Library not found")?;

        // Use FileMatcher to find the matching chapter
        let file_matcher = FileMatcher::new(db.clone());
        let file_info = FileInfo {
            path: file.path.clone(),
            size: file.size as i64,
            file_index: None,
            source_name: None,
        };

        let filename = std::path::Path::new(&file.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file.path);

        // Match audio file against the library
        let matches = file_matcher.match_audio_file(&file_info, filename, &[library]).await?;

        // Find a match for this specific audiobook
        let chapter_match = matches.into_iter().find(|m| {
            if let FileMatchTarget::Chapter { audiobook_id: matched_audiobook_id, .. } = &m.match_target {
                matched_audiobook_id == &audiobook_id
            } else {
                false
            }
        });

        if let Some(result) = chapter_match {
            if let FileMatchTarget::Chapter { chapter_id, chapter_number: matched_chapter, .. } = result.match_target {
                // Use FileProcessor to create media_file and set bidirectional links
                let file_processor = if let Some(queue) = analysis_queue {
                    FileProcessor::with_analysis_queue(db.clone(), queue.clone())
                } else {
                    FileProcessor::new(db.clone())
                };

                match file_processor
                    .link_existing_file(
                        &file.path,
                        file.size as i64,
                        library_id,
                        ProcessTarget::Chapter(chapter_id),
                    )
                    .await
                {
                    Ok(_media_file) => {
                        files_linked.fetch_add(1, Ordering::SeqCst);
                        new_files.fetch_add(1, Ordering::SeqCst);
                        debug!(
                            path = %file.path,
                            chapter_id = %chapter_id,
                            chapter_number = matched_chapter,
                            "Linked file to chapter via FileMatcher + FileProcessor"
                        );
                    }
                    Err(e) => {
                        error!(path = %file.path, error = %e, "Failed to link file to chapter");
                    }
                }
            }
        } else {
            // FileMatcher couldn't find a match - use the passed chapter_number as fallback
            // Find or create the chapter
            let chapter_id = db.chapters()
                .get_or_create_by_number(audiobook_id, chapter_number)
                .await?
                .id;

            // Use FileProcessor to create media_file and set bidirectional links
            let file_processor = if let Some(queue) = analysis_queue {
                FileProcessor::with_analysis_queue(db.clone(), queue.clone())
            } else {
                FileProcessor::new(db.clone())
            };

            match file_processor
                .link_existing_file(
                    &file.path,
                    file.size as i64,
                    library_id,
                    ProcessTarget::Chapter(chapter_id),
                )
                .await
            {
                Ok(_media_file) => {
                    files_linked.fetch_add(1, Ordering::SeqCst);
                    new_files.fetch_add(1, Ordering::SeqCst);
                    debug!(
                        path = %file.path,
                        chapter_id = %chapter_id,
                        chapter_number = chapter_number,
                        "Linked file to chapter via chapter number fallback"
                    );
                }
                Err(e) => {
                    error!(path = %file.path, error = %e, "Failed to link file to chapter");
                }
            }
        }

        Ok(())
    }

    /// Simple file processing without show matching
    async fn process_files_simple(
        &self,
        library_id: Uuid,
        _library_path: &str,
        files: Vec<DiscoveredFile>,
        mut progress: ScanProgress,
    ) -> Result<ScanProgress> {
        let media_files_repo = self.db.media_files();

        for file in files {
            progress.scanned_files += 1;
            progress.current_file = Some(file.path.clone());

            if progress.scanned_files % 10 == 0 {
                let _ = self.progress_tx.send(progress.clone());
            }

            // Check if file already exists
            if media_files_repo.get_by_path(&file.path).await?.is_some() {
                debug!(path = %file.path, "File already in database, skipping");
                continue;
            }

            self.create_media_file(library_id, &file, &mut progress)
                .await?;
        }

        Ok(progress)
    }

    /// Add a file without linking to a show
    async fn add_unlinked_file(
        &self,
        library_id: Uuid,
        file: &DiscoveredFile,
        progress: &mut ScanProgress,
    ) -> Result<()> {
        progress.scanned_files += 1;
        progress.current_file = Some(file.path.clone());

        let media_files_repo = self.db.media_files();
        if let Some(existing) = media_files_repo.get_by_path(&file.path).await? {
            // File exists - check if it needs analysis
            if existing.ffprobe_analyzed_at.is_some() {
                debug!(path = %file.path, "File already analyzed, skipping");
            } else {
                // Needs analysis - queue it
                self.queue_analysis_for_existing(&existing).await;
            }
            return Ok(());
        }

        self.create_media_file(library_id, file, progress)
            .await
    }

    /// Queue analysis for an existing file that hasn't been analyzed yet
    async fn queue_analysis_for_existing(&self, media_file: &crate::db::MediaFileRecord) {
        if let Some(ref queue) = self.analysis_queue {
            let job = MediaAnalysisJob {
                media_file_id: media_file.id,
                path: std::path::PathBuf::from(&media_file.path),
                check_subtitles: true,
            };
            if let Err(e) = queue.submit(job).await {
                warn!(
                    media_file_id = %media_file.id,
                    error = %e,
                    "Failed to queue existing file for analysis"
                );
            } else {
                debug!(
                    media_file_id = %media_file.id,
                    path = %media_file.path,
                    "Queued existing file for analysis"
                );
            }
        }
    }

    /// Create a media file record (unlinked)
    ///
    /// This is only used for files that could not be matched to any content item.
    /// For linked files, use FileProcessor.link_existing_file() instead, which
    /// properly sets bidirectional links (content.media_file_id and media_file.{content}_id).
    async fn create_media_file(
        &self,
        library_id: Uuid,
        file: &DiscoveredFile,
        progress: &mut ScanProgress,
    ) -> Result<()> {
        let create_input = CreateMediaFile {
            library_id,
            path: file.path.clone(),
            size_bytes: file.size as i64,
            container: Path::new(&file.path)
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase()),
            video_codec: file.parsed.codec.clone(),
            audio_codec: file.parsed.audio.clone(),
            width: None,
            height: None,
            duration: None,
            bitrate: None,
            file_hash: None,
            // All content FKs are None - this is an unlinked file
            episode_id: None,
            movie_id: None,
            track_id: None,
            album_id: None,
            audiobook_id: None,
            chapter_id: None,
            relative_path: file.relative_path.clone(),
            original_name: Some(file.filename.clone()),
            resolution: file.parsed.resolution.clone(),
            is_hdr: file.parsed.hdr.is_some().then_some(true),
            hdr_type: file.parsed.hdr.clone(),
        };

        let media_files_repo = self.db.media_files();
        match media_files_repo.upsert(create_input).await {
            Ok(media_file) => {
                progress.new_files += 1;
                debug!(path = %file.path, "Added unlinked media file");

                // Queue for FFmpeg analysis to get real metadata
                if let Some(ref queue) = self.analysis_queue {
                    let job = MediaAnalysisJob {
                        media_file_id: media_file.id,
                        path: std::path::PathBuf::from(&file.path),
                        check_subtitles: true,
                    };
                    if let Err(e) = queue.submit(job).await {
                        warn!(
                            media_file_id = %media_file.id,
                            error = %e,
                            "Failed to queue file for FFmpeg analysis"
                        );
                    }
                }
            }
            Err(e) => {
                error!(path = %file.path, error = %e, "Failed to create media file record");
            }
        }

        Ok(())
    }

    /// Scan all libraries for a user
    pub async fn scan_all_for_user(&self, user_id: Uuid) -> Result<Vec<ScanProgress>> {
        let libraries = self.db.libraries().list_by_user(user_id).await?;
        let mut results = Vec::new();

        for library in libraries {
            match self.scan_library(library.id).await {
                Ok(progress) => results.push(progress),
                Err(e) => {
                    error!(library_id = %library.id, error = %e, "Failed to scan library");
                }
            }
        }

        Ok(results)
    }

    /// Scan all libraries (for scheduled job)
    pub async fn scan_all_libraries(&self) -> Result<()> {
        use crate::db::sqlite_helpers::str_to_uuid;
        let pool = self.db.pool();

        let library_id_strs: Vec<String> =
            sqlx::query_scalar("SELECT id FROM libraries WHERE auto_scan = true")
                .fetch_all(pool)
                .await?;

        let library_ids: Vec<Uuid> = library_id_strs
            .iter()
            .filter_map(|s| str_to_uuid(s).ok())
            .collect();

        info!(
            count = library_ids.len(),
            "Scanning libraries with auto_scan enabled"
        );

        for library_id in library_ids {
            if let Err(e) = self.scan_library(library_id).await {
                error!(library_id = %library_id, error = %e, "Library scan failed");
            }
        }

        Ok(())
    }

    /// Organize library files into proper folder structure
    ///
    /// This method:
    /// 1. Gets all unorganized files linked to episodes
    /// 2. For each file, checks if the show has organize_files enabled (respecting overrides)
    /// 3. Creates the show/season folder structure
    /// 4. Moves the file to the correct location
    /// 5. Optionally renames the file based on rename_style setting
    async fn organize_library_files(&self, library_id: Uuid) -> Result<()> {
        let organizer = OrganizerService::new(self.db.clone());

        // Get all shows in this library that have episodes with unorganized files
        let shows = self.db.tv_shows().list_by_library(library_id).await?;

        for show in shows {
            // Check if this show should be organized (respecting overrides)
            let (organize_enabled, _rename_style) =
                organizer.get_show_organize_settings(&show).await?;

            if !organize_enabled {
                debug!(
                    show_id = %show.id,
                    show_name = %show.name,
                    "Show has organize_files disabled (via override), skipping"
                );
                continue;
            }

            // Create folder structure for this show
            if let Err(e) = organizer.create_show_folders(show.id).await {
                warn!(
                    show_id = %show.id,
                    error = %e,
                    "Failed to create folder structure for show"
                );
                continue;
            }
        }

        // Step 1: Deduplicate - remove duplicate files for the same episode
        // This deletes both the file from disk and the database record
        match organizer.deduplicate_library(library_id).await {
            Ok(dedup) => {
                if dedup.duplicates_removed > 0 {
                    info!(
                        library_id = %library_id,
                        duplicates_removed = dedup.duplicates_removed,
                        files_deleted = dedup.files_deleted,
                        "Deduplication complete"
                    );
                }
            }
            Err(e) => {
                warn!(
                    library_id = %library_id,
                    error = %e,
                    "Failed to deduplicate library"
                );
            }
        }

        // Step 2: Organize - move/rename files to correct locations
        let results = organizer.organize_library(library_id).await?;

        let success_count = results.iter().filter(|r| r.success).count();
        let error_count = results.iter().filter(|r| !r.success).count();

        if !results.is_empty() {
            info!(
                library_id = %library_id,
                total = results.len(),
                success = success_count,
                errors = error_count,
                "Library organization complete"
            );
        }

        // Step 3: Clean up orphan files (legacy links left behind after organization)
        match organizer.cleanup_orphan_files(library_id).await {
            Ok(cleanup) => {
                if cleanup.folders_removed > 0 {
                    info!(
                        library_id = %library_id,
                        orphan_files_deleted = cleanup.folders_removed,
                        "Orphan file cleanup complete"
                    );
                }
            }
            Err(e) => {
                warn!(
                    library_id = %library_id,
                    error = %e,
                    "Failed to clean up orphan files"
                );
            }
        }

        // Step 4: Clean up empty folders (library-type aware, protects registered show/season folders)
        match organizer.cleanup_empty_folders(library_id).await {
            Ok(cleanup) => {
                if cleanup.folders_removed > 0 {
                    info!(
                        library_id = %library_id,
                        folders_removed = cleanup.folders_removed,
                        "Empty folder cleanup complete"
                    );
                }
            }
            Err(e) => {
                warn!(
                    library_id = %library_id,
                    error = %e,
                    "Failed to clean up empty folders"
                );
            }
        }

        Ok(())
    }
}

/// Create a shared scanner service with default config
pub fn create_scanner_service(
    db: Database,
    metadata_service: Arc<MetadataService>,
) -> Arc<ScannerService> {
    Arc::new(ScannerService::new(db, metadata_service))
}

/// Create a shared scanner service with custom config
pub fn create_scanner_service_with_config(
    db: Database,
    metadata_service: Arc<MetadataService>,
    config: ScannerConfig,
) -> Arc<ScannerService> {
    Arc::new(ScannerService::with_config(db, metadata_service, config))
}
