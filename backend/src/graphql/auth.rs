//! GraphQL authentication and authorization
//!
//! Provides JWT token verification and user context for GraphQL operations.

use async_graphql::{Context, ErrorExtensions, Result};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

/// User context extracted from JWT, available in GraphQL resolvers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub user_id: String,
    pub email: Option<String>,
    pub role: Option<String>,
}

/// JWT claims structure for Supabase tokens
#[derive(Debug, Deserialize)]
pub struct SupabaseClaims {
    pub sub: String,
    pub email: Option<String>,
    pub role: Option<String>,
    pub exp: usize,
    pub iat: usize,
}

/// Authentication token passed via WebSocket or HTTP header
#[derive(Debug, Clone, Default)]
pub struct AuthToken(pub Option<String>);

impl AuthToken {
    pub fn new(token: Option<String>) -> Self {
        Self(token)
    }
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
    // Supabase tokens have aud="authenticated", disable audience validation
    // or set it explicitly
    validation.validate_aud = false;

    let token_data = decode::<SupabaseClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| {
        tracing::error!("JWT verification failed: {}", e);
        async_graphql::Error::new(format!("Invalid token: {}", e))
            .extend_with(|_, e| e.set("code", "UNAUTHORIZED"))
    })?;

    tracing::info!("JWT verified for user: {:?}", token_data.claims.email);

    Ok(AuthUser {
        user_id: token_data.claims.sub,
        email: token_data.claims.email,
        role: token_data.claims.role,
    })
}

/// Extension trait to get authenticated user from GraphQL context
pub trait AuthExt {
    /// Get the authenticated user, or return an error if not authenticated
    fn auth_user(&self) -> Result<&AuthUser>;

    /// Get the authenticated user if present, or None
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

/// Guard that requires authentication
pub struct AuthGuard;

impl AuthGuard {
    pub fn check(ctx: &Context<'_>) -> Result<()> {
        ctx.auth_user()?;
        Ok(())
    }
}

/// Guard that requires a specific role
pub struct RoleGuard {
    pub role: String,
}

impl RoleGuard {
    pub fn new(role: impl Into<String>) -> Self {
        Self { role: role.into() }
    }

    pub fn check(&self, ctx: &Context<'_>) -> Result<()> {
        let user = ctx.auth_user()?;
        match &user.role {
            Some(r) if r == &self.role => Ok(()),
            _ => Err(async_graphql::Error::new(format!(
                "Role '{}' required",
                self.role
            ))
            .extend_with(|_, e| e.set("code", "FORBIDDEN"))),
        }
    }
}
