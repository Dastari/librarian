//! Exercise the narrow mutable-link policy and its generated GraphQL update
//! path, including real database hooks, without servers or application SQL.
use super::*;
use crate::graphql::entities::{
    CreateLibraryInput, Library, TorrentMutations, TorrentQueries, UpdateLibraryInput,
    UpdateTorrentInput,
};
use crate::services::database::DatabaseService;
use crate::services::graphql::entities::torrent::authorization_tests::{account, fixture, torrent};
use async_graphql::{EmptySubscription, Request, Schema, Variables};
use graphql_orm::graphql::orm::{MutationContext, TransactionMode};
use serde_json::json;
use std::time::Duration;

// The shared in-memory fixture has one connection. Hold it exactly as the
// generated update does; both the field gate and invariant must avoid pool reads.
async fn validate_link_on_transaction(
    tx: &mut MutationContext<'_>,
    user: Option<&AuthUser>,
    entity_name: &'static str,
    field_name: &str,
    record: Option<&(dyn std::any::Any + Send + Sync)>,
    value: Option<&(dyn std::any::Any + Send + Sync)>,
) -> async_graphql::Result<bool> {
    tokio::time::timeout(Duration::from_secs(5), async {
        AppFieldPolicy::mutable_torrent_library(user, entity_name, field_name, record, value)?;
        let mut write_ctx = WriteInputContext::repository_in_transaction(entity_name, tx);
        AppFieldPolicy::validate_mutable_torrent_library(&mut write_ctx, user, record, value)
            .await?;
        Ok(true)
    })
    .await
    .expect("mutable-link validation must use the held transaction, not re-enter its pool")
}

async fn validate_link(
    db: &Database,
    user: Option<&AuthUser>,
    entity_name: &'static str,
    field_name: &str,
    record: Option<&(dyn std::any::Any + Send + Sync)>,
    value: Option<&(dyn std::any::Any + Send + Sync)>,
) -> async_graphql::Result<bool> {
    let mut tx = MutationContext::begin(db, TransactionMode::StateMachine)
        .await
        .unwrap();
    let result =
        validate_link_on_transaction(&mut tx, user, entity_name, field_name, record, value).await;
    tx.rollback().await.unwrap();
    result
}

async fn library(db: &Database, owner: &AuthUser, name: &str) -> Library {
    Library::insert(
        db,
        CreateLibraryInput {
            user_id: owner.user_id.clone(),
            name: name.to_string(),
            path: format!("/fixture/{name}"),
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
    .unwrap()
}

fn assert_forbidden(result: async_graphql::Result<bool>) {
    let error = result.expect_err("a rejected nullable write must be an error, never Ok(false)");
    assert_eq!(
        error
            .extensions
            .as_ref()
            .and_then(|extensions| extensions.get("code")),
        Some(&async_graphql::Value::from("FORBIDDEN")),
    );
}

#[tokio::test]
async fn member_can_link_relink_and_explicitly_unlink_only_its_persisted_torrent() {
    let db = fixture().await;
    let member = account(&db, "member").await;
    let row = torrent(&db, &member, "mutable-own").await;
    for target in [
        Some(library(&db, &member, "first").await.id),
        Some(library(&db, &member, "second").await.id),
        None,
    ] {
        assert!(
            validate_link(
                &db,
                Some(&member),
                "Torrent",
                "libraryId",
                Some(&row),
                Some(&target),
            )
            .await
            .unwrap()
        );
    }
}

#[tokio::test]
async fn foreign_library_and_foreign_torrent_including_unlink_fail_with_forbidden() {
    let db = fixture().await;
    let member = account(&db, "member").await;
    let other = account(&db, "member").await;
    let own = torrent(&db, &member, "own-link").await;
    let foreign = torrent(&db, &other, "foreign-link").await;
    let own_library = Some(library(&db, &member, "own").await.id);
    let foreign_library = Some(library(&db, &other, "foreign").await.id);
    assert_forbidden(
        validate_link(
            &db,
            Some(&member),
            "Torrent",
            "libraryId",
            Some(&own),
            Some(&foreign_library),
        )
        .await,
    );
    assert_forbidden(
        validate_link(
            &db,
            Some(&member),
            "Torrent",
            "libraryId",
            Some(&foreign),
            Some(&own_library),
        )
        .await,
    );
    assert_forbidden(
        validate_link(
            &db,
            Some(&member),
            "Torrent",
            "libraryId",
            Some(&foreign),
            Some(&None::<String>),
        )
        .await,
    );
}

#[tokio::test]
async fn mutable_link_does_not_trust_a_stale_owner_or_a_missing_row() {
    let db = fixture().await;
    let member = account(&db, "member").await;
    let other = account(&db, "member").await;
    let stale = torrent(&db, &member, "stale-link").await;
    let target = Some(library(&db, &member, "stale-target").await.id);
    Torrent::update_by_id(
        &db,
        &stale.id,
        UpdateTorrentInput {
            user_id: Some(other.user_id),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_forbidden(
        validate_link(
            &db,
            Some(&member),
            "Torrent",
            "libraryId",
            Some(&stale),
            Some(&target),
        )
        .await,
    );
    Torrent::delete_by_id(&db, &stale.id).await.unwrap();
    assert_forbidden(
        validate_link(
            &db,
            Some(&member),
            "Torrent",
            "libraryId",
            Some(&stale),
            Some(&None::<String>),
        )
        .await,
    );
}

#[tokio::test]
async fn mutable_link_reads_uncommitted_torrent_and_destination_ownership() {
    let db = fixture().await;
    let member = account(&db, "member").await;
    let other = account(&db, "member").await;
    let row = torrent(&db, &member, "transaction-local-link").await;
    let destination = library(&db, &member, "transaction-local-destination").await;
    let target = Some(destination.id.clone());
    let mut tx = MutationContext::begin(&db, TransactionMode::StateMachine)
        .await
        .unwrap();

    // The supplied snapshots remain owned, but the transaction has new owners.
    // A pool read would either deadlock or miss these uncommitted changes.
    tx.update_by_id::<Library>(
        &destination.id,
        UpdateLibraryInput {
            user_id: Some(other.user_id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_forbidden(
        validate_link_on_transaction(
            &mut tx,
            Some(&member),
            "Torrent",
            "libraryId",
            Some(&row),
            Some(&target),
        )
        .await,
    );
    tx.update_by_id::<Library>(
        &destination.id,
        UpdateLibraryInput {
            user_id: Some(member.user_id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        validate_link_on_transaction(
            &mut tx,
            Some(&member),
            "Torrent",
            "libraryId",
            Some(&row),
            Some(&target),
        )
        .await
        .unwrap()
    );
    tx.update_by_id::<Torrent>(
        &row.id,
        UpdateTorrentInput {
            user_id: Some(other.user_id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    for target in [target, None] {
        assert_forbidden(
            validate_link_on_transaction(
                &mut tx,
                Some(&member),
                "Torrent",
                "libraryId",
                Some(&row),
                Some(&target),
            )
            .await,
        );
    }
    tx.delete_by_id::<Torrent>(&row.id).await.unwrap();
    assert_forbidden(
        validate_link_on_transaction(
            &mut tx,
            Some(&member),
            "Torrent",
            "libraryId",
            Some(&row),
            Some(&None::<String>),
        )
        .await,
    );
    tx.rollback().await.unwrap();
    assert_eq!(
        Torrent::get(db.pool(), &row.id)
            .await
            .unwrap()
            .unwrap()
            .user_id,
        member.user_id
    );
    assert_eq!(
        Library::get(db.pool(), &destination.id)
            .await
            .unwrap()
            .unwrap()
            .user_id,
        member.user_id
    );
}

#[tokio::test]
async fn missing_library_record_or_typed_value_and_wrong_policy_scope_are_denied() {
    let db = fixture().await;
    let member = account(&db, "member").await;
    let row = torrent(&db, &member, "bad-link").await;
    let missing = Some("missing-library".to_string());
    assert_forbidden(
        validate_link(
            &db,
            Some(&member),
            "Torrent",
            "libraryId",
            Some(&row),
            Some(&missing),
        )
        .await,
    );
    assert_forbidden(
        validate_link(
            &db,
            Some(&member),
            "Torrent",
            "libraryId",
            None,
            Some(&None::<String>),
        )
        .await,
    );
    assert_forbidden(
        validate_link(&db, Some(&member), "Torrent", "libraryId", Some(&row), None).await,
    );
    assert_forbidden(
        validate_link(
            &db,
            Some(&member),
            "Torrent",
            "libraryId",
            Some(&row),
            Some(&"untyped-string".to_string()),
        )
        .await,
    );
    assert_forbidden(
        validate_link(
            &db,
            Some(&member),
            "Library",
            "libraryId",
            Some(&row),
            Some(&None::<String>),
        )
        .await,
    );
    assert_forbidden(
        validate_link(
            &db,
            None,
            "Torrent",
            "libraryId",
            Some(&row),
            Some(&None::<String>),
        )
        .await,
    );
    assert_forbidden(
        validate_link(
            &db,
            Some(&member),
            "Torrent",
            "movieId",
            Some(&row),
            Some(&None::<String>),
        )
        .await,
    );
    let no_role = AuthUser {
        role: None,
        ..member.clone()
    };
    assert_forbidden(
        validate_link(
            &db,
            Some(&no_role),
            "Torrent",
            "libraryId",
            Some(&row),
            Some(&None::<String>),
        )
        .await,
    );
}

#[tokio::test]
async fn explicit_admin_authority_can_maintain_another_owners_approved_link() {
    let db = fixture().await;
    let admin = account(&db, "admin").await;
    let owner = account(&db, "member").await;
    let row = torrent(&db, &owner, "admin-link").await;
    let target = Some(library(&db, &owner, "approved-owner").await.id);
    assert!(
        validate_link(
            &db,
            Some(&admin),
            "Torrent",
            "libraryId",
            Some(&row),
            Some(&target),
        )
        .await
        .unwrap()
    );
    assert!(
        validate_link(
            &db,
            Some(&admin),
            "Torrent",
            "libraryId",
            Some(&row),
            Some(&None::<String>),
        )
        .await
        .unwrap()
    );
}

#[tokio::test]
async fn generated_update_persists_link_and_unlink_and_rejects_foreign_link_atomically() {
    let db = DatabaseService::new(fixture().await).pool().clone();
    let member = account(&db, "member").await;
    let other = account(&db, "member").await;
    let admin = account(&db, "admin").await;
    let row = torrent(&db, &member, "graphql-mutable-link").await;
    let foreign_row = torrent(&db, &other, "graphql-foreign-torrent").await;
    let first = library(&db, &member, "graphql-first").await;
    let second = library(&db, &member, "graphql-second").await;
    let foreign = library(&db, &other, "graphql-foreign").await;
    let schema = Schema::build(TorrentQueries, TorrentMutations, EmptySubscription)
        .data(db.clone())
        .finish();
    let document = "mutation($id:String!,$input:UpdateTorrentInput!){updateTorrent(id:$id,input:$input){success error torrent{id libraryId userId}}}";
    let request = |actor: &AuthUser, id: &str, input: serde_json::Value| {
        Request::new(document)
            .variables(Variables::from_json(json!({ "id": id, "input": input })))
            .data(actor.clone())
            .data(graphql_orm::graphql::auth::AuthSubject::from_parts(
                actor.user_id.clone(),
                vec![actor.role.clone().unwrap()],
                Vec::new(),
                None,
            ))
    };
    for target in [
        Some(first.id.clone()),
        Some(second.id.clone()),
        None,
        Some(second.id.clone()),
    ] {
        let response = tokio::time::timeout(
            Duration::from_secs(5),
            schema.execute(request(&member, &row.id, json!({ "libraryId": target }))),
        )
        .await
        .expect("generated update must not re-enter its transaction's single-connection pool");
        assert!(response.errors.is_empty(), "{response:?}");
        let data = response.data.into_json().unwrap();
        assert_eq!(data["updateTorrent"]["success"], true, "{data}");
        assert_eq!(data["updateTorrent"]["torrent"]["libraryId"], json!(target));
        let current = Torrent::get(db.pool(), &row.id).await.unwrap().unwrap();
        assert_eq!(current.library_id, target);
        assert_eq!(current.user_id, member.user_id);
    }

    // A previously owned destination is not an ownership proof after transfer.
    Library::update_by_id(
        &db,
        &first.id,
        UpdateLibraryInput {
            user_id: Some(other.user_id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    for target in [
        foreign.id.clone(),
        first.id.clone(),
        "missing-library".into(),
    ] {
        let response = tokio::time::timeout(
            Duration::from_secs(5),
            schema.execute(request(
                &member,
                &row.id,
                json!({ "libraryId": target, "name": "must not commit" }),
            )),
        )
        .await
        .expect("rejected generated update must not re-enter its transaction's pool");
        assert_response_forbidden(&response);
        let current = Torrent::get(db.pool(), &row.id).await.unwrap().unwrap();
        assert_eq!(current.library_id, Some(second.id.clone()));
        assert_eq!(current.name, row.name);
        assert_eq!(current.user_id, member.user_id);
    }
    for target in [Some(second.id.clone()), None] {
        let response = tokio::time::timeout(
            Duration::from_secs(5),
            schema.execute(request(
                &member,
                &foreign_row.id,
                json!({ "libraryId": target, "name": "must not commit" }),
            )),
        )
        .await
        .expect("foreign torrent update must fail without pool re-entry");
        assert_response_forbidden(&response);
        let current = Torrent::get(db.pool(), &foreign_row.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(current.library_id, None);
        assert_eq!(current.name, foreign_row.name);
        assert_eq!(current.user_id, other.user_id);
    }

    // Administrator authority retains the explicit override, including across
    // owners, while the torrent's owner remains unchanged.
    for target in [Some(foreign.id.clone()), None] {
        let response = tokio::time::timeout(
            Duration::from_secs(5),
            schema.execute(request(&admin, &row.id, json!({ "libraryId": target }))),
        )
        .await
        .expect("administrator update must use the generated transaction");
        assert!(response.errors.is_empty(), "{response:?}");
        let data = response.data.into_json().unwrap();
        assert_eq!(data["updateTorrent"]["success"], true, "{data}");
        let current = Torrent::get(db.pool(), &row.id).await.unwrap().unwrap();
        assert_eq!(current.library_id, target);
        assert_eq!(current.user_id, member.user_id);
        assert_eq!(current.name, row.name);
    }
}

fn assert_response_forbidden(response: &async_graphql::Response) {
    assert!(
        response.errors.iter().any(|error| error
            .extensions
            .as_ref()
            .and_then(|extensions| extensions.get("code"))
            == Some(&async_graphql::Value::from("FORBIDDEN"))),
        "{response:?}"
    );
}
