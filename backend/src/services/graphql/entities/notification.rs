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
    table = "notifications",
    plural = "Notifications",
    default_sort = "created_at",
    index = "user_id",
    keyset = "created_at desc, id desc"
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
    #[sortable]
    pub title: String,

    #[graphql(name = "message")]
    #[filterable(type = "string")]
    pub message: String,

    #[graphql(name = "libraryId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub library_id: Option<String>,

    #[graphql(name = "torrentId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub torrent_id: Option<String>,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub media_file_id: Option<String>,

    #[graphql(name = "pendingMatchId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub pending_match_id: Option<String>,

    #[graphql(name = "actionType")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "admin.write")]
    pub action_type: Option<String>,

    #[graphql(name = "actionData")]
    #[graphql_orm(write_policy = "admin.write")]
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

    // Query relations do not add physical FKs or change existing delete behavior.
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Library",
        from = "library_id",
        to = "id",
        on_delete = "set_null",
        emit_fk = false
    )]
    pub library: Option<Box<Library>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Torrent",
        from = "torrent_id",
        to = "id",
        on_delete = "set_null",
        emit_fk = false
    )]
    pub torrent: Option<Box<Torrent>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "MediaFile",
        from = "media_file_id",
        to = "id",
        on_delete = "set_null",
        emit_fk = false
    )]
    pub media_file: Option<Box<MediaFile>>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "PendingFileMatch",
        from = "pending_match_id",
        to = "id",
        on_delete = "set_null",
        emit_fk = false
    )]
    pub pending_match: Option<Box<PendingFileMatch>>,
}

#[derive(Default)]
pub struct NotificationCustomOperations;
