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
    default_sort = "created_at",
    read_policy = "admin.read",
    write_policy = "admin.write"
)]
pub struct RefreshToken {
    #[graphql(name = "id")]
    #[primary_key]
    #[graphql_orm(auto_generated = false)]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql_orm(private)]
    #[filterable(type = "string")]
    pub token_hash: String,

    #[graphql(name = "sessionId")]
    #[filterable(type = "string")]
    pub session_id: String,

    #[graphql(name = "sessionFamilyId")]
    #[filterable(type = "string")]
    pub session_family_id: String,

    #[graphql(name = "scopes")]
    #[json_field]
    pub scopes: Vec<String>,

    #[graphql_orm(private)]
    pub session: String,

    #[graphql_orm(private)]
    pub ip_address: Option<String>,

    #[graphql_orm(private)]
    pub user_agent: Option<String>,

    #[graphql(name = "expiresAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub expires_at: String,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "lastUsedAt")]
    #[filterable(type = "date")]
    pub last_used_at: Option<String>,

    #[graphql(name = "revokedAt")]
    #[filterable(type = "date")]
    pub revoked_at: Option<String>,

    #[graphql(name = "replacedByTokenId")]
    pub replaced_by_token_id: Option<String>,

    #[graphql(name = "revocationReason")]
    pub revocation_reason: Option<String>,
}

#[derive(Default)]
pub struct RefreshTokenCustomOperations;
