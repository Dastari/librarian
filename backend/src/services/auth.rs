//! Authentication service for user management and JWT handling
//!
//! Provides:
//! - User registration and login
//! - Password hashing with bcrypt
//! - JWT token generation and validation
//! - Refresh token management
//! - Library access control

use anyhow::{anyhow, Result};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::{
    CreateUser, Database, UpdateUser, UserRecord, UsersRepository,
};
use crate::db::sqlite_helpers::now_iso8601;

// ============================================================================
// JWT Claims
// ============================================================================

/// Claims structure for access tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// User ID (subject)
    pub sub: String,
    /// Username
    pub username: String,
    /// User role (admin, member, guest)
    pub role: String,
    /// Email (optional)
    pub email: Option<String>,
    /// Token type
    pub token_type: String,
    /// Expiration timestamp
    pub exp: i64,
    /// Issued at timestamp
    pub iat: i64,
}

/// Claims structure for refresh tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    /// User ID (subject)
    pub sub: String,
    /// Token type
    pub token_type: String,
    /// Unique token ID (for revocation)
    pub jti: String,
    /// Expiration timestamp
    pub exp: i64,
    /// Issued at timestamp
    pub iat: i64,
}

// ============================================================================
// Auth Types
// ============================================================================

/// Token pair returned after successful authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokens {
    /// Short-lived access token
    pub access_token: String,
    /// Long-lived refresh token
    pub refresh_token: String,
    /// Access token expiration in seconds
    pub expires_in: i64,
    /// Token type (always "Bearer")
    pub token_type: String,
}

/// User info returned after successful authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub role: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Registration input
#[derive(Debug, Clone)]
pub struct RegisterInput {
    pub email: String,
    pub name: String,
    pub password: String,
}

/// Login result
#[derive(Debug, Clone)]
pub struct LoginResult {
    pub user: AuthenticatedUser,
    pub tokens: AuthTokens,
}

// ============================================================================
// Configuration
// ============================================================================

/// Auth service configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// JWT signing secret
    pub jwt_secret: String,
    /// Access token lifetime in seconds (default: 15 minutes)
    pub access_token_lifetime: i64,
    /// Refresh token lifetime in seconds (default: 7 days)
    pub refresh_token_lifetime: i64,
    /// Bcrypt cost factor (default: 12)
    pub bcrypt_cost: u32,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            access_token_lifetime: 15 * 60,        // 15 minutes
            refresh_token_lifetime: 7 * 24 * 60 * 60, // 7 days
            bcrypt_cost: DEFAULT_COST,
        }
    }
}

impl AuthConfig {
    pub fn from_env() -> Self {
        Self {
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            access_token_lifetime: std::env::var("ACCESS_TOKEN_LIFETIME")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(15 * 60),
            refresh_token_lifetime: std::env::var("REFRESH_TOKEN_LIFETIME")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(7 * 24 * 60 * 60),
            bcrypt_cost: std::env::var("BCRYPT_COST")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_COST),
        }
    }
}

// ============================================================================
// Auth Service
// ============================================================================

/// Authentication service
#[derive(Clone)]
pub struct AuthService {
    db: Database,
    config: AuthConfig,
}

impl AuthService {
    /// Create a new auth service
    pub fn new(db: Database, config: AuthConfig) -> Self {
        Self { db, config }
    }

    /// Create with default config from environment
    pub fn with_env(db: Database) -> Self {
        Self::new(db, AuthConfig::from_env())
    }

    // ========================================================================
    // User Registration
    // ========================================================================

    /// Register a new user
    pub async fn register(&self, input: RegisterInput) -> Result<LoginResult> {
        let users = self.db.users();

        // Check if email already exists
        if users.get_by_email(&input.email).await?.is_some() {
            return Err(anyhow!("Email already registered"));
        }

        // Use email as username (for uniqueness) but display name as the shown name
        let username = input.email.clone();

        // Check if username already exists (email-based)
        if users.get_by_username(&username).await?.is_some() {
            return Err(anyhow!("Email already registered"));
        }

        // Determine role - first user becomes admin
        let role = if users.has_admin().await? {
            "member".to_string()
        } else {
            tracing::info!("Creating first admin user: {}", input.email);
            "admin".to_string()
        };

        // Hash password
        let password_hash = self.hash_password(&input.password)?;

        // Create user
        let user = users.create(CreateUser {
            username,
            email: Some(input.email),
            password_hash,
            role,
            display_name: Some(input.name),
        }).await?;

        // Generate tokens
        let tokens = self.generate_tokens(&user)?;

        // Update last login
        users.update_last_login(&user.id).await?;

        Ok(LoginResult {
            user: self.user_to_authenticated(&user),
            tokens,
        })
    }

    /// Register a user with a specific role (admin only)
    pub async fn register_with_role(&self, input: RegisterInput, role: &str) -> Result<LoginResult> {
        let users = self.db.users();

        // Validate role
        if !["admin", "member", "guest"].contains(&role) {
            return Err(anyhow!("Invalid role: {}", role));
        }

        // Check if email already exists
        if users.get_by_email(&input.email).await?.is_some() {
            return Err(anyhow!("Email already registered"));
        }

        // Use email as username
        let username = input.email.clone();

        // Hash password
        let password_hash = self.hash_password(&input.password)?;

        // Create user
        let user = users.create(CreateUser {
            username,
            email: Some(input.email),
            password_hash,
            role: role.to_string(),
            display_name: Some(input.name),
        }).await?;

        // Generate tokens
        let tokens = self.generate_tokens(&user)?;

        Ok(LoginResult {
            user: self.user_to_authenticated(&user),
            tokens,
        })
    }

    // ========================================================================
    // Login
    // ========================================================================

    /// Login with username/email and password
    pub async fn login(&self, username_or_email: &str, password: &str) -> Result<LoginResult> {
        let users = self.db.users();

        // Try to find user by username or email
        let user = users.get_by_username(username_or_email).await?
            .or(users.get_by_email(username_or_email).await?);

        let user = match user {
            Some(u) => u,
            None => return Err(anyhow!("Invalid username or password")),
        };

        // Check if user is active
        if !user.is_active {
            return Err(anyhow!("Account is disabled"));
        }

        // Verify password
        if !self.verify_password(password, &user.password_hash)? {
            return Err(anyhow!("Invalid username or password"));
        }

        // Generate tokens
        let tokens = self.generate_tokens(&user)?;

        // Update last login
        users.update_last_login(&user.id).await?;

        Ok(LoginResult {
            user: self.user_to_authenticated(&user),
            tokens,
        })
    }

    // ========================================================================
    // Token Management
    // ========================================================================

    /// Refresh access token using refresh token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<AuthTokens> {
        // Decode and validate refresh token
        let claims = self.decode_refresh_token(refresh_token)?;

        // Hash the token to look up in database
        let token_hash = self.hash_token(refresh_token);

        let users = self.db.users();

        // Verify token exists in database and is not expired
        let stored_token = users.get_refresh_token_by_hash(&token_hash).await?
            .ok_or_else(|| anyhow!("Invalid refresh token"))?;

        // Get user
        let user = users.get_by_id(&claims.sub).await?
            .ok_or_else(|| anyhow!("User not found"))?;

        // Check if user is still active
        if !user.is_active {
            return Err(anyhow!("Account is disabled"));
        }

        // Update token last used timestamp
        users.update_refresh_token_used(&stored_token.id).await?;

        // Generate new tokens (token rotation for security)
        let new_tokens = self.generate_tokens(&user)?;

        // Delete old refresh token
        users.delete_refresh_token(&stored_token.id).await?;

        Ok(new_tokens)
    }

    /// Validate access token and return user info
    pub fn validate_access_token(&self, token: &str) -> Result<AuthenticatedUser> {
        let claims = self.decode_access_token(token)?;

        Ok(AuthenticatedUser {
            id: claims.sub,
            username: claims.username,
            email: claims.email,
            role: claims.role,
            display_name: None,
            avatar_url: None,
        })
    }

    /// Logout - invalidate refresh token
    pub async fn logout(&self, refresh_token: &str) -> Result<()> {
        let token_hash = self.hash_token(refresh_token);
        let users = self.db.users();

        if let Some(stored_token) = users.get_refresh_token_by_hash(&token_hash).await? {
            users.delete_refresh_token(&stored_token.id).await?;
        }

        Ok(())
    }

    /// Logout all sessions for a user
    pub async fn logout_all(&self, user_id: &str) -> Result<u64> {
        self.db.users().delete_user_refresh_tokens(user_id).await
    }

    // ========================================================================
    // Password Management
    // ========================================================================

    /// Change user password
    pub async fn change_password(
        &self,
        user_id: &str,
        current_password: &str,
        new_password: &str,
    ) -> Result<()> {
        let users = self.db.users();

        let user = users.get_by_id(user_id).await?
            .ok_or_else(|| anyhow!("User not found"))?;

        // Verify current password
        if !self.verify_password(current_password, &user.password_hash)? {
            return Err(anyhow!("Current password is incorrect"));
        }

        // Hash new password
        let new_hash = self.hash_password(new_password)?;

        // Update password
        users.update(user_id, UpdateUser {
            password_hash: Some(new_hash),
            ..Default::default()
        }).await?;

        // Invalidate all refresh tokens (force re-login)
        users.delete_user_refresh_tokens(user_id).await?;

        Ok(())
    }

    /// Admin reset password (no current password required)
    pub async fn admin_reset_password(&self, user_id: &str, new_password: &str) -> Result<()> {
        let users = self.db.users();

        let new_hash = self.hash_password(new_password)?;

        users.update(user_id, UpdateUser {
            password_hash: Some(new_hash),
            ..Default::default()
        }).await?;

        // Invalidate all refresh tokens
        users.delete_user_refresh_tokens(user_id).await?;

        Ok(())
    }

    // ========================================================================
    // Library Access
    // ========================================================================

    /// Check if user has access to a library
    pub async fn check_library_access(&self, user_id: &str, library_id: &str) -> Result<bool> {
        self.db.users().has_library_access(user_id, library_id).await
    }

    /// Grant library access to a user
    pub async fn grant_library_access(
        &self,
        user_id: &str,
        library_id: &str,
        access_level: &str,
        granted_by: &str,
    ) -> Result<()> {
        self.db.users().grant_library_access(user_id, library_id, access_level, Some(granted_by)).await?;
        Ok(())
    }

    /// Revoke library access
    pub async fn revoke_library_access(&self, user_id: &str, library_id: &str) -> Result<bool> {
        self.db.users().revoke_library_access(user_id, library_id).await
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Hash a password with bcrypt
    fn hash_password(&self, password: &str) -> Result<String> {
        hash(password, self.config.bcrypt_cost)
            .map_err(|e| anyhow!("Failed to hash password: {}", e))
    }

    /// Verify a password against a hash
    fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        verify(password, hash)
            .map_err(|e| anyhow!("Failed to verify password: {}", e))
    }

    /// Hash a token for storage (using SHA-256)
    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Generate access and refresh tokens for a user
    fn generate_tokens(&self, user: &UserRecord) -> Result<AuthTokens> {
        let now = Utc::now();
        let access_exp = now + Duration::seconds(self.config.access_token_lifetime);
        let refresh_exp = now + Duration::seconds(self.config.refresh_token_lifetime);
        let jti = Uuid::new_v4().to_string();

        // Create access token
        let access_claims = AccessTokenClaims {
            sub: user.id.clone(),
            username: user.username.clone(),
            role: user.role.clone(),
            email: user.email.clone(),
            token_type: "access".to_string(),
            exp: access_exp.timestamp(),
            iat: now.timestamp(),
        };

        let access_token = encode(
            &Header::new(Algorithm::HS256),
            &access_claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        ).map_err(|e| anyhow!("Failed to create access token: {}", e))?;

        // Create refresh token
        let refresh_claims = RefreshTokenClaims {
            sub: user.id.clone(),
            token_type: "refresh".to_string(),
            jti: jti.clone(),
            exp: refresh_exp.timestamp(),
            iat: now.timestamp(),
        };

        let refresh_token = encode(
            &Header::new(Algorithm::HS256),
            &refresh_claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        ).map_err(|e| anyhow!("Failed to create refresh token: {}", e))?;

        // Store refresh token hash in database
        let token_hash = self.hash_token(&refresh_token);
        let expires_at = refresh_exp.to_rfc3339();

        // Note: This is synchronous in the token generation, but we need to store it
        // We'll handle this by having the caller store it, or make this async
        // For now, we'll spawn a task to store it
        let users = self.db.users();
        let user_id = user.id.clone();
        tokio::spawn(async move {
            if let Err(e) = users.create_refresh_token(
                &user_id,
                &token_hash,
                &expires_at,
                None,
                None,
            ).await {
                tracing::error!("Failed to store refresh token: {}", e);
            }
        });

        Ok(AuthTokens {
            access_token,
            refresh_token,
            expires_in: self.config.access_token_lifetime,
            token_type: "Bearer".to_string(),
        })
    }

    /// Decode and validate access token
    fn decode_access_token(&self, token: &str) -> Result<AccessTokenClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let token_data = decode::<AccessTokenClaims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &validation,
        ).map_err(|e| anyhow!("Invalid access token: {}", e))?;

        if token_data.claims.token_type != "access" {
            return Err(anyhow!("Invalid token type"));
        }

        Ok(token_data.claims)
    }

    /// Decode and validate refresh token
    fn decode_refresh_token(&self, token: &str) -> Result<RefreshTokenClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let token_data = decode::<RefreshTokenClaims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &validation,
        ).map_err(|e| anyhow!("Invalid refresh token: {}", e))?;

        if token_data.claims.token_type != "refresh" {
            return Err(anyhow!("Invalid token type"));
        }

        Ok(token_data.claims)
    }

    /// Convert UserRecord to AuthenticatedUser
    fn user_to_authenticated(&self, user: &UserRecord) -> AuthenticatedUser {
        AuthenticatedUser {
            id: user.id.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            role: user.role.clone(),
            display_name: user.display_name.clone(),
            avatar_url: user.avatar_url.clone(),
        }
    }

    // ========================================================================
    // Setup and Maintenance
    // ========================================================================

    /// Check if initial setup is required (no admin exists)
    pub async fn needs_setup(&self) -> Result<bool> {
        Ok(!self.db.users().has_admin().await?)
    }

    /// Clean up expired refresh tokens
    pub async fn cleanup_expired_tokens(&self) -> Result<u64> {
        self.db.users().cleanup_expired_refresh_tokens().await
    }
}

// ============================================================================
// Compatibility with existing auth.rs
// ============================================================================

/// Verify a token (compatibility function for existing code)
/// This wraps the new AuthService for backward compatibility
pub fn verify_token(token: &str) -> async_graphql::Result<crate::graphql::auth::AuthUser> {
    let jwt_secret = std::env::var("JWT_SECRET")
        .map_err(|_| async_graphql::Error::new("JWT_SECRET not configured"))?;

    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.validate_aud = false;

    let token_data = decode::<AccessTokenClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| async_graphql::Error::new(format!("Invalid token: {}", e)))?;

    Ok(crate::graphql::auth::AuthUser {
        user_id: token_data.claims.sub,
        email: token_data.claims.email,
        role: Some(token_data.claims.role),
    })
}
