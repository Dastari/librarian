//! Acquisition pipeline contract: the manual "search now" mutations and the
//! auto-download retry sweep.
//!
//! The pure decision functions (`needs_retry`, `blocklist_reason`,
//! `seeding::*`) already have unit tests. What is only observable end to end —
//! and what this file covers — is the sweep run against the real SQLite
//! schema and service manager: that it finds torrents through the generated
//! ORM query, writes blocklist rows with the right reason and linkage, leaves
//! healthy grabs alone, and is safe to run twice.

mod common;

use std::sync::Arc;

use chrono::Utc;
use common::{TestApp, TestAppOptions};
use serde_json::{Value, json};

/// RFC3339 with milliseconds — the format `added_at` is stored in.
fn hours_ago(hours: i64) -> String {
    (Utc::now() - chrono::Duration::hours(hours))
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

const CREATE_TORRENT: &str = r#"mutation C($input: CreateTorrentInput!) {
  createTorrent(input: $input) { success error torrent { id infoHash } }
}"#;

#[test]
fn search_missing_reports_a_missing_service_and_a_missing_target() {
    common::run_app_test(|| async {
        // No auto-download service registered: the mutation must say so
        // rather than silently reporting "nothing to do".
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;

        // Both mutations resolve to an error rather than a zero-result
        // success, so an admin pressing "search" on a server without the
        // service is told something is wrong. (The message itself is masked
        // by the error sanitiser, so only the failure is asserted.)
        let unavailable = admin
            .post(
                "mutation { searchMissing(input: { showId: \"missing\" }) \
                 { success error queued searched } }",
                json!({}),
            )
            .await;
        assert_eq!(unavailable.status, 200);
        assert!(
            !unavailable.errors().is_empty(),
            "searchMissing should fail without the auto-download service: {}",
            unavailable.body
        );
        assert!(unavailable.body["data"]["searchMissing"].is_null());

        let no_trigger = admin
            .post("mutation { triggerAutoDownload { success } }", json!({}))
            .await;
        assert!(
            !no_trigger.errors().is_empty(),
            "triggerAutoDownload should also fail without the service: {}",
            no_trigger.body
        );

        app.shutdown().await;
    });
}

#[test]
fn search_missing_runs_a_real_pass_when_the_service_is_registered() {
    common::run_app_test(|| async {
        let app = TestApp::start_with(TestAppOptions {
            with_auto_download: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let admin_id = admin.user_id.clone().expect("admin id");

        // An input with no target is a user error, not a GraphQL error.
        let empty = admin
            .query(
                "mutation { searchMissing(input: {}) { success error queued searched } }",
                json!({}),
            )
            .await;
        assert_eq!(empty["searchMissing"]["success"], false);
        assert!(
            empty["searchMissing"]["error"]
                .as_str()
                .unwrap_or_default()
                .contains("At least one of"),
            "expected the required-target message, got {}",
            empty["searchMissing"]["error"]
        );
        assert_eq!(empty["searchMissing"]["queued"], 0);

        // A real target with no indexers configured: the hunt runs and finds
        // nothing, which is success with zero grabs — not an error.
        let library_id = create_library(&mut admin, &admin_id).await;
        let show_id = create_show(&mut admin, &admin_id, &library_id, "Sweep Show").await;

        let searched = admin
            .query(
                "mutation S($id: String!) { searchMissing(input: { showId: $id }) \
                 { success error queued searched } }",
                json!({ "id": show_id }),
            )
            .await;
        assert_eq!(searched["searchMissing"]["success"], true, "{searched}");
        assert_eq!(searched["searchMissing"]["queued"], 0);

        // The whole-library pass has the same shape.
        let triggered = admin
            .query(
                "mutation T($id: String!) { triggerAutoDownload(libraryId: $id) \
                 { success grabbed searched candidatesConsidered errors error } }",
                json!({ "id": library_id }),
            )
            .await;
        assert_eq!(
            triggered["triggerAutoDownload"]["success"], true,
            "{triggered}"
        );
        assert_eq!(triggered["triggerAutoDownload"]["grabbed"], 0);

        app.shutdown().await;
    });
}

#[test]
fn retry_sweep_blocklists_dead_grabs_and_leaves_healthy_ones_alone() {
    common::run_app_test(|| async {
        let app = TestApp::start_with(TestAppOptions {
            with_torrent: true,
            with_auto_download: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let admin_id = admin.user_id.clone().expect("admin id");
        let library_id = create_library(&mut admin, &admin_id).await;
        let show_id = create_show(&mut admin, &admin_id, &library_id, "Dead Grabs").await;

        let base = json!({
            "userId": admin_id,
            "state": "downloading",
            "progress": 0.0,
            "totalBytes": 1_000_000,
            "downloadedBytes": 0,
            "uploadedBytes": 0,
            "uploadedBytesTotal": 0,
            "savePath": "/tmp/downloads",
            "excludedFiles": [],
        });
        let with = |extra: Value| -> Value {
            let mut input = base.as_object().expect("base object").clone();
            for (key, value) in extra.as_object().expect("extra object") {
                input.insert(key.clone(), value.clone());
            }
            json!({ "input": input })
        };

        // 1. Import failed with a wanted linkage -> blocklisted "import_failed".
        admin
            .query(
                CREATE_TORRENT,
                with(json!({
                    "infoHash": "1000000000000000000000000000000000000001",
                    "name": "Dead Grabs S01E01 FAILED",
                    "addedAt": hours_ago(2),
                    "showId": show_id,
                    "postProcessStatus": "failed",
                })),
            )
            .await;

        // 2. No progress for two days with a wanted linkage -> "stalled".
        admin
            .query(
                CREATE_TORRENT,
                with(json!({
                    "infoHash": "2000000000000000000000000000000000000002",
                    "name": "Dead Grabs S01E02 STALLED",
                    "addedAt": hours_ago(48),
                    "showId": show_id,
                })),
            )
            .await;

        // 3. Same failure, but a manual add (no linkage) -> left alone. The
        //    blocklist only exists to steer the automatic hunt.
        admin
            .query(
                CREATE_TORRENT,
                with(json!({
                    "infoHash": "3000000000000000000000000000000000000003",
                    "name": "Manual Add FAILED",
                    "addedAt": hours_ago(48),
                    "postProcessStatus": "failed",
                })),
            )
            .await;

        // 4. Stalled-looking but actually progressing -> left alone.
        admin
            .query(
                CREATE_TORRENT,
                with(json!({
                    "infoHash": "4000000000000000000000000000000000000004",
                    "name": "Dead Grabs S01E03 HEALTHY",
                    "addedAt": hours_ago(48),
                    "showId": show_id,
                    "progress": 0.4,
                    "downloadedBytes": 400_000,
                })),
            )
            .await;

        // 5. Already imported -> terminal, never blocklisted or retried.
        admin
            .query(
                CREATE_TORRENT,
                with(json!({
                    "infoHash": "5000000000000000000000000000000000000005",
                    "name": "Dead Grabs S01E04 DONE",
                    "addedAt": hours_ago(48),
                    "showId": show_id,
                    "progress": 1.0,
                    "downloadedBytes": 1_000_000,
                    "postProcessStatus": "completed",
                })),
            )
            .await;

        let services: Arc<_> = app.services.clone();
        librarian::jobs::download_monitor::run_retry_sweep(&services, 60, 7)
            .await
            .expect("retry sweep should succeed");

        let expected = [
            ("1000000000000000000000000000000000000001", "import_failed"),
            ("2000000000000000000000000000000000000002", "stalled"),
        ]
        .map(|(hash, reason)| (hash.to_string(), reason.to_string()))
        .to_vec();
        assert_eq!(
            blocklist_rows(&mut admin).await,
            expected,
            "only linked, genuinely dead grabs should be blocklisted"
        );

        // The blocklist row carries the linkage so the hunt can scope the
        // exclusion to the right target.
        let detail = admin
            .query(
                r#"query { releaseBlocklists(where: { reason: { eq: "stalled" } })
                     { edges { node { showId title sourceId expiresAt } } } }"#,
                json!({}),
            )
            .await;
        let node = &detail["releaseBlocklists"]["edges"][0]["node"];
        assert_eq!(node["showId"], show_id);
        assert_eq!(node["title"], "Dead Grabs S01E02 STALLED");
        assert!(
            node["expiresAt"].is_null(),
            "dead grabs are blocked forever"
        );

        // Running the sweep again must not duplicate rows: `already_blocklisted`
        // is what keeps a permanently dead grab from growing the table on
        // every sweep interval.
        librarian::jobs::download_monitor::run_retry_sweep(&services, 60, 7)
            .await
            .expect("second retry sweep should succeed");
        assert_eq!(
            blocklist_rows(&mut admin).await.len(),
            2,
            "the sweep must be idempotent"
        );

        app.shutdown().await;
    });
}

#[test]
fn retry_sweep_ignores_torrents_outside_the_retry_window() {
    common::run_app_test(|| async {
        let app = TestApp::start_with(TestAppOptions {
            with_torrent: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;
        let admin_id = admin.user_id.clone().expect("admin id");
        let library_id = create_library(&mut admin, &admin_id).await;
        let show_id = create_show(&mut admin, &admin_id, &library_id, "Old Grabs").await;

        // 30 days old: outside the 7-day window, so even though it is stalled
        // and linked it is never touched again. Without the window the sweep
        // would re-hunt years of history on every pass.
        admin
            .query(
                CREATE_TORRENT,
                json!({ "input": {
                    "userId": admin_id,
                    "infoHash": "9000000000000000000000000000000000000009",
                    "name": "Old Grabs S01E01",
                    "state": "downloading",
                    "progress": 0.0,
                    "totalBytes": 1_000_000,
                    "downloadedBytes": 0,
                    "uploadedBytes": 0,
                    "uploadedBytesTotal": 0,
                    "savePath": "/tmp/downloads",
                    "excludedFiles": [],
                    "addedAt": hours_ago(30 * 24),
                    "showId": show_id,
                }}),
            )
            .await;

        librarian::jobs::download_monitor::run_retry_sweep(&app.services.clone(), 60, 7)
            .await
            .expect("retry sweep should succeed");

        assert!(
            blocklist_rows(&mut admin).await.is_empty(),
            "torrents older than the retry window must be ignored"
        );

        app.shutdown().await;
    });
}

#[test]
fn retry_sweep_is_a_no_op_without_the_torrent_service() {
    common::run_app_test(|| async {
        // The sweep needs a download client to re-process anything, so it
        // must fail loudly rather than half-run when one is not registered.
        let app = TestApp::start().await;
        let _ = app.admin_client().await;

        let result =
            librarian::jobs::download_monitor::run_retry_sweep(&app.services.clone(), 60, 7).await;
        let error = result.expect_err("sweep should fail without a torrent service");
        assert!(
            error.to_string().contains("torrent service not available"),
            "unexpected error: {error}"
        );

        app.shutdown().await;
    });
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

async fn create_library(admin: &mut common::GqlClient, user_id: &str) -> String {
    let created = admin
        .query(
            r#"mutation C($input: CreateLibraryInput!) {
                 createLibrary(input: $input) { success error library { id } }
               }"#,
            json!({ "input": {
                "userId": user_id,
                "name": "Acquisition",
                "path": "/tmp/acquisition",
                "libraryType": "tv",
                "autoScan": false,
                "autoOrganize": false,
                "namingPattern": "",
                "scanIntervalMinutes": 60,
                "watchForChanges": false,
                "scanning": false
            }}),
        )
        .await;
    assert_eq!(created["createLibrary"]["success"], true, "{created}");
    created["createLibrary"]["library"]["id"]
        .as_str()
        .expect("library id")
        .to_string()
}

async fn create_show(
    admin: &mut common::GqlClient,
    user_id: &str,
    library_id: &str,
    name: &str,
) -> String {
    let created = admin
        .query(
            r#"mutation C($input: CreateShowInput!) {
                 createShow(input: $input) { success error show { id name } }
               }"#,
            json!({ "input": {
                "userId": user_id,
                "libraryId": library_id,
                "name": name,
                "genres": [],
                // Wanted-mode is what makes the show a hunt candidate.
                "autoDownload": true,
                "autoDownloadMode": "WANTED"
            }}),
        )
        .await;
    assert_eq!(created["createShow"]["success"], true, "{created}");
    created["createShow"]["show"]["id"]
        .as_str()
        .expect("show id")
        .to_string()
}

/// `(infoHash, reason)` for every blocklist row, sorted for stable asserts.
async fn blocklist_rows(admin: &mut common::GqlClient) -> Vec<(String, String)> {
    let data = admin
        .query(
            "query { releaseBlocklists { edges { node { infoHash reason } } } }",
            json!({}),
        )
        .await;
    // `infoHash` is not a sortable column, so sort client-side.
    let mut rows: Vec<(String, String)> = data["releaseBlocklists"]["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|edge| {
            (
                edge["node"]["infoHash"].as_str().unwrap_or("").to_string(),
                edge["node"]["reason"].as_str().unwrap_or("").to_string(),
            )
        })
        .collect();
    rows.sort();
    rows
}
