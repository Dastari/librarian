//! GraphQL auth mutations: register, login, refresh, logout.
//! Resolvers delegate to [crate::services::auth::AuthService].

use std::sync::Arc;

use async_graphql::{Context, InputObject, Object, Result};

use crate::services::auth::{AuthService, AuthTokens, AuthenticatedUser, RegisterInput};
use crate::services::graphql::error::{internal_message, service_unavailable};
use chrono::{DateTime, Utc};

const REFRESH_TOKEN_COOKIE: &str = "librarian_refresh_token";
const ACCESS_TOKEN_COOKIE: &str = "librarian_access_token";

#[derive(Debug, Clone, Default)]
pub struct AuthCookieContext {
    pub refresh_token: Option<String>,
    pub secure: bool,
}

fn read_refresh_token(ctx: &Context<'_>) -> Option<String> {
    ctx.data_opt::<AuthCookieContext>()
        .and_then(|c| c.refresh_token.clone())
        .filter(|token| !token.trim().is_empty())
}

fn append_auth_cookie(
    ctx: &Context<'_>,
    name: &str,
    value: &str,
    path: &str,
    max_age_seconds: i64,
    secure: bool,
) {
    let cookie = build_auth_cookie(name, value, path, max_age_seconds, secure);
    let _ = ctx.append_http_header("Set-Cookie", cookie);
}

fn build_auth_cookie(
    name: &str,
    value: &str,
    path: &str,
    max_age_seconds: i64,
    secure: bool,
) -> String {
    let secure_attr = if secure { "; Secure" } else { "" };
    format!(
        "{name}={value}; Max-Age={max_age}; Path={path}; HttpOnly; SameSite=Lax{secure}",
        max_age = max_age_seconds.max(0),
        secure = secure_attr
    )
}

fn append_auth_cookies(
    ctx: &Context<'_>,
    tokens: &AuthTokens,
    access_max_age_seconds: i64,
    refresh_max_age_seconds: i64,
    secure: bool,
) {
    append_auth_cookie(
        ctx,
        ACCESS_TOKEN_COOKIE,
        &tokens.access_token,
        "/",
        access_max_age_seconds,
        secure,
    );
    append_auth_cookie(
        ctx,
        REFRESH_TOKEN_COOKIE,
        &tokens.refresh_token,
        "/graphql",
        refresh_max_age_seconds,
        secure,
    );
}

fn clear_auth_cookie(ctx: &Context<'_>, name: &str, path: &str, secure: bool) {
    let cookie = build_clear_auth_cookie(name, path, secure);
    let _ = ctx.append_http_header("Set-Cookie", cookie);
}

fn build_clear_auth_cookie(name: &str, path: &str, secure: bool) -> String {
    let secure_attr = if secure { "; Secure" } else { "" };
    let expires = DateTime::<Utc>::from(std::time::UNIX_EPOCH).to_rfc2822();
    format!(
        "{name}=; Max-Age=0; Expires={expires}; Path={path}; HttpOnly; SameSite=Lax{secure}",
        secure = secure_attr
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_cookies_are_http_only_lax_and_path_scoped() {
        let access = build_auth_cookie(ACCESS_TOKEN_COOKIE, "secret", "/", 900, true);
        assert!(access.contains("HttpOnly"));
        assert!(access.contains("SameSite=Lax"));
        assert!(access.contains("Path=/"));
        assert!(access.contains("Max-Age=900"));
        assert!(access.contains("; Secure"));

        let refresh = build_auth_cookie(REFRESH_TOKEN_COOKIE, "secret", "/graphql", 3600, false);
        assert!(refresh.contains("Path=/graphql"));
        assert!(!refresh.contains("; Secure"));
    }

    #[test]
    fn cleared_auth_cookies_retain_security_attributes() {
        let cookie = build_clear_auth_cookie(REFRESH_TOKEN_COOKIE, "/graphql", true);
        assert!(cookie.contains("Max-Age=0"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Path=/graphql"));
        assert!(cookie.contains("; Secure"));
    }

    #[test]
    fn login_payload_does_not_mask_rate_limits_as_bad_passwords() {
        assert_eq!(
            public_login_error("authentication flow is temporarily locked"),
            crate::services::login_rate_limit::RATE_LIMITED_MESSAGE
        );
        assert_eq!(
            public_login_error("invalid credentials"),
            "Invalid username/email or password"
        );
    }
}

fn clear_auth_cookies(ctx: &Context<'_>, secure: bool) {
    clear_auth_cookie(ctx, ACCESS_TOKEN_COOKIE, "/", secure);
    clear_auth_cookie(ctx, REFRESH_TOKEN_COOKIE, "/graphql", secure);
}

fn public_login_error(detail: &str) -> &'static str {
    if crate::services::login_rate_limit::is_rate_limit_message(detail) {
        crate::services::login_rate_limit::RATE_LIMITED_MESSAGE
    } else {
        "Invalid username/email or password"
    }
}

/// GraphQL input for user registration.
#[derive(Debug, Clone, InputObject)]
#[graphql(name = "RegisterUserInput")]
pub struct RegisterUserInput {
    #[graphql(name = "email")]
    pub email: String,
    #[graphql(name = "name")]
    pub name: String,
    #[graphql(name = "password")]
    pub password: String,
    /// Invite token value. Required to register once at least one user already exists;
    /// omitted (or ignored) for the very first registration, which self-bootstraps the
    /// initial admin account.
    #[graphql(name = "inviteToken")]
    pub invite_token: Option<String>,
}

/// GraphQL input for login (username or email + password).
#[derive(Debug, Clone, InputObject)]
#[graphql(name = "LoginInput")]
pub struct LoginInput {
    #[graphql(name = "usernameOrEmail")]
    pub username_or_email: String,
    #[graphql(name = "password")]
    pub password: String,
}

/// Non-secret token lifetime metadata returned to the browser. Access and refresh token
/// values exist only in server-set HttpOnly cookies.
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(name = "AuthSessionInfo")]
pub struct AuthSessionInfo {
    #[graphql(name = "expiresIn")]
    pub expires_in: i64,
    #[graphql(name = "tokenType")]
    pub token_type: String,
}

impl From<&AuthTokens> for AuthSessionInfo {
    fn from(tokens: &AuthTokens) -> Self {
        Self {
            expires_in: tokens.expires_in,
            token_type: tokens.token_type.clone(),
        }
    }
}

/// Shared auth result: success flag, optional error, and optional user/tokens.
#[derive(Debug, Clone)]
pub struct AuthPayload {
    pub success: bool,
    pub error: Option<String>,
    pub user: Option<AuthenticatedUser>,
    pub tokens: Option<AuthSessionInfo>,
}

#[async_graphql::Object]
impl AuthPayload {
    #[graphql(name = "success")]
    async fn success(&self) -> bool {
        self.success
    }

    #[graphql(name = "error")]
    async fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    #[graphql(name = "user")]
    async fn user(&self) -> Option<&AuthenticatedUser> {
        self.user.as_ref()
    }

    #[graphql(name = "tokens")]
    async fn tokens(&self) -> Option<&AuthSessionInfo> {
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
    #[graphql(name = "success")]
    async fn success(&self) -> bool {
        self.success
    }

    #[graphql(name = "error")]
    async fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

#[Object]
impl AuthMutations {
    #[graphql(name = "register")]
    async fn register(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: RegisterUserInput,
    ) -> Result<AuthPayload> {
        let auth = ctx
            .data::<Arc<AuthService>>()
            .map_err(|_| service_unavailable("Authentication"))?;
        let secure_cookie = ctx
            .data_opt::<AuthCookieContext>()
            .map(|c| c.secure)
            .unwrap_or(false);
        let inner = RegisterInput {
            email: input.email,
            name: input.name,
            password: input.password,
            invite_token: input.invite_token,
        };
        match auth.register(inner).await {
            Ok(login_result) => {
                append_auth_cookies(
                    ctx,
                    &login_result.tokens,
                    auth.access_token_lifetime_seconds(),
                    auth.refresh_token_lifetime_seconds(),
                    secure_cookie,
                );
                let session_info = AuthSessionInfo::from(&login_result.tokens);
                Ok(AuthPayload {
                    success: true,
                    error: None,
                    user: Some(login_result.user),
                    tokens: Some(session_info),
                })
            }
            Err(e) => {
                let detail = e.to_string();
                let public = if detail.contains("Registration requires a valid invite token") {
                    "Registration requires a valid invite token".to_string()
                } else if detail.to_ascii_lowercase().contains("already") {
                    "An account with those details already exists".to_string()
                } else if detail.to_ascii_lowercase().contains("password") {
                    "The registration request does not meet password requirements".to_string()
                } else {
                    internal_message("auth.register", &e)
                };
                Ok(AuthPayload {
                    success: false,
                    error: Some(public),
                    user: None,
                    tokens: None,
                })
            }
        }
    }

    #[graphql(name = "login")]
    async fn login(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: LoginInput,
    ) -> Result<AuthPayload> {
        let auth = ctx
            .data::<Arc<AuthService>>()
            .map_err(|_| service_unavailable("Authentication"))?;
        let secure_cookie = ctx
            .data_opt::<AuthCookieContext>()
            .map(|c| c.secure)
            .unwrap_or(false);
        match auth.login(&input.username_or_email, &input.password).await {
            Ok(login_result) => {
                append_auth_cookies(
                    ctx,
                    &login_result.tokens,
                    auth.access_token_lifetime_seconds(),
                    auth.refresh_token_lifetime_seconds(),
                    secure_cookie,
                );
                let session_info = AuthSessionInfo::from(&login_result.tokens);
                Ok(AuthPayload {
                    success: true,
                    error: None,
                    user: Some(login_result.user),
                    tokens: Some(session_info),
                })
            }
            Err(e) => {
                let detail = e.to_string();
                if crate::services::login_rate_limit::is_rate_limit_message(&detail) {
                    tracing::warn!(
                        principal = %input.username_or_email.trim(),
                        reason = %detail,
                        "Login rejected by rate limit"
                    );
                } else {
                    tracing::debug!(
                        principal = %input.username_or_email.trim(),
                        reason = %detail,
                        "Login rejected"
                    );
                }
                Ok(AuthPayload {
                    success: false,
                    error: Some(public_login_error(&detail).to_string()),
                    user: None,
                    tokens: None,
                })
            }
        }
    }

    #[graphql(name = "refreshToken")]
    async fn refresh_token(&self, ctx: &Context<'_>) -> Result<AuthPayload> {
        let auth = ctx
            .data::<Arc<AuthService>>()
            .map_err(|_| service_unavailable("Authentication"))?;
        let secure_cookie = ctx
            .data_opt::<AuthCookieContext>()
            .map(|c| c.secure)
            .unwrap_or(false);
        let refresh_token = match read_refresh_token(ctx) {
            Some(token) => token,
            None => {
                clear_auth_cookies(ctx, secure_cookie);
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
                append_auth_cookies(
                    ctx,
                    &tokens,
                    auth.access_token_lifetime_seconds(),
                    auth.refresh_token_lifetime_seconds(),
                    secure_cookie,
                );
                let user = auth.validate_access_token(&tokens.access_token).await.ok();
                let session_info = AuthSessionInfo::from(&tokens);
                Ok(AuthPayload {
                    success: true,
                    error: None,
                    user,
                    tokens: Some(session_info),
                })
            }
            Err(err) if !is_invalid_refresh_session(&err) => {
                // A database outage is not evidence that the session is invalid.
                // Leave the cookies intact so the browser can retry renewal.
                Err(service_unavailable("Authentication"))
            }
            Err(_) => {
                clear_auth_cookies(ctx, secure_cookie);
                Ok(AuthPayload {
                    success: false,
                    error: Some("Session refresh failed; sign in again".to_string()),
                    user: None,
                    tokens: None,
                })
            }
        }
    }

    #[graphql(name = "logout")]
    async fn logout(&self, ctx: &Context<'_>) -> Result<LogoutPayload> {
        let auth = ctx
            .data::<Arc<AuthService>>()
            .map_err(|_| service_unavailable("Authentication"))?;
        let secure_cookie = ctx
            .data_opt::<AuthCookieContext>()
            .map(|c| c.secure)
            .unwrap_or(false);
        let refresh_token = read_refresh_token(ctx);

        clear_auth_cookies(ctx, secure_cookie);

        if let Some(token) = refresh_token {
            match auth.logout(&token).await {
                Ok(()) => Ok(LogoutPayload {
                    success: true,
                    error: None,
                }),
                Err(e) => Ok(LogoutPayload {
                    success: false,
                    error: Some(internal_message("auth.logout", &e)),
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

fn is_invalid_refresh_session(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<agql_auth::AuthError>(),
        Some(
            agql_auth::AuthError::InvalidRefreshToken
                | agql_auth::AuthError::RefreshTokenExpired
                | agql_auth::AuthError::RefreshTokenReplayDetected
                | agql_auth::AuthError::UserDisabled
        )
    )
}
