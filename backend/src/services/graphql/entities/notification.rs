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
#[graphql(name = "Notification")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "notifications",
    plural = "Notifications",
    default_sort = "created_at"
)]
pub struct Notification {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "NotificationType")]
    #[filterable(type = "string")]
    #[sortable]
    pub notification_type: String,

    #[graphql(name = "Category")]
    #[filterable(type = "string")]
    #[sortable]
    pub category: String,

    #[graphql(name = "Title")]
    #[filterable(type = "string")]
    pub title: String,

    #[graphql(name = "Message")]
    pub message: String,

    #[graphql(name = "LibraryId")]
    #[filterable(type = "string")]
    pub library_id: Option<String>,

    #[graphql(name = "TorrentId")]
    #[filterable(type = "string")]
    pub torrent_id: Option<String>,

    #[graphql(name = "MediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: Option<String>,

    #[graphql(name = "PendingMatchId")]
    #[filterable(type = "string")]
    pub pending_match_id: Option<String>,

    #[graphql(name = "ActionType")]
    #[filterable(type = "string")]
    pub action_type: Option<String>,

    #[graphql(name = "ActionData")]
    pub action_data: Option<String>,

    #[graphql(name = "ReadAt")]
    #[filterable(type = "date")]
    pub read_at: Option<String>,

    #[graphql(name = "ResolvedAt")]
    #[filterable(type = "date")]
    pub resolved_at: Option<String>,

    #[graphql(name = "Resolution")]
    #[filterable(type = "string")]
    pub resolution: Option<String>,

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
pub struct NotificationCustomOperations;
