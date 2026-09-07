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
    table = "torrent_files",
    plural = "TorrentFiles",
    default_sort = "file_index",
    index = "torrent_id",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct TorrentFile {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "torrentId")]
    #[filterable(type = "string")]
    pub torrent_id: String,

    #[graphql(name = "fileIndex")]
    #[filterable(type = "number")]
    #[sortable]
    pub file_index: i32,

    #[graphql(name = "filePath")]
    #[filterable(type = "string")]
    pub file_path: String,

    #[graphql(name = "relativePath")]
    #[filterable(type = "string")]
    pub relative_path: String,

    #[graphql(name = "fileSize")]
    #[filterable(type = "number")]
    #[sortable]
    pub file_size: i64,

    #[graphql(name = "downloadedBytes")]
    #[filterable(type = "number")]
    pub downloaded_bytes: i64,

    #[graphql(name = "progress")]
    #[filterable(type = "number")]
    #[sortable]
    pub progress: f64,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: Option<String>,

    #[graphql(name = "isExcluded")]
    #[filterable(type = "boolean")]
    pub is_excluded: bool,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

#[derive(Default)]
pub struct TorrentFileCustomOperations;
