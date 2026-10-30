use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity,
    GraphQLRelations,
    GraphQLOperations,
    async_graphql::SimpleObject,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[graphql(complex)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "audio_streams",
    plural = "AudioStreams",
    default_sort = "stream_index",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct AudioStream {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub media_file_id: String,

    #[graphql(name = "streamIndex")]
    #[filterable(type = "number")]
    #[sortable]
    pub stream_index: i32,

    #[graphql(name = "codec")]
    #[filterable(type = "string")]
    pub codec: String,

    #[graphql(name = "codecLongName")]
    pub codec_long_name: Option<String>,

    #[graphql(name = "channels")]
    #[filterable(type = "number")]
    pub channels: i32,

    #[graphql(name = "channelLayout")]
    pub channel_layout: Option<String>,

    #[graphql(name = "sampleRate")]
    #[filterable(type = "number")]
    pub sample_rate: Option<i32>,

    #[graphql(name = "bitrate")]
    #[filterable(type = "number")]
    pub bitrate: Option<i32>,

    #[graphql(name = "bitDepth")]
    #[filterable(type = "number")]
    pub bit_depth: Option<i32>,

    #[graphql(name = "language")]
    #[filterable(type = "string")]
    pub language: Option<String>,

    #[graphql(name = "title")]
    pub title: Option<String>,

    #[graphql(name = "isDefault")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_default: bool,

    #[graphql(name = "isCommentary")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_commentary: bool,

    #[graphql(name = "metadata")]
    pub metadata: Option<String>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    // Query relations do not add physical FKs or change existing delete behavior.
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "MediaFile",
        from = "media_file_id",
        to = "id",
        on_delete = "cascade",
        propagate_change = "up",
        emit_fk = false
    )]
    pub media_file: Option<Box<MediaFile>>,
}

#[derive(Default)]
pub struct AudioStreamCustomOperations;
