use std::sync::Arc;

use async_graphql::{Context, Object, Result, SimpleObject};
use librarian_macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use crate::services::auth::AuthService;
use super::super::auth::AuthUser;

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
#[graphql(name = "User")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "users", plural = "Users", default_sort = "username")]
pub struct User {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "Username")]
    #[filterable(type = "string")]
    #[sortable]
    pub username: String,

    #[graphql(name = "Email")]
    #[filterable(type = "string")]
    pub email: Option<String>,

    #[graphql(skip)]
    pub password_hash: String,

    #[graphql(name = "Role")]
    #[filterable(type = "string")]
    #[sortable]
    pub role: String,

    #[graphql(name = "DisplayName")]
    #[filterable(type = "string")]
    pub display_name: Option<String>,

    #[graphql(name = "AvatarUrl")]
    pub avatar_url: Option<String>,

    #[graphql(name = "IsActive")]
    #[filterable(type = "boolean")]
    pub is_active: bool,

    #[graphql(name = "LastLoginAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_login_at: Option<String>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

/// Current user info returned by Me query (PascalCase).
#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "MeUser")]
pub struct MeUser {
    #[graphql(name = "Id")]
    pub id: String,
    #[graphql(name = "Email")]
    pub email: Option<String>,
    #[graphql(name = "Username")]
    pub username: String,
    #[graphql(name = "Role")]
    pub role: String,
    #[graphql(name = "DisplayName")]
    pub display_name: Option<String>,
}

#[derive(Default)]
pub struct UserCustomOperations;

#[Object]
impl UserCustomOperations {
    /// True if no admin user exists yet (first-time setup required).
    #[graphql(name = "NeedsSetup")]
    async fn needs_setup(&self, ctx: &Context<'_>) -> Result<bool> {
        let auth = ctx.data::<Arc<AuthService>>().map_err(|e| {
            async_graphql::Error::new(format!("Auth service unavailable: {:?}", e))
        })?;
        auth.needs_setup()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))
    }

    /// Current authenticated user (requires valid JWT). Returns null if not authenticated.
    #[graphql(name = "Me")]
    async fn me(&self, ctx: &Context<'_>) -> Result<Option<MeUser>> {
        let auth_user = match ctx.data_opt::<AuthUser>() {
            Some(u) => u,
            None => return Ok(None),
        };
        let db = ctx.data::<crate::db::Database>().map_err(|e| {
            async_graphql::Error::new(format!("Database unavailable: {:?}", e))
        })?;
        let user = match User::get(db, &auth_user.user_id).await {
            Ok(Some(u)) => u,
            _ => return Ok(None),
        };
        if !user.is_active {
            return Ok(None);
        }
        Ok(Some(MeUser {
            id: user.id,
            email: user.email,
            username: user.username,
            role: user.role,
            display_name: user.display_name,
        }))
    }
}
