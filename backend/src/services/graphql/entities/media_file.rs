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
#[graphql(name = "MediaFile")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "media_files", plural = "MediaFiles", default_sort = "path")]
pub struct MediaFile {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "LibraryId")]
    #[filterable(type = "string")]
    pub library_id: String,

    #[graphql(name = "EpisodeId")]
    #[filterable(type = "string")]
    pub episode_id: Option<String>,

    #[graphql(name = "MovieId")]
    #[filterable(type = "string")]
    pub movie_id: Option<String>,

    #[graphql(name = "TrackId")]
    #[filterable(type = "string")]
    pub track_id: Option<String>,

    #[graphql(name = "Path")]
    #[filterable(type = "string")]
    #[sortable]
    pub path: String,

    #[graphql(name = "RelativePath")]
    pub relative_path: Option<String>,

    #[graphql(name = "OriginalName")]
    pub original_name: Option<String>,

    #[graphql(name = "Size")]
    #[filterable(type = "number")]
    #[sortable]
    pub size: i64,

    #[graphql(name = "Container")]
    #[filterable(type = "string")]
    pub container: Option<String>,

    #[graphql(name = "VideoCodec")]
    #[filterable(type = "string")]
    pub video_codec: Option<String>,

    #[graphql(name = "AudioCodec")]
    #[filterable(type = "string")]
    pub audio_codec: Option<String>,

    #[graphql(name = "Width")]
    #[filterable(type = "number")]
    pub width: Option<i32>,

    #[graphql(name = "Height")]
    #[filterable(type = "number")]
    pub height: Option<i32>,

    #[graphql(name = "Duration")]
    #[filterable(type = "number")]
    #[sortable]
    pub duration: Option<i32>,

    #[graphql(name = "Bitrate")]
    #[filterable(type = "number")]
    pub bitrate: Option<i32>,

    #[graphql(name = "Resolution")]
    #[filterable(type = "string")]
    #[sortable]
    pub resolution: Option<String>,

    #[graphql(name = "IsHdr")]
    #[filterable(type = "boolean")]
    pub is_hdr: bool,

    #[graphql(name = "HdrType")]
    #[filterable(type = "string")]
    pub hdr_type: Option<String>,

    #[graphql(name = "AudioChannels")]
    #[filterable(type = "string")]
    pub audio_channels: Option<String>,

    #[graphql(name = "ContentType")]
    #[filterable(type = "string")]
    pub content_type: Option<String>,

    #[graphql(name = "AddedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub added_at: String,
}

#[derive(Default)]
pub struct MediaFileCustomOperations;
