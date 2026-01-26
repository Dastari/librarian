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
#[graphql(name = "UsenetServer")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "usenet_servers",
    plural = "UsenetServers",
    default_sort = "priority"
)]
pub struct UsenetServer {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "Host")]
    #[filterable(type = "string")]
    pub host: String,

    #[graphql(name = "Port")]
    #[filterable(type = "number")]
    pub port: i32,

    #[graphql(name = "UseSsl")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub use_ssl: bool,

    #[graphql(name = "Username")]
    pub username: Option<String>,

    #[graphql(name = "EncryptedPassword")]
    pub encrypted_password: Option<String>,

    #[graphql(name = "PasswordNonce")]
    pub password_nonce: Option<String>,

    #[graphql(name = "Connections")]
    #[filterable(type = "number")]
    pub connections: i32,

    #[graphql(name = "Priority")]
    #[filterable(type = "number")]
    #[sortable]
    pub priority: i32,

    #[graphql(name = "RetentionDays")]
    #[filterable(type = "number")]
    pub retention_days: Option<i32>,

    #[graphql(name = "Enabled")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub enabled: bool,

    #[graphql(name = "LastError")]
    pub last_error: Option<String>,

    #[graphql(name = "ErrorCount")]
    #[filterable(type = "number")]
    pub error_count: i32,

    #[graphql(name = "LastSuccessAt")]
    #[filterable(type = "date")]
    pub last_success_at: Option<String>,

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
pub struct UsenetServerCustomOperations;
