//! GraphQL authentication helpers.

use async_graphql::{Context, ErrorExtensions, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub user_id: String,
    pub email: Option<String>,
    pub role: Option<String>,
}

impl From<agql_auth::AuthUser> for AuthUser {
    fn from(user: agql_auth::AuthUser) -> Self {
        let role = user.roles.first().cloned();
        Self {
            user_id: user.user_id,
            email: None,
            role,
        }
    }
}

pub fn verify_token(_token: &str, _jwt_secret: &str) -> Result<AuthUser> {
    Err(
        async_graphql::Error::new("direct JWT verification has been replaced by agql-auth")
            .extend_with(|_, e| e.set("code", "UNAUTHORIZED")),
    )
}

pub trait AuthExt {
    fn librarian_auth_user(&self) -> Result<&AuthUser>;
    fn try_auth_user(&self) -> Option<&AuthUser>;
}

impl<'a> AuthExt for Context<'a> {
    fn librarian_auth_user(&self) -> Result<&AuthUser> {
        self.data_opt::<AuthUser>().ok_or_else(|| {
            async_graphql::Error::new("Authentication required")
                .extend_with(|_, e| e.set("code", "UNAUTHORIZED"))
        })
    }

    fn try_auth_user(&self) -> Option<&AuthUser> {
        self.data_opt::<AuthUser>()
    }
}

pub struct AuthGuard;

impl async_graphql::Guard for AuthGuard {
    fn check(&self, ctx: &Context<'_>) -> impl std::future::Future<Output = Result<()>> + Send {
        let result = ctx.librarian_auth_user().map(|_| ());
        async move { result }
    }
}

pub struct RoleGuard {
    pub role: String,
}

impl RoleGuard {
    pub fn new(role: impl Into<String>) -> Self {
        Self { role: role.into() }
    }
}

impl async_graphql::Guard for RoleGuard {
    fn check(&self, ctx: &Context<'_>) -> impl std::future::Future<Output = Result<()>> + Send {
        let result = ctx.librarian_auth_user().and_then(|user| {
            if user.role.as_deref() == Some(self.role.as_str()) {
                Ok(())
            } else {
                Err(
                    async_graphql::Error::new(format!("Role '{}' required", self.role))
                        .extend_with(|_, e| e.set("code", "FORBIDDEN")),
                )
            }
        });
        async move { result }
    }
}
