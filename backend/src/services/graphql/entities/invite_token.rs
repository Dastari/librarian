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
    table = "invite_tokens",
    plural = "InviteTokens",
    default_sort = "created_at",
    read_policy = "admin.read",
    write_policy = "admin.write"
)]
pub struct InviteToken {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql_orm(private)]
    #[filterable(type = "string")]
    pub token: String,

    #[graphql(name = "createdBy")]
    #[filterable(type = "string")]
    pub created_by: String,

    #[graphql(name = "libraryIds")]
    #[json_field]
    pub library_ids: Vec<String>,

    #[graphql(name = "role")]
    #[filterable(type = "string")]
    pub role: String,

    #[graphql(name = "accessLevel")]
    #[filterable(type = "string")]
    pub access_level: String,

    #[graphql(name = "expiresAt")]
    #[filterable(type = "date")]
    pub expires_at: Option<String>,

    #[graphql(name = "maxUses")]
    #[filterable(type = "number")]
    pub max_uses: Option<i32>,

    #[graphql(name = "useCount")]
    #[filterable(type = "number")]
    pub use_count: i32,

    #[graphql(name = "applyRestrictions")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub apply_restrictions: bool,

    #[graphql(name = "restrictionsTemplate")]
    pub restrictions_template: Option<String>,

    #[graphql(name = "isActive")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_active: bool,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,
}

#[derive(Default)]
pub struct InviteTokenCustomOperations;
