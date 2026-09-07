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
    table = "notifications",
    plural = "Notifications",
    default_sort = "created_at",
    index = "user_id"
)]
pub struct Notification {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,

    #[graphql(name = "notificationType")]
    #[filterable(type = "string")]
    #[sortable]
    pub notification_type: String,

    #[graphql(name = "category")]
    #[filterable(type = "string")]
    #[sortable]
    pub category: String,

    #[graphql(name = "title")]
    #[filterable(type = "string")]
    pub title: String,

    #[graphql(name = "message")]
    pub message: String,

    #[graphql(name = "libraryId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub library_id: Option<String>,

    #[graphql(name = "torrentId")]
    #[filterable(type = "string")]
    pub torrent_id: Option<String>,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub media_file_id: Option<String>,

    #[graphql(name = "pendingMatchId")]
    #[filterable(type = "string")]
    pub pending_match_id: Option<String>,

    #[graphql(name = "actionType")]
    #[filterable(type = "string")]
    pub action_type: Option<String>,

    #[graphql(name = "actionData")]
    pub action_data: Option<String>,

    #[graphql(name = "readAt")]
    #[filterable(type = "date")]
    pub read_at: Option<String>,

    #[graphql(name = "resolvedAt")]
    #[filterable(type = "date")]
    pub resolved_at: Option<String>,

    #[graphql(name = "resolution")]
    #[filterable(type = "string")]
    pub resolution: Option<String>,

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
pub struct NotificationCustomOperations;
