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
    table = "cast_sessions",
    plural = "CastSessions",
    default_sort = "started_at",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct CastSession {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    pub user_id: Option<String>,

    #[graphql(name = "deviceId")]
    #[filterable(type = "string")]
    pub device_id: Option<String>,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: Option<String>,

    #[graphql(name = "episodeId")]
    #[filterable(type = "string")]
    pub episode_id: Option<String>,

    #[graphql(skip)]
    #[input_only]
    pub stream_url: String,

    #[graphql(name = "receiverAddress")]
    pub receiver_address: Option<String>,

    #[graphql(name = "grantExpiresAt")]
    #[filterable(type = "date")]
    pub grant_expires_at: Option<String>,

    #[graphql(name = "receiverTransportId")]
    pub receiver_transport_id: Option<String>,

    #[graphql(name = "receiverSessionId")]
    pub receiver_session_id: Option<String>,

    #[graphql(name = "mediaSessionId")]
    pub media_session_id: Option<i32>,

    #[graphql(name = "lastError")]
    pub last_error: Option<String>,

    #[graphql(name = "playbackDecision")]
    #[filterable(type = "string")]
    pub playback_decision: Option<String>,

    #[graphql(name = "playbackReason")]
    pub playback_reason: Option<String>,

    #[graphql(name = "playerState")]
    #[filterable(type = "string")]
    pub player_state: String,

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

    #[graphql(name = "startedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub started_at: String,

    #[graphql(name = "endedAt")]
    #[filterable(type = "date")]
    pub ended_at: Option<String>,

    #[graphql(name = "lastPosition")]
    #[filterable(type = "number")]
    pub last_position: Option<f64>,

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
pub struct CastSessionCustomOperations;
