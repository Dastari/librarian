//! Usenet Download Service
//!
//! Manages Usenet downloads using native NNTP client.
//! Parallel to TorrentService but for NZB-based downloads.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::{CreateUsenetDownload, Database, UsenetDownloadRecord};

/// Usenet download event for subscriptions
#[derive(Debug, Clone, Serialize)]
pub enum UsenetEvent {
    /// Download added
    Added(UsenetDownloadInfo),
    /// Download progress updated
    Progress(UsenetProgressUpdate),
    /// Download state changed
    StateChanged {
        id: Uuid,
        old_state: String,
        new_state: String,
    },
    /// Download completed
    Completed(UsenetDownloadInfo),
    /// Download failed
    Failed { id: Uuid, error: String },
    /// Download removed
    Removed(Uuid),
}

/// Progress update for a download
#[derive(Debug, Clone, Serialize)]
pub struct UsenetProgressUpdate {
    pub id: Uuid,
    pub progress: f64,
    pub downloaded_bytes: i64,
    pub total_bytes: Option<i64>,
    pub speed_bytes_sec: i64,
    pub eta_seconds: Option<i32>,
}

/// Information about a usenet download
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsenetDownloadInfo {
    pub id: Uuid,
    pub name: String,
    pub state: String,
    pub progress: f64,
    pub size_bytes: Option<i64>,
    pub downloaded_bytes: i64,
    pub download_speed: i64,
    pub eta_seconds: Option<i32>,
    pub error_message: Option<String>,
    pub download_path: Option<String>,
    pub library_id: Option<Uuid>,
    pub episode_id: Option<Uuid>,
    pub movie_id: Option<Uuid>,
    pub album_id: Option<Uuid>,
    pub audiobook_id: Option<Uuid>,
}

impl From<&UsenetDownloadRecord> for UsenetDownloadInfo {
    fn from(record: &UsenetDownloadRecord) -> Self {
        Self {
            id: record.id,
            name: record.nzb_name.clone(),
            state: record.state.clone(),
            progress: record
                .progress
                .map(|p| p.to_string().parse::<f64>().unwrap_or(0.0))
                .unwrap_or(0.0),
            size_bytes: record.size_bytes,
            downloaded_bytes: record.downloaded_bytes.unwrap_or(0),
            download_speed: record.download_speed.unwrap_or(0),
            eta_seconds: record.eta_seconds,
            error_message: record.error_message.clone(),
            download_path: record.download_path.clone(),
            library_id: record.library_id,
            episode_id: record.episode_id,
            movie_id: record.movie_id,
            album_id: record.album_id,
            audiobook_id: record.audiobook_id,
        }
    }
}

/// Active download tracking
struct ActiveDownload {
    pub id: Uuid,
    pub nzb_data: Vec<u8>,
    pub download_path: PathBuf,
    pub cancel_token: tokio_util::sync::CancellationToken,
}

/// Usenet download service configuration
#[derive(Debug, Clone)]
pub struct UsenetServiceConfig {
    /// Base directory for downloads
    pub downloads_path: PathBuf,
    /// Maximum concurrent downloads
    pub max_concurrent_downloads: usize,
    /// Event channel capacity
    pub event_channel_capacity: usize,
}

impl Default for UsenetServiceConfig {
    fn default() -> Self {
        Self {
            downloads_path: PathBuf::from("/downloads"),
            max_concurrent_downloads: 3,
            event_channel_capacity: 1024,
        }
    }
}

/// Usenet download service
pub struct UsenetService {
    db: Database,
    config: UsenetServiceConfig,
    /// Active downloads being processed
    active_downloads: RwLock<HashMap<Uuid, ActiveDownload>>,
    /// Event broadcaster
    event_tx: broadcast::Sender<UsenetEvent>,
}

impl UsenetService {
    /// Create a new UsenetService
    pub fn new(db: Database, config: UsenetServiceConfig) -> Self {
        let (event_tx, _) = broadcast::channel(config.event_channel_capacity);

        Self {
            db,
            config,
            active_downloads: RwLock::new(HashMap::new()),
            event_tx,
        }
    }

    /// Subscribe to usenet events
    pub fn subscribe(&self) -> broadcast::Receiver<UsenetEvent> {
        self.event_tx.subscribe()
    }

    /// Add an NZB download from raw bytes
    pub async fn add_nzb(
        &self,
        nzb_data: &[u8],
        name: String,
        user_id: Uuid,
        library_id: Option<Uuid>,
        episode_id: Option<Uuid>,
        movie_id: Option<Uuid>,
        album_id: Option<Uuid>,
        audiobook_id: Option<Uuid>,
        indexer_id: Option<Uuid>,
    ) -> Result<UsenetDownloadInfo> {
        // Calculate NZB hash for deduplication
        let nzb_hash = format!("{:x}", md5::compute(nzb_data));

        // Check if already exists
        if let Some(existing) = self.db.usenet_downloads().get_by_hash(&nzb_hash).await? {
            info!(
                nzb_hash = %nzb_hash,
                existing_id = %existing.id,
                "NZB already exists in database"
            );
            return Ok(UsenetDownloadInfo::from(&existing));
        }

        // Create download directory
        let download_dir = self.config.downloads_path.join(&nzb_hash);

        // Create database record
        let record = self
            .db
            .usenet_downloads()
            .create(CreateUsenetDownload {
                user_id,
                nzb_name: name.clone(),
                nzb_hash: Some(nzb_hash.clone()),
                size_bytes: None, // Will be determined from NZB parsing
                download_path: Some(download_dir.to_string_lossy().to_string()),
                library_id,
                episode_id,
                movie_id,
                album_id,
                audiobook_id,
                indexer_id,
            })
            .await?;

        let info = UsenetDownloadInfo::from(&record);

        // Broadcast event
        let _ = self.event_tx.send(UsenetEvent::Added(info.clone()));

        info!(
            id = %record.id,
            name = %name,
            nzb_hash = %nzb_hash,
            "Added usenet download"
        );

        // TODO: Start the actual download in the background
        // This will be implemented when the NNTP client is ready
        // self.start_download(record.id, nzb_data.to_vec()).await?;

        Ok(info)
    }

    /// Add an NZB download from a URL
    pub async fn add_nzb_url(
        &self,
        url: &str,
        user_id: Uuid,
        library_id: Option<Uuid>,
        episode_id: Option<Uuid>,
        movie_id: Option<Uuid>,
        album_id: Option<Uuid>,
        audiobook_id: Option<Uuid>,
        indexer_id: Option<Uuid>,
    ) -> Result<UsenetDownloadInfo> {
        // Download the NZB file
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download NZB: HTTP {}",
                response.status()
            ));
        }

        let nzb_data = response.bytes().await?;

        // Extract filename from URL or Content-Disposition
        let name = url
            .split('/')
            .last()
            .and_then(|s| s.split('?').next())
            .unwrap_or("download")
            .trim_end_matches(".nzb")
            .to_string();

        self.add_nzb(
            &nzb_data,
            name,
            user_id,
            library_id,
            episode_id,
            movie_id,
            album_id,
            audiobook_id,
            indexer_id,
        )
        .await
    }

    /// Pause a download
    pub async fn pause(&self, id: Uuid) -> Result<()> {
        // Cancel the active download task if running
        if let Some(download) = self.active_downloads.write().get(&id) {
            download.cancel_token.cancel();
        }

        self.db.usenet_downloads().pause(id).await?;

        let _ = self.event_tx.send(UsenetEvent::StateChanged {
            id,
            old_state: "downloading".to_string(),
            new_state: "paused".to_string(),
        });

        info!(id = %id, "Paused usenet download");
        Ok(())
    }

    /// Resume a paused download
    pub async fn resume(&self, id: Uuid) -> Result<()> {
        self.db.usenet_downloads().resume(id).await?;

        let _ = self.event_tx.send(UsenetEvent::StateChanged {
            id,
            old_state: "paused".to_string(),
            new_state: "queued".to_string(),
        });

        info!(id = %id, "Resumed usenet download");

        // TODO: Restart the download task
        Ok(())
    }

    /// Remove a download
    pub async fn remove(&self, id: Uuid, delete_files: bool) -> Result<()> {
        // Cancel the active download task if running
        if let Some(download) = self.active_downloads.write().remove(&id) {
            download.cancel_token.cancel();

            // Delete files if requested
            if delete_files {
                if let Err(e) = std::fs::remove_dir_all(&download.download_path) {
                    warn!(
                        id = %id,
                        path = %download.download_path.display(),
                        error = %e,
                        "Failed to delete download files"
                    );
                }
            }
        }

        self.db.usenet_downloads().remove(id).await?;

        let _ = self.event_tx.send(UsenetEvent::Removed(id));

        info!(id = %id, delete_files = delete_files, "Removed usenet download");
        Ok(())
    }

    /// List all downloads for a user
    pub async fn list(&self, user_id: Uuid) -> Result<Vec<UsenetDownloadInfo>> {
        let records = self.db.usenet_downloads().list_by_user(user_id).await?;
        Ok(records.iter().map(UsenetDownloadInfo::from).collect())
    }

    /// List active downloads for a user
    pub async fn list_active(&self, user_id: Uuid) -> Result<Vec<UsenetDownloadInfo>> {
        let records = self.db.usenet_downloads().list_active(user_id).await?;
        Ok(records.iter().map(UsenetDownloadInfo::from).collect())
    }

    /// Get a specific download
    pub async fn get(&self, id: Uuid) -> Result<Option<UsenetDownloadInfo>> {
        let record = self.db.usenet_downloads().get(id).await?;
        Ok(record.as_ref().map(UsenetDownloadInfo::from))
    }

    /// Link download to library item
    pub async fn link_to_library(
        &self,
        id: Uuid,
        library_id: Option<Uuid>,
        episode_id: Option<Uuid>,
        movie_id: Option<Uuid>,
        album_id: Option<Uuid>,
        audiobook_id: Option<Uuid>,
    ) -> Result<()> {
        self.db
            .usenet_downloads()
            .link_to_library(id, library_id, episode_id, movie_id, album_id, audiobook_id)
            .await?;

        info!(
            id = %id,
            library_id = ?library_id,
            episode_id = ?episode_id,
            movie_id = ?movie_id,
            album_id = ?album_id,
            audiobook_id = ?audiobook_id,
            "Linked usenet download to library item"
        );

        Ok(())
    }

    /// Get downloads pending post-processing
    pub async fn list_pending_processing(&self) -> Result<Vec<UsenetDownloadInfo>> {
        let records = self.db.usenet_downloads().list_pending_processing().await?;
        Ok(records.iter().map(UsenetDownloadInfo::from).collect())
    }

    /// List usenet downloads pending processing (raw records)
    pub async fn list_pending_processing_records(&self) -> Result<Vec<crate::db::usenet_downloads::UsenetDownloadRecord>> {
        self.db.usenet_downloads().list_pending_processing().await
    }

    /// Update post-processing status
    pub async fn set_post_process_status(&self, id: Uuid, status: &str) -> Result<()> {
        self.db
            .usenet_downloads()
            .set_post_process_status(id, status)
            .await?;
        Ok(())
    }

    // TODO: Implement the actual download logic when NNTP client is ready
    // async fn start_download(&self, id: Uuid, nzb_data: Vec<u8>) -> Result<()> { ... }
}

impl std::fmt::Debug for UsenetService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UsenetService")
            .field("downloads_path", &self.config.downloads_path)
            .field(
                "active_downloads",
                &self.active_downloads.read().len(),
            )
            .finish()
    }
}
