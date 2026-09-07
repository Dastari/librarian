use std::sync::Arc;

use crate::graphql::entities::*;
use async_graphql::{Context, Object, Result};
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use super::super::auth::AuthUser;
use crate::services::auth::AuthService;

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "users",
    plural = "Users",
    default_sort = "username",
    read_policy = "admin.read",
    write_policy = "admin.write"
)]
pub struct User {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "username")]
    #[filterable(type = "string")]
    #[sortable]
    pub username: String,

    #[graphql(name = "email")]
    #[filterable(type = "string")]
    pub email: Option<String>,

    #[graphql_orm(private)]
    pub password_hash: String,

    #[graphql(name = "role")]
    #[filterable(type = "string")]
    #[sortable]
    pub role: String,

    #[graphql(name = "displayName")]
    #[filterable(type = "string")]
    pub display_name: Option<String>,

    #[graphql(name = "avatarUrl")]
    pub avatar_url: Option<String>,

    #[graphql(name = "isActive")]
    #[filterable(type = "boolean")]
    pub is_active: bool,

    #[graphql(name = "lastLoginAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_login_at: Option<String>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

/// Current user info returned by the Me query.
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(name = "MeUser")]
pub struct MeUser {
    #[graphql(name = "id")]
    pub id: String,
    #[graphql(name = "email")]
    pub email: Option<String>,
    #[graphql(name = "username")]
    pub username: String,
    #[graphql(name = "role")]
    pub role: String,
    #[graphql(name = "displayName")]
    pub display_name: Option<String>,
}

#[derive(Default)]
pub struct UserCustomOperations;

#[Object]
impl UserCustomOperations {
    /// True if no admin user exists yet (first-time setup required).
    #[graphql(name = "needsSetup")]
    async fn needs_setup(&self, ctx: &Context<'_>) -> Result<bool> {
        let auth = ctx
            .data::<Arc<AuthService>>()
            .map_err(|e| async_graphql::Error::new(format!("Auth service unavailable: {:?}", e)))?;
        auth.needs_setup()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))
    }

    /// Current authenticated user (requires valid JWT). Returns null if not authenticated.
    #[graphql(name = "me")]
    async fn me(&self, ctx: &Context<'_>) -> Result<Option<MeUser>> {
        let auth_user = match ctx.data_opt::<AuthUser>() {
            Some(u) => u,
            None => return Ok(None),
        };
        let db = ctx
            .data::<crate::db::Database>()
            .map_err(|e| async_graphql::Error::new(format!("Database unavailable: {:?}", e)))?;
        let user = match User::get(db.pool(), &auth_user.user_id).await {
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
