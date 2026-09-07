//! Authentication service backed by `agql-auth` and GraphQL ORM entities.

use std::sync::Arc;

use agql_auth::{
    AuthConfig as AgqlAuthConfig, AuthMethod, AuthPayload as AgqlAuthPayload,
    AuthService as AgqlAuthService, AuthUser, ClientMetadata, RefreshTokenRevocationReason,
    RefreshTokenStore, SessionContext, StoredRefreshToken, StoredUser, UserStore,
};
use anyhow::{Result, anyhow};
use async_graphql::SimpleObject;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::db::Database;
use crate::graphql::entities::{
    CreateRefreshTokenInput, CreateUserInput, InviteToken, RefreshToken, UpdateInviteTokenInput,
    UpdateRefreshTokenInput, UpdateUserInput, User,
};
use crate::services::graphql::auth::Role;
use crate::services::manager::{Service, ServiceHealth};

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub access_token_ttl_seconds: i64,
    pub refresh_token_ttl_seconds: i64,
}

/// Minimum acceptable length (in bytes) for a JWT signing secret.
const MIN_JWT_SECRET_LEN: usize = 32;

impl AuthConfig {
    /// Load the auth config from the environment. Fails startup (rather than
    /// falling back to an insecure default) if no secret is configured or the
    /// configured secret is too short to be a meaningful signing key.
    pub fn from_env() -> Result<Self> {
        const HELP: &str = "Set LIBRARIAN_JWT_SECRET to a random value of at least 32 bytes \
            (e.g. run `openssl rand -base64 48` and copy the output) before starting the server. \
            JWT_SECRET is also accepted as a fallback env var name.";

        let jwt_secret = std::env::var("LIBRARIAN_JWT_SECRET")
            .or_else(|_| std::env::var("JWT_SECRET"))
            .ok()
            .filter(|value| !value.trim().is_empty());

        let jwt_secret = match jwt_secret {
            Some(secret) if secret.len() >= MIN_JWT_SECRET_LEN => secret,
            Some(_) => {
                return Err(anyhow!(
                    "LIBRARIAN_JWT_SECRET (or JWT_SECRET) is set but is shorter than {MIN_JWT_SECRET_LEN} \
                     bytes, which is not safe as a JWT signing key. {HELP}"
                ));
            }
            None => {
                return Err(anyhow!(
                    "LIBRARIAN_JWT_SECRET is not set. Refusing to start with an insecure default \
                     signing key. {HELP}"
                ));
            }
        };

        Ok(Self {
            jwt_secret,
            access_token_ttl_seconds: 15 * 60,
            refresh_token_ttl_seconds: 30 * 24 * 60 * 60,
        })
    }

    /// Fixed, non-secret config for tests that don't exercise env-var loading.
    /// Never used outside `#[cfg(test)]` builds.
    #[cfg(test)]
    pub fn for_tests() -> Self {
        Self {
            jwt_secret: "test-only-jwt-signing-secret-do-not-use-in-prod".to_string(),
            access_token_ttl_seconds: 15 * 60,
            refresh_token_ttl_seconds: 30 * 24 * 60 * 60,
        }
    }

    fn to_agql(&self) -> AgqlAuthConfig {
        let mut config = AgqlAuthConfig::new(self.jwt_secret.clone());
        config.issuer = "librarian".to_string();
        config.audience = "librarian-clients".to_string();
        config.access_token_ttl = Duration::seconds(self.access_token_ttl_seconds);
        config.refresh_token_ttl = Duration::seconds(self.refresh_token_ttl_seconds);
        config
    }
}

#[derive(Debug, Clone)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub token_type: String,
}

#[derive(Debug, Clone, SimpleObject)]
pub struct AuthenticatedUser {
    pub id: String,
    pub email: Option<String>,
    pub username: String,
    pub role: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RegisterInput {
    pub email: String,
    pub name: String,
    pub password: String,
    /// Optional invite token value. Required for every registration except the very
    /// first (see [`AuthService::needs_setup`]), which self-bootstraps the initial admin.
    pub invite_token: Option<String>,
}

/// Error message returned (as the `error` field of the register mutation's payload, not
/// a raw GraphQL error) when registration is attempted without a valid invite token.
pub const INVITE_TOKEN_REQUIRED_MESSAGE: &str = "Registration requires a valid invite token";

/// Minimal, DB-independent snapshot of an [`InviteToken`] row, used so the redemption
/// gate below can be unit tested without a `Database`.
#[derive(Debug, Clone)]
pub struct InviteTokenState {
    pub is_active: bool,
    pub expires_at: Option<OffsetDateTime>,
    pub use_count: i32,
    pub max_uses: Option<i32>,
}

/// Pure gate deciding whether a registration attempt may proceed.
///
/// - If no users exist yet (`needs_setup`), registration is always allowed regardless of
///   `token` - the first user self-bootstraps as admin, matching legacy behavior.
/// - Otherwise a token must be supplied (`Some`) and must be `is_active`, not expired as
///   of `now` (an unset `expires_at` never expires), and under its use limit (an unset or
///   non-positive `max_uses` means unlimited uses).
pub fn validate_invite_token(
    needs_setup: bool,
    token: Option<&InviteTokenState>,
    now: OffsetDateTime,
) -> Result<(), String> {
    if needs_setup {
        return Ok(());
    }
    let Some(state) = token else {
        return Err(INVITE_TOKEN_REQUIRED_MESSAGE.to_string());
    };
    if !state.is_active {
        return Err(INVITE_TOKEN_REQUIRED_MESSAGE.to_string());
    }
    if let Some(expires_at) = state.expires_at
        && expires_at <= now
    {
        return Err(INVITE_TOKEN_REQUIRED_MESSAGE.to_string());
    }
    if let Some(max_uses) = state.max_uses
        && max_uses > 0
        && state.use_count >= max_uses
    {
        return Err(INVITE_TOKEN_REQUIRED_MESSAGE.to_string());
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct LoginResult {
    pub user: AuthenticatedUser,
    pub tokens: AuthTokens,
}

#[derive(Clone)]
struct LibrarianAuthStore {
    db: Database,
}

impl LibrarianAuthStore {
    fn new(db: Database) -> Self {
        Self { db }
    }

    async fn users(&self) -> agql_auth::AuthResult<Vec<User>> {
        User::query(self.db.pool())
            .fetch_all()
            .await
            .map_err(|err| agql_auth::AuthError::Store(err.to_string()))
    }

    async fn refresh_tokens(&self) -> agql_auth::AuthResult<Vec<RefreshToken>> {
        RefreshToken::query(self.db.pool())
            .fetch_all()
            .await
            .map_err(|err| agql_auth::AuthError::Store(err.to_string()))
    }
}

fn parse_time(value: &str) -> agql_auth::AuthResult<OffsetDateTime> {
    // Generated ORM timestamps are Unix seconds; older/auth-written fields are
    // RFC3339. Refresh rotation must be able to read both from the same record.
    if let Ok(seconds) = value.parse::<i64>() {
        return OffsetDateTime::from_unix_timestamp(seconds)
            .map_err(|err| agql_auth::AuthError::Store(err.to_string()));
    }
    OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|err| agql_auth::AuthError::Store(err.to_string()))
}

fn format_time(value: OffsetDateTime) -> String {
    value.format(&Rfc3339).unwrap_or_else(|_| value.to_string())
}

fn to_stored_user(user: User) -> Option<StoredUser> {
    if user.password_hash.trim().is_empty() || !user.password_hash.starts_with("$argon2") {
        return None;
    }
    let principal = user.email.clone().unwrap_or_else(|| user.username.clone());
    Some(StoredUser {
        id: user.id,
        principal,
        password_hash: user.password_hash,
        roles: vec![user.role],
        scopes: Vec::new(),
        disabled: !user.is_active,
    })
}

fn to_stored_refresh_token(token: RefreshToken) -> agql_auth::AuthResult<StoredRefreshToken> {
    Ok(StoredRefreshToken {
        id: Uuid::parse_str(&token.id)
            .map_err(|err| agql_auth::AuthError::Store(err.to_string()))?,
        user_id: token.user_id,
        session_id: Uuid::parse_str(&token.session_id)
            .map_err(|err| agql_auth::AuthError::Store(err.to_string()))?,
        session_family_id: Uuid::parse_str(&token.session_family_id)
            .map_err(|err| agql_auth::AuthError::Store(err.to_string()))?,
        scopes: token.scopes,
        session: serde_json::from_str(&token.session).unwrap_or_default(),
        refreshable_metadata: None,
        token_hash: token.token_hash,
        created_at: parse_time(&token.created_at)?,
        expires_at: parse_time(&token.expires_at)?,
        last_used_at: token.last_used_at.as_deref().map(parse_time).transpose()?,
        revoked_at: token.revoked_at.as_deref().map(parse_time).transpose()?,
        replaced_by_token_id: token
            .replaced_by_token_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|err| agql_auth::AuthError::Store(err.to_string()))?,
        user_agent: token.user_agent,
        ip_address: token.ip_address,
    })
}

#[async_trait]
impl UserStore for LibrarianAuthStore {
    async fn find_user_by_principal(
        &self,
        principal: &str,
    ) -> agql_auth::AuthResult<Option<StoredUser>> {
        let principal = principal.trim().to_ascii_lowercase();
        Ok(self.users().await?.into_iter().find_map(|user| {
            let username_matches = user.username.eq_ignore_ascii_case(&principal);
            let email_matches = user
                .email
                .as_deref()
                .map(|email| email.eq_ignore_ascii_case(&principal))
                .unwrap_or(false);
            if username_matches || email_matches {
                to_stored_user(user)
            } else {
                None
            }
        }))
    }

    async fn find_user_by_id(&self, user_id: &str) -> agql_auth::AuthResult<Option<StoredUser>> {
        Ok(User::get(self.db.pool(), &user_id.to_string())
            .await
            .map_err(|err| agql_auth::AuthError::Store(err.to_string()))?
            .and_then(to_stored_user))
    }
}

#[async_trait]
impl RefreshTokenStore for LibrarianAuthStore {
    async fn insert_refresh_token(&self, token: StoredRefreshToken) -> agql_auth::AuthResult<()> {
        RefreshToken::insert(
            &self.db,
            CreateRefreshTokenInput {
                id: token.id.to_string(),
                user_id: token.user_id,
                token_hash: token.token_hash,
                session_id: token.session_id.to_string(),
                session_family_id: token.session_family_id.to_string(),
                scopes: token.scopes,
                session: serde_json::to_string(&token.session).unwrap_or_default(),
                ip_address: token.ip_address,
                user_agent: token.user_agent,
                expires_at: format_time(token.expires_at),
                last_used_at: token.last_used_at.map(format_time),
                revoked_at: token.revoked_at.map(format_time),
                replaced_by_token_id: token.replaced_by_token_id.map(|id| id.to_string()),
                revocation_reason: None,
            },
        )
        .await
        .map(|_| ())
        .map_err(|err| agql_auth::AuthError::Store(err.to_string()))
    }

    async fn find_refresh_token_by_hash(
        &self,
        token_hash: &str,
    ) -> agql_auth::AuthResult<Option<StoredRefreshToken>> {
        self.refresh_tokens()
            .await?
            .into_iter()
            .find(|token| token.token_hash == token_hash)
            .map(to_stored_refresh_token)
            .transpose()
    }

    async fn revoke_refresh_token(
        &self,
        token_id: Uuid,
        revoked_at: OffsetDateTime,
        replaced_by_token_id: Option<Uuid>,
        reason: RefreshTokenRevocationReason,
    ) -> agql_auth::AuthResult<()> {
        RefreshToken::update_by_id(
            &self.db,
            &token_id.to_string(),
            UpdateRefreshTokenInput {
                revoked_at: Some(Some(format_time(revoked_at))),
                replaced_by_token_id: Some(replaced_by_token_id.map(|id| id.to_string())),
                revocation_reason: Some(Some(format!("{reason:?}"))),
                ..Default::default()
            },
        )
        .await
        .map(|_| ())
        .map_err(|err| agql_auth::AuthError::Store(err.to_string()))
    }

    async fn revoke_refresh_token_family(
        &self,
        session_family_id: Uuid,
        revoked_at: OffsetDateTime,
        reason: RefreshTokenRevocationReason,
    ) -> agql_auth::AuthResult<()> {
        for token in self.refresh_tokens().await? {
            if token.session_family_id == session_family_id.to_string()
                && token.revoked_at.is_none()
            {
                self.revoke_refresh_token(
                    Uuid::parse_str(&token.id)
                        .map_err(|err| agql_auth::AuthError::Store(err.to_string()))?,
                    revoked_at,
                    None,
                    reason.clone(),
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn touch_refresh_token(
        &self,
        token_id: Uuid,
        used_at: OffsetDateTime,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> agql_auth::AuthResult<()> {
        RefreshToken::update_by_id(
            &self.db,
            &token_id.to_string(),
            UpdateRefreshTokenInput {
                last_used_at: Some(Some(format_time(used_at))),
                ip_address: Some(ip_address),
                user_agent: Some(user_agent),
                ..Default::default()
            },
        )
        .await
        .map(|_| ())
        .map_err(|err| agql_auth::AuthError::Store(err.to_string()))
    }

    async fn rotate_refresh_token(
        &self,
        current_token_id: Uuid,
        replacement: StoredRefreshToken,
        rotated_at: OffsetDateTime,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> agql_auth::AuthResult<bool> {
        let current = RefreshToken::get(self.db.pool(), &current_token_id.to_string())
            .await
            .map_err(|err| agql_auth::AuthError::Store(err.to_string()))?;
        let Some(current) = current else {
            return Ok(false);
        };
        if current.revoked_at.is_some() {
            return Ok(false);
        }

        let replacement_id = replacement.id.to_string();
        self.insert_refresh_token(replacement).await?;

        RefreshToken::update_by_id(
            &self.db,
            &current_token_id.to_string(),
            UpdateRefreshTokenInput {
                revoked_at: Some(Some(format_time(rotated_at))),
                replaced_by_token_id: Some(Some(replacement_id)),
                revocation_reason: Some(Some(format!(
                    "{:?}",
                    RefreshTokenRevocationReason::Rotation
                ))),
                last_used_at: Some(Some(format_time(rotated_at))),
                ip_address: Some(ip_address),
                user_agent: Some(user_agent),
                ..Default::default()
            },
        )
        .await
        .map(|_| true)
        .map_err(|err| agql_auth::AuthError::Store(err.to_string()))
    }
}

type InnerAuth = AgqlAuthService<LibrarianAuthStore, LibrarianAuthStore>;

pub struct AuthService {
    manager: Arc<crate::services::ServicesManager>,
    config: AuthConfig,
    db: RwLock<Option<Database>>,
    inner: RwLock<Option<Arc<InnerAuth>>>,
    login_limiter: crate::services::login_rate_limit::LoginRateLimiter,
}

impl AuthService {
    pub fn new(manager: Arc<crate::services::ServicesManager>, config: AuthConfig) -> Self {
        Self {
            manager,
            config,
            db: RwLock::new(None),
            inner: RwLock::new(None),
            login_limiter: crate::services::login_rate_limit::LoginRateLimiter::default(),
        }
    }

    async fn inner(&self) -> Result<Arc<InnerAuth>> {
        self.inner
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow!("auth service not started"))
    }

    async fn db(&self) -> Result<Database> {
        self.db
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow!("auth service not started"))
    }

    pub fn refresh_token_lifetime_seconds(&self) -> i64 {
        self.config.refresh_token_ttl_seconds
    }

    pub fn access_token_lifetime_seconds(&self) -> i64 {
        self.config.access_token_ttl_seconds
    }

    pub async fn get_jwt_secret(&self) -> Result<String> {
        Ok(self.config.jwt_secret.clone())
    }

    pub async fn authenticate_access_token(&self, token: &str) -> Result<AuthUser> {
        self.inner()
            .await?
            .authenticate_bearer(token)
            .map_err(|err| anyhow!(err.to_string()))
    }

    pub async fn needs_setup(&self) -> Result<bool> {
        let db = self.db().await?;
        let users = User::query(db.pool()).fetch_all().await?;
        Ok(!users.into_iter().any(|user| {
            user.is_active && user.role == "admin" && user.password_hash.starts_with("$argon2")
        }))
    }

    pub async fn register(&self, input: RegisterInput) -> Result<LoginResult> {
        let db = self.db().await?;
        let inner = self.inner().await?;
        let needs_setup = self.needs_setup().await?;

        // InviteToken's `token` field is `#[graphql_orm(private)]` (not filterable via
        // the generated WhereInput), so - matching the existing lookup pattern for
        // RefreshToken's private `token_hash` field in this file - fetch all invite
        // tokens and match in memory. The table is small (admin-issued invites only).
        let matched_token = if needs_setup {
            None
        } else {
            let raw_token = input
                .invite_token
                .as_deref()
                .unwrap_or("")
                .trim()
                .to_string();
            if raw_token.is_empty() {
                None
            } else {
                InviteToken::query(db.pool())
                    .fetch_all()
                    .await?
                    .into_iter()
                    .find(|candidate| candidate.token == raw_token)
            }
        };

        let now = OffsetDateTime::now_utc();
        let token_state = matched_token.as_ref().map(|token| InviteTokenState {
            is_active: token.is_active,
            expires_at: token
                .expires_at
                .as_deref()
                .and_then(|value| parse_time(value).ok()),
            use_count: token.use_count,
            max_uses: token.max_uses,
        });
        validate_invite_token(needs_setup, token_state.as_ref(), now)
            .map_err(|message| anyhow!(message))?;

        let role = if needs_setup {
            Role::Admin
        } else {
            matched_token
                .as_ref()
                .and_then(|token| Role::parse(&token.role))
                .unwrap_or(Role::Member)
        };

        let password_hash = inner
            .hash_password(&input.password)
            .map_err(|err| anyhow!(err.to_string()))?;
        let username = input
            .email
            .split('@')
            .next()
            .filter(|value| !value.is_empty())
            .unwrap_or("admin")
            .to_string();
        let user = User::insert(
            &db,
            CreateUserInput {
                username,
                email: Some(input.email.clone()),
                password_hash,
                role: role.as_str().to_string(),
                display_name: Some(input.name),
                avatar_url: None,
                is_active: true,
                last_login_at: None,
            },
        )
        .await?;

        if let Some(token) = &matched_token {
            let new_use_count = token.use_count + 1;
            let exhausted = token
                .max_uses
                .map(|max_uses| max_uses > 0 && new_use_count >= max_uses)
                .unwrap_or(false);
            InviteToken::update_by_id(
                &db,
                &token.id,
                UpdateInviteTokenInput {
                    use_count: Some(new_use_count),
                    is_active: if exhausted { Some(false) } else { None },
                    ..UpdateInviteTokenInput::default()
                },
            )
            .await?;
        }

        let payload = inner
            .issue_verified_user_session(
                user.id.clone(),
                vec![user.role.clone()],
                AuthMethod::Password,
                ClientMetadata::default(),
            )
            .await
            .map_err(|err| anyhow!(err.to_string()))?;
        Ok(to_login_result(user, payload))
    }

    pub async fn login(&self, principal: &str, password: &str) -> Result<LoginResult> {
        if self.login_limiter.is_blocked(principal) {
            return Err(anyhow!(
                crate::services::login_rate_limit::RATE_LIMITED_MESSAGE
            ));
        }

        let login_result = self
            .inner()
            .await?
            .login(principal, password, ClientMetadata::default())
            .await;
        let payload = match login_result {
            Ok(payload) => {
                self.login_limiter.clear(principal);
                payload
            }
            Err(err) => {
                let message = err.to_string();
                // Throttle/lockout is already a denial; counting it as another
                // failed password guess extends the lockout.
                if !crate::services::login_rate_limit::is_rate_limit_message(&message) {
                    self.login_limiter.record_failure(principal);
                }
                return Err(anyhow!(message));
            }
        };
        let db = self.db().await?;
        let user = User::get(db.pool(), &payload.user.user_id)
            .await?
            .ok_or_else(|| anyhow!("authenticated user not found"))?;
        let _ = User::update_by_id(
            &db,
            &user.id,
            UpdateUserInput {
                last_login_at: Some(Some(format_time(OffsetDateTime::now_utc()))),
                ..Default::default()
            },
        )
        .await;
        Ok(to_login_result(user, payload))
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> Result<AuthTokens> {
        let payload = self
            .inner()
            .await?
            .refresh(refresh_token, ClientMetadata::default())
            .await
            .map_err(anyhow::Error::new)?;
        Ok(to_tokens(payload))
    }

    pub async fn validate_access_token(&self, token: &str) -> Result<AuthenticatedUser> {
        let auth_user = self.authenticate_access_token(token).await?;
        let db = self.db().await?;
        let user = User::get(db.pool(), &auth_user.user_id)
            .await?
            .ok_or_else(|| anyhow!("authenticated user not found"))?;
        Ok(to_authenticated_user(user))
    }

    pub async fn logout(&self, refresh_token: &str) -> Result<()> {
        self.inner()
            .await?
            .logout(refresh_token, false)
            .await
            .map_err(|err| anyhow!(err.to_string()))
    }
}

fn to_authenticated_user(user: User) -> AuthenticatedUser {
    AuthenticatedUser {
        id: user.id,
        email: user.email,
        username: user.username,
        role: user.role,
        display_name: user.display_name,
    }
}

fn to_tokens(payload: AgqlAuthPayload) -> AuthTokens {
    let expires_in = (payload.access_token_expires_at - OffsetDateTime::now_utc()).whole_seconds();
    AuthTokens {
        access_token: payload.access_token,
        refresh_token: payload.refresh_token,
        expires_in,
        token_type: "Bearer".to_string(),
    }
}

fn to_login_result(user: User, payload: AgqlAuthPayload) -> LoginResult {
    LoginResult {
        user: to_authenticated_user(user),
        tokens: to_tokens(payload),
    }
}

#[async_trait]
impl Service for AuthService {
    fn name(&self) -> &str {
        "auth"
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["database".to_string()]
    }

    async fn start(&self) -> Result<()> {
        let db = self
            .manager
            .get_database()
            .await
            .map(|service| service.pool().clone())
            .ok_or_else(|| anyhow!("database service not available"))?;
        let store = Arc::new(LibrarianAuthStore::new(db.clone()));
        let inner = Arc::new(
            InnerAuth::new(self.config.to_agql(), store.clone(), store)
                .map_err(|err| anyhow!(err.to_string()))?,
        );
        *self.db.write().await = Some(db);
        *self.inner.write().await = Some(inner);
        info!(service = "auth", "Auth service started");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        *self.inner.write().await = None;
        *self.db.write().await = None;
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        if self.inner.read().await.is_some() {
            Ok(ServiceHealth::healthy())
        } else {
            Ok(ServiceHealth::unhealthy("auth service not started"))
        }
    }
}

#[cfg(test)]
mod invite_token_tests {
    use super::{INVITE_TOKEN_REQUIRED_MESSAGE, InviteTokenState, validate_invite_token};
    use time::Duration;
    use time::OffsetDateTime;

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    fn valid_state(now: OffsetDateTime) -> InviteTokenState {
        InviteTokenState {
            is_active: true,
            expires_at: Some(now + Duration::days(1)),
            use_count: 0,
            max_uses: Some(5),
        }
    }

    #[test]
    fn first_user_bypasses_invite_requirement() {
        // needs_setup = true, no token at all - still allowed.
        assert!(validate_invite_token(true, None, now()).is_ok());
    }

    #[test]
    fn missing_token_is_required_once_setup_is_complete() {
        let err = validate_invite_token(false, None, now()).unwrap_err();
        assert_eq!(err, INVITE_TOKEN_REQUIRED_MESSAGE);
    }

    #[test]
    fn valid_token_is_accepted() {
        let now = now();
        let state = valid_state(now);
        assert!(validate_invite_token(false, Some(&state), now).is_ok());
    }

    #[test]
    fn expired_token_is_rejected() {
        let now = now();
        let mut state = valid_state(now);
        state.expires_at = Some(now - Duration::seconds(1));
        let err = validate_invite_token(false, Some(&state), now).unwrap_err();
        assert_eq!(err, INVITE_TOKEN_REQUIRED_MESSAGE);
    }

    #[test]
    fn exhausted_token_is_rejected() {
        let now = now();
        let mut state = valid_state(now);
        state.max_uses = Some(3);
        state.use_count = 3;
        let err = validate_invite_token(false, Some(&state), now).unwrap_err();
        assert_eq!(err, INVITE_TOKEN_REQUIRED_MESSAGE);
    }

    #[test]
    fn inactive_token_is_rejected() {
        let now = now();
        let mut state = valid_state(now);
        state.is_active = false;
        let err = validate_invite_token(false, Some(&state), now).unwrap_err();
        assert_eq!(err, INVITE_TOKEN_REQUIRED_MESSAGE);
    }

    #[test]
    fn unlimited_and_no_expiry_token_is_accepted() {
        let now = now();
        let state = InviteTokenState {
            is_active: true,
            expires_at: None,
            use_count: 1000,
            max_uses: None,
        };
        assert!(validate_invite_token(false, Some(&state), now).is_ok());
    }

    #[test]
    fn zero_max_uses_means_unlimited() {
        let now = now();
        let mut state = valid_state(now);
        state.max_uses = Some(0);
        state.use_count = 50;
        assert!(validate_invite_token(false, Some(&state), now).is_ok());
    }
}
