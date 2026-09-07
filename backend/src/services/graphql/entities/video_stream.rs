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
    table = "video_streams",
    plural = "VideoStreams",
    default_sort = "stream_index",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct VideoStream {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
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

    #[graphql(name = "width")]
    #[filterable(type = "number")]
    pub width: i32,

    #[graphql(name = "height")]
    #[filterable(type = "number")]
    pub height: i32,

    #[graphql(name = "aspectRatio")]
    pub aspect_ratio: Option<String>,

    #[graphql(name = "frameRate")]
    pub frame_rate: Option<String>,

    #[graphql(name = "avgFrameRate")]
    pub avg_frame_rate: Option<String>,

    #[graphql(name = "bitrate")]
    #[filterable(type = "number")]
    pub bitrate: Option<i32>,

    #[graphql(name = "pixelFormat")]
    pub pixel_format: Option<String>,

    #[graphql(name = "colorSpace")]
    pub color_space: Option<String>,

    #[graphql(name = "colorTransfer")]
    pub color_transfer: Option<String>,

    #[graphql(name = "colorPrimaries")]
    pub color_primaries: Option<String>,

    #[graphql(name = "hdrType")]
    #[filterable(type = "string")]
    pub hdr_type: Option<String>,

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

    #[graphql(name = "metadata")]
    pub metadata: Option<String>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,
}

#[derive(Default)]
pub struct VideoStreamCustomOperations;
