//! The `app_settings` contract: how values are encoded, what a fresh database
//! is seeded with, and how the environment overrides those seeds.
//!
//! `app_settings` rows outrank the process configuration for most of the
//! torrent service's knobs, so what is in this table on first boot decides
//! where a real deployment downloads to and which port it binds. These
//! assertions are about that table specifically, through the same admin API an
//! operator would use.

mod common;

use std::sync::{Mutex, MutexGuard};

use common::{GqlClient, TestApp};
use serde_json::{Value, json};

/// `seed_app_settings` reads process-wide environment variables, so only one
/// test in this binary may be manipulating them at a time.
static ENVIRONMENT: Mutex<()> = Mutex::new(());

fn lock_environment() -> MutexGuard<'static, ()> {
    ENVIRONMENT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn seeded_defaults_cover_the_torrent_extraction_and_download_knobs() {
    let _guard = lock_environment();
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;
        let settings = all_settings(&mut admin).await;

        // Seeding, and the values a fresh install runs with. `seed_ratio_limit`
        // and `seed_time_minutes` are the pair `enforce_seeding_policy` reads;
        // `remove_after_import` is the one that deletes payloads, so its
        // default has to stay conservative.
        for (key, expected, category) in [
            ("torrent.seed_ratio_limit", "1.0", "torrent"),
            ("torrent.seed_time_minutes", "0", "torrent"),
            ("torrent.remove_after_import", "false", "torrent"),
            ("extract.max_unpacked_gb", "60", "torrent"),
            ("extract.max_entries", "20000", "torrent"),
        ] {
            let row = settings
                .get(key)
                .unwrap_or_else(|| panic!("{key} should be seeded; saw {:?}", keys(&settings)));
            assert_eq!(row["value"], expected, "{key} default changed");
            assert_eq!(row["category"], category, "{key} category changed");
            assert!(
                row["description"].as_str().is_some_and(|d| !d.is_empty()),
                "{key} needs a description: an operator edits these by hand"
            );
        }

        // `remove_after_import` on its own does nothing unless one of the
        // seeding rules can ever be satisfied — the shipped defaults must not
        // be a combination that silently never cleans up.
        assert_eq!(settings["torrent.remove_after_import"]["value"], "false");

        // The `auto_download.*` keys are deliberately *not* seeded: their
        // defaults live in `AutoDownloadServiceConfig`, and the absence of a
        // row is what keeps the unattended hunt disabled on a fresh install.
        // A row appearing here would silently turn automation on.
        for key in [
            "auto_download.enabled",
            "auto_download.interval_minutes",
            "auto_download.retry_sweep_interval_minutes",
            "auto_download.retry_after_minutes",
            "auto_download.retry_window_days",
        ] {
            assert!(
                !settings.contains_key(key),
                "{key} is not seeded on purpose; if that changed, check the \
                 auto-download safety default still holds"
            );
        }

        // `auto_download.search_state` is runtime state, not a seed.
        assert!(!settings.contains_key("auto_download.search_state"));

        app.shutdown().await;
    });
}

#[test]
fn string_settings_are_json_quoted_in_storage_and_unquoted_when_decoded() {
    let _guard = lock_environment();
    common::run_app_test(|| async {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;
        let settings = all_settings(&mut admin).await;

        // Every value in the column is JSON text. Path-like settings are
        // therefore stored *with* their quotes.
        for key in ["torrent.download_dir", "torrent.session_dir"] {
            let raw = settings[key]["value"].as_str().expect("value");
            assert!(
                raw.starts_with('"') && raw.ends_with('"'),
                "{key} should be stored as a JSON string, got {raw}"
            );

            // Regression guard for payloads landing in `<cwd>/"/data/downloads"`:
            // a consumer that forgets to JSON-decode ends up with the quote
            // characters inside a PathBuf. The decoded value must never
            // contain one.
            let decoded: String =
                serde_json::from_str(raw).unwrap_or_else(|e| panic!("{key} is not JSON: {e}"));
            assert!(
                !decoded.contains('"'),
                "decoded {key} still contains a quote: {decoded}"
            );
            assert!(
                std::path::Path::new(&decoded).is_absolute(),
                "decoded {key} should be an absolute path, got {decoded}"
            );
        }

        // Non-string settings are bare JSON scalars, not quoted strings — a
        // quoted `"6881"` would fail `get_setting::<u16>`.
        for (key, expected) in [
            ("torrent.listen_port", json!(6881)),
            ("torrent.enable_dht", json!(true)),
            ("torrent.seed_ratio_limit", json!(1.0)),
            ("extract.max_entries", json!(20000)),
        ] {
            let raw = settings[key]["value"].as_str().expect("value");
            assert!(
                !raw.starts_with('"'),
                "{key} must not be stored as a quoted string, got {raw}"
            );
            let decoded: Value = serde_json::from_str(raw)
                .unwrap_or_else(|e| panic!("{key} is not valid JSON: {e}"));
            assert_eq!(decoded, expected, "{key}");
        }

        // Every seeded value in the table must parse as JSON, whatever its
        // type — this is the invariant the decoder relies on.
        for (key, row) in &settings {
            let raw = row["value"].as_str().expect("value");
            serde_json::from_str::<Value>(raw)
                .unwrap_or_else(|e| panic!("app_setting {key} holds non-JSON {raw:?}: {e}"));
        }

        app.shutdown().await;
    });
}

#[test]
fn the_environment_seeds_the_torrent_paths_and_port_on_a_fresh_database() {
    let _guard = lock_environment();

    let downloads = tempfile::tempdir().expect("downloads dir");
    let session = tempfile::tempdir().expect("session dir");
    let downloads_path = downloads.path().to_string_lossy().to_string();
    let session_path = session.path().to_string_lossy().to_string();

    // SAFETY: `ENVIRONMENT` serialises every test in this binary that reads or
    // writes these variables, and each one is restored below.
    unsafe {
        std::env::set_var("TORRENT_LISTEN_PORT", "51820");
        std::env::set_var("DOWNLOADS_PATH", &downloads_path);
        std::env::set_var("SESSION_PATH", &session_path);
    }

    let outcome = std::panic::catch_unwind(|| {
        let downloads_path = downloads_path.clone();
        let session_path = session_path.clone();
        common::run_app_test(move || async move {
            // No torrent service: the harness rewrites exactly these rows when
            // one is registered, which would mask what is being tested.
            let app = TestApp::start().await;
            let mut admin = app.admin_client().await;
            let settings = all_settings(&mut admin).await;

            // A port is a bare number; the paths are JSON strings.
            assert_eq!(
                settings["torrent.listen_port"]["value"], "51820",
                "TORRENT_LISTEN_PORT should have replaced the 6881 seed"
            );
            assert_eq!(
                settings["torrent.download_dir"]["value"],
                json!(downloads_path).to_string(),
                "DOWNLOADS_PATH should have replaced the /data/downloads seed"
            );
            assert_eq!(
                settings["torrent.session_dir"]["value"],
                json!(session_path).to_string(),
                "SESSION_PATH should have replaced the /data/session seed"
            );

            // Only the three mapped keys are affected; the rest keep their
            // static seeds.
            assert_eq!(settings["torrent.seed_ratio_limit"]["value"], "1.0");
            assert_eq!(settings["torrent.enable_dht"]["value"], "true");

            // An operator's own value outranks the environment: seeding only
            // replaces a row that still holds the untouched static seed.
            admin
                .query(
                    r#"mutation U($id: String!) {
                         updateAppSetting(id: $id, input: { value: "\"/mnt/chosen-by-hand\"" })
                           { success error }
                       }"#,
                    json!({ "id": settings["torrent.download_dir"]["id"] }),
                )
                .await;

            // Re-seeding is what happens on the next boot.
            let reseeded = restart_and_reload(&app, &mut admin).await;
            assert_eq!(
                reseeded["torrent.download_dir"]["value"], "\"/mnt/chosen-by-hand\"",
                "an edited setting must survive re-seeding, environment or not"
            );
            assert_eq!(
                reseeded["torrent.session_dir"]["value"],
                json!(session_path).to_string(),
                "an untouched row still tracks the environment"
            );

            app.shutdown().await;
        });
    });

    // SAFETY: as above — restore before any other test can observe them.
    unsafe {
        std::env::remove_var("TORRENT_LISTEN_PORT");
        std::env::remove_var("DOWNLOADS_PATH");
        std::env::remove_var("SESSION_PATH");
    }

    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Every `app_settings` row, keyed by `key`.
async fn all_settings(admin: &mut GqlClient) -> std::collections::BTreeMap<String, Value> {
    let data = admin
        .query(
            "query { appSettings(page: { limit: 500, offset: 0 }) \
             { edges { node { id key value description category } } } }",
            json!({}),
        )
        .await;
    data["appSettings"]["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .map(|edge| {
            let node = edge["node"].clone();
            (node["key"].as_str().expect("key").to_string(), node)
        })
        .collect()
}

fn keys(settings: &std::collections::BTreeMap<String, Value>) -> Vec<&String> {
    settings.keys().collect()
}

/// Run the seeding pass again, as a restart would, and re-read the table.
async fn restart_and_reload(
    app: &TestApp,
    admin: &mut GqlClient,
) -> std::collections::BTreeMap<String, Value> {
    let database = app.services.get_database().await.expect("database service");
    librarian::services::Service::start(database.as_ref())
        .await
        .expect("re-running the database service start should re-seed");
    all_settings(admin).await
}
