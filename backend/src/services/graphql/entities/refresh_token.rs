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
    table = "refresh_tokens",
    plural = "RefreshTokens",
    default_sort = "created_at"
)]
pub struct RefreshToken {
    #[graphql(name = "Id")]
    #[primary_key]
    #[graphql_orm(auto_generated = false)]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "TokenHash")]
    #[filterable(type = "string")]
    pub token_hash: String,

    #[graphql(name = "SessionId")]
    #[filterable(type = "string")]
    pub session_id: String,

    #[graphql(name = "SessionFamilyId")]
    #[filterable(type = "string")]
    pub session_family_id: String,

    #[graphql(name = "Scopes")]
    #[json_field]
    pub scopes: Vec<String>,

    #[graphql(name = "Session")]
    pub session: String,

    #[graphql(name = "IpAddress")]
    pub ip_address: Option<String>,

    #[graphql(name = "UserAgent")]
    pub user_agent: Option<String>,

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

    #[graphql(name = "RevokedAt")]
    #[filterable(type = "date")]
    pub revoked_at: Option<String>,

    #[graphql(name = "ReplacedByTokenId")]
    pub replaced_by_token_id: Option<String>,

    #[graphql(name = "RevocationReason")]
    pub revocation_reason: Option<String>,
}

#[derive(Default)]
pub struct RefreshTokenCustomOperations;
