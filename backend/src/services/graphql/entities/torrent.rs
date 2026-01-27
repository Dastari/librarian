use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Result, SimpleObject};
use librarian_macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::super::auth::AuthExt;
use super::torrent_file::TorrentFile;
use crate::services::torrent::{
    TorrentInfo as ServiceTorrentInfo, TorrentService, TorrentState as ServiceTorrentState,
};
use crate::services::ServicesManager;

#[derive(
    GraphQLEntity,
    GraphQLRelations,
    GraphQLOperations,
    SimpleObject,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[graphql(name = "Torrent", complex)]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "torrents", plural = "Torrents", default_sort = "added_at")]
pub struct Torrent {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "InfoHash")]
    #[filterable(type = "string")]
    pub info_hash: String,

    #[graphql(name = "MagnetUri")]
    pub magnet_uri: Option<String>,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "State")]
    #[filterable(type = "string")]
    #[sortable]
    pub state: String,

    #[graphql(name = "Progress")]
    #[filterable(type = "number")]
    #[sortable]
    pub progress: f64,

    #[graphql(name = "TotalBytes")]
    #[filterable(type = "number")]
    #[sortable]
    pub total_bytes: i64,

    #[graphql(name = "DownloadedBytes")]
    #[filterable(type = "number")]
    pub downloaded_bytes: i64,

    #[graphql(name = "UploadedBytes")]
    #[filterable(type = "number")]
    pub uploaded_bytes: i64,

    #[graphql(name = "SavePath")]
    #[filterable(type = "string")]
    pub save_path: String,

    #[graphql(name = "DownloadPath")]
    pub download_path: Option<String>,

    #[graphql(name = "SourceUrl")]
    pub source_url: Option<String>,

    #[graphql(name = "SourceFeedId")]
    #[filterable(type = "string")]
    pub source_feed_id: Option<String>,

    #[graphql(name = "SourceIndexerId")]
    #[filterable(type = "string")]
    pub source_indexer_id: Option<String>,

    #[graphql(name = "LibraryId")]
    #[filterable(type = "string")]
    pub library_id: Option<String>,

    #[graphql(name = "PostProcessStatus")]
    #[filterable(type = "string")]
    pub post_process_status: Option<String>,

    #[graphql(name = "PostProcessError")]
    pub post_process_error: Option<String>,

    #[graphql(name = "ProcessedAt")]
    #[filterable(type = "date")]
    pub processed_at: Option<String>,

    #[graphql(name = "ExcludedFiles")]
    #[json_field]
    pub excluded_files: Vec<i32>,

    #[graphql(name = "AddedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub added_at: String,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "CompletedAt")]
    #[filterable(type = "date")]
    pub completed_at: Option<String>,

    #[graphql(skip)]
    #[serde(skip)]
    #[relation(target = "TorrentFile", from = "id", to = "torrent_id", multiple)]
    pub files: Vec<TorrentFile>,
}

#[derive(Default)]
pub struct TorrentCustomOperations;

// =============================================================================
// GraphQL types for custom operations (live client state; PascalCase)
// =============================================================================

/// Live torrent file (from torrent client)
#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "LiveTorrentFile")]
pub struct LiveTorrentFile {
    #[graphql(name = "Index")]
    pub index: i32,
    #[graphql(name = "Path")]
    pub path: String,
    #[graphql(name = "Size")]
    pub size: i64,
    #[graphql(name = "Progress")]
    pub progress: f64,
}

/// Live torrent (from torrent client, not DB)
#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "LiveTorrent")]
pub struct LiveTorrent {
    #[graphql(name = "Id")]
    pub id: i32,
    #[graphql(name = "InfoHash")]
    pub info_hash: String,
    #[graphql(name = "Name")]
    pub name: String,
    #[graphql(name = "State")]
    pub state: String,
    #[graphql(name = "Progress")]
    pub progress: f64,
    #[graphql(name = "Size")]
    pub size: i64,
    #[graphql(name = "Downloaded")]
    pub downloaded: i64,
    #[graphql(name = "Uploaded")]
    pub uploaded: i64,
    #[graphql(name = "DownloadSpeed")]
    pub download_speed: i64,
    #[graphql(name = "UploadSpeed")]
    pub upload_speed: i64,
    #[graphql(name = "Peers")]
    pub peers: i32,
    #[graphql(name = "SavePath")]
    pub save_path: String,
    #[graphql(name = "Files")]
    pub files: Vec<LiveTorrentFile>,
}

fn service_state_to_string(s: ServiceTorrentState) -> &'static str {
    match s {
        ServiceTorrentState::Queued => "queued",
        ServiceTorrentState::Checking => "checking",
        ServiceTorrentState::Downloading => "downloading",
        ServiceTorrentState::Seeding => "seeding",
        ServiceTorrentState::Paused => "paused",
        ServiceTorrentState::Error => "error",
    }
}

fn service_torrent_to_live(t: ServiceTorrentInfo) -> LiveTorrent {
    LiveTorrent {
        id: t.id as i32,
        info_hash: t.info_hash,
        name: t.name,
        state: service_state_to_string(t.state).to_string(),
        progress: t.progress,
        size: t.size as i64,
        downloaded: t.downloaded as i64,
        uploaded: t.uploaded as i64,
        download_speed: t.download_speed as i64,
        upload_speed: t.upload_speed as i64,
        peers: t.peers as i32,
        save_path: t.save_path,
        files: t
            .files
            .into_iter()
            .map(|f| LiveTorrentFile {
                index: f.index as i32,
                path: f.path,
                size: f.size as i64,
                progress: f.progress,
            })
            .collect(),
    }
}

/// Input for adding a torrent
#[derive(InputObject)]
#[graphql(name = "AddTorrentInput")]
pub struct AddTorrentInput {
    #[graphql(name = "Magnet")]
    pub magnet: Option<String>,
    #[graphql(name = "Url")]
    pub url: Option<String>,
}

/// Result of add torrent mutation
#[derive(Debug, SimpleObject)]
#[graphql(name = "AddTorrentResult")]
pub struct AddTorrentResult {
    #[graphql(name = "Success")]
    pub success: bool,
    #[graphql(name = "Torrent")]
    pub torrent: Option<LiveTorrent>,
    #[graphql(name = "Error")]
    pub error: Option<String>,
}

/// Result of pause/resume/remove
#[derive(Debug, SimpleObject)]
#[graphql(name = "TorrentActionResult")]
pub struct TorrentActionResult {
    #[graphql(name = "Success")]
    pub success: bool,
    #[graphql(name = "Error")]
    pub error: Option<String>,
}

#[Object]
impl TorrentCustomOperations {
    /// Get all torrents with live state from the torrent client
    #[graphql(name = "LiveTorrents")]
    async fn live_torrents(&self, ctx: &Context<'_>) -> Result<Vec<LiveTorrent>> {
        let _user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let list = service.list_torrents().await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(list.into_iter().map(service_torrent_to_live).collect())
    }

    /// Get a single live torrent by numeric id
    #[graphql(name = "LiveTorrent")]
    async fn live_torrent(&self, ctx: &Context<'_>, #[graphql(name = "Id")] id: i32) -> Result<Option<LiveTorrent>> {
        let _user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager.get_torrent().await;
        Ok(match service {
            Some(svc) => svc
                .get_torrent_info(id as usize)
                .await
                .ok()
                .map(service_torrent_to_live),
            None => None,
        })
    }

    /// Count of active (downloading/checking) torrents
    #[graphql(name = "ActiveDownloadCount")]
    async fn active_download_count(&self, ctx: &Context<'_>) -> Result<i32> {
        let _user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service: Arc<TorrentService> = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let list: Vec<ServiceTorrentInfo> = service.list_active_downloads().await.map_err(|e: anyhow::Error| async_graphql::Error::new(e.to_string()))?;
        Ok(list.len() as i32)
    }
}

/// Mutations that use the torrent client (add, pause, resume, remove)
#[derive(Default)]
pub struct TorrentClientMutations;

#[Object]
impl TorrentClientMutations {
    /// Add a torrent from a magnet link or URL
    #[graphql(name = "AddTorrent")]
    async fn add_torrent(&self, ctx: &Context<'_>, #[graphql(name = "Input")] input: AddTorrentInput) -> Result<AddTorrentResult> {
        let user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        let user_id = Uuid::parse_str(&user.user_id).ok();
        let result = if let Some(ref magnet) = input.magnet {
            service.add_magnet(magnet, user_id).await
        } else if let Some(ref url) = input.url {
            service.add_magnet(url, user_id).await
        } else {
            return Ok(AddTorrentResult {
                success: false,
                torrent: None,
                error: Some("Either Magnet or Url must be provided".to_string()),
            });
        };
        match result {
            Ok(info) => Ok(AddTorrentResult {
                success: true,
                torrent: Some(service_torrent_to_live(info)),
                error: None,
            }),
            Err(e) => Ok(AddTorrentResult {
                success: false,
                torrent: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Pause a torrent
    #[graphql(name = "PauseTorrent")]
    async fn pause_torrent(&self, ctx: &Context<'_>, #[graphql(name = "Id")] id: i32) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        match service.pause(id as usize).await {
            Ok(()) => Ok(TorrentActionResult { success: true, error: None }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Resume a paused torrent
    #[graphql(name = "ResumeTorrent")]
    async fn resume_torrent(&self, ctx: &Context<'_>, #[graphql(name = "Id")] id: i32) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        match service.resume(id as usize).await {
            Ok(()) => Ok(TorrentActionResult { success: true, error: None }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Remove a torrent
    #[graphql(name = "RemoveTorrent")]
    async fn remove_torrent(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Id")] id: i32,
        #[graphql(name = "DeleteFiles", default = false)] delete_files: bool,
    ) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        match service.remove(id as usize, delete_files).await {
            Ok(()) => Ok(TorrentActionResult { success: true, error: None }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    #[graphql(name = "PauseTorrentByInfoHash")]
    async fn pause_torrent_by_info_hash(&self, ctx: &Context<'_>, #[graphql(name = "InfoHash")] info_hash: String) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager.get_torrent().await.ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        match service.pause_by_info_hash(&info_hash).await {
            Ok(()) => Ok(TorrentActionResult { success: true, error: None }),
            Err(e) => Ok(TorrentActionResult { success: false, error: Some(e.to_string()) }),
        }
    }

    #[graphql(name = "ResumeTorrentByInfoHash")]
    async fn resume_torrent_by_info_hash(&self, ctx: &Context<'_>, #[graphql(name = "InfoHash")] info_hash: String) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager.get_torrent().await.ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        match service.resume_by_info_hash(&info_hash).await {
            Ok(()) => Ok(TorrentActionResult { success: true, error: None }),
            Err(e) => Ok(TorrentActionResult { success: false, error: Some(e.to_string()) }),
        }
    }

    #[graphql(name = "RemoveTorrentByInfoHash")]
    async fn remove_torrent_by_info_hash(&self, ctx: &Context<'_>, #[graphql(name = "InfoHash")] info_hash: String, #[graphql(name = "DeleteFiles", default = false)] delete_files: bool) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager.get_torrent().await.ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        match service.remove_by_info_hash(&info_hash, delete_files).await {
            Ok(()) => Ok(TorrentActionResult { success: true, error: None }),
            Err(e) => Ok(TorrentActionResult { success: false, error: Some(e.to_string()) }),
        }
    }
}

// =============================================================================
// Legacy commented sections below (organize, rematch, process, import, settings,
// pending file matches, helpers) – re-enable when dependent services exist.
// =============================================================================

//     /// Get pending file matches for a source (legacy)
//     #[graphql(name = "AddTorrent")]
//     async fn add_torrent(
//         &self,
//         ctx: &Context<'_>,
//         input: AddTorrentInput,
//     ) -> Result<AddTorrentResult> {
//         let user = ctx.auth_user()?;
//         let service = ctx.data_unchecked::<Arc<TorrentService>>();
//         let db = ctx.data_unchecked::<Database>();

//         let user_id = Uuid::parse_str(&user.user_id).ok();
//         let album_id = input.album_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());
//         let movie_id = input.movie_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());
//         let episode_id = input.episode_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());

//         let result = if let Some(magnet) = input.magnet {
//             service.add_magnet(&magnet, user_id).await
//         } else if let Some(url) = input.url {
//             // Check if we need authenticated download via indexer
//             if let Some(ref indexer_id_str) = input.indexer_id {
//                 if let Ok(indexer_uuid) = Uuid::parse_str(indexer_id_str) {
//                     // Get indexer config
//                     let config = EntityQuery::<IndexerConfigEntity>::new()
//                         .filter(&IndexerConfigEntityWhereInput {
//                             id: Some(StringFilter::eq(indexer_id_str)),
//                             ..Default::default()
//                         })
//                         .fetch_one(db.pool())
//                         .await?;

//                     if let Some(config) = config {
//                         // Get credentials
//                         let credentials = EntityQuery::<IndexerCredentialEntity>::new()
//                             .filter(&IndexerCredentialEntityWhereInput {
//                                 indexer_config_id: Some(StringFilter::eq(indexer_id_str)),
//                                 ..Default::default()
//                             })
//                             .fetch_all(db.pool())
//                             .await
//                             .unwrap_or_default();

//                         // Get encryption key
//                         let encryption_key = self.get_or_create_setting(
//                             db,
//                             "indexer_encryption_key",
//                             "security",
//                         ).await;

//                         if let Ok(Some(encryption_key)) = encryption_key {
//                             if let Ok(encryption) = crate::indexer::encryption::CredentialEncryption::from_base64_key(&encryption_key) {
//                                 let mut decrypted_creds: std::collections::HashMap<String, String> = std::collections::HashMap::new();
//                                 for cred in credentials {
//                                     if let Ok(value) = encryption.decrypt(&cred.encrypted_value, &cred.nonce) {
//                                         decrypted_creds.insert(cred.credential_type, value);
//                                     }
//                                 }

//                                 let torrent_bytes = download_torrent_file_authenticated(
//                                     &url,
//                                     &config.indexer_type,
//                                     &decrypted_creds,
//                                 ).await;

//                                 if let Ok(bytes) = torrent_bytes {
//                                     match service.add_torrent_bytes(&bytes, user_id).await {
//                                         Ok(info) => {
//                                             create_file_matches_for_target(db, &info, album_id, movie_id, episode_id).await;
//                                             return Ok(AddTorrentResult {
//                                                 success: true,
//                                                 torrent: Some(info.into()),
//                                                 error: None,
//                                             });
//                                         }
//                                         Err(e) => {
//                                             return Ok(AddTorrentResult {
//                                                 success: false,
//                                                 torrent: None,
//                                                 error: Some(e.to_string()),
//                                             });
//                                         }
//                                     }
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }
//             service.add_torrent_url(&url, user_id).await
//         } else {
//             return Ok(AddTorrentResult {
//                 success: false,
//                 torrent: None,
//                 error: Some("Either magnet or url must be provided".to_string()),
//             });
//         };

//         match result {
//             Ok(info) => {
//                 create_file_matches_for_target(db, &info, album_id, movie_id, episode_id).await;
//                 Ok(AddTorrentResult {
//                     success: true,
//                     torrent: Some(info.into()),
//                     error: None,
//                 })
//             }
//             Err(e) => Ok(AddTorrentResult {
//                 success: false,
//                 torrent: None,
//                 error: Some(e.to_string()),
//             }),
//         }
//     }

//     /// Pause a torrent
//     #[graphql(name = "PauseTorrent")]
//     async fn pause_torrent(&self, ctx: &Context<'_>, id: i32) -> Result<TorrentActionResult> {
//         let _user = ctx.auth_user()?;
//         let service = ctx.data_unchecked::<Arc<TorrentService>>();

//         match service.pause(id as usize).await {
//             Ok(_) => Ok(TorrentActionResult { success: true, error: None }),
//             Err(e) => Ok(TorrentActionResult { success: false, error: Some(e.to_string()) }),
//         }
//     }

//     /// Resume a paused torrent
//     #[graphql(name = "ResumeTorrent")]
//     async fn resume_torrent(&self, ctx: &Context<'_>, id: i32) -> Result<TorrentActionResult> {
//         let _user = ctx.auth_user()?;
//         let service = ctx.data_unchecked::<Arc<TorrentService>>();

//         match service.resume(id as usize).await {
//             Ok(_) => Ok(TorrentActionResult { success: true, error: None }),
//             Err(e) => Ok(TorrentActionResult { success: false, error: Some(e.to_string()) }),
//         }
//     }

//     /// Remove a torrent
//     #[graphql(name = "RemoveTorrent")]
//     async fn remove_torrent(
//         &self,
//         ctx: &Context<'_>,
//         id: i32,
//         #[graphql(name = "DeleteFiles", default = false)] delete_files: bool,
//     ) -> Result<TorrentActionResult> {
//         let _user = ctx.auth_user()?;
//         let service = ctx.data_unchecked::<Arc<TorrentService>>();

//         match service.remove(id as usize, delete_files).await {
//             Ok(_) => Ok(TorrentActionResult { success: true, error: None }),
//             Err(e) => Ok(TorrentActionResult { success: false, error: Some(e.to_string()) }),
//         }
//     }

//     /// Organize a completed torrent's files into the library structure
//     #[graphql(name = "OrganizeTorrent")]
//     async fn organize_torrent(
//         &self,
//         ctx: &Context<'_>,
//         id: i32,
//         #[graphql(name = "LibraryId", desc = "Optional library ID to limit matching scope")] library_id: Option<String>,
//         #[graphql(name = "AlbumId", desc = "Optional album ID for music torrents")] album_id: Option<String>,
//     ) -> Result<OrganizeTorrentResult> {
//         use crate::services::file_processor::FileProcessor;

//         let _user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();
//         let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();
//         let analysis_queue = ctx.data_opt::<Arc<crate::services::queues::MediaAnalysisQueue>>();

//         let torrent_info = match torrent_service.get_torrent_info(id as usize).await {
//             Ok(info) => info,
//             Err(e) => {
//                 return Ok(OrganizeTorrentResult {
//                     success: false,
//                     organized_count: 0,
//                     failed_count: 0,
//                     messages: vec![format!("Failed to get torrent info: {}", e)],
//                 });
//             }
//         };

//         // Get torrent record from database
//         let torrent_record = EntityQuery::<TorrentEntity>::new()
//             .filter(&TorrentEntityWhereInput {
//                 info_hash: Some(StringFilter::eq(&torrent_info.info_hash)),
//                 ..Default::default()
//             })
//             .fetch_one(db.pool())
//             .await?;

//         let torrent_record = match torrent_record {
//             Some(t) => t,
//             None => {
//                 return Ok(OrganizeTorrentResult {
//                     success: false,
//                     organized_count: 0,
//                     failed_count: 0,
//                     messages: vec!["Torrent not found in database".to_string()],
//                 });
//             }
//         };

//         let torrent_uuid = Uuid::parse_str(&torrent_record.id).unwrap_or_default();

//         // If album_id is provided, create file matches for it
//         if let Some(ref album_id_str) = album_id {
//             if let Ok(album_uuid) = Uuid::parse_str(album_id_str) {
//                 // Delete existing matches
//                 self.delete_pending_matches_by_source(db, "torrent", &torrent_record.id).await.ok();
//                 create_file_matches_for_target(db, &torrent_info, Some(album_uuid), None, None).await;
//             }
//         }

//         let processor = match analysis_queue {
//             Some(queue) => FileProcessor::with_analysis_queue(db.clone(), queue.clone()),
//             None => FileProcessor::new(db.clone()),
//         };

//         let result = processor.process_source("torrent", torrent_uuid).await;

//         match result {
//             Ok(proc_result) => Ok(OrganizeTorrentResult {
//                 success: proc_result.files_failed == 0,
//                 organized_count: proc_result.files_processed as i32,
//                 failed_count: proc_result.files_failed as i32,
//                 messages: proc_result.messages,
//             }),
//             Err(e) => Ok(OrganizeTorrentResult {
//                 success: false,
//                 organized_count: 0,
//                 failed_count: 0,
//                 messages: vec![format!("Processing failed: {}", e)],
//             }),
//         }
//     }

//     /// Update torrent client settings
//     #[graphql(name = "UpdateTorrentSettings")]
//     async fn update_torrent_settings(
//         &self,
//         ctx: &Context<'_>,
//         input: UpdateTorrentSettingsInput,
//     ) -> Result<SettingsResult> {
//         let _user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();

//         if let Some(v) = input.download_dir {
//             self.upsert_setting(db, "torrent.download_dir", &v, "torrent").await?;
//         }
//         if let Some(v) = input.session_dir {
//             self.upsert_setting(db, "torrent.session_dir", &v, "torrent").await?;
//         }
//         if let Some(v) = input.enable_dht {
//             self.upsert_setting(db, "torrent.enable_dht", &v.to_string(), "torrent").await?;
//         }
//         if let Some(v) = input.listen_port {
//             self.upsert_setting(db, "torrent.listen_port", &v.to_string(), "torrent").await?;
//         }
//         if let Some(v) = input.max_concurrent {
//             self.upsert_setting(db, "torrent.max_concurrent", &v.to_string(), "torrent").await?;
//         }
//         if let Some(v) = input.upload_limit {
//             self.upsert_setting(db, "torrent.upload_limit", &v.to_string(), "torrent").await?;
//         }
//         if let Some(v) = input.download_limit {
//             self.upsert_setting(db, "torrent.download_limit", &v.to_string(), "torrent").await?;
//         }

//         Ok(SettingsResult { success: true, error: None })
//     }

//     /// Re-match all files from a source
//     #[graphql(name = "RematchSource")]
//     async fn rematch_source(
//         &self,
//         ctx: &Context<'_>,
//         #[graphql(name = "SourceType")] source_type: String,
//         #[graphql(name = "SourceId")] source_id: String,
//         #[graphql(name = "LibraryId", desc = "Optional library ID to limit matching scope")] library_id: Option<String>,
//     ) -> Result<RematchSourceResult> {
//         use crate::services::file_matcher::{FileInfo, FileMatcher};

//         let user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();
//         let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();

//         let user_id = Uuid::parse_str(&user.user_id)?;

//         // Resolve source_id to UUID string
//         let (source_uuid_str, source_uuid) = if let Ok(uuid) = Uuid::parse_str(&source_id) {
//             (source_id.clone(), uuid)
//         } else if source_type == "torrent" {
//             let torrent = EntityQuery::<TorrentEntity>::new()
//                 .filter(&TorrentEntityWhereInput {
//                     info_hash: Some(StringFilter::eq(&source_id)),
//                     ..Default::default()
//                 })
//                 .fetch_one(db.pool())
//                 .await?
//                 .ok_or_else(|| async_graphql::Error::new("Torrent not found by info_hash"))?;
//             let uuid = Uuid::parse_str(&torrent.id)?;
//             (torrent.id, uuid)
//         } else {
//             return Err(async_graphql::Error::new("Invalid source ID"));
//         };

//         let target_library_id = library_id.as_ref().and_then(|s| Uuid::parse_str(s).ok());

//         // Delete existing matches
//         self.delete_pending_matches_by_source(db, &source_type, &source_uuid_str).await?;

//         // Get files for the source
//         let files: Vec<FileInfo> = if source_type == "torrent" {
//             let torrent = EntityQuery::<TorrentEntity>::new()
//                 .filter(&TorrentEntityWhereInput {
//                     id: Some(StringFilter::eq(&source_uuid_str)),
//                     ..Default::default()
//                 })
//                 .fetch_one(db.pool())
//                 .await?
//                 .ok_or_else(|| async_graphql::Error::new("Torrent not found"))?;

//             let torrent_files = torrent_service.get_files_for_torrent(&torrent.info_hash).await?;

//             torrent_files
//                 .iter()
//                 .enumerate()
//                 .map(|(idx, f)| FileInfo {
//                     path: f.path.clone(),
//                     size: f.size as i64,
//                     file_index: Some(idx as i32),
//                     source_name: Some(torrent.name.clone()),
//                 })
//                 .collect()
//         } else {
//             return Err(async_graphql::Error::new(format!("Unsupported source type: {}", source_type)));
//         };

//         let matcher = FileMatcher::new(db.clone());
//         let matches = matcher.match_files(user_id, files, target_library_id).await?;
//         let saved = matcher.save_matches(user_id, &source_type, Some(source_uuid), &matches).await?;

//         Ok(RematchSourceResult {
//             success: true,
//             match_count: saved.len() as i32,
//             error: None,
//         })
//     }

//     /// Process all pending matches for a source
//     #[graphql(name = "ProcessSource")]
//     async fn process_source(
//         &self,
//         ctx: &Context<'_>,
//         #[graphql(name = "SourceType")] source_type: String,
//         #[graphql(name = "SourceId")] source_id: String,
//     ) -> Result<ProcessSourceResult> {
//         use crate::services::file_processor::FileProcessor;

//         let _user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();
//         let analysis_queue = ctx.data_opt::<Arc<crate::services::queues::MediaAnalysisQueue>>();

//         let source_uuid = if let Ok(uuid) = Uuid::parse_str(&source_id) {
//             uuid
//         } else if source_type == "torrent" {
//             let torrent = EntityQuery::<TorrentEntity>::new()
//                 .filter(&TorrentEntityWhereInput {
//                     info_hash: Some(StringFilter::eq(&source_id)),
//                     ..Default::default()
//                 })
//                 .fetch_one(db.pool())
//                 .await?
//                 .ok_or_else(|| async_graphql::Error::new("Torrent not found by info_hash"))?;
//             Uuid::parse_str(&torrent.id)?
//         } else {
//             return Err(async_graphql::Error::new("Invalid source ID"));
//         };

//         let processor = match analysis_queue {
//             Some(queue) => FileProcessor::with_analysis_queue(db.clone(), queue.clone()),
//             None => FileProcessor::new(db.clone()),
//         };

//         let result = processor.process_source(&source_type, source_uuid).await?;

//         Ok(ProcessSourceResult {
//             success: result.files_failed == 0,
//             files_processed: result.files_processed as i32,
//             files_failed: result.files_failed as i32,
//             messages: result.messages,
//             error: None,
//         })
//     }

//     /// Import a completed torrent's files into a library
//     #[graphql(name = "ImportToLibrary")]
//     async fn import_to_library(
//         &self,
//         ctx: &Context<'_>,
//         #[graphql(name = "TorrentId", desc = "Torrent ID (numeric)")] torrent_id: i32,
//         #[graphql(name = "LibraryId", desc = "Target library ID")] library_id: String,
//     ) -> Result<ImportToLibraryResult> {
//         use crate::services::ScannerService;
//         use std::path::PathBuf;

//         let _user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();
//         let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();
//         let scanner = ctx.data_unchecked::<Arc<ScannerService>>();

//         let library_uuid = Uuid::parse_str(&library_id)?;

//         let torrent_info = match torrent_service.get_torrent_info(torrent_id as usize).await {
//             Ok(info) => info,
//             Err(e) => {
//                 return Ok(ImportToLibraryResult { success: false, files_copied: 0, error: Some(format!("Failed to get torrent info: {}", e)) });
//             }
//         };

//         if torrent_info.progress < 1.0 {
//             return Ok(ImportToLibraryResult { success: false, files_copied: 0, error: Some("Torrent is not complete".to_string()) });
//         }

//         let library = EntityQuery::<LibraryEntity>::new()
//             .filter(&super::LibraryEntityWhereInput {
//                 id: Some(StringFilter::eq(&library_id)),
//                 ..Default::default()
//             })
//             .fetch_one(db.pool())
//             .await?
//             .ok_or_else(|| async_graphql::Error::new("Library not found"))?;

//         let source_base = PathBuf::from(&torrent_info.save_path);
//         let dest_base = PathBuf::from(&library.path);

//         let (source_path, dest_path) = if torrent_info.files.len() == 1 {
//             (
//                 source_base.join(&torrent_info.files[0].path),
//                 dest_base.join(PathBuf::from(&torrent_info.files[0].path).file_name().unwrap_or_default()),
//             )
//         } else {
//             (
//                 source_base.join(&torrent_info.name),
//                 dest_base.join(&torrent_info.name),
//             )
//         };

//         let files_copied = if source_path.is_file() {
//             tokio::fs::copy(&source_path, &dest_path).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
//             1
//         } else if source_path.is_dir() {
//             copy_dir_recursive(&source_path, &dest_path).await?
//         } else {
//             return Ok(ImportToLibraryResult { success: false, files_copied: 0, error: Some("Source path does not exist".to_string()) });
//         };

//         tokio::spawn({
//             let scanner = scanner.clone();
//             async move {
//                 let _ = scanner.scan_library(library_uuid).await;
//             }
//         });

//         Ok(ImportToLibraryResult { success: true, files_copied: files_copied as i32, error: None })
//     }

//     /// Set a match target for a pending file match
//     #[graphql(name = "SetMatch")]
//     async fn set_match(
//         &self,
//         ctx: &Context<'_>,
//         #[graphql(name = "MatchId")] match_id: String,
//         #[graphql(name = "TargetType")] target_type: String,
//         #[graphql(name = "TargetId")] target_id: String,
//     ) -> Result<SetMatchResult> {
//         let _user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();

//         // Determine which column to update
//         let (column, clear_columns) = match target_type.to_lowercase().as_str() {
//             "episode" => ("episode_id", vec!["movie_id", "track_id", "chapter_id"]),
//             "movie" => ("movie_id", vec!["episode_id", "track_id", "chapter_id"]),
//             "track" => ("track_id", vec!["episode_id", "movie_id", "chapter_id"]),
//             "chapter" => ("chapter_id", vec!["episode_id", "movie_id", "track_id"]),
//             _ => return Err(async_graphql::Error::new("Invalid target type")),
//         };

//         // Build update SQL
//         let sql = format!(
//             "UPDATE pending_file_matches SET {} = ?1, {} = NULL, {} = NULL, {} = NULL, updated_at = datetime('now') WHERE id = ?2",
//             column, clear_columns[0], clear_columns[1], clear_columns[2]
//         );

//         execute_with_binds(
//             &sql,
//             &[SqlValue::String(target_id), SqlValue::String(match_id)],
//             db.pool(),
//         ).await?;

//         Ok(SetMatchResult { success: true, error: None })
//     }

//     /// Remove a pending file match
//     #[graphql(name = "RemoveMatch")]
//     async fn remove_match(&self, ctx: &Context<'_>, #[graphql(name = "MatchId")] match_id: String) -> Result<SetMatchResult> {
//         let _user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();

//         // Use auto-generated delete operation instead of custom SQL
//         PendingFileMatchEntity::delete(db.pool(), &match_id).await?;

//         Ok(SetMatchResult { success: true, error: None })
//     }

//     // =========================================================================
//     // Helper methods (moved from standalone functions)
//     // =========================================================================

//     async fn get_or_create_setting(&self, db: &Database, key: &str, category: &str) -> Result<Option<String>, sqlx::Error> {
//         // Try to get existing setting
//         let setting = EntityQuery::<AppSettingEntity>::new()
//             .filter(&AppSettingEntityWhereInput {
//                 key: Some(StringFilter::eq(key)),
//                 ..Default::default()
//             })
//             .fetch_one(db.pool())
//             .await?;

//         if let Some(s) = setting {
//             return Ok(Some(s.value));
//         }

//         // For encryption key, generate and save
//         if key == "indexer_encryption_key" {
//             use base64::Engine;
//             let mut key_bytes = [0u8; 32];
//             getrandom::getrandom(&mut key_bytes).ok();
//             let key_value = base64::engine::general_purpose::STANDARD.encode(key_bytes);

//             let id = Uuid::new_v4().to_string();
//             sqlx::query(
//                 "INSERT INTO app_settings (id, key, value, category, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))"
//             )
//             .bind(&id)
//             .bind(key)
//             .bind(&key_value)
//             .bind(category)
//             .execute(db.pool())
//             .await?;

//             return Ok(Some(key_value));
//         }

//         Ok(None)
//     }

//     async fn upsert_setting(&self, db: &Database, key: &str, value: &str, category: &str) -> Result<(), sqlx::Error> {
//         let id = Uuid::new_v4().to_string();
//         let sql = "INSERT INTO app_settings (id, key, value, category, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now')) ON CONFLICT(key) DO UPDATE SET value = ?3, updated_at = datetime('now')";
//         execute_with_binds(
//             sql,
//             &[
//                 SqlValue::String(id),
//                 SqlValue::String(key.to_string()),
//                 SqlValue::String(value.to_string()),
//                 SqlValue::String(category.to_string()),
//             ],
//             db.pool(),
//         ).await?;
//         Ok(())
//     }

//     async fn delete_pending_matches_by_source(&self, db: &Database, source_type: &str, source_id: &str) -> Result<(), sqlx::Error> {
//         execute_with_binds(
//             "DELETE FROM pending_file_matches WHERE source_type = ?1 AND source_id = ?2",
//             &[SqlValue::String(source_type.to_string()), SqlValue::String(source_id.to_string())],
//             db.pool(),
//         ).await?;
//         Ok(())
//     }
// }

// // =============================================================================
// // Helper Functions
// // =============================================================================

// async fn copy_dir_recursive(source: &std::path::Path, dest: &std::path::Path) -> anyhow::Result<usize> {
//     use tokio::fs;

//     let mut count = 0;
//     fs::create_dir_all(dest).await?;

//     let mut entries = fs::read_dir(source).await?;
//     while let Some(entry) = entries.next_entry().await? {
//         let src_path = entry.path();
//         let dest_path = dest.join(entry.file_name());

//         if src_path.is_dir() {
//             count += Box::pin(copy_dir_recursive(&src_path, &dest_path)).await?;
//         } else {
//             fs::copy(&src_path, &dest_path).await?;
//             count += 1;
//         }
//     }

//     Ok(count)
// }

// async fn download_torrent_file_authenticated(
//     url: &str,
//     indexer_type: &str,
//     credentials: &std::collections::HashMap<String, String>,
// ) -> anyhow::Result<Vec<u8>> {
//     let client = reqwest::Client::builder()
//         .timeout(std::time::Duration::from_secs(30))
//         .build()?;

//     let mut request = client.get(url);

//     match indexer_type {
//         "iptorrents" => {
//             if let Some(cookie) = credentials.get("cookie") {
//                 request = request.header("Cookie", cookie);
//             }
//             if let Some(user_agent) = credentials.get("user_agent") {
//                 request = request.header("User-Agent", user_agent);
//             }
//         }
//         _ => {}
//     }

//     let response = request.send().await?;
//     if !response.status().is_success() {
//         anyhow::bail!("HTTP {}", response.status());
//     }

//     Ok(response.bytes().await?.to_vec())
// }

// async fn create_file_matches_for_target(
//     db: &Database,
//     torrent_info: &crate::services::torrent::TorrentInfo,
//     album_id: Option<Uuid>,
//     movie_id: Option<Uuid>,
//     episode_id: Option<Uuid>,
// ) {
//     use crate::services::file_utils::{is_audio_file, is_video_file};
//     use crate::services::filename_parser;

//     // Get torrent record
//     let torrent_record = EntityQuery::<TorrentEntity>::new()
//         .filter(&TorrentEntityWhereInput {
//             info_hash: Some(StringFilter::eq(&torrent_info.info_hash)),
//             ..Default::default()
//         })
//         .fetch_one(db.pool())
//         .await
//         .ok()
//         .flatten();

//     let torrent_record = match torrent_record {
//         Some(r) => r,
//         None => return,
//     };

//     let user_id = &torrent_record.user_id;

//     if let Some(album_id) = album_id {
//         // Get tracks for the album
//         let tracks = EntityQuery::<TrackEntity>::new()
//             .filter(&TrackEntityWhereInput {
//                 album_id: Some(StringFilter::eq(&album_id.to_string())),
//                 ..Default::default()
//             })
//             .fetch_all(db.pool())
//             .await
//             .unwrap_or_default();

//         let audio_files: Vec<_> = torrent_info.files.iter().enumerate().filter(|(_, f)| is_audio_file(&f.path)).collect();

//         for (idx, file) in &audio_files {
//             let file_name = std::path::Path::new(&file.path).file_name().and_then(|n| n.to_str()).unwrap_or(&file.path);
//             let mut best_match: Option<(String, f64)> = None;

//             for track in &tracks {
//                 if track.media_file_id.is_some() { continue; }
//                 let similarity = filename_parser::show_name_similarity(file_name, &track.title);
//                 if similarity >= 0.5 && (best_match.is_none() || similarity > best_match.as_ref().unwrap().1) {
//                     best_match = Some((track.id.clone(), similarity));
//                 }
//             }

//             if let Some((track_id, _)) = best_match {
//                 let quality = filename_parser::parse_quality(&file.path);
//                 create_pending_file_match(
//                     db.pool(),
//                     user_id,
//                     &file.path,
//                     "torrent",
//                     &torrent_record.id,
//                     Some(*idx as i32),
//                     file.size as i64,
//                     None,
//                     None,
//                     Some(&track_id),
//                     None,
//                     quality.resolution.as_deref(),
//                     quality.codec.as_deref(),
//                     quality.source.as_deref(),
//                     quality.audio.as_deref(),
//                 ).await.ok();
//             }
//         }
//     }

//     if let Some(movie_id) = movie_id {
//         let video_files: Vec<_> = torrent_info.files.iter().enumerate().filter(|(_, f)| is_video_file(&f.path)).collect();
//         if let Some((idx, file)) = video_files.iter().max_by_key(|(_, f)| f.size) {
//             let quality = filename_parser::parse_quality(&file.path);
//             create_pending_file_match(
//                 db.pool(),
//                 user_id,
//                 &file.path,
//                 "torrent",
//                 &torrent_record.id,
//                 Some(*idx as i32),
//                 file.size as i64,
//                 None,
//                 Some(&movie_id.to_string()),
//                 None,
//                 None,
//                 quality.resolution.as_deref(),
//                 quality.codec.as_deref(),
//                 quality.source.as_deref(),
//                 quality.audio.as_deref(),
//             ).await.ok();
//         }
//     }

//     if let Some(episode_id) = episode_id {
//         let video_files: Vec<_> = torrent_info.files.iter().enumerate().filter(|(_, f)| is_video_file(&f.path)).collect();
//         if let Some((idx, file)) = video_files.iter().max_by_key(|(_, f)| f.size) {
//             let quality = filename_parser::parse_quality(&file.path);
//             create_pending_file_match(
//                 db.pool(),
//                 user_id,
//                 &file.path,
//                 "torrent",
//                 &torrent_record.id,
//                 Some(*idx as i32),
//                 file.size as i64,
//                 Some(&episode_id.to_string()),
//                 None,
//                 None,
//                 None,
//                 quality.resolution.as_deref(),
//                 quality.codec.as_deref(),
//                 quality.source.as_deref(),
//                 quality.audio.as_deref(),
//             ).await.ok();
//         }
//     }
// }

// #[allow(clippy::too_many_arguments)]
// async fn create_pending_file_match(
//     pool: &sqlx::SqlitePool,
//     user_id: &str,
//     source_path: &str,
//     source_type: &str,
//     source_id: &str,
//     source_file_index: Option<i32>,
//     file_size: i64,
//     episode_id: Option<&str>,
//     movie_id: Option<&str>,
//     track_id: Option<&str>,
//     chapter_id: Option<&str>,
//     parsed_resolution: Option<&str>,
//     parsed_codec: Option<&str>,
//     parsed_source: Option<&str>,
//     parsed_audio: Option<&str>,
// ) -> Result<(), sqlx::Error> {
//     let id = Uuid::new_v4().to_string();

//     sqlx::query(
//         r#"
//         INSERT INTO pending_file_matches (
//             id, user_id, source_path, source_type, source_id, source_file_index, file_size,
//             episode_id, movie_id, track_id, chapter_id, match_type, match_confidence, match_attempts,
//             parsed_resolution, parsed_codec, parsed_source, parsed_audio, copy_attempts,
//             created_at, updated_at
//         ) VALUES (
//             ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'manual', 1.0, 1,
//             ?12, ?13, ?14, ?15, 0, datetime('now'), datetime('now')
//         )
//         "#,
//     )
//     .bind(&id)
//     .bind(user_id)
//     .bind(source_path)
//     .bind(source_type)
//     .bind(source_id)
//     .bind(source_file_index)
//     .bind(file_size)
//     .bind(episode_id)
//     .bind(movie_id)
//     .bind(track_id)
//     .bind(chapter_id)
//     .bind(parsed_resolution)
//     .bind(parsed_codec)
//     .bind(parsed_source)
//     .bind(parsed_audio)
//     .execute(pool)
//     .await?;

//     Ok(())
// }
