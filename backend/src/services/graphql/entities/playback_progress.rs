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
    default_sort = "updated_at",
    index = "user_id"
)]
pub struct PlaybackProgress {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub media_file_id: Option<String>,

    #[graphql(name = "currentPosition")]
    #[filterable(type = "number")]
    pub current_position: f64,

    #[graphql(name = "duration")]
    #[filterable(type = "number")]
    pub duration: Option<f64>,

    #[graphql(name = "progressPercent")]
    #[filterable(type = "number")]
    pub progress_percent: f64,

    #[graphql(name = "isWatched")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_watched: bool,

    #[graphql(name = "watchedAt")]
    #[filterable(type = "date")]
    pub watched_at: Option<String>,

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
pub struct PlaybackProgressCustomOperations;
