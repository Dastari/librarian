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
    table = "playback_progress",
    plural = "PlaybackProgresses",
    default_sort = "updated_at"
)]
pub struct PlaybackProgress {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "MediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: Option<String>,

    #[graphql(name = "CurrentPosition")]
    #[filterable(type = "number")]
    pub current_position: f64,

    #[graphql(name = "Duration")]
    #[filterable(type = "number")]
    pub duration: Option<f64>,

    #[graphql(name = "ProgressPercent")]
    #[filterable(type = "number")]
    pub progress_percent: f64,

    #[graphql(name = "IsWatched")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_watched: bool,

    #[graphql(name = "WatchedAt")]
    #[filterable(type = "date")]
    pub watched_at: Option<String>,

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
pub struct PlaybackProgressCustomOperations;
