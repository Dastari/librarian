//! Deterministic end-to-end torrent transfer over loopback.
//!
//! Two in-process librqbit sessions — one seeding a generated payload, one
//! downloading it — with DHT and trackers disabled and the seeder supplied as
//! an explicit initial peer, so the test needs no network and no third party.
//!
//! It covers the two claims the torrent service depends on and that unit tests
//! cannot check:
//!
//! 1. `add -> progress -> completion` really works with the same
//!    `AddTorrentOptions` the service uses (`overwrite: true`), and the file
//!    librqbit reports under `handle.output_folder()` is the file that exists
//!    on disk. `services/torrent/database.rs::build_file_rows` builds the
//!    import paths from exactly that folder.
//! 2. Deleting a torrent's payload (`session.delete(.., delete_files = true)`,
//!    which `torrent.remove_after_import` triggers) leaves a hardlink made
//!    into the library tree completely intact. That is the safety property
//!    behind the seeding cleanup policy.

use std::path::Path;
use std::time::{Duration, Instant};

use librqbit::spawn_utils::BlockingSpawner;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, CreateTorrentOptions, ListenerOptions,
    Session, SessionOptions, create_torrent,
};

/// Small enough to transfer instantly over loopback, large enough to span
/// several pieces so the transfer exercises real piece requests.
const PAYLOAD_BYTES: usize = 2 * 1024 * 1024;
const PIECE_LENGTH: u32 = 128 * 1024;
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(45);

fn deterministic_payload() -> Vec<u8> {
    // Cheap LCG so the bytes are incompressible-ish but reproducible.
    let mut state: u32 = 0x1234_5678;
    (0..PAYLOAD_BYTES)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 24) as u8
        })
        .collect()
}

fn session_opts(listen_port: u16) -> SessionOptions {
    SessionOptions {
        dht: None,
        persistence: None,
        listen: Some(ListenerOptions {
            listen_addr: ([127, 0, 0, 1], listen_port).into(),
            ipv4_only: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Mirrors `services::torrent::client::add_torrent_opts`, plus the loopback-only
/// wiring the test needs.
fn add_opts(paused: bool) -> AddTorrentOptions {
    AddTorrentOptions {
        overwrite: true,
        paused,
        disable_trackers: true,
        ..Default::default()
    }
}

async fn wait_for_completion(handle: &std::sync::Arc<librqbit::ManagedTorrent>) -> (f64, Duration) {
    let started = Instant::now();
    loop {
        let stats = handle.stats();
        let progress = if stats.total_bytes == 0 {
            0.0
        } else {
            stats.progress_bytes as f64 / stats.total_bytes as f64
        };
        if progress >= 1.0 {
            return (progress, started.elapsed());
        }
        assert!(
            started.elapsed() < TRANSFER_TIMEOUT,
            "loopback transfer did not finish within {}s (progress {:.4}, state {:?})",
            TRANSFER_TIMEOUT.as_secs(),
            progress,
            stats.state
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn seeds_and_downloads_over_loopback_then_payload_deletion_spares_the_hardlink() {
    let seed_dir = tempfile::tempdir().expect("seed dir");
    let leech_dir = tempfile::tempdir().expect("leech dir");
    let library_dir = tempfile::tempdir().expect("library dir");

    let payload = deterministic_payload();
    let seed_file = seed_dir.path().join("librarian-test-payload.bin");
    std::fs::write(&seed_file, &payload).expect("write seed payload");

    let torrent = create_torrent(
        &seed_file,
        CreateTorrentOptions {
            name: None,
            trackers: Vec::new(),
            piece_length: Some(PIECE_LENGTH),
        },
        &BlockingSpawner::new(1),
    )
    .await
    .expect("create torrent from payload");
    let torrent_bytes = torrent.as_bytes().expect("serialize torrent");

    // --- Seeder -------------------------------------------------------------
    let seeder = Session::new_with_opts(seed_dir.path().to_path_buf(), session_opts(0))
        .await
        .expect("seeder session");
    let seeder_addr = seeder
        .listen_addr()
        .expect("seeder should have bound a listen address");

    let seed_handle = match seeder
        .add_torrent(
            AddTorrent::from_bytes(torrent_bytes.clone()),
            Some(add_opts(false)),
        )
        .await
        .expect("seeder add")
    {
        AddTorrentResponse::Added(_, handle) => handle,
        other => panic!("seeder expected Added, got {:?}", response_kind(&other)),
    };
    // The payload is already on disk, so this is a hash check, not a download.
    wait_for_completion(&seed_handle).await;

    // --- Leecher ------------------------------------------------------------
    let leecher = Session::new_with_opts(leech_dir.path().to_path_buf(), session_opts(0))
        .await
        .expect("leecher session");

    let leech_handle = match leecher
        .add_torrent(
            AddTorrent::from_bytes(torrent_bytes),
            Some(AddTorrentOptions {
                initial_peers: Some(vec![seeder_addr]),
                ..add_opts(false)
            }),
        )
        .await
        .expect("leecher add")
    {
        AddTorrentResponse::Added(_, handle) => handle,
        other => panic!("leecher expected Added, got {:?}", response_kind(&other)),
    };

    let (progress, elapsed) = wait_for_completion(&leech_handle).await;
    assert!(progress >= 1.0, "leecher progress was {progress}");

    // The service builds every import path from `output_folder()` joined with
    // each file's relative name — assert that path is the real one.
    let metadata = leech_handle
        .metadata
        .load_full()
        .expect("leecher should have torrent metadata after completion");
    assert_eq!(metadata.file_infos.len(), 1, "single-file torrent expected");
    let downloaded = leech_handle.output_folder().join(
        metadata.file_infos[0]
            .relative_filename
            .to_string_lossy()
            .as_ref(),
    );
    assert!(
        downloaded.is_file(),
        "expected downloaded payload at {} after {:?}",
        downloaded.display(),
        elapsed
    );
    assert_eq!(
        std::fs::read(&downloaded).expect("read downloaded payload"),
        payload,
        "downloaded bytes differ from the seeded payload"
    );

    // --- Import: hardlink into the "library", as library_scan does ----------
    let library_file = library_dir.path().join("Imported Release.bin");
    // tempfile puts everything under the same /tmp filesystem, so the hardlink
    // is the same code path a real same-filesystem import takes.
    std::fs::hard_link(&downloaded, &library_file).unwrap_or_else(|e| {
        panic!(
            "hardlink {} -> {} failed: {e}",
            downloaded.display(),
            library_file.display()
        )
    });
    assert_eq!(link_count(&downloaded), 2, "payload should have two links");

    // --- remove_after_import cleanup ---------------------------------------
    let id = leech_handle.id();
    leecher
        .delete(librqbit::api::TorrentIdOrHash::Id(id), true)
        .await
        .expect("delete torrent with files");

    assert!(
        !downloaded.exists(),
        "payload {} should have been deleted",
        downloaded.display()
    );
    assert!(
        library_file.is_file(),
        "library hardlink {} must survive payload deletion",
        library_file.display()
    );
    assert_eq!(
        std::fs::read(&library_file).expect("read library file"),
        payload,
        "library file contents changed after the payload was deleted"
    );
    assert_eq!(
        link_count(&library_file),
        1,
        "library file should be the last remaining link"
    );

    // Seeder still holds its own copy; it was never touched.
    assert!(seed_file.is_file());
}

#[cfg(unix)]
fn link_count(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path)
        .unwrap_or_else(|e| panic!("stat {}: {e}", path.display()))
        .nlink()
}

/// Windows has no cheap hard-link count on `Metadata`; the test only needs the file to exist.
#[cfg(not(unix))]
fn link_count(path: &Path) -> u64 {
    u64::from(path.is_file())
}

fn response_kind(response: &AddTorrentResponse) -> &'static str {
    match response {
        AddTorrentResponse::Added(_, _) => "Added",
        AddTorrentResponse::AlreadyManaged(_, _) => "AlreadyManaged",
        AddTorrentResponse::ListOnly(_) => "ListOnly",
    }
}
