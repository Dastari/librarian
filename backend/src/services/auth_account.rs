//! Account security on [`AuthService`]: sign-in in steps, two-factor sign-in, passwords and
//! reset links (design.md Q147–Q148). Secrets live encrypted (TOTP) or hashed (recovery codes,
//! passwords) on the `User` row in private fields; nothing here logs one.

use agql_auth::{AuthMethod, ClientMetadata, TotpOptions};
use anyhow::{Result, anyhow};
use time::{Duration, OffsetDateTime};
use tracing::{info, warn};

use super::{AuthService, LoginResult, format_time, to_login_result};
use crate::graphql::entities::{PasswordResetToken, PasswordResetTokenWhereInput, UpdateUserInput, User};
use crate::services::account_security::{
    self as security, ChallengeStage, PendingSignIn, ResetLinkStore, TOTP_FACTOR, TOTP_ISSUER,
};
use crate::services::login_rate_limit::RATE_LIMITED_MESSAGE;
use crate::services::sources::encryption::CredentialEncryption;

/// What a correct password leads to.
#[derive(Debug, Clone)]
pub enum SignIn {
    /// Signed in: cookies can be set.
    Session(LoginResult),
    /// Another step first. No session exists yet.
    Challenge(SignInChallenge),
}

#[derive(Clone)]
pub struct SignInChallenge {
    /// Opaque, single-purpose: only the completion mutations accept it.
    pub token: String,
    pub stage: ChallengeStage,
    pub expires_in_seconds: i64,
}

impl std::fmt::Debug for SignInChallenge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SignInChallenge")
            .field("token", &"[redacted]")
            .field("stage", &self.stage)
            .finish()
    }
}

/// A secret to scan, shown once while enrolling.
#[derive(Clone)]
pub struct TwoFactorEnrollment {
    pub secret: String,
    pub uri: String,
    pub qr_svg: String,
}

impl std::fmt::Debug for TwoFactorEnrollment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TwoFactorEnrollment([redacted])")
    }
}

/// The message for any wrong or stale second step, so it says nothing about the account.
pub const EXPIRED_MESSAGE: &str = "This sign-in has expired. Sign in again.";
pub const WRONG_CODE_MESSAGE: &str = "That code didn't work";
pub const WRONG_PASSWORD_MESSAGE: &str = "The current password is wrong";

fn now_text() -> String {
    format_time(OffsetDateTime::now_utc())
}

fn label(user: &User) -> String {
    user.display_name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| user.username.clone())
}

fn needs_second_step(user: &User) -> Option<ChallengeStage> {
    if user.totp_secret.is_some() {
        Some(ChallengeStage::TwoFactor)
    } else if user.must_change_password {
        Some(ChallengeStage::PasswordChange)
    } else {
        None
    }
}

impl AuthService {
    async fn find_user(&self, principal: &str) -> Result<Option<User>> {
        let db = self.db().await?;
        let principal = principal.trim();
        let users = super::LibrarianAuthStore::new(db)
            .users()
            .await
            .map_err(|error| anyhow!(error.to_string()))?;
        Ok(users.into_iter().find(|user| {
            user.username.eq_ignore_ascii_case(principal)
                || user
                    .email
                    .as_deref()
                    .is_some_and(|email| email.eq_ignore_ascii_case(principal))
        }))
    }

    async fn current_user(&self, user_id: &str) -> Result<User> {
        let db = self.db().await?;
        User::get(db.pool(), &user_id.to_string())
            .await?
            .filter(|user| user.is_active)
            .ok_or_else(|| anyhow!("Account not found"))
    }

    /// Argon2 off the async threads.
    async fn password_matches(&self, password: &str, hash: &str) -> Result<bool> {
        if !hash.starts_with("$argon2") {
            return Ok(false);
        }
        let inner = self.inner().await?;
        let (password, hash) = (password.to_string(), hash.to_string());
        match tokio::task::spawn_blocking(move || inner.verify_password(&password, &hash)).await? {
            Ok(()) => Ok(true),
            Err(agql_auth::AuthError::InvalidCredentials) => Ok(false),
            Err(error) => Err(anyhow!(error.to_string())),
        }
    }

    /// The current password, for changes made while signed in. Wrong guesses count against
    /// the account in the login rate limiter.
    async fn check_current_password(&self, user: &User, password: &str) -> Result<()> {
        let key = format!("account:{}", user.id);
        if self.login_limiter.is_blocked(&key) {
            return Err(anyhow!(RATE_LIMITED_MESSAGE));
        }
        if self.password_matches(password, &user.password_hash).await? {
            self.login_limiter.clear(&key);
            Ok(())
        } else {
            self.login_limiter.record_failure(&key);
            Err(anyhow!(WRONG_PASSWORD_MESSAGE))
        }
    }

    async fn hash_new_password(&self, password: &str) -> Result<String> {
        security::check_new_password(password).map_err(|message| anyhow!(message))?;
        let inner = self.inner().await?;
        let password = password.to_string();
        tokio::task::spawn_blocking(move || inner.hash_password(&password))
            .await?
            .map_err(|error| anyhow!(error.to_string()))
    }

    async fn encryption(&self) -> Result<CredentialEncryption> {
        let sources = self
            .manager
            .get_sources()
            .await
            .ok_or_else(|| anyhow!("The credential key is not loaded, so two-factor sign-in is unavailable"))?;
        let manager = sources
            .get_manager()
            .await
            .ok_or_else(|| anyhow!("The credential key is not loaded, so two-factor sign-in is unavailable"))?;
        Ok(manager.encryption().clone())
    }

    async fn update_user(&self, user_id: &str, input: UpdateUserInput) -> Result<User> {
        let db = self.db().await?;
        User::update_by_id(&db, &user_id.to_string(), input)
            .await?
            .ok_or_else(|| anyhow!("Account not found"))
    }

    /// A session for an account whose second step is done.
    async fn issue_session(&self, user: User, method: AuthMethod) -> Result<LoginResult> {
        let payload = self
            .inner()
            .await?
            .issue_verified_user_session_with_scopes(
                user.id.clone(),
                vec![user.role.clone()],
                Vec::new(),
                method,
                ClientMetadata::default(),
            )
            .await
            .map_err(|error| anyhow!(error.to_string()))?;
        let user = self
            .update_user(
                &user.id,
                UpdateUserInput {
                    last_login_at: Some(Some(now_text())),
                    ..Default::default()
                },
            )
            .await
            .unwrap_or(user);
        Ok(to_login_result(user, payload))
    }

    /// Password sign-in. An account with two-factor sign-in or a temporary password gets a
    /// challenge, never a session: its password is checked here, against the same rate limiter.
    pub async fn sign_in(&self, principal: &str, password: &str) -> Result<SignIn> {
        let candidate = self.find_user(principal).await?;
        let Some((user, stage)) = candidate
            .filter(|user| user.is_active && user.password_hash.starts_with("$argon2"))
            .and_then(|user| needs_second_step(&user).map(|stage| (user, stage)))
        else {
            return Ok(SignIn::Session(self.password_login(principal, password).await?));
        };
        if self.login_limiter.is_blocked(principal) {
            return Err(anyhow!(RATE_LIMITED_MESSAGE));
        }
        if !self.password_matches(password, &user.password_hash).await? {
            self.login_limiter.record_failure(principal);
            return Err(anyhow!("invalid credentials"));
        }
        let token = self
            .pending
            .create(&user.id, principal, stage, &user.password_hash);
        Ok(SignIn::Challenge(SignInChallenge {
            token,
            stage,
            expires_in_seconds: security::CHALLENGE_TTL.as_secs() as i64,
        }))
    }

    /// The pending sign-in and the account as it is now. A password changed or an account
    /// disabled since the first step ends it.
    async fn pending_account(
        &self,
        token: &str,
        stage: ChallengeStage,
    ) -> Result<(PendingSignIn, User)> {
        let pending = self
            .pending
            .get(token, stage)
            .ok_or_else(|| anyhow!(EXPIRED_MESSAGE))?;
        if self.login_limiter.is_blocked(&pending.principal) {
            return Err(anyhow!(RATE_LIMITED_MESSAGE));
        }
        match self.current_user(&pending.user_id).await {
            Ok(user) if user.password_hash == pending.password_hash => Ok((pending, user)),
            _ => {
                self.pending.remove(token);
                Err(anyhow!(EXPIRED_MESSAGE))
            }
        }
    }

    /// Spend one recovery code. Serialised, so the same code can't be spent twice at once.
    async fn spend_recovery_code(&self, user_id: &str, code: &str) -> Result<bool> {
        let _guard = self.recovery_lock.lock().await;
        let user = self.current_user(user_id).await?;
        let Some(left) =
            security::consume_recovery_code(user.recovery_codes.as_deref(), &user.id, code)
        else {
            return Ok(false);
        };
        self.update_user(
            &user.id,
            UpdateUserInput {
                recovery_codes: Some(Some(
                    serde_json::to_string(&left).unwrap_or_else(|_| "[]".into()),
                )),
                recovery_codes_remaining: Some(left.len() as i32),
                ..Default::default()
            },
        )
        .await?;
        info!(
            user = %user.username,
            remaining = left.len(),
            "'{}' signed in with a recovery code ({} left)",
            user.username,
            left.len()
        );
        Ok(true)
    }

    /// A TOTP code (replay-safe, with agql-auth's own abuse limits) against an encrypted secret.
    async fn totp_matches(&self, user_id: &str, encrypted: &str, code: &str) -> Result<bool> {
        let secret = self
            .encryption()
            .await?
            .decrypt_envelope(encrypted)
            .map_err(|_| anyhow!("The two-factor secret could not be read"))?;
        let result = self
            .inner()
            .await?
            .verify_totp_code_with_replay_store_and_metadata(
                &self.totp_replay,
                user_id,
                Some(TOTP_FACTOR),
                &secret,
                &security::compact_code(code),
                TotpOptions::default(),
                OffsetDateTime::now_utc(),
                ClientMetadata::default(),
            )
            .await;
        match result {
            Ok(()) => Ok(true),
            Err(
                agql_auth::AuthError::InvalidTotpCode | agql_auth::AuthError::TotpCodeReplayed,
            ) => Ok(false),
            Err(error) => Err(anyhow!(error.to_string())),
        }
    }

    /// The second step: a code from the authenticator app, or a recovery code once.
    pub async fn complete_two_factor(&self, token: &str, code: &str) -> Result<SignIn> {
        let (pending, user) = self.pending_account(token, ChallengeStage::TwoFactor).await?;
        let Some(secret) = user.totp_secret.clone() else {
            self.pending.remove(token);
            return Err(anyhow!(EXPIRED_MESSAGE));
        };
        let accepted = if security::looks_like_totp(code) {
            self.totp_matches(&user.id, &secret, code).await?
        } else {
            self.spend_recovery_code(&user.id, code).await?
        };
        if !accepted {
            self.login_limiter.record_failure(&pending.principal);
            self.pending.record_failure(token);
            return Err(anyhow!(WRONG_CODE_MESSAGE));
        }
        self.login_limiter.clear(&pending.principal);
        if user.must_change_password {
            self.pending.advance(token, ChallengeStage::PasswordChange);
            return Ok(SignIn::Challenge(SignInChallenge {
                token: token.to_string(),
                stage: ChallengeStage::PasswordChange,
                expires_in_seconds: security::CHALLENGE_TTL.as_secs() as i64,
            }));
        }
        self.pending.remove(token);
        Ok(SignIn::Session(
            self.issue_session(user, AuthMethod::TotpStepUp).await?,
        ))
    }

    /// The last step after a temporary password: choose a new one, then sign in.
    pub async fn complete_password_change(
        &self,
        token: &str,
        new_password: &str,
    ) -> Result<LoginResult> {
        let (_, user) = self
            .pending_account(token, ChallengeStage::PasswordChange)
            .await?;
        if self.password_matches(new_password, &user.password_hash).await? {
            return Err(anyhow!("Choose a password other than the temporary one"));
        }
        let hash = self.hash_new_password(new_password).await?;
        let user = self
            .update_user(
                &user.id,
                UpdateUserInput {
                    password_hash: Some(hash),
                    must_change_password: Some(false),
                    password_changed_at: Some(Some(now_text())),
                    ..Default::default()
                },
            )
            .await?;
        self.pending.remove(token);
        info!(user = %user.username, "'{}' replaced a temporary password", user.username);
        self.issue_session(user, AuthMethod::Password).await
    }

    /// Change your own password with the current one. Every session of the account is revoked
    /// by the User update hook, so a new one is issued for this device.
    pub async fn change_password(
        &self,
        user_id: &str,
        current_password: &str,
        new_password: &str,
    ) -> Result<LoginResult> {
        let user = self.current_user(user_id).await?;
        self.check_current_password(&user, current_password).await?;
        if current_password == new_password {
            return Err(anyhow!("The new password is the same as the current one"));
        }
        let hash = self.hash_new_password(new_password).await?;
        let user = self
            .update_user(
                &user.id,
                UpdateUserInput {
                    password_hash: Some(hash),
                    must_change_password: Some(false),
                    password_changed_at: Some(Some(now_text())),
                    ..Default::default()
                },
            )
            .await?;
        info!(user = %user.username, "'{}' changed their password", user.username);
        self.issue_session(user, AuthMethod::Password).await
    }

    /// Start enrolling: a new secret, kept encrypted as pending until a code from it is
    /// confirmed. Starting again replaces it.
    pub async fn begin_two_factor(
        &self,
        user_id: &str,
        password: &str,
    ) -> Result<TwoFactorEnrollment> {
        let user = self.current_user(user_id).await?;
        self.check_current_password(&user, password).await?;
        let encryption = self.encryption().await?;
        let inner = self.inner().await?;
        let secret = inner
            .generate_totp_secret(20)
            .map_err(|error| anyhow!(error.to_string()))?;
        let account = user.email.clone().unwrap_or_else(|| user.username.clone());
        let provisioning = inner
            .build_totp_provisioning(&secret, TOTP_ISSUER, account, TotpOptions::default())
            .map_err(|error| anyhow!(error.to_string()))?;
        let qr_svg = security::qr_svg(&provisioning.uri)?;
        self.update_user(
            &user.id,
            UpdateUserInput {
                totp_pending_secret: Some(Some(encryption.encrypt_envelope(&secret.base32_secret)?)),
                ..Default::default()
            },
        )
        .await?;
        Ok(TwoFactorEnrollment {
            secret: provisioning.secret,
            uri: provisioning.uri,
            qr_svg,
        })
    }

    /// Finish enrolling with a code from the new secret. Returns the recovery codes, the only
    /// time they are shown.
    pub async fn confirm_two_factor(&self, user_id: &str, code: &str) -> Result<Vec<String>> {
        let user = self.current_user(user_id).await?;
        let Some(pending) = user.totp_pending_secret.clone() else {
            return Err(anyhow!("Start setting up two-factor sign-in first"));
        };
        if !security::looks_like_totp(code) || !self.totp_matches(&user.id, &pending, code).await? {
            return Err(anyhow!(WRONG_CODE_MESSAGE));
        }
        let codes = security::generate_recovery_codes();
        let user = self
            .update_user(
                &user.id,
                UpdateUserInput {
                    totp_secret: Some(Some(pending)),
                    totp_pending_secret: Some(None),
                    two_factor_enabled_at: Some(Some(now_text())),
                    recovery_codes: Some(Some(security::recovery_hashes_json(&user.id, &codes))),
                    recovery_codes_remaining: Some(codes.len() as i32),
                    ..Default::default()
                },
            )
            .await?;
        info!(user = %user.username, "Two-factor sign-in turned on for '{}'", user.username);
        Ok(codes)
    }

    /// New recovery codes, replacing the old ones.
    pub async fn regenerate_recovery_codes(
        &self,
        user_id: &str,
        password: &str,
    ) -> Result<Vec<String>> {
        let user = self.current_user(user_id).await?;
        if user.totp_secret.is_none() {
            return Err(anyhow!("Two-factor sign-in is off"));
        }
        self.check_current_password(&user, password).await?;
        let codes = security::generate_recovery_codes();
        self.update_user(
            &user.id,
            UpdateUserInput {
                recovery_codes: Some(Some(security::recovery_hashes_json(&user.id, &codes))),
                recovery_codes_remaining: Some(codes.len() as i32),
                ..Default::default()
            },
        )
        .await?;
        info!(user = %user.username, "New recovery codes made for '{}'", user.username);
        Ok(codes)
    }

    fn two_factor_cleared() -> UpdateUserInput {
        UpdateUserInput {
            totp_secret: Some(None),
            totp_pending_secret: Some(None),
            two_factor_enabled_at: Some(None),
            recovery_codes: Some(None),
            recovery_codes_remaining: Some(0),
            ..Default::default()
        }
    }

    /// Turn your own two-factor sign-in off, with your password.
    pub async fn disable_two_factor(&self, user_id: &str, password: &str) -> Result<()> {
        let user = self.current_user(user_id).await?;
        self.check_current_password(&user, password).await?;
        self.update_user(&user.id, Self::two_factor_cleared()).await?;
        info!(user = %user.username, "Two-factor sign-in turned off by '{}'", user.username);
        Ok(())
    }

    async fn admin_and_target(&self, admin_id: &str, user_id: &str) -> Result<(User, User)> {
        let db = self.db().await?;
        let admin = self
            .current_user(admin_id)
            .await
            .ok()
            .filter(|admin| admin.role == "admin")
            .ok_or_else(|| anyhow!("This needs an administrator"))?;
        let target = User::get(db.pool(), &user_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("Account not found"))?;
        Ok((admin, target))
    }

    /// Account recovery: an administrator turns someone's two-factor sign-in off. Logged.
    pub async fn admin_reset_two_factor(&self, admin_id: &str, user_id: &str) -> Result<()> {
        let (admin, target) = self.admin_and_target(admin_id, user_id).await?;
        self.update_user(&target.id, Self::two_factor_cleared()).await?;
        warn!(
            admin = %admin.username,
            user = %target.username,
            "Administrator '{}' turned off two-factor sign-in for '{}' (account recovery)",
            admin.username,
            target.username
        );
        Ok(())
    }

    /// A temporary password: the person must choose a new one at their next sign-in. Every
    /// session of theirs is revoked by the User update hook.
    pub async fn admin_set_temporary_password(
        &self,
        admin_id: &str,
        user_id: &str,
        password: &str,
    ) -> Result<()> {
        let (admin, target) = self.admin_and_target(admin_id, user_id).await?;
        let hash = self.hash_new_password(password).await?;
        self.update_user(
            &target.id,
            UpdateUserInput {
                password_hash: Some(hash),
                must_change_password: Some(true),
                password_changed_at: Some(Some(now_text())),
                ..Default::default()
            },
        )
        .await?;
        warn!(
            admin = %admin.username,
            user = %target.username,
            "Administrator '{}' set a temporary password for '{}'",
            admin.username,
            target.username
        );
        Ok(())
    }

    /// A one-time reset link for an administrator to pass on (nothing sends mail). Earlier
    /// links for the account stop working. Returns the token and when it expires.
    pub async fn admin_create_reset_link(
        &self,
        admin_id: &str,
        user_id: &str,
    ) -> Result<(String, OffsetDateTime)> {
        let (admin, target) = self.admin_and_target(admin_id, user_id).await?;
        let db = self.db().await?;
        let earlier = PasswordResetToken::query(db.pool())
            .filter(PasswordResetTokenWhereInput {
                user_id: Some(graphql_orm::graphql::filters::StringFilter {
                    eq: Some(target.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        for row in earlier {
            PasswordResetToken::delete_by_id(&db, &row.id).await?;
        }
        let store = ResetLinkStore {
            db,
            created_by: Some(admin.id.clone()),
        };
        let issued = self
            .inner()
            .await?
            .issue_password_reset_token_with_store(
                &store,
                target.id.clone(),
                Duration::hours(security::RESET_LINK_TTL_HOURS),
            )
            .await
            .map_err(|error| anyhow!(error.to_string()))?;
        warn!(
            admin = %admin.username,
            user = %target.username,
            "Administrator '{}' made a password reset link for '{}'",
            admin.username,
            target.username
        );
        Ok((issued.token, issued.expires_at))
    }

    /// Use a reset link: it works once, before it expires, for an account that is still
    /// active. Sessions are revoked by the User update hook; the person then signs in as usual,
    /// with two-factor sign-in when they have it.
    pub async fn reset_password_with_link(&self, token: &str, new_password: &str) -> Result<()> {
        security::check_new_password(new_password).map_err(|message| anyhow!(message))?;
        let db = self.db().await?;
        let store = ResetLinkStore {
            db: db.clone(),
            created_by: None,
        };
        let verified = self
            .inner()
            .await?
            .consume_password_reset_token_with_metadata(
                &store,
                token.trim(),
                ClientMetadata::default(),
            )
            .await
            .map_err(|error| anyhow!(error.to_string()))?;
        let user = self.current_user(&verified.user_id).await?;
        let hash = self.hash_new_password(new_password).await?;
        let user = self
            .update_user(
                &user.id,
                UpdateUserInput {
                    password_hash: Some(hash),
                    must_change_password: Some(false),
                    password_changed_at: Some(Some(now_text())),
                    ..Default::default()
                },
            )
            .await?;
        info!(user = %user.username, "'{}' set a new password with a reset link", label(&user));
        Ok(())
    }
}
