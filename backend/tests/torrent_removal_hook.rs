//! `deleteTorrent` is the only way to remove a torrent: its lifecycle hooks stop the
//! transfer after commit, optionally delete the payload, and refuse (while the row is still
//! intact) to delete files that an import is reading. These tests drive the generated
//! mutations over HTTP against a real librqbit session.
//!
//! The service has no "add from file" entry point (a magnet needs peers), so each test
//! seeds a Json-persisted librqbit session folder with complete payloads and then starts
//! the service on it, exactly as a restart would restore them.

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use common::{GqlClient, TestApp, TestAppOptions};
use librarian::services::manager::Service;
use librarian::services::torrent::TorrentService;
use librqbit::spawn_utils::BlockingSpawner;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, CreateTorrentOptions, Session,
    SessionOptions, SessionPersistenceConfig, create_torrent,
};
use serde_json::{Value, json};

const CREATE_TORRENT: &str = r#"mutation C($input: CreateTorrentInput!) {
  createTorrent(input: $input) { success error torrent { id infoHash deleteFilesOnRemove } }
}"#;
const UPDATE_TORRENT: &str = r#"mutation U($id: String!, $input: UpdateTorrentInput!) {
  updateTorrent(id: $id, input: $input) { success error }
}"#;
const DELETE_TORRENT: &str = r#"mutation D($id: String!) {
  deleteTorrent(id: $id) { success error }
}"#;
const DELETE_TORRENTS: &str = r#"mutation D($where: TorrentWhereInput) {
  deleteTorrents(where: $where) { success error }
}"#;

struct Seeded {
    hash: String,
    payload: PathBuf,
    id: String,
}

/// Put complete single-file torrents into the app's persisted session folder, restart the
/// torrent service on it, and create each torrent's database row.
async fn seed(app: &TestApp, admin: &mut GqlClient, names: &[&str]) -> Vec<Seeded> {
    let service = app.services.get_torrent().await.expect("torrent service");
    service.stop().await.expect("stop torrent service");

    let downloads = app.path("downloads");
    let standalone = Session::new_with_opts(
        downloads.clone(),
        SessionOptions {
            dht: None,
            listen: None,
            persistence: Some(SessionPersistenceConfig::Json {
                folder: Some(app.path("session")),
            }),
            ..Default::default()
        },
    )
    .await
    .expect("standalone session");

    let mut seeded = Vec::new();
    for name in names {
        let payload = downloads.join(format!("{name}.bin"));
        std::fs::write(&payload, name.repeat(40_000)).expect("write payload");
        let torrent = create_torrent(
            &payload,
            CreateTorrentOptions {
                name: None,
                trackers: Vec::new(),
                piece_length: Some(16 * 1024),
            },
            &BlockingSpawner::new(1),
        )
        .await
        .expect("create torrent");
        let bytes = torrent.as_bytes().expect("torrent bytes");
        let AddTorrentResponse::Added(_, handle) = standalone
            .add_torrent(
                AddTorrent::from_bytes(bytes),
                Some(AddTorrentOptions {
                    overwrite: true,
                    paused: true,
                    disable_trackers: true,
                    ..Default::default()
                }),
            )
            .await
            .expect("add torrent")
        else {
            panic!("expected a new torrent");
        };
        let hash = handle.info_hash().as_string();
        let admin_id = admin.user_id.clone().expect("admin id");
        let created = admin
            .query(
                CREATE_TORRENT,
                json!({ "input": {
                    "userId": admin_id,
                    "infoHash": hash,
                    "name": name,
                    "state": "paused",
                    "progress": 1.0,
                    "totalBytes": 1,
                    "downloadedBytes": 1,
                    "uploadedBytes": 0,
                    "uploadedBytesTotal": 0,
                    "deleteFilesOnRemove": false,
                    "savePath": downloads.to_string_lossy(),
                    "excludedFiles": [],
                    "addedAt": "2026-11-01T00:00:00Z",
                }}),
            )
            .await;
        assert_eq!(created["createTorrent"]["success"], true, "{created}");
        assert_eq!(
            created["createTorrent"]["torrent"]["deleteFilesOnRemove"],
            false
        );
        seeded.push(Seeded {
            hash,
            payload,
            id: created["createTorrent"]["torrent"]["id"]
                .as_str()
                .expect("torrent id")
                .to_string(),
        });
    }
    standalone.stop().await;

    service.start().await.expect("restart torrent service");
    for item in &seeded {
        wait_for_live(&service, &item.hash, true).await;
    }
    seeded
}

async fn live_hashes(service: &TorrentService) -> Vec<String> {
    service
        .list_torrents()
        .await
        .expect("list torrents")
        .into_iter()
        .map(|info| info.info_hash)
        .collect()
}

async fn wait_for_live(service: &Arc<TorrentService>, hash: &str, present: bool) {
    common::wait_for(
        Duration::from_secs(30),
        &format!("torrent {hash} live={present}"),
        || async {
            (live_hashes(service).await.iter().any(|h| h == hash) == present).then_some(())
        },
    )
    .await;
}

async fn row_exists(app: &TestApp, admin: &mut GqlClient, id: &str) -> bool {
    let _ = app;
    let data = admin
        .query(
            "query($id: String!) { torrent(id: $id) { id } }",
            json!({ "id": id }),
        )
        .await;
    !data["torrent"].is_null()
}

async fn set_delete_files(admin: &mut GqlClient, id: &str, value: bool) {
    let updated = admin
        .query(
            UPDATE_TORRENT,
            json!({ "id": id, "input": { "deleteFilesOnRemove": value } }),
        )
        .await;
    assert_eq!(updated["updateTorrent"]["success"], true, "{updated}");
}

fn exists(path: &Path) -> bool {
    path.exists()
}

#[test]
fn delete_removes_the_row_and_stops_the_transfer_but_keeps_files_by_default() {
    common::run_app_test(|| async {
        let app = TestApp::start_with(TestAppOptions {
            with_torrent: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let service = app.services.get_torrent().await.unwrap();
        let items = seed(&app, &mut admin, &["keep"]).await;
        let item = &items[0];

        let deleted = admin.query(DELETE_TORRENT, json!({ "id": item.id })).await;
        assert_eq!(deleted["deleteTorrent"]["success"], true, "{deleted}");

        wait_for_live(&service, &item.hash, false).await;
        assert!(!row_exists(&app, &mut admin, &item.id).await);
        assert!(exists(&item.payload), "files stay unless the flag is set");
        app.shutdown().await;
    });
}

#[test]
fn delete_with_the_flag_deletes_the_payload_too() {
    common::run_app_test(|| async {
        let app = TestApp::start_with(TestAppOptions {
            with_torrent: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let service = app.services.get_torrent().await.unwrap();
        let items = seed(&app, &mut admin, &["wipe"]).await;
        let item = &items[0];

        set_delete_files(&mut admin, &item.id, true).await;
        let deleted = admin.query(DELETE_TORRENT, json!({ "id": item.id })).await;
        assert_eq!(deleted["deleteTorrent"]["success"], true, "{deleted}");

        wait_for_live(&service, &item.hash, false).await;
        common::wait_for(Duration::from_secs(30), "payload deleted", || async {
            (!exists(&item.payload)).then_some(())
        })
        .await;
        assert!(!row_exists(&app, &mut admin, &item.id).await);
        app.shutdown().await;
    });
}

#[test]
fn flagged_delete_is_refused_during_post_processing_and_leaves_everything_intact() {
    common::run_app_test(|| async {
        let app = TestApp::start_with(TestAppOptions {
            with_torrent: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let service = app.services.get_torrent().await.unwrap();
        let items = seed(&app, &mut admin, &["busy"]).await;
        let item = &items[0];

        service.set_post_processing_for_test(&item.hash, true).await;
        set_delete_files(&mut admin, &item.id, true).await;
        let refused = admin.post(DELETE_TORRENT, json!({ "id": item.id })).await;
        let text = refused.body.to_string();
        assert!(
            text.contains("post-processing is still running"),
            "client must see the guard's reason: {text}"
        );
        assert!(row_exists(&app, &mut admin, &item.id).await, "row survives");
        assert!(
            live_hashes(&service).await.contains(&item.hash),
            "still live"
        );
        assert!(exists(&item.payload));

        // Without the flag, removing the record is safe even mid-import.
        set_delete_files(&mut admin, &item.id, false).await;
        let removed = admin.query(DELETE_TORRENT, json!({ "id": item.id })).await;
        assert_eq!(removed["deleteTorrent"]["success"], true, "{removed}");
        wait_for_live(&service, &item.hash, false).await;
        assert!(exists(&item.payload));
        app.shutdown().await;
    });
}

#[test]
fn a_rolled_back_bulk_delete_leaves_every_torrent_running() {
    common::run_app_test(|| async {
        let app = TestApp::start_with(TestAppOptions {
            with_torrent: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let service = app.services.get_torrent().await.unwrap();
        let items = seed(&app, &mut admin, &["first", "second"]).await;

        // The second torrent is mid-import with files flagged for deletion, so its
        // before-delete hook aborts the whole bulk transaction, including the first row.
        service
            .set_post_processing_for_test(&items[1].hash, true)
            .await;
        set_delete_files(&mut admin, &items[0].id, true).await;
        set_delete_files(&mut admin, &items[1].id, true).await;
        let refused = admin
            .post(
                DELETE_TORRENTS,
                json!({ "where": { "state": { "eq": "paused" } } }),
            )
            .await;
        // The bulk path reports a hook failure as a failed payload, without the reason.
        assert_eq!(
            refused.body["data"]["deleteTorrents"]["success"], false,
            "{}",
            refused.body
        );

        // Give a (wrongly) scheduled removal time to run before asserting nothing happened.
        tokio::time::sleep(Duration::from_millis(500)).await;
        for item in &items {
            assert!(row_exists(&app, &mut admin, &item.id).await);
            assert!(live_hashes(&service).await.contains(&item.hash));
            assert!(exists(&item.payload));
        }

        // Once the import is over the same bulk delete removes both, files included.
        service
            .set_post_processing_for_test(&items[1].hash, false)
            .await;
        let done = admin
            .query(
                DELETE_TORRENTS,
                json!({ "where": { "state": { "eq": "paused" } } }),
            )
            .await;
        assert_eq!(done["deleteTorrents"]["success"], true, "{done}");
        for item in &items {
            wait_for_live(&service, &item.hash, false).await;
            common::wait_for(Duration::from_secs(30), "payload deleted", || async {
                (!exists(&item.payload)).then_some(())
            })
            .await;
        }
        app.shutdown().await;
    });
}

#[test]
fn deleting_a_row_without_a_live_torrent_just_removes_the_row() {
    common::run_app_test(|| async {
        let app = TestApp::start_with(TestAppOptions {
            with_torrent: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let admin_id = admin.user_id.clone().expect("admin id");
        let created: Value = admin
            .query(
                CREATE_TORRENT,
                json!({ "input": {
                    "userId": admin_id,
                    "infoHash": "7".repeat(40),
                    "name": "never started",
                    "state": "error",
                    "progress": 0.0,
                    "totalBytes": 1,
                    "downloadedBytes": 0,
                    "uploadedBytes": 0,
                    "uploadedBytesTotal": 0,
                    "savePath": "/tmp/downloads",
                    "excludedFiles": [],
                    "addedAt": "2026-11-01T00:00:00Z",
                    "deleteFilesOnRemove": true,
                }}),
            )
            .await;
        let id = created["createTorrent"]["torrent"]["id"].as_str().unwrap();
        let deleted = admin.query(DELETE_TORRENT, json!({ "id": id })).await;
        assert_eq!(deleted["deleteTorrent"]["success"], true, "{deleted}");
        assert!(!row_exists(&app, &mut admin, id).await);
        app.shutdown().await;
    });
}
