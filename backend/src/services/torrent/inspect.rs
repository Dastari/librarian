//! Inspecting a torrent before grabbing it: its file list, without downloading anything (Q99).
//!
//! A release's name says what it should hold; its files say what it does. To read them:
//!
//! - **A `.torrent` link** (Torznab, IPTorrents, a feed item): the file is fetched (through its
//!   source's login when the link is on the source's own site, exactly as a grab fetches it) and
//!   parsed locally. No peer or tracker is asked, so a private tracker never sees an announce.
//! - **A magnet** (The Pirate Bay and the like): librqbit's list-only mode asks peers for the info
//!   dictionary and returns it *before* any storage, handle or file exists. It never starts a
//!   download, writes payload or leaves a torrent in the session. A hard timeout bounds it.
//!
//! What was read is kept forever as a [`TorrentContents`] row keyed by info hash (metadata is
//! immutable), with each file's meaning ([`ContentFile`]: episode, track, sample, extra...).
//! Reading is budgeted: a few candidates per target, `.torrent` fetches counted against the hourly
//! search budget, at most two magnets resolved at once, and a failure is not retried for hours.
//! Links carry passkeys, so nothing here logs or stores one (`redact`).

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, LazyLock, Mutex as StdMutex};
use std::time::Duration;

use anyhow::{Context, Result};
use base64::Engine;
use chrono::{DateTime, Utc};
use graphql_orm::graphql::filters::StringFilter;
use librqbit::{AddTorrent, AddTorrentOptions, AddTorrentResponse, Session};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use tracing::{debug, info, warn};

use crate::db::Database;
use crate::graphql::entities::{
    CreateTorrentContentsInput, FEED_ITEM_GUID_PREFIX, RssFeedItem, TorrentContents,
    TorrentContentsWhereInput,
};
use crate::services::ServicesManager;
use crate::services::quality::scoring::parse_release;
use crate::services::redact::redact_text;
use crate::services::release_match;
use crate::services::sources::SourceRelease;

/// Longest a magnet's metadata is waited for. Peers answer in seconds when they exist at all.
pub const MAGNET_TIMEOUT: Duration = Duration::from_secs(40);
/// Longest a `.torrent` download is waited for.
pub const FILE_TIMEOUT: Duration = Duration::from_secs(30);
/// A release that could not be read is not tried again for this long.
const RETRY_AFTER: chrono::Duration = chrono::Duration::hours(6);
/// Magnets resolved at the same time.
const MAGNET_SLOTS: usize = 2;

// ===========================================================================
// What a file is
// ===========================================================================

/// What a file in a torrent is, for deciding what the torrent holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Video,
    Audio,
    Subtitle,
    /// A video sample (`sample/`, `-sample.mkv`): never an episode.
    Sample,
    /// A featurette, behind-the-scenes or bonus video: never an episode.
    Extra,
    /// `.nfo`, `.txt`, `.sfv` and the like.
    Info,
    Image,
    Ebook,
    Archive,
    Other,
}

/// One file of a torrent and what its path says it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentFile {
    /// Index in the torrent's file list (padding files included), which `only_files` uses.
    pub index: usize,
    pub path: String,
    pub size: u64,
    pub kind: FileKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub season: Option<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub episodes: Vec<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disc: Option<i32>,
}

impl ContentFile {
    /// The file's name without folders or extension.
    pub fn stem(&self) -> &str {
        let file = self.path.rsplit(['/', '\\']).next().unwrap_or(&self.path);
        file.rsplit_once('.').map_or(file, |(stem, _)| stem)
    }

    /// An episode or a track, not a sample, extra or sidecar.
    pub fn is_main_media(&self) -> bool {
        matches!(self.kind, FileKind::Video | FileKind::Audio)
    }
}

fn extension(path: &str) -> String {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file.rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default()
}

const SUBTITLE_EXTENSIONS: &[&str] = &["srt", "ass", "ssa", "sub", "idx", "vtt", "sup"];
const INFO_EXTENSIONS: &[&str] = &[
    "nfo", "txt", "sfv", "md5", "sha1", "cue", "log", "m3u", "m3u8", "url", "html", "exe",
];
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "bmp", "webp", "tbn"];

static SAMPLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(^|[/\\._ \-\[(])sample([/\\._ \-\])]|$)").expect("sample pattern")
});
static EXTRA_FOLDER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(^|[/\\])(extras?|featurettes?|bonus|behind[ ._-]the[ ._-]scenes|deleted[ ._-]scenes|interviews?|trailers?|making[ ._-]of)[/\\]",
    )
    .expect("extra folder pattern")
});
static EXTRA_NAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[._ \-](featurette|trailer|behind[._ ]the[._ ]scenes|deleted[._ ]scene|interview|bloopers?|gag[._ ]reel|making[._ ]of)([._ \-]|$)")
        .expect("extra name pattern")
});
static SEASON_FOLDER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[/\\ ._-])(?:season|series|s)[ ._-]?(\d{1,2})(?:[/\\ ._-]|$)")
        .expect("season folder pattern")
});
static DISC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[/\\ ._(\[-])(?:cd|disc|disk)[ ._-]?(\d{1,2})(?:[/\\ ._)\]-]|$)")
        .expect("disc pattern")
});

/// The track number a file name leads with: `01 - Title.flac`, `1-02 Title`, `03. Title`.
fn leading_track_number(stem: &str) -> Option<i32> {
    let trimmed = stem.trim_start();
    let position = trimmed.find(|c: char| c.is_ascii_digit())?;
    if position > 3 {
        return None;
    }
    let rest = &trimmed[position..];
    let first: String = rest.chars().take_while(char::is_ascii_digit).collect();
    // `1-02` is disc 1, track 2.
    let after = &rest[first.len()..];
    let second: String = after
        .strip_prefix('-')
        .map(|tail| tail.chars().take_while(char::is_ascii_digit).collect())
        .unwrap_or_default();
    let number = if first.len() == 1 && second.len() == 2 {
        &second
    } else {
        &first
    };
    (!number.is_empty() && number.len() <= 3)
        .then(|| number.parse().ok())
        .flatten()
}

/// What one file of a torrent is, from its path and size.
pub fn describe_file(index: usize, path: &str, size: u64) -> ContentFile {
    let ext = extension(path);
    let stem = {
        let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
        file.rsplit_once('.')
            .map_or(file, |(stem, _)| stem)
            .to_string()
    };
    let folder = path
        .rsplit_once(['/', '\\'])
        .map(|(folder, _)| folder)
        .unwrap_or("");
    let is_video = release_match::VIDEO_EXTENSIONS.contains(&ext.as_str());
    let is_audio = release_match::AUDIO_EXTENSIONS.contains(&ext.as_str());
    let kind = if is_video {
        if SAMPLE_RE.is_match(path) && size < 1_000_000_000 {
            FileKind::Sample
        } else if EXTRA_FOLDER_RE.is_match(path) || EXTRA_NAME_RE.is_match(&stem) {
            FileKind::Extra
        } else {
            FileKind::Video
        }
    } else if is_audio {
        FileKind::Audio
    } else if SUBTITLE_EXTENSIONS.contains(&ext.as_str()) {
        FileKind::Subtitle
    } else if INFO_EXTENSIONS.contains(&ext.as_str()) {
        FileKind::Info
    } else if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        FileKind::Image
    } else if release_match::EBOOK_EXTENSIONS.contains(&ext.as_str()) {
        FileKind::Ebook
    } else if release_match::ARCHIVE_EXTENSIONS.contains(&ext.as_str())
        || (ext.len() == 3 && ext.starts_with('r') && ext[1..].chars().all(|c| c.is_ascii_digit()))
    {
        FileKind::Archive
    } else {
        FileKind::Other
    };

    let mut file = ContentFile {
        index,
        path: path.to_string(),
        size,
        kind,
        season: None,
        episodes: Vec::new(),
        track: None,
        disc: None,
    };
    match kind {
        FileKind::Video | FileKind::Subtitle | FileKind::Sample | FileKind::Extra => {
            let parsed = parse_release(&stem);
            let folder_season = SEASON_FOLDER_RE
                .captures(folder)
                .and_then(|found| found.get(1)?.as_str().parse::<i32>().ok());
            if !parsed.episodes.is_empty() {
                file.season = parsed.season.or(folder_season);
                file.episodes = parsed.episodes;
            } else if let Some(season) = folder_season
                && let Some(number) = leading_track_number(&stem)
            {
                // `Season 2/05 - Title.mkv`.
                file.season = Some(season);
                file.episodes = vec![number];
            } else {
                file.season = parsed.season.or(folder_season);
            }
        }
        FileKind::Audio => {
            file.track = leading_track_number(&stem);
            file.disc = DISC_RE
                .captures(folder)
                .and_then(|found| found.get(1)?.as_str().parse().ok());
        }
        _ => {}
    }
    file
}

// ===========================================================================
// Reading metadata
// ===========================================================================

/// A torrent's metadata as read, before it is stored.
#[derive(Debug, Clone)]
pub struct Inspected {
    pub info_hash: String,
    pub name: String,
    pub total_size: u64,
    pub piece_length: u32,
    pub piece_count: u32,
    pub is_private: bool,
    pub files: Vec<ContentFile>,
    /// The bencoded info dictionary.
    pub info_bytes: Vec<u8>,
}

/// Parse a `.torrent` file locally. Nothing is fetched or announced.
pub fn parse_torrent_file(bytes: &[u8]) -> Result<Inspected> {
    let meta = librqbit::torrent_from_bytes(bytes)
        .map_err(|error| anyhow::anyhow!("not a valid torrent file: {error}"))?;
    let info_hash = meta.info_hash.as_string();
    let info_bytes = meta.info.raw_bytes.as_ref().to_vec();
    let info = meta
        .info
        .data
        .validate()
        .map_err(|error| anyhow::anyhow!("the torrent's file list is not valid: {error}"))?;
    let is_private = info.info().private;
    let piece_length = info.info().piece_length;
    let lengths = info.lengths();
    let total_size = lengths.total_length();
    let piece_count = total_size.div_ceil(u64::from(piece_length.max(1))) as u32;
    let mut files = Vec::new();
    for (index, details) in info.iter_file_details().enumerate() {
        if details.attrs().padding {
            continue;
        }
        let path = details.filename.to_vec().join("/");
        files.push(describe_file(index, &path, details.len));
    }
    let name = info
        .name()
        .map(|name| name.into_owned())
        .or_else(|| files.first().map(|file| file.path.clone()))
        .unwrap_or_else(|| info_hash.clone());
    Ok(Inspected {
        info_hash,
        name,
        total_size,
        piece_length,
        piece_count,
        is_private,
        files,
        info_bytes,
    })
}

/// Ask librqbit for a torrent's metadata in list-only mode, within `timeout`: a magnet's info
/// dictionary from peers, an `http(s)` `.torrent` from its address, or a file's bytes as they are.
/// librqbit returns a list-only answer before creating storage, a handle or a session entry, and
/// does not announce for a torrent it was given the metadata of. Dropping the future on timeout
/// stops the peer connections. Returns the `.torrent` bytes.
pub async fn list_only(
    session: &Arc<Session>,
    add: AddTorrent<'_>,
    initial_peers: Option<Vec<std::net::SocketAddr>>,
    timeout: Duration,
) -> Result<bytes::Bytes> {
    let options = AddTorrentOptions {
        list_only: true,
        paused: true,
        initial_peers,
        ..Default::default()
    };
    let answer = tokio::time::timeout(timeout, session.add_torrent(add, Some(options)))
        .await
        .with_context(|| {
            format!(
                "no metadata within {} seconds (no peer answered)",
                timeout.as_secs()
            )
        })?
        .map_err(|error| anyhow::anyhow!(redact_text(&format!("{error:#}"))))?;
    match answer {
        AddTorrentResponse::ListOnly(listed) => Ok(listed.torrent_bytes),
        AddTorrentResponse::AlreadyManaged(_, handle) => handle
            .metadata
            .load_full()
            .map(|metadata| metadata.torrent_bytes.clone())
            .context("the torrent is in the session but has no metadata yet"),
        AddTorrentResponse::Added(..) => {
            // Never expected: list-only mode returns before a handle exists.
            anyhow::bail!("the client added the torrent instead of only listing it")
        }
    }
}

// ===========================================================================
// The cache
// ===========================================================================

fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

/// The key a release's link is cached under: a hash, because the link can hold a passkey.
pub fn link_key(release: &SourceRelease) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(release.source_id.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"\n");
    hasher.update(release.guid.as_bytes());
    hex::encode(hasher.finalize())
}

/// The info hash a release names, from its listing or its magnet, as lowercase hex.
pub fn known_info_hash(release: &SourceRelease) -> Option<String> {
    let is_hex = |hash: &str| hash.len() == 40 && hash.chars().all(|c| c.is_ascii_hexdigit());
    release
        .info_hash
        .as_deref()
        .map(str::to_ascii_lowercase)
        .filter(|hash| is_hex(hash))
        .or_else(|| {
            release
                .magnet_uri
                .as_deref()
                .and_then(crate::jobs::rss_poll::magnet_info_hash)
                .filter(|hash| is_hex(hash))
        })
}

/// What is known of a torrent's files.
#[derive(Debug, Clone, PartialEq)]
pub struct Contents {
    pub info_hash: String,
    pub name: String,
    pub total_size: i64,
    pub is_private: bool,
    pub files: Vec<ContentFile>,
}

impl Contents {
    pub fn from_row(row: &TorrentContents) -> Self {
        Self {
            info_hash: row.info_hash.clone(),
            name: row.name.clone(),
            total_size: row.total_size,
            is_private: row.is_private,
            files: serde_json::from_str(&row.files).unwrap_or_default(),
        }
    }

    fn from_inspected(inspected: &Inspected) -> Self {
        Self {
            info_hash: inspected.info_hash.clone(),
            name: inspected.name.clone(),
            total_size: inspected.total_size as i64,
            is_private: inspected.is_private,
            files: inspected.files.clone(),
        }
    }
}

/// A cached reading of a torrent, by info hash.
pub async fn cached(db: &Database, info_hash: &str) -> Option<Contents> {
    TorrentContents::query(db.pool())
        .filter(TorrentContentsWhereInput {
            info_hash: Some(string_eq(&info_hash.to_ascii_lowercase())),
            ..Default::default()
        })
        .fetch_all()
        .await
        .ok()?
        .first()
        .map(Contents::from_row)
}

async fn cached_by_link(db: &Database, key: &str) -> Option<Contents> {
    TorrentContents::query(db.pool())
        .filter(TorrentContentsWhereInput {
            link_key: Some(string_eq(key)),
            ..Default::default()
        })
        .fetch_all()
        .await
        .ok()?
        .first()
        .map(Contents::from_row)
}

/// The cached reading of a release, from its info hash or its link, without reading anything.
pub async fn cached_for(db: &Database, release: &SourceRelease) -> Option<Contents> {
    if let Some(hash) = known_info_hash(release)
        && let Some(found) = cached(db, &hash).await
    {
        return Some(found);
    }
    cached_by_link(db, &link_key(release)).await
}

/// Keep a reading. A row for the same info hash already there is kept as it is (metadata does not
/// change), so two readers racing each other store one row.
pub async fn store(
    db: &Database,
    inspected: &Inspected,
    via: &str,
    link_key: Option<String>,
) -> Result<Contents> {
    if let Some(found) = cached(db, &inspected.info_hash).await {
        return Ok(found);
    }
    let now = Utc::now().to_rfc3339();
    let input = CreateTorrentContentsInput {
        info_hash: inspected.info_hash.clone(),
        link_key,
        name: inspected.name.clone(),
        total_size: inspected.total_size as i64,
        file_count: inspected.files.len() as i32,
        piece_length: inspected.piece_length as i32,
        piece_count: inspected.piece_count as i32,
        is_private: inspected.is_private,
        via: via.to_string(),
        files: serde_json::to_string(&inspected.files).context("could not write the file list")?,
        // A private torrent's metadata is fetched from its tracker again when it is grabbed.
        info_bytes: (!inspected.is_private)
            .then(|| base64::engine::general_purpose::STANDARD.encode(&inspected.info_bytes)),
        fetched_at: now,
    };
    if let Err(error) = TorrentContents::insert(db, input).await
        && cached(db, &inspected.info_hash).await.is_none()
    {
        return Err(error).context("could not keep the torrent's file list");
    }
    Ok(Contents::from_inspected(inspected))
}

// ===========================================================================
// Budgets
// ===========================================================================

/// `.torrent` downloads in the last hour, shared by every inspection.
static FETCHES: LazyLock<StdMutex<VecDeque<DateTime<Utc>>>> =
    LazyLock::new(|| StdMutex::new(VecDeque::new()));
/// When a release could not be read, and why.
type Failure = (DateTime<Utc>, String);
/// Releases that could not be read, by cache key.
static FAILURES: LazyLock<StdMutex<HashMap<String, Failure>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));
static MAGNETS: LazyLock<tokio::sync::Semaphore> =
    LazyLock::new(|| tokio::sync::Semaphore::new(MAGNET_SLOTS));

fn take_fetch(per_hour: u32) -> bool {
    let Ok(mut spent) = FETCHES.lock() else {
        return false;
    };
    let now = Utc::now();
    while spent
        .front()
        .is_some_and(|at| now - *at >= chrono::Duration::hours(1))
    {
        spent.pop_front();
    }
    if spent.len() as u32 >= per_hour {
        return false;
    }
    spent.push_back(now);
    true
}

fn recent_failure(key: &str) -> Option<String> {
    let failures = FAILURES.lock().ok()?;
    let (at, why) = failures.get(key)?;
    (Utc::now() - *at < RETRY_AFTER).then(|| why.clone())
}

fn remember_failure(key: &str, why: &str) {
    if let Ok(mut failures) = FAILURES.lock() {
        let now = Utc::now();
        failures.retain(|_, (at, _)| now - *at < RETRY_AFTER);
        failures.insert(key.to_string(), (now, why.to_string()));
    }
}

// ===========================================================================
// Inspecting a release
// ===========================================================================

/// What inspecting a release gave.
#[derive(Debug, Clone, PartialEq)]
pub enum Inspection {
    /// The files are known (from the cache or just read).
    Known(Contents),
    /// Not read, and why: no link, out of budget, failed recently, failed now.
    Unavailable(String),
}

impl Inspection {
    pub fn contents(&self) -> Option<&Contents> {
        match self {
            Self::Known(contents) => Some(contents),
            Self::Unavailable(_) => None,
        }
    }
}

/// The link a grab would use for a release: its magnet, its download link, or (a feed release)
/// its catalogue item's link revealed.
async fn release_link(
    manager: &Arc<ServicesManager>,
    db: &Database,
    release: &SourceRelease,
) -> Result<String> {
    if let Some(magnet) = &release.magnet_uri {
        return Ok(magnet.clone());
    }
    if let Some(link) = &release.link {
        return Ok(link.clone());
    }
    let item_id = release
        .guid
        .strip_prefix(FEED_ITEM_GUID_PREFIX)
        .context("the release has neither a magnet nor a download link")?;
    let item = RssFeedItem::get(db.pool(), &item_id.to_string())
        .await?
        .context("the release is no longer in its feed")?;
    crate::jobs::rss_poll::reveal_item_link(manager, &item.link).await
}

/// Read a release's file list: from the cache if it has been read before, else from its
/// `.torrent` or its magnet, within the budgets. Never downloads payload. `fetches_per_hour` is
/// the hourly budget for `.torrent` downloads (`auto_download.hourly_search_budget`).
pub async fn inspect_release(
    manager: &Arc<ServicesManager>,
    db: &Database,
    release: &SourceRelease,
    fetches_per_hour: u32,
) -> Inspection {
    if let Some(found) = cached_for(db, release).await {
        return Inspection::Known(found);
    }
    let key = known_info_hash(release).unwrap_or_else(|| link_key(release));
    if let Some(why) = recent_failure(&key) {
        return Inspection::Unavailable(format!("could not be read earlier: {why}"));
    }
    let started = std::time::Instant::now();
    let outcome = read_release(manager, db, release, fetches_per_hour).await;
    match outcome {
        Ok(Some((inspected, via))) => {
            let link = Some(link_key(release));
            match store(db, &inspected, via, link).await {
                Ok(contents) => {
                    info!(
                        release = %release.title,
                        info_hash = %contents.info_hash,
                        via,
                        files = contents.files.len(),
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "Inspected '{}' without downloading it: {} files, {} bytes, via {via}",
                        release.title,
                        contents.files.len(),
                        contents.total_size
                    );
                    Inspection::Known(contents)
                }
                Err(error) => {
                    warn!(release = %release.title, error = %error, "Read the files of '{}' but could not keep them", release.title);
                    Inspection::Known(Contents::from_inspected(&inspected))
                }
            }
        }
        Ok(None) => {
            Inspection::Unavailable("the hourly budget for reading torrents is used up".to_string())
        }
        Err(error) => {
            let why = redact_text(&format!("{error:#}"));
            debug!(release = %release.title, error = %why, "Could not inspect '{}': {why}", release.title);
            remember_failure(&key, &why);
            Inspection::Unavailable(why)
        }
    }
}

/// `Ok(None)` when the budget is spent.
async fn read_release(
    manager: &Arc<ServicesManager>,
    db: &Database,
    release: &SourceRelease,
    fetches_per_hour: u32,
) -> Result<Option<(Inspected, &'static str)>> {
    let link = release_link(manager, db, release).await?;
    if link.to_ascii_lowercase().starts_with("magnet:") {
        let torrent = manager
            .get_torrent()
            .await
            .context("the torrent client is not running")?;
        let _slot = MAGNETS
            .acquire()
            .await
            .context("magnet inspection is shutting down")?;
        let bytes = torrent.read_metadata_only(&link, MAGNET_TIMEOUT).await?;
        return Ok(Some((parse_torrent_file(&bytes)?, "magnet")));
    }
    if !take_fetch(fetches_per_hour) {
        return Ok(None);
    }
    // Through the source's login when the link is on its own site, as a grab fetches it.
    if let Some(source_id) = release.source_id.as_deref()
        && let Some(bytes) = tokio::time::timeout(
            FILE_TIMEOUT,
            crate::services::graphql::entities::fetch_torrent_from_source(
                manager, source_id, &link,
            ),
        )
        .await
        .context("the source did not send the torrent in time")??
    {
        return Ok(Some((parse_torrent_file(&bytes)?, "torrent_file")));
    }
    // A plain link (an indexer proxy, a feed's own link): fetched by the client in list-only mode,
    // which reads the file and asks no peer.
    let torrent = manager
        .get_torrent()
        .await
        .context("the torrent client is not running")?;
    let bytes = torrent.read_metadata_only(&link, FILE_TIMEOUT).await?;
    Ok(Some((parse_torrent_file(&bytes)?, "torrent_file")))
}

#[cfg(test)]
#[path = "inspect_tests.rs"]
mod tests;
