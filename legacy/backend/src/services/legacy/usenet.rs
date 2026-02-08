//! Usenet Download Service
//!
//! Manages Usenet downloads using native NNTP client.
//! Parallel to TorrentService but for NZB-based downloads.

use std::collections::HashMap;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::{CreateUsenetDownload, Database, UsenetDownloadRecord, UsenetServerRecord};
use crate::indexer::encryption::CredentialEncryption;
use crate::usenet::{NntpClient, NntpConfig, NzbFile, NzbFileEntry, decode_yenc};

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
    pub download_path: PathBuf,
    pub cancel_token: CancellationToken,
    /// Track download progress
    pub downloaded_bytes: Arc<AtomicU64>,
    pub total_bytes: u64,
    pub is_running: Arc<AtomicBool>,
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
    active_downloads: Arc<RwLock<HashMap<Uuid, ActiveDownload>>>,
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
            active_downloads: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
    }

    /// Get the encryption service for decrypting server passwords
    async fn get_encryption(&self) -> Result<CredentialEncryption> {
        let key = self
            .db
            .settings()
            .get_or_create_indexer_encryption_key()
            .await?;
        CredentialEncryption::from_base64_key(&key)
    }

    /// Get enabled usenet servers for a user, with decrypted passwords
    async fn get_servers_for_user(&self, user_id: Uuid) -> Result<Vec<(UsenetServerRecord, Option<String>)>> {
        let servers = self.db.usenet_servers().list_enabled_by_user(user_id).await?;
        
        if servers.is_empty() {
            return Err(anyhow!("No usenet servers configured"));
        }

        let encryption = self.get_encryption().await?;
        let mut result = Vec::with_capacity(servers.len());

        for server in servers {
            let password = if let (Some(enc_pass), Some(nonce)) = 
                (&server.encrypted_password, &server.password_nonce) 
            {
                match encryption.decrypt(enc_pass, nonce) {
                    Ok(pass) => Some(pass),
                    Err(e) => {
                        warn!(server_id = %server.id, error = %e, "Failed to decrypt server password");
                        None
                    }
                }
            } else {
                None
            };
            result.push((server, password));
        }

        Ok(result)
    }

    /// Create an NNTP config from a server record
    fn create_nntp_config(server: &UsenetServerRecord, password: Option<String>) -> NntpConfig {
        NntpConfig {
            host: server.host.clone(),
            port: server.port as u16,
            use_tls: server.use_ssl,
            username: server.username.clone(),
            password,
            timeout: Duration::from_secs(30),
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

        // Start the actual download in the background
        self.start_download(record.id, user_id, nzb_data.to_vec(), download_dir).await?;

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

        // Note: Resume requires storing NZB data - for now, user needs to re-add the NZB
        // A full implementation would store the NZB in the database or on disk
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
    pub async fn list_pending_processing_records(&self) -> Result<Vec<crate::db::UsenetDownloadRecord>> {
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

    /// Start downloading an NZB in the background
    async fn start_download(
        &self,
        download_id: Uuid,
        user_id: Uuid,
        nzb_data: Vec<u8>,
        download_path: PathBuf,
    ) -> Result<()> {
        // Parse the NZB
        let nzb = NzbFile::parse(&nzb_data)?;
        let total_bytes = nzb.total_size;

        info!(
            id = %download_id,
            files = nzb.files.len(),
            segments = nzb.total_segments(),
            total_bytes = total_bytes,
            "Starting usenet download"
        );

        // Update database with size
        self.db
            .usenet_downloads()
            .update_progress(download_id, 0.0, 0, 0, None)
            .await?;

        // Get servers for this user
        let servers = self.get_servers_for_user(user_id).await?;

        // Create download directory
        std::fs::create_dir_all(&download_path)?;

        // Setup cancellation and progress tracking
        let cancel_token = CancellationToken::new();
        let downloaded_bytes = Arc::new(AtomicU64::new(0));
        let is_running = Arc::new(AtomicBool::new(true));

        // Register active download
        {
            let mut downloads = self.active_downloads.write();
            downloads.insert(download_id, ActiveDownload {
                id: download_id,
                download_path: download_path.clone(),
                cancel_token: cancel_token.clone(),
                downloaded_bytes: downloaded_bytes.clone(),
                total_bytes,
                is_running: is_running.clone(),
            });
        }

        // Clone what we need for the background task
        let db = self.db.clone();
        let event_tx = self.event_tx.clone();
        let active_downloads = self.active_downloads.clone();

        // Spawn background download task
        tokio::spawn(async move {
            let result = Self::download_task(
                download_id,
                nzb,
                servers,
                download_path.clone(),
                cancel_token.clone(),
                downloaded_bytes.clone(),
                db.clone(),
                event_tx.clone(),
            )
            .await;

            // Mark as not running
            is_running.store(false, Ordering::SeqCst);

            // Remove from active downloads
            {
                let mut downloads = active_downloads.write();
                downloads.remove(&download_id);
            }

            match result {
                Ok(()) => {
                    info!(id = %download_id, "Usenet download completed");
                    
                    // Mark as completed in database
                    if let Err(e) = db
                        .usenet_downloads()
                        .mark_completed(download_id, download_path.to_string_lossy().as_ref())
                        .await
                    {
                        error!(id = %download_id, error = %e, "Failed to mark download complete");
                    }

                    // Broadcast completion event
                    if let Ok(Some(record)) = db.usenet_downloads().get(download_id).await {
                        let _ = event_tx.send(UsenetEvent::Completed(UsenetDownloadInfo::from(&record)));
                    }
                }
                Err(e) => {
                    // Check if cancelled
                    if cancel_token.is_cancelled() {
                        info!(id = %download_id, "Usenet download cancelled");
                    } else {
                        error!(id = %download_id, error = %e, "Usenet download failed");
                        
                        // Mark as failed in database
                        if let Err(db_err) = db
                            .usenet_downloads()
                            .mark_failed(download_id, &e.to_string())
                            .await
                        {
                            error!(id = %download_id, error = %db_err, "Failed to mark download as failed");
                        }

                        // Broadcast failure event
                        let _ = event_tx.send(UsenetEvent::Failed {
                            id: download_id,
                            error: e.to_string(),
                        });
                    }
                }
            }
        });

        Ok(())
    }

    /// The actual download task that runs in background
    async fn download_task(
        download_id: Uuid,
        nzb: NzbFile,
        servers: Vec<(UsenetServerRecord, Option<String>)>,
        download_path: PathBuf,
        cancel_token: CancellationToken,
        downloaded_bytes: Arc<AtomicU64>,
        db: Database,
        event_tx: broadcast::Sender<UsenetEvent>,
    ) -> Result<()> {
        let total_bytes = nzb.total_size;
        let mut last_progress_update = Instant::now();
        let mut last_downloaded = 0u64;

        // Process each file in the NZB
        for file_entry in &nzb.files {
            if cancel_token.is_cancelled() {
                return Err(anyhow!("Download cancelled"));
            }

            // Download single file
            Self::download_file(
                download_id,
                file_entry,
                &servers,
                &download_path,
                &cancel_token,
                &downloaded_bytes,
                total_bytes,
                &mut last_progress_update,
                &mut last_downloaded,
                &db,
                &event_tx,
            )
            .await?;
        }

        Ok(())
    }

    /// Download a single file from the NZB
    async fn download_file(
        download_id: Uuid,
        file_entry: &NzbFileEntry,
        servers: &[(UsenetServerRecord, Option<String>)],
        download_path: &PathBuf,
        cancel_token: &CancellationToken,
        downloaded_bytes: &Arc<AtomicU64>,
        total_bytes: u64,
        last_progress_update: &mut Instant,
        last_downloaded: &mut u64,
        db: &Database,
        event_tx: &broadcast::Sender<UsenetEvent>,
    ) -> Result<()> {
        let file_path = download_path.join(&file_entry.filename);
        
        debug!(
            file = %file_entry.filename,
            segments = file_entry.segments.len(),
            size = file_entry.size,
            "Downloading file"
        );

        // Create the output file
        let mut output_file = std::fs::File::create(&file_path)?;

        // Sort segments by number
        let mut segments = file_entry.segments.clone();
        segments.sort_by_key(|s| s.number);

        // Download each segment
        for segment in &segments {
            if cancel_token.is_cancelled() {
                return Err(anyhow!("Download cancelled"));
            }

            // Try each server until one succeeds
            let mut article_data = None;
            let mut last_error = None;

            for (server, password) in servers {
                match Self::fetch_article_from_server(
                    server,
                    password.clone(),
                    &file_entry.groups,
                    &segment.message_id,
                )
                .await
                {
                    Ok(data) => {
                        // Record success
                        let _ = db.usenet_servers().record_success(server.id).await;
                        article_data = Some(data);
                        break;
                    }
                    Err(e) => {
                        debug!(
                            server = %server.name,
                            message_id = %segment.message_id,
                            error = %e,
                            "Failed to fetch article, trying next server"
                        );
                        // Record error
                        let _ = db.usenet_servers().record_error(server.id, &e.to_string()).await;
                        last_error = Some(e);
                    }
                }
            }

            let article_data = article_data.ok_or_else(|| {
                last_error.unwrap_or_else(|| anyhow!("No servers available"))
            })?;

            // Decode yEnc
            let decoded = decode_yenc(&article_data)?;

            // Write to file at correct position (for multi-part)
            if let Some(begin) = decoded.begin {
                output_file.seek(SeekFrom::Start(begin - 1))?;
            }
            output_file.write_all(&decoded.data)?;

            // Update progress
            let segment_bytes = segment.bytes;
            let current = downloaded_bytes.fetch_add(segment_bytes, Ordering::SeqCst) + segment_bytes;

            // Throttle progress updates to every 500ms
            if last_progress_update.elapsed() >= Duration::from_millis(500) {
                let elapsed = last_progress_update.elapsed().as_secs_f64();
                let bytes_since_last = current - *last_downloaded;
                let speed = if elapsed > 0.0 {
                    (bytes_since_last as f64 / elapsed) as i64
                } else {
                    0
                };

                let progress = if total_bytes > 0 {
                    (current as f64 / total_bytes as f64) * 100.0
                } else {
                    0.0
                };

                let remaining_bytes = total_bytes.saturating_sub(current);
                let eta = if speed > 0 {
                    Some((remaining_bytes as i64 / speed) as i32)
                } else {
                    None
                };

                // Update database
                let _ = db
                    .usenet_downloads()
                    .update_progress(download_id, progress, current as i64, speed, eta)
                    .await;

                // Broadcast progress event
                let _ = event_tx.send(UsenetEvent::Progress(UsenetProgressUpdate {
                    id: download_id,
                    progress,
                    downloaded_bytes: current as i64,
                    total_bytes: Some(total_bytes as i64),
                    speed_bytes_sec: speed,
                    eta_seconds: eta,
                }));

                *last_progress_update = Instant::now();
                *last_downloaded = current;
            }
        }

        output_file.sync_all()?;
        
        info!(
            file = %file_entry.filename,
            size = file_entry.size,
            "File download complete"
        );

        Ok(())
    }

    /// Fetch an article from a specific server
    async fn fetch_article_from_server(
        server: &UsenetServerRecord,
        password: Option<String>,
        groups: &[String],
        message_id: &str,
    ) -> Result<Vec<u8>> {
        // Run NNTP operations in blocking task since they use std IO
        let config = Self::create_nntp_config(server, password);
        let groups = groups.to_vec();
        let message_id = message_id.to_string();

        tokio::task::spawn_blocking(move || {
            let mut client = NntpClient::new(config);
            
            // Connect
            client.connect()?;

            // Try to select a group (optional for most servers with article by message-id)
            if let Some(group) = groups.first() {
                if let Err(e) = client.group(group) {
                    debug!(group = %group, error = %e, "Failed to select group, continuing anyway");
                }
            }

            // Fetch article body
            let body = client.body(&message_id)?;

            // Disconnect
            client.quit()?;

            Ok(body)
        })
        .await?
    }
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
