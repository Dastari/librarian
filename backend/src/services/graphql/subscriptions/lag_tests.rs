//! Exercise a real generated subscription after overflowing its bounded channel.
//! No fabricated events, sockets or application SQL; normal runtime builds only.
use async_graphql::{EmptyMutation, Request, Schema, Variables};
use futures::{FutureExt, StreamExt};
use graphql_orm::graphql::auth::AuthSubject;
use graphql_orm::graphql::subscriptions::SubscriptionEvent;
use serde_json::json;

use crate::graphql::entities::{
    CreateLibraryInput, Library, LibraryChangedEvent, LibraryQueries, LibrarySubscriptions,
    UpdateLibraryInput,
};
use crate::services::database::DatabaseService;
use crate::services::graphql::entities::torrent::authorization_tests::{account, fixture};

#[tokio::test]
async fn generated_subscription_reports_lag_then_reconciles_and_delivers_again() {
    let db = DatabaseService::new(fixture().await).pool().clone();
    let owner = account(&db, "member").await;
    let library = Library::insert(
        &db,
        CreateLibraryInput {
            user_id: owner.user_id.clone(),
            name: "Before burst".into(),
            path: "/fixture/library".into(),
            library_type: "movies".into(),
            icon: None,
            color: None,
            auto_scan: false,
            auto_organize: false,
            naming_pattern: String::new(),
            scan_interval_minutes: 60,
            watch_for_changes: false,
            scanning: false,
            last_scanned_at: None,
            quality_profile_id: None,
        },
    )
    .await
    .unwrap();
    // Use the public channel registration hook, then publish only through real
    // generated writes. A tiny buffer makes overflow deterministic and cheap.
    let (sender, _) = tokio::sync::broadcast::channel::<SubscriptionEvent<LibraryChangedEvent>>(2);
    db.register_event_sender(sender);
    let schema = Schema::build(LibraryQueries, EmptyMutation, LibrarySubscriptions)
        .data(db.clone())
        .finish();
    let request = |document: &str| {
        Request::new(document)
            .data(owner.clone())
            .data(owner.user_id.clone())
            .data(AuthSubject::from_parts(
                owner.user_id.clone(),
                vec!["member".into()],
                vec![],
                None,
            ))
    };
    let subscription = "subscription { libraryChanged { id action } }";
    let mut stream = Box::pin(schema.execute_stream(request(subscription)));
    assert!(
        stream.next().now_or_never().is_none(),
        "listener should be pending before writes"
    );
    for index in 0..5 {
        Library::update_by_id(
            &db,
            &library.id,
            UpdateLibraryInput {
                name: Some(format!("Burst {index}")),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    }
    let response = tokio::time::timeout(std::time::Duration::from_secs(2), stream.next())
        .await
        .unwrap()
        .unwrap();
    assert!(
        response.errors.iter().any(|error| error
            .extensions
            .as_ref()
            .and_then(|extensions| extensions.get("code"))
            == Some(&async_graphql::Value::from("LIVE_RESYNC_REQUIRED"))),
        "{response:?}"
    );
    drop(stream);

    // The recovery contract is a new authorized snapshot, followed by a new
    // listener; replaying the dropped burst is neither required nor promised.
    let snapshot = schema
        .execute(
            request("query($id:String!){library(id:$id){id name}}")
                .variables(Variables::from_json(json!({"id":library.id}))),
        )
        .await;
    assert!(snapshot.errors.is_empty(), "{snapshot:?}");
    assert_eq!(
        snapshot.data.into_json().unwrap()["library"]["name"],
        "Burst 4"
    );
    let mut resumed = Box::pin(schema.execute_stream(request(subscription)));
    assert!(resumed.next().now_or_never().is_none());
    Library::update_by_id(
        &db,
        &library.id,
        UpdateLibraryInput {
            name: Some("After reconciliation".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let response = tokio::time::timeout(std::time::Duration::from_secs(2), resumed.next())
        .await
        .unwrap()
        .unwrap();
    assert!(response.errors.is_empty(), "{response:?}");
    assert_eq!(
        response.data.into_json().unwrap()["libraryChanged"]["id"],
        library.id
    );
}
