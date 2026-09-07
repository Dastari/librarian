//! AppLog Entity

use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

/// AppLog Entity - application logs
#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "app_logs",
    plural = "AppLogs",
    default_sort = "timestamp",
    read_policy = "admin.read",
    write_policy = "admin.write",
    index = "timestamp",
    index = "target",
    index = "level"
)]
pub struct AppLog {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "timestamp")]
    #[filterable(type = "date")]
    #[sortable]
    pub timestamp: String,

    #[graphql(name = "level")]
    #[filterable(type = "string")]
    #[sortable]
    pub level: String,

    #[graphql(name = "target")]
    #[filterable(type = "string")]
    #[sortable]
    pub target: String,

    #[graphql(name = "message")]
    #[filterable(type = "string")]
    pub message: String,

    #[graphql(name = "fields")]
    pub fields: Option<String>,

    #[graphql(name = "spanName")]
    #[filterable(type = "string")]
    pub span_name: Option<String>,

    #[graphql(name = "spanId")]
    #[filterable(type = "string")]
    pub span_id: Option<String>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,
}

#[derive(Default)]
pub struct AppLogCustomOperations;
