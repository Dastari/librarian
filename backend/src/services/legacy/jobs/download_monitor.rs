//! Download monitoring job
//!
//! This job monitors torrent and usenet completions and triggers post-download processing.
//!
//! Processing is delegated to the source-agnostic FileProcessor service which handles:
//! 1. Detecting completed torrents/usenet downloads with pending matches
//! 2. Copying files to library folders according to naming rules
//! 3. Creating media file records
//! 4. Updating item status to 'downloaded'
//! 5. Queueing files for FFmpeg analysis

use std::sync::Arc;

use anyhow::Result;
use tracing::{debug, info, warn};

type DbPool = crate::db::Pool;

use crate::db::Database;
use crate::services::file_matcher::{FileInfo, FileMatcher};
use crate::services::file_processor::FileProcessor;
use crate::services::queues::MediaAnalysisQueue;
use crate::services::torrent::TorrentService;
use crate::services::usenet::UsenetService;

/// Process completed torrents
///
/// Called every minute by the job scheduler. Uses the FileProcessor
/// to process all torrents that have pending file matches.
pub async fn process_completed_torrents(
    pool: DbPool,
    _torrent_service: Arc<TorrentService>,
    analysis_queue: Option<Arc<MediaAnalysisQueue>>,
) -> Result<()> {
    let db = Database::new(pool);

    // Get all torrents that are complete and have uncopied matches
    let completed_torrents: Vec<crate::db::TorrentRecord> = sqlx::query_as(
        r#"
        SELECT DISTINCT t.* FROM torrents t
        INNER JOIN pending_file_matches pfm ON pfm.source_id = t.id AND pfm.source_type = 'torrent'
        WHERE pfm.copied_at IS NULL
        AND t.state IN ('completed', 'seeding')
        "#,
    )
    .fetch_all(db.pool())
    .await?;

    if completed_torrents.is_empty() {
        return Ok(());
    }

    info!(
        "Processing {} completed torrents with pending matches",
        completed_torrents.len()
    );

    let processor = match analysis_queue {
        Some(queue) => FileProcessor::with_analysis_queue(db.clone(), queue),
        None => FileProcessor::new(db.clone()),
    };

    let mut total_processed = 0;

    for torrent in completed_torrents {
        match processor.process_source("torrent", torrent.id).await {
            Ok(result) => {
                if result.files_processed > 0 {
                    info!(
                        "Processed torrent '{}': {} files copied",
                        torrent.name, result.files_processed
                    );
                    total_processed += result.files_processed;
                }
            }
            Err(e) => {
                warn!(
                    "Failed to process torrent '{}': {}",
                    torrent.name, e
                );
            }
        }
    }

    if total_processed > 0 {
        info!(
            "Scheduled job processed {} files from completed torrents",
            total_processed
        );
    }

    Ok(())
}

/// Process unmatched torrents on startup
///
/// Called once on startup to retry matching for torrents that previously
/// failed to match (e.g., because the show wasn't in the library yet).
pub async fn process_unmatched_on_startup(
    pool: DbPool,
    torrent_service: Arc<TorrentService>,
    _analysis_queue: Option<Arc<MediaAnalysisQueue>>,
) -> Result<()> {
    let db = Database::new(pool);

    // Get torrents that have no pending matches but are complete
    let unmatched_torrents: Vec<crate::db::TorrentRecord> = sqlx::query_as(
        r#"
        SELECT t.* FROM torrents t
        WHERE t.state IN ('completed', 'seeding')
        AND NOT EXISTS (
            SELECT 1 FROM pending_file_matches pfm 
            WHERE pfm.source_id = t.id
              AND pfm.source_type = 'torrent'
              AND (pfm.episode_id IS NOT NULL
                   OR pfm.movie_id IS NOT NULL
                   OR pfm.track_id IS NOT NULL
                   OR pfm.chapter_id IS NOT NULL)
        )
        AND t.created_at > datetime('now', '-7 days')
        "#,
    )
    .fetch_all(db.pool())
    .await?;

    if unmatched_torrents.is_empty() {
        return Ok(());
    }

    info!(
        "Retrying matching for {} unmatched torrents",
        unmatched_torrents.len()
    );

    let matcher = FileMatcher::new(db.clone());
    let mut total_matched = 0;

    for torrent in unmatched_torrents {
        // Get files from torrent service
        let files = match torrent_service.get_files_for_torrent(&torrent.info_hash).await {
            Ok(f) => f,
            Err(e) => {
                debug!(
                    "Cannot get files for '{}': {}",
                    torrent.name, e
                );
                continue;
            }
        };

        if files.is_empty() {
            continue;
        }

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

        // Try to match files
        match matcher.match_files(torrent.user_id, file_infos, None).await {
            Ok(matches) => {
                let summary = FileMatcher::summarize_matches(&matches);
                if summary.matched > 0 {
                    // Save matches
                    match matcher
                        .save_matches(torrent.user_id, "torrent", Some(torrent.id), &matches)
                        .await
                    {
                        Ok(saved) => {
                            info!(
                                "Matched {} files in '{}' on retry",
                                saved.len(),
                                torrent.name
                            );
                            total_matched += saved.len();
                        }
                        Err(e) => {
                            warn!(
                                "Failed to save matches for '{}': {}",
                                torrent.name, e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to match files in '{}': {}",
                    torrent.name, e
                );
            }
        }
    }

    if total_matched > 0 {
        info!(
            "Startup retry matched {} files from previously unmatched torrents",
            total_matched
        );
    }

    Ok(())
}

/// Process completed usenet downloads
///
/// Called every minute by the job scheduler. Checks for usenet downloads
/// that have completed and need post-processing.
pub async fn process_completed_usenet_downloads(
    pool: DbPool,
    usenet_service: Arc<UsenetService>,
    analysis_queue: Option<Arc<MediaAnalysisQueue>>,
) -> Result<()> {
    let db = Database::new(pool);

    // Get usenet downloads that are completed but not yet processed
    let pending = usenet_service.list_pending_processing_records().await?;

    if pending.is_empty() {
        return Ok(());
    }

    info!("Processing {} completed usenet downloads", pending.len());

    let processor = match analysis_queue {
        Some(queue) => FileProcessor::with_analysis_queue(db.clone(), queue),
        None => FileProcessor::new(db.clone()),
    };

    let mut processed = 0;

    for download in pending {
        match processor.process_source("usenet", download.id).await {
            Ok(result) => {
                if result.files_processed > 0 || result.files_failed == 0 {
                    processed += 1;
                    usenet_service
                        .set_post_process_status(download.id, "completed")
                        .await?;
                } else {
                    usenet_service
                        .set_post_process_status(download.id, "failed")
                        .await?;
                }
            }
            Err(e) => {
                warn!(
                    download_id = %download.id,
                    name = %download.nzb_name,
                    error = %e,
                    "Failed to process usenet download"
                );
                usenet_service
                    .set_post_process_status(download.id, "failed")
                    .await?;
            }
        }
    }

    if processed > 0 {
        info!(
            "Scheduled job processed {} completed usenet downloads",
            processed
        );
    }

    Ok(())
}

/// Process all completed downloads (both torrents and usenet)
///
/// Convenience function that processes both types of downloads in sequence.
pub async fn process_all_completed_downloads(
    pool: DbPool,
    torrent_service: Arc<TorrentService>,
    usenet_service: Option<Arc<UsenetService>>,
    analysis_queue: Option<Arc<MediaAnalysisQueue>>,
) -> Result<()> {
    // Process torrents
    process_completed_torrents(pool.clone(), torrent_service, analysis_queue.clone()).await?;

    // Process usenet downloads if service is available
    if let Some(usenet) = usenet_service {
        process_completed_usenet_downloads(pool, usenet, analysis_queue).await?;
    }

    Ok(())
}
