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
#[graphql(name = "TorrentFile")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "torrent_files",
    plural = "TorrentFiles",
    default_sort = "file_index"
)]
pub struct TorrentFile {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "TorrentId")]
    #[filterable(type = "string")]
    pub torrent_id: String,

    #[graphql(name = "FileIndex")]
    #[filterable(type = "number")]
    #[sortable]
    pub file_index: i32,

    #[graphql(name = "FilePath")]
    #[filterable(type = "string")]
    pub file_path: String,

    #[graphql(name = "RelativePath")]
    #[filterable(type = "string")]
    pub relative_path: String,

    #[graphql(name = "FileSize")]
    #[filterable(type = "number")]
    #[sortable]
    pub file_size: i64,

    #[graphql(name = "DownloadedBytes")]
    #[filterable(type = "number")]
    pub downloaded_bytes: i64,

    #[graphql(name = "Progress")]
    #[filterable(type = "number")]
    #[sortable]
    pub progress: f64,

    #[graphql(name = "MediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: Option<String>,

    #[graphql(name = "IsExcluded")]
    #[filterable(type = "boolean")]
    pub is_excluded: bool,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

#[derive(Default)]
pub struct TorrentFileCustomOperations;
