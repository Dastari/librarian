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
use tokio::sync::{broadcast, Semaphore};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

use super::filename_parser::{self, ParsedEpisode};
use super::metadata::{AddTvShowOptions, MetadataProvider, MetadataService};
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
const AUDIOBOOK_EXTENSIONS: &[&str] = &[
    "mp3", "m4a", "m4b", "aac", "ogg", "opus", "flac",
];

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

/// Scanner service for discovering media files
use crate::graphql::{Library as GqlLibrary, LibraryChangedEvent, LibraryChangeType};

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
            library_changed_tx: None,
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
        // Get library info
        let library = self
            .db
            .libraries()
            .get_by_id(library_id)
            .await?
            .context("Library not found")?;

        // Check if already scanning
        if library.scanning {
            warn!(library_id = %library_id, "Scan already in progress, skipping");
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
            library_id = %library_id,
            path = %library.path,
            library_type = %library.library_type,
            auto_add_discovered = library.auto_add_discovered,
            "Starting library scan"
        );

        let library_path = Path::new(&library.path);
        if !library_path.exists() {
            warn!(path = %library.path, "Library path does not exist");
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
            info!(library_id = %library_id, "Running pre-scan consolidation");
            let organizer = OrganizerService::new(self.db.clone());
            match organizer.consolidate_library(library_id).await {
                Ok(result) => {
                    if result.files_moved > 0 || result.folders_removed > 0 {
                        info!(
                            library_id = %library_id,
                            files_moved = result.files_moved,
                            folders_removed = result.folders_removed,
                            "Pre-scan consolidation complete"
                        );
                    }
                }
                Err(e) => {
                    warn!(library_id = %library_id, error = %e, "Pre-scan consolidation failed");
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
        info!(total = total_files, "Found video files to scan");

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

        // Check if this is a TV library with auto_add_discovered enabled
        let is_tv_library = library.library_type == "tv";
        let auto_add = library.auto_add_discovered;

        // If auto-add is enabled for a TV library, group files by show name and process
        if is_tv_library && auto_add {
            progress = self
                .process_tv_library_with_auto_add(
                    library_id,
                    library.user_id,
                    video_files,
                    progress,
                )
                .await?;
        } else {
            // Simple processing - just add files without show matching
            progress = self
                .process_files_simple(library_id, &library.path, video_files, progress)
                .await?;
        }

        // Note: File removal is handled in process_* methods
        // The scan tracks new files added; removed files would need a separate cleanup job

        // Update library last_scanned_at
        self.db.libraries().update_last_scanned(library_id).await?;

        // Auto-organize files if the library has organize_files enabled
        if library.organize_files && is_tv_library {
            info!(library_id = %library_id, "Running automatic file organization");
            if let Err(e) = self.organize_library_files(library_id).await {
                error!(library_id = %library_id, error = %e, "Failed to organize library files");
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
            "Library scan completed"
        );

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
            
            for (normalized_name, show_files) in chunk {
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

                    info!(show_name = %show_name, file_count = show_files.len(), "Processing show group");

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
                                if let Err(e) = Self::create_media_file_static(
                                    &db,
                                    library_id,
                                    &file,
                                    None,
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
                processed = processed_shows,
                total = show_count,
                "Processed show chunk"
            );

            // Small delay between chunks
            if self.config.metadata_batch_delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.config.metadata_batch_delay_ms)).await;
            }
        }

        // Update progress with final counts
        progress.shows_added = shows_added.load(Ordering::SeqCst);
        progress.scanned_files = scanned_files.load(Ordering::SeqCst);
        progress.episodes_linked = episodes_linked.load(Ordering::SeqCst);
        progress.new_files = new_files.load(Ordering::SeqCst);

        Ok(progress)
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
            info!(show_name = %existing.name, "Show already exists in library");
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
                quality_profile_id: None,
                path: None,
            })
            .await?;

        Ok(Some((tv_show.id, true)))
    }

    /// Process a single file for a show
    async fn process_file_for_show(
        db: &Database,
        library_id: Uuid,
        tv_show_id: Uuid,
        file: &DiscoveredFile,
        episodes_linked: &Arc<AtomicI32>,
        new_files: &Arc<AtomicI32>,
        analysis_queue: Option<&Arc<MediaAnalysisQueue>>,
    ) -> Result<()> {
        let media_files_repo = db.media_files();

        // Check if file already exists
        if let Some(existing_file) = media_files_repo.get_by_path(&file.path).await? {
            // File exists - check if it needs to be linked to an episode
            if existing_file.episode_id.is_none() {
                if let (Some(season), Some(episode)) = (file.parsed.season, file.parsed.episode) {
                    let episodes_repo = db.episodes();
                    if let Some(ep) = episodes_repo
                        .get_by_show_season_episode(tv_show_id, season as i32, episode as i32)
                        .await?
                    {
                        // Link the existing file to the episode
                        media_files_repo.link_to_episode(existing_file.id, ep.id).await?;
                        info!(path = %file.path, season = season, episode = episode, "Linked existing file to episode");
                        episodes_linked.fetch_add(1, Ordering::SeqCst);

                        // Mark episode as downloaded
                        episodes_repo.mark_downloaded(ep.id, existing_file.id).await?;
                    }
                }
            } else {
                debug!(path = %file.path, "File already linked to episode, skipping");
            }
            return Ok(());
        }

        // Try to link to an episode
        let episode_id = if let (Some(season), Some(episode)) = (file.parsed.season, file.parsed.episode) {
            let episodes_repo = db.episodes();
            if let Some(ep) = episodes_repo
                .get_by_show_season_episode(tv_show_id, season as i32, episode as i32)
                .await?
            {
                episodes_linked.fetch_add(1, Ordering::SeqCst);
                Some(ep.id)
            } else {
                // Try to create placeholder episode
                match episodes_repo.create(CreateEpisode {
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
                    status: Some("downloaded".to_string()),
                }).await {
                    Ok(ep) => {
                        episodes_linked.fetch_add(1, Ordering::SeqCst);
                        Some(ep.id)
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to create placeholder episode");
                        None
                    }
                }
            }
        } else {
            None
        };

        // Create media file record
        Self::create_media_file_static(db, library_id, file, episode_id, new_files, analysis_queue).await
    }

    /// Static version of create_media_file for use in spawned tasks
    async fn create_media_file_static(
        db: &Database,
        library_id: Uuid,
        file: &DiscoveredFile,
        episode_id: Option<Uuid>,
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
            episode_id,
            movie_id: None,
            relative_path: file.relative_path.clone(),
            original_name: Some(file.filename.clone()),
            resolution: file.parsed.resolution.clone(),
            is_hdr: file.parsed.hdr.is_some().then_some(true),
            hdr_type: file.parsed.hdr.clone(),
        };

        let media_files_repo = db.media_files();
        match media_files_repo.create(create_input).await {
            Ok(media_file) => {
                new_files.fetch_add(1, Ordering::SeqCst);
                debug!(path = %file.path, "Added new media file");

                // If linked to an episode, mark it as downloaded
                if let Some(ep_id) = episode_id {
                    let episodes_repo = db.episodes();
                    if let Err(e) = episodes_repo.mark_downloaded(ep_id, media_file.id).await {
                        warn!(episode_id = %ep_id, error = %e, "Failed to mark episode as downloaded");
                    }
                }

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

            self.create_media_file(library_id, &file, None, &mut progress)
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
        if media_files_repo.get_by_path(&file.path).await?.is_some() {
            debug!(path = %file.path, "File already in database, skipping");
            return Ok(());
        }

        self.create_media_file(library_id, file, None, progress)
            .await
    }

    /// Create a media file record
    async fn create_media_file(
        &self,
        library_id: Uuid,
        file: &DiscoveredFile,
        episode_id: Option<Uuid>,
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
            episode_id,
            movie_id: None,
            relative_path: file.relative_path.clone(),
            original_name: Some(file.filename.clone()),
            resolution: file.parsed.resolution.clone(),
            is_hdr: file.parsed.hdr.is_some().then_some(true),
            hdr_type: file.parsed.hdr.clone(),
        };

        let media_files_repo = self.db.media_files();
        match media_files_repo.create(create_input).await {
            Ok(media_file) => {
                progress.new_files += 1;
                debug!(path = %file.path, "Added new media file");

                // If linked to an episode, mark it as downloaded
                if let Some(ep_id) = episode_id {
                    let episodes_repo = self.db.episodes();
                    if let Err(e) = episodes_repo.mark_downloaded(ep_id, media_file.id).await {
                        warn!(episode_id = %ep_id, error = %e, "Failed to mark episode as downloaded");
                    }
                }

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
        let pool = self.db.pool();

        let library_ids: Vec<Uuid> =
            sqlx::query_scalar("SELECT id FROM libraries WHERE auto_scan = true")
                .fetch_all(pool)
                .await?;

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

        // Step 3: Clean up orphan files (hardlinks left behind after organization)
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
