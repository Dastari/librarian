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
    pendingMatches(page: { limit: 10, offset: 0 }) { edges { node { sourcePath movieId chapterId copiedAt copyError unmatchedReason } } }
  }
}"#;

struct Seeded {
    hash: String,
    id: String,
    payload: PathBuf,
}

/// A completed one-file torrent in the app's persisted session, with its database row.
async fn seed(app: &TestApp, admin: &mut GqlClient, file_name: &str) -> Seeded {
    seed_with(app, admin, file_name, |payload| {
        std::fs::write(payload, "matroska".repeat(40_000)).unwrap();
    })
    .await
}

/// Real audio with tags, because a release's lengths and tags are what decide where it goes.
fn write_audio(path: &std::path::Path, seconds: u32, tags: &[(&str, String)]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut command = std::process::Command::new(
        std::env::var("LIBRARIAN_ACCEPTANCE_FFMPEG").unwrap_or_else(|_| "ffmpeg".to_string()),
    );
    command.args(["-y", "-loglevel", "error", "-f", "lavfi", "-i"]);
    command.arg("anullsrc=r=8000:cl=mono");
    command.args(["-t", &seconds.to_string()]);
    for (key, value) in tags {
        command.arg("-metadata").arg(format!("{key}={value}"));
    }
    if path.extension().is_some_and(|extension| extension == "m4b") {
        command.args(["-c:a", "aac", "-f", "mp4"]);
    }
    let output = command.arg(path).output().expect("ffmpeg");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Parts of a book, each `seconds` long and tagged as its album says.
fn write_parts(folder: &std::path::Path, album: &str, count: u32, seconds: u32) {
    for number in 1..=count {
        write_audio(
            &folder.join(format!("{album} -- {number:02} of {count}.mp3")),
            seconds,
            &[
                ("title", format!("{number:02} - {album}")),
                ("album", album.to_string()),
                ("artist", "An Author".to_string()),
                ("track", format!("{number}/{count}")),
            ],
        );
    }
}

async fn seed_with(
    app: &TestApp,
    admin: &mut GqlClient,
    file_name: &str,
    write: impl FnOnce(&std::path::Path),
) -> Seeded {
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
    write(&payload);
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

async fn library(
    app: &TestApp,
    admin: &mut GqlClient,
    name: &str,
    kind: &str,
) -> (String, PathBuf) {
    let path = app.path(&format!("media/{name}"));
    std::fs::create_dir_all(&path).unwrap();
    let id = playback::create(
        admin,
        "Library",
        json!({
            "userId": admin.user_id.clone().expect("admin id"),
            "name": name,
            "path": path,
            "libraryType": kind,
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
    let (library_id, library_path) = library(&app, &mut admin, "films", "MOVIES").await;
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
    let (chosen, chosen_path) = library(&app, &mut admin, "chosen", "MOVIES").await;
    let (other, other_path) = library(&app, &mut admin, "other", "MOVIES").await;
    let elsewhere = movie(&mut admin, &other, "Elsewhere").await;

    let result = match_file(&mut admin, &seeded.hash, &chosen, &elsewhere).await;
    assert_eq!(result["success"], false, "{result}");
    assert!(files_under(&chosen_path).is_empty());
    assert!(files_under(&other_path).is_empty());
    app.shutdown().await;
}

async fn audiobook_with_chapters(
    admin: &mut GqlClient,
    library: &str,
    title: &str,
    chapters: i32,
) -> (String, Vec<String>) {
    let owner = admin.user_id.clone().expect("admin id");
    let book = playback::create(
        admin,
        "Audiobook",
        json!({
            "libraryId": library,
            "userId": owner,
            "title": title,
            "authorName": "An Author",
            "autoDownload": false,
            "autoDownloadMode": "NONE",
            "hasFiles": false,
            "narrators": [],
        }),
    )
    .await;
    let mut ids = Vec::new();
    for number in 1..=chapters {
        ids.push(
            playback::create(
                admin,
                "Chapter",
                json!({
                    "audiobookId": book,
                    "chapterNumber": number,
                    "title": format!("Part {number}"),
                    "startTimeSecs": f64::from(number - 1) * 60.0,
                    "durationSecs": 60,
                    "wanted": false,
                }),
            )
            .await,
        );
    }
    (book, ids)
}

const CHAPTER_FILES: &str = r#"query C($id: String!) {
  audiobook(id: $id) { chapters(page: { limit: 20, offset: 0 }) { edges { node { mediaFileId wanted } } } }
}"#;

#[tokio::test(flavor = "multi_thread")]
async fn one_file_for_a_whole_book_belongs_to_every_chapter_and_takes_its_name() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    let seeded = seed_with(&app, &mut admin, "The Long Way.m4b", |payload| {
        write_audio(payload, 180, &[("album", "The Long Way".to_string())]);
    })
    .await;
    let (library_id, library_path) = library(&app, &mut admin, "books", "AUDIOBOOKS").await;
    let (book, _) = audiobook_with_chapters(&mut admin, &library_id, "The Long Way", 3).await;
    let linked = admin
        .query(
            r#"mutation U($id: String!, $input: UpdateTorrentInput!) { updateTorrent(id: $id, input: $input) { success error } }"#,
            json!({ "id": seeded.id, "input": { "audiobookId": book, "libraryId": library_id } }),
        )
        .await;
    assert_eq!(linked["updateTorrent"]["success"], true, "{linked}");

    let processed = admin
        .query(
            r#"mutation P($id: String!) { processSource(sourceType: "torrent", sourceId: $id) { success filesProcessed error } }"#,
            json!({ "id": seeded.hash }),
        )
        .await;
    assert_eq!(processed["processSource"]["success"], true, "{processed}");

    let imported = files_under(&library_path);
    assert_eq!(imported.len(), 1, "{imported:?}");
    assert_eq!(
        imported[0].file_name().and_then(|name| name.to_str()),
        Some("The Long Way.m4b"),
        "{imported:?}"
    );
    let chapters = admin.query(CHAPTER_FILES, json!({ "id": book })).await;
    let rows = chapters["audiobook"]["chapters"]["edges"]
        .as_array()
        .unwrap();
    assert_eq!(rows.len(), 3, "{chapters}");
    let first = rows[0]["node"]["mediaFileId"].as_str().expect("linked");
    for row in rows {
        assert_eq!(row["node"]["mediaFileId"], first, "{chapters}");
        assert_eq!(row["node"]["wanted"], false, "{chapters}");
    }

    // Unlinking the file releases every chapter that shared it.
    let unlinked = admin
        .query(
            r#"mutation U($id: String!, $input: UpdateMediaFileInput!) { updateMediaFile(id: $id, input: $input) { success error } }"#,
            json!({ "id": first, "input": { "chapterId": null } }),
        )
        .await;
    assert_eq!(unlinked["updateMediaFile"]["success"], true, "{unlinked}");
    let chapters = admin.query(CHAPTER_FILES, json!({ "id": book })).await;
    for row in chapters["audiobook"]["chapters"]["edges"]
        .as_array()
        .unwrap()
    {
        assert!(row["node"]["mediaFileId"].is_null(), "{chapters}");
    }
    app.shutdown().await;
}

async fn link_to_book(admin: &mut GqlClient, seeded: &Seeded, book: &str, library: &str) {
    let linked = admin
        .query(
            r#"mutation U($id: String!, $input: UpdateTorrentInput!) { updateTorrent(id: $id, input: $input) { success error } }"#,
            json!({ "id": seeded.id, "input": { "audiobookId": book, "libraryId": library } }),
        )
        .await;
    assert_eq!(linked["updateTorrent"]["success"], true, "{linked}");
}

async fn process(admin: &mut GqlClient, seeded: &Seeded) -> Value {
    admin
        .query(
            r#"mutation P($id: String!) { processSource(sourceType: "torrent", sourceId: $id) { success filesProcessed error } }"#,
            json!({ "id": seeded.hash }),
        )
        .await
}

const COPIED: &str = r#"query S($id: String!) {
  torrent(id: $id) {
    pendingMatches(page: { limit: 50, offset: 0 }) { edges { node { sourcePath copiedAt unmatchedReason } } }
  }
}"#;

async fn want_every_chapter(admin: &mut GqlClient, book: &str) {
    for id in placed_chapter_ids(admin, book).await {
        let wanted = admin
            .query(
                r#"mutation W($id: String!, $input: UpdateChapterInput!) { updateChapter(id: $id, input: $input) { success error } }"#,
                json!({ "id": id, "input": { "wanted": true } }),
            )
            .await;
        assert_eq!(wanted["updateChapter"]["success"], true, "{wanted}");
    }
}

const PREVIEW: &str = r#"query P($infoHash: String!) {
  torrentMatchPreview(infoHash: $infoHash) {
    status summary reason canAccept chapterSource fileShape evidence completeFacts
    groups { key label fileCount seconds isTarget bookId bookTitle }
    files { fileIndex groupKey status target detail warning }
  }
}"#;
const PLACED: &str = r#"query C($id: String!) {
  audiobook(id: $id) {
    chapterSource fileShape providerChapters
    chapters(orderBy: { chapterNumber: ASC }, page: { limit: 50, offset: 0 }) {
      edges { node { title chapterNumber startTimeSecs fileOffsetSecs mediaFileId wanted } }
    }
  }
}"#;
const IMPORT_ITEM: &str = r#"mutation I($input: ImportIdentifiedItemInput!) {
  importIdentifiedItem(input: $input) { success matchCount error }
}"#;
const IDENTIFIED: &str = r#"query R($hash: String!) {
  torrents(where: { infoHash: { eq: $hash } }, page: { limit: 1, offset: 0 }) {
    edges { node { identificationStatus identificationSummary } }
  }
  identifiedItems(where: { infoHash: { eq: $hash } }, page: { limit: 50, offset: 0 }) {
    edges { node { id itemKey kind status label title creator fileCount summary evidence canAccept libraryItemId } }
  }
  identificationEvents(where: { infoHash: { eq: $hash } }, page: { limit: 500, offset: 0 }) {
    edges { node { runId trigger sequence status message durationMs } }
  }
}"#;

/// Providers and listening reach outside a test. What's being tested is what happens around them.
async fn identify_offline(admin: &mut GqlClient) {
    for key in [
        "metadata.audible_enabled",
        "metadata.openlibrary_enabled",
        "metadata.listening_enabled",
    ] {
        let saved = admin
            .query(
                r#"mutation S($input: CreateAppSettingInput!) { createAppSetting(input: $input) { success error } }"#,
                json!({ "input": { "key": key, "value": "false", "category": "metadata" } }),
            )
            .await;
        assert_eq!(saved["createAppSetting"]["success"], true, "{saved}");
    }
}

async fn run_identification(admin: &mut GqlClient, hash: &str) {
    let asked = admin
        .query(
            r#"mutation A($hash: String!) { identifyTorrent(infoHash: $hash) { success error } }"#,
            json!({ "hash": hash }),
        )
        .await;
    assert_eq!(asked["identifyTorrent"]["success"], true, "{asked}");
}

/// Wait for a finished run whose results satisfy `settled`.
async fn results_when(
    admin: &mut GqlClient,
    hash: &str,
    settled: impl Fn(&[Value]) -> bool,
) -> Value {
    let mut last = Value::Null;
    for _ in 0..120 {
        last = admin.query(IDENTIFIED, json!({ "hash": hash })).await;
        let items: Vec<Value> = last["identifiedItems"]["edges"]
            .as_array()
            .map(|edges| edges.iter().map(|edge| edge["node"].clone()).collect())
            .unwrap_or_default();
        let status = &last["torrents"]["edges"][0]["node"]["identificationStatus"];
        if status == "done" && settled(&items) {
            return last;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    panic!("identification did not settle: {last}");
}

fn item_labelled<'a>(results: &'a Value, label: &str) -> &'a Value {
    results["identifiedItems"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| &edge["node"])
        .find(|item| item["label"] == label)
        .unwrap_or_else(|| panic!("no result for {label}: {results}"))
}

fn has_item(items: &[Value], label: &str) -> bool {
    items.iter().any(|item| item["label"] == label)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_finished_pack_brings_in_what_is_wanted_and_reports_the_rest() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    identify_offline(&mut admin).await;
    let (library_id, library_path) = library(&app, &mut admin, "books", "AUDIOBOOKS").await;
    let (book, _) =
        audiobook_with_chapters(&mut admin, &library_id, "The Long Way: Book 1", 3).await;
    want_every_chapter(&mut admin, &book).await;
    let seeded = seed_with(&app, &mut admin, "The Series (1-2)", |payload| {
        write_parts(
            &payload.join("Book One - The Long Way"),
            "The Long Way",
            6,
            30,
        );
        write_parts(
            &payload.join("Book Two - Another Road"),
            "Another Road",
            2,
            60,
        );
    })
    .await;

    let settled = results_when(&mut admin, &seeded.hash, |items| {
        items
            .iter()
            .any(|item| item["label"] == "The Long Way" && item["status"] == "imported")
            && has_item(items, "Another Road")
    })
    .await;
    assert_eq!(
        item_labelled(&settled, "The Long Way")["libraryItemId"],
        book,
        "{settled}"
    );
    assert_eq!(
        item_labelled(&settled, "Another Road")["status"],
        "unknown",
        "{settled}"
    );
    assert_eq!(files_under(&library_path).len(), 6);
    let steps = settled["identificationEvents"]["edges"].as_array().unwrap();
    assert!(
        steps
            .iter()
            .any(|step| step["node"]["durationMs"].is_number()),
        "steps say how long they took: {settled}"
    );

    // Running again changes nothing that was done.
    run_identification(&mut admin, &seeded.hash).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    let again = results_when(&mut admin, &seeded.hash, |items| {
        has_item(items, "The Long Way")
    })
    .await;
    assert_eq!(
        item_labelled(&again, "The Long Way")["status"],
        "imported",
        "{again}"
    );
    assert_eq!(files_under(&library_path).len(), 6);
    app.shutdown().await;
}

/// Restarting must not pull old downloads into a library, even when one now matches something
/// wanted. A restored torrent reads as empty and then has all its files finish at once.
#[tokio::test(flavor = "multi_thread")]
async fn titles_sort_without_a_leading_article() {
    let app = TestApp::start_with(TestAppOptions::default()).await;
    let mut admin = app.admin_client().await;
    let (library_id, _) = library(&app, &mut admin, "books", "AUDIOBOOKS").await;
    let (book, _) = audiobook_with_chapters(&mut admin, &library_id, "The Long Way", 1).await;
    let sort_title = |result: Value| result["audiobooks"]["edges"][0]["node"]["sortTitle"].clone();
    const READ: &str = r#"query R($id: String!) { audiobooks(where: { id: { eq: $id } }) { edges { node { sortTitle } } } }"#;
    const RENAME: &str = r#"mutation U($id: String!, $input: UpdateAudiobookInput!) { updateAudiobook(id: $id, input: $input) { success error } }"#;
    assert_eq!(
        sort_title(admin.query(READ, json!({ "id": book })).await),
        "Long Way"
    );

    admin
        .query(
            RENAME,
            json!({ "id": book, "input": { "title": "A Short Way" } }),
        )
        .await;
    assert_eq!(
        sort_title(admin.query(READ, json!({ "id": book })).await),
        "Short Way"
    );

    // Someone's own choice stays through a rename.
    admin
        .query(
            RENAME,
            json!({ "id": book, "input": { "sortTitle": "Mine" } }),
        )
        .await;
    admin
        .query(
            RENAME,
            json!({ "id": book, "input": { "title": "The Third Way" } }),
        )
        .await;
    assert_eq!(
        sort_title(admin.query(READ, json!({ "id": book })).await),
        "Mine"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_latest_identification_runs_are_kept() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    identify_offline(&mut admin).await;
    let seeded = seed_with(&app, &mut admin, "The Long Way", |payload| {
        write_parts(payload, "The Long Way", 2, 30);
    })
    .await;
    results_when(&mut admin, &seeded.hash, |items| !items.is_empty()).await;

    let runs_of = |results: &Value| -> Vec<String> {
        let mut runs: Vec<String> = results["identificationEvents"]["edges"]
            .as_array()
            .map(|steps| {
                steps
                    .iter()
                    .filter_map(|step| step["node"]["runId"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        runs.sort();
        runs.dedup();
        runs
    };
    for _ in 0..5 {
        let before = admin
            .query(IDENTIFIED, json!({ "hash": seeded.hash }))
            .await;
        let newest = runs_of(&before).pop();
        run_identification(&mut admin, &seeded.hash).await;
        for _ in 0..120 {
            let now = admin
                .query(IDENTIFIED, json!({ "hash": seeded.hash }))
                .await;
            let finished = now["torrents"]["edges"][0]["node"]["identificationStatus"] == "done";
            if finished && runs_of(&now).pop() != newest {
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    let results = admin
        .query(IDENTIFIED, json!({ "hash": seeded.hash }))
        .await;
    assert_eq!(runs_of(&results).len(), 3, "{results}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_restart_identifies_old_downloads_without_importing_them() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    identify_offline(&mut admin).await;
    let seeded = seed_with(&app, &mut admin, "The Long Way", |payload| {
        write_parts(payload, "The Long Way", 6, 30);
    })
    .await;
    // Let its first finish run its course, which is allowed to import.
    let mut finished = Value::Null;
    for _ in 0..120 {
        finished = admin
            .query(IDENTIFIED, json!({ "hash": seeded.hash }))
            .await;
        let done_after_finish = finished["identificationEvents"]["edges"]
            .as_array()
            .is_some_and(|steps| {
                steps
                    .iter()
                    .any(|step| step["node"]["trigger"] == "Download finished")
            })
            && finished["torrents"]["edges"][0]["node"]["identificationStatus"] == "done";
        if done_after_finish {
            break;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert_eq!(
        finished["torrents"]["edges"][0]["node"]["identificationStatus"], "done",
        "{finished}"
    );

    // It finished before the book was wanted, and has been through post-processing.
    let service = app.services.get_torrent().await.expect("torrent service");
    service.stop().await.expect("stop torrent service");
    let (library_id, library_path) = library(&app, &mut admin, "books", "AUDIOBOOKS").await;
    let (book, _) = audiobook_with_chapters(&mut admin, &library_id, "The Long Way", 3).await;
    want_every_chapter(&mut admin, &book).await;
    let reset = admin
        .query(
            r#"mutation U($id: String!, $input: UpdateTorrentInput!) { updateTorrent(id: $id, input: $input) { success error } }"#,
            json!({ "id": seeded.id, "input": { "postProcessStatus": "unmatched", "identificationStatus": null } }),
        )
        .await;
    assert_eq!(reset["updateTorrent"]["success"], true, "{reset}");
    service.start().await.expect("restart torrent service");

    let looked = results_when(&mut admin, &seeded.hash, |items| {
        items
            .iter()
            .any(|item| item["label"] == "The Long Way" && item["status"] == "wanted")
    })
    .await;
    // Long enough for the client to have checked the files and reported them finished.
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(files_under(&library_path).is_empty(), "{looked}");
    let later = admin
        .query(IDENTIFIED, json!({ "hash": seeded.hash }))
        .await;
    assert_eq!(
        item_labelled(&later, "The Long Way")["status"],
        "wanted",
        "{later}"
    );
    app.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_person_can_say_what_kind_of_thing_a_download_is() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    identify_offline(&mut admin).await;
    let seeded = seed_with(&app, &mut admin, "Some Songs", |payload| {
        write_parts(payload, "Some Songs", 3, 60);
    })
    .await;
    run_identification(&mut admin, &seeded.hash).await;
    let first = results_when(&mut admin, &seeded.hash, |items| {
        items.iter().any(|item| item["kind"] == "music")
    })
    .await;
    assert_eq!(
        item_labelled(&first, "Some Songs")["kind"],
        "music",
        "{first}"
    );
    let told = admin
        .query(
            r#"mutation A($hash: String!) { identifyTorrent(infoHash: $hash, kind: "audiobook") { success error } }"#,
            json!({ "hash": seeded.hash }),
        )
        .await;
    assert_eq!(told["identifyTorrent"]["success"], true, "{told}");
    results_when(&mut admin, &seeded.hash, |items| {
        items.iter().any(|item| item["kind"] == "audiobook")
    })
    .await;
    // Refused rather than stored: only kinds that exist.
    let refused = admin
        .query(
            r#"mutation A($hash: String!) { identifyTorrent(infoHash: $hash, kind: "podcast") { success error } }"#,
            json!({ "hash": seeded.hash }),
        )
        .await;
    assert_eq!(refused["identifyTorrent"]["success"], false, "{refused}");
    assert!(
        refused["identifyTorrent"]["error"]
            .as_str()
            .unwrap()
            .contains("podcast"),
        "{refused}"
    );
    app.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_book_of_the_same_name_by_someone_else_is_not_this_book() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    identify_offline(&mut admin).await;
    let (library_id, library_path) = library(&app, &mut admin, "books", "AUDIOBOOKS").await;
    let (book, _) = audiobook_with_chapters(&mut admin, &library_id, "The Last Colony", 3).await;
    let updated = admin
        .query(
            r#"mutation U($id: String!, $input: UpdateAudiobookInput!) { updateAudiobook(id: $id, input: $input) { success error } }"#,
            json!({ "id": book, "input": { "authorName": "Mike Robb" } }),
        )
        .await;
    assert_eq!(updated["updateAudiobook"]["success"], true, "{updated}");
    let seeded = seed_with(&app, &mut admin, "The Last Colony", |payload| {
        write_parts(payload, "The Last Colony", 3, 60);
    })
    .await;

    run_identification(&mut admin, &seeded.hash).await;
    let results = results_when(&mut admin, &seeded.hash, |items| !items.is_empty()).await;
    let item = item_labelled(&results, "The Last Colony");
    assert!(item["libraryItemId"].is_null(), "{results}");
    assert!(files_under(&library_path).is_empty());
    app.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn one_book_needing_review_leaves_another_books_import_alone() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    identify_offline(&mut admin).await;
    let seeded = seed_with(&app, &mut admin, "The Series", |payload| {
        write_parts(&payload.join("One"), "The Long Way", 3, 60);
        write_parts(&payload.join("Two"), "Another Road", 4, 60);
    })
    .await;
    let (library_id, library_path) = library(&app, &mut admin, "books", "AUDIOBOOKS").await;
    let (first, _) = audiobook_with_chapters(&mut admin, &library_id, "The Long Way", 3).await;
    want_every_chapter(&mut admin, &first).await;
    run_identification(&mut admin, &seeded.hash).await;
    let done = results_when(&mut admin, &seeded.hash, |items| {
        items
            .iter()
            .any(|item| item["label"] == "The Long Way" && item["status"] == "imported")
    })
    .await;
    assert_eq!(files_under(&library_path).len(), 3, "{done}");

    // Another book whose length is far from these files asks for review, and only about them.
    let (second, _) = audiobook_with_chapters(&mut admin, &library_id, "Another Road", 40).await;
    let refused = admin
        .query(
            IMPORT_ITEM,
            json!({ "input": {
                "itemId": item_labelled(&done, "Another Road")["id"],
                "libraryItemId": second,
            } }),
        )
        .await;
    assert_eq!(files_under(&library_path).len(), 3, "{refused}");
    let state = admin.query(COPIED, json!({ "id": seeded.id })).await;
    let copied = state["torrent"]["pendingMatches"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|edge| !edge["node"]["copiedAt"].is_null())
        .count();
    assert_eq!(
        copied, 3,
        "the first book is still recorded as imported: {state}"
    );
    app.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pack_imports_only_its_book_with_chapters_placed_by_time() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    identify_offline(&mut admin).await;
    let seeded = seed_with(&app, &mut admin, "The Series (1-2)", |payload| {
        write_parts(
            &payload.join("Book One - The Long Way"),
            "The Long Way",
            6,
            30,
        );
        write_parts(
            &payload.join("Book Two - Another Road"),
            "Another Road",
            2,
            60,
        );
    })
    .await;
    let (library_id, library_path) = library(&app, &mut admin, "books", "AUDIOBOOKS").await;
    // Three 60-second chapters that the 30-second parts don't line up with.
    let (book, _) =
        audiobook_with_chapters(&mut admin, &library_id, "The Long Way: Book 1", 3).await;
    link_to_book(&mut admin, &seeded, &book, &library_id).await;

    // The preview changes nothing and says what will happen.
    let preview = admin
        .query(PREVIEW, json!({ "infoHash": seeded.hash }))
        .await;
    let plan = &preview["torrentMatchPreview"];
    assert_eq!(plan["status"], "ready", "{preview}");
    assert_eq!(plan["fileShape"], "parts", "{preview}");
    assert_eq!(plan["chapterSource"], "provider", "{preview}");
    assert_eq!(plan["completeFacts"], true, "{preview}");
    let groups = plan["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 2, "{preview}");
    let other = groups
        .iter()
        .find(|group| group["isTarget"] == false)
        .unwrap();
    assert_eq!(other["label"], "Another Road", "{preview}");
    assert!(
        files_under(&library_path).is_empty(),
        "a preview moves nothing"
    );

    let processed = process(&mut admin, &seeded).await;
    assert_eq!(
        processed["processSource"]["filesProcessed"], 6,
        "{processed}"
    );
    let state = admin.query(STATE, json!({ "id": seeded.id })).await;
    assert_eq!(state["torrent"]["postProcessStatus"], "partial", "{state}");

    let imported = files_under(&library_path);
    assert_eq!(imported.len(), 6, "{imported:?}");
    let names: Vec<String> = imported
        .iter()
        .filter_map(|path| path.file_name()?.to_str().map(str::to_string))
        .collect();
    assert!(
        names.iter().all(|name| name.contains("Part 0")),
        "{names:?}"
    );
    assert!(
        names.iter().all(|name| !name.contains("Another")),
        "{names:?}"
    );

    let placed = admin.query(PLACED, json!({ "id": book })).await;
    assert_eq!(placed["audiobook"]["fileShape"], "parts", "{placed}");
    assert_eq!(placed["audiobook"]["chapterSource"], "provider", "{placed}");
    let rows = placed["audiobook"]["chapters"]["edges"].as_array().unwrap();
    assert_eq!(rows.len(), 3, "the provider's chapters stay: {placed}");
    let file_of = |index: usize| {
        rows[index]["node"]["mediaFileId"]
            .as_str()
            .expect("linked")
            .to_string()
    };
    // Chapters start at 0, 60 and 120 s: parts 1, 3 and 5, each at the start of its file.
    assert!(
        rows.iter().all(|row| row["node"]["wanted"] == false),
        "{placed}"
    );
    assert_ne!(file_of(0), file_of(1), "{placed}");
    assert_ne!(file_of(1), file_of(2), "{placed}");

    // Every part is still linked to the book, including those no chapter starts in.
    let chapter_ids = placed_chapter_ids(&mut admin, &book).await;
    let files = admin
        .query(
            r#"query F($ids: [String!]!) { mediaFiles(where: { chapterId: { inList: $ids } }, page: { limit: 50, offset: 0 }) { edges { node { id } } } }"#,
            json!({ "ids": chapter_ids }),
        )
        .await;
    assert_eq!(
        files["mediaFiles"]["edges"].as_array().unwrap().len(),
        6,
        "{files}"
    );

    // The other book can then be imported from the same download.
    let (second, _) = audiobook_with_chapters(&mut admin, &library_id, "Another Road", 2).await;
    run_identification(&mut admin, &seeded.hash).await;
    let results = results_when(&mut admin, &seeded.hash, |items| {
        items
            .iter()
            .any(|item| item["label"] == "Another Road" && item["kind"] == "audiobook")
    })
    .await;
    let imported = admin
        .query(
            IMPORT_ITEM,
            json!({ "input": {
                "itemId": item_labelled(&results, "Another Road")["id"],
                "libraryItemId": second,
            } }),
        )
        .await;
    assert_eq!(
        imported["importIdentifiedItem"]["success"], true,
        "{imported}"
    );
    assert_eq!(
        imported["importIdentifiedItem"]["matchCount"], 2,
        "{imported}"
    );
    assert_eq!(files_under(&library_path).len(), 8);
    app.shutdown().await;
}

/// From a real release that crashed the server: a chapter starting in a part whose own chapter has
/// already moved on to a later part. The hooks used to chase each other until the stack ran out.
#[tokio::test(flavor = "multi_thread")]
async fn a_chapter_in_a_part_whose_named_chapter_moved_on_does_not_loop() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    let seeded = seed_with(&app, &mut admin, "The Long Way", |payload| {
        write_parts(payload, "The Long Way", 3, 120);
    })
    .await;
    let (library_id, library_path) = library(&app, &mut admin, "books", "AUDIOBOOKS").await;
    let owner = admin.user_id.clone().expect("admin id");
    let book = playback::create(
        &mut admin,
        "Audiobook",
        json!({
            "libraryId": library_id, "userId": owner, "title": "The Long Way",
            "authorName": "An Author", "autoDownload": false, "autoDownloadMode": "NONE",
            "hasFiles": false, "narrators": [],
        }),
    )
    .await;
    // Parts start at 0, 120 and 240 s. The second and third chapters both start in part 2, and the
    // third runs on through part 3.
    for (number, title, start, seconds) in [
        (1, "One", 0.0, 150),
        (2, "Two", 150.0, 50),
        (3, "Three", 200.0, 160),
    ] {
        playback::create(
            &mut admin,
            "Chapter",
            json!({
                "audiobookId": book, "chapterNumber": number, "title": title,
                "startTimeSecs": start, "durationSecs": seconds, "wanted": true,
            }),
        )
        .await;
    }
    link_to_book(&mut admin, &seeded, &book, &library_id).await;
    let processed = process(&mut admin, &seeded).await;
    assert_eq!(
        processed["processSource"]["filesProcessed"], 3,
        "{processed}"
    );
    assert_eq!(files_under(&library_path).len(), 3);

    let placed = admin.query(PLACED, json!({ "id": book })).await;
    let rows = placed["audiobook"]["chapters"]["edges"].as_array().unwrap();
    let file = |index: usize| rows[index]["node"]["mediaFileId"].as_str().expect("linked");
    assert_eq!(file(1), file(2), "both start in part 2: {placed}");
    assert_ne!(file(0), file(1), "{placed}");
    assert!(
        (rows[1]["node"]["fileOffsetSecs"].as_f64().unwrap() - 30.0).abs() < 1.0,
        "{placed}"
    );
    assert!(
        (rows[2]["node"]["fileOffsetSecs"].as_f64().unwrap() - 80.0).abs() < 1.0,
        "{placed}"
    );
    let chapter_ids = placed_chapter_ids(&mut admin, &book).await;
    let files = admin
        .query(
            r#"query F($ids: [String!]!) { mediaFiles(where: { chapterId: { inList: $ids } }, page: { limit: 50, offset: 0 }) { edges { node { id chapterId } } } }"#,
            json!({ "ids": chapter_ids }),
        )
        .await;
    assert_eq!(
        files["mediaFiles"]["edges"].as_array().unwrap().len(),
        3,
        "{files}"
    );
    app.shutdown().await;
}

/// One real torrent grew to 13,164 records for one file, because each re-import added another.
#[tokio::test(flavor = "multi_thread")]
async fn importing_a_download_again_does_not_add_records_for_the_same_file() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    let seeded = seed_with(&app, &mut admin, "The Long Way", |payload| {
        write_parts(payload, "The Long Way", 3, 60);
    })
    .await;
    let (library_id, library_path) = library(&app, &mut admin, "books", "AUDIOBOOKS").await;
    let (book, _) = audiobook_with_chapters(&mut admin, &library_id, "The Long Way", 3).await;
    link_to_book(&mut admin, &seeded, &book, &library_id).await;
    const COUNT: &str =
        r#"query { mediaFiles(page: { limit: 100, offset: 0 }) { edges { node { id path } } } }"#;
    for attempt in 1..=3 {
        let processed = process(&mut admin, &seeded).await;
        assert_eq!(
            processed["processSource"]["filesProcessed"], 3,
            "attempt {attempt}: {processed}"
        );
        let found = admin.query(COUNT, json!({})).await;
        let rows = found["mediaFiles"]["edges"].as_array().unwrap();
        let paths: std::collections::HashSet<&str> = rows
            .iter()
            .filter_map(|row| row["node"]["path"].as_str())
            .collect();
        assert_eq!(rows.len(), 3, "attempt {attempt}: {found}");
        assert_eq!(paths.len(), 3, "attempt {attempt}: {found}");
    }
    assert_eq!(files_under(&library_path).len(), 3);
    let placed = admin.query(PLACED, json!({ "id": book })).await;
    for row in placed["audiobook"]["chapters"]["edges"].as_array().unwrap() {
        assert!(row["node"]["mediaFileId"].is_string(), "{placed}");
    }
    app.shutdown().await;
}

/// The key is write-only through GraphQL, encrypted at rest, and for administrators alone.
#[tokio::test(flavor = "multi_thread")]
async fn the_jev_key_is_stored_encrypted_and_cannot_be_read_back() {
    let app = TestApp::start().await;
    let mut admin = app.admin_client().await;
    const SECRET: &str = "ts-not-a-real-key-0123456789";
    let created = admin
        .query(
            r#"mutation C($input: CreateMatchingAssistantInput!) {
                createMatchingAssistant(input: $input) { success error matchingAssistant { id enabled model hasApiKey } }
            }"#,
            json!({ "input": { "enabled": true, "model": "jev-latest", "apiKey": SECRET, "hasApiKey": false } }),
        )
        .await;
    let row = &created["createMatchingAssistant"]["matchingAssistant"];
    assert_eq!(row["hasApiKey"], true, "{created}");
    let id = row["id"].as_str().unwrap().to_string();

    // Not a field that can be asked for at all.
    let refused = admin
        .post(
            r#"query Q($id: String!) { matchingAssistant(id: $id) { apiKey } }"#,
            json!({ "id": id }),
        )
        .await;
    assert!(
        refused.body["errors"].is_array(),
        "apiKey must not be selectable: {}",
        refused.body
    );
    assert!(!refused.body.to_string().contains(SECRET));

    // On disk it is an encrypted envelope, not the key.
    let database = app.services.get_database().await.expect("database");
    let stored = librarian::graphql::entities::MatchingAssistant::query(database.pool().pool())
        .fetch_all()
        .await
        .unwrap()
        .remove(0);
    assert_ne!(stored.api_key, SECRET);
    assert!(stored.api_key.starts_with("v2:"), "not an envelope");
    assert!(!stored.api_key.contains(SECRET));

    // Changing a setting keeps the key, and clearing it says so.
    let updated = admin
        .query(
            r#"mutation U($id: String!, $input: UpdateMatchingAssistantInput!) {
                updateMatchingAssistant(id: $id, input: $input) { success matchingAssistant { enabled hasApiKey } }
            }"#,
            json!({ "id": id, "input": { "enabled": false, "hasApiKey": false } }),
        )
        .await;
    assert_eq!(
        updated["updateMatchingAssistant"]["matchingAssistant"]["hasApiKey"], true,
        "{updated}"
    );
    let cleared = admin
        .query(
            r#"mutation U($id: String!, $input: UpdateMatchingAssistantInput!) {
                updateMatchingAssistant(id: $id, input: $input) { success matchingAssistant { hasApiKey } }
            }"#,
            json!({ "id": id, "input": { "apiKey": "" } }),
        )
        .await;
    assert_eq!(
        cleared["updateMatchingAssistant"]["matchingAssistant"]["hasApiKey"], false,
        "{cleared}"
    );
    app.shutdown().await;
}

async fn placed_chapter_ids(admin: &mut GqlClient, book: &str) -> Vec<String> {
    let found = admin
        .query(
            r#"query C($id: String!) { audiobook(id: $id) { chapters(page: { limit: 50, offset: 0 }) { edges { node { id } } } } }"#,
            json!({ "id": book }),
        )
        .await;
    found["audiobook"]["chapters"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| edge["node"]["id"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_ebook_release_fails_so_another_can_be_tried() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    let seeded = seed(&app, &mut admin, "The Long Way.epub").await;
    let (library_id, library_path) = library(&app, &mut admin, "books", "AUDIOBOOKS").await;
    let (book, _) = audiobook_with_chapters(&mut admin, &library_id, "The Long Way", 3).await;
    link_to_book(&mut admin, &seeded, &book, &library_id).await;

    let preview = admin
        .query(PREVIEW, json!({ "infoHash": seeded.hash }))
        .await;
    assert_eq!(
        preview["torrentMatchPreview"]["status"], "review",
        "{preview}"
    );
    assert!(
        preview["torrentMatchPreview"]["summary"]
            .as_str()
            .unwrap()
            .contains(".epub"),
        "{preview}"
    );

    process(&mut admin, &seeded).await;
    let state = admin.query(STATE, json!({ "id": seeded.id })).await;
    assert_eq!(state["torrent"]["postProcessStatus"], "failed", "{state}");
    assert!(files_under(&library_path).is_empty());
    app.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_wrong_book_waits_for_a_person_and_can_be_accepted() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    identify_offline(&mut admin).await;
    let seeded = seed_with(&app, &mut admin, "Another Road", |payload| {
        write_parts(payload, "Another Road", 3, 60);
    })
    .await;
    let (library_id, library_path) = library(&app, &mut admin, "books", "AUDIOBOOKS").await;
    let (book, _) = audiobook_with_chapters(&mut admin, &library_id, "The Long Way", 3).await;
    link_to_book(&mut admin, &seeded, &book, &library_id).await;

    process(&mut admin, &seeded).await;
    let state = admin.query(STATE, json!({ "id": seeded.id })).await;
    assert_eq!(
        state["torrent"]["postProcessStatus"], "needs_review",
        "{state}"
    );
    assert!(
        state["torrent"]["postProcessError"]
            .as_str()
            .unwrap()
            .contains("Another Road"),
        "{state}"
    );
    assert!(
        files_under(&library_path).is_empty(),
        "nothing moves until someone decides"
    );

    // Identification doesn't bring it in either: it was asked for, but its tags say otherwise.
    run_identification(&mut admin, &seeded.hash).await;
    let results = results_when(&mut admin, &seeded.hash, |items| {
        items
            .iter()
            .any(|item| item["label"] == "Another Road" && item["kind"] == "audiobook")
    })
    .await;
    let item = item_labelled(&results, "Another Road");
    assert_eq!(item["status"], "needs_choice", "{results}");
    assert_eq!(item["canAccept"], true, "{results}");
    assert!(files_under(&library_path).is_empty(), "{results}");

    let accepted = admin
        .query(
            IMPORT_ITEM,
            json!({ "input": { "itemId": item["id"], "libraryItemId": book, "acceptReview": true } }),
        )
        .await;
    let state = admin.query(STATE, json!({ "id": seeded.id })).await;
    assert_eq!(
        accepted["importIdentifiedItem"]["success"], true,
        "{accepted} {state}"
    );
    assert_eq!(files_under(&library_path).len(), 3);
    app.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_files_own_chapters_replace_the_listing_and_can_be_undone() {
    let app = TestApp::start_with(TestAppOptions {
        with_torrent: true,
        ..Default::default()
    })
    .await;
    let mut admin = app.admin_client().await;
    let seeded = seed_with(&app, &mut admin, "The Long Way.m4b", |payload| {
        write_audio(payload, 2, &[("album", "The Long Way".to_string())]);
        // A file with its own chapter markers, which ffmpeg reads from a metadata file.
        let meta = payload.with_extension("txt");
        std::fs::write(
            &meta,
            ";FFMETADATA1\n[CHAPTER]\nTIMEBASE=1/1\nSTART=0\nEND=120\ntitle=Enlist\n[CHAPTER]\nTIMEBASE=1/1\nSTART=120\nEND=180\ntitle=Serve\n",
        )
        .unwrap();
        let out = payload.with_extension("tmp.m4b");
        let output = std::process::Command::new(
            std::env::var("LIBRARIAN_ACCEPTANCE_FFMPEG").unwrap_or_else(|_| "ffmpeg".to_string()),
        )
        .args(["-y", "-loglevel", "error", "-f", "lavfi", "-i", "anullsrc=r=8000:cl=mono", "-i"])
        .arg(&meta)
        .args(["-map_metadata", "1", "-t", "180", "-c:a", "aac", "-f", "mp4"])
        .arg(&out)
        .output()
        .expect("ffmpeg");
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        std::fs::rename(&out, payload).unwrap();
        std::fs::remove_file(&meta).unwrap();
    })
    .await;
    let (library_id, _) = library(&app, &mut admin, "books", "AUDIOBOOKS").await;
    let (book, _) = audiobook_with_chapters(&mut admin, &library_id, "The Long Way", 3).await;
    link_to_book(&mut admin, &seeded, &book, &library_id).await;
    let processed = process(&mut admin, &seeded).await;
    assert_eq!(
        processed["processSource"]["filesProcessed"], 1,
        "{processed}"
    );

    let placed = admin.query(PLACED, json!({ "id": book })).await;
    assert_eq!(placed["audiobook"]["chapterSource"], "embedded", "{placed}");
    let rows = placed["audiobook"]["chapters"]["edges"].as_array().unwrap();
    assert_eq!(rows.len(), 2, "{placed}");
    assert_eq!(rows[1]["node"]["title"], "Serve", "{placed}");
    assert_eq!(rows[1]["node"]["fileOffsetSecs"], 120.0, "{placed}");
    assert!(rows[1]["node"]["mediaFileId"].is_string(), "{placed}");
    assert!(
        placed["audiobook"]["providerChapters"].is_string(),
        "kept for undoing: {placed}"
    );

    let restored = admin
        .query(
            r#"mutation R($id: String!) { restoreAudiobookChapters(audiobookId: $id) { success error } }"#,
            json!({ "id": book }),
        )
        .await;
    assert_eq!(
        restored["restoreAudiobookChapters"]["success"], true,
        "{restored}"
    );
    let placed = admin.query(PLACED, json!({ "id": book })).await;
    assert_eq!(placed["audiobook"]["chapterSource"], "provider", "{placed}");
    let rows = placed["audiobook"]["chapters"]["edges"].as_array().unwrap();
    assert_eq!(rows.len(), 3, "{placed}");
    assert_eq!(rows[2]["node"]["title"], "Part 3", "{placed}");
    assert!(rows[2]["node"]["mediaFileId"].is_string(), "{placed}");
    app.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_running_download_shows_on_its_titles_until_they_have_a_file() {
    let app = TestApp::start().await;
    let mut admin = app.admin_client().await;
    let (library_id, _) = library(&app, &mut admin, "books", "AUDIOBOOKS").await;
    let (book, chapters) =
        audiobook_with_chapters(&mut admin, &library_id, "The Long Way", 2).await;
    let (other, other_chapters) =
        audiobook_with_chapters(&mut admin, &library_id, "Another", 1).await;
    assert_ne!(book, other);
    let owner = admin.user_id.clone().expect("admin id");
    let torrent = playback::create(
        &mut admin,
        "Torrent",
        json!({
            "userId": owner,
            "infoHash": "a".repeat(40),
            "name": "The Long Way",
            "state": "downloading",
            "progress": 0.36,
            "totalBytes": 100,
            "downloadedBytes": 36,
            "uploadedBytes": 0,
            "uploadedBytesTotal": 0,
            "deleteFilesOnRemove": false,
            "savePath": "/tmp/none",
            "excludedFiles": [],
            "addedAt": "2026-11-01T00:00:00Z",
            "audiobookId": book,
        }),
    )
    .await;
    const STATUS: &str = r#"query S($inputs: [ContentStatusRequestInput!]!) {
      contentStatuses(inputs: $inputs) { id status downloadProgress }
    }"#;
    let inputs = json!({ "inputs": [
        { "contentType": "CHAPTER", "id": chapters[0] },
        { "contentType": "CHAPTER", "id": other_chapters[0] },
    ]});
    let found = admin.query(STATUS, inputs.clone()).await;
    let rows = found["contentStatuses"].as_array().unwrap();
    let row = |id: &str| rows.iter().find(|row| row["id"] == id).unwrap().clone();
    assert_eq!(row(&chapters[0])["status"], "DOWNLOADING", "{found}");
    assert_eq!(row(&chapters[0])["downloadProgress"], 0.36, "{found}");
    assert_ne!(row(&other_chapters[0])["status"], "DOWNLOADING", "{found}");
    assert!(
        row(&other_chapters[0])["downloadProgress"].is_null(),
        "{found}"
    );

    // Finished, or paused: not downloading any more.
    let paused = admin
        .query(
            r#"mutation U($id: String!, $input: UpdateTorrentInput!) { updateTorrent(id: $id, input: $input) { success error } }"#,
            json!({ "id": torrent, "input": { "state": "paused" } }),
        )
        .await;
    assert_eq!(paused["updateTorrent"]["success"], true, "{paused}");
    let found = admin.query(STATUS, inputs).await;
    assert!(
        found["contentStatuses"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["downloadProgress"].is_null()),
        "{found}"
    );
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
