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
    table = "subtitles",
    plural = "Subtitles",
    default_sort = "created_at",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct Subtitle {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: String,

    #[graphql(name = "sourceType")]
    #[filterable(type = "string")]
    pub source_type: String,

    #[graphql(name = "streamIndex")]
    #[filterable(type = "number")]
    pub stream_index: Option<i32>,

    #[graphql(name = "filePath")]
    pub file_path: Option<String>,

    #[graphql(name = "codec")]
    #[filterable(type = "string")]
    pub codec: Option<String>,

    #[graphql(name = "codecLongName")]
    pub codec_long_name: Option<String>,

    #[graphql(name = "language")]
    #[filterable(type = "string")]
    pub language: Option<String>,

    #[graphql(name = "title")]
    pub title: Option<String>,

    #[graphql(name = "isDefault")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_default: bool,

    #[graphql(name = "isForced")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_forced: bool,

    #[graphql(name = "isHearingImpaired")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_hearing_impaired: bool,

    #[graphql(name = "opensubtitlesId")]
    #[filterable(type = "string")]
    pub opensubtitles_id: Option<String>,

    #[graphql(name = "downloadedAt")]
    #[filterable(type = "date")]
    pub downloaded_at: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<String>,

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
pub struct SubtitleCustomOperations;
