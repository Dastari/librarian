use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "usenet_downloads",
    plural = "UsenetDownloads",
    default_sort = "created_at"
)]
pub struct UsenetDownload {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,

    #[graphql(name = "nzbName")]
    #[filterable(type = "string")]
    #[sortable]
    pub nzb_name: String,

    #[graphql(name = "nzbHash")]
    #[filterable(type = "string")]
    pub nzb_hash: Option<String>,

    #[graphql(name = "nzbUrl")]
    pub nzb_url: Option<String>,

    #[graphql(name = "nzbData")]
    pub nzb_data: Option<String>,

    #[graphql(name = "state")]
    #[filterable(type = "string")]
    #[sortable]
    pub state: String,

    #[graphql(name = "progress")]
    pub progress: Option<String>,

    #[graphql(name = "sizeBytes")]
    #[filterable(type = "number")]
    #[sortable]
    pub size_bytes: Option<i64>,

    #[graphql(name = "downloadedBytes")]
    #[filterable(type = "number")]
    pub downloaded_bytes: Option<i64>,

    #[graphql(name = "downloadSpeed")]
    #[filterable(type = "number")]
    pub download_speed: Option<i32>,

    #[graphql(name = "etaSeconds")]
    #[filterable(type = "number")]
    pub eta_seconds: Option<i32>,

    #[graphql(name = "errorMessage")]
    pub error_message: Option<String>,

    #[graphql(name = "retryCount")]
    #[filterable(type = "number")]
    pub retry_count: i32,

    #[graphql(name = "downloadPath")]
    pub download_path: Option<String>,

    #[graphql(name = "libraryId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub library_id: Option<String>,

    #[graphql(name = "episodeId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub episode_id: Option<String>,

    #[graphql(name = "movieId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub movie_id: Option<String>,

    #[graphql(name = "albumId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub album_id: Option<String>,

    #[graphql(name = "audiobookId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub audiobook_id: Option<String>,

    #[graphql(name = "indexerId")]
    #[filterable(type = "string")]
    pub indexer_id: Option<String>,

    #[graphql(name = "postProcessStatus")]
    #[filterable(type = "string")]
    pub post_process_status: Option<String>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,

    #[graphql(name = "completedAt")]
    #[filterable(type = "date")]
    pub completed_at: Option<String>,
}

#[derive(Default)]
pub struct UsenetDownloadCustomOperations;
