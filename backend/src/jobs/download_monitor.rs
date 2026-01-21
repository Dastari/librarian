//! Download monitoring job
//!
//! This job monitors torrent and usenet completions and triggers post-download processing.
//!
//! Processing is delegated to the unified MediaProcessor service which handles:
//! 1. Detecting completed torrents/usenet downloads
//! 2. Matching files to library items (TV episodes, movies, music, audiobooks)
//! 3. Creating media file records
//! 4. Running file organization (if enabled for library)
//! 5. Updating item status to 'downloaded'
//! 6. Queueing files for FFmpeg analysis (if analysis queue provided)
//! 7. Auto-adding movies from TMDB when auto_add_discovered is enabled
//!
//! The processor respects show-level and library-level overrides for organization settings.
//!
//! For Usenet downloads, the same pipeline is used once the download completes
//! and files are extracted/decoded.

use std::sync::Arc;

use anyhow::Result;
use sqlx::PgPool;
use tracing::info;

use crate::db::Database;
use crate::services::{MediaAnalysisQueue, MediaProcessor, MetadataService, TorrentService, UsenetService};

/// Process completed torrents
///
/// Called every minute by the job scheduler. Uses the unified MediaProcessor
/// to handle all torrents that have completed downloading but haven't been
/// processed yet.
pub async fn process_completed_torrents(
    pool: PgPool,
    torrent_service: Arc<TorrentService>,
    analysis_queue: Option<Arc<MediaAnalysisQueue>>,
    metadata_service: Option<Arc<MetadataService>>,
) -> Result<()> {
    let db = Database::new(pool);
    let processor = match (analysis_queue, metadata_service) {
        (Some(queue), Some(metadata)) => MediaProcessor::with_services(db, queue, metadata),
        (Some(queue), None) => MediaProcessor::with_analysis_queue(db, queue),
        _ => MediaProcessor::new(db),
    };

    let processed = processor
        .process_pending_torrents(&torrent_service, "scheduled job")
        .await?;

    if processed > 0 {
        info!("Scheduled job processed {} completed torrents", processed);
    }

    Ok(())
}

/// Process unmatched torrents on startup
///
/// Called once on startup to retry matching for torrents that previously
/// failed to match (e.g., because the show wasn't in the library yet).
pub async fn process_unmatched_on_startup(
    pool: PgPool,
    torrent_service: Arc<TorrentService>,
    analysis_queue: Option<Arc<MediaAnalysisQueue>>,
    metadata_service: Option<Arc<MetadataService>>,
) -> Result<()> {
    let db = Database::new(pool);
    let processor = match (analysis_queue, metadata_service) {
        (Some(queue), Some(metadata)) => MediaProcessor::with_services(db, queue, metadata),
        (Some(queue), None) => MediaProcessor::with_analysis_queue(db, queue),
        _ => MediaProcessor::new(db),
    };

    let matched = processor
        .process_unmatched_torrents(&torrent_service, "startup retry")
        .await?;

    if matched > 0 {
        info!(
            "Startup retry matched {} previously unmatched torrents",
            matched
        );
    }

    Ok(())
}

/// Process completed usenet downloads
///
/// Called every minute by the job scheduler. Checks for usenet downloads
/// that have completed and need post-processing (matching to library items,
/// organizing files, etc.).
///
/// Uses the same MediaProcessor pipeline since the post-download logic
/// is source-agnostic - it only cares about the files and metadata.
pub async fn process_completed_usenet_downloads(
    pool: PgPool,
    usenet_service: Arc<UsenetService>,
    analysis_queue: Option<Arc<MediaAnalysisQueue>>,
    metadata_service: Option<Arc<MetadataService>>,
) -> Result<()> {
    let db = Database::new(pool.clone());
    
    // Get usenet downloads that are completed but not yet processed
    let pending = usenet_service.list_pending_processing_records().await?;
    
    if pending.is_empty() {
        return Ok(());
    }
    
    info!("Processing {} completed usenet downloads", pending.len());
    
    let processor = match (analysis_queue, metadata_service) {
        (Some(queue), Some(metadata)) => MediaProcessor::with_services(db, queue, metadata),
        (Some(queue), None) => MediaProcessor::with_analysis_queue(db, queue),
        _ => MediaProcessor::new(db),
    };
    
    let mut processed = 0;
    
    for download in pending {
        // For usenet downloads, we use the DownloadSource trait
        // to provide a unified interface for file matching and organization
        match processor.process_usenet_download(&download, &usenet_service, "scheduled job").await {
            Ok(_) => {
                processed += 1;
                usenet_service.set_post_process_status(download.id, "completed").await?;
            }
            Err(e) => {
                tracing::error!(
                    download_id = %download.id,
                    name = %download.nzb_name,
                    error = %e,
                    "Failed to process usenet download"
                );
                // Mark as failed so we don't keep retrying
                usenet_service.set_post_process_status(download.id, "failed").await?;
            }
        }
    }
    
    if processed > 0 {
        info!("Scheduled job processed {} completed usenet downloads", processed);
    }
    
    Ok(())
}

/// Process all completed downloads (both torrents and usenet)
///
/// Convenience function that processes both types of downloads in sequence.
pub async fn process_all_completed_downloads(
    pool: PgPool,
    torrent_service: Arc<TorrentService>,
    usenet_service: Option<Arc<UsenetService>>,
    analysis_queue: Option<Arc<MediaAnalysisQueue>>,
    metadata_service: Option<Arc<MetadataService>>,
) -> Result<()> {
    // Process torrents
    process_completed_torrents(
        pool.clone(),
        torrent_service,
        analysis_queue.clone(),
        metadata_service.clone(),
    ).await?;
    
    // Process usenet downloads if service is available
    if let Some(usenet) = usenet_service {
        process_completed_usenet_downloads(
            pool,
            usenet,
            analysis_queue,
            metadata_service,
        ).await?;
    }
    
    Ok(())
}
