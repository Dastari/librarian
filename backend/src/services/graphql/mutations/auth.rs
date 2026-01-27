//! GraphQL auth mutations: register, login, refresh, logout.
//! Resolvers delegate to [crate::services::auth::AuthService]; types use PascalCase.

use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Result};

use crate::services::auth::{
    AuthService, AuthTokens, AuthenticatedUser, RegisterInput,
};

/// GraphQL input for user registration (PascalCase field names).
#[derive(Debug, Clone, InputObject)]
#[graphql(name = "RegisterUserInput")]
pub struct RegisterUserInput {
    #[graphql(name = "Email")]
    pub email: String,
    #[graphql(name = "Name")]
    pub name: String,
    #[graphql(name = "Password")]
    pub password: String,
}

/// GraphQL input for login (username or email + password).
#[derive(Debug, Clone, InputObject)]
#[graphql(name = "LoginInput")]
pub struct LoginInput {
    #[graphql(name = "UsernameOrEmail")]
    pub username_or_email: String,
    #[graphql(name = "Password")]
    pub password: String,
}

/// GraphQL input for logout (refresh token to invalidate).
#[derive(Debug, Clone, InputObject)]
#[graphql(name = "LogoutInput")]
pub struct LogoutInput {
    #[graphql(name = "RefreshToken")]
    pub refresh_token: String,
}

/// GraphQL input for refresh token mutation.
#[derive(Debug, Clone, InputObject)]
#[graphql(name = "RefreshTokenInput")]
pub struct RefreshTokenInput {
    #[graphql(name = "RefreshToken")]
    pub refresh_token: String,
}

/// Shared auth result: success flag, optional error, and optional user/tokens.
#[derive(Debug, Clone)]
pub struct AuthPayload {
    pub success: bool,
    pub error: Option<String>,
    pub user: Option<AuthenticatedUser>,
    pub tokens: Option<AuthTokens>,
}

#[async_graphql::Object]
impl AuthPayload {
    #[graphql(name = "Success")]
    async fn success(&self) -> bool {
        self.success
    }

    #[graphql(name = "Error")]
    async fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    #[graphql(name = "User")]
    async fn user(&self) -> Option<&AuthenticatedUser> {
        self.user.as_ref()
    }

    #[graphql(name = "Tokens")]
    async fn tokens(&self) -> Option<&AuthTokens> {
        self.tokens.as_ref()
    }
}

/// Root type for auth-related mutations (register, login, refresh, logout).
#[derive(Default)]
pub struct AuthMutations;

/// Result of logout: success and optional error message.
#[derive(Debug, Clone)]
pub struct LogoutPayload {
    pub success: bool,
    pub error: Option<String>,
}

#[async_graphql::Object]
impl LogoutPayload {
    #[graphql(name = "Success")]
    async fn success(&self) -> bool {
        self.success
    }

    #[graphql(name = "Error")]
    async fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

#[Object]
impl AuthMutations {
    #[graphql(name = "Register")]
    async fn register(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: RegisterUserInput,
    ) -> Result<AuthPayload> {
        let auth = ctx.data::<Arc<AuthService>>().map_err(|e| {
            async_graphql::Error::new(format!("Auth service unavailable: {:?}", e))
        })?;
        let inner = RegisterInput {
            email: input.email,
            name: input.name,
            password: input.password,
        };
        match auth.register(inner).await {
            Ok(login_result) => Ok(AuthPayload {
                success: true,
                error: None,
                user: Some(login_result.user),
                tokens: Some(login_result.tokens),
            }),
            Err(e) => Ok(AuthPayload {
                success: false,
                error: Some(e.to_string()),
                user: None,
                tokens: None,
            }),
        }
    }

    #[graphql(name = "Login")]
    async fn login(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: LoginInput,
    ) -> Result<AuthPayload> {
        let auth = ctx.data::<Arc<AuthService>>().map_err(|e| {
            async_graphql::Error::new(format!("Auth service unavailable: {:?}", e))
        })?;
        match auth
            .login(&input.username_or_email, &input.password)
            .await
        {
            Ok(login_result) => Ok(AuthPayload {
                success: true,
                error: None,
                user: Some(login_result.user),
                tokens: Some(login_result.tokens),
            }),
            Err(e) => Ok(AuthPayload {
                success: false,
                error: Some(e.to_string()),
                user: None,
                tokens: None,
            }),
        }
    }

    #[graphql(name = "RefreshToken")]
    async fn refresh_token(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: RefreshTokenInput,
    ) -> Result<AuthPayload> {
        let auth = ctx.data::<Arc<AuthService>>().map_err(|e| {
            async_graphql::Error::new(format!("Auth service unavailable: {:?}", e))
        })?;
        match auth.refresh_token(&input.refresh_token).await {
            Ok(tokens) => {
                let user = auth
                    .validate_access_token(&tokens.access_token)
                    .await
                    .ok();
                Ok(AuthPayload {
                    success: true,
                    error: None,
                    user,
                    tokens: Some(tokens),
                })
            }
            Err(e) => Ok(AuthPayload {
                success: false,
                error: Some(e.to_string()),
                user: None,
                tokens: None,
            }),
        }
    }

    #[graphql(name = "Logout")]
    async fn logout(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: LogoutInput,
    ) -> Result<LogoutPayload> {
        let auth = ctx.data::<Arc<AuthService>>().map_err(|e| {
            async_graphql::Error::new(format!("Auth service unavailable: {:?}", e))
        })?;
        match auth.logout(&input.refresh_token).await {
            Ok(()) => Ok(LogoutPayload {
                success: true,
                error: None,
            }),
            Err(e) => Ok(LogoutPayload {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }
}
