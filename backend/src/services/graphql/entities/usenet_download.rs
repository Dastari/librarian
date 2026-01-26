use async_graphql::SimpleObject;
use librarian_macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

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
#[graphql(name = "UsenetDownload")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "usenet_downloads",
    plural = "UsenetDownloads",
    default_sort = "created_at"
)]
pub struct UsenetDownload {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "NzbName")]
    #[filterable(type = "string")]
    #[sortable]
    pub nzb_name: String,

    #[graphql(name = "NzbHash")]
    #[filterable(type = "string")]
    pub nzb_hash: Option<String>,

    #[graphql(name = "NzbUrl")]
    pub nzb_url: Option<String>,

    #[graphql(name = "NzbData")]
    pub nzb_data: Option<String>,

    #[graphql(name = "State")]
    #[filterable(type = "string")]
    #[sortable]
    pub state: String,

    #[graphql(name = "Progress")]
    pub progress: Option<String>,

    #[graphql(name = "SizeBytes")]
    #[filterable(type = "number")]
    #[sortable]
    pub size_bytes: Option<i64>,

    #[graphql(name = "DownloadedBytes")]
    #[filterable(type = "number")]
    pub downloaded_bytes: Option<i64>,

    #[graphql(name = "DownloadSpeed")]
    #[filterable(type = "number")]
    pub download_speed: Option<i32>,

    #[graphql(name = "EtaSeconds")]
    #[filterable(type = "number")]
    pub eta_seconds: Option<i32>,

    #[graphql(name = "ErrorMessage")]
    pub error_message: Option<String>,

    #[graphql(name = "RetryCount")]
    #[filterable(type = "number")]
    pub retry_count: i32,

    #[graphql(name = "DownloadPath")]
    pub download_path: Option<String>,

    #[graphql(name = "LibraryId")]
    #[filterable(type = "string")]
    pub library_id: Option<String>,

    #[graphql(name = "EpisodeId")]
    #[filterable(type = "string")]
    pub episode_id: Option<String>,

    #[graphql(name = "MovieId")]
    #[filterable(type = "string")]
    pub movie_id: Option<String>,

    #[graphql(name = "AlbumId")]
    #[filterable(type = "string")]
    pub album_id: Option<String>,

    #[graphql(name = "AudiobookId")]
    #[filterable(type = "string")]
    pub audiobook_id: Option<String>,

    #[graphql(name = "IndexerId")]
    #[filterable(type = "string")]
    pub indexer_id: Option<String>,

    #[graphql(name = "PostProcessStatus")]
    #[filterable(type = "string")]
    pub post_process_status: Option<String>,

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
}

#[derive(Default)]
pub struct UsenetDownloadCustomOperations;
