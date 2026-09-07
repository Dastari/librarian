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
    table = "playback_sessions",
    plural = "PlaybackSessions",
    default_sort = "started_at"
)]
pub struct PlaybackSession {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,

    #[graphql(name = "contentType")]
    #[filterable(type = "string")]
    pub content_type: Option<String>,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub media_file_id: Option<String>,

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

    #[graphql(name = "audiobookId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub audiobook_id: Option<String>,

    #[graphql(name = "tvShowId")]
    #[filterable(type = "string")]
    pub tv_show_id: Option<String>,

    #[graphql(name = "albumId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub album_id: Option<String>,

    #[graphql(name = "currentPosition")]
    #[filterable(type = "number")]
    pub current_position: f64,

    #[graphql(name = "duration")]
    #[filterable(type = "number")]
    pub duration: Option<f64>,

    #[graphql(name = "volume")]
    #[filterable(type = "number")]
    pub volume: f64,

    #[graphql(name = "isMuted")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_muted: bool,

    #[graphql(name = "isPlaying")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_playing: bool,

    #[graphql(name = "startedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub started_at: String,

    #[graphql(name = "lastUpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_updated_at: String,

    #[graphql(name = "completedAt")]
    #[filterable(type = "date")]
    pub completed_at: Option<String>,

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
pub struct PlaybackSessionCustomOperations;
