//! Exercise the actual ORM records and GraphQL cookie responses, not a mock store.
use super::auth::*;
use crate::graphql::entities::{RefreshToken, UpdateRefreshTokenInput, UpdateUserInput, User};
use crate::services::auth::{AuthConfig, RegisterInput};
use crate::services::{DatabaseServiceConfig, ServicesManager};
use async_graphql::{EmptySubscription, Object, Request, Response, Schema};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

struct TestQuery;
#[Object]
impl TestQuery {
    async fn ping(&self) -> bool {
        true
    }
}

fn cookie(response: &Response, name: &str) -> String {
    response
        .http_headers
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|value| value.strip_prefix(&format!("{name}=")))
        .expect("response should set the auth cookie")
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

fn request(query: &str, token: &str) -> Request {
    Request::new(query).data(AuthCookieContext {
        refresh_token: Some(token.to_string()),
        secure: true,
    })
}

const REFRESH: &str = "mutation { refreshToken { success error tokens { expiresIn } } }";

#[tokio::test]
async fn refresh_session_rotates_persisted_tokens_for_thirty_days() {
    let temp = tempfile::tempdir().unwrap();
    let services = ServicesManager::builder()
        .add_service(DatabaseServiceConfig {
            database_url: format!("sqlite://{}", temp.path().join("auth.db").display()),
            connect_timeout: std::time::Duration::from_secs(5),
        })
        .add_service(AuthConfig::for_tests())
        .start()
        .await
        .unwrap();
    let auth = services.get_auth().await.unwrap();
    let db_service = services.get_database().await.unwrap();
    let db = db_service.pool();
    let registered = auth
        .register(RegisterInput {
            email: "session-test@example.test".into(),
            name: "Session Test".into(),
            password: "test-fixture-password-only".into(),
            invite_token: None,
        })
        .await
        .unwrap();
    // Account creation must issue a StoredUser grant, just like password login.
    auth.refresh_token(&registered.tokens.refresh_token)
        .await
        .expect("a newly registered account must have a refreshable session");
    let schema = Schema::build(TestQuery, AuthMutations, EmptySubscription)
        .data(auth.clone())
        .finish();
    let login = schema.execute(Request::new(r#"mutation {
        login(input: {usernameOrEmail: "session-test@example.test", password: "test-fixture-password-only"}) {
            success tokens { expiresIn }
        }
    }"#).data(AuthCookieContext { refresh_token: None, secure: true })).await;
    assert!(login.errors.is_empty());
    assert_eq!(
        login.data.clone().into_json().unwrap()["login"]["success"],
        true
    );
    let original = cookie(&login, "librarian_refresh_token");
    let refresh_header = login
        .http_headers
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap())
        .find(|value| value.starts_with("librarian_refresh_token="))
        .unwrap();
    for attribute in [
        "Max-Age=2592000",
        "Path=/graphql",
        "HttpOnly",
        "Secure",
        "SameSite=Lax",
    ] {
        assert!(refresh_header.contains(attribute));
    }
    let rows = RefreshToken::query(db.pool()).fetch_all().await.unwrap();
    for row in &rows {
        assert_eq!(
            serde_json::from_str::<agql_auth::RefreshGrantSource>(
                row.grant_source.as_deref().unwrap()
            )
            .unwrap(),
            agql_auth::RefreshGrantSource::StoredUser,
        );
        assert_eq!(
            serde_json::from_str::<Vec<String>>(row.roles.as_deref().unwrap()).unwrap(),
            vec!["admin"]
        );
        assert_eq!(
            serde_json::from_str::<Vec<String>>(row.authorization_roles.as_deref().unwrap())
                .unwrap(),
            Vec::<String>::new()
        );
        // This generated timestamp previously made every refresh fail to deserialize.
        let created = row
            .created_at
            .parse::<i64>()
            .expect("ORM writes Unix seconds");
        let expires = OffsetDateTime::parse(&row.expires_at, &Rfc3339)
            .unwrap()
            .unix_timestamp();
        assert!((expires - created - 30 * 86400).abs() <= 1);
        // Age the persisted session to day 29: one day of refresh validity remains.
        RefreshToken::update_by_id(
            db,
            &row.id,
            UpdateRefreshTokenInput {
                expires_at: Some(
                    (OffsetDateTime::now_utc() + Duration::days(1))
                        .format(&Rfc3339)
                        .unwrap(),
                ),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    }
    // No access cookie/auth context: this is reopening the app after access expiry.
    let renewed = schema.execute(request(REFRESH, &original)).await;
    assert!(renewed.errors.is_empty(), "{:?}", renewed.errors);
    assert_eq!(
        renewed.data.clone().into_json().unwrap()["refreshToken"]["success"],
        true
    );
    let next = cookie(&renewed, "librarian_refresh_token");
    assert_ne!(original, next);
    auth.validate_access_token(&cookie(&renewed, "librarian_access_token"))
        .await
        .unwrap();
    let renewed_again = schema.execute(request(REFRESH, &next)).await;
    assert!(renewed_again.errors.is_empty());
    assert_eq!(
        renewed_again.data.clone().into_json().unwrap()["refreshToken"]["success"],
        true
    );
    let latest = cookie(&renewed_again, "librarian_refresh_token");

    // Replaying a rotated token revokes its family, including its replacement.
    let replay = schema.execute(request(REFRESH, &original)).await;
    assert_eq!(
        replay.data.into_json().unwrap()["refreshToken"]["success"],
        false
    );
    assert!(auth.refresh_token(&latest).await.is_err());

    // A session past its 30-day refresh expiry must require login.
    let fresh = auth
        .login("session-test@example.test", "test-fixture-password-only")
        .await
        .unwrap();
    for row in RefreshToken::query(db.pool()).fetch_all().await.unwrap() {
        if row.revoked_at.is_none() {
            RefreshToken::update_by_id(
                db,
                &row.id,
                UpdateRefreshTokenInput {
                    expires_at: Some(
                        (OffsetDateTime::now_utc() - Duration::seconds(1))
                            .format(&Rfc3339)
                            .unwrap(),
                    ),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }
    }
    let expired = schema
        .execute(request(REFRESH, &fresh.tokens.refresh_token))
        .await;
    assert_eq!(
        expired.data.clone().into_json().unwrap()["refreshToken"]["success"],
        false
    );
    assert!(cookie(&expired, "librarian_refresh_token").is_empty());

    let fresh = auth
        .login("session-test@example.test", "test-fixture-password-only")
        .await
        .unwrap();
    let logout = schema
        .execute(request(
            "mutation { logout { success } }",
            &fresh.tokens.refresh_token,
        ))
        .await;
    assert_eq!(logout.data.into_json().unwrap()["logout"]["success"], true);
    assert!(
        auth.refresh_token(&fresh.tokens.refresh_token)
            .await
            .is_err()
    );

    // Old rows carry no trustworthy grant source. They must reauthenticate;
    // do not guess provenance from the password auth method in session JSON.
    let legacy = auth
        .login("session-test@example.test", "test-fixture-password-only")
        .await
        .unwrap();
    for row in RefreshToken::query(db.pool()).fetch_all().await.unwrap() {
        if row.revoked_at.is_none() {
            RefreshToken::update_by_id(
                db,
                &row.id,
                UpdateRefreshTokenInput {
                    grant_source: Some(None),
                    roles: Some(None),
                    authorization_roles: Some(None),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }
    }
    assert!(
        auth.refresh_token(&legacy.tokens.refresh_token)
            .await
            .is_err()
    );

    // A deliberately narrowed persisted role ceiling must survive rotation
    // even while the current user still has the broader admin authority.
    let privileged = auth
        .login("session-test@example.test", "test-fixture-password-only")
        .await
        .unwrap();
    for row in RefreshToken::query(db.pool()).fetch_all().await.unwrap() {
        if row.revoked_at.is_none() {
            RefreshToken::update_by_id(
                db,
                &row.id,
                UpdateRefreshTokenInput {
                    roles: Some(Some("[]".into())),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }
    }
    let narrowed = auth
        .refresh_token(&privileged.tokens.refresh_token)
        .await
        .unwrap();
    assert!(
        auth.authenticate_access_token(&narrowed.access_token)
            .await
            .unwrap()
            .roles
            .is_empty()
    );
    let still_narrowed = auth.refresh_token(&narrowed.refresh_token).await.unwrap();
    assert!(
        auth.authenticate_access_token(&still_narrowed.access_token)
            .await
            .unwrap()
            .roles
            .is_empty()
    );

    // Librarian additionally revokes sessions immediately on a role change.
    User::update_by_id(
        db,
        &registered.user.id,
        UpdateUserInput {
            role: Some("member".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        auth.refresh_token(&still_narrowed.refresh_token)
            .await
            .is_err()
    );
    assert!(
        auth.authenticate_access_token(&still_narrowed.access_token)
            .await
            .is_err(),
        "demotion must revoke even an already narrowed session"
    );

    // A transient store failure must not delete an otherwise usable refresh cookie.
    let fresh = auth
        .login("session-test@example.test", "test-fixture-password-only")
        .await
        .unwrap();
    db.pool().close().await;
    let unavailable = schema
        .execute(request(REFRESH, &fresh.tokens.refresh_token))
        .await;
    assert!(!unavailable.errors.is_empty());
    assert!(unavailable.http_headers.get("set-cookie").is_none());
    services.stop_all().await.unwrap();
}
