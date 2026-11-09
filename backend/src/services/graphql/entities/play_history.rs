//! Play history and who may see its statistics (design.md Q134–Q136).
//!
//! `PlayHistory` is one play of one item: written only by the playback-session hook (and the
//! progress import), never through the generated create or update. Its owner and
//! administrators read and delete it. Content ids are plain historical references with title
//! snapshots, not relations: a play stays in the history after its file or title is removed.
//!
//! `StatsShare` is one grant of read-only statistics: to one user, or to everyone on this
//! server. No rows means private.

use crate::graphql::entities::*;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "play_history",
    plural = "PlayHistories",
    default_sort = "started_at",
    index = "user_id,started_at",
    index = "playback_session_id",
    index = "source_progress_id",
    read_policy = "member.read",
    write_policy = "member.write"
)]
pub struct PlayHistory {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "deviceId")]
    #[filterable(type = "string")]
    pub device_id: Option<String>,

    /// The device's name when the play happened.
    #[graphql(name = "deviceName")]
    pub device_name: Option<String>,

    #[graphql(name = "playbackSessionId")]
    #[filterable(type = "string")]
    pub playback_session_id: Option<String>,

    /// `movie`, `episode`, `track` or `audiobook`.
    #[graphql(name = "contentType")]
    #[filterable(type = "string")]
    pub content_type: String,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: Option<String>,

    #[graphql(name = "movieId")]
    #[filterable(type = "string")]
    pub movie_id: Option<String>,

    #[graphql(name = "episodeId")]
    #[filterable(type = "string")]
    pub episode_id: Option<String>,

    #[graphql(name = "showId")]
    #[filterable(type = "string")]
    pub show_id: Option<String>,

    #[graphql(name = "trackId")]
    #[filterable(type = "string")]
    pub track_id: Option<String>,

    #[graphql(name = "albumId")]
    #[filterable(type = "string")]
    pub album_id: Option<String>,

    #[graphql(name = "artistId")]
    #[filterable(type = "string")]
    pub artist_id: Option<String>,

    #[graphql(name = "audiobookId")]
    #[filterable(type = "string")]
    pub audiobook_id: Option<String>,

    #[graphql(name = "chapterId")]
    #[filterable(type = "string")]
    pub chapter_id: Option<String>,

    /// What was played, as it was named then ("Pilot", "Paranoid Android").
    #[graphql(name = "title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: String,

    /// The show, artist or author it belongs to, as named then.
    #[graphql(name = "parentTitle")]
    pub parent_title: Option<String>,

    /// "S1 · E3", the album name or the chapter: one line under the title.
    #[graphql(name = "subtitle")]
    pub subtitle: Option<String>,

    #[graphql(name = "startedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub started_at: String,

    /// The last moment anything was played in it.
    #[graphql(name = "endedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub ended_at: String,

    /// Seconds of real playing: wall time while playing, never the distance of a seek.
    #[graphql(name = "secondsPlayed")]
    #[filterable(type = "number")]
    #[sortable]
    pub seconds_played: f64,

    #[graphql(name = "startPosition")]
    pub start_position: f64,

    #[graphql(name = "endPosition")]
    pub end_position: f64,

    #[graphql(name = "duration")]
    pub duration: Option<f64>,

    #[graphql(name = "completed")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub completed: bool,

    /// `direct`, `hls`, `transcode` or `cast`; null when not known (imported).
    #[graphql(name = "playMethod")]
    #[filterable(type = "string")]
    pub play_method: Option<String>,

    /// Seeded from watch progress: the play happened, its length is unknown.
    #[graphql(name = "imported")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub imported: bool,

    /// For an imported play, the checkpoint it came from, so an import runs once per item.
    #[graphql(name = "sourceProgressId")]
    #[filterable(type = "string")]
    pub source_progress_id: Option<String>,

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
pub struct PlayHistoryCustomOperations;

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "stats_shares",
    plural = "StatsShares",
    default_sort = "created_at",
    index = "user_id",
    index = "viewer_user_id",
    read_policy = "member.read",
    write_policy = "member.write"
)]
pub struct StatsShare {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    /// Whose statistics are shared.
    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,

    /// `everyone` or `user`.
    #[graphql(name = "audience")]
    #[filterable(type = "string")]
    pub audience: String,

    /// The user they are shared with; null when the audience is everyone.
    #[graphql(name = "viewerUserId")]
    #[filterable(type = "string")]
    pub viewer_user_id: Option<String>,

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
pub struct StatsShareCustomOperations;
