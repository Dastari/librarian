//! A real torrent downloaded onto an SMB share through Librarian's torrent storage (Q100): a
//! seeding session on loopback serves a two-file torrent from a local folder, and a second
//! session downloads it into a folder on the share, pauses, resumes, and deletes it.
//!
//! Skipped unless a share is given (a throwaway local Samba server is enough):
//!
//! ```text
//! LIBRARIAN_SMB_TEST_URL=smb://127.0.0.1:4455/dl
//! LIBRARIAN_SMB_TEST_USER=smbtest
//! LIBRARIAN_SMB_TEST_PASSWORD_FILE=/path/to/password
//! cargo test --test smb_torrent
//! ```
//!
//! Everything is made in a `librarian-tor-<id>` folder under that URL and removed at the end.
//!
//! With `LIBRARIAN_SMB_TEST_STOP` and `LIBRARIAN_SMB_TEST_START` (shell commands that stop and
//! start the test server), a second test stops the share half way through a download and starts
//! it again, and checks the download finishes from where it was without anything deleted.

use std::sync::Arc;
use std::time::{Duration, Instant};

use librarian::services::library_storage::{
    SmbCredentials, SmbLocation, SmbStorage, StoragePath, library_storage,
};
use librarian::services::torrent::share_storage::{LibrarianStorageFactory, set_default_folder};
use librqbit::storage::StorageFactoryExt;
use librqbit::{
    AddTorrent, AddTorrentOptions, CreateTorrentOptions, ListenerOptions, Session, SessionOptions,
};

fn env() -> Option<(String, SmbCredentials)> {
    let url = std::env::var("LIBRARIAN_SMB_TEST_URL").ok()?;
    let file = std::env::var("LIBRARIAN_SMB_TEST_PASSWORD_FILE").ok()?;
    let password = std::fs::read_to_string(file)
        .expect("password file")
        .trim_end_matches(['\n', '\r'])
        .to_string();
    let username = std::env::var("LIBRARIAN_SMB_TEST_USER").unwrap_or_else(|_| "toby".into());
    Some((url, SmbCredentials { username, password }))
}

fn options(factory: bool) -> SessionOptions {
    limited(factory, None)
}

/// Session options, with an upload limit for a seeder that should take a while.
fn limited(factory: bool, upload_bps: Option<u32>) -> SessionOptions {
    SessionOptions {
        ratelimits: librqbit::limits::LimitsConfig {
            upload_bps: upload_bps.and_then(std::num::NonZeroU32::new),
            download_bps: None,
        },
        dht: None,
        persistence: None,
        disable_local_service_discovery: true,
        listen: Some(ListenerOptions {
            listen_addr: ([127, 0, 0, 1], 0).into(),
            ipv4_only: true,
            ..Default::default()
        }),
        default_storage_factory: factory.then(|| LibrarianStorageFactory.boxed()),
        ..Default::default()
    }
}

/// Deterministic bytes that are not all one value, so a misplaced block is noticed.
fn payload(len: usize, seed: u64) -> Vec<u8> {
    let mut state = seed | 1;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

async fn wait_finished(handle: &librqbit::ManagedTorrent, within: Duration) {
    let started = Instant::now();
    loop {
        let stats = handle.stats();
        if stats.finished {
            return;
        }
        assert!(
            started.elapsed() < within,
            "not finished after {within:?}: {stats}"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn remove_tree(root: &StoragePath) {
    let storage = library_storage();
    let mut dirs = vec![root.clone()];
    let mut order = Vec::new();
    while let Some(dir) = dirs.pop() {
        if let Ok(entries) = storage.list_dir(&dir).await {
            for entry in entries {
                let path = dir.join(&entry.name);
                if entry.kind == librarian::services::library_storage::EntryKind::Directory {
                    dirs.push(path);
                } else {
                    let _ = storage.remove_file(&path).await;
                }
            }
        }
        order.push(dir);
    }
    for dir in order.into_iter().rev() {
        let _ = storage.remove_dir(&dir).await;
    }
}

#[test]
fn a_torrent_downloads_onto_a_share_and_resumes() {
    let Some((url, credentials)) = env() else {
        eprintln!("LIBRARIAN_SMB_TEST_URL not set; skipping");
        return;
    };
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            let share = SmbLocation::parse(&url).unwrap();
            let storage = SmbStorage::new(share.clone(), credentials);
            storage.check().await.expect("the test share answers");
            library_storage()
                .register_remote(&share.share_root().storage_path(), Arc::new(storage));

            let work = share.join(&format!("librarian-tor-{}", uuid::Uuid::new_v4().simple()));
            let work_path = work.storage_path();
            library_storage().create_dir_all(&work_path).await.unwrap();

            let outcome =
                tokio::time::timeout(Duration::from_secs(300), exercise(&work_path)).await;
            remove_tree(&work_path).await;
            outcome.expect("the test finished in time");
        });
}

async fn exercise(work: &StoragePath) {
    // A two-file torrent, 9 MiB and 300 KiB, in 256 KiB pieces: blocks are gathered into runs,
    // pieces cross the file boundary, and the last piece is short.
    let source = tempfile::tempdir().unwrap();
    let folder = source.path().join("Some.Release");
    std::fs::create_dir_all(folder.join("Extras")).unwrap();
    let big = payload(9 * 1024 * 1024 + 123, 7);
    let small = payload(300 * 1024 + 5, 11);
    std::fs::write(folder.join("Some.Release.mkv"), &big).unwrap();
    std::fs::write(folder.join("Extras").join("notes.nfo"), &small).unwrap();

    let seeder = Session::new_with_opts(source.path().to_path_buf(), options(false))
        .await
        .unwrap();
    let (created, _seeding) = seeder
        .create_and_serve_torrent(
            &folder,
            CreateTorrentOptions {
                name: Some("Some.Release"),
                trackers: vec![],
                piece_length: Some(256 * 1024),
            },
        )
        .await
        .unwrap();
    let bytes = created.as_bytes().unwrap();
    let seeder_addr = seeder.listen_addr().expect("the seeder listens");

    // As the torrent service does: the session's download folder is the share folder.
    let download_dir = std::path::PathBuf::from(work.as_str());
    set_default_folder(&download_dir);
    let leecher = Session::new_with_opts(download_dir, options(true))
        .await
        .unwrap();
    let started = Instant::now();
    let handle = leecher
        .add_torrent(
            AddTorrent::TorrentFileBytes(bytes.clone()),
            Some(AddTorrentOptions {
                overwrite: true,
                initial_peers: Some(vec![seeder_addr]),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_handle()
        .unwrap();
    assert_eq!(
        handle.output_folder().to_string_lossy(),
        format!("{}/Some.Release", work.as_str()),
        "a multi-file torrent goes in a folder of its name under the download folder"
    );
    wait_finished(&handle, Duration::from_secs(120)).await;
    eprintln!(
        "downloaded {} bytes onto the share in {:?}",
        big.len() + small.len(),
        started.elapsed()
    );

    let on_share = work.join("Some.Release");
    let got = library_storage()
        .read_all(on_share.join("Some.Release.mkv"))
        .await
        .unwrap();
    assert!(
        got == big,
        "the big file on the share is byte for byte the source"
    );
    let got = library_storage()
        .read_all(on_share.join("Extras").join("notes.nfo"))
        .await
        .unwrap();
    assert!(
        got == small,
        "the small file on the share is byte for byte the source"
    );

    // Pausing hands the files to a fresh storage; resuming checks them and stays complete.
    leecher.pause(&handle).await.unwrap();
    leecher.unpause(&handle).await.unwrap();
    wait_finished(&handle, Duration::from_secs(60)).await;
    assert_eq!(
        handle.stats().progress_bytes,
        (big.len() + small.len()) as u64
    );

    // A second session over the same files (a restart): it checks what is on the share and
    // downloads nothing.
    let id = handle.id();
    leecher.delete(id.into(), false).await.unwrap();
    let again = leecher
        .add_torrent(
            AddTorrent::TorrentFileBytes(bytes),
            Some(AddTorrentOptions {
                overwrite: true,
                paused: true,
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_handle()
        .unwrap();
    leecher.unpause(&again).await.unwrap();
    wait_finished(&again, Duration::from_secs(60)).await;
    let live = again
        .stats()
        .live
        .map(|live| live.snapshot.downloaded_and_checked_bytes);
    assert_eq!(
        live.unwrap_or(0),
        0,
        "the files already on the share were checked, not downloaded again"
    );

    // Deleting with files removes them from the share.
    leecher.delete(again.id().into(), true).await.unwrap();
    let left = library_storage()
        .exists(on_share.join("Some.Release.mkv"))
        .await
        .unwrap();
    assert!(
        !left,
        "deleting the torrent with its files removed them from the share"
    );
    seeder.stop().await;
    leecher.stop().await;
}

fn shell(command: &str) {
    let status = std::process::Command::new("sh")
        .args(["-c", command])
        .status()
        .expect("run the test server command");
    assert!(status.success(), "`{command}` failed");
}

#[test]
fn a_download_waits_for_a_share_that_went_away_and_finishes_without_starting_over() {
    let (Some((url, credentials)), Ok(stop), Ok(start)) = (
        env(),
        std::env::var("LIBRARIAN_SMB_TEST_STOP"),
        std::env::var("LIBRARIAN_SMB_TEST_START"),
    ) else {
        eprintln!("LIBRARIAN_SMB_TEST_URL, _STOP or _START not set; skipping");
        return;
    };
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            let share = SmbLocation::parse(&url).unwrap();
            let storage = SmbStorage::new(share.clone(), credentials);
            storage.check().await.expect("the test share answers");
            library_storage()
                .register_remote(&share.share_root().storage_path(), Arc::new(storage));
            let work = share.join(&format!("librarian-tor-{}", uuid::Uuid::new_v4().simple()));
            let work_path = work.storage_path();
            library_storage().create_dir_all(&work_path).await.unwrap();
            let outcome =
                tokio::time::timeout(Duration::from_secs(300), outage(&work_path, &stop, &start))
                    .await;
            // Whatever happened, the server is running again for the clean-up.
            shell(&start);
            tokio::time::sleep(Duration::from_secs(1)).await;
            remove_tree(&work_path).await;
            outcome.expect("the test finished in time");
        });
}

async fn outage(work: &StoragePath, stop: &str, start: &str) {
    let source = tempfile::tempdir().unwrap();
    let folder = source.path().join("Slow.Release");
    std::fs::create_dir_all(&folder).unwrap();
    let big = payload(12 * 1024 * 1024, 23);
    std::fs::write(folder.join("Slow.Release.mkv"), &big).unwrap();
    std::fs::write(folder.join("Slow.Release.nfo"), b"notes").unwrap();
    // 2 MB/s: about six seconds, so the share can go away in the middle.
    let seeder =
        Session::new_with_opts(source.path().to_path_buf(), limited(false, Some(2_000_000)))
            .await
            .unwrap();
    let (created, _seeding) = seeder
        .create_and_serve_torrent(
            &folder,
            CreateTorrentOptions {
                name: Some("Slow.Release"),
                trackers: vec![],
                piece_length: Some(256 * 1024),
            },
        )
        .await
        .unwrap();
    let bytes = created.as_bytes().unwrap();
    let seeder_addr = seeder.listen_addr().unwrap();
    let download_dir = std::path::PathBuf::from(work.as_str());
    set_default_folder(&download_dir);
    let leecher = Session::new_with_opts(download_dir, options(true))
        .await
        .unwrap();
    let handle = leecher
        .add_torrent(
            AddTorrent::TorrentFileBytes(bytes),
            Some(AddTorrentOptions {
                overwrite: true,
                initial_peers: Some(vec![seeder_addr]),
                ..Default::default()
            }),
        )
        .await
        .unwrap()
        .into_handle()
        .unwrap();
    // Wait for a third of it, then take the share away.
    let started = Instant::now();
    while handle.stats().progress_bytes < big.len() as u64 / 3 {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "no progress: {}",
            handle.stats()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    shell(stop);
    eprintln!("share stopped at {} bytes", handle.stats().progress_bytes);
    tokio::time::sleep(Duration::from_secs(4)).await;
    let during = handle.stats();
    eprintln!("while the share was down: {during}");
    assert!(!during.finished, "it cannot finish while the share is down");
    // What the share watcher does: pause while it is down, resume once it answers.
    let _ = leecher.pause(&handle).await;
    shell(start);
    // The registered share replaces its dead connections itself on the next call.
    let back = Instant::now();
    loop {
        if library_storage().stat(work).await.is_ok() {
            break;
        }
        assert!(
            back.elapsed() < Duration::from_secs(30),
            "the share did not come back"
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let before = handle.stats().progress_bytes;
    // An unfinished torrent is checked again from the share, as the watcher does (with the
    // seeder given back to it, as trackers or DHT would).
    let folder = handle.output_folder().to_path_buf();
    let (add, mut options) =
        librarian::services::torrent::share_watch::readd_options(&handle, &folder, false).unwrap();
    options.initial_peers = Some(vec![seeder_addr]);
    leecher.delete(handle.id().into(), false).await.unwrap();
    let again = leecher
        .add_torrent(add, Some(options))
        .await
        .unwrap()
        .into_handle()
        .unwrap();
    leecher.unpause(&again).await.ok();
    wait_finished(&again, Duration::from_secs(120)).await;
    let fetched = again
        .stats()
        .live
        .map(|live| live.snapshot.downloaded_and_checked_bytes)
        .unwrap_or(0);
    eprintln!("after the share came back: {before} bytes kept, {fetched} downloaded");
    assert!(before > 0, "what was checked before the outage was kept");
    assert!(
        fetched < big.len() as u64,
        "it carried on from what was on the share instead of starting over"
    );
    let got = library_storage()
        .read_all(work.join("Slow.Release").join("Slow.Release.mkv"))
        .await
        .unwrap();
    assert!(
        got == big,
        "the file on the share is byte for byte the source"
    );
    seeder.stop().await;
    leecher.stop().await;
}
