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
    table = "media_files",
    plural = "MediaFiles",
    default_sort = "path",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct MediaFile {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "libraryId")]
    #[filterable(type = "string")]
    pub library_id: Option<String>,

    #[graphql(name = "episodeId")]
    #[filterable(type = "string")]
    pub episode_id: Option<String>,

    #[graphql(name = "movieId")]
    #[filterable(type = "string")]
    pub movie_id: Option<String>,

    #[graphql(name = "trackId")]
    #[filterable(type = "string")]
    pub track_id: Option<String>,

    #[graphql(name = "chapterId")]
    #[filterable(type = "string")]
    pub chapter_id: Option<String>,

    #[graphql(name = "path")]
    #[filterable(type = "string")]
    #[sortable]
    pub path: String,

    #[graphql(name = "relativePath")]
    pub relative_path: Option<String>,

    #[graphql(name = "originalName")]
    pub original_name: Option<String>,

    #[graphql(name = "size")]
    #[filterable(type = "number")]
    #[sortable]
    pub size: i64,

    #[graphql(name = "container")]
    #[filterable(type = "string")]
    pub container: Option<String>,

    #[graphql(name = "videoCodec")]
    #[filterable(type = "string")]
    pub video_codec: Option<String>,

    #[graphql(name = "audioCodec")]
    #[filterable(type = "string")]
    pub audio_codec: Option<String>,

    #[graphql(name = "width")]
    #[filterable(type = "number")]
    pub width: Option<i32>,

    #[graphql(name = "height")]
    #[filterable(type = "number")]
    pub height: Option<i32>,

    #[graphql(name = "duration")]
    #[filterable(type = "number")]
    #[sortable]
    pub duration: Option<i32>,

    #[graphql(name = "bitrate")]
    #[filterable(type = "number")]
    pub bitrate: Option<i32>,

    #[graphql(name = "resolution")]
    #[filterable(type = "string")]
    #[sortable]
    pub resolution: Option<String>,

    #[graphql(name = "isHdr")]
    #[filterable(type = "boolean")]
    pub is_hdr: bool,

    #[graphql(name = "hdrType")]
    #[filterable(type = "string")]
    pub hdr_type: Option<String>,

    #[graphql(name = "audioChannels")]
    #[filterable(type = "string")]
    pub audio_channels: Option<String>,

    #[graphql(name = "metadata")]
    pub metadata: Option<String>,

    #[graphql(name = "contentType")]
    #[filterable(type = "string")]
    pub content_type: Option<String>,

    #[graphql(name = "addedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub added_at: String,

    #[graphql(name = "analyzedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub analyzed_at: Option<String>,

    /// Filesystem modification timestamp observed during the last scan. This
    /// is paired with `size` to invalidate stale analysis after in-place file
    /// replacement without reprocessing unchanged files.
    #[graphql(name = "fileModifiedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub file_modified_at: Option<String>,

    // Manual-match protection (design.md Q9b): a file matched via explicit user
    // input is "manual" and must never be silently overwritten by automatic
    // matching/rescan, even with force:true. Automatic candidate-ranking matches
    // are "auto". All three fields are cleared together on unmatch.
    #[graphql(name = "matchType")]
    #[filterable(type = "string")]
    pub match_type: Option<String>,

    #[graphql(name = "matchedByUserId")]
    #[filterable(type = "string")]
    pub matched_by_user_id: Option<String>,

    #[graphql(name = "matchConfirmedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub match_confirmed_at: Option<String>,

    /// `optimal` / `suboptimal` / `null` (unknown). Authoritative quality
    /// verdict computed by `services::quality::profile::evaluate` against the
    /// resolved `QualityProfile`, populated after ffprobe analysis and
    /// recomputed when the assigned profile changes (design.md Q23/Q24).
    #[graphql(name = "qualityStatus")]
    #[filterable(type = "string")]
    pub quality_status: Option<String>,
}

#[derive(Default)]
pub struct MediaFileCustomOperations;
