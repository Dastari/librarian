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
    table = "usenet_servers",
    plural = "UsenetServers",
    default_sort = "priority",
    read_policy = "admin.read",
    write_policy = "admin.write"
)]
pub struct UsenetServer {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "host")]
    #[filterable(type = "string")]
    pub host: String,

    #[graphql(name = "port")]
    #[filterable(type = "number")]
    pub port: i32,

    #[graphql(name = "useSsl")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub use_ssl: bool,

    #[graphql(name = "username")]
    pub username: Option<String>,

    #[graphql_orm(private)]
    pub encrypted_password: Option<String>,

    #[graphql_orm(private)]
    pub password_nonce: Option<String>,

    #[graphql(name = "connections")]
    #[filterable(type = "number")]
    pub connections: i32,

    #[graphql(name = "priority")]
    #[filterable(type = "number")]
    #[sortable]
    pub priority: i32,

    #[graphql(name = "retentionDays")]
    #[filterable(type = "number")]
    pub retention_days: Option<i32>,

    #[graphql(name = "enabled")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub enabled: bool,

    #[graphql(name = "lastError")]
    pub last_error: Option<String>,

    #[graphql(name = "errorCount")]
    #[filterable(type = "number")]
    pub error_count: i32,

    #[graphql(name = "lastSuccessAt")]
    #[filterable(type = "date")]
    pub last_success_at: Option<String>,

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
pub struct UsenetServerCustomOperations;
