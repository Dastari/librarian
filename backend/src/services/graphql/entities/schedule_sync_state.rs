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
    default_sort = "country_code"
)]
pub struct ScheduleSyncState {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "CountryCode")]
    #[filterable(type = "string")]
    #[sortable]
    pub country_code: String,

    #[graphql(name = "LastSyncedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_synced_at: String,

    #[graphql(name = "LastSyncDays")]
    #[filterable(type = "number")]
    pub last_sync_days: i32,

    #[graphql(name = "SyncError")]
    pub sync_error: Option<String>,

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
pub struct ScheduleSyncStateCustomOperations;
