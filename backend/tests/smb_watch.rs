//! A watched library on a real SMB share (`docs/design.md` Q115 to Q117): files created, renamed
//! and deleted by another client are each picked up by a scan of just that place; hidden folders,
//! partial downloads and Librarian's own writes are not scanned; a copy still being written is not
//! scanned half written; and a share that drops is watched again when it is back, with one
//! catch-up scan of what changed meanwhile.
//!
//! Skipped unless a share is given, as `tests/smb_share.rs`:
//!
//! ```text
//! LIBRARIAN_SMB_TEST_URL=smb://127.0.0.1:4466/media
//! LIBRARIAN_SMB_TEST_USER=smbwatch
//! LIBRARIAN_SMB_TEST_PASSWORD_FILE=/path/to/password
//! cargo test --test smb_watch
//! ```
//!
//! The library reaches the share through a TCP relay in this test, so the share can be made to
//! drop (every connection cut, new ones refused) and come back without touching the server. Files
//! are changed through a separate SMB session straight to the server, as another computer would.
//! Everything happens in a throwaway `librarian-it-<id>` folder, removed at the end.
mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use common::{GqlClient, TestApp, playback};
use librarian::services::graphql::filesystem_network::{
    NetworkShareProvider, save_share_credentials,
};
use librarian::services::library_storage::{
    EntryKind, PlaceMode, SmbCredentials, SmbLocation, SmbStorage, Storage, StoragePath,
    WalkOptions, library_storage,
};
use serde_json::{Value, json};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

struct ShareEnv {
    url: String,
    user: String,
    password: String,
}

fn share_env() -> Option<ShareEnv> {
    let url = std::env::var("LIBRARIAN_SMB_TEST_URL").ok()?;
    let file = std::env::var("LIBRARIAN_SMB_TEST_PASSWORD_FILE").ok()?;
    let password = std::fs::read_to_string(&file)
        .expect("the password file named by LIBRARIAN_SMB_TEST_PASSWORD_FILE")
        .trim_end_matches(['\n', '\r'])
        .to_string();
    Some(ShareEnv {
        url,
        user: std::env::var("LIBRARIAN_SMB_TEST_USER").unwrap_or_else(|_| "toby".into()),
        password,
    })
}

/// A TCP relay to the SMB server that can drop every connection and refuse new ones.
struct Relay {
    port: u16,
    down: Arc<AtomicBool>,
    connections: Arc<Mutex<Vec<JoinHandle<()>>>>,
    _accept: JoinHandle<()>,
}

impl Relay {
    async fn start(upstream: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let down = Arc::new(AtomicBool::new(false));
        let connections: Arc<Mutex<Vec<JoinHandle<()>>>> = Arc::default();
        let accept = {
            let (down, connections) = (down.clone(), connections.clone());
            tokio::spawn(async move {
                loop {
                    let Ok((mut client, _)) = listener.accept().await else {
                        return;
                    };
                    if down.load(Ordering::SeqCst) {
                        drop(client);
                        continue;
                    }
                    let upstream = upstream.clone();
                    connections.lock().await.push(tokio::spawn(async move {
                        if let Ok(mut server) = TcpStream::connect(&upstream).await {
                            let _ = tokio::io::copy_bidirectional(&mut client, &mut server).await;
                        }
                    }));
                }
            })
        };
        Self {
            port,
            down,
            connections,
            _accept: accept,
        }
    }

    /// The share goes away: every connection is cut and new ones are refused.
    async fn cut(&self) {
        self.down.store(true, Ordering::SeqCst);
        for connection in self.connections.lock().await.drain(..) {
            connection.abort();
        }
    }

    fn restore(&self) {
        self.down.store(false, Ordering::SeqCst);
    }
}

async fn remove_tree(storage: &SmbStorage, root: &StoragePath) {
    let mut walk = storage.walk(root, WalkOptions::default(), CancellationToken::new());
    let mut folders = vec![root.clone()];
    while let Some(entry) = walk.entries.recv().await {
        let Ok(entry) = entry else { continue };
        match entry.kind {
            EntryKind::Directory => folders.push(entry.path),
            _ => {
                let _ = storage.remove_file(&entry.path).await;
            }
        }
    }
    let _ = walk.task.await;
    folders.sort_by_key(|path| std::cmp::Reverse(path.as_str().len()));
    for folder in folders {
        let _ = storage.remove_dir(&folder).await;
    }
}

/// Another computer writing to the share: a session of its own, straight to the server, so none
/// of it goes through Librarian's storage registry.
struct OtherClient {
    storage: SmbStorage,
    scratch: tempfile::TempDir,
}

impl OtherClient {
    async fn write(&self, path: &StoragePath, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            self.storage.create_dir_all(&parent).await.unwrap();
        }
        let local = self
            .scratch
            .path()
            .join(uuid::Uuid::new_v4().simple().to_string());
        std::fs::write(&local, bytes).unwrap();
        self.storage
            .put_from_local(&local, path, PlaceMode::Replace)
            .await
            .unwrap();
    }
}

struct Watched<'a> {
    admin: &'a mut GqlClient,
    library: String,
}

impl Watched<'_> {
    /// `(path below the library, size)` of every file record.
    async fn files(&mut self) -> Vec<(String, i64)> {
        let data = self
            .admin
            .query(
                "query($id:String!){mediaFiles(where:{libraryId:{eq:$id}},page:{limit:200,offset:0}){edges{node{relativePath size}}}}",
                json!({"id": self.library}),
            )
            .await;
        let mut files: Vec<(String, i64)> = data["mediaFiles"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|edge| {
                (
                    edge["node"]["relativePath"]
                        .as_str()
                        .unwrap_or("")
                        .to_string(),
                    edge["node"]["size"].as_i64().unwrap_or(-1),
                )
            })
            .collect();
        files.sort();
        files
    }

    /// Summaries of the library's scan runs, oldest first.
    async fn runs(&mut self) -> Vec<String> {
        let data = self
            .admin
            .query(
                "query($id:String!){libraryScanRuns(where:{libraryId:{eq:$id}},orderBy:[{createdAt:ASC}],page:{limit:200,offset:0}){edges{node{summary finishedAt}}}}",
                json!({"id": self.library}),
            )
            .await;
        data["libraryScanRuns"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|edge| edge["node"]["summary"].as_str().unwrap_or("").to_string())
            .collect()
    }

    async fn idle(&mut self) -> bool {
        let data = self
            .admin
            .query(
                "query($id:String!){libraryScanRuns(where:{libraryId:{eq:$id}},page:{limit:200,offset:0}){edges{node{finishedAt}}} library(id:$id){scanning}}",
                json!({"id": self.library}),
            )
            .await;
        data["library"]["scanning"] == false
            && data["libraryScanRuns"]["edges"]
                .as_array()
                .unwrap()
                .iter()
                .all(|run| run["node"]["finishedAt"].is_string())
    }

    async fn watch_status(&mut self) -> Value {
        let data = self
            .admin
            .query(
                "query($id:String!){library(id:$id){watchStatus watchDetail watchStatusSince}}",
                json!({"id": self.library}),
            )
            .await;
        data["library"].clone()
    }

    async fn wait_for_status(&mut self, wanted: &str, within: Duration) -> Value {
        let started = Instant::now();
        loop {
            let status = self.watch_status().await;
            if status["watchStatus"] == wanted {
                return status;
            }
            assert!(
                started.elapsed() < within,
                "the watch never said '{wanted}': {status}"
            );
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    /// Wait until the records are `wanted` and every scan has finished; return the runs added
    /// since `runs_before`.
    async fn wait_for_files(
        &mut self,
        wanted: &[(&str, i64)],
        runs_before: usize,
        within: Duration,
    ) -> Vec<String> {
        let wanted: Vec<(String, i64)> = wanted
            .iter()
            .map(|(path, size)| (path.to_string(), *size))
            .collect();
        let started = Instant::now();
        loop {
            let files = self.files().await;
            if files == wanted && self.idle().await {
                return self.runs().await.split_off(runs_before);
            }
            assert!(
                started.elapsed() < within,
                "records never became {wanted:?}: {files:?}; runs: {:?}",
                self.runs().await
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    /// Nothing is scanned for `long`.
    async fn no_scan_for(&mut self, long: Duration, why: &str) {
        let before = self.runs().await.len();
        tokio::time::sleep(long).await;
        let runs = self.runs().await;
        assert_eq!(runs.len(), before, "{why}: {:?}", &runs[before..]);
    }
}

/// A summary of a scan of changed paths only (with or without issues: these files are not real
/// video, so their analysis fails).
fn is_change_scan_of(run: &str, places: &str) -> bool {
    run.starts_with(&format!("Changes checked in {places}:"))
        || run.starts_with(&format!("Changes checked in {places} with issues:"))
}

fn only_changes_of(runs: &[String], place: &str) {
    assert_eq!(runs.len(), 1, "one scan of the change, not more: {runs:?}");
    assert!(
        is_change_scan_of(&runs[0], place),
        "a scan of just '{place}', not the whole library: {runs:?}"
    );
}

#[test]
fn a_watched_library_on_a_share() {
    let Some(env) = share_env() else {
        eprintln!(
            "skipped: set LIBRARIAN_SMB_TEST_URL and LIBRARIAN_SMB_TEST_PASSWORD_FILE to run against a share"
        );
        return;
    };
    common::run_app_test(move || async move {
        let app = TestApp::start().await;
        let mut admin = app.admin_client().await;
        library_storage().install_share_provider(Arc::new(NetworkShareProvider::new(
            app.db.clone(),
            app.services.clone(),
        )));

        let base = SmbLocation::parse(&env.url).expect("a valid LIBRARIAN_SMB_TEST_URL");
        let relay = Relay::start(base.address()).await;
        let mut relayed = base.clone();
        relayed.port = Some(relay.port);
        save_share_credentials(
            &app.db,
            &app.services,
            &relayed.credential_ref(),
            &env.user,
            &env.password,
        )
        .await
        .expect("save the share's sign-in");

        let folder = format!("librarian-it-{}", uuid::Uuid::new_v4().simple());
        let direct = SmbStorage::new(
            base.clone(),
            SmbCredentials {
                username: env.user.clone(),
                password: env.password.clone(),
            },
        );
        let other = OtherClient {
            storage: direct.clone(),
            scratch: tempfile::tempdir().unwrap(),
        };
        let work = base.join(&folder).join("Movies").storage_path();
        direct
            .create_dir_all(&work)
            .await
            .expect("make the work folder");

        let outcome = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(exercise(
            &mut admin,
            &relay,
            &other,
            relayed.join(&folder).join("Movies"),
            work.clone(),
        )))
        .await;
        relay.restore();
        remove_tree(&direct, &base.join(&folder).storage_path()).await;
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
        app.shutdown().await;
    });
}

async fn exercise(
    admin: &mut GqlClient,
    relay: &Relay,
    other: &OtherClient,
    library_root: SmbLocation,
    work: StoragePath,
) {
    let owner = admin.user_id.clone().expect("administrator");
    let library = playback::create(
        admin,
        "Library",
        json!({"userId":owner,"name":"Watched films","path":library_root.to_uri(),"libraryType":"movies","autoScan":false,"autoOrganize":false,"namingPattern":"","scanIntervalMinutes":60,"watchForChanges":true,"scanning":false}),
    )
    .await;
    let mut watched = Watched { admin, library };
    let at = |relative: &str| work.join_relative(relative);
    let long = Duration::from_secs(60);

    // Armed, then one catch-up scan: the library had never been scanned.
    let status = watched.wait_for_status("watching", long).await;
    assert!(status["watchStatusSince"].is_string(), "{status}");
    let runs = watched.wait_for_files(&[], 0, long).await;
    assert_eq!(runs.len(), 1, "one catch-up scan on arming: {runs:?}");
    assert!(runs[0].starts_with("Scan completed"), "{runs:?}");
    let mut seen = 1;

    // Created: a scan of the new folder alone.
    let film = vec![7_u8; 64 * 1024];
    other
        .write(&at("Film A (2020)/Film A (2020).mkv"), &film)
        .await;
    let runs = watched
        .wait_for_files(&[("Film A (2020)/Film A (2020).mkv", 65_536)], seen, long)
        .await;
    only_changes_of(&runs, "Film A (2020)");
    seen += runs.len();

    // Renamed: the old name's record goes, the new one's comes, in one scan of the file's names.
    other
        .storage
        .rename(
            &at("Film A (2020)/Film A (2020).mkv"),
            &at("Film A (2020)/Film A (2020) - Director's Cut.mkv"),
            PlaceMode::NoClobber,
        )
        .await
        .unwrap();
    let runs = watched
        .wait_for_files(
            &[("Film A (2020)/Film A (2020) - Director's Cut.mkv", 65_536)],
            seen,
            long,
        )
        .await;
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert!(
        is_change_scan_of(
            &runs[0],
            "Film A (2020)/Film A (2020) - Director's Cut.mkv, Film A (2020)/Film A (2020).mkv"
        ),
        "a scan of the two names only: {runs:?}"
    );
    seen += runs.len();

    // Deleted: its record goes.
    other
        .storage
        .remove_file(&at("Film A (2020)/Film A (2020) - Director's Cut.mkv"))
        .await
        .unwrap();
    let runs = watched.wait_for_files(&[], seen, long).await;
    only_changes_of(&runs, "Film A (2020)/Film A (2020) - Director's Cut.mkv");
    seen += runs.len();

    // Not scanned: the library's own trash, other hidden names, partial downloads, a sidecar no
    // record needs, and Librarian's own writes.
    other
        .write(&at(".librarian-trash/Old (1999).mkv"), &film)
        .await;
    other
        .write(&at("Film B (2021)/.Film B (2021).mkv.x1y2z3"), &film)
        .await;
    other.write(&at("Film C (2022).mkv.part"), &film).await;
    other.write(&at("Film A (2020)/notes.nfo"), b"notes").await;
    let own = other.scratch.path().join("own.mkv");
    std::fs::write(&own, &film).unwrap();
    library_storage()
        .create_dir_all(at("Own (2023)"))
        .await
        .unwrap();
    library_storage()
        .put_from_local(&own, at("Own (2023)/Own (2023).mkv"), PlaceMode::NoClobber)
        .await
        .unwrap();
    watched
        .no_scan_for(
            Duration::from_secs(15),
            "hidden, partial and own changes are not scanned",
        )
        .await;
    assert!(watched.files().await.is_empty());

    // A copy still being written: never catalogued half written.
    let (handle, _) = other
        .storage
        .open_file(&at("Big (2024).mkv"), true)
        .await
        .unwrap();
    let chunk = vec![1_u8; 256 * 1024];
    for index in 0..20_u64 {
        handle
            .write_at(index * chunk.len() as u64, &chunk)
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(
            watched
                .files()
                .await
                .iter()
                .all(|(path, _)| path != "Big (2024).mkv"),
            "the file was scanned while it was still being written"
        );
    }
    handle.close().await;
    let full = 20 * chunk.len() as i64;
    let runs = watched
        .wait_for_files(&[("Big (2024).mkv", full)], seen, long)
        .await;
    only_changes_of(&runs, "Big (2024).mkv");
    seen += runs.len();

    // The share drops: the watch says so, nothing is removed, and a file added meanwhile is found
    // by one catch-up scan once it is back.
    relay.cut().await;
    let status = watched.wait_for_status("unavailable", long).await;
    assert!(
        status["watchDetail"]
            .as_str()
            .is_some_and(|detail| detail.starts_with("Not reachable, retrying")),
        "{status}"
    );
    other
        .write(&at("Film D (2025)/Film D (2025).mkv"), &film)
        .await;
    tokio::time::sleep(Duration::from_secs(8)).await;
    assert_eq!(
        watched.files().await,
        vec![("Big (2024).mkv".to_string(), full)],
        "no record is touched while the share is down"
    );
    relay.restore();
    watched
        .wait_for_status("watching", Duration::from_secs(120))
        .await;
    let runs = watched
        .wait_for_files(
            &[
                ("Big (2024).mkv", full),
                ("Film D (2025)/Film D (2025).mkv", 65_536),
            ],
            seen,
            long,
        )
        .await;
    assert!(
        runs.iter().any(|run| run.starts_with("Scan completed")),
        "a whole-library catch-up scan found it: {runs:?}"
    );
    assert!(
        runs.iter().all(|run| !run.starts_with("Changes checked")),
        "nothing else scanned: {runs:?}"
    );

    // Watching turned off: the state goes with it.
    let data = watched
        .admin
        .query(
            "mutation($id:String!){updateLibrary(id:$id,input:{watchForChanges:false}){success error}}",
            json!({"id": watched.library}),
        )
        .await;
    assert_eq!(data["updateLibrary"]["success"], true, "{data}");
    let started = Instant::now();
    while !watched.watch_status().await["watchStatus"].is_null() {
        assert!(
            started.elapsed() < long,
            "the watch state stayed after watching was turned off"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
