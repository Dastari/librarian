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
#[graphql(name = "Subtitle")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "subtitles", plural = "Subtitles", default_sort = "created_at")]
pub struct Subtitle {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "MediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: String,

    #[graphql(name = "SourceType")]
    #[filterable(type = "string")]
    pub source_type: String,

    #[graphql(name = "StreamIndex")]
    #[filterable(type = "number")]
    pub stream_index: Option<i32>,

    #[graphql(name = "FilePath")]
    pub file_path: Option<String>,

    #[graphql(name = "Codec")]
    #[filterable(type = "string")]
    pub codec: Option<String>,

    #[graphql(name = "CodecLongName")]
    pub codec_long_name: Option<String>,

    #[graphql(name = "Language")]
    #[filterable(type = "string")]
    pub language: Option<String>,

    #[graphql(name = "Title")]
    pub title: Option<String>,

    #[graphql(name = "IsDefault")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_default: bool,

    #[graphql(name = "IsForced")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_forced: bool,

    #[graphql(name = "IsHearingImpaired")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_hearing_impaired: bool,

    #[graphql(name = "OpensubtitlesId")]
    #[filterable(type = "string")]
    pub opensubtitles_id: Option<String>,

    #[graphql(name = "DownloadedAt")]
    #[filterable(type = "date")]
    pub downloaded_at: Option<String>,

    #[graphql(name = "Metadata")]
    pub metadata: Option<String>,

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
pub struct SubtitleCustomOperations;
