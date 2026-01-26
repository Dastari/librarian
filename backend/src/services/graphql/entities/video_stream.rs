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
#[graphql(name = "VideoStream")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "video_streams",
    plural = "VideoStreams",
    default_sort = "stream_index"
)]
pub struct VideoStream {
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

    #[graphql(name = "Width")]
    #[filterable(type = "number")]
    pub width: i32,

    #[graphql(name = "Height")]
    #[filterable(type = "number")]
    pub height: i32,

    #[graphql(name = "AspectRatio")]
    pub aspect_ratio: Option<String>,

    #[graphql(name = "FrameRate")]
    pub frame_rate: Option<String>,

    #[graphql(name = "AvgFrameRate")]
    pub avg_frame_rate: Option<String>,

    #[graphql(name = "Bitrate")]
    #[filterable(type = "number")]
    pub bitrate: Option<i32>,

    #[graphql(name = "PixelFormat")]
    pub pixel_format: Option<String>,

    #[graphql(name = "ColorSpace")]
    pub color_space: Option<String>,

    #[graphql(name = "ColorTransfer")]
    pub color_transfer: Option<String>,

    #[graphql(name = "ColorPrimaries")]
    pub color_primaries: Option<String>,

    #[graphql(name = "HdrType")]
    #[filterable(type = "string")]
    pub hdr_type: Option<String>,

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

    #[graphql(name = "Metadata")]
    pub metadata: Option<String>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,
}

#[derive(Default)]
pub struct VideoStreamCustomOperations;
