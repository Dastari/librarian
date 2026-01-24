//! Content download progress monitoring job
//!
//! This job monitors active downloads and broadcasts progress events for content items
//! (movies, episodes, tracks, chapters) that are linked to downloading torrent files.
//! This enables real-time download progress display on content detail pages.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::broadcast;
use tracing::{debug, trace};
use uuid::Uuid;

use crate::db::Database;
use crate::graphql::types::{ContentDownloadProgressEvent, ContentType};
use crate::services::TorrentService;

/// Information about a content item's download progress
#[derive(Debug, Clone)]
struct ContentProgress {
    content_type: ContentType,
    content_id: Uuid,
    library_id: Uuid,
    content_name: Option<String>,
    parent_id: Option<Uuid>,
    total_bytes: i64,
    downloaded_bytes: i64,
}

/// Calculate and broadcast content download progress
///
/// This function:
/// 1. Gets all active (downloading) torrents
/// 2. Finds torrent files that are linked to media files
/// 3. Finds content items (movies, episodes, tracks, chapters) linked to those media files
/// 4. Calculates aggregate progress per content item
/// 5. Broadcasts ContentDownloadProgressEvent for each
pub async fn broadcast_content_progress(
    db: &Database,
    torrent_service: &Arc<TorrentService>,
    progress_tx: &broadcast::Sender<ContentDownloadProgressEvent>,
) -> Result<()> {
    // Get active torrents (downloading state)
    let active_torrents = torrent_service.list_active_downloads().await;
    
    if active_torrents.is_empty() {
        return Ok(());
    }

    trace!("Checking {} active torrents for content progress", active_torrents.len());

    // Collect progress data for all content items
    let mut content_progress: HashMap<(ContentType, Uuid), ContentProgress> = HashMap::new();

    for torrent_info in &active_torrents {
        // Look up the torrent's database ID from its info_hash
        let torrent_record = match db.torrents().get_by_info_hash(&torrent_info.info_hash).await {
            Ok(Some(record)) => record,
            Ok(None) => {
                debug!(info_hash = %torrent_info.info_hash, "Torrent not found in database");
                continue;
            }
            Err(e) => {
                debug!(error = %e, info_hash = %torrent_info.info_hash, "Failed to get torrent from database");
                continue;
            }
        };

        // Get torrent files with their progress and media_file links
        let torrent_files = match db.torrent_files().get_by_torrent(torrent_record.id).await {
            Ok(files) => files,
            Err(e) => {
                debug!(error = %e, torrent_id = %torrent_record.id, "Failed to get torrent files");
                continue;
            }
        };

        for file in torrent_files {
            // Skip files not linked to a media file
            let media_file_id = match file.media_file_id {
                Some(id) => id,
                None => continue,
            };

            // Get the media file to find linked content
            let media_file = match db.media_files().get_by_id(media_file_id).await {
                Ok(Some(mf)) => mf,
                Ok(None) => continue,
                Err(e) => {
                    debug!(error = %e, media_file_id = %media_file_id, "Failed to get media file");
                    continue;
                }
            };

            // Determine content type and ID based on media file links
            let (content_type, content_id, content_name, parent_id) = 
                if let Some(movie_id) = media_file.movie_id {
                    // Get movie name
                    let name = db.movies().get_by_id(movie_id).await
                        .ok().flatten().map(|m| m.title);
                    (ContentType::Movie, movie_id, name, None)
                } else if let Some(episode_id) = media_file.episode_id {
                    // Get episode info and show ID
                    let (name, show_id) = db.episodes().get_by_id(episode_id).await
                        .ok().flatten()
                        .map(|e| (e.title, Some(e.tv_show_id)))
                        .unwrap_or((None, None));
                    (ContentType::Episode, episode_id, name, show_id)
                } else if let Some(track_id) = media_file.track_id {
                    // Get track info and album ID
                    let (name, album_id) = db.tracks().get_by_id(track_id).await
                        .ok().flatten()
                        .map(|t| (Some(t.title), Some(t.album_id)))
                        .unwrap_or((None, None));
                    (ContentType::Track, track_id, name, album_id)
                } else if let Some(chapter_id) = media_file.chapter_id {
                    // Get chapter info and audiobook ID
                    let (name, audiobook_id) = db.chapters().get_by_id(chapter_id).await
                        .ok().flatten()
                        .map(|c| (c.title, Some(c.audiobook_id)))
                        .unwrap_or((None, None));
                    (ContentType::Chapter, chapter_id, name, audiobook_id)
                } else {
                    // No content link, skip
                    continue;
                };

            let key = (content_type, content_id);
            let entry = content_progress.entry(key).or_insert_with(|| ContentProgress {
                content_type,
                content_id,
                library_id: media_file.library_id,
                content_name,
                parent_id,
                total_bytes: 0,
                downloaded_bytes: 0,
            });

            entry.total_bytes += file.file_size;
            entry.downloaded_bytes += file.downloaded_bytes;
        }
    }

    // Broadcast progress events
    for ((content_type, content_id), progress) in content_progress {
        let progress_value = if progress.total_bytes > 0 {
            progress.downloaded_bytes as f64 / progress.total_bytes as f64
        } else {
            0.0
        };

        // Only broadcast if there's actual progress (not 0% or 100%)
        // 100% items should have been processed by download monitor
        if progress_value > 0.0 && progress_value < 1.0 {
            let event = ContentDownloadProgressEvent {
                content_type,
                content_id: content_id.to_string(),
                library_id: progress.library_id.to_string(),
                progress: progress_value,
                download_speed: None, // Could be added later if needed
                content_name: progress.content_name,
                parent_id: progress.parent_id.map(|id| id.to_string()),
            };

            // Ignore send errors (no subscribers)
            let _ = progress_tx.send(event);
        }
    }

    Ok(())
}

/// Start the content progress monitoring loop
///
/// This runs in the background and periodically broadcasts content download progress.
/// It only does work when there are active downloads.
pub async fn start_content_progress_monitor(
    db: Database,
    torrent_service: Arc<TorrentService>,
    progress_tx: broadcast::Sender<ContentDownloadProgressEvent>,
) {
    use tokio::time::{interval, Duration};

    let mut interval = interval(Duration::from_secs(2));

    loop {
        interval.tick().await;

        if let Err(e) = broadcast_content_progress(&db, &torrent_service, &progress_tx).await {
            debug!(error = %e, "Content progress broadcast failed");
        }
    }
}
