//! Two-factor sign-in, recovery codes, temporary passwords and reset links against the real
//! ORM records (design.md Q147–Q148).
use super::*;
use crate::graphql::entities::{
    CreateUserInput, PasswordResetToken, UpdatePasswordResetTokenInput,
};
use crate::services::account_security::{CHALLENGE_MAX_ATTEMPTS, ChallengeStage};
use crate::services::auth::account::{EXPIRED_MESSAGE, WRONG_CODE_MESSAGE};
use crate::services::sources::encryption::CredentialEncryption;
use crate::services::sources::service::SourcesServiceConfig;
use crate::services::{DatabaseServiceConfig, ServicesManager};
use hmac_sha1_compat::{Hmac, Mac};

const PASSWORD: &str = "test-only-password";

struct Fixture {
    _temp: tempfile::TempDir,
    _services: Arc<ServicesManager>,
    db: Database,
    auth: Arc<AuthService>,
    admin: User,
    member: User,
}

async fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let services = ServicesManager::builder()
        .add_service(DatabaseServiceConfig {
            database_url: format!("sqlite://{}", temp.path().join("account.db").display()),
            connect_timeout: std::time::Duration::from_secs(5),
        })
        .add_service(AuthConfig::for_tests())
        .add_service(SourcesServiceConfig::with_key(
            CredentialEncryption::generate_key(),
        ))
        .start()
        .await
        .unwrap();
    let auth = services.get_auth().await.unwrap();
    let db = services.get_database().await.unwrap().pool().clone();
    let admin = auth
        .register(RegisterInput {
            email: "admin@example.test".into(),
            name: "Admin".into(),
            password: PASSWORD.into(),
            invite_token: None,
        })
        .await
        .unwrap();
    let admin = User::get(db.pool(), &admin.user.id).await.unwrap().unwrap();
    let hash = auth.inner().await.unwrap().hash_password(PASSWORD).unwrap();
    let member = User::insert(
        &db,
        CreateUserInput {
            username: "sam".into(),
            email: Some("sam@example.test".into()),
            password_hash: hash,
            role: "member".into(),
            display_name: Some("Sam".into()),
            avatar_url: None,
            is_active: true,
            last_login_at: None,
            phone: None,
            genre_interests: None,
            must_change_password: false,
            two_factor_enabled_at: None,
            recovery_codes_remaining: 0,
            password_changed_at: None,
            totp_secret: None,
            totp_pending_secret: None,
            recovery_codes: None,
        },
    )
    .await
    .unwrap();
    Fixture {
        _temp: temp,
        _services: services,
        db,
        auth,
        admin,
        member,
    }
}

/// RFC 6238 with the defaults agql-auth uses (SHA-1, six digits, 30 seconds).
fn totp(secret_base32: &str, offset_steps: i64) -> String {
    let secret = data_encoding_decode(secret_base32);
    let step = (OffsetDateTime::now_utc().unix_timestamp() / 30 + offset_steps) as u64;
    let mut mac = Hmac::<sha1::Sha1>::new_from_slice(&secret).unwrap();
    mac.update(&step.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = (digest[19] & 0x0f) as usize;
    let binary = ((u32::from(digest[offset]) & 0x7f) << 24)
        | (u32::from(digest[offset + 1]) << 16)
        | (u32::from(digest[offset + 2]) << 8)
        | u32::from(digest[offset + 3]);
    format!("{:06}", binary % 1_000_000)
}

/// Base32 without padding (RFC 4648), as authenticator apps read it.
fn data_encoding_decode(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bits = 0u32;
    let mut count = 0;
    let mut out = Vec::new();
    for byte in text.bytes() {
        let value = ALPHABET.iter().position(|letter| *letter == byte).unwrap() as u32;
        bits = (bits << 5) | value;
        count += 5;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    out
}

async fn reload(f: &Fixture, id: &str) -> User {
    User::get(f.db.pool(), &id.to_string())
        .await
        .unwrap()
        .unwrap()
}

/// Enrol the member and return their secret and recovery codes.
async fn enrol(f: &Fixture) -> (String, Vec<String>) {
    let enrollment = f
        .auth
        .begin_two_factor(&f.member.id, PASSWORD)
        .await
        .unwrap();
    // A wrong code never confirms.
    let wrong = if totp(&enrollment.secret, 0) == "000000" {
        "111111"
    } else {
        "000000"
    };
    assert!(
        f.auth
            .confirm_two_factor(&f.member.id, wrong)
            .await
            .is_err()
    );
    let codes = f
        .auth
        .confirm_two_factor(&f.member.id, &totp(&enrollment.secret, 0))
        .await
        .unwrap();
    (enrollment.secret, codes)
}

async fn challenge(f: &Fixture) -> SignInChallenge {
    match f.auth.sign_in("sam@example.test", PASSWORD).await.unwrap() {
        SignIn::Challenge(challenge) => challenge,
        SignIn::Session(_) => panic!("an account with a second step must not get a session"),
    }
}

#[tokio::test]
async fn enrolling_keeps_the_secret_encrypted_and_the_codes_hashed() {
    let f = fixture().await;
    assert!(
        f.auth
            .begin_two_factor(&f.member.id, "wrong-password")
            .await
            .is_err()
    );
    let enrollment = f
        .auth
        .begin_two_factor(&f.member.id, PASSWORD)
        .await
        .unwrap();
    assert!(enrollment.uri.starts_with("otpauth://totp/"));
    assert!(enrollment.qr_svg.contains("<svg"));
    let pending = reload(&f, &f.member.id).await;
    let stored = pending.totp_pending_secret.clone().unwrap();
    assert!(
        stored.starts_with("v2:"),
        "the pending secret is a credential envelope"
    );
    assert!(!stored.contains(&enrollment.secret));
    assert!(pending.totp_secret.is_none() && pending.two_factor_enabled_at.is_none());

    let codes = f
        .auth
        .confirm_two_factor(&f.member.id, &totp(&enrollment.secret, 0))
        .await
        .unwrap();
    assert_eq!(codes.len(), 10);
    let user = reload(&f, &f.member.id).await;
    assert_eq!(user.totp_secret.as_deref(), Some(stored.as_str()));
    assert!(user.totp_pending_secret.is_none());
    assert!(user.two_factor_enabled_at.is_some());
    assert_eq!(user.recovery_codes_remaining, 10);
    let hashes = user.recovery_codes.unwrap();
    assert!(codes.iter().all(|code| !hashes.contains(code.as_str())));
}

#[tokio::test]
async fn the_password_step_issues_no_usable_token_and_a_code_signs_in() {
    let f = fixture().await;
    let (secret, _) = enrol(&f).await;
    let pending = challenge(&f).await;
    assert_eq!(pending.stage, ChallengeStage::TwoFactor);
    // The challenge is no credential: not a bearer token, not a refresh token.
    assert!(
        f.auth
            .authenticate_access_token(&pending.token)
            .await
            .is_err()
    );
    assert!(f.auth.refresh_token(&pending.token).await.is_err());
    // The plain login keeps refusing an account with a second step.
    assert!(f.auth.login("sam@example.test", PASSWORD).await.is_err());

    // The code that confirmed enrolment was spent; the next step's code signs in.
    let signed_in = f
        .auth
        .complete_two_factor(&pending.token, &totp(&secret, 1))
        .await
        .unwrap();
    let SignIn::Session(result) = signed_in else {
        panic!("a correct code signs in")
    };
    let user = f
        .auth
        .authenticate_access_token(&result.tokens.access_token)
        .await
        .unwrap();
    assert_eq!(user.user_id, f.member.id);
    f.auth
        .refresh_token(&result.tokens.refresh_token)
        .await
        .expect("a two-factor session refreshes");
    // The challenge is gone once used.
    assert_eq!(
        f.auth
            .complete_two_factor(&pending.token, &totp(&secret, 1))
            .await
            .unwrap_err()
            .to_string(),
        EXPIRED_MESSAGE
    );
}

#[tokio::test]
async fn a_recovery_code_works_once() {
    let f = fixture().await;
    let (_, codes) = enrol(&f).await;
    let first = challenge(&f).await;
    let SignIn::Session(_) = f
        .auth
        .complete_two_factor(&first.token, &codes[3].to_uppercase())
        .await
        .unwrap()
    else {
        panic!("a recovery code signs in");
    };
    assert_eq!(reload(&f, &f.member.id).await.recovery_codes_remaining, 9);
    let second = challenge(&f).await;
    assert_eq!(
        f.auth
            .complete_two_factor(&second.token, &codes[3])
            .await
            .unwrap_err()
            .to_string(),
        WRONG_CODE_MESSAGE
    );
    let SignIn::Session(_) = f
        .auth
        .complete_two_factor(&second.token, &codes[4])
        .await
        .unwrap()
    else {
        panic!("another unused code still works");
    };
}

#[tokio::test]
async fn wrong_codes_end_the_challenge_and_then_rate_limit_the_account() {
    let f = fixture().await;
    enrol(&f).await;
    let pending = challenge(&f).await;
    for _ in 0..CHALLENGE_MAX_ATTEMPTS {
        assert_eq!(
            f.auth
                .complete_two_factor(&pending.token, "zzzzz-zzzzz")
                .await
                .unwrap_err()
                .to_string(),
            WRONG_CODE_MESSAGE
        );
    }
    assert_eq!(
        f.auth
            .complete_two_factor(&pending.token, "zzzzz-zzzzz")
            .await
            .unwrap_err()
            .to_string(),
        EXPIRED_MESSAGE
    );
    // Ten wrong codes in the window block the sign-in name, the password step included.
    let again = challenge(&f).await;
    for _ in 0..CHALLENGE_MAX_ATTEMPTS {
        let _ = f
            .auth
            .complete_two_factor(&again.token, "zzzzz-zzzzz")
            .await;
    }
    let blocked = f
        .auth
        .sign_in("sam@example.test", PASSWORD)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        crate::services::login_rate_limit::is_rate_limit_message(&blocked),
        "{blocked}"
    );
}

#[tokio::test]
async fn a_temporary_password_must_be_replaced_before_any_session() {
    let f = fixture().await;
    let before = f.auth.login("sam@example.test", PASSWORD).await.unwrap();
    // Only an administrator can set one.
    assert!(
        f.auth
            .admin_set_temporary_password(&f.member.id, &f.admin.id, "another-temporary-1")
            .await
            .is_err()
    );
    f.auth
        .admin_set_temporary_password(&f.admin.id, &f.member.id, "temporary-pass-123")
        .await
        .unwrap();
    assert!(reload(&f, &f.member.id).await.must_change_password);
    // The old session ended with the password.
    assert!(
        f.auth
            .refresh_token(&before.tokens.refresh_token)
            .await
            .is_err()
    );

    let pending = match f.auth.sign_in("sam", "temporary-pass-123").await.unwrap() {
        SignIn::Challenge(challenge) => challenge,
        SignIn::Session(_) => panic!("a temporary password must not sign in"),
    };
    assert_eq!(pending.stage, ChallengeStage::PasswordChange);
    assert!(
        f.auth
            .authenticate_access_token(&pending.token)
            .await
            .is_err()
    );
    assert!(
        f.auth
            .complete_password_change(&pending.token, "temporary-pass-123")
            .await
            .is_err()
    );
    assert!(
        f.auth
            .complete_password_change(&pending.token, "short")
            .await
            .is_err()
    );
    let result = f
        .auth
        .complete_password_change(&pending.token, "my-own-password-9")
        .await
        .unwrap();
    f.auth
        .authenticate_access_token(&result.tokens.access_token)
        .await
        .unwrap();
    let user = reload(&f, &f.member.id).await;
    assert!(!user.must_change_password);
    f.auth.login("sam", "my-own-password-9").await.unwrap();
}

#[tokio::test]
async fn a_reset_link_works_once_and_not_after_it_expires() {
    let f = fixture().await;
    assert!(
        f.auth
            .admin_create_reset_link(&f.member.id, &f.admin.id)
            .await
            .is_err(),
        "members make no links"
    );
    let (token, expires_at) = f
        .auth
        .admin_create_reset_link(&f.admin.id, &f.member.id)
        .await
        .unwrap();
    assert!(expires_at > OffsetDateTime::now_utc() + Duration::hours(23));
    let rows = PasswordResetToken::query(f.db.pool())
        .fetch_all()
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(
        !format!("{rows:?}").contains(&token),
        "the link itself is never stored"
    );

    f.auth
        .reset_password_with_link(&token, "reset-password-42")
        .await
        .unwrap();
    assert!(
        f.auth
            .reset_password_with_link(&token, "reset-password-43")
            .await
            .is_err(),
        "a link works once"
    );
    f.auth.login("sam", "reset-password-42").await.unwrap();

    // A newer link replaces the older; an expired one is refused.
    let (old, _) = f
        .auth
        .admin_create_reset_link(&f.admin.id, &f.member.id)
        .await
        .unwrap();
    let (expiring, _) = f
        .auth
        .admin_create_reset_link(&f.admin.id, &f.member.id)
        .await
        .unwrap();
    assert!(
        f.auth
            .reset_password_with_link(&old, "reset-password-44")
            .await
            .is_err()
    );
    let row = PasswordResetToken::query(f.db.pool())
        .fetch_all()
        .await
        .unwrap()
        .pop()
        .unwrap();
    PasswordResetToken::update_by_id(
        &f.db,
        &row.id,
        UpdatePasswordResetTokenInput {
            expires_at: Some(format_time(
                OffsetDateTime::now_utc() - Duration::minutes(1),
            )),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        f.auth
            .reset_password_with_link(&expiring, "reset-password-45")
            .await
            .is_err()
    );
    f.auth.login("sam", "reset-password-42").await.unwrap();
}

#[tokio::test]
async fn only_an_administrator_turns_off_someone_elses_two_factor_sign_in() {
    let f = fixture().await;
    enrol(&f).await;
    assert!(
        f.auth
            .admin_reset_two_factor(&f.member.id, &f.member.id)
            .await
            .is_err()
    );
    assert!(reload(&f, &f.member.id).await.totp_secret.is_some());
    f.auth
        .admin_reset_two_factor(&f.admin.id, &f.member.id)
        .await
        .unwrap();
    let user = reload(&f, &f.member.id).await;
    assert!(
        user.totp_secret.is_none()
            && user.two_factor_enabled_at.is_none()
            && user.recovery_codes.is_none()
    );
    assert!(matches!(
        f.auth.sign_in("sam", PASSWORD).await.unwrap(),
        SignIn::Session(_)
    ));
}

#[tokio::test]
async fn changing_your_password_needs_the_current_one_and_reissues_this_session() {
    let f = fixture().await;
    let before = f.auth.login("sam", PASSWORD).await.unwrap();
    assert!(
        f.auth
            .change_password(&f.member.id, "wrong-password", "brand-new-pass-1")
            .await
            .is_err()
    );
    let result = f
        .auth
        .change_password(&f.member.id, PASSWORD, "brand-new-pass-1")
        .await
        .unwrap();
    assert!(
        f.auth
            .refresh_token(&before.tokens.refresh_token)
            .await
            .is_err(),
        "other sessions end"
    );
    f.auth
        .authenticate_access_token(&result.tokens.access_token)
        .await
        .unwrap();
    assert!(reload(&f, &f.member.id).await.password_changed_at.is_some());
}
