//! `matchTorrentFile` imports one file of a download as what a person says it is. These tests run
//! the real import against a temporary library and a real librqbit session restored from disk.

mod common;

use std::path::PathBuf;
use std::time::Duration;

use common::{GqlClient, TestApp, TestAppOptions, playback};
use librarian::services::manager::Service;
use librqbit::spawn_utils::BlockingSpawner;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, CreateTorrentOptions, Session,
    SessionOptions, SessionPersistenceConfig, create_torrent,
};
use serde_json::{Value, json};

const MATCH: &str = r#"mutation M($input: MatchTorrentFileInput!) {
  matchTorrentFile(input: $input) { success matchCount error }
}"#;
const CANDIDATES: &str = r#"query C($infoHash: String!, $fileIndex: Int!, $libraryId: String!) {
  torrentFileCandidates(infoHash: $infoHash, fileIndex: $fileIndex, libraryId: $libraryId) {
    targetType targetId targetName
  }
}"#;
const STATE: &str = r#"query S($id: String!) {
  torrent(id: $id) {
    postProcessStatus postProcessError
    pendingMatches(page: { limit: 10, offset: 0 }) { edges { node { sourcePath movieId copiedAt } } }
  }
}"#;

struct Seeded {
    hash: String,
    id: String,
    payload: PathBuf,
}

/// A completed one-file torrent in the app's persisted session, with its database row.
async fn seed(app: &TestApp, admin: &mut GqlClient, file_name: &str) -> Seeded {
    let service = app.services.get_torrent().await.expect("torrent service");
    service.stop().await.expect("stop torrent service");
    let downloads = app.path("downloads");
    std::fs::create_dir_all(&downloads).unwrap();
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
    let payload = downloads.join(file_name);
    std::fs::write(&payload, "matroska".repeat(40_000)).unwrap();
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
    let AddTorrentResponse::Added(_, handle) = standalone
        .add_torrent(
            AddTorrent::from_bytes(torrent.as_bytes().expect("torrent bytes")),
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
    let id = playback::create(
        admin,
        "Torrent",
        json!({
            "userId": admin.user_id.clone().expect("admin id"),
            "infoHash": hash,
            "name": file_name,
            "state": "seeding",
            "progress": 1.0,
            "totalBytes": 320000,
            "downloadedBytes": 320000,
            "uploadedBytes": 0,
            "uploadedBytesTotal": 0,
            "deleteFilesOnRemove": false,
            "savePath": downloads.to_string_lossy(),
            "excludedFiles": [],
            "addedAt": "2026-11-01T00:00:00Z",
        }),
    )
    .await;
    standalone.stop().await;
    service.start().await.expect("restart torrent service");
    // The restored torrent's files have to be readable before it can be matched.
    let probe = hash.clone();
    let service_ref = service.clone();
    common::wait_for(Duration::from_secs(30), "restored torrent", move || {
        let service = service_ref.clone();
        let hash = probe.clone();
        async move {
            let file = service.completed_file(&hash, 0).await.ok();
            file.map(|_| ())
        }
    })
    .await;
    Seeded { hash, id, payload }
}

async fn library(app: &TestApp, admin: &mut GqlClient, name: &str) -> (String, PathBuf) {
    let path = app.path(&format!("media/{name}"));
    std::fs::create_dir_all(&path).unwrap();
    let id = playback::create(
        admin,
        "Library",
        json!({
            "userId": admin.user_id.clone().expect("admin id"),
            "name": name,
            "path": path,
            "libraryType": "MOVIES",
            "autoScan": false,
            "autoOrganize": false,
            "namingPattern": "{title}",
            "scanIntervalMinutes": 60,
            "watchForChanges": false,
            "scanning": false,
        }),
    )
    .await;
    (id, path)
}

async fn movie(admin: &mut GqlClient, library: &str, title: &str) -> String {
    playback::create(
        admin,
        "Movie",
        json!({
            "userId": admin.user_id.clone().expect("admin id"),
            "libraryId": library,
            "title": title,
            "year": 2026,
            "genres": [],
            "castNames": [],
            "productionCountries": [],
            "spokenLanguages": [],
            "monitored": false,
            "wanted": true,
            "hasFile": false,
        }),
    )
    .await
}

fn files_under(root: &std::path::Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            if entry.path().is_dir() {
                pending.push(entry.path());
            } else {
                found.push(entry.path());
            }
        }
    }
    found
}

async fn match_file(admin: &mut GqlClient, hash: &str, library: &str, target: &str) -> Value {
    admin
        .query(
            MATCH,
            json!({ "input": {
                "infoHash": hash,
                "fileIndex": 0,
                "libraryId": library,
                "targetType": "Movie",
                "targetId": target,
            }}),
        )
        .await["matchTorrentFile"]
        .clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_chosen_file_lands_in_the_library_and_the_download_keeps_its_copy() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    let seeded = seed(&app, &mut admin, "Unrecognisable.Release.Name.mkv").await;
    let (library_id, library_path) = library(&app, &mut admin, "films").await;
    let target = movie(&mut admin, &library_id, "The Chosen Film").await;

    let suggested = admin
        .query(
            CANDIDATES,
            json!({ "infoHash": seeded.hash, "fileIndex": 0, "libraryId": library_id }),
        )
        .await;
    assert!(suggested["torrentFileCandidates"].is_array(), "{suggested}");

    let result = match_file(&mut admin, &seeded.hash, &library_id, &target).await;
    assert_eq!(result["success"], true, "{result}");
    assert_eq!(result["matchCount"], 1);

    let imported = files_under(&library_path);
    assert_eq!(imported.len(), 1, "{imported:?}");
    assert_eq!(
        std::fs::metadata(&imported[0]).unwrap().len(),
        std::fs::metadata(&seeded.payload).unwrap().len()
    );
    assert!(
        seeded.payload.exists(),
        "the download keeps seeding its own copy"
    );

    let state = admin.query(STATE, json!({ "id": seeded.id })).await;
    assert_eq!(
        state["torrent"]["postProcessStatus"], "completed",
        "{state}"
    );
    let rows = &state["torrent"]["pendingMatches"]["edges"];
    assert_eq!(rows[0]["node"]["movieId"], target.as_str(), "{state}");
    assert!(rows[0]["node"]["copiedAt"].is_string(), "{state}");

    let matched = admin
        .query(
            r#"query F($id: String!) { movie(id: $id) { mediaFileId mediaFile { matchType path } } }"#,
            json!({ "id": target }),
        )
        .await;
    assert_eq!(
        matched["movie"]["mediaFile"]["matchType"], "manual",
        "{matched}"
    );
    app.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_title_from_another_library_is_refused_and_nothing_is_copied() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    let seeded = seed(&app, &mut admin, "Some.Release.mkv").await;
    let (chosen, chosen_path) = library(&app, &mut admin, "chosen").await;
    let (other, other_path) = library(&app, &mut admin, "other").await;
    let elsewhere = movie(&mut admin, &other, "Elsewhere").await;

    let result = match_file(&mut admin, &seeded.hash, &chosen, &elsewhere).await;
    assert_eq!(result["success"], false, "{result}");
    assert!(files_under(&chosen_path).is_empty());
    assert!(files_under(&other_path).is_empty());
    app.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn only_an_administrator_can_match_a_file() {
    let app = TestApp::start().await;
    let mut anonymous = app.client();
    let response = anonymous
        .post(
            MATCH,
            json!({ "input": {
                "infoHash": "0".repeat(40),
                "fileIndex": 0,
                "libraryId": "x",
                "targetType": "Movie",
                "targetId": "y",
            }}),
        )
        .await;
    assert!(!response.errors().is_empty());
    app.shutdown().await;
}
