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
    table = "schedule_sync_state",
    plural = "ScheduleSyncStates",
    default_sort = "country_code",
    read_policy = "admin.read",
    write_policy = "admin.write"
)]
pub struct ScheduleSyncState {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "countryCode")]
    #[filterable(type = "string")]
    #[sortable]
    pub country_code: String,

    #[graphql(name = "lastSyncedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_synced_at: String,

    #[graphql(name = "lastSyncDays")]
    #[filterable(type = "number")]
    pub last_sync_days: i32,

    #[graphql(name = "syncError")]
    pub sync_error: Option<String>,

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
pub struct ScheduleSyncStateCustomOperations;
