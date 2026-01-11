//! Auto-download job for available episodes
//!
//! This job:
//! 1. Finds episodes with status 'available' (matched from RSS)
//! 2. Checks library and show settings to determine if auto-download is enabled
//! 3. Starts downloading enabled episodes via the torrent service
//! 4. Updates episode status to 'downloading'

use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::services::TorrentService;

/// Check for available episodes and start downloading them
pub async fn process_available_episodes(
    pool: PgPool,
    torrent_service: Arc<TorrentService>,
) -> Result<()> {
    info!(job = "auto_download", "Starting auto-download job: checking for available episodes");

    let db = Database::new(pool);

    // Get all episodes with 'available' status
    let available_episodes = db.episodes().list_available().await?;

    if available_episodes.is_empty() {
        debug!(job = "auto_download", "No available episodes found to download");
        return Ok(());
    }

    info!(
        job = "auto_download",
        episode_count = available_episodes.len(),
        "Found available episodes to download"
    );

    let mut downloaded = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for episode in available_episodes {
        // Get show info
        let show = match db.tv_shows().get_by_id(episode.tv_show_id).await? {
            Some(s) => s,
            None => {
                warn!(
                    job = "auto_download",
                    episode_id = %episode.id,
                    "Skipping episode: no associated show found"
                );
                continue;
            }
        };

        // Get library info
        let library = match db.libraries().get_by_id(show.library_id).await? {
            Some(l) => l,
            None => {
                warn!(
                    job = "auto_download",
                    show_id = %show.id,
                    show_name = %show.name,
                    "Skipping show: no associated library found"
                );
                continue;
            }
        };

        // Check if auto-download is enabled
        // Show override takes precedence, then fall back to library setting
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
            skipped += 1;
            continue;
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
                continue;
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
        // The torrent link could be a .torrent URL or a magnet link
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

                // Link torrent to episode for post-processing
                if let Err(e) = db.torrents().link_to_episode(&torrent_info.info_hash, episode.id).await {
                    error!(
                        job = "auto_download",
                        show_name = %show.name,
                        error = %e,
                        "Failed to link torrent to episode"
                    );
                }

                // Update episode status to 'downloading'
                if let Err(e) = db.episodes().mark_downloading(episode.id).await {
                    error!(
                        job = "auto_download",
                        show_name = %show.name,
                        episode_id = %episode.id,
                        error = %e,
                        "Failed to update episode status to downloading"
                    );
                }

                downloaded += 1;
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
                failed += 1;
            }
        }
    }

    info!(
        job = "auto_download",
        downloaded = downloaded,
        skipped = skipped,
        failed = failed,
        "Auto-download job complete: {} started, {} skipped (disabled), {} failed",
        downloaded, skipped, failed
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

                // Link torrent to episode for post-processing
                if let Err(e) = db.torrents().link_to_episode(&torrent_info.info_hash, episode.id).await {
                    error!("Failed to link torrent to episode: {:?}", e);
                }

                if let Err(e) = db.episodes().mark_downloading(episode.id).await {
                    error!("Failed to update episode status: {:?}", e);
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
