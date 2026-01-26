//! AppLog Entity

use async_graphql::SimpleObject;
use librarian_macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

/// AppLog Entity - application logs
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
#[graphql(name = "AppLog")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "app_logs", plural = "AppLogs", default_sort = "timestamp")]
pub struct AppLog {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "Timestamp")]
    #[filterable(type = "date")]
    #[sortable]
    pub timestamp: String,

    #[graphql(name = "Level")]
    #[filterable(type = "string")]
    #[sortable]
    pub level: String,

    #[graphql(name = "Target")]
    #[filterable(type = "string")]
    #[sortable]
    pub target: String,

    #[graphql(name = "Message")]
    #[filterable(type = "string")]
    pub message: String,

    #[graphql(name = "Fields")]
    pub fields: Option<String>,

    #[graphql(name = "SpanName")]
    #[filterable(type = "string")]
    pub span_name: Option<String>,

    #[graphql(name = "SpanId")]
    #[filterable(type = "string")]
    pub span_id: Option<String>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,
}

#[derive(Default)]
pub struct AppLogCustomOperations;
