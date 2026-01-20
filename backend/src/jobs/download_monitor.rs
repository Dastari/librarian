//! Download monitoring job
//!
//! This job monitors torrent completions and triggers post-download processing.
//!
//! Processing is delegated to the unified TorrentProcessor service which handles:
//! 1. Detecting completed torrents
//! 2. Matching files to library items (TV episodes, movies, music, audiobooks)
//! 3. Creating media file records
//! 4. Running file organization (if enabled for library)
//! 5. Updating item status to 'downloaded'
//! 6. Queueing files for FFmpeg analysis (if analysis queue provided)
//! 7. Auto-adding movies from TMDB when auto_add_discovered is enabled
//!
//! The processor respects show-level and library-level overrides for organization settings.

use std::sync::Arc;

use anyhow::Result;
use sqlx::PgPool;
use tracing::{debug, info};

use crate::db::Database;
use crate::services::{MediaAnalysisQueue, MetadataService, TorrentProcessor, TorrentService};

/// Process completed torrents
///
/// Called every minute by the job scheduler. Uses the unified TorrentProcessor
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
        (Some(queue), Some(metadata)) => TorrentProcessor::with_services(db, queue, metadata),
        (Some(queue), None) => TorrentProcessor::with_analysis_queue(db, queue),
        _ => TorrentProcessor::new(db),
    };

    let processed = processor.process_pending_torrents(&torrent_service, "scheduled job").await?;

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
        (Some(queue), Some(metadata)) => TorrentProcessor::with_services(db, queue, metadata),
        (Some(queue), None) => TorrentProcessor::with_analysis_queue(db, queue),
        _ => TorrentProcessor::new(db),
    };

    let matched = processor
        .process_unmatched_torrents(&torrent_service, "startup retry")
        .await?;

    if matched > 0 {
        info!("Startup retry matched {} previously unmatched torrents", matched);
    }

    Ok(())
}
