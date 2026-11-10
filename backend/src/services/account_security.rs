//! The parts of signing in that `agql-auth` leaves to its host (design.md Q147–Q148):
//!
//! * **Pending sign-ins.** A password that is right for an account with two-factor sign-in, or
//!   with a temporary password, gets an opaque challenge token instead of a session. It is
//!   random, kept only as its SHA-256 in memory, expires after five minutes and allows five
//!   wrong codes. It is not a JWT and is accepted by nothing but the two completion mutations,
//!   so it can't be used as a session.
//! * **Recovery codes.** Ten random codes of 50 bits, shown once, stored as SHA-256 hashes
//!   salted with the account id. Each works once.
//! * **TOTP replay.** `agql-auth` verifies codes; this store remembers each accepted time step
//!   for a few minutes so the same code can't be used twice.
//! * **Reset links.** `agql-auth` signs the link; its id is a `PasswordResetToken` row, consumed
//!   once by a conditional update, so a link works once and only before it expires.
//!
//! Nothing here logs a secret, a code or a token.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use agql_auth::{AuthResult, PasswordResetTokenStore, TotpReplayStore};
use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use graphql_orm::graphql::filters::{DateFilter, StringFilter};
use graphql_orm::graphql::orm::{MutationContext, TransactionMode};
use parking_lot::Mutex;
use rand::Rng;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::db::Database;
use crate::graphql::entities::{
    CreatePasswordResetTokenInput, PasswordResetToken, PasswordResetTokenWhereInput,
    UpdatePasswordResetTokenInput,
};

/// How long a pending sign-in waits for its code or new password.
pub const CHALLENGE_TTL: Duration = Duration::from_secs(5 * 60);
/// Wrong codes one pending sign-in allows before it is dropped.
pub const CHALLENGE_MAX_ATTEMPTS: u32 = 5;
/// How long an administrator's reset link works.
pub const RESET_LINK_TTL_HOURS: i64 = 24;
pub const RECOVERY_CODE_COUNT: usize = 10;
/// The same floor as registration's.
pub const MIN_PASSWORD_CHARS: usize = 10;
pub const MAX_PASSWORD_CHARS: usize = 1024;
/// The one TOTP factor an account has.
pub const TOTP_FACTOR: &str = "primary";
pub const TOTP_ISSUER: &str = "Librarian";

/// What a pending sign-in waits for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeStage {
    /// A code from the authenticator app, or a recovery code.
    TwoFactor,
    /// A new password, replacing a temporary one.
    PasswordChange,
}

impl ChallengeStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TwoFactor => "two_factor",
            Self::PasswordChange => "password_change",
        }
    }
}

/// A password that was right, waiting for the rest.
#[derive(Debug, Clone)]
pub struct PendingSignIn {
    pub user_id: String,
    /// What was typed to sign in, for the login rate limiter.
    pub principal: String,
    pub stage: ChallengeStage,
    /// The password hash the sign-in was checked against: a password changed meanwhile
    /// voids it.
    pub password_hash: String,
    expires_at: Instant,
    attempts: u32,
}

/// Pending sign-ins by the SHA-256 of their token. Process-local: a restart asks people to
/// sign in again, which is the safe direction.
#[derive(Default)]
pub struct PendingSignIns {
    entries: Mutex<HashMap<String, PendingSignIn>>,
}

fn token_key(token: &str) -> String {
    hex::encode(Sha256::digest(token.trim().as_bytes()))
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    rand::rng().fill_bytes(&mut bytes);
    bytes
}

impl PendingSignIns {
    /// Start one and return its token, shown to the client once.
    pub fn create(
        &self,
        user_id: &str,
        principal: &str,
        stage: ChallengeStage,
        password_hash: &str,
    ) -> String {
        let token = URL_SAFE_NO_PAD.encode(random_bytes::<32>());
        let now = Instant::now();
        let mut entries = self.entries.lock();
        entries.retain(|_, entry| entry.expires_at > now);
        entries.insert(
            token_key(&token),
            PendingSignIn {
                user_id: user_id.to_string(),
                principal: principal.to_string(),
                stage,
                password_hash: password_hash.to_string(),
                expires_at: now + CHALLENGE_TTL,
                attempts: 0,
            },
        );
        token
    }

    /// The pending sign-in, when it exists, has not expired and waits for `stage`.
    pub fn get(&self, token: &str, stage: ChallengeStage) -> Option<PendingSignIn> {
        let key = token_key(token);
        let mut entries = self.entries.lock();
        let entry = entries.get(&key)?;
        if entry.expires_at <= Instant::now() {
            entries.remove(&key);
            return None;
        }
        (entry.stage == stage).then(|| entry.clone())
    }

    /// Count a wrong code; the sign-in is dropped after the last allowed one.
    pub fn record_failure(&self, token: &str) {
        let key = token_key(token);
        let mut entries = self.entries.lock();
        if let Some(entry) = entries.get_mut(&key) {
            entry.attempts += 1;
            if entry.attempts >= CHALLENGE_MAX_ATTEMPTS {
                entries.remove(&key);
            }
        }
    }

    /// Move on to the next stage with the same token and a fresh expiry.
    pub fn advance(&self, token: &str, stage: ChallengeStage) {
        if let Some(entry) = self.entries.lock().get_mut(&token_key(token)) {
            entry.stage = stage;
            entry.attempts = 0;
            entry.expires_at = Instant::now() + CHALLENGE_TTL;
        }
    }

    pub fn remove(&self, token: &str) {
        self.entries.lock().remove(&token_key(token));
    }
}

/// Readable codes like `k7m2q-x9p4t`: 10 characters from a 32-letter alphabet without
/// look-alikes (no 0/o, 1/l/i), 50 bits each.
pub fn generate_recovery_codes() -> Vec<String> {
    const ALPHABET: &[u8; 32] = b"abcdefghjkmnpqrstuvwxyz23456789-";
    (0..RECOVERY_CODE_COUNT)
        .map(|_| {
            let bytes = random_bytes::<10>();
            let letters: String = bytes
                .iter()
                // 31 usable letters: the last is the separator, so map into the first 31.
                .map(|byte| ALPHABET[(*byte as usize) % 31] as char)
                .collect();
            format!("{}-{}", &letters[..5], &letters[5..])
        })
        .collect()
}

/// Lower case without separators or spaces, so a code typed any way matches.
pub fn normalize_recovery_code(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

pub fn hash_recovery_code(user_id: &str, code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(user_id.as_bytes());
    hasher.update(b":");
    hasher.update(normalize_recovery_code(code).as_bytes());
    hex::encode(hasher.finalize())
}

/// The stored hashes as JSON.
pub fn recovery_hashes_json(user_id: &str, codes: &[String]) -> String {
    let hashes: Vec<String> = codes
        .iter()
        .map(|code| hash_recovery_code(user_id, code))
        .collect();
    serde_json::to_string(&hashes).unwrap_or_else(|_| "[]".to_string())
}

/// The hashes left after using `code`, or `None` when it isn't one of them.
pub fn consume_recovery_code(
    stored: Option<&str>,
    user_id: &str,
    code: &str,
) -> Option<Vec<String>> {
    let hashes: Vec<String> = serde_json::from_str(stored?).ok()?;
    if normalize_recovery_code(code).len() != 10 {
        return None;
    }
    let wanted = hash_recovery_code(user_id, code);
    let position = hashes.iter().position(|hash| *hash == wanted)?;
    let mut left = hashes;
    left.remove(position);
    Some(left)
}

/// A code from an authenticator app is six digits (spaces allowed); anything else is tried as
/// a recovery code.
pub fn looks_like_totp(code: &str) -> bool {
    let digits: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    digits.len() == 6 && digits.chars().all(|c| c.is_ascii_digit())
}

pub fn compact_code(code: &str) -> String {
    code.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Accepted TOTP steps, for long enough to cover the verification window.
#[derive(Default)]
pub struct MemoryTotpReplay {
    used: Mutex<HashMap<(String, i64), OffsetDateTime>>,
}

#[async_trait]
impl TotpReplayStore for MemoryTotpReplay {
    async fn consume_totp_step(
        &self,
        principal_id: &str,
        factor_id: Option<&str>,
        step: i64,
        consumed_at: OffsetDateTime,
    ) -> AuthResult<bool> {
        let key = (
            format!("{principal_id}/{}", factor_id.unwrap_or(TOTP_FACTOR)),
            step,
        );
        let mut used = self.used.lock();
        used.retain(|_, at| consumed_at - *at < time::Duration::minutes(10));
        if used.contains_key(&key) {
            return Ok(false);
        }
        used.insert(key, consumed_at);
        Ok(true)
    }
}

fn rfc3339(value: OffsetDateTime) -> String {
    value.format(&Rfc3339).unwrap_or_else(|_| value.to_string())
}

fn store_error(error: impl std::fmt::Display) -> agql_auth::AuthError {
    agql_auth::AuthError::Store(error.to_string())
}

/// Reset link ids as `PasswordResetToken` rows (trusted repository writes; the generated roots
/// for this entity are closed).
pub struct ResetLinkStore {
    pub db: Database,
    /// The administrator making links through this store.
    pub created_by: Option<String>,
}

#[async_trait]
impl PasswordResetTokenStore for ResetLinkStore {
    async fn insert_password_reset_token(
        &self,
        token_id: Uuid,
        user_id: &str,
        expires_at: OffsetDateTime,
    ) -> AuthResult<()> {
        PasswordResetToken::insert(
            &self.db,
            CreatePasswordResetTokenInput {
                id: token_id.to_string(),
                user_id: user_id.to_string(),
                created_by_user_id: self.created_by.clone(),
                expires_at: rfc3339(expires_at),
                consumed_at: None,
            },
        )
        .await
        .map_err(store_error)?;
        Ok(())
    }

    async fn consume_password_reset_token(
        &self,
        token_id: Uuid,
        consumed_at: OffsetDateTime,
    ) -> AuthResult<bool> {
        let mut transaction = MutationContext::begin(&self.db, TransactionMode::StateMachine)
            .await
            .map_err(store_error)?;
        let changed = transaction
            .update_where::<PasswordResetToken>(
                PasswordResetTokenWhereInput {
                    id: Some(StringFilter {
                        eq: Some(token_id.to_string()),
                        ..Default::default()
                    }),
                    consumed_at: Some(DateFilter {
                        is_null: Some(true),
                        ..Default::default()
                    }),
                    expires_at: Some(DateFilter {
                        gt: Some(rfc3339(consumed_at)),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                UpdatePasswordResetTokenInput {
                    consumed_at: Some(Some(rfc3339(consumed_at))),
                    ..Default::default()
                },
            )
            .await
            .map_err(store_error)?;
        transaction.commit_and_emit().await.map_err(store_error)?;
        Ok(changed == 1)
    }
}

/// The QR code an authenticator app scans, as SVG.
pub fn qr_svg(uri: &str) -> anyhow::Result<String> {
    let code = qrcode::QrCode::new(uri.as_bytes())?;
    Ok(code
        .render::<qrcode::render::svg::Color<'_>>()
        .min_dimensions(220, 220)
        .quiet_zone(true)
        .dark_color(qrcode::render::svg::Color("#000000"))
        .light_color(qrcode::render::svg::Color("#ffffff"))
        .build())
}

/// Registration's rule for any new password.
pub fn check_new_password(password: &str) -> Result<(), &'static str> {
    let length = password.chars().count();
    if length < MIN_PASSWORD_CHARS {
        return Err("Use at least 10 characters");
    }
    if length > MAX_PASSWORD_CHARS {
        return Err("That password is too long");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_codes_are_ten_distinct_readable_codes() {
        let codes = generate_recovery_codes();
        assert_eq!(codes.len(), RECOVERY_CODE_COUNT);
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len());
        for code in &codes {
            assert_eq!(code.len(), 11);
            assert_eq!(&code[5..6], "-");
            assert!(!code.contains(['0', 'o', '1', 'l', 'i']));
        }
    }

    #[test]
    fn a_recovery_code_matches_however_it_is_typed_and_only_once() {
        let codes = vec!["abcde-fghjk".to_string(), "mnpqr-stuvw".to_string()];
        let stored = recovery_hashes_json("user-1", &codes);
        assert!(!stored.contains("abcde"));
        let left = consume_recovery_code(Some(&stored), "user-1", " ABCDE FGHJK ").unwrap();
        assert_eq!(left.len(), 1);
        let left_json = serde_json::to_string(&left).unwrap();
        assert!(consume_recovery_code(Some(&left_json), "user-1", "abcde-fghjk").is_none());
        // Salted with the account: another account's hashes never match.
        assert!(consume_recovery_code(Some(&stored), "user-2", "abcde-fghjk").is_none());
    }

    #[test]
    fn six_digits_are_a_totp_code_anything_else_a_recovery_code() {
        assert!(looks_like_totp("123 456"));
        assert!(!looks_like_totp("12345"));
        assert!(!looks_like_totp("abcde-fghjk"));
    }

    #[test]
    fn a_pending_sign_in_waits_for_its_stage_and_drops_after_five_wrong_codes() {
        let pending = PendingSignIns::default();
        let token = pending.create("u", "alice", ChallengeStage::TwoFactor, "hash");
        assert!(
            pending
                .get(&token, ChallengeStage::PasswordChange)
                .is_none()
        );
        assert!(pending.get(&token, ChallengeStage::TwoFactor).is_some());
        for _ in 0..CHALLENGE_MAX_ATTEMPTS - 1 {
            pending.record_failure(&token);
            assert!(pending.get(&token, ChallengeStage::TwoFactor).is_some());
        }
        pending.record_failure(&token);
        assert!(pending.get(&token, ChallengeStage::TwoFactor).is_none());
        assert!(
            pending
                .get("not-a-token", ChallengeStage::TwoFactor)
                .is_none()
        );
    }

    #[tokio::test]
    async fn an_accepted_totp_step_cannot_be_used_again() {
        let replay = MemoryTotpReplay::default();
        let now = OffsetDateTime::now_utc();
        assert!(replay.consume_totp_step("u", None, 42, now).await.unwrap());
        assert!(!replay.consume_totp_step("u", None, 42, now).await.unwrap());
        assert!(replay.consume_totp_step("u", None, 43, now).await.unwrap());
        assert!(replay.consume_totp_step("v", None, 42, now).await.unwrap());
    }

    #[test]
    fn the_qr_code_is_svg() {
        let svg = qr_svg("otpauth://totp/Librarian:alice?secret=ABC&issuer=Librarian").unwrap();
        assert!(svg.contains("<svg"));
    }
}
