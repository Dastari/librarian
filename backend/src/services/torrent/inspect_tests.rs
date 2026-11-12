//! Reading a torrent's files without downloading it: real `.torrent` fixtures, what each file
//! means, the cache, and a magnet resolved from a librqbit seeder on loopback.

use super::*;
use graphql_orm::DbPool;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/torrents");

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{FIXTURES}/{name}")).expect("fixture exists")
}

async fn database() -> Database {
    let mut db = Database::new(DbPool::connect("sqlite::memory:").await.unwrap());
    crate::services::database::bootstrap_schema(&db)
        .await
        .unwrap();
    db.set_entity_policy(crate::services::graphql::policy::AppEntityPolicy);
    db.set_field_policy(crate::services::graphql::policy::AppFieldPolicy);
    db.set_row_policy(crate::services::graphql::row_policy::OwnershipRowPolicy);
    db
}

#[test]
fn a_private_season_pack_is_read_locally_with_every_file_explained() {
    let inspected = parse_torrent_file(&fixture("babylon5_s01_private.torrent")).unwrap();
    assert_eq!(
        inspected.info_hash,
        "0a01de5a391d589709f498849ccd673c0a64650d"
    );
    assert_eq!(inspected.name, "Babylon.5.S01.1080p.BluRay.x264-GRP");
    assert!(inspected.is_private);
    assert_eq!(inspected.files.len(), 6);
    assert_eq!(inspected.piece_length, 16384);
    assert_eq!(
        inspected.total_size,
        inspected.files.iter().map(|file| file.size).sum::<u64>()
    );
    let episodes: Vec<(Option<i32>, Vec<i32>)> = inspected
        .files
        .iter()
        .filter(|file| file.kind == FileKind::Video)
        .map(|file| (file.season, file.episodes.clone()))
        .collect();
    assert_eq!(
        episodes,
        vec![(Some(1), vec![1]), (Some(1), vec![2]), (Some(1), vec![3])]
    );
    let kinds: Vec<FileKind> = inspected.files.iter().map(|file| file.kind).collect();
    assert!(kinds.contains(&FileKind::Sample));
    assert!(kinds.contains(&FileKind::Subtitle));
    assert!(kinds.contains(&FileKind::Info));
    // The announce address carries the passkey; nothing read from the file keeps it.
    assert!(!format!("{inspected:?}").contains("PASSKEY"));
    assert!(
        !String::from_utf8_lossy(&inspected.info_bytes).contains("PASSKEY"),
        "the info dictionary never holds the announce address"
    );
}

#[test]
fn a_public_album_is_read_with_track_and_disc_numbers() {
    let inspected = parse_torrent_file(&fixture("queen_opera_public.torrent")).unwrap();
    assert_eq!(
        inspected.info_hash,
        "963a21807274bb8202c9546a88322f032b84520e"
    );
    assert!(!inspected.is_private);
    let tracks: Vec<(Option<i32>, Option<i32>)> = inspected
        .files
        .iter()
        .filter(|file| file.kind == FileKind::Audio)
        .map(|file| (file.disc, file.track))
        .collect();
    assert_eq!(
        tracks,
        vec![(Some(1), Some(1)), (Some(1), Some(2)), (Some(1), Some(11))]
    );
    assert!(
        inspected
            .files
            .iter()
            .any(|file| file.kind == FileKind::Image)
    );
}

#[test]
fn garbage_is_not_a_torrent() {
    assert!(parse_torrent_file(b"<!DOCTYPE html><title>Login</title>").is_err());
    assert!(parse_torrent_file(b"").is_err());
}

#[test]
fn file_meanings_from_paths() {
    let folder = describe_file(0, "Show/Season 2/05 - The Title.mkv", 900_000_000);
    assert_eq!(
        (folder.kind, folder.season, folder.episodes),
        (FileKind::Video, Some(2), vec![5])
    );
    let double = describe_file(1, "Show.S01E01E02.1080p.mkv", 2_000_000_000);
    assert_eq!(double.episodes, vec![1, 2]);
    assert_eq!(
        describe_file(2, "Show.S01E01-sample.mkv", 40_000_000).kind,
        FileKind::Sample
    );
    assert_eq!(
        describe_file(3, "Show S01/Featurettes/Making the Show.mkv", 300_000_000).kind,
        FileKind::Extra
    );
    assert_eq!(
        describe_file(4, "Show.S01E05.Behind.the.Scenes.mkv", 300_000_000).kind,
        FileKind::Extra
    );
    let track = describe_file(5, "Album/Disc 2/1-07 Song.flac", 30_000_000);
    assert_eq!((track.disc, track.track), (Some(2), Some(7)));
    assert_eq!(
        describe_file(6, "Book/Book.epub", 1_000).kind,
        FileKind::Ebook
    );
    assert_eq!(
        describe_file(7, "Pack/pack.r01", 50_000_000).kind,
        FileKind::Archive
    );
    // A file named for a sample of something larger is still the episode.
    assert_eq!(
        describe_file(8, "Show.S02E03.Free.Sample.Day.mkv", 1_500_000_000).kind,
        FileKind::Video
    );
}

#[tokio::test]
async fn a_reading_is_cached_by_info_hash_and_link_and_private_metadata_is_not_kept() {
    let db = database().await;
    let private = parse_torrent_file(&fixture("babylon5_s01_private.torrent")).unwrap();
    let mut release = SourceRelease::new(
        "Babylon.5.S01.1080p.BluRay.x264-GRP".into(),
        "https://tracker.example/details.php?id=1".into(),
        Utc::now(),
    );
    release.source_id = Some("source-1".into());
    let stored = store(&db, &private, "torrent_file", Some(link_key(&release)))
        .await
        .unwrap();
    assert_eq!(stored.files.len(), 6);
    // Twice is still one row.
    store(&db, &private, "torrent_file", None).await.unwrap();
    let rows = TorrentContents::query(db.pool()).fetch_all().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].info_bytes.is_none(), "private metadata is not kept");
    assert!(!rows[0].files.contains("PASSKEY"));
    // Found by link before the info hash is known, and by info hash.
    assert_eq!(
        cached_for(&db, &release).await.unwrap().info_hash,
        private.info_hash
    );
    assert!(
        cached(&db, &private.info_hash.to_ascii_uppercase())
            .await
            .is_some()
    );

    let public = parse_torrent_file(&fixture("queen_opera_public.torrent")).unwrap();
    store(&db, &public, "magnet", None).await.unwrap();
    let row = TorrentContents::query(db.pool())
        .filter(TorrentContentsWhereInput {
            info_hash: Some(string_eq(&public.info_hash)),
            ..Default::default()
        })
        .fetch_all()
        .await
        .unwrap()
        .remove(0);
    let kept = base64::engine::general_purpose::STANDARD
        .decode(row.info_bytes.unwrap())
        .unwrap();
    assert_eq!(kept, public.info_bytes);
}

#[test]
fn a_known_info_hash_comes_from_the_listing_or_the_magnet() {
    let mut release = SourceRelease::new("X".into(), "guid".into(), Utc::now());
    assert_eq!(known_info_hash(&release), None);
    release.magnet_uri =
        Some("magnet:?xt=urn:btih:0A01DE5A391D589709F498849CCD673C0A64650D&dn=x".to_string());
    assert_eq!(
        known_info_hash(&release).as_deref(),
        Some("0a01de5a391d589709f498849ccd673c0a64650d")
    );
    // The link key never contains the link.
    release.guid = "https://feed.example/dl/1?torrent_pass=SECRET".into();
    assert!(!link_key(&release).contains("SECRET"));
}

#[test]
fn a_kept_info_dictionary_rebuilds_the_same_torrent_with_its_trackers() {
    let public = parse_torrent_file(&fixture("queen_opera_public.torrent")).unwrap();
    let trackers = vec![
        "udp://tracker.example:1337/announce".to_string(),
        "https://tracker.example/announce".to_string(),
    ];
    let rebuilt = torrent_file_from_info(&public.info_bytes, &trackers);
    let parsed = parse_torrent_file(&rebuilt).unwrap();
    assert_eq!(parsed.info_hash, public.info_hash);
    assert_eq!(parsed.files, public.files);
    assert_eq!(parsed.trackers, trackers);
    // No trackers is still a valid torrent (DHT finds its peers).
    let bare = parse_torrent_file(&torrent_file_from_info(&public.info_bytes, &[])).unwrap();
    assert_eq!(bare.info_hash, public.info_hash);
    assert!(bare.trackers.is_empty());
}

#[test]
fn trackers_with_a_passkey_are_never_kept() {
    let kept = public_trackers([
        "udp://open.tracker.example:6969/announce".to_string(),
        "https://tracker.example/announce.php?passkey=0123456789abcdef0123456789abcdef".to_string(),
        "https://tracker.example/a/0123456789abcdef0123456789abcdef/announce".to_string(),
        "http://tracker.example/announce?torrent_pass=SECRET".to_string(),
        "udp://open.tracker.example:6969/announce".to_string(),
        "javascript:alert(1)".to_string(),
    ]);
    assert_eq!(
        kept,
        vec!["udp://open.tracker.example:6969/announce".to_string()]
    );
    // A private torrent's announce (with its passkey) is not read at all.
    let private = parse_torrent_file(&fixture("babylon5_s01_private.torrent")).unwrap();
    assert!(private.trackers.is_empty());
}

#[test]
fn a_magnet_is_built_from_the_hash_name_and_trackers() {
    let magnet = magnet_for(
        "963A21807274BB8202C9546A88322F032B84520E",
        "Queen - A Night at the Opera",
        &["udp://tracker.example:1337/announce".to_string()],
    );
    assert!(
        magnet.starts_with("magnet:?xt=urn:btih:963a21807274bb8202c9546a88322f032b84520e&dn=Queen")
    );
    let parsed = librqbit::Magnet::parse(&magnet).unwrap();
    assert_eq!(
        parsed.trackers,
        vec!["udp://tracker.example:1337/announce".to_string()]
    );
    assert_eq!(
        parsed.as_id20().unwrap().as_string(),
        "963a21807274bb8202c9546a88322f032b84520e"
    );
}

#[tokio::test]
async fn a_grab_finds_the_kept_torrent_and_keeps_a_magnet_for_restoring() {
    let db = database().await;
    let mut public = parse_torrent_file(&fixture("queen_opera_public.torrent")).unwrap();
    public.trackers = vec!["udp://tracker.example:1337/announce".to_string()];
    let mut release = SourceRelease::new(
        "Queen - A Night at the Opera (1975) [FLAC]".into(),
        "guid-queen".into(),
        Utc::now(),
    );
    release.source_id = Some("source-1".into());
    store(&db, &public, "torrent_file", Some(link_key(&release)))
        .await
        .unwrap();

    // A `.torrent` link with no info hash in its listing: found by its link, magnet built.
    let stored = stored_torrent(&db, &release).await.expect("kept");
    assert_eq!(stored.info_hash, public.info_hash);
    assert_eq!(
        parse_torrent_file(&stored.torrent_file).unwrap().info_hash,
        public.info_hash
    );
    assert!(stored.magnet.contains(&public.info_hash));
    assert!(stored.magnet.contains("tracker.example"));

    // The rebuilt file is enough for the client: no peer exists here, and it answers at once.
    let root = tempfile::tempdir().unwrap();
    let session = Session::new_with_opts(
        root.path().to_owned(),
        librqbit::SessionOptions {
            dht: None,
            disable_trackers: true,
            disable_local_service_discovery: true,
            listen: None,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let started = std::time::Instant::now();
    let listed = list_only(
        &session,
        AddTorrent::from_bytes(stored.torrent_file.clone()),
        None,
        Duration::from_secs(5),
    )
    .await
    .expect("a kept torrent needs no peer");
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(
        parse_torrent_file(&listed).unwrap().info_hash,
        public.info_hash
    );
    session.stop().await;

    // A magnet release keeps its own magnet on the row, and its trackers go into the file.
    release.magnet_uri = Some(format!(
        "magnet:?xt=urn:btih:{}&tr=udp%3A%2F%2Fother.example%3A80%2Fannounce",
        public.info_hash
    ));
    let stored = stored_torrent(&db, &release).await.expect("kept");
    assert_eq!(stored.magnet, release.magnet_uri.clone().unwrap());
    let trackers = parse_torrent_file(&stored.torrent_file).unwrap().trackers;
    assert!(trackers.contains(&"udp://other.example:80/announce".to_string()));

    // A private torrent keeps no info bytes, so it is fetched from its tracker as before.
    let private = parse_torrent_file(&fixture("babylon5_s01_private.torrent")).unwrap();
    let mut private_release = SourceRelease::new("Babylon 5".into(), "guid-b5".into(), Utc::now());
    private_release.info_hash = Some(private.info_hash.clone());
    store(&db, &private, "torrent_file", None).await.unwrap();
    assert!(stored_torrent(&db, &private_release).await.is_none());
}

/// A magnet resolved from a real librqbit seeder on loopback: the files come back, the reading
/// session keeps no torrent, and nothing is written to its download folder.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_magnet_is_listed_from_a_loopback_seeder_without_downloading() {
    use librqbit::{CreateTorrentOptions, ListenerMode, ListenerOptions, SessionOptions};

    let seed_root = tempfile::tempdir().unwrap();
    let content = seed_root.path().join("Babylon.5.S01");
    std::fs::create_dir_all(&content).unwrap();
    for (name, size) in [
        ("Babylon.5.S01E01.1080p.mkv", 70_000usize),
        ("Babylon.5.S01E02.1080p.mkv", 50_000),
    ] {
        let bytes: Vec<u8> = (0..size).map(|index| (index * 31 % 251) as u8).collect();
        std::fs::write(content.join(name), bytes).unwrap();
    }
    let created = librqbit::create_torrent(
        &content,
        CreateTorrentOptions {
            piece_length: Some(16384),
            ..Default::default()
        },
        &librqbit::spawn_utils::BlockingSpawner::new(1),
    )
    .await
    .unwrap();
    let info_hash = created.info_hash().as_string();
    let torrent_bytes = created.as_bytes().unwrap();

    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let seeder = Session::new_with_opts(
        seed_root.path().to_owned(),
        SessionOptions {
            dht: None,
            disable_trackers: true,
            disable_local_service_discovery: true,
            listen: Some(ListenerOptions {
                mode: ListenerMode::TcpOnly,
                listen_addr: (std::net::Ipv4Addr::LOCALHOST, port).into(),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let seeding = seeder
        .add_torrent(
            AddTorrent::from_bytes(torrent_bytes),
            Some(AddTorrentOptions {
                overwrite: true,
                output_folder: Some(content.to_string_lossy().to_string()),
                ..Default::default()
            }),
        )
        .await
        .unwrap();
    let AddTorrentResponse::Added(_, handle) = seeding else {
        panic!("the seeder should add the torrent");
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while !matches!(handle.stats().state, librqbit::TorrentStatsState::Live) {
        assert!(
            std::time::Instant::now() < deadline,
            "the seeder never went live"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let reader_root = tempfile::tempdir().unwrap();
    let reader = Session::new_with_opts(
        reader_root.path().to_owned(),
        SessionOptions {
            dht: None,
            disable_trackers: true,
            disable_local_service_discovery: true,
            listen: None,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let magnet = format!("magnet:?xt=urn:btih:{info_hash}&dn=Babylon.5.S01");
    let bytes = list_only(
        &reader,
        AddTorrent::from_url(&magnet),
        Some(vec![(std::net::Ipv4Addr::LOCALHOST, port).into()]),
        Duration::from_secs(30),
    )
    .await
    .expect("the seeder answers with the metadata");
    let inspected = parse_torrent_file(&bytes).unwrap();
    assert_eq!(inspected.info_hash, info_hash);
    let mut names: Vec<String> = inspected
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec!["Babylon.5.S01E01.1080p.mkv", "Babylon.5.S01E02.1080p.mkv"]
    );
    assert!(
        inspected
            .files
            .iter()
            .all(|file| file.kind == FileKind::Video && file.season == Some(1))
    );

    // Nothing was added to the reading session and nothing was written.
    assert_eq!(reader.with_torrents(|torrents| torrents.count()), 0);
    let written: Vec<_> = walk(reader_root.path());
    assert!(written.is_empty(), "the reader wrote {written:?}");

    // A magnet nobody seeds gives up within its timeout instead of hanging.
    let started = std::time::Instant::now();
    let dead = list_only(
        &reader,
        AddTorrent::from_url("magnet:?xt=urn:btih:1111111111111111111111111111111111111111"),
        Some(vec![(std::net::Ipv4Addr::LOCALHOST, 9).into()]),
        Duration::from_secs(2),
    )
    .await;
    assert!(dead.is_err());
    assert!(started.elapsed() < Duration::from_secs(10));
    assert_eq!(reader.with_torrents(|torrents| torrents.count()), 0);

    seeder.stop().await;
    reader.stop().await;
}

fn walk(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}
