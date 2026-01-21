//! Auto-download job for available episodes
//!
//! This job:
//! 1. Finds episodes with status 'available' (matched from RSS)
//! 2. Checks library and show settings to determine if auto-download is enabled
//! 3. Starts downloading enabled episodes via the torrent service
//! 4. Updates episode status to 'downloading'
//!
//! Uses bounded concurrency to prevent overwhelming the torrent service.

use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::services::TorrentService;
use crate::services::torrent::TorrentInfo;
// Use the unified FileMatcher for all matching operations
use crate::services::file_matcher::{FileInfo, FileMatcher, KnownMatchTarget};

/// Create file-level matches for an episode after torrent download starts
/// Uses the unified FileMatcher for all matching operations
async fn create_file_matches_for_episode(
    db: &Database,
    torrent_info: &TorrentInfo,
    episode_id: Uuid,
) -> Result<()> {
    let torrent_record = db
        .torrents()
        .get_by_info_hash(&torrent_info.info_hash)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Torrent record not found"))?;

    // Convert torrent files to FileInfo
    let files: Vec<FileInfo> = torrent_info
        .files
        .iter()
        .enumerate()
        .map(|(idx, f)| FileInfo {
            path: f.path.clone(),
            size: f.size as i64,
            file_index: Some(idx as i32),
            source_name: Some(torrent_info.name.clone()),
        })
        .collect();

    // Use the unified FileMatcher
    let matcher = FileMatcher::new(db.clone());
    let records = matcher
        .create_matches_for_target(
            torrent_record.user_id,
            "torrent",
            torrent_record.id,
            files,
            KnownMatchTarget::Episode(episode_id),
        )
        .await?;

    debug!(
        job = "auto_download",
        torrent_id = %torrent_record.id,
        episode_id = %episode_id,
        files = torrent_info.files.len(),
        matches_created = records.len(),
        "Created file-level matches for episode via FileMatcher"
    );

    Ok(())
}

/// Maximum concurrent download starts
const MAX_CONCURRENT_DOWNLOADS: usize = 3;

/// Delay between download batches (ms)
const BATCH_DELAY_MS: u64 = 500;

/// Check for available episodes and start downloading them
///
/// Uses bounded concurrency to prevent overwhelming the torrent client.
pub async fn process_available_episodes(
    pool: PgPool,
    torrent_service: Arc<TorrentService>,
) -> Result<()> {
    info!(
        job = "auto_download",
        "Starting auto-download job: checking for available episodes"
    );

    let db = Database::new(pool);

    // Get all episodes with 'available' status
    let available_episodes = db.episodes().list_available().await?;

    if available_episodes.is_empty() {
        debug!(
            job = "auto_download",
            "No available episodes found to download"
        );
        return Ok(());
    }

    info!(
        job = "auto_download",
        episode_count = available_episodes.len(),
        max_concurrent = MAX_CONCURRENT_DOWNLOADS,
        "Found available episodes to download"
    );

    // Atomic counters for thread-safe progress tracking
    let downloaded = Arc::new(AtomicI32::new(0));
    let skipped = Arc::new(AtomicI32::new(0));
    let failed = Arc::new(AtomicI32::new(0));

    // Semaphore to limit concurrent downloads
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS));

    // Process episodes in chunks
    let chunk_size = MAX_CONCURRENT_DOWNLOADS;

    for chunk in available_episodes.chunks(chunk_size) {
        let mut handles = Vec::with_capacity(chunk.len());

        for episode in chunk {
            let db = db.clone();
            let torrent_service = torrent_service.clone();
            let semaphore = semaphore.clone();
            let downloaded = downloaded.clone();
            let skipped = skipped.clone();
            let failed = failed.clone();
            let episode = episode.clone();

            let handle = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.expect("Semaphore closed");

                // Get show info
                let show = match db.tv_shows().get_by_id(episode.tv_show_id).await {
                    Ok(Some(s)) => s,
                    _ => {
                        warn!(
                            job = "auto_download",
                            episode_id = %episode.id,
                            "Skipping episode: no associated show found"
                        );
                        return;
                    }
                };

                // Get library info
                let library = match db.libraries().get_by_id(show.library_id).await {
                    Ok(Some(l)) => l,
                    _ => {
                        warn!(
                            job = "auto_download",
                            show_id = %show.id,
                            show_name = %show.name,
                            "Skipping show: no associated library found"
                        );
                        return;
                    }
                };

                // Check if auto-download is enabled
                let auto_download_enabled = match show.auto_download_override {
                    Some(override_value) => override_value,
                    None => library.auto_download,
                };

                if !auto_download_enabled {
                    debug!(
                        job = "auto_download",
                        show_name = %show.name,
                        show_override = ?show.auto_download_override,
                        library_setting = library.auto_download,
                        "Auto-download disabled for show, skipping"
                    );
                    skipped.fetch_add(1, Ordering::SeqCst);
                    return;
                }

                let torrent_link = match &episode.torrent_link {
                    Some(link) => link.clone(),
                    None => {
                        warn!(
                            job = "auto_download",
                            show_name = %show.name,
                            season = episode.season,
                            episode = episode.episode,
                            "Skipping episode: no torrent link available"
                        );
                        return;
                    }
                };

                info!(
                    job = "auto_download",
                    show_name = %show.name,
                    season = episode.season,
                    episode = episode.episode,
                    "Starting download for {} S{:02}E{:02}",
                    show.name,
                    episode.season,
                    episode.episode
                );

                // Try to add the torrent
                let add_result = if torrent_link.starts_with("magnet:") {
                    torrent_service.add_magnet(&torrent_link, None).await
                } else {
                    torrent_service.add_torrent_url(&torrent_link, None).await
                };

                match add_result {
                    Ok(torrent_info) => {
                        info!(
                            job = "auto_download",
                            show_name = %show.name,
                            season = episode.season,
                            episode = episode.episode,
                            torrent_id = torrent_info.id,
                            torrent_name = %torrent_info.name,
                            "Successfully started download for {} S{:02}E{:02}",
                            show.name, episode.season, episode.episode
                        );

                        // Create file-level matches for the episode
                        if let Err(e) =
                            create_file_matches_for_episode(&db, &torrent_info, episode.id).await
                        {
                            error!(
                                job = "auto_download",
                                show_name = %show.name,
                                error = %e,
                                "Failed to create file matches for episode"
                            );
                        }

                        downloaded.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(e) => {
                        error!(
                            job = "auto_download",
                            show_name = %show.name,
                            season = episode.season,
                            episode = episode.episode,
                            error = %e,
                            "Failed to start download for {} S{:02}E{:02}",
                            show.name, episode.season, episode.episode
                        );
                        failed.fetch_add(1, Ordering::SeqCst);
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all downloads in this chunk to start
        for handle in handles {
            if let Err(e) = handle.await {
                error!(job = "auto_download", error = %e, "Download task panicked");
            }
        }

        // Small delay between chunks
        if BATCH_DELAY_MS > 0 {
            tokio::time::sleep(Duration::from_millis(BATCH_DELAY_MS)).await;
        }
    }

    let final_downloaded = downloaded.load(Ordering::SeqCst);
    let final_skipped = skipped.load(Ordering::SeqCst);
    let final_failed = failed.load(Ordering::SeqCst);

    info!(
        job = "auto_download",
        downloaded = final_downloaded,
        skipped = final_skipped,
        failed = final_failed,
        "Auto-download job complete: {} started, {} skipped (disabled), {} failed",
        final_downloaded,
        final_skipped,
        final_failed
    );

    Ok(())
}

/// Download available episodes for a specific show immediately
/// Called when a show is added with backfill enabled
pub async fn download_available_for_show(
    db: &Database,
    torrent_service: Arc<TorrentService>,
    show_id: Uuid,
) -> Result<i32> {
    info!("Downloading available episodes for show {}", show_id);

    // Get the show
    let show = match db.tv_shows().get_by_id(show_id).await? {
        Some(s) => s,
        None => {
            warn!("Show {} not found", show_id);
            return Ok(0);
        }
    };

    // Get the library
    let library = match db.libraries().get_by_id(show.library_id).await? {
        Some(l) => l,
        None => {
            warn!("Library for show {} not found", show_id);
            return Ok(0);
        }
    };

    // Check if auto-download is enabled for this show
    let auto_download_enabled = match show.auto_download_override {
        Some(override_value) => override_value,
        None => library.auto_download,
    };

    if !auto_download_enabled {
        info!(
            "Auto-download disabled for {} - skipping immediate download",
            show.name
        );
        return Ok(0);
    }

    // Get available episodes for this show
    let episodes = db.episodes().list_available_for_show(show_id).await?;

    if episodes.is_empty() {
        info!("No available episodes for show {}", show.name);
        return Ok(0);
    }

    info!(
        "Found {} available episodes for {} to download immediately",
        episodes.len(),
        show.name
    );

    let mut downloaded = 0;

    for episode in episodes {
        let torrent_link = match &episode.torrent_link {
            Some(link) => link.clone(),
            None => continue,
        };

        info!(
            "Downloading {} S{:02}E{:02}",
            show.name, episode.season, episode.episode
        );

        let add_result = if torrent_link.starts_with("magnet:") {
            torrent_service.add_magnet(&torrent_link, None).await
        } else {
            torrent_service.add_torrent_url(&torrent_link, None).await
        };

        match add_result {
            Ok(torrent_info) => {
                info!(
                    "Started download: {} (id: {})",
                    torrent_info.name, torrent_info.id
                );

                // Create file-level matches for the episode
                if let Err(e) =
                    create_file_matches_for_episode(&db, &torrent_info, episode.id).await
                {
                    error!("Failed to create file matches for episode: {:?}", e);
                }

                downloaded += 1;
            }
            Err(e) => {
                error!(
                    "Failed to start download for S{:02}E{:02}: {:?}",
                    episode.season, episode.episode, e
                );
            }
        }
    }

    info!(
        "Immediate download for {}: {} episodes started",
        show.name, downloaded
    );

    Ok(downloaded)
}

/// Standalone function for job scheduler (gets pool from env)
/// NOTE: This is deprecated - use process_available_episodes() directly instead.
#[allow(dead_code)]
pub async fn check_and_download() -> Result<()> {
    // This is a placeholder - the actual implementation should receive
    // the torrent service from the job scheduler context
    info!("Auto-download check triggered (scheduler mode)");

    // For now, just log - the full implementation requires access to TorrentService
    // which should be passed from the main application

    Ok(())
}
