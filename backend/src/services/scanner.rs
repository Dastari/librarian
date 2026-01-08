//! Library scanner service
//!
//! Walks library directories to discover media files, parse filenames,
//! identify TV shows, and update the database.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::db::{CreateEpisode, CreateMediaFile, Database};
use super::filename_parser::{self, ParsedEpisode};
use super::metadata::{AddTvShowOptions, MetadataProvider, MetadataService};

/// Video file extensions we recognize
const VIDEO_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "m4v", "mov", "wmv", "flv", "webm", "mpeg", "mpg", "ts", "m2ts",
];

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
#[derive(Debug)]
struct DiscoveredFile {
    path: String,
    size: u64,
    filename: String,
    parsed: ParsedEpisode,
    relative_path: Option<String>,
}

/// Scanner service for discovering media files
pub struct ScannerService {
    db: Database,
    metadata_service: Arc<MetadataService>,
    progress_tx: broadcast::Sender<ScanProgress>,
}

impl ScannerService {
    /// Create a new scanner service
    pub fn new(db: Database, metadata_service: Arc<MetadataService>) -> Self {
        let (progress_tx, _) = broadcast::channel(100);
        Self { db, metadata_service, progress_tx }
    }

    /// Subscribe to scan progress updates
    pub fn subscribe(&self) -> broadcast::Receiver<ScanProgress> {
        self.progress_tx.subscribe()
    }

    /// Scan a specific library
    pub async fn scan_library(&self, library_id: Uuid) -> Result<ScanProgress> {
        // Get library info
        let library = self.db.libraries()
            .get_by_id(library_id)
            .await?
            .context("Library not found")?;

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

        // First pass: collect all video files
        let mut video_files: Vec<DiscoveredFile> = Vec::new();
        
        for entry in WalkDir::new(library_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                        let path_str = path.to_string_lossy().to_string();
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        let filename = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();
                        let parsed = filename_parser::parse_episode(&filename);
                        let relative_path = path.strip_prefix(library_path)
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
            progress = self.process_tv_library_with_auto_add(
                library_id,
                library.user_id,
                video_files,
                progress,
            ).await?;
        } else {
            // Simple processing - just add files without show matching
            progress = self.process_files_simple(
                library_id,
                &library.path,
                video_files,
                progress,
            ).await?;
        }

        // Note: File removal is handled in process_* methods
        // The scan tracks new files added; removed files would need a separate cleanup job

        // Update library last_scanned_at
        self.db.libraries().update_last_scanned(library_id).await?;

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
    async fn process_tv_library_with_auto_add(
        &self,
        library_id: Uuid,
        user_id: Uuid,
        files: Vec<DiscoveredFile>,
        mut progress: ScanProgress,
    ) -> Result<ScanProgress> {
        // Group files by parsed show name
        let mut files_by_show: HashMap<String, Vec<DiscoveredFile>> = HashMap::new();
        
        for file in files {
            if let Some(ref show_name) = file.parsed.show_name {
                let normalized = show_name.to_lowercase();
                files_by_show.entry(normalized).or_default().push(file);
            } else {
                // No show name parsed, add without linking
                self.add_unlinked_file(library_id, &file, &mut progress).await?;
            }
        }

        info!(show_groups = files_by_show.len(), "Grouped files by show name");

        // Process each show group
        for (_normalized_name, show_files) in files_by_show {
            if show_files.is_empty() {
                continue;
            }

            // Get the original show name from the first file
            let show_name = show_files[0].parsed.show_name.clone().unwrap_or_default();
            let year = show_files[0].parsed.year;

            info!(show_name = %show_name, file_count = show_files.len(), "Processing show group");

            // Try to find or create the TV show
            let tv_show_id = match self.find_or_create_tv_show(
                library_id,
                user_id,
                &show_name,
                year,
            ).await {
                Ok(Some((id, is_new))) => {
                    if is_new {
                        progress.shows_added += 1;
                    }
                    id
                }
                Ok(None) => {
                    warn!(show_name = %show_name, "Could not find show in metadata providers");
                    // Add files without linking
                    for file in show_files {
                        self.add_unlinked_file(library_id, &file, &mut progress).await?;
                    }
                    continue;
                }
                Err(e) => {
                    error!(show_name = %show_name, error = %e, "Error finding/creating show");
                    for file in show_files {
                        self.add_unlinked_file(library_id, &file, &mut progress).await?;
                    }
                    continue;
                }
            };

            // Process each file for this show
            for file in show_files {
                progress.scanned_files += 1;
                progress.current_file = Some(file.path.clone());

                if progress.scanned_files % 10 == 0 {
                    let _ = self.progress_tx.send(progress.clone());
                }

                // Check if file already exists
                let media_files_repo = self.db.media_files();
                if let Some(existing_file) = media_files_repo.get_by_path(&file.path).await? {
                    // File exists - check if it needs to be linked to an episode
                    if existing_file.episode_id.is_none() {
                        if let (Some(season), Some(episode)) = (file.parsed.season, file.parsed.episode) {
                            if let Ok(Some(ep_id)) = self.find_or_create_episode(tv_show_id, season as i32, episode as i32).await {
                                // Link the existing file to the episode
                                if let Err(e) = media_files_repo.link_to_episode(existing_file.id, ep_id).await {
                                    warn!(error = %e, "Failed to link existing file to episode");
                                } else {
                                    info!(path = %file.path, season = season, episode = episode, "Linked existing file to episode");
                                    progress.episodes_linked += 1;
                                    
                                    // Mark episode as downloaded
                                    let episodes_repo = self.db.episodes();
                                    if let Err(e) = episodes_repo.mark_downloaded(ep_id, existing_file.id).await {
                                        warn!(error = %e, "Failed to mark episode as downloaded");
                                    }
                                }
                            }
                        }
                    } else {
                        debug!(path = %file.path, "File already linked to episode, skipping");
                    }
                    continue;
                }

                // Try to link to an episode
                let episode_id = if let (Some(season), Some(episode)) = (file.parsed.season, file.parsed.episode) {
                    match self.find_or_create_episode(tv_show_id, season as i32, episode as i32).await {
                        Ok(Some(ep_id)) => {
                            progress.episodes_linked += 1;
                            Some(ep_id)
                        }
                        Ok(None) => None,
                        Err(e) => {
                            warn!(error = %e, "Failed to find/create episode");
                            None
                        }
                    }
                } else {
                    None
                };

                // Create media file record
                self.create_media_file(library_id, &file, episode_id, &mut progress).await?;
            }

            // Update show stats after processing all files
            if let Err(e) = self.db.tv_shows().update_stats(tv_show_id).await {
                warn!(tv_show_id = %tv_show_id, error = %e, "Failed to update show stats");
            }
        }

        Ok(progress)
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

            self.create_media_file(library_id, &file, None, &mut progress).await?;
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

        self.create_media_file(library_id, file, None, progress).await
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
            }
            Err(e) => {
                error!(path = %file.path, error = %e, "Failed to create media file record");
            }
        }

        Ok(())
    }

    /// Find or create a TV show, returns (show_id, is_new)
    /// 
    /// This method searches for a show by name, checks if it already exists
    /// in the library, and if not, creates it using the unified
    /// `add_tv_show_from_provider` method from the metadata service.
    async fn find_or_create_tv_show(
        &self,
        library_id: Uuid,
        user_id: Uuid,
        show_name: &str,
        year: Option<u32>,
    ) -> Result<Option<(Uuid, bool)>> {
        let tv_shows_repo = self.db.tv_shows();

        // Build search query
        let mut query = show_name.to_string();
        if let Some(y) = year {
            query = format!("{} {}", query, y);
        }

        // Search for the show using metadata service
        let mut search_results = self.metadata_service.search_shows(&query).await?;
        
        if search_results.is_empty() {
            // Try without year
            search_results = self.metadata_service.search_shows(show_name).await?;
            if search_results.is_empty() {
                return Ok(None);
            }
        }

        // Get the best match
        let best_match = &search_results[0];
        
        // Check if we already have this show in the library
        if best_match.provider == MetadataProvider::TvMaze {
            if let Some(existing) = tv_shows_repo
                .get_by_tvmaze_id(library_id, best_match.provider_id as i32)
                .await?
            {
                info!(show_name = %existing.name, "Show already exists in library");
                return Ok(Some((existing.id, false)));
            }
        }

        // Use the unified add_tv_show_from_provider method which handles:
        // - Creating the TV show record with normalized status
        // - Fetching and creating all episodes
        // - Updating show statistics
        let tv_show = self.metadata_service
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

    /// Find or create an episode, returns episode ID if found/created
    async fn find_or_create_episode(
        &self,
        tv_show_id: Uuid,
        season: i32,
        episode: i32,
    ) -> Result<Option<Uuid>> {
        let episodes_repo = self.db.episodes();

        // Try to find existing episode
        if let Some(ep) = episodes_repo
            .get_by_show_season_episode(tv_show_id, season, episode)
            .await?
        {
            return Ok(Some(ep.id));
        }

        // Episode doesn't exist - this shouldn't happen if we populated correctly,
        // but let's create a placeholder
        warn!(
            tv_show_id = %tv_show_id,
            season = season,
            episode = episode,
            "Episode not found, creating placeholder"
        );

        let ep = episodes_repo.create(CreateEpisode {
            tv_show_id,
            season,
            episode,
            absolute_number: None,
            title: None,
            overview: None,
            air_date: None,
            runtime: None,
            tvmaze_id: None,
            tmdb_id: None,
            tvdb_id: None,
            status: Some("downloaded".to_string()), // We have the file
        }).await?;

        Ok(Some(ep.id))
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
        
        let library_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM libraries WHERE auto_scan = true"
        )
        .fetch_all(pool)
        .await?;

        info!(count = library_ids.len(), "Scanning libraries with auto_scan enabled");

        for library_id in library_ids {
            if let Err(e) = self.scan_library(library_id).await {
                error!(library_id = %library_id, error = %e, "Library scan failed");
            }
        }

        Ok(())
    }
}

/// Create a shared scanner service
pub fn create_scanner_service(db: Database, metadata_service: Arc<MetadataService>) -> Arc<ScannerService> {
    Arc::new(ScannerService::new(db, metadata_service))
}
