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
#[graphql(name = "PlaybackSession")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "playback_sessions",
    plural = "PlaybackSessions",
    default_sort = "started_at"
)]
pub struct PlaybackSession {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "ContentType")]
    #[filterable(type = "string")]
    pub content_type: Option<String>,

    #[graphql(name = "MediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: Option<String>,

    #[graphql(name = "EpisodeId")]
    #[filterable(type = "string")]
    pub episode_id: Option<String>,

    #[graphql(name = "MovieId")]
    #[filterable(type = "string")]
    pub movie_id: Option<String>,

    #[graphql(name = "TrackId")]
    #[filterable(type = "string")]
    pub track_id: Option<String>,

    #[graphql(name = "AudiobookId")]
    #[filterable(type = "string")]
    pub audiobook_id: Option<String>,

    #[graphql(name = "TvShowId")]
    #[filterable(type = "string")]
    pub tv_show_id: Option<String>,

    #[graphql(name = "AlbumId")]
    #[filterable(type = "string")]
    pub album_id: Option<String>,

    #[graphql(name = "CurrentPosition")]
    #[filterable(type = "number")]
    pub current_position: f64,

    #[graphql(name = "Duration")]
    #[filterable(type = "number")]
    pub duration: Option<f64>,

    #[graphql(name = "Volume")]
    #[filterable(type = "number")]
    pub volume: f64,

    #[graphql(name = "IsMuted")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_muted: bool,

    #[graphql(name = "IsPlaying")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_playing: bool,

    #[graphql(name = "StartedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub started_at: String,

    #[graphql(name = "LastUpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_updated_at: String,

    #[graphql(name = "CompletedAt")]
    #[filterable(type = "date")]
    pub completed_at: Option<String>,

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
pub struct PlaybackSessionCustomOperations;
