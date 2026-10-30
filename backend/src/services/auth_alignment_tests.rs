use super::*;
use crate::graphql::entities::CreateInviteTokenInput;
use crate::services::{DatabaseServiceConfig, ServicesManager};

async fn fixture() -> (
    tempfile::TempDir,
    Arc<ServicesManager>,
    Database,
    Arc<AuthService>,
    LoginResult,
) {
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
    let db = services.get_database().await.unwrap().pool().clone();
    let admin = auth
        .register(registration("owner@example.test", None))
        .await
        .unwrap();
    (temp, services, db, auth, admin)
}

#[tokio::test]
async fn registration_never_infers_bootstrap_from_the_first_hundred_users() {
    let temp = tempfile::tempdir().unwrap();
    let services = ServicesManager::builder()
        .add_service(DatabaseServiceConfig {
            database_url: format!("sqlite://{}", temp.path().join("many-users.db").display()),
            connect_timeout: std::time::Duration::from_secs(5),
        })
        .add_service(AuthConfig::for_tests())
        .start()
        .await
        .unwrap();
    let auth = services.get_auth().await.unwrap();
    let db = services.get_database().await.unwrap().pool().clone();
    // Legacy/unusable rows may precede the first real administrator. Put that
    // administrator after a full transaction page; another signup must use its
    // invite role rather than silently entering setup mode.
    let mut tx = MutationContext::begin(&db, TransactionMode::StateMachine)
        .await
        .unwrap();
    for index in 0..101 {
        tx.insert::<User>(CreateUserInput {
            username: format!("legacy-{index:03}"),
            email: Some(format!("legacy-{index}@example.test")),
            password_hash: "not-a-password-grant".into(),
            role: "member".into(),
            display_name: None,
            avatar_url: None,
            is_active: true,
            last_login_at: None,
        })
        .await
        .unwrap();
    }
    tx.commit_and_emit().await.unwrap();
    let admin = auth
        .register(registration("zz-admin@example.test", None))
        .await
        .unwrap();
    assert!(!auth.needs_setup().await.unwrap());
    let store = LibrarianAuthStore::new(db.clone());
    assert_eq!(store.users().await.unwrap().len(), 102);
    assert!(
        store
            .find_user_by_principal("zz-admin@example.test")
            .await
            .unwrap()
            .is_some()
    );
    let invite = InviteToken::insert(
        &db,
        CreateInviteTokenInput {
            token: "many-users-member-invite".into(),
            created_by: admin.user.id,
            library_ids: vec![],
            role: "member".into(),
            access_level: "full".into(),
            expires_at: None,
            max_uses: Some(1),
            use_count: 0,
            apply_restrictions: false,
            restrictions_template: None,
            is_active: true,
        },
    )
    .await
    .unwrap();
    let member = auth
        .register(registration("new-member@example.test", Some(invite.token)))
        .await
        .unwrap();
    assert_eq!(member.user.role, "member");
    assert_eq!(
        InviteToken::get(db.pool(), &invite.id)
            .await
            .unwrap()
            .unwrap()
            .use_count,
        1
    );
    services.stop_all().await.unwrap();
}

fn registration(email: &str, invite: Option<String>) -> RegisterInput {
    RegisterInput {
        email: email.into(),
        name: "Fixture".into(),
        password: "test-only-password".into(),
        invite_token: invite,
    }
}

#[tokio::test]
async fn private_refresh_lookup_and_family_revocation_cross_page_boundaries() {
    let (_temp, services, db, _auth, _admin) = fixture().await;
    let original = RefreshToken::query(db.pool())
        .limit(1)
        .fetch_all()
        .await
        .unwrap()
        .remove(0);
    let token = to_stored_refresh_token(original).unwrap();
    let mut tx = MutationContext::begin(&db, TransactionMode::StateMachine)
        .await
        .unwrap();
    for index in 0..101 {
        let mut copy = token.clone();
        copy.id = Uuid::new_v4();
        copy.token_hash = format!("later-refresh-{index}");
        tx.insert::<RefreshToken>(refresh_token_input(copy).unwrap())
            .await
            .unwrap();
    }
    tx.commit_and_emit().await.unwrap();
    let store = LibrarianAuthStore::new(db);
    assert_eq!(store.refresh_tokens().await.unwrap().len(), 102);
    assert!(
        store
            .find_refresh_token_by_hash("later-refresh-100")
            .await
            .unwrap()
            .is_some()
    );
    store
        .revoke_refresh_token_family(
            token.session_family_id,
            OffsetDateTime::now_utc(),
            RefreshTokenRevocationReason::AdminRevoked,
        )
        .await
        .unwrap();
    let revoked = store.refresh_tokens().await.unwrap();
    assert_eq!(revoked.len(), 102);
    assert!(revoked.iter().all(|row| row.revoked_at.is_some()));
    services.stop_all().await.unwrap();
}

#[tokio::test]
async fn logout_of_rotated_cookie_revokes_the_successor_but_not_another_session() {
    let (_temp, services, _db, auth, admin) = fixture().await;
    let independent = auth
        .login("owner@example.test", "test-only-password")
        .await
        .unwrap();
    let rotated = auth
        .refresh_token(&admin.tokens.refresh_token)
        .await
        .unwrap();
    auth.logout(&admin.tokens.refresh_token).await.unwrap();
    assert!(
        auth.authenticate_access_token(&rotated.access_token)
            .await
            .is_err()
    );
    assert!(auth.refresh_token(&rotated.refresh_token).await.is_err());
    assert!(
        auth.authenticate_access_token(&independent.tokens.access_token)
            .await
            .is_ok()
    );
    services.stop_all().await.unwrap();
}

#[tokio::test]
async fn concurrent_family_logout_and_rotation_leave_no_active_successor() {
    let (_temp, services, db, _auth, _admin) = fixture().await;
    let template = to_stored_refresh_token(
        RefreshToken::query(db.pool())
            .fetch_all()
            .await
            .unwrap()
            .remove(0),
    )
    .unwrap();
    let store = LibrarianAuthStore::new(db);
    for iteration in 0..4 {
        let mut original = template.clone();
        original.id = Uuid::new_v4();
        original.session_id = Uuid::new_v4();
        original.session_family_id = Uuid::new_v4();
        original.token_hash = format!("concurrent-original-{iteration}");
        store.insert_refresh_token(original.clone()).await.unwrap();
        let mut replacement = original.clone();
        replacement.id = Uuid::new_v4();
        replacement.token_hash = format!("concurrent-replacement-{iteration}");
        let now = OffsetDateTime::now_utc();
        let logout = store.revoke_refresh_token_family(
            original.session_family_id,
            now,
            RefreshTokenRevocationReason::Logout,
        );
        let rotate = store.rotate_refresh_token(original.id, replacement, now, None, None);
        if iteration % 2 == 0 {
            let (revoked, rotated) = tokio::join!(logout, rotate);
            revoked.unwrap();
            rotated.unwrap();
        } else {
            let (rotated, revoked) = tokio::join!(rotate, logout);
            rotated.unwrap();
            revoked.unwrap();
        }
        let family = store
            .refresh_tokens()
            .await
            .unwrap()
            .into_iter()
            .filter(|row| row.session_family_id == original.session_family_id.to_string())
            .collect::<Vec<_>>();
        assert!(!family.is_empty());
        assert!(
            family.iter().all(|row| row.revoked_at.is_some()),
            "a concurrent refresh successor escaped family logout"
        );
    }
    services.stop_all().await.unwrap();
}

#[tokio::test]
async fn refresh_rotation_has_one_winner_and_rolls_back_failed_replacement() {
    let (_temp, services, db, _auth, _admin) = fixture().await;
    let original = RefreshToken::query(db.pool())
        .fetch_all()
        .await
        .unwrap()
        .remove(0);
    let token = to_stored_refresh_token(original.clone()).unwrap();
    let store = LibrarianAuthStore::new(db.clone());
    // Reusing the current primary key forces replacement insertion to fail.
    assert!(
        store
            .rotate_refresh_token(
                token.id,
                token.clone(),
                OffsetDateTime::now_utc(),
                None,
                None
            )
            .await
            .is_err()
    );
    assert!(
        RefreshToken::get(db.pool(), &original.id)
            .await
            .unwrap()
            .unwrap()
            .revoked_at
            .is_none()
    );
    assert_eq!(
        RefreshToken::query(db.pool())
            .fetch_all()
            .await
            .unwrap()
            .len(),
        1
    );
    let mut first = token.clone();
    first.id = Uuid::new_v4();
    first.token_hash = "replacement-one".into();
    let mut second = token.clone();
    second.id = Uuid::new_v4();
    second.token_hash = "replacement-two".into();
    let now = OffsetDateTime::now_utc();
    let (a, b) = tokio::join!(
        store.rotate_refresh_token(token.id, first, now, None, None),
        store.rotate_refresh_token(token.id, second, now, None, None),
    );
    assert_eq!(usize::from(a.unwrap()) + usize::from(b.unwrap()), 1);
    let rows = RefreshToken::query(db.pool()).fetch_all().await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.iter().filter(|row| row.revoked_at.is_none()).count(),
        1
    );
    services.stop_all().await.unwrap();
}

#[tokio::test]
async fn one_use_invite_is_consumed_with_exactly_one_account_creation() {
    let (_temp, services, db, auth, admin) = fixture().await;
    let invite = InviteToken::insert(
        &db,
        CreateInviteTokenInput {
            token: "one-use-fixture".into(),
            created_by: admin.user.id,
            library_ids: vec![],
            role: "member".into(),
            access_level: "full".into(),
            expires_at: None,
            max_uses: Some(1),
            use_count: 0,
            apply_restrictions: false,
            restrictions_template: None,
            is_active: true,
        },
    )
    .await
    .unwrap();
    // A failed account creation must not consume the invite.
    assert!(
        auth.register(registration(
            "OWNER@example.test",
            Some(invite.token.clone())
        ))
        .await
        .is_err()
    );
    assert_eq!(
        InviteToken::get(db.pool(), &invite.id)
            .await
            .unwrap()
            .unwrap()
            .use_count,
        0
    );
    let (a, b) = tokio::join!(
        auth.register(registration(
            "first@example.test",
            Some(invite.token.clone())
        )),
        auth.register(registration(
            "second@example.test",
            Some(invite.token.clone())
        )),
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert_eq!(User::query(db.pool()).fetch_all().await.unwrap().len(), 2);
    let saved = InviteToken::get(db.pool(), &invite.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.use_count, 1);
    assert!(!saved.is_active);
    services.stop_all().await.unwrap();
}

#[tokio::test]
async fn access_token_admission_observes_logout_and_account_revocation() {
    let (_temp, services, db, auth, admin) = fixture().await;
    let token = admin.tokens.access_token;
    assert!(auth.authenticate_access_token(&token).await.is_ok());
    auth.logout(&admin.tokens.refresh_token).await.unwrap();
    assert!(auth.authenticate_access_token(&token).await.is_err());
    let login = auth
        .login("owner@example.test", "test-only-password")
        .await
        .unwrap();
    User::update_by_id(
        &db,
        &login.user.id,
        UpdateUserInput {
            is_active: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        auth.authenticate_access_token(&login.tokens.access_token)
            .await
            .is_err()
    );
    services.stop_all().await.unwrap();
}
