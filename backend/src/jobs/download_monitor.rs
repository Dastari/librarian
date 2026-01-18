//! Download monitoring job
//!
//! This job monitors torrent completions and triggers post-download processing:
//! 1. Detects completed torrents (state = 'seeding', post_process_status IS NULL)
//! 2. Identifies files belonging to episodes
//! 3. Creates media file entries in database
//! 4. Runs file organization (if enabled for library/show)
//! 5. Updates episode status to 'downloaded'
//!
//! Show-level overrides are respected:
//! - `organize_files_override`: Override library's organize_files setting
//! - `rename_style_override`: Override library's rename_style setting

use anyhow::Result;
use sqlx::PgPool;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::db::Database;
use crate::services::{OrganizerService, TorrentService};

/// Process completed torrents and organize files
///
/// Called every minute by the job scheduler. Finds torrents that have completed
/// downloading (seeding) but haven't been processed yet, then:
/// 1. Creates media file records for video files
/// 2. Organizes files into library structure (if enabled)
/// 3. Updates episode status to 'downloaded'
/// 4. Marks the torrent as processed
pub async fn process_completed_torrents(
    pool: PgPool,
    torrent_service: Arc<TorrentService>,
) -> Result<()> {
    let db = Database::new(pool);

    // Get all completed torrents that need processing
    let completed_torrents = db.torrents().list_pending_processing().await?;

    if completed_torrents.is_empty() {
        debug!(job = "download_monitor", "No completed torrents to process");
        return Ok(());
    }

    info!(
        job = "download_monitor",
        torrent_count = completed_torrents.len(),
        "Processing completed torrents"
    );

    let organizer = OrganizerService::new(db.clone());

    for torrent in completed_torrents {
        if let Err(e) = process_single_torrent(&db, &torrent_service, &organizer, &torrent).await {
            error!(
                job = "download_monitor",
                info_hash = %torrent.info_hash,
                torrent_name = %torrent.name,
                error = %e,
                "Failed to process completed torrent: {}",
                torrent.name
            );
        }
    }

    Ok(())
}

async fn process_single_torrent(
    db: &Database,
    torrent_service: &Arc<TorrentService>,
    organizer: &OrganizerService,
    torrent: &crate::db::TorrentRecord,
) -> Result<()> {
    info!(
        job = "download_monitor",
        info_hash = %torrent.info_hash,
        torrent_name = %torrent.name,
        "Processing completed torrent: {}",
        torrent.name
    );

    // Get files from the torrent
    let files = match torrent_service
        .get_files_for_torrent(&torrent.info_hash)
        .await
    {
        Ok(f) => f,
        Err(e) => {
            warn!(
                job = "download_monitor",
                info_hash = %torrent.info_hash,
                torrent_name = %torrent.name,
                error = %e,
                "Could not get files for torrent: {}",
                torrent.name
            );
            // Mark as processed anyway to avoid retrying
            db.torrents().mark_processed(&torrent.info_hash).await?;
            return Ok(());
        }
    };

    // Find the episode associated with this torrent
    let episode = match &torrent.episode_id {
        Some(ep_id) => db.episodes().get_by_id(*ep_id).await?,
        None => None,
    };

    // Get show and library for organization
    let (show, library) = if let Some(ref ep) = episode {
        let show = db.tv_shows().get_by_id(ep.tv_show_id).await?;
        let lib = if let Some(ref s) = show {
            db.libraries().get_by_id(s.library_id).await?
        } else {
            None
        };
        (show, lib)
    } else {
        (None, None)
    };

    // Process each file
    for file_info in files {
        // Skip non-video files
        if !is_video_file(&file_info.path) {
            debug!(path = %file_info.path, "Skipping non-video file");
            continue;
        }

        // Create media file record
        let file_path = file_info.path.clone();
        let file_size = file_info.size;

        // Check if file already exists
        if db.media_files().exists_by_path(&file_path).await? {
            debug!(path = %file_path, "Media file already exists");
            continue;
        }

        // Get library id
        let library_id = library.as_ref().map(|l| l.id);

        if let Some(lib_id) = library_id {
            // Create media file entry
            let media_file = db
                .media_files()
                .create(crate::db::CreateMediaFile {
                    library_id: lib_id,
                    path: file_path.clone(),
                    size_bytes: file_size as i64,
                    container: get_container(&file_path),
                    video_codec: None,
                    audio_codec: None,
                    width: None,
                    height: None,
                    duration: None,
                    bitrate: None,
                    file_hash: None,
                    episode_id: episode.as_ref().map(|e| e.id),
                    relative_path: None,
                    original_name: Path::new(&file_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string()),
                    resolution: None,
                    is_hdr: None,
                    hdr_type: None,
                })
                .await?;

            info!(
                job = "download_monitor",
                file_id = %media_file.id,
                path = %file_path,
                torrent_name = %torrent.name,
                "Created media file record for torrent: {}",
                torrent.name
            );

            // Organize the file if enabled (respects show-level overrides)
            if let (Some(lib), Some(ep), Some(s)) = (&library, &episode, &show) {
                let (organize_enabled, rename_style, action) =
                    organizer.get_full_organize_settings(s).await?;

                if organize_enabled {
                    info!(
                        job = "download_monitor",
                        file_id = %media_file.id,
                        show_name = %s.name,
                        rename_style = ?rename_style,
                        action = %action,
                        "Organizing file for show: {}",
                        s.name
                    );

                    match organizer
                        .organize_file(&media_file, s, ep, &lib.path, rename_style, &action, false)
                        .await
                    {
                        Ok(result) => {
                            if result.success {
                                info!(
                                    job = "download_monitor",
                                    file_id = %result.file_id,
                                    show_name = %s.name,
                                    new_path = %result.new_path,
                                    action = %action,
                                    "File organized successfully for show: {}",
                                    s.name
                                );
                            } else {
                                warn!(
                                    job = "download_monitor",
                                    file_id = %result.file_id,
                                    show_name = %s.name,
                                    error = ?result.error,
                                    "Failed to organize file for show: {}",
                                    s.name
                                );
                            }
                        }
                        Err(e) => {
                            error!(
                                job = "download_monitor",
                                file_id = %media_file.id,
                                show_name = %s.name,
                                error = %e,
                                "Error organizing file for show: {}",
                                s.name
                            );
                        }
                    }
                } else {
                    debug!(
                        job = "download_monitor",
                        file_id = %media_file.id,
                        show_name = %s.name,
                        "Organization disabled for show: {}, skipping",
                        s.name
                    );
                }
            }
        }
    }

    // Update episode status to downloaded
    if let Some(ref ep) = episode {
        db.episodes().update_status(ep.id, "downloaded").await?;
        if let Some(ref s) = show {
            info!(
                job = "download_monitor",
                episode_id = %ep.id,
                show_name = %s.name,
                season = ep.season,
                episode = ep.episode,
                "Episode marked as downloaded: {} S{:02}E{:02}",
                s.name, ep.season, ep.episode
            );
            // Update show stats
            db.tv_shows().update_stats(s.id).await?;
        } else {
            info!(
                job = "download_monitor",
                episode_id = %ep.id,
                "Episode marked as downloaded"
            );
        }
    }

    // Mark torrent as processed
    db.torrents().mark_processed(&torrent.info_hash).await?;

    info!(
        job = "download_monitor",
        info_hash = %torrent.info_hash,
        torrent_name = %torrent.name,
        "Torrent processing complete: {}",
        torrent.name
    );

    Ok(())
}

/// Check if a file is a video file based on extension
fn is_video_file(path: &str) -> bool {
    let video_extensions = [
        ".mkv", ".mp4", ".avi", ".mov", ".wmv", ".flv", ".webm", ".m4v", ".ts", ".m2ts",
    ];

    let lower = path.to_lowercase();
    video_extensions.iter().any(|ext| lower.ends_with(ext))
}

/// Get container format from file extension
fn get_container(path: &str) -> Option<String> {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
}
