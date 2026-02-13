//! GraphQL auth mutations: register, login, refresh, logout.
//! Resolvers delegate to [crate::services::auth::AuthService]; types use PascalCase.

use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Result};

use crate::services::auth::{AuthService, AuthTokens, AuthenticatedUser, RegisterInput};
use chrono::{DateTime, Utc};

const REFRESH_TOKEN_COOKIE: &str = "librarian_refresh_token";

#[derive(Debug, Clone, Default)]
pub struct AuthCookieContext {
    pub refresh_token: Option<String>,
    pub secure: bool,
}

fn read_refresh_token(ctx: &Context<'_>, input_refresh_token: &str) -> Option<String> {
    if !input_refresh_token.trim().is_empty() {
        return Some(input_refresh_token.to_string());
    }

    ctx.data_opt::<AuthCookieContext>()
        .and_then(|c| c.refresh_token.clone())
        .filter(|token| !token.trim().is_empty())
}

fn append_refresh_cookie(
    ctx: &Context<'_>,
    refresh_token: &str,
    max_age_seconds: i64,
    secure: bool,
) {
    let secure_attr = if secure { "; Secure" } else { "" };
    let cookie = format!(
        "{name}={value}; Max-Age={max_age}; Path=/; HttpOnly; SameSite=Lax{secure}",
        name = REFRESH_TOKEN_COOKIE,
        value = refresh_token,
        max_age = max_age_seconds.max(0),
        secure = secure_attr
    );
    let _ = ctx.append_http_header("Set-Cookie", cookie);
}

fn clear_refresh_cookie(ctx: &Context<'_>, secure: bool) {
    let secure_attr = if secure { "; Secure" } else { "" };
    let expires = DateTime::<Utc>::from(std::time::UNIX_EPOCH).to_rfc2822();
    let cookie = format!(
        "{name}=; Max-Age=0; Expires={expires}; Path=/; HttpOnly; SameSite=Lax{secure}",
        name = REFRESH_TOKEN_COOKIE,
        expires = expires,
        secure = secure_attr
    );
    let _ = ctx.append_http_header("Set-Cookie", cookie);
}

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
        let auth = ctx
            .data::<Arc<AuthService>>()
            .map_err(|e| async_graphql::Error::new(format!("Auth service unavailable: {:?}", e)))?;
        let secure_cookie = ctx
            .data_opt::<AuthCookieContext>()
            .map(|c| c.secure)
            .unwrap_or(false);
        let inner = RegisterInput {
            email: input.email,
            name: input.name,
            password: input.password,
        };
        match auth.register(inner).await {
            Ok(login_result) => {
                append_refresh_cookie(
                    ctx,
                    &login_result.tokens.refresh_token,
                    auth.refresh_token_lifetime_seconds(),
                    secure_cookie,
                );
                Ok(AuthPayload {
                    success: true,
                    error: None,
                    user: Some(login_result.user),
                    tokens: Some(login_result.tokens),
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

    #[graphql(name = "Login")]
    async fn login(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: LoginInput,
    ) -> Result<AuthPayload> {
        let auth = ctx
            .data::<Arc<AuthService>>()
            .map_err(|e| async_graphql::Error::new(format!("Auth service unavailable: {:?}", e)))?;
        let secure_cookie = ctx
            .data_opt::<AuthCookieContext>()
            .map(|c| c.secure)
            .unwrap_or(false);
        match auth.login(&input.username_or_email, &input.password).await {
            Ok(login_result) => {
                append_refresh_cookie(
                    ctx,
                    &login_result.tokens.refresh_token,
                    auth.refresh_token_lifetime_seconds(),
                    secure_cookie,
                );
                Ok(AuthPayload {
                    success: true,
                    error: None,
                    user: Some(login_result.user),
                    tokens: Some(login_result.tokens),
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

    #[graphql(name = "RefreshToken")]
    async fn refresh_token(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: RefreshTokenInput,
    ) -> Result<AuthPayload> {
        let auth = ctx
            .data::<Arc<AuthService>>()
            .map_err(|e| async_graphql::Error::new(format!("Auth service unavailable: {:?}", e)))?;
        let secure_cookie = ctx
            .data_opt::<AuthCookieContext>()
            .map(|c| c.secure)
            .unwrap_or(false);
        let refresh_token = match read_refresh_token(ctx, &input.refresh_token) {
            Some(token) => token,
            None => {
                clear_refresh_cookie(ctx, secure_cookie);
                return Ok(AuthPayload {
                    success: false,
                    error: Some("Refresh token missing".to_string()),
                    user: None,
                    tokens: None,
                });
            }
        };

        match auth.refresh_token(&refresh_token).await {
            Ok(tokens) => {
                append_refresh_cookie(
                    ctx,
                    &tokens.refresh_token,
                    auth.refresh_token_lifetime_seconds(),
                    secure_cookie,
                );
                let user = auth.validate_access_token(&tokens.access_token).await.ok();
                Ok(AuthPayload {
                    success: true,
                    error: None,
                    user,
                    tokens: Some(tokens),
                })
            }
            Err(e) => {
                clear_refresh_cookie(ctx, secure_cookie);
                Ok(AuthPayload {
                    success: false,
                    error: Some(e.to_string()),
                    user: None,
                    tokens: None,
                })
            }
        }
    }

    #[graphql(name = "Logout")]
    async fn logout(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: LogoutInput,
    ) -> Result<LogoutPayload> {
        let auth = ctx
            .data::<Arc<AuthService>>()
            .map_err(|e| async_graphql::Error::new(format!("Auth service unavailable: {:?}", e)))?;
        let secure_cookie = ctx
            .data_opt::<AuthCookieContext>()
            .map(|c| c.secure)
            .unwrap_or(false);
        let refresh_token = read_refresh_token(ctx, &input.refresh_token);

        clear_refresh_cookie(ctx, secure_cookie);

        if let Some(token) = refresh_token {
            match auth.logout(&token).await {
                Ok(()) => Ok(LogoutPayload {
                    success: true,
                    error: None,
                }),
                Err(e) => Ok(LogoutPayload {
                    success: false,
                    error: Some(e.to_string()),
                }),
            }
        } else {
            Ok(LogoutPayload {
                success: true,
                error: None,
            })
        }
    }
}
