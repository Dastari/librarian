use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Result, SimpleObject};
use macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::super::auth::AuthExt;
use super::torrent_file::TorrentFile;
use crate::services::ServicesManager;
use crate::services::torrent::{
    TorrentInfo as ServiceTorrentInfo, TorrentService, TorrentState as ServiceTorrentState,
};

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

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,

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

/// Result of processing matched files from a source
#[derive(Debug, SimpleObject)]
#[graphql(name = "ProcessSourceResult")]
pub struct ProcessSourceResult {
    #[graphql(name = "Success")]
    pub success: bool,
    #[graphql(name = "FilesProcessed")]
    pub files_processed: i32,
    #[graphql(name = "FilesFailed")]
    pub files_failed: i32,
    #[graphql(name = "Messages")]
    pub messages: Vec<String>,
    #[graphql(name = "Error")]
    pub error: Option<String>,
}

/// Result of re-matching files for a source
#[derive(Debug, SimpleObject)]
#[graphql(name = "RematchSourceResult")]
pub struct RematchSourceResult {
    #[graphql(name = "Success")]
    pub success: bool,
    #[graphql(name = "MatchCount")]
    pub match_count: i32,
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
        let list = service
            .list_torrents()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(list.into_iter().map(service_torrent_to_live).collect())
    }

    /// Get a single live torrent by numeric id
    #[graphql(name = "LiveTorrent")]
    async fn live_torrent(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Id")] id: i32,
    ) -> Result<Option<LiveTorrent>> {
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
        let list: Vec<ServiceTorrentInfo> = service
            .list_active_downloads()
            .await
            .map_err(|e: anyhow::Error| async_graphql::Error::new(e.to_string()))?;
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
    async fn add_torrent(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: AddTorrentInput,
    ) -> Result<AddTorrentResult> {
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
    async fn pause_torrent(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Id")] id: i32,
    ) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        match service.pause(id as usize).await {
            Ok(()) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Resume a paused torrent
    #[graphql(name = "ResumeTorrent")]
    async fn resume_torrent(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Id")] id: i32,
    ) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        match service.resume(id as usize).await {
            Ok(()) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
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
            Ok(()) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    #[graphql(name = "PauseTorrentByInfoHash")]
    async fn pause_torrent_by_info_hash(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "InfoHash")] info_hash: String,
    ) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        match service.pause_by_info_hash(&info_hash).await {
            Ok(()) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    #[graphql(name = "ResumeTorrentByInfoHash")]
    async fn resume_torrent_by_info_hash(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "InfoHash")] info_hash: String,
    ) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        match service.resume_by_info_hash(&info_hash).await {
            Ok(()) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    #[graphql(name = "RemoveTorrentByInfoHash")]
    async fn remove_torrent_by_info_hash(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "InfoHash")] info_hash: String,
        #[graphql(name = "DeleteFiles", default = false)] delete_files: bool,
    ) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;
        match service.remove_by_info_hash(&info_hash, delete_files).await {
            Ok(()) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Process pending file matches for a source.
    /// Note: full processing pipeline from legacy code is being re-implemented.
    #[graphql(name = "ProcessSource")]
    async fn process_source(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "SourceType")] source_type: String,
        #[graphql(name = "SourceId")] source_id: String,
    ) -> Result<ProcessSourceResult> {
        let _user = ctx.auth_user()?;

        let normalized_source = source_type.trim().to_ascii_lowercase();
        if normalized_source != "torrent" {
            return Ok(ProcessSourceResult {
                success: false,
                files_processed: 0,
                files_failed: 0,
                messages: vec![],
                error: Some("Only source type 'torrent' is currently supported".to_string()),
            });
        }

        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;

        let exists = service
            .list_torrents()
            .await
            .map(|list| {
                list.into_iter()
                    .any(|t| t.info_hash.eq_ignore_ascii_case(&source_id))
            })
            .unwrap_or(false);

        if !exists {
            return Ok(ProcessSourceResult {
                success: false,
                files_processed: 0,
                files_failed: 0,
                messages: vec![],
                error: Some("Torrent not found for SourceId".to_string()),
            });
        }

        Ok(ProcessSourceResult {
            success: false,
            files_processed: 0,
            files_failed: 0,
            messages: vec![
                "ProcessSource has not been fully re-implemented after refactor".to_string(),
            ],
            error: Some("Not implemented".to_string()),
        })
    }

    /// Re-run matching for files from a source.
    /// Note: full matching pipeline from legacy code is being re-implemented.
    #[graphql(name = "RematchSource")]
    async fn rematch_source(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "SourceType")] source_type: String,
        #[graphql(name = "SourceId")] source_id: String,
        #[graphql(name = "LibraryId")] _library_id: Option<String>,
    ) -> Result<RematchSourceResult> {
        let _user = ctx.auth_user()?;

        let normalized_source = source_type.trim().to_ascii_lowercase();
        if normalized_source != "torrent" {
            return Ok(RematchSourceResult {
                success: false,
                match_count: 0,
                error: Some("Only source type 'torrent' is currently supported".to_string()),
            });
        }

        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let service = manager
            .get_torrent()
            .await
            .ok_or_else(|| async_graphql::Error::new("Torrent service not available"))?;

        let exists = service
            .list_torrents()
            .await
            .map(|list| {
                list.into_iter()
                    .any(|t| t.info_hash.eq_ignore_ascii_case(&source_id))
            })
            .unwrap_or(false);

        if !exists {
            return Ok(RematchSourceResult {
                success: false,
                match_count: 0,
                error: Some("Torrent not found for SourceId".to_string()),
            });
        }

        Ok(RematchSourceResult {
            success: false,
            match_count: 0,
            error: Some("Not implemented".to_string()),
        })
    }
}
