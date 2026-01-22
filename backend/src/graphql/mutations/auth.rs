//! GraphQL authentication mutations
//!
//! Provides mutations for user registration, login, token management, and logout.
//! Most auth mutations do not require authentication (register, login, refreshToken, logout),
//! while some operations like logoutAll require the user to be authenticated.

use async_graphql::{Context, InputObject, Object, Result, SimpleObject};

use crate::db::Database;
use crate::graphql::auth::AuthExt;
use crate::services::{AuthService, RegisterInput};

// ============================================================================
// Input Types
// ============================================================================

/// Input for user registration
#[derive(Debug, InputObject)]
pub struct RegisterUserInput {
    /// Email address (primary login identifier)
    pub email: String,
    /// Full name / display name
    pub name: String,
    /// Password (will be hashed)
    pub password: String,
}

/// Input for user login
#[derive(Debug, InputObject)]
pub struct LoginInput {
    /// Username or email address
    pub username_or_email: String,
    /// Password
    pub password: String,
}

/// Input for token refresh
#[derive(Debug, InputObject)]
pub struct RefreshTokenInput {
    /// The refresh token to exchange for new tokens
    pub refresh_token: String,
}

/// Input for logout
#[derive(Debug, InputObject)]
pub struct LogoutInput {
    /// The refresh token to invalidate
    pub refresh_token: String,
}

// ============================================================================
// Output Types
// ============================================================================

/// Token pair returned after successful authentication
#[derive(Debug, SimpleObject)]
pub struct AuthTokens {
    /// Short-lived access token (JWT)
    pub access_token: String,
    /// Long-lived refresh token (JWT)
    pub refresh_token: String,
    /// Access token expiration in seconds
    pub expires_in: i64,
    /// Token type (always "Bearer")
    pub token_type: String,
}

impl From<crate::services::AuthTokens> for AuthTokens {
    fn from(tokens: crate::services::AuthTokens) -> Self {
        Self {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_in: tokens.expires_in,
            token_type: tokens.token_type,
        }
    }
}

/// User information returned after authentication
#[derive(Debug, SimpleObject)]
pub struct AuthUser {
    /// User ID
    pub id: String,
    /// Username
    pub username: String,
    /// Email address (optional)
    pub email: Option<String>,
    /// User role (admin, member, guest)
    pub role: String,
    /// Display name (optional)
    pub display_name: Option<String>,
    /// Avatar URL (optional)
    pub avatar_url: Option<String>,
}

impl From<crate::services::AuthenticatedUser> for AuthUser {
    fn from(user: crate::services::AuthenticatedUser) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            role: user.role,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
        }
    }
}

/// Result of register or login mutation
#[derive(Debug, SimpleObject)]
pub struct AuthResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// The authenticated user (if successful)
    pub user: Option<AuthUser>,
    /// The auth tokens (if successful)
    pub tokens: Option<AuthTokens>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Result of token refresh mutation
#[derive(Debug, SimpleObject)]
pub struct RefreshTokenResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// The new auth tokens (if successful)
    pub tokens: Option<AuthTokens>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Simple success/error result
#[derive(Debug, SimpleObject)]
pub struct AuthMutationResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
}

// ============================================================================
// Mutations
// ============================================================================

#[derive(Default)]
pub struct AuthMutations;

#[Object]
impl AuthMutations {
    /// Register a new user account
    ///
    /// No authentication required. The first registered user becomes an admin.
    async fn register(&self, ctx: &Context<'_>, input: RegisterUserInput) -> Result<AuthResult> {
        let db = ctx.data_unchecked::<Database>();
        let auth_service = AuthService::with_env(db.clone());

        let register_input = RegisterInput {
            email: input.email,
            name: input.name,
            password: input.password,
        };

        match auth_service.register(register_input).await {
            Ok(result) => {
                tracing::info!(
                    user_id = %result.user.id,
                    username = %result.user.username,
                    "User registered successfully"
                );
                Ok(AuthResult {
                    success: true,
                    user: Some(result.user.into()),
                    tokens: Some(result.tokens.into()),
                    error: None,
                })
            }
            Err(e) => {
                tracing::warn!(error = %e, "User registration failed");
                Ok(AuthResult {
                    success: false,
                    user: None,
                    tokens: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Authenticate a user with username/email and password
    ///
    /// No authentication required.
    async fn login(&self, ctx: &Context<'_>, input: LoginInput) -> Result<AuthResult> {
        let db = ctx.data_unchecked::<Database>();
        let auth_service = AuthService::with_env(db.clone());

        match auth_service
            .login(&input.username_or_email, &input.password)
            .await
        {
            Ok(result) => {
                tracing::info!(
                    user_id = %result.user.id,
                    username = %result.user.username,
                    "User logged in successfully"
                );
                Ok(AuthResult {
                    success: true,
                    user: Some(result.user.into()),
                    tokens: Some(result.tokens.into()),
                    error: None,
                })
            }
            Err(e) => {
                tracing::warn!(
                    username_or_email = %input.username_or_email,
                    error = %e,
                    "Login failed"
                );
                Ok(AuthResult {
                    success: false,
                    user: None,
                    tokens: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Exchange a refresh token for new access and refresh tokens
    ///
    /// No authentication required (uses the refresh token for authorization).
    /// The old refresh token is invalidated (token rotation).
    async fn refresh_token(
        &self,
        ctx: &Context<'_>,
        input: RefreshTokenInput,
    ) -> Result<RefreshTokenResult> {
        let db = ctx.data_unchecked::<Database>();
        let auth_service = AuthService::with_env(db.clone());

        match auth_service.refresh_token(&input.refresh_token).await {
            Ok(tokens) => {
                tracing::debug!("Token refreshed successfully");
                Ok(RefreshTokenResult {
                    success: true,
                    tokens: Some(tokens.into()),
                    error: None,
                })
            }
            Err(e) => {
                tracing::warn!(error = %e, "Token refresh failed");
                Ok(RefreshTokenResult {
                    success: false,
                    tokens: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Invalidate a specific refresh token (logout from one device)
    ///
    /// No authentication required (uses the refresh token for authorization).
    async fn logout(&self, ctx: &Context<'_>, input: LogoutInput) -> Result<AuthMutationResult> {
        let db = ctx.data_unchecked::<Database>();
        let auth_service = AuthService::with_env(db.clone());

        match auth_service.logout(&input.refresh_token).await {
            Ok(()) => {
                tracing::debug!("User logged out successfully");
                Ok(AuthMutationResult {
                    success: true,
                    error: None,
                })
            }
            Err(e) => {
                tracing::warn!(error = %e, "Logout failed");
                Ok(AuthMutationResult {
                    success: false,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Invalidate all refresh tokens for the current user (logout from all devices)
    ///
    /// Requires authentication.
    async fn logout_all(&self, ctx: &Context<'_>) -> Result<AuthMutationResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let auth_service = AuthService::with_env(db.clone());

        match auth_service.logout_all(&user.user_id).await {
            Ok(count) => {
                tracing::info!(
                    user_id = %user.user_id,
                    sessions_invalidated = count,
                    "User logged out from all devices"
                );
                Ok(AuthMutationResult {
                    success: true,
                    error: None,
                })
            }
            Err(e) => {
                tracing::warn!(
                    user_id = %user.user_id,
                    error = %e,
                    "Logout all failed"
                );
                Ok(AuthMutationResult {
                    success: false,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Change the current user's password
    ///
    /// Requires authentication. All refresh tokens will be invalidated.
    async fn change_password(
        &self,
        ctx: &Context<'_>,
        current_password: String,
        new_password: String,
    ) -> Result<AuthMutationResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let auth_service = AuthService::with_env(db.clone());

        match auth_service
            .change_password(&user.user_id, &current_password, &new_password)
            .await
        {
            Ok(()) => {
                tracing::info!(
                    user_id = %user.user_id,
                    "Password changed successfully"
                );
                Ok(AuthMutationResult {
                    success: true,
                    error: None,
                })
            }
            Err(e) => {
                tracing::warn!(
                    user_id = %user.user_id,
                    error = %e,
                    "Password change failed"
                );
                Ok(AuthMutationResult {
                    success: false,
                    error: Some(e.to_string()),
                })
            }
        }
    }
}
