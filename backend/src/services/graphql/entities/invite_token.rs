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
#[graphql(name = "InviteToken")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "invite_tokens",
    plural = "InviteTokens",
    default_sort = "created_at"
)]
pub struct InviteToken {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "Token")]
    #[filterable(type = "string")]
    pub token: String,

    #[graphql(name = "CreatedBy")]
    #[filterable(type = "string")]
    pub created_by: String,

    #[graphql(name = "LibraryIds")]
    #[json_field]
    pub library_ids: Vec<String>,

    #[graphql(name = "Role")]
    #[filterable(type = "string")]
    pub role: String,

    #[graphql(name = "AccessLevel")]
    #[filterable(type = "string")]
    pub access_level: String,

    #[graphql(name = "ExpiresAt")]
    #[filterable(type = "date")]
    pub expires_at: Option<String>,

    #[graphql(name = "MaxUses")]
    #[filterable(type = "number")]
    pub max_uses: Option<i32>,

    #[graphql(name = "UseCount")]
    #[filterable(type = "number")]
    pub use_count: i32,

    #[graphql(name = "ApplyRestrictions")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub apply_restrictions: bool,

    #[graphql(name = "RestrictionsTemplate")]
    pub restrictions_template: Option<String>,

    #[graphql(name = "IsActive")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_active: bool,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,
}

#[derive(Default)]
pub struct InviteTokenCustomOperations;
