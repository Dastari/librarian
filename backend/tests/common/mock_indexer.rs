//! An in-test Torznab indexer that serves *real* torrents.
//!
//! The acquisition pipeline is only meaningful end to end if the magnet the
//! indexer advertises actually resolves and downloads, so this mock is three
//! things behind one loopback HTTP server:
//!
//! 1. **Torznab API** (`/api`) — `t=caps`, `t=tvsearch`, `t=search`, backed by
//!    a list of releases the test registers up front.
//! 2. **BitTorrent HTTP tracker** (`/announce`) — hands out one peer: the
//!    seeder below. The app's session has DHT disabled and no internet, so the
//!    tracker in the magnet's `tr=` parameter is the *only* way it can find a
//!    peer. Without this, `TorrentService::add_magnet` blocks for its full
//!    120s metadata timeout on every grab.
//! 3. **Seeder** — a second in-process librqbit session holding the payloads,
//!    exactly the pattern `tests/torrent_localhost_transfer.rs` established.
//!
//! Nothing here touches the network.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use librqbit::spawn_utils::BlockingSpawner;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, CreateTorrentOptions, ListenerOptions,
    Session, SessionOptions, create_torrent,
};
use tokio::sync::oneshot;

/// Payload files are tiny: the point is a real transfer, not a slow one.
const PIECE_LENGTH: u32 = 16 * 1024;

/// One release as the indexer advertises it, and as the seeder serves it.
#[derive(Debug, Clone)]
pub struct MockRelease {
    pub title: String,
    pub info_hash: String,
    pub magnet: String,
    /// `.torrent` download URL served by the mock.
    pub torrent_url: String,
    pub size_bytes: u64,
    pub seeders: u32,
    pub leechers: u32,
    /// Torznab category (5000 = TV, 2000 = Movies).
    pub category: u32,
    /// Days before now to report as the publication date.
    pub age_days: i64,
}

/// A file inside a release's payload: a path relative to the torrent root and
/// a size. Contents are deterministic filler.
pub struct PayloadFile {
    pub relative_path: String,
    pub size_bytes: usize,
}

impl PayloadFile {
    pub fn new(relative_path: &str, size_bytes: usize) -> Self {
        Self {
            relative_path: relative_path.to_string(),
            size_bytes,
        }
    }
}

/// Deterministic, mildly incompressible filler so pieces differ.
pub fn filler(seed: u32, len: usize) -> Vec<u8> {
    let mut state = seed | 1;
    (0..len)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 24) as u8
        })
        .collect()
}

struct MockState {
    api_key: String,
    releases: parking_lot::Mutex<Vec<MockRelease>>,
    /// Per-`q` search hit counter, so tests can assert search backoff.
    search_calls: parking_lot::Mutex<HashMap<String, usize>>,
    seeder_addr: SocketAddr,
    /// `.torrent` file bodies, by info hash.
    torrent_files: parking_lot::Mutex<HashMap<String, Vec<u8>>>,
    /// Advertise `magnet:` links instead of `.torrent` URLs.
    ///
    /// `.torrent` is the default because it makes the app's `add_torrent`
    /// deterministic: librqbit reads the metadata straight out of the HTTP
    /// response instead of blocking on a peer handshake, and
    /// `add_magnet_with_metadata` keeps the GraphQL caller waiting for
    /// exactly that step.
    advertise_magnets: parking_lot::Mutex<bool>,
}

/// A running mock indexer. Drop or [`MockIndexer::shutdown`] to stop it.
pub struct MockIndexer {
    pub base_url: String,
    pub api_url: String,
    pub api_key: String,
    seed_root: PathBuf,
    seeder: Arc<Session>,
    state: Arc<MockState>,
    _temp: tempfile::TempDir,
    shutdown: Option<oneshot::Sender<()>>,
}

impl MockIndexer {
    pub async fn start() -> Self {
        let temp = tempfile::tempdir().expect("mock indexer temp dir");
        let seed_root = temp.path().join("seed");
        std::fs::create_dir_all(&seed_root).expect("seed root");

        let seeder = Session::new_with_opts(
            seed_root.clone(),
            SessionOptions {
                dht: None,
                persistence: None,
                listen: Some(ListenerOptions {
                    listen_addr: ([127, 0, 0, 1], 0).into(),
                    ipv4_only: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
        .expect("mock indexer seeder session");
        let seeder_addr = seeder
            .listen_addr()
            .expect("seeder should have bound a listen address");

        let state = Arc::new(MockState {
            api_key: "mock-torznab-key".to_string(),
            releases: parking_lot::Mutex::new(Vec::new()),
            search_calls: parking_lot::Mutex::new(HashMap::new()),
            seeder_addr,
            torrent_files: parking_lot::Mutex::new(HashMap::new()),
            advertise_magnets: parking_lot::Mutex::new(false),
        });

        let router = axum::Router::new()
            .route("/api", get(torznab_api))
            .route("/announce", get(announce))
            .route("/download/{info_hash}", get(download_torrent))
            .with_state(state.clone());

        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind mock indexer");
        let addr = listener.local_addr().expect("mock indexer addr");
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        tokio::spawn(async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await;
        });

        Self {
            base_url: format!("http://127.0.0.1:{}", addr.port()),
            api_url: format!("http://127.0.0.1:{}/api", addr.port()),
            api_key: state.api_key.clone(),
            seed_root,
            seeder,
            state,
            _temp: temp,
            shutdown: Some(shutdown_tx),
        }
    }

    /// Register a release whose payload is a single file.
    pub async fn add_release(
        &self,
        title: &str,
        file_name: &str,
        size_bytes: usize,
    ) -> MockRelease {
        self.add_release_with_files(title, None, &[PayloadFile::new(file_name, size_bytes)])
            .await
    }

    /// Register a release. `folder` is the torrent's root directory name for
    /// multi-file releases (season packs, archives); `None` makes a
    /// single-file torrent.
    pub async fn add_release_with_files(
        &self,
        title: &str,
        folder: Option<&str>,
        files: &[PayloadFile],
    ) -> MockRelease {
        let payload_root = self.seed_root.join(sanitize(title));
        std::fs::create_dir_all(&payload_root).expect("payload root");

        // librqbit uses an explicit `output_folder` verbatim — it does not
        // append the torrent's own top-level directory — so the seeder must be
        // pointed at whatever `relative_filename` is relative to: the parent
        // for a single-file torrent, the folder itself for a multi-file one.
        let (torrent_source, total) = match folder {
            Some(folder) => {
                let dir = payload_root.join(folder);
                std::fs::create_dir_all(&dir).expect("payload folder");
                let mut total = 0usize;
                for (index, file) in files.iter().enumerate() {
                    total += write_payload_file(&dir, file, index as u32);
                }
                (dir, total)
            }
            None => {
                let file = files.first().expect("single-file release needs one file");
                let total = write_payload_file(&payload_root, file, 0);
                (payload_root.join(&file.relative_path), total)
            }
        };

        let seed_output = if folder.is_some() {
            torrent_source.clone()
        } else {
            payload_root.clone()
        };

        self.seed_and_register(title, &torrent_source, &seed_output, total as u64)
            .await
    }

    /// Create the torrent for an already-written payload directory, start
    /// seeding it, and advertise it.
    async fn register(&self, title: &str, dir: &Path) -> MockRelease {
        let total = dir_size(dir);
        self.seed_and_register(title, dir, dir, total).await
    }

    async fn seed_and_register(
        &self,
        title: &str,
        torrent_source: &Path,
        seed_output: &Path,
        total: u64,
    ) -> MockRelease {
        let torrent = create_torrent(
            torrent_source,
            CreateTorrentOptions {
                name: None,
                trackers: vec![format!("{}/announce", self.base_url)],
                piece_length: Some(PIECE_LENGTH),
            },
            &BlockingSpawner::new(1),
        )
        .await
        .expect("create torrent");
        let info_hash = format!("{:?}", torrent.info_hash());
        let bytes = torrent.as_bytes().expect("serialize torrent");
        let bytes_for_http = bytes.to_vec();

        let handle = match self
            .seeder
            .add_torrent(
                AddTorrent::from_bytes(bytes),
                Some(AddTorrentOptions {
                    overwrite: true,
                    // Without this the seeder looks for the payload under the
                    // session root, finds nothing, and tries to download it —
                    // forever, since the only peer it could ask is itself.
                    output_folder: Some(seed_output.to_string_lossy().into_owned()),
                    // The seeder must not announce. The tracker below answers
                    // every announce with the seeder's own address, so a
                    // seeder that announces spends its time connecting to
                    // itself; those self-connections occupy the accept loop
                    // and starve the real incoming connection, which then
                    // fails its metadata handshake on a 10s read timeout.
                    // The seeder's only job is to accept.
                    disable_trackers: true,
                    ..Default::default()
                }),
            )
            .await
            .expect("seeder add")
        {
            AddTorrentResponse::Added(_, handle) => handle,
            AddTorrentResponse::AlreadyManaged(_, handle) => handle,
            AddTorrentResponse::ListOnly(_) => panic!("seeder returned ListOnly"),
        };
        // Only a hash check: the bytes are already on disk. A timeout here
        // means the seeder was pointed at the wrong directory.
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            handle
                .wait_until_completed()
                .await
                .expect("seeder should verify its own payload");
        })
        .await
        .unwrap_or_else(|_| panic!("seeder did not verify '{title}' within 20s"));

        self.state
            .torrent_files
            .lock()
            .insert(info_hash.clone(), bytes_for_http);

        let release = MockRelease {
            title: title.to_string(),
            magnet: format!(
                "magnet:?xt=urn:btih:{info_hash}&dn={}&tr={}/announce",
                urlencoding::encode(title),
                self.base_url
            ),
            torrent_url: format!("{}/download/{info_hash}", self.base_url),
            info_hash,
            size_bytes: total,
            seeders: 50,
            leechers: 3,
            category: 5000,
            age_days: 1,
        };
        self.state.releases.lock().push(release.clone());
        release
    }

    /// Register a release whose payload is a single ZIP archive, the shape
    /// that exercises the extraction + staging path.
    pub async fn add_zip_release(
        &self,
        title: &str,
        folder: &str,
        zip_name: &str,
        entries: &[(&str, usize)],
    ) -> MockRelease {
        let payload_root = self.seed_root.join(sanitize(title));
        let dir = payload_root.join(folder);
        std::fs::create_dir_all(&dir).expect("payload folder");

        let file = std::fs::File::create(dir.join(zip_name)).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        // Stored, not deflated: the filler is random-ish, and the test cares
        // about the entries, not the codec.
        let options: zip::write::FileOptions<'_, ()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (index, (name, size)) in entries.iter().enumerate() {
            use std::io::Write;
            zip.start_file(*name, options).expect("zip entry");
            zip.write_all(&filler(index as u32 + 11, *size))
                .expect("zip entry body");
        }
        zip.finish().expect("finish zip");

        self.register(title, &dir).await
    }

    /// Advertise `magnet:` links rather than `.torrent` URLs, so a test can
    /// exercise the magnet path the app takes for most real indexers.
    pub fn advertise_magnets(&self) {
        *self.state.advertise_magnets.lock() = true;
    }

    /// Adjust the last-registered release (seeders, size, age, category).
    pub fn amend_last<F: FnOnce(&mut MockRelease)>(&self, edit: F) -> MockRelease {
        let mut releases = self.state.releases.lock();
        let release = releases.last_mut().expect("no releases registered");
        edit(release);
        release.clone()
    }

    /// How many searches the indexer has answered whose `q` contained `needle`.
    pub fn searches_matching(&self, needle: &str) -> usize {
        self.state
            .search_calls
            .lock()
            .iter()
            .filter(|(query, _)| query.contains(needle))
            .map(|(_, count)| *count)
            .sum()
    }

    /// Every search query the indexer has answered, for failure messages.
    pub fn search_log(&self) -> Vec<String> {
        let calls = self.state.search_calls.lock();
        let mut log: Vec<String> = calls
            .iter()
            .map(|(query, count)| format!("{query} x{count}"))
            .collect();
        log.sort();
        log
    }

    pub fn total_searches(&self) -> usize {
        self.state.search_calls.lock().values().sum()
    }

    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for MockIndexer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}

fn write_payload_file(root: &Path, file: &PayloadFile, seed: u32) -> usize {
    let path = root.join(&file.relative_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("payload parent dir");
    }
    std::fs::write(&path, filler(seed.wrapping_add(7), file.size_bytes)).expect("write payload");
    file.size_bytes
}

fn dir_size(dir: &Path) -> u64 {
    let mut total = 0;
    for entry in std::fs::read_dir(dir).expect("read payload dir") {
        let entry = entry.expect("payload entry");
        let metadata = entry.metadata().expect("payload metadata");
        total += if metadata.is_dir() {
            dir_size(&entry.path())
        } else {
            metadata.len()
        };
    }
    total
}

fn sanitize(title: &str) -> String {
    title
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

// ---------------------------------------------------------------------------
// .torrent download
// ---------------------------------------------------------------------------

/// Serve the `.torrent` for an info hash. `AddTorrent::from_url` fetches this
/// directly, so the app has full metadata before it ever needs a peer.
async fn download_torrent(
    State(state): State<Arc<MockState>>,
    axum::extract::Path(info_hash): axum::extract::Path<String>,
) -> Response {
    let info_hash = info_hash.trim_end_matches(".torrent").to_ascii_lowercase();
    match state.torrent_files.lock().get(&info_hash) {
        Some(bytes) => (
            [("content-type", "application/x-bittorrent")],
            bytes.clone(),
        )
            .into_response(),
        None => (axum::http::StatusCode::NOT_FOUND, "unknown info hash").into_response(),
    }
}

// ---------------------------------------------------------------------------
// HTTP tracker
// ---------------------------------------------------------------------------

/// Minimal BEP 3 HTTP tracker: always answers with the seeder, in compact
/// form. librqbit only needs `interval` and `peers` to start connecting.
async fn announce(State(state): State<Arc<MockState>>) -> Response {
    let SocketAddr::V4(addr) = state.seeder_addr else {
        panic!("seeder should be bound to an IPv4 loopback address");
    };
    let mut peers = Vec::with_capacity(6);
    peers.extend_from_slice(&addr.ip().octets());
    peers.extend_from_slice(&addr.port().to_be_bytes());

    let mut body = Vec::new();
    body.extend_from_slice(b"d8:completei1e10:incompletei0e8:intervali60e5:peers6:");
    body.extend_from_slice(&peers);
    body.extend_from_slice(b"e");

    ([("content-type", "text/plain")], body).into_response()
}

// ---------------------------------------------------------------------------
// Torznab API
// ---------------------------------------------------------------------------

async fn torznab_api(
    State(state): State<Arc<MockState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if params.get("apikey").map(String::as_str) != Some(state.api_key.as_str()) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            "<error code=\"100\" description=\"Invalid API key\"/>".to_string(),
        )
            .into_response();
    }

    match params.get("t").map(String::as_str) {
        Some("caps") => xml(caps_xml()),
        Some("tvsearch") | Some("search") | Some("movie") | Some("music") | Some("book") => {
            let query = search_key(&params);
            *state.search_calls.lock().entry(query).or_insert(0) += 1;
            let releases = state.releases.lock().clone();
            let magnets = *state.advertise_magnets.lock();
            xml(search_xml(&releases, magnets))
        }
        other => (
            axum::http::StatusCode::BAD_REQUEST,
            format!("unsupported t={}", other.unwrap_or("<missing>")),
        )
            .into_response(),
    }
}

/// The identity of a search, for the call counter: the free-text query plus
/// season/episode if present.
fn search_key(params: &HashMap<String, String>) -> String {
    let mut key = params.get("q").cloned().unwrap_or_default();
    if let Some(season) = params.get("season") {
        key.push_str(&format!(" S{season}"));
    }
    if let Some(episode) = params.get("ep") {
        key.push_str(&format!("E{episode}"));
    }
    key
}

fn xml(body: String) -> Response {
    ([("content-type", "application/xml")], body).into_response()
}

fn caps_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<caps>
  <server title="Mock Torznab"/>
  <limits max="100" default="50"/>
  <searching>
    <search available="yes" supportedParams="q"/>
    <tv-search available="yes" supportedParams="q,season,ep"/>
    <movie-search available="yes" supportedParams="q,imdbid"/>
    <music-search available="yes" supportedParams="q,artist,album"/>
    <book-search available="yes" supportedParams="q,author,title"/>
  </searching>
  <categories>
    <category id="2000" name="Movies"/>
    <category id="3000" name="Audio"/>
    <category id="5000" name="TV"/>
    <category id="7000" name="Books"/>
  </categories>
</caps>
"#
    .to_string()
}

fn search_xml(releases: &[MockRelease], magnets: bool) -> String {
    let items: String = releases
        .iter()
        .map(|release| item_xml(release, magnets))
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:torznab="http://torznab.com/schemas/2015/feed">
  <channel>
    <title>Mock Torznab</title>
{items}  </channel>
</rss>
"#
    )
}

fn item_xml(release: &MockRelease, magnets: bool) -> String {
    let published = (chrono::Utc::now() - chrono::Duration::days(release.age_days))
        .format("%a, %d %b %Y %H:%M:%S +0000");
    let title = escape(&release.title);
    // `<link>` is what the hunt ends up handing to librqbit: it prefers a
    // magnet when one is advertised, and falls back to the link otherwise.
    let link = escape(if magnets {
        &release.magnet
    } else {
        &release.torrent_url
    });
    let magnet_attr = if magnets {
        format!(
            "      <torznab:attr name=\"magneturl\" value=\"{}\"/>\n",
            escape(&release.magnet)
        )
    } else {
        String::new()
    };
    format!(
        r#"    <item>
      <title>{title}</title>
      <guid isPermaLink="false">{}</guid>
      <link>{link}</link>
      <enclosure url="{link}" length="{}" type="application/x-bittorrent"/>
      <pubDate>{published}</pubDate>
      <size>{}</size>
      <category>{}</category>
      <torznab:attr name="category" value="{}"/>
      <torznab:attr name="seeders" value="{}"/>
      <torznab:attr name="peers" value="{}"/>
      <torznab:attr name="leechers" value="{}"/>
      <torznab:attr name="size" value="{}"/>
      <torznab:attr name="infohash" value="{}"/>
{magnet_attr}    </item>
"#,
        release.info_hash,
        release.size_bytes,
        release.size_bytes,
        release.category,
        release.category,
        release.seeders,
        release.seeders + release.leechers,
        release.leechers,
        release.size_bytes,
        release.info_hash,
    )
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
