use super::*;
use crate::graphql::entities::{UpdateUserInput, User};
use crate::services::auth::RegisterInput;
use crate::services::{AuthConfig, DatabaseServiceConfig};
use async_graphql::{Context, Data, EmptySubscription, Object, Request, Schema};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Query;
#[Object]
impl Query {
    async fn ping(&self) -> bool {
        true
    }
}
struct Actions;
#[Object]
impl Actions {
    async fn change(&self, ctx: &Context<'_>) -> bool {
        ctx.data_unchecked::<Arc<AtomicUsize>>()
            .fetch_add(1, Ordering::SeqCst);
        true
    }
}

#[tokio::test]
async fn stale_socket_authority_is_rejected_before_mutation_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let services = ServicesManager::builder()
        .add_service(DatabaseServiceConfig {
            database_url: format!("sqlite://{}", temp.path().join("sessions.db").display()),
            connect_timeout: std::time::Duration::from_secs(5),
        })
        .add_service(AuthConfig::for_tests())
        .start()
        .await
        .unwrap();
    let auth = services.get_auth().await.unwrap();
    let db = services.get_database().await.unwrap().pool().clone();
    let login = auth
        .register(RegisterInput {
            email: "guard@example.test".into(),
            name: "Guard".into(),
            password: "fixture-only-password".into(),
            invite_token: None,
        })
        .await
        .unwrap();
    let authenticated = auth
        .authenticate_access_token(&login.tokens.access_token)
        .await
        .unwrap();
    let effects = Arc::new(AtomicUsize::new(0));
    let schema = Schema::build(Query, Actions, EmptySubscription)
        .extension(SessionDeliveryGuard)
        .data(services.clone())
        .data(effects.clone())
        .finish();
    let socket_data = |user: agql_auth::AuthUser| {
        let mut data = Data::default();
        let authority = SessionAuthority(user.clone());
        data.insert(authority.subject());
        data.insert(authority);
        data.insert(crate::graphql::AuthUser::from(user));
        Arc::new(data)
    };
    let valid = schema
        .execute_stream_with_session_data("mutation { change }", socket_data(authenticated.clone()))
        .next()
        .await
        .unwrap();
    assert!(valid.errors.is_empty());
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    auth.logout(&login.tokens.refresh_token).await.unwrap();
    let rejected = schema
        .execute_stream_with_session_data("mutation { change }", socket_data(authenticated))
        .next()
        .await
        .unwrap();
    assert!(!rejected.errors.is_empty());
    assert_eq!(effects.load(Ordering::SeqCst), 1);

    let login = auth
        .login("guard@example.test", "fixture-only-password")
        .await
        .unwrap();
    let authenticated = auth
        .authenticate_access_token(&login.tokens.access_token)
        .await
        .unwrap();
    User::update_by_id(
        &db,
        &login.user.id,
        UpdateUserInput {
            role: Some("member".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let rejected = schema
        .execute_stream_with_session_data("mutation { change }", socket_data(authenticated))
        .next()
        .await
        .unwrap();
    assert!(!rejected.errors.is_empty());
    assert_eq!(effects.load(Ordering::SeqCst), 1);

    let login = auth
        .login("guard@example.test", "fixture-only-password")
        .await
        .unwrap();
    let mut expired = auth
        .authenticate_access_token(&login.tokens.access_token)
        .await
        .unwrap();
    expired.token_claims.expires_at =
        Some(time::OffsetDateTime::now_utc() - time::Duration::seconds(1));
    // Also exercise HTTP/request-scoped authority: prepare_request cannot see
    // Request.data yet, so the execute guard must reject before dispatch.
    let response = schema
        .execute(Request::new("mutation { change }").data(SessionAuthority(expired)))
        .await;
    assert!(!response.errors.is_empty());
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    services.stop_all().await.unwrap();
}
