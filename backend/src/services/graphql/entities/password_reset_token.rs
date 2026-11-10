//! One-time password reset links an administrator made for someone (design.md Q148). The link
//! itself is a signed token that is shown once and never stored; this row is its id, so it can
//! be used only once and stops working when it expires. Administrators can read the rows; only
//! the auth actions write them.

use crate::graphql::entities::*;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "password_reset_tokens",
    plural = "PasswordResetTokens",
    default_sort = "created_at",
    index = "user_id",
    read_policy = "admin.read",
    write_policy = "admin.write"
)]
pub struct PasswordResetToken {
    /// The signed token's id (`jti`).
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    pub user_id: String,

    /// The administrator who made the link.
    #[graphql(name = "createdByUserId")]
    #[filterable(type = "string")]
    pub created_by_user_id: Option<String>,

    #[graphql(name = "expiresAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub expires_at: String,

    /// When the link was used; it never works again.
    #[graphql(name = "consumedAt")]
    #[filterable(type = "date")]
    pub consumed_at: Option<String>,

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
pub struct PasswordResetTokenCustomOperations;
