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
    table = "media_files",
    plural = "MediaFiles",
    default_sort = "path",
    // A library's files in path order, without sorting every row of a large library to find a page.
    index = "library_id,path",
    // One record per file in a library. Re-imports and scans used to add more for the same path.
    unique_index = "path,library_id",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct MediaFile {
    /// General data revision, including stream/marker relation changes. Managed
    /// by generated writes; distinct from analysis or metadata refresh times.
    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    #[serde(default)]
    #[graphql_orm(default = false)]
    pub updated_at: Option<String>,

    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "libraryId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub library_id: Option<String>,

    #[graphql(name = "episodeId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub episode_id: Option<String>,

    #[graphql(name = "movieId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub movie_id: Option<String>,

    #[graphql(name = "trackId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub track_id: Option<String>,

    #[graphql(name = "chapterId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
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

    /// The video carries closed captions (EIA-608/708) inside its picture stream, as ffprobe
    /// reports them. Unknown until the file is analysed. See `docs/design.md` Q119.
    #[graphql(name = "closedCaptions")]
    pub closed_captions: Option<bool>,

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

    /// The picture inside black bars baked into the video (letterbox or pillarbox), found with
    /// ffmpeg `cropdetect` after analysis (design.md Q123). `null` when the picture fills the
    /// frame, when detection was inconclusive, or before it has run.
    #[graphql(name = "pictureCrop")]
    #[json_field]
    pub picture_crop: Option<PictureCrop>,

    /// Seek-bar preview sprite sheets, made after analysis (design.md Q122). Sheets are served
    /// at `/api/media/<id>/storyboard/<key>/<sheet>.jpg`.
    #[graphql(name = "storyboard")]
    #[json_field]
    pub storyboard: Option<VideoStoryboard>,

    /// When the preview job (crop detection and storyboard) last finished for this version of
    /// the file, whatever it found. Cleared when the file changes, so the job runs again.
    #[graphql(name = "previewsCheckedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub previews_checked_at: Option<String>,

    // Query relations do not add physical FKs or change existing delete behavior.
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Library",
        from = "library_id",
        to = "id",
        on_delete = "cascade",
        propagate_change = "up",
        emit_fk = false
    )]
    pub library: Option<Box<Library>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Episode",
        from = "episode_id",
        to = "id",
        on_delete = "set_null",
        propagate_change = "up",
        emit_fk = false
    )]
    pub episode: Option<Box<Episode>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Movie",
        from = "movie_id",
        to = "id",
        on_delete = "set_null",
        propagate_change = "up",
        emit_fk = false
    )]
    pub movie: Option<Box<Movie>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Track",
        from = "track_id",
        to = "id",
        on_delete = "set_null",
        propagate_change = "up",
        emit_fk = false
    )]
    pub track: Option<Box<Track>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Chapter",
        from = "chapter_id",
        to = "id",
        on_delete = "set_null",
        propagate_change = "up",
        emit_fk = false
    )]
    pub chapter: Option<Box<Chapter>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "VideoStream",
        from = "id",
        to = "media_file_id",
        multiple,
        emit_fk = false
    )]
    pub video_streams: Vec<VideoStream>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "AudioStream",
        from = "id",
        to = "media_file_id",
        multiple,
        emit_fk = false
    )]
    pub audio_streams: Vec<AudioStream>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Subtitle",
        from = "id",
        to = "media_file_id",
        multiple,
        emit_fk = false
    )]
    pub subtitles: Vec<Subtitle>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "MediaChapter",
        from = "id",
        to = "media_file_id",
        multiple,
        emit_fk = false
    )]
    pub media_chapters: Vec<MediaChapter>,
}

/// The picture's rectangle inside a frame with black bars, in the source's pixels.
#[derive(async_graphql::SimpleObject, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct PictureCrop {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    /// The frame the rectangle was measured in (the stream's coded size).
    pub frame_width: i32,
    pub frame_height: i32,
}

/// Sprite sheets of evenly spaced frames for the seek bar's preview.
#[derive(async_graphql::SimpleObject, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct VideoStoryboard {
    /// Names this set of sheets in their URLs; changes whenever they are made again.
    pub key: String,
    /// Seconds of video per tile. Tile `n` shows the picture near `(n + 0.5) * interval`.
    pub interval: f64,
    pub tile_width: i32,
    pub tile_height: i32,
    /// Tiles per row and rows per sheet.
    pub columns: i32,
    pub rows: i32,
    /// Tiles in all, across every sheet.
    pub frame_count: i32,
    pub sheet_count: i32,
}

#[derive(Default)]
pub struct MediaFileCustomOperations;
