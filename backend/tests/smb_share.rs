//! Libraries on a real SMB share, end to end: scan, stream a range, ffprobe through the tool URL,
//! import-copy, organize-rename, move a library onto the share, and a share that goes away.
//!
//! Skipped unless a share is given, so CI never needs one:
//!
//! ```text
//! LIBRARIAN_SMB_TEST_URL=smb://10.0.1.190/media          # share and a folder to work in
//! LIBRARIAN_SMB_TEST_USER=toby                           # default: toby
//! LIBRARIAN_SMB_TEST_PASSWORD_FILE=/root/.config/librarian-spike/truenas-smb-password
//! cargo test --test smb_share
//! ```
//!
//! Everything is made inside a throwaway folder (`librarian-it-<id>`) under that URL, which is
//! removed at the end, pass or fail. The password is read from the file and never printed.
mod common;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use common::{ACCESS_COOKIE, GqlClient, TestApp, playback};
use librarian::services::graphql::filesystem_network::{
    NetworkShareProvider, save_share_credentials,
};
use librarian::services::library_storage::{
    EntryKind, PlaceMode, PlaceOutcome, SmbLocation, Storage, StorageError, StorageErrorKind,
    StoragePath, StorageResult, WalkOptions, library_storage, tool_argument,
};
use librarian::services::tool_url::{GlobalToolUrls, tool_urls};
use reqwest::{Client, StatusCode};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

struct ShareEnv {
    /// `smb://host/share[/folder]`.
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

/// A short test video.
fn make_video(path: &std::path::Path, seconds: u32) {
    let output = std::process::Command::new("ffmpeg")
        .args(["-nostdin", "-v", "error", "-y", "-f", "lavfi", "-i"])
        .arg("testsrc2=size=160x90:rate=10")
        .args(["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000"])
        .args([
            "-t",
            &seconds.to_string(),
            "-pix_fmt",
            "yuv420p",
            "-c:v",
            "libx264",
        ])
        .args(["-c:a", "aac", "-movflags", "+faststart"])
        .arg(path)
        .output()
        .expect("ffmpeg must be installed");
    assert!(
        output.status.success(),
        "ffmpeg: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Remove a folder and everything in it on the share, deepest first.
async fn remove_tree(root: &StoragePath) {
    let storage = library_storage();
    let Ok(mut walk) = storage
        .walk(root, WalkOptions::default(), CancellationToken::new())
        .await
    else {
        return;
    };
    let mut folders = Vec::new();
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

async fn create_library(client: &mut GqlClient, name: &str, path: &str, kind: &str) -> String {
    let owner = client.user_id.clone().expect("administrator");
    playback::create(
        client,
        "Library",
        json!({"userId":owner,"name":name,"path":path,"libraryType":kind,"autoScan":false,"autoOrganize":false,"namingPattern":"","scanIntervalMinutes":60,"watchForChanges":false,"scanning":false}),
    )
    .await
}

async fn scan(client: &mut GqlClient, library_id: &str) {
    let started = client
        .query(
            "mutation($id:String!){scanLibrary(id:$id){success status message}}",
            json!({"id": library_id}),
        )
        .await;
    assert_eq!(started["scanLibrary"]["success"], true, "{started}");
    // Done when a run has finished and the library is not scanning.
    client
        .wait_until(
            Duration::from_secs(90),
            "the scan to finish",
            "query($id:String!){libraryScanRuns(where:{libraryId:{eq:$id}}){edges{node{status finishedAt}}} library(id:$id){scanning}}",
            json!({"id": library_id}),
            |data| {
                data["library"]["scanning"] == false
                    && data["libraryScanRuns"]["edges"]
                        .as_array()
                        .is_some_and(|runs| {
                            !runs.is_empty()
                                && runs.iter().all(|run| run["node"]["finishedAt"].is_string())
                        })
            },
        )
        .await;
}

async fn media_files(client: &mut GqlClient, library_id: &str) -> Vec<Value> {
    let data = client
        .query(
            "query($id:String!){mediaFiles(where:{libraryId:{eq:$id}},page:{limit:100,offset:0}){edges{node{id path relativePath size analyzedAt movieId}}}}",
            json!({"id": library_id}),
        )
        .await;
    data["mediaFiles"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| edge["node"].clone())
        .collect()
}

async fn open_issues(client: &mut GqlClient, library_id: &str) -> Vec<String> {
    let data = client
        .query(
            "query($id:String!){libraryScanIssues(where:{libraryId:{eq:$id}},page:{limit:100,offset:0}){edges{node{issueCode resolvedAt}}}}",
            json!({"id": library_id}),
        )
        .await;
    data["libraryScanIssues"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|edge| edge["node"]["resolvedAt"].is_null())
        .map(|edge| edge["node"]["issueCode"].as_str().unwrap().to_string())
        .collect()
}

/// A provider that behaves like a share that has gone away.
struct DownShare;

#[async_trait]
impl librarian::services::library_storage::ShareProvider for DownShare {
    async fn connect(&self, share: &SmbLocation) -> StorageResult<Arc<dyn Storage>> {
        Err(StorageError::new(
            StorageErrorKind::Unavailable,
            format!("Could not reach {} (simulated outage)", share.host),
        ))
    }
}

async fn update(client: &mut GqlClient, kind: &str, id: &str, input: Value) {
    let field = format!("update{kind}");
    let data = client
        .query(
            &format!("mutation($id:String!,$input:Update{kind}Input!){{{field}(id:$id,input:$input){{success error}}}}"),
            json!({"id": id, "input": input}),
        )
        .await;
    assert_eq!(data[&field]["success"], true, "{data}");
}

/// A movie linked both ways to a media file, so a move has links to keep.
async fn movie_for(client: &mut GqlClient, library_id: &str, file_id: &str, title: &str) -> String {
    let owner = client.user_id.clone().expect("administrator");
    let movie = playback::create(
        client,
        "Movie",
        json!({"userId":owner,"libraryId":library_id,"title":title,"year":2023,"genres":[],"castNames":[],"productionCountries":[],"spokenLanguages":[],"monitored":false,"wanted":true,"hasFile":false,"overview":"A test film."}),
    )
    .await;
    update(client, "MediaFile", file_id, json!({"movieId": movie})).await;
    update(client, "Movie", &movie, json!({"mediaFileId": file_id})).await;
    movie
}

fn file_named<'a>(files: &'a [Value], name: &str) -> &'a Value {
    files
        .iter()
        .find(|file| {
            file["path"]
                .as_str()
                .is_some_and(|path| path.ends_with(name))
        })
        .unwrap_or_else(|| panic!("no media file named {name} in {files:?}"))
}

#[test]
fn libraries_on_a_real_share() {
    let Some(env) = share_env() else {
        eprintln!(
            "skipped: set LIBRARIAN_SMB_TEST_URL and LIBRARIAN_SMB_TEST_PASSWORD_FILE to run against a share"
        );
        return;
    };
    common::run_app_test(move || async move {
        let app = TestApp::start_with(common::TestAppOptions {
            with_transcode: true,
            ..Default::default()
        })
        .await;
        let mut admin = app.admin_client().await;

        // What a running server sets up at start.
        library_storage().install_share_provider(Arc::new(NetworkShareProvider::new(
            app.db.clone(),
            app.services.clone(),
        )));
        tool_urls().configure(std::net::IpAddr::from([127, 0, 0, 1]), app.addr.port());
        library_storage().install_tool_url_issuer(Arc::new(GlobalToolUrls));

        let base = SmbLocation::parse(&env.url).expect("a valid LIBRARIAN_SMB_TEST_URL");
        save_share_credentials(
            &app.db,
            &app.services,
            &base.credential_ref(),
            &env.user,
            &env.password,
        )
        .await
        .expect("save the share's sign-in");
        let work = base.join(&format!("librarian-it-{}", uuid::Uuid::new_v4().simple()));

        let outcome = futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(exercise(
            &app, &mut admin, &work,
        )))
        .await;
        remove_tree(&work.storage_path()).await;
        let left = library_storage().exists(work.storage_path()).await;
        // Nothing of any test run is left in the folder it worked in.
        let strays: Vec<_> = library_storage()
            .list_dir(base.storage_path())
            .await
            .map(|entries| {
                entries
                    .into_iter()
                    .map(|entry| entry.name)
                    .filter(|name| {
                        name.starts_with("librarian-it-") || name.starts_with("librarian-unit-")
                    })
                    .collect()
            })
            .unwrap_or_default();
        app.shutdown().await;
        assert!(strays.is_empty(), "left on the share: {strays:?}");
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
        assert_eq!(
            left.ok(),
            Some(false),
            "the throwaway folder was not removed"
        );
    });
}

async fn exercise(app: &TestApp, admin: &mut GqlClient, work: &SmbLocation) {
    let storage = library_storage();
    let tmp = tempfile::tempdir().unwrap();
    let http = Client::new();
    let cookie = format!("{ACCESS_COOKIE}={}", admin.cookies[ACCESS_COOKIE]);

    // ---- Films on the share: a copy in (no-clobber), then a library over them.
    let one_local = tmp.path().join("one.mp4");
    let two_local = tmp.path().join("two.mp4");
    let three_local = tmp.path().join("three.mp4");
    make_video(&one_local, 3);
    make_video(&two_local, 2);
    make_video(&three_local, 5);
    let movies = work.join("Movies");
    let one = movies
        .join("Film One (2020)")
        .join("Film One (2020).mp4")
        .storage_path();
    let two = movies
        .join("Film Two (2021)")
        .join("Film Two (2021).mp4")
        .storage_path();
    for (local, path) in [(&one_local, &one), (&two_local, &two)] {
        storage
            .create_dir_all(path.parent().unwrap())
            .await
            .unwrap();
        assert_eq!(
            storage
                .put_from_local(local, path, PlaceMode::NoClobber)
                .await
                .unwrap(),
            PlaceOutcome::Placed
        );
    }
    assert_eq!(
        storage
            .put_from_local(&three_local, &one, PlaceMode::NoClobber)
            .await
            .unwrap(),
        PlaceOutcome::TargetExists,
        "an import never replaces a file on the share"
    );
    assert_eq!(
        storage.stat(&one).await.unwrap().size,
        std::fs::metadata(&one_local).unwrap().len(),
        "the existing file was left alone"
    );
    // No half-copied files are left beside the real ones.
    let names: Vec<_> = storage
        .list_dir(one.parent().unwrap())
        .await
        .unwrap()
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    assert_eq!(names, vec!["Film One (2020).mp4".to_string()], "{names:?}");

    // The address is normalised when the library is saved: host and share in lower case.
    let typed = format!(
        "smb://{}{}/{}/{}",
        movies.host.to_uppercase(),
        movies.port.map(|port| format!(":{port}")).unwrap_or_default(),
        movies.share.to_uppercase(),
        movies.path
    );
    let library = create_library(admin, "Share Movies", &typed, "movies").await;
    let saved = admin
        .query(
            "query($id:String!){library(id:$id){path}}",
            json!({"id": library}),
        )
        .await;
    assert_eq!(saved["library"]["path"], movies.to_uri());

    // ---- Scan a library that lives on the share.
    scan(admin, &library).await;
    let files = media_files(admin, &library).await;
    assert_eq!(files.len(), 2, "{files:?}");
    let first = file_named(&files, "Film One (2020).mp4").clone();
    assert_eq!(first["relativePath"], "Film One (2020)/Film One (2020).mp4");
    assert_eq!(
        first["size"].as_i64().unwrap() as u64,
        std::fs::metadata(&one_local).unwrap().len()
    );
    let first_id = first["id"].as_str().unwrap().to_string();

    // ---- Stream ranges through the endpoint, from the share.
    let local = std::fs::read(&one_local).unwrap();
    let stream = |range: Option<&str>| {
        let mut request = http
            .get(format!("{}/api/media/{first_id}/stream", app.base_url))
            .header("Cookie", &cookie);
        if let Some(range) = range {
            request = request.header("Range", range);
        }
        request
    };
    let head = stream(Some("bytes=0-99")).send().await.unwrap();
    assert_eq!(head.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(head.bytes().await.unwrap().as_ref(), &local[..100]);
    let middle = stream(Some("bytes=5000-9999")).send().await.unwrap();
    assert_eq!(middle.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(middle.bytes().await.unwrap().as_ref(), &local[5000..10000]);
    let tail = stream(Some("bytes=-64")).send().await.unwrap();
    assert_eq!(
        tail.bytes().await.unwrap().as_ref(),
        &local[local.len() - 64..]
    );
    let whole = stream(None).send().await.unwrap();
    assert_eq!(whole.status(), StatusCode::OK);
    assert_eq!(whole.bytes().await.unwrap().as_ref(), local.as_slice());

    // ---- ffprobe reads the file through the tool URL.
    let url = tool_argument(&one).await.unwrap();
    assert!(
        url.starts_with(&format!(
            "http://127.0.0.1:{}/internal/tool/",
            app.addr.port()
        )),
        "{url}"
    );
    let probe = tokio::process::Command::new("ffprobe")
        .args(["-v", "error", "-protocol_whitelist", "file,pipe,http,tcp"])
        .args(["-show_format", "-print_format", "json"])
        .arg(&url)
        .output()
        .await
        .unwrap();
    assert!(
        probe.status.success(),
        "{}",
        String::from_utf8_lossy(&probe.stderr)
    );
    let probed: Value = serde_json::from_slice(&probe.stdout).unwrap();
    let duration: f64 = probed["format"]["duration"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((2.5..3.5).contains(&duration), "{probed}");
    // And the scan's own analysis does too.
    let analysed = admin
        .query(
            "mutation($id:String!,$path:String!){analyzeMediaFile(mediaFileId:$id,path:$path){success queued message}}",
            json!({"id": first_id, "path": first["path"]}),
        )
        .await;
    assert_eq!(analysed["analyzeMediaFile"]["success"], true, "{analysed}");
    admin
        .wait_until(
            Duration::from_secs(60),
            "ffprobe analysis of a file on the share",
            "query($id:String!){mediaFile(id:$id){analyzedAt duration}}",
            json!({"id": first_id}),
            |data| data["mediaFile"]["analyzedAt"].is_string(),
        )
        .await;

    // ---- fpcalc (when one is given) reads a song on the share through the tool URL.
    if let Ok(fpcalc) = std::env::var("LIBRARIAN_FPCALC") {
        let song_local = tmp.path().join("song.flac");
        let made = std::process::Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=5",
            ])
            .arg(&song_local)
            .status()
            .unwrap();
        assert!(made.success());
        let song = work
            .join("Music")
            .join("Artist")
            .join("Song.flac")
            .storage_path();
        storage
            .create_dir_all(song.parent().unwrap())
            .await
            .unwrap();
        storage
            .put_from_local(&song_local, &song, PlaceMode::NoClobber)
            .await
            .unwrap();
        // The program itself accepts the URL (no staged copy needed)...
        let url = tool_argument(&song).await.unwrap();
        let direct = tokio::process::Command::new(&fpcalc)
            .args(["-json", "-length", "120"])
            .arg(&url)
            .output()
            .await
            .unwrap();
        assert!(
            direct.status.success(),
            "fpcalc refused the tool URL: {}",
            String::from_utf8_lossy(&direct.stderr)
        );
        let direct: Value = serde_json::from_slice(&direct.stdout).unwrap();
        assert!(
            (4.5..5.5).contains(&direct["duration"].as_f64().unwrap()),
            "{direct}"
        );
        // ...and the helper that stages a copy for a tool that refuses it gives the same answer.
        let scratch = tmp.path().join("scratch");
        std::fs::create_dir_all(&scratch).unwrap();
        let fingerprinted =
            librarian::services::library_storage::run_on_input(&song, &scratch, |input| {
                let fpcalc = fpcalc.clone();
                async move {
                    let output = tokio::process::Command::new(fpcalc)
                        .args(["-json", "-length", "120"])
                        .arg(input)
                        .output()
                        .await?;
                    anyhow::ensure!(output.status.success(), "fpcalc failed");
                    Ok(serde_json::from_slice::<Value>(&output.stdout)?)
                }
            })
            .await
            .unwrap();
        assert_eq!(fingerprinted["fingerprint"], direct["fingerprint"]);
        // And a copy staged from the share is the same file, and removed when dropped.
        let staged = librarian::services::library_storage::StagedCopy::of(&song, &scratch)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(staged.path()).unwrap(),
            std::fs::read(&song_local).unwrap()
        );
        let staged_path = staged.path().to_path_buf();
        drop(staged);
        assert!(!staged_path.exists());
    }

    // ---- Import: a download is copied onto the share and stays where it was.
    let three = movies
        .join("Film Three (2022)")
        .join("Film Three (2022).mp4")
        .storage_path();
    storage
        .create_dir_all(three.parent().unwrap())
        .await
        .unwrap();
    storage
        .put_from_local(&three_local, &three, PlaceMode::NoClobber)
        .await
        .unwrap();
    assert!(three_local.exists(), "the download is not touched");
    scan(admin, &library).await;
    assert_eq!(media_files(admin, &library).await.len(), 3);

    // ---- Organize renames inside the share.
    let two_file = file_named(&media_files(admin, &library).await, "Film Two (2021).mp4").clone();
    let two_id = two_file["id"].as_str().unwrap().to_string();
    movie_for(admin, &library, &two_id, "Organized Film").await;
    let organized = admin
        .query(
            "mutation($input:OrganizeMediaFileInput!){organizeMediaFile(input:$input){success oldPath newPath reason}}",
            json!({"input": {"mediaFileId": two_id}}),
        )
        .await;
    let result = &organized["organizeMediaFile"];
    assert_eq!(result["success"], true, "{organized}");
    let new_path = result["newPath"].as_str().unwrap();
    assert!(
        new_path.starts_with(&movies.to_uri()) && new_path != two.as_str(),
        "{new_path}"
    );
    assert!(
        storage.exists(new_path).await.unwrap(),
        "the file is at its new name"
    );
    assert!(
        !storage.exists(&two).await.unwrap(),
        "and gone from the old one"
    );

    // ---- Move a local library onto the share, keeping every record.
    let local_root = app.path("media/local-movies");
    for (name, source) in [("Alpha (2001)", &one_local), ("Beta (2002)", &two_local)] {
        std::fs::create_dir_all(local_root.join(name)).unwrap();
        std::fs::copy(source, local_root.join(name).join(format!("{name}.mp4"))).unwrap();
    }
    let local_library = create_library(
        admin,
        "Local Movies",
        local_root.to_str().unwrap(),
        "movies",
    )
    .await;
    scan(admin, &local_library).await;
    let before = media_files(admin, &local_library).await;
    assert_eq!(before.len(), 2, "{before:?}");
    let alpha = file_named(&before, "Alpha (2001).mp4").clone();
    let alpha_id = alpha["id"].as_str().unwrap().to_string();
    let alpha_movie = movie_for(admin, &local_library, &alpha_id, "Alpha").await;

    let target = work.join("Moved");
    let target_uri = target.to_uri();
    // Nothing is there yet.
    let plan_query = "query($id:String!,$path:String!){libraryMovePlan(libraryId:$id,newPath:$path){fileCount matched missing missingFiles changed extra extraFiles blocked canApply}}";
    let plan = admin
        .query(plan_query, json!({"id": local_library, "path": target_uri}))
        .await;
    assert!(plan["libraryMovePlan"]["blocked"].is_string(), "{plan}");
    storage
        .create_dir_all(&target.storage_path())
        .await
        .unwrap();
    let plan = admin
        .query(plan_query, json!({"id": local_library, "path": target_uri}))
        .await;
    assert_eq!(plan["libraryMovePlan"]["missing"], 2, "{plan}");
    assert_eq!(plan["libraryMovePlan"]["canApply"], false);
    let refused = admin
        .query(
            "mutation($input:MoveLibraryInput!){moveLibrary(input:$input){success error}}",
            json!({"input": {"libraryId": local_library, "newPath": target_uri}}),
        )
        .await;
    assert_eq!(refused["moveLibrary"]["success"], false, "{refused}");
    // Copy one file: still incomplete, and it names what is left.
    let alpha_there = target
        .join("Alpha (2001)")
        .join("Alpha (2001).mp4")
        .storage_path();
    storage
        .create_dir_all(alpha_there.parent().unwrap())
        .await
        .unwrap();
    storage
        .put_from_local(&one_local, &alpha_there, PlaceMode::NoClobber)
        .await
        .unwrap();
    let plan = admin
        .query(plan_query, json!({"id": local_library, "path": target_uri}))
        .await;
    assert_eq!(plan["libraryMovePlan"]["matched"], 1, "{plan}");
    assert_eq!(
        plan["libraryMovePlan"]["missingFiles"],
        json!(["Beta (2002)/Beta (2002).mp4"])
    );
    // A copy that did not finish is not the same file.
    let beta_there = target
        .join("Beta (2002)")
        .join("Beta (2002).mp4")
        .storage_path();
    storage
        .create_dir_all(beta_there.parent().unwrap())
        .await
        .unwrap();
    storage
        .put_from_local(&three_local, &beta_there, PlaceMode::NoClobber)
        .await
        .unwrap();
    let plan = admin
        .query(plan_query, json!({"id": local_library, "path": target_uri}))
        .await;
    assert_eq!(plan["libraryMovePlan"]["changed"], 1, "{plan}");
    storage.remove_file(&beta_there).await.unwrap();
    storage
        .put_from_local(&two_local, &beta_there, PlaceMode::NoClobber)
        .await
        .unwrap();
    // A media file the library does not hold is reported, not an error.
    let extra = target
        .join("Extra (2003)")
        .join("Extra (2003).mp4")
        .storage_path();
    storage
        .create_dir_all(extra.parent().unwrap())
        .await
        .unwrap();
    storage
        .put_from_local(&three_local, &extra, PlaceMode::NoClobber)
        .await
        .unwrap();
    let plan = admin
        .query(plan_query, json!({"id": local_library, "path": target_uri}))
        .await;
    let found = &plan["libraryMovePlan"];
    assert_eq!(
        (
            &found["fileCount"],
            &found["matched"],
            &found["missing"],
            &found["extra"],
            &found["canApply"]
        ),
        (&json!(2), &json!(2), &json!(0), &json!(1), &json!(true)),
        "{plan}"
    );

    let moved = admin
        .query(
            "mutation($input:MoveLibraryInput!){moveLibrary(input:$input){success error movedFiles}}",
            json!({"input": {"libraryId": local_library, "newPath": target_uri}}),
        )
        .await;
    assert_eq!(moved["moveLibrary"]["success"], true, "{moved}");
    assert_eq!(moved["moveLibrary"]["movedFiles"], 2);
    let after = media_files(admin, &local_library).await;
    assert_eq!(after.len(), 2);
    let alpha_after = file_named(&after, "Alpha (2001).mp4");
    assert_eq!(alpha_after["id"], alpha["id"], "the record keeps its id");
    assert_eq!(alpha_after["path"], alpha_there.as_str());
    assert_eq!(alpha_after["relativePath"], alpha["relativePath"]);
    assert_eq!(
        alpha_after["movieId"],
        json!(alpha_movie),
        "and its link to the movie"
    );
    let library_now = admin
        .query(
            "query($id:String!){library(id:$id){path}}",
            json!({"id": local_library}),
        )
        .await;
    assert_eq!(library_now["library"]["path"], target_uri);
    // The local copy is untouched: Librarian never deletes the originals.
    assert!(local_root.join("Alpha (2001)/Alpha (2001).mp4").exists());
    // A scan finds every file where the move put it, and the extra one.
    scan(admin, &local_library).await;
    let scanned = media_files(admin, &local_library).await;
    assert_eq!(scanned.len(), 3, "{scanned:?}");
    assert_eq!(file_named(&scanned, "Alpha (2001).mp4")["id"], alpha["id"]);
    assert_eq!(
        file_named(&scanned, "Alpha (2001).mp4")["analyzedAt"],
        alpha_after["analyzedAt"],
        "a moved file is not re-analysed"
    );
    // Moving to where it already is is refused.
    let again = admin
        .query(
            "mutation($input:MoveLibraryInput!){moveLibrary(input:$input){success error}}",
            json!({"input": {"libraryId": local_library, "newPath": target_uri}}),
        )
        .await;
    assert_eq!(again["moveLibrary"]["success"], false, "{again}");

    // ---- A share that stops answering: a scan removes nothing and says why.
    let before_outage = media_files(admin, &library).await.len();
    library_storage().install_share_provider(Arc::new(DownShare));
    library_storage().forget_share(work);
    scan(admin, &library).await;
    assert_eq!(
        media_files(admin, &library).await.len(),
        before_outage,
        "no file was removed"
    );
    assert!(
        open_issues(admin, &library)
            .await
            .contains(&"LIBRARY_STORAGE_UNAVAILABLE".to_string()),
        "the library is reported unavailable"
    );
    // Back again: the same library scans normally.
    library_storage().install_share_provider(Arc::new(NetworkShareProvider::new(
        app.db.clone(),
        app.services.clone(),
    )));
    library_storage().forget_share(work);
    scan(admin, &library).await;
    assert_eq!(media_files(admin, &library).await.len(), before_outage);
}
