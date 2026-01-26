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
#[graphql(name = "CastSession")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "cast_sessions",
    plural = "CastSessions",
    default_sort = "started_at"
)]
pub struct CastSession {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "DeviceId")]
    #[filterable(type = "string")]
    pub device_id: Option<String>,

    #[graphql(name = "MediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: Option<String>,

    #[graphql(name = "EpisodeId")]
    #[filterable(type = "string")]
    pub episode_id: Option<String>,

    #[graphql(name = "StreamUrl")]
    pub stream_url: String,

    #[graphql(name = "PlayerState")]
    #[filterable(type = "string")]
    pub player_state: String,

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

    #[graphql(name = "StartedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub started_at: String,

    #[graphql(name = "EndedAt")]
    #[filterable(type = "date")]
    pub ended_at: Option<String>,

    #[graphql(name = "LastPosition")]
    #[filterable(type = "number")]
    pub last_position: Option<f64>,

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
pub struct CastSessionCustomOperations;
