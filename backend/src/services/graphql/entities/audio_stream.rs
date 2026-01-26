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
#[graphql(name = "AudioStream")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "audio_streams",
    plural = "AudioStreams",
    default_sort = "stream_index"
)]
pub struct AudioStream {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "MediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: String,

    #[graphql(name = "StreamIndex")]
    #[filterable(type = "number")]
    #[sortable]
    pub stream_index: i32,

    #[graphql(name = "Codec")]
    #[filterable(type = "string")]
    pub codec: String,

    #[graphql(name = "CodecLongName")]
    pub codec_long_name: Option<String>,

    #[graphql(name = "Channels")]
    #[filterable(type = "number")]
    pub channels: i32,

    #[graphql(name = "ChannelLayout")]
    pub channel_layout: Option<String>,

    #[graphql(name = "SampleRate")]
    #[filterable(type = "number")]
    pub sample_rate: Option<i32>,

    #[graphql(name = "Bitrate")]
    #[filterable(type = "number")]
    pub bitrate: Option<i32>,

    #[graphql(name = "BitDepth")]
    #[filterable(type = "number")]
    pub bit_depth: Option<i32>,

    #[graphql(name = "Language")]
    #[filterable(type = "string")]
    pub language: Option<String>,

    #[graphql(name = "Title")]
    pub title: Option<String>,

    #[graphql(name = "IsDefault")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_default: bool,

    #[graphql(name = "IsCommentary")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_commentary: bool,

    #[graphql(name = "Metadata")]
    pub metadata: Option<String>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,
}

#[derive(Default)]
pub struct AudioStreamCustomOperations;
