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
#[graphql(name = "RefreshToken")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "refresh_tokens",
    plural = "RefreshTokens",
    default_sort = "created_at"
)]
pub struct RefreshToken {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "TokenHash")]
    #[filterable(type = "string")]
    pub token_hash: String,

    #[graphql(name = "DeviceInfo")]
    pub device_info: Option<String>,

    #[graphql(name = "IpAddress")]
    pub ip_address: Option<String>,

    #[graphql(name = "ExpiresAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub expires_at: String,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "LastUsedAt")]
    #[filterable(type = "date")]
    pub last_used_at: Option<String>,
}

#[derive(Default)]
pub struct RefreshTokenCustomOperations;
