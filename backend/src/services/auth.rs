//! Authentication service for user management and JWT handling.
//!
//! Implements [Service](crate::services::manager::Service) and depends on the database service.
//! Provides user registration, login, JWT/refresh tokens, and library access control.
//! Uses the `User` and `RefreshToken` entities and `db::operations` for persistence.

use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::db::Database;
use crate::db::operations::{
    CreateUserParams, cleanup_expired_refresh_tokens, create_refresh_token, create_user,
    delete_refresh_token, delete_user_refresh_tokens, grant_library_access, has_admin_user,
    has_library_access, revoke_library_access, update_refresh_token_used, update_user_last_login,
    update_user_password,
};
use crate::services::graphql::entities::{
    RefreshToken, RefreshTokenWhereInput, User, UserWhereInput,
};
use crate::services::graphql::orm::StringFilter;
use crate::services::manager::{Service, ServiceHealth};

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
#[derive(Debug, Clone, Serialize, Deserialize, async_graphql::SimpleObject)]
#[graphql(name = "AuthTokens")]
#[serde(rename_all = "PascalCase")]
pub struct AuthTokens {
    /// Short-lived access token
    #[graphql(name = "AccessToken")]
    pub access_token: String,
    /// Long-lived refresh token
    #[graphql(name = "RefreshToken")]
    pub refresh_token: String,
    /// Access token expiration in seconds
    #[graphql(name = "ExpiresIn")]
    pub expires_in: i64,
    /// Token type (always "Bearer")
    #[graphql(name = "TokenType")]
    pub token_type: String,
}

/// User info returned after successful authentication
#[derive(Debug, Clone, Serialize, Deserialize, async_graphql::SimpleObject)]
#[graphql(name = "AuthenticatedUser")]
#[serde(rename_all = "PascalCase")]
pub struct AuthenticatedUser {
    #[graphql(name = "Id")]
    pub id: String,
    #[graphql(name = "Username")]
    pub username: String,
    #[graphql(name = "Email")]
    pub email: Option<String>,
    #[graphql(name = "Role")]
    pub role: String,
    #[graphql(name = "DisplayName")]
    pub display_name: Option<String>,
    #[graphql(name = "AvatarUrl")]
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
            // Not used at runtime; JWT secret is loaded from database in auth service start.
            jwt_secret: String::new(),
            access_token_lifetime: 15 * 60,           // 15 minutes
            refresh_token_lifetime: 7 * 24 * 60 * 60, // 7 days
            bcrypt_cost: DEFAULT_COST,
        }
    }
}

impl AuthConfig {
    pub fn from_env() -> Self {
        Self {
            // Not used at runtime; JWT secret is loaded from database in auth service start.
            jwt_secret: String::new(),
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

/// Key for JWT secret in auth_secrets table. Must match database service constant.
const AUTH_SECRETS_JWT_KEY: &str = "jwt_secret";

/// Authentication service. Depends on the database service; obtains the pool in [start](Service::start).
/// JWT secret is loaded from the database at start and never exposed via GraphQL.
pub struct AuthService {
    manager: Arc<crate::services::ServicesManager>,
    config: AuthConfig,
    db: RwLock<Option<Database>>,
    /// JWT signing secret loaded from auth_secrets table at start. Never exposed.
    jwt_secret: RwLock<Option<String>>,
}

impl AuthService {
    /// Create a new auth service. Register with the manager and start it so [start](Service::start) can obtain the database.
    pub fn new(manager: Arc<crate::services::ServicesManager>, config: AuthConfig) -> Self {
        Self {
            manager,
            config,
            db: RwLock::new(None),
            jwt_secret: RwLock::new(None),
        }
    }

    /// Create with config from environment.
    pub fn with_env(manager: Arc<crate::services::ServicesManager>) -> Self {
        Self::new(manager, AuthConfig::from_env())
    }

    /// Get the database pool. Returns an error if the service has not been started.
    async fn get_pool(&self) -> Result<Database> {
        self.db
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow!("auth service not started"))
    }

    /// Return the JWT signing secret loaded from the database. Used for token creation and verification.
    /// Never expose this value via GraphQL or logs.
    pub async fn get_jwt_secret(&self) -> Result<String> {
        self.jwt_secret
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow!("JWT secret not loaded (auth service not started or missing in database)"))
    }

    // ========================================================================
    // User Registration
    // ========================================================================

    /// Register a new user
    pub async fn register(&self, input: RegisterInput) -> Result<LoginResult> {
        let pool = self.get_pool().await?;

        // Check if email already exists
        if User::query(&pool)
            .filter(UserWhereInput {
                email: Some(StringFilter::eq(&input.email)),
                ..Default::default()
            })
            .fetch_optional()
            .await?
            .is_some()
        {
            return Err(anyhow!("Email already registered"));
        }

        let username = input.email.clone();
        if User::query(&pool)
            .filter(UserWhereInput {
                username: Some(StringFilter::eq(&username)),
                ..Default::default()
            })
            .fetch_optional()
            .await?
            .is_some()
        {
            return Err(anyhow!("Email already registered"));
        }

        let role = if has_admin_user(&pool).await? {
            "member".to_string()
        } else {
            tracing::info!("Creating first admin user: {}", input.email);
            "admin".to_string()
        };

        let password_hash = self.hash_password(&input.password)?;
        let id = Uuid::new_v4().to_string();

        create_user(
            &pool,
            &CreateUserParams {
                id: id.clone(),
                username: username.clone(),
                email: Some(input.email.clone()),
                password_hash,
                role: role.clone(),
                display_name: Some(input.name),
                avatar_url: None,
            },
        )
        .await?;

        let user = User::get(&pool, &id)
            .await?
            .ok_or_else(|| anyhow!("Failed to load created user"))?;

        let tokens = self.generate_tokens(&user).await?;

        update_user_last_login(&pool, &user.id).await?;

        Ok(LoginResult {
            user: self.user_to_authenticated(&user),
            tokens,
        })
    }

    /// Register a user with a specific role (admin only)
    pub async fn register_with_role(
        &self,
        input: RegisterInput,
        role: &str,
    ) -> Result<LoginResult> {
        let pool = self.get_pool().await?;

        if !["admin", "member", "guest"].contains(&role) {
            return Err(anyhow!("Invalid role: {}", role));
        }

        if User::query(&pool)
            .filter(UserWhereInput {
                email: Some(StringFilter::eq(&input.email)),
                ..Default::default()
            })
            .fetch_optional()
            .await?
            .is_some()
        {
            return Err(anyhow!("Email already registered"));
        }

        let username = input.email.clone();
        let password_hash = self.hash_password(&input.password)?;
        let id = Uuid::new_v4().to_string();

        create_user(
            &pool,
            &CreateUserParams {
                id: id.clone(),
                username: username.clone(),
                email: Some(input.email.clone()),
                password_hash,
                role: role.to_string(),
                display_name: Some(input.name),
                avatar_url: None,
            },
        )
        .await?;

        let user = User::get(&pool, &id)
            .await?
            .ok_or_else(|| anyhow!("Failed to load created user"))?;

        let tokens = self.generate_tokens(&user).await?;

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
        let pool = self.get_pool().await?;

        let by_username = User::query(&pool)
            .filter(UserWhereInput {
                username: Some(StringFilter::eq(username_or_email)),
                ..Default::default()
            })
            .fetch_optional()
            .await?;
        let by_email = User::query(&pool)
            .filter(UserWhereInput {
                email: Some(StringFilter::eq(username_or_email)),
                ..Default::default()
            })
            .fetch_optional()
            .await?;

        let user = match by_username.or(by_email) {
            Some(u) => u,
            None => return Err(anyhow!("Invalid username or password")),
        };

        if !user.is_active {
            return Err(anyhow!("Account is disabled"));
        }

        if !self.verify_password(password, &user.password_hash)? {
            return Err(anyhow!("Invalid username or password"));
        }

        let tokens = self.generate_tokens(&user).await?;
        update_user_last_login(&pool, &user.id).await?;

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
        let secret = self.get_jwt_secret().await?;
        let claims = self.decode_refresh_token_with_secret(refresh_token, &secret)?;
        let token_hash = self.hash_token(refresh_token);
        let pool = self.get_pool().await?;

        let stored_token = RefreshToken::query(&pool)
            .filter(RefreshTokenWhereInput {
                token_hash: Some(StringFilter::eq(&token_hash)),
                ..Default::default()
            })
            .fetch_optional()
            .await?
            .ok_or_else(|| anyhow!("Invalid refresh token"))?;

        let user = User::get(&pool, &claims.sub)
            .await?
            .ok_or_else(|| anyhow!("User not found"))?;

        if !user.is_active {
            return Err(anyhow!("Account is disabled"));
        }

        update_refresh_token_used(&pool, &stored_token.id).await?;
        let new_tokens = self.generate_tokens(&user).await?;
        delete_refresh_token(&pool, &stored_token.id).await?;

        Ok(new_tokens)
    }

    /// Validate access token and return user info (from claims only; no DB lookup)
    pub async fn validate_access_token(&self, token: &str) -> Result<AuthenticatedUser> {
        let secret = self.get_jwt_secret().await?;
        let claims = self.decode_access_token_with_secret(token, &secret)?;
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
        let pool = self.get_pool().await?;
        if let Some(stored_token) = RefreshToken::query(&pool)
            .filter(RefreshTokenWhereInput {
                token_hash: Some(StringFilter::eq(&token_hash)),
                ..Default::default()
            })
            .fetch_optional()
            .await?
        {
            delete_refresh_token(&pool, &stored_token.id).await?;
        }
        Ok(())
    }

    /// Logout all sessions for a user
    pub async fn logout_all(&self, user_id: &str) -> Result<u64> {
        let pool = self.get_pool().await?;
        delete_user_refresh_tokens(&pool, user_id).await
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
        let pool = self.get_pool().await?;
        let user = User::get(&pool, user_id)
            .await?
            .ok_or_else(|| anyhow!("User not found"))?;

        if !self.verify_password(current_password, &user.password_hash)? {
            return Err(anyhow!("Current password is incorrect"));
        }

        let new_hash = self.hash_password(new_password)?;
        update_user_password(&pool, user_id, &new_hash).await?;
        delete_user_refresh_tokens(&pool, user_id).await?;
        Ok(())
    }

    /// Admin reset password (no current password required)
    pub async fn admin_reset_password(&self, user_id: &str, new_password: &str) -> Result<()> {
        let pool = self.get_pool().await?;
        let new_hash = self.hash_password(new_password)?;
        update_user_password(&pool, user_id, &new_hash).await?;
        delete_user_refresh_tokens(&pool, user_id).await?;
        Ok(())
    }

    // ========================================================================
    // Library Access
    // ========================================================================

    /// Check if user has access to a library
    pub async fn check_library_access(&self, user_id: &str, library_id: &str) -> Result<bool> {
        let pool = self.get_pool().await?;
        has_library_access(&pool, user_id, library_id).await
    }

    /// Grant library access to a user
    pub async fn grant_library_access(
        &self,
        user_id: &str,
        library_id: &str,
        access_level: &str,
        granted_by: &str,
    ) -> Result<()> {
        let pool = self.get_pool().await?;
        grant_library_access(
            &pool,
            user_id,
            library_id,
            access_level,
            Some(granted_by),
        )
        .await
    }

    /// Revoke library access
    pub async fn revoke_library_access(&self, user_id: &str, library_id: &str) -> Result<bool> {
        let pool = self.get_pool().await?;
        revoke_library_access(&pool, user_id, library_id).await
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
        verify(password, hash).map_err(|e| anyhow!("Failed to verify password: {}", e))
    }

    /// Hash a token for storage (using SHA-256)
    fn hash_token(&self, token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Generate access and refresh tokens for a user
    async fn generate_tokens(&self, user: &User) -> Result<AuthTokens> {
        let secret = self.get_jwt_secret().await?;
        let now = Utc::now();
        let access_exp = now + Duration::seconds(self.config.access_token_lifetime);
        let refresh_exp = now + Duration::seconds(self.config.refresh_token_lifetime);

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
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| anyhow!("Failed to create access token: {}", e))?;

        let jti = Uuid::new_v4().to_string();
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
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| anyhow!("Failed to create refresh token: {}", e))?;

        let token_hash = self.hash_token(&refresh_token);
        let expires_at = refresh_exp.to_rfc3339();
        let token_id = Uuid::new_v4().to_string();

        let pool = self.get_pool().await?;
        create_refresh_token(
            &pool,
            &token_id,
            &user.id,
            &token_hash,
            &expires_at,
            None,
            None,
        )
        .await?;

        Ok(AuthTokens {
            access_token,
            refresh_token,
            expires_in: self.config.access_token_lifetime,
            token_type: "Bearer".to_string(),
        })
    }

    /// Decode and validate access token using the given secret.
    fn decode_access_token_with_secret(&self, token: &str, secret: &str) -> Result<AccessTokenClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let token_data = decode::<AccessTokenClaims>(
            token,
            &DecodingKey::from_secret(secret.trim().as_bytes()),
            &validation,
        )
        .map_err(|e| anyhow!("Invalid access token: {}", e))?;

        if token_data.claims.token_type != "access" {
            return Err(anyhow!("Invalid token type"));
        }

        Ok(token_data.claims)
    }

    /// Decode and validate refresh token using the given secret.
    fn decode_refresh_token_with_secret(&self, token: &str, secret: &str) -> Result<RefreshTokenClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let token_data = decode::<RefreshTokenClaims>(
            token,
            &DecodingKey::from_secret(secret.trim().as_bytes()),
            &validation,
        )
        .map_err(|e| anyhow!("Invalid refresh token: {}", e))?;

        if token_data.claims.token_type != "refresh" {
            return Err(anyhow!("Invalid token type"));
        }

        Ok(token_data.claims)
    }

    /// Convert User entity to AuthenticatedUser
    fn user_to_authenticated(&self, user: &User) -> AuthenticatedUser {
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
        let pool = self.get_pool().await?;
        Ok(!has_admin_user(&pool).await?)
    }

    /// Clean up expired refresh tokens
    pub async fn cleanup_expired_tokens(&self) -> Result<u64> {
        let pool = self.get_pool().await?;
        cleanup_expired_refresh_tokens(&pool).await
    }
}

// ============================================================================
// Service impl
// ============================================================================

#[async_trait]
impl Service for AuthService {
    fn name(&self) -> &str {
        "auth"
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["database".to_string()]
    }

    async fn start(&self) -> Result<()> {
        info!(service = "auth", "Auth service starting");
        let db = self
            .manager
            .get_database()
            .await
            .map(|svc| svc.pool().clone())
            .ok_or_else(|| anyhow!("database service not available"))?;
        *self.db.write().await = Some(db.clone());

        let row: Option<(String,)> =
            sqlx::query_as("SELECT value FROM auth_secrets WHERE key = ?")
                .bind(AUTH_SECRETS_JWT_KEY)
                .fetch_optional(&db)
                .await?;
        let secret = row
            .map(|(v,)| v)
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| anyhow!("JWT secret not found in database (run database service first)"))?;
        *self.jwt_secret.write().await = Some(secret);
        info!(service = "auth", "Auth service started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        *self.jwt_secret.write().await = None;
        *self.db.write().await = None;
        info!(service = "auth", "Stopped");
        Ok(())
    }
}
