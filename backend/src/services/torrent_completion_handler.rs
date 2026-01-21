//! Torrent completion handler
//!
//! Subscribes to TorrentEvent::Completed events and immediately processes
//! completed torrents. This provides near-instant processing instead of
//! waiting for the 1-minute cron job.
//!
//! The handler:
//! 1. Listens for TorrentEvent::Completed broadcasts
//! 2. Retrieves the torrent's file matches from the database
//! 3. Organizes each matched file according to library settings
//! 4. Updates item statuses from 'downloading' to 'downloaded'
//! 5. Queues files for FFmpeg analysis

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::db::Database;
use crate::services::MetadataService;
use crate::services::queues::MediaAnalysisQueue;
use crate::services::torrent::{TorrentEvent, TorrentService};

/// Handler configuration
#[derive(Debug, Clone)]
pub struct CompletionHandlerConfig {
    /// Maximum number of concurrent torrent processing tasks
    pub max_concurrent: usize,
}

impl Default for CompletionHandlerConfig {
    fn default() -> Self {
        Self { max_concurrent: 3 }
    }
}

/// Torrent completion handler
///
/// Spawns a background task that listens for completion events
/// and processes torrents immediately.
pub struct TorrentCompletionHandler {
    db: Database,
    torrent_service: Arc<TorrentService>,
    analysis_queue: Option<Arc<MediaAnalysisQueue>>,
    metadata_service: Option<Arc<MetadataService>>,
    config: CompletionHandlerConfig,
}

impl TorrentCompletionHandler {
    pub fn new(
        db: Database,
        torrent_service: Arc<TorrentService>,
        config: CompletionHandlerConfig,
    ) -> Self {
        Self {
            db,
            torrent_service,
            analysis_queue: None,
            metadata_service: None,
            config,
        }
    }

    /// Add analysis queue for FFmpeg metadata extraction
    pub fn with_analysis_queue(mut self, queue: Arc<MediaAnalysisQueue>) -> Self {
        self.analysis_queue = Some(queue);
        self
    }

    /// Add metadata service for auto-adding discovered content
    pub fn with_metadata_service(mut self, service: Arc<MetadataService>) -> Self {
        self.metadata_service = Some(service);
        self
    }

    /// Start the completion handler
    ///
    /// Spawns a background task that processes completion events.
    /// Returns a handle that can be used to stop the handler.
    pub fn start(self) -> CompletionHandlerHandle {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let handle = tokio::spawn(self.run(shutdown_rx));

        CompletionHandlerHandle {
            shutdown_tx: Some(shutdown_tx),
            task_handle: Some(handle),
        }
    }

    /// Main run loop
    async fn run(self, mut shutdown_rx: tokio::sync::oneshot::Receiver<()>) {
        info!("Torrent completion handler started");

        // Subscribe to torrent events
        let mut event_rx = self.torrent_service.subscribe();

        // Semaphore for limiting concurrent processing
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent));

        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = &mut shutdown_rx => {
                    info!("Torrent completion handler shutting down");
                    break;
                }

                // Handle torrent events
                event = event_rx.recv() => {
                    match event {
                        Ok(TorrentEvent::Completed { id: _, info_hash, name }) => {
                            info!("Download finished for '{}', starting post-download processing", name);

                            // Acquire semaphore permit
                            let permit = match semaphore.clone().try_acquire_owned() {
                                Ok(permit) => permit,
                                Err(_) => {
                                    warn!(
                                        "Too many torrents processing at once, '{}' will be processed by scheduled job",
                                        name
                                    );
                                    continue;
                                }
                            };

                            // Spawn processing task
                            let db = self.db.clone();
                            let torrent_service = self.torrent_service.clone();
                            let analysis_queue = self.analysis_queue.clone();
                            let metadata_service = self.metadata_service.clone();
                            let info_hash_clone = info_hash.clone();
                            let name_clone = name.clone();

                            tokio::spawn(async move {
                                let _permit = permit;

                                if let Err(e) = process_completed_torrent(
                                    db,
                                    torrent_service,
                                    &info_hash_clone,
                                    analysis_queue,
                                    metadata_service,
                                    "download completed",
                                ).await {
                                    error!(
                                        "Failed to process '{}': {}",
                                        name_clone, e
                                    );
                                }
                            });
                        }
                        Ok(TorrentEvent::Added { id: _, name, info_hash }) => {
                            info!("New torrent added: '{}', matching files to library items", name);

                            // Spawn matching task
                            let db = self.db.clone();
                            let torrent_service = self.torrent_service.clone();
                            let info_hash_clone = info_hash.clone();
                            let name_clone = name.clone();

                            tokio::spawn(async move {
                                if let Err(e) = match_torrent_files_on_add(
                                    db,
                                    torrent_service,
                                    &info_hash_clone,
                                ).await {
                                    error!(
                                        "Failed to match files in torrent '{}': {}",
                                        name_clone, e
                                    );
                                }
                            });
                        }
                        Ok(_) => {
                            // Ignore other events
                        }
                        Err(broadcast::error::RecvError::Lagged(count)) => {
                            warn!(
                                lagged_count = count,
                                "Completion handler lagged behind, some events may have been missed"
                            );
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            info!("Torrent event channel closed, stopping handler");
                            break;
                        }
                    }
                }
            }
        }
    }
}

/// Handle for controlling the completion handler
pub struct CompletionHandlerHandle {
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl CompletionHandlerHandle {
    /// Stop the completion handler
    pub async fn stop(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.task_handle.take() {
            let _ = handle.await;
        }
    }
}

impl Drop for CompletionHandlerHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        // Note: we can't await the task handle in Drop, so we just abort it
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }
}

/// Process a completed torrent (using new source-agnostic FileProcessor)
async fn process_completed_torrent(
    db: Database,
    _torrent_service: Arc<TorrentService>,
    info_hash: &str,
    analysis_queue: Option<Arc<MediaAnalysisQueue>>,
    _metadata_service: Option<Arc<MetadataService>>,
    trigger_reason: &str,
) -> Result<()> {
    use crate::services::file_matcher::FileMatcher;
    use crate::services::file_processor::FileProcessor;

    // Get torrent record
    let torrent = match db.torrents().get_by_info_hash(info_hash).await? {
        Some(t) => t,
        None => {
            warn!("Torrent {} not found in database", info_hash);
            return Ok(());
        }
    };

    info!(
        "Processing '{}' (triggered by {})",
        torrent.name, trigger_reason
    );

    // NEW: Verify existing matches using embedded metadata before processing
    // This corrects any mismatches from pre-download filename matching
    let matcher = FileMatcher::new(db.clone());
    match matcher.verify_matches_with_metadata("torrent", torrent.id).await {
        Ok(verification) => {
            if verification.corrected > 0 || verification.flagged > 0 {
                info!(
                    "Verification for '{}': {} verified, {} corrected, {} flagged",
                    torrent.name,
                    verification.verified,
                    verification.corrected,
                    verification.flagged
                );
            }
        }
        Err(e) => {
            warn!(
                "Failed to verify matches for '{}': {}",
                torrent.name, e
            );
            // Continue with processing even if verification fails
        }
    }

    // Create file processor (source-agnostic)
    let processor = match analysis_queue {
        Some(queue) => FileProcessor::with_analysis_queue(db.clone(), queue),
        None => FileProcessor::new(db.clone()),
    };

    // Process all pending matches for this torrent
    let result = processor.process_source("torrent", torrent.id).await?;

    if result.files_processed > 0 {
        info!(
            "Finished processing '{}': {} files copied to library, {} failed",
            torrent.name, result.files_processed, result.files_failed
        );
    } else if result.files_failed > 0 {
        warn!(
            "Processing '{}' failed: {} files failed",
            torrent.name, result.files_failed
        );
    } else {
        debug!(
            "No pending matches to process for '{}'",
            torrent.name
        );
    }

    // Update torrent status
    let status = if result.files_failed == 0 && result.files_processed > 0 {
        "completed"
    } else if result.files_processed > 0 {
        "partial"
    } else {
        "unmatched"
    };
    
    db.torrents()
        .update_post_process_status(info_hash, status)
        .await?;

    Ok(())
}

/// Match torrent files when a torrent is added (using new source-agnostic FileMatcher)
async fn match_torrent_files_on_add(
    db: Database,
    torrent_service: Arc<TorrentService>,
    info_hash: &str,
) -> Result<()> {
    use crate::services::file_matcher::{FileInfo, FileMatcher};

    // Get the torrent record
    let torrent = match db.torrents().get_by_info_hash(info_hash).await? {
        Some(t) => t,
        None => {
            debug!("Torrent {} not yet in database, will retry", info_hash);
            return Ok(());
        }
    };

    // Check if matches already exist for this torrent (e.g., created by auto_hunt)
    // If so, skip re-matching to avoid duplicate key errors
    let existing_matches = db
        .pending_file_matches()
        .count_by_source("torrent", torrent.id)
        .await
        .unwrap_or(0);

    if existing_matches > 0 {
        info!(
            "Torrent '{}' already has {} pending matches (created by auto_hunt), skipping re-match",
            torrent.name, existing_matches
        );
        return Ok(());
    }

    // Get files from the torrent
    let files = match torrent_service.get_files_for_torrent(info_hash).await {
        Ok(f) => f,
        Err(_) => {
            debug!(
                "Cannot get files for '{}' yet, metadata still loading",
                torrent.name
            );
            return Ok(());
        }
    };

    if files.is_empty() {
        debug!(
            "Torrent '{}' has no files yet, metadata still loading",
            torrent.name
        );
        return Ok(());
    }

    let total_size_mb = files.iter().map(|f| f.size).sum::<u64>() / 1_000_000;
    info!(
        "Starting file matching for '{}' ({} files, {} MB)",
        torrent.name,
        files.len(),
        total_size_mb
    );

    // Convert torrent files to FileInfo
    let file_infos: Vec<FileInfo> = files
        .iter()
        .enumerate()
        .map(|(idx, f)| FileInfo {
            path: f.path.clone(),
            size: f.size as i64,
            file_index: Some(idx as i32),
            source_name: Some(torrent.name.clone()),
        })
        .collect();

    // Create file matcher (source-agnostic)
    let matcher = FileMatcher::new(db.clone());

    // Match files (no target library - will match against all user libraries with auto_download)
    let matches = matcher
        .match_files(torrent.user_id, file_infos, None)
        .await?;

    // Get summary for logging
    let summary = FileMatcher::summarize_matches(&matches);

    // Save matches to database (this also updates item statuses to 'downloading')
    let saved = matcher
        .save_matches(torrent.user_id, "torrent", Some(torrent.id), &matches)
        .await?;

    info!(
        "File matching complete for '{}': {} files, {} matched ({} episodes, {} movies, {} tracks, {} chapters), {} saved",
        torrent.name,
        summary.total_files,
        summary.matched,
        summary.episodes,
        summary.movies,
        summary.tracks,
        summary.chapters,
        saved.len()
    );

    Ok(())
}
