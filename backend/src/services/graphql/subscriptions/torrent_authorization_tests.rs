//! Test the actual authorization stream with generated entities and in-memory
//! channels, without starting TorrentService, GraphQL services, or a server.
use super::*;
use crate::graphql::entities::{UpdateTorrentInput, UpdateUserInput, User};
use crate::services::graphql::entities::torrent::authorization_tests::{account, fixture, torrent};
use crate::services::torrent::TorrentState;

fn events(hash: &str, id: usize) -> Vec<TorrentEvent> {
    vec![
        TorrentEvent::Added {
            id,
            info_hash: hash.into(),
            name: "live".into(),
        },
        TorrentEvent::Progress {
            id,
            info_hash: hash.into(),
            progress: 0.876,
            download_speed: 9876,
            upload_speed: 12,
            peers: 2,
            state: TorrentState::Downloading,
        },
        TorrentEvent::Completed {
            id,
            info_hash: hash.into(),
            name: "live".into(),
        },
        TorrentEvent::Removed {
            id,
            info_hash: hash.into(),
        },
    ]
}

fn stream(db: &Database, user: &AuthUser) -> (broadcast::Sender<TorrentEvent>, RuntimeStream) {
    let (tx, rx) = broadcast::channel(32);
    let changes = db.ensure_event_sender::<TorrentWitness>().subscribe();
    (
        tx,
        authorized_runtime_events(db.clone(), user.clone(), rx, changes),
    )
}

#[tokio::test]
async fn every_runtime_event_is_owner_scoped_and_retains_runtime_freshness() {
    let db = fixture().await;
    let owner = account(&db, "member").await;
    let other = account(&db, "member").await;
    torrent(&db, &owner, "own").await;
    torrent(&db, &other, "foreign").await;
    let (tx, mut stream) = stream(&db, &owner);
    for hash in ["foreign", "unmapped", "own"] {
        for event in events(hash, 9) {
            tx.send(event).unwrap();
        }
    }
    drop(tx);
    let mut delivered = Vec::new();
    while let Some(event) = stream.next().await {
        assert_eq!(event_hash(&event), "own");
        if let TorrentEvent::Progress {
            progress,
            download_speed,
            ..
        } = &event
        {
            assert_eq!(*progress, 0.876);
            assert_eq!(*download_speed, 9876);
        }
        delivered.push(event);
    }
    assert_eq!(delivered.len(), 4);
}

#[tokio::test]
async fn removal_uses_generated_delete_preimage_for_only_the_deleted_owner() {
    let db = fixture().await;
    let owner = account(&db, "member").await;
    let other = account(&db, "member").await;
    let own = torrent(&db, &owner, "own").await;
    let foreign = torrent(&db, &other, "foreign").await;
    let (tx, mut stream) = stream(&db, &owner);
    // Both independent channels can buffer multiple deletions before polling.
    Torrent::delete_by_id(&db, &foreign.id).await.unwrap();
    Torrent::delete_by_id(&db, &own.id).await.unwrap();
    tx.send(TorrentEvent::Removed {
        id: 1,
        info_hash: "foreign".into(),
    })
    .unwrap();
    tx.send(TorrentEvent::Removed {
        id: 2,
        info_hash: "own".into(),
    })
    .unwrap();
    // A delete preimage authorizes removal only, never subsequent progress.
    tx.send(events("own", 2).remove(1)).unwrap();
    drop(tx);
    assert!(matches!(
        stream.next().await,
        Some(TorrentEvent::Removed { id: 2, .. })
    ));
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn raw_observer_event_cannot_manufacture_removal_provenance() {
    let db = fixture().await;
    let owner = account(&db, "member").await;
    let row = torrent(&db, &owner, "deleted-before-subscription").await;
    let mut observer = db.ensure_event_sender::<TorrentChangedEvent>().subscribe();
    Torrent::delete_by_id(&db, &row.id).await.unwrap();
    let raw = observer.recv().await.unwrap();
    let (tx, mut stream) = stream(&db, &owner);
    db.emit_event(raw); // The raw channel is deliberately not consumed.
    tx.send(TorrentEvent::Removed {
        id: 1,
        info_hash: row.info_hash,
    })
    .unwrap();
    drop(tx);
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn hash_reuse_and_ownership_changes_override_old_delete_provenance() {
    let db = fixture().await;
    let owner = account(&db, "member").await;
    let other = account(&db, "member").await;
    let old = torrent(&db, &owner, "reused").await;
    let reassigned = torrent(&db, &owner, "reassigned").await;
    let (tx, mut stream) = stream(&db, &owner);
    Torrent::delete_by_id(&db, &old.id).await.unwrap();
    torrent(&db, &other, "reused").await;
    Torrent::update_by_id(
        &db,
        &reassigned.id,
        UpdateTorrentInput {
            user_id: Some(other.user_id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    tx.send(TorrentEvent::Removed {
        id: 1,
        info_hash: "reused".into(),
    })
    .unwrap();
    tx.send(events("reassigned", 2).remove(1)).unwrap();
    drop(tx);
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn stream_rechecks_admin_demotion_and_account_disable() {
    let db = fixture().await;
    let admin = account(&db, "admin").await;
    let member = account(&db, "member").await;
    torrent(&db, &member, "foreign").await;
    torrent(&db, &admin, "own").await;
    let (tx, mut stream) = stream(&db, &admin);
    User::update_by_id(
        &db,
        &admin.user_id,
        UpdateUserInput {
            role: Some("member".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    tx.send(events("foreign", 1).remove(1)).unwrap();
    tx.send(events("own", 2).remove(1)).unwrap();
    assert!(matches!(
        stream.next().await,
        Some(TorrentEvent::Progress { id: 2, .. })
    ));
    User::update_by_id(
        &db,
        &admin.user_id,
        UpdateUserInput {
            is_active: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    tx.send(events("own", 2).remove(1)).unwrap();
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn active_admin_intentionally_sees_unmapped_runtime_events() {
    let db = fixture().await;
    let admin = account(&db, "admin").await;
    let (tx, mut stream) = stream(&db, &admin);
    for event in events("unmapped", 3) {
        tx.send(event).unwrap();
    }
    drop(tx);
    let mut count = 0;
    while stream.next().await.is_some() {
        count += 1;
    }
    assert_eq!(count, 4);
}

#[tokio::test]
async fn lagged_runtime_stream_fails_closed() {
    let db = fixture().await;
    let owner = account(&db, "member").await;
    let (tx, rx) = broadcast::channel(1);
    let changes = db.ensure_event_sender::<TorrentWitness>().subscribe();
    let mut stream = authorized_runtime_events(db, owner, rx, changes);
    tx.send(events("first", 1).remove(1)).unwrap();
    tx.send(events("second", 2).remove(1)).unwrap();
    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn untrusted_or_mismatched_witnesses_do_not_authorize_deleted_rows() {
    let db = fixture().await;
    let owner = account(&db, "member").await;
    let row = torrent(&db, &owner, "own").await;
    let mut changes = db.ensure_event_sender::<TorrentWitness>().subscribe();
    Torrent::delete_by_id(&db, &row.id).await.unwrap();
    let witness = changes.recv().await.unwrap();
    assert_eq!(trusted_deleted_torrent(&witness).unwrap().id, row.id);
    let mut untrusted = witness.clone();
    untrusted.trusted_delete = false;
    assert!(trusted_deleted_torrent(&untrusted).is_none());
    let mut wrong_key = witness.clone();
    wrong_key.keys = Torrent::__gom_key_values(&"another-id".to_string());
    assert!(trusted_deleted_torrent(&wrong_key).is_none());
    let mut wrong_kind = witness;
    Arc::make_mut(&mut wrong_kind.event).change_kind = ChangeKind::Propagated;
    assert!(trusted_deleted_torrent(&wrong_kind).is_none());
}
