//! GraphQL authentication and authorization
//!
//! Provides JWT token verification and user context for GraphQL operations.
//!
//! ## Guards
//!
//! Use `AuthGuard` to require authentication on any GraphQL operation:
//!
//! ```ignore
//! #[graphql(guard = "AuthGuard")]
//! async fn protected_query(&self, ctx: &Context<'_>) -> Result<String> { ... }
//! ```
//!
//! Use `RoleGuard` to require a specific role:
//!
//! ```ignore
//! #[graphql(guard = "RoleGuard::new(\"admin\")")]
//! async fn admin_only(&self, ctx: &Context<'_>) -> Result<String> { ... }
//! ```

use async_graphql::{Context, ErrorExtensions, Result};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};

/// User context extracted from JWT, available in GraphQL resolvers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub user_id: String,
    pub email: Option<String>,
    pub role: Option<String>,
}

/// Authentication token passed via WebSocket or HTTP header (for future use)
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct AuthToken(pub Option<String>);

#[allow(dead_code)]
impl AuthToken {
    pub fn new(token: Option<String>) -> Self {
        Self(token)
    }
}

/// Claims structure for new custom auth tokens
#[derive(Debug, Deserialize)]
struct AccessTokenClaims {
    sub: String,
    username: String,
    role: String,
    email: Option<String>,
    token_type: String,
    #[allow(dead_code)]
    exp: i64,
    #[allow(dead_code)]
    iat: i64,
}

/// Verify a JWT token and extract user info
pub fn verify_token(token: &str) -> Result<AuthUser> {
    let jwt_secret = std::env::var("JWT_SECRET")
        .map_err(|_| async_graphql::Error::new("JWT_SECRET not configured"))?;

    // Trim any whitespace/newlines from the secret
    let jwt_secret = jwt_secret.trim();

    tracing::debug!("JWT_SECRET length: {}", jwt_secret.len());
    tracing::debug!("Token length: {}", token.len());

    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.validate_aud = false;

    let token_data = decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| {
        tracing::error!("JWT verification failed: {}", e);
        async_graphql::Error::new(format!("Invalid token: {}", e))
            .extend_with(|_, e| e.set("code", "UNAUTHORIZED"))
    })?;

    tracing::debug!("JWT verified for user: {:?}", token_data.claims.email);

    Ok(AuthUser {
        user_id: token_data.claims.sub,
        email: token_data.claims.email,
        role: Some(token_data.claims.role),
    })
}

/// Extension trait to get authenticated user from GraphQL context
pub trait AuthExt {
    /// Get the authenticated user, or return an error if not authenticated
    fn auth_user(&self) -> Result<&AuthUser>;

    /// Get the authenticated user if present, or None (for future use)
    #[allow(dead_code)]
    fn try_auth_user(&self) -> Option<&AuthUser>;
}

impl<'a> AuthExt for Context<'a> {
    fn auth_user(&self) -> Result<&AuthUser> {
        self.data_opt::<AuthUser>().ok_or_else(|| {
            async_graphql::Error::new("Authentication required")
                .extend_with(|_, e| e.set("code", "UNAUTHORIZED"))
        })
    }

    fn try_auth_user(&self) -> Option<&AuthUser> {
        self.data_opt::<AuthUser>()
    }
}

/// Guard that requires authentication for GraphQL operations.
///
/// Use with `#[graphql(guard = "AuthGuard")]` on queries, mutations, or subscriptions.
///
/// # Example
/// ```ignore
/// #[graphql(guard = "AuthGuard")]
/// async fn my_protected_query(&self, ctx: &Context<'_>) -> Result<String> {
///     // Only authenticated users can reach here
///     Ok("secret data".to_string())
/// }
/// ```
pub struct AuthGuard;

impl async_graphql::Guard for AuthGuard {
    fn check(&self, ctx: &Context<'_>) -> impl std::future::Future<Output = Result<()>> + Send {
        let result = ctx.auth_user().map(|_| ());
        async move { result }
    }
}

/// Guard that requires a specific role for GraphQL operations.
///
/// Use with `#[graphql(guard = "RoleGuard::new(\"admin\")")]` on queries, mutations, or subscriptions.
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
        let result = ctx.auth_user().and_then(|user| match &user.role {
            Some(r) if r == &self.role => Ok(()),
            _ => Err(
                async_graphql::Error::new(format!("Role '{}' required", self.role))
                    .extend_with(|_, e| e.set("code", "FORBIDDEN")),
            ),
        });
        async move { result }
    }
}
