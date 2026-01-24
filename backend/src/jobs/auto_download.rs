//! Auto-download job for available episodes
//!
//! NOTE: This job is deprecated. The new media pipeline derives episode status
//! from media_file_id presence and uses auto-hunt + pending_file_matches for
//! tracking downloads. Episodes no longer have status or torrent_link fields.
//!
//! The old workflow was:
//! 1. RSS poller finds episodes and marks them as "available" with torrent_link
//! 2. Auto-download job picks up "available" episodes and downloads them
//! 3. Episode status changes to "downloading" then "downloaded"
//!
//! The new workflow is:
//! 1. Auto-hunt searches for content and downloads directly
//! 2. pending_file_matches tracks file -> episode mappings
//! 3. Episode status is derived from media_file_id presence

use anyhow::Result;
use std::sync::Arc;

type DbPool = crate::db::Pool;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::services::TorrentService;
use crate::services::file_matcher::{FileInfo, FileMatcher, KnownMatchTarget};
use crate::services::torrent::TorrentInfo;

/// Create file-level matches for an episode after torrent download starts
/// Uses the unified FileMatcher for all matching operations
#[allow(dead_code)]
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

/// Check for available episodes and start downloading them
///
/// NOTE: This function is deprecated. Episodes no longer have status or torrent_link
/// fields. Use auto-hunt for automated downloads instead.
pub async fn process_available_episodes(
    _pool: DbPool,
    _torrent_service: Arc<TorrentService>,
) -> Result<()> {
    // This job is deprecated - episodes no longer have status/torrent_link fields.
    // Downloads are now handled by:
    // 1. Auto-hunt searching for content and downloading directly
    // 2. pending_file_matches tracking file -> episode mappings
    warn!(
        job = "auto_download",
        "Auto-download job is deprecated. Episodes now derive status from media_file_id. Use auto-hunt for automated downloads."
    );

    info!(
        job = "auto_download",
        "Skipping deprecated auto-download job"
    );

    Ok(())
}

/// Download available episodes for a specific show immediately
/// Called when a show is added with backfill enabled
///
/// NOTE: This function is deprecated. Use auto-hunt instead.
pub async fn download_available_for_show(
    db: &Database,
    _torrent_service: Arc<TorrentService>,
    show_id: Uuid,
) -> Result<i32> {
    // This function is deprecated - episodes no longer have status/torrent_link fields.
    warn!(
        job = "auto_download",
        show_id = %show_id,
        "download_available_for_show is deprecated. Use auto-hunt instead."
    );

    // Get the show for logging
    if let Ok(Some(show)) = db.tv_shows().get_by_id(show_id).await {
        info!(
            "Skipping deprecated download_available_for_show for {}. Use auto-hunt instead.",
            show.name
        );
    }

    Ok(0)
}

/// Standalone function for job scheduler (gets pool from env)
/// NOTE: This is deprecated - use auto-hunt instead.
#[allow(dead_code)]
pub async fn check_and_download() -> Result<()> {
    info!("Auto-download check triggered (scheduler mode) - deprecated, use auto-hunt");
    Ok(())
}
