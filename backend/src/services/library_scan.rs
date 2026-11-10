use std::cell::Cell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context as AnyhowContext, Result};
use async_graphql::{Request, Variables};
use async_trait::async_trait;
use chrono::{DateTime, Datelike, SecondsFormat, Utc};
use futures::{
    FutureExt,
    stream::{self, StreamExt},
};
use graphql_orm::graphql::auth::AuthSubject;
use graphql_orm::graphql::errors::{OrmErrorCode, OrmPublicError};
use graphql_orm::graphql::filters::StringFilter;
use graphql_orm::graphql::orm::{DbAuthContext, TransactionMode};
use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::services::extract;
use crate::services::graphql::entities::common::AutoDownloadMode;
use crate::services::graphql::entities::{
    Chapter, ChapterWhereInput, CreateLibraryScanIssueInput, CreateLibraryScanRunInput, Episode,
    EpisodeWhereInput, LibraryScanIssue, LibraryScanIssueWhereInput, LibraryScanRun, MediaFile,
    MediaFileWhereInput, Subtitle, SubtitleWhereInput, Track, TrackWhereInput,
    UpdateLibraryScanIssueInput, UpdateLibraryScanRunInput, User,
};
use crate::services::graphql::{AuthUser, LibrarianSchema};
use crate::services::library_storage::own_writes::own_write;
use crate::services::library_storage::{
    EntryKind, IntoStoragePath, PlaceMode, StorageErrorKind, StoragePath, TOOL_PROTOCOL_WHITELIST,
    WalkFilter, WalkOptions, is_smb_uri, is_tool_url, library_storage, place_no_clobber_blocking,
    tool_argument,
};
use crate::services::manager::{Service, ServiceHealth, ServicesManager};
use crate::services::metadata::providers::{
    AddAlbumOptions, AddAudiobookOptions, AddMovieOptions, AddTvShowOptions, MetadataProvider,
    MetadataService,
};
use crate::services::music_rules;
use crate::services::ollama::{OllamaClient, OllamaParsedHint, OllamaParserSettings};
use crate::services::quality::{profile, scoring};
use crate::services::release_match;

mod identify;
mod listen;
mod move_library;
mod music;
#[cfg(test)]
mod organize_tests;
mod watch;

pub use identify::{IdentifiedChoice, IdentifyCandidate, IdentifyTrigger};
pub use move_library::{LibraryMoveOutcome, LibraryMovePlan};
use watch::describe_change_targets;
pub use watch::nudge_library_watches;

/// Transport success is not mutation success. These internal documents select a
/// success/error payload for every mutation root, including compatibility aliases.
fn check_mutation_payloads(data: &serde_json::Value) -> anyhow::Result<()> {
    let roots = data
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Mutation response is not an object"))?;
    if roots.is_empty() {
        anyhow::bail!("Mutation response has no result payload");
    }
    let mut checked = false;
    for (name, payload) in roots {
        if name == "__typename" {
            continue;
        }
        checked = true;
        let success = payload
            .get("success")
            .or_else(|| payload.get("Success"))
            .and_then(serde_json::Value::as_bool);
        if success != Some(true) {
            let error = payload
                .get("error")
                .or_else(|| payload.get("Error"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("missing or unsuccessful mutation payload");
            anyhow::bail!("{name}: {error}");
        }
    }
    if !checked {
        anyhow::bail!("Mutation response has no result payload");
    }
    Ok(())
}

tokio::task_local! {
    static SCAN_GRAPHQL_ENTITY_OPERATIONS: Cell<u64>;
}

#[derive(Debug, Clone)]
pub struct LibraryScanServiceConfig {
    pub autoscan_poll_interval: Duration,
    pub analyze_workers: usize,
}

impl Default for LibraryScanServiceConfig {
    fn default() -> Self {
        Self {
            autoscan_poll_interval: Duration::from_secs(60),
            analyze_workers: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatchMethod {
    Filename,
    Metadata,
    Ollama,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MatchWantedPolicy {
    #[default]
    PreferWanted,
    WantedOnly,
    All,
}

#[derive(Debug, Clone, Default)]
pub struct MatchRequest {
    pub media_file_id: String,
    pub library_id: Option<String>,
    pub episode_id: Option<String>,
    pub movie_id: Option<String>,
    pub track_id: Option<String>,
    pub chapter_id: Option<String>,
    pub methods: Vec<MatchMethod>,
    pub force: bool,
    pub auto_match: bool,
    pub candidate_limit: usize,
    pub allow_provider_fallback: bool,
    pub wanted_policy: MatchWantedPolicy,
    /// The GraphQL caller's user id, when known. Recorded as `matchedByUserId` when this
    /// request results in a manual match (explicit episodeId/movieId/trackId/chapterId).
    /// Internal callers (scan worker, torrent import) leave this `None`, which is correct
    /// since those paths never set explicit target IDs and therefore never produce a
    /// manual match.
    pub requested_by_user_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MatchCandidate {
    pub target_type: String,
    pub target_id: String,
    pub target_name: Option<String>,
    pub score: f64,
    pub reason: Option<String>,
    pub wanted: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct MatchResult {
    pub success: bool,
    pub auto_matched: bool,
    pub already_matched: bool,
    pub matched_type: Option<String>,
    pub matched_id: Option<String>,
    pub confidence: f64,
    pub reason: Option<String>,
    pub candidates: Vec<MatchCandidate>,
}

#[derive(Debug, Clone, Default)]
pub struct OrganizeResult {
    pub success: bool,
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SourceFileImport {
    pub source_path: String,
    pub relative_path: String,
    pub file_size: i64,
    pub downloaded_bytes: i64,
    pub file_index: Option<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct SourceProcessSummary {
    pub success: bool,
    pub files_processed: i32,
    pub files_failed: i32,
    pub messages: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
struct SelectedImportMatch {
    library: LibraryRow,
    candidate: MatchCandidate,
}

/// A person's choice of what one downloaded file is. Imported as a manual match.
#[derive(Debug, Clone)]
pub struct ManualFileTarget {
    pub library_id: String,
    pub target_type: String,
    pub target_id: String,
}

/// `match_type` recorded when a person chose the target.
const MANUAL_MATCH_TYPE: &str = "manual";
/// Where a library keeps duplicates a person removed, recoverable. Directly under the root.
const LIBRARY_TRASH_DIR: &str = ".librarian-trash";

/// In a match's reason: this one file is the entire book, so it belongs to every chapter.
const WHOLE_BOOK_REASON: &str = "one file holds the whole book";

/// How much of a book's length one file has to cover to count as the whole of it.
const WHOLE_BOOK_MINIMUM_COVERAGE: f64 = 0.8;

/// Whether a book's one linked file is the whole book, rather than the first of many.
fn file_covers_book(file_seconds: i64, book_seconds: f64) -> bool {
    book_seconds > 0.0 && file_seconds as f64 >= book_seconds * WHOLE_BOOK_MINIMUM_COVERAGE
}

/// `match_type` recorded when the Torrent row named the exact target.
const TORRENT_LINK_MATCH_TYPE: &str = "torrent_link";

/// `match_type` recorded when the Torrent row named the *parent* (show, album,
/// audiobook) and the individual file was resolved by parsing/ordering.
const TORRENT_LINK_PARSED_MATCH_TYPE: &str = "torrent_link_parsed";

/// Video files smaller than this are treated as samples/extras when the same
/// release also contains a larger video file (design.md Q54).
const MIN_IMPORT_VIDEO_BYTES: i64 = 50 * 1024 * 1024;

/// One file offered to the importer. Files unpacked from archives go through the same
/// hardlink/copy transfer as seeding payload; their staging directory is removed after
/// the import, so an extracted file ends up moved rather than duplicated.
#[derive(Debug, Clone)]
struct ImportFile {
    file: SourceFileImport,
}

/// Result of splitting a torrent's files by what the importer can do with them.
#[derive(Debug, Default)]
struct PartitionedImportFiles {
    media: Vec<SourceFileImport>,
    archives: Vec<SourceFileImport>,
    ignored_extensions: BTreeSet<String>,
}

/// A file deliberately not imported (sample, extras, undersized).
#[derive(Debug, Clone)]
struct SkippedImportFile {
    source_path: String,
    reason: String,
}

/// A file that reached matching but found no target.
#[derive(Debug, Clone)]
struct UnmatchedImportFile {
    source_path: String,
    reason: String,
}

/// Everything needed to explain a torrent's final post-process status
/// (design.md Q55).
#[derive(Debug, Default)]
struct ImportDiagnostics {
    archive_failures: Vec<String>,
    skipped: Vec<SkippedImportFile>,
    unmatched: Vec<UnmatchedImportFile>,
    notes: Vec<String>,
    ignored_extensions: BTreeSet<String>,
    /// Human label of what the torrent was grabbed for, e.g. `'The Expanse'`.
    target_label: Option<String>,
}

/// Wanted-target linkage stamped on the `Torrent` row at grab time.
#[derive(Debug, Clone, Default)]
struct TorrentImportLink {
    user_id: Option<String>,
    name: Option<String>,
    library_id: Option<String>,
    movie_id: Option<String>,
    episode_id: Option<String>,
    track_id: Option<String>,
    chapter_id: Option<String>,
    show_id: Option<String>,
    album_id: Option<String>,
    audiobook_id: Option<String>,
    season: Option<i32>,
    /// A person chose to put better copies on tracks that already have files.
    better_copies: bool,
}

impl TorrentImportLink {
    fn from_torrent(row: &crate::graphql::entities::Torrent) -> Self {
        Self {
            user_id: Some(row.user_id.clone()),
            name: Some(row.name.clone()),
            library_id: row.library_id.clone(),
            movie_id: row.movie_id.clone(),
            episode_id: row.episode_id.clone(),
            track_id: row.track_id.clone(),
            chapter_id: row.chapter_id.clone(),
            show_id: row.show_id.clone(),
            album_id: row.album_id.clone(),
            audiobook_id: row.audiobook_id.clone(),
            season: row.season,
            better_copies: false,
        }
    }
    fn has_target(&self) -> bool {
        self.movie_id.is_some()
            || self.episode_id.is_some()
            || self.track_id.is_some()
            || self.chapter_id.is_some()
            || self.show_id.is_some()
            || self.album_id.is_some()
            || self.audiobook_id.is_some()
    }
}

/// Private proof established before the processing/error boundary. Status writes use
/// this persisted ID and owner, never a fresh system lookup of an untrusted hash.
#[derive(Debug, Clone)]
struct AuthorizedTorrentImport {
    id: String,
    info_hash: String,
    owner_id: String,
    actor: AuthUser,
    link: TorrentImportLink,
}

/// A concrete import target derived from the recorded torrent linkage.
#[derive(Debug, Clone)]
struct LinkedImportTarget {
    library_id: String,
    target_type: String,
    target_id: String,
    target_name: Option<String>,
    match_type: &'static str,
    reason: String,
    /// The file's name in the library, when it isn't named after what it's linked to.
    file_stem: Option<String>,
}

/// Per-file plan produced from the torrent linkage.
#[derive(Debug, Default, Clone)]
struct LinkedImportPlan {
    targets: HashMap<String, LinkedImportTarget>,
    /// Why a specific file could not be placed even though linkage exists.
    unmatched_reasons: HashMap<String, String>,
    notes: Vec<String>,
    target_label: Option<String>,
    /// Chapters to point at the imported files once they have landed.
    book_writes: Option<BookWrites>,
    /// Files in this release that belong to something else and are left alone.
    left_alone: HashSet<String>,
    /// A person has to decide: nothing is imported and the release isn't tried again.
    review: Option<String>,
    /// This release can never satisfy the grab, so another one should be tried.
    hard_failure: Option<String>,
    /// One plain line for the torrent, replacing the per-file detail.
    summary: Option<String>,
    /// Each file's place in a book of several files, and the book's title, for when a library's
    /// naming pattern has nothing in it to tell the files apart.
    book_file_numbers: HashMap<String, usize>,
    book_title: Option<String>,
}

/// What an import writes to a book's chapters after its files land.
#[derive(Debug, Clone, Default)]
struct BookWrites {
    audiobook_id: String,
    /// Chapter id, source path of the file holding it, and where it starts in that file.
    placements: Vec<(String, String, f64)>,
    source: &'static str,
    shape: &'static str,
}

/// What matching thinks of a download, for showing before and after anything moves.
#[derive(Debug, Clone, Default)]
pub struct TorrentMatchPreview {
    /// `ready`, `review`, `unlinked` or `none`.
    pub status: &'static str,
    pub summary: Option<String>,
    pub reason: Option<String>,
    /// A person can accept this anyway.
    pub can_accept: bool,
    pub chapter_source: Option<String>,
    pub file_shape: Option<String>,
    pub evidence: Vec<String>,
    /// False while it rests on names and sizes alone.
    pub complete_facts: bool,
    pub groups: Vec<PreviewGroup>,
    pub files: Vec<PreviewFile>,
}

/// One book, or what looks like one, in a download.
#[derive(Debug, Clone, Default)]
pub struct PreviewGroup {
    pub key: String,
    pub label: String,
    pub file_count: i32,
    pub seconds: Option<f64>,
    pub is_target: bool,
    pub book_id: Option<String>,
    pub book_title: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PreviewFile {
    pub path: String,
    pub group_key: Option<String>,
    /// `match`, `review`, `other` or `none` for a book; `album` or `new` for a music file, by
    /// whether its album is already held.
    pub status: &'static str,
    pub target: Option<String>,
    pub detail: Option<String>,
    pub warning: Option<String>,
}

/// What an import should bring files in as, in place of what the download was grabbed for.
#[derive(Debug, Clone, Default)]
struct ImportLinkPatch {
    movie_id: Option<String>,
    show_id: Option<String>,
    season: Option<i32>,
    album_id: Option<String>,
    /// Better copies replace the files tracks have now, which stay on disk.
    better_copies: bool,
}

/// How a person told an import to treat a release.
#[derive(Debug, Clone, Default)]
pub struct BookImportChoice {
    /// Import this group of files as the book.
    pub group_key: Option<String>,
    /// Import even though the plan asks for review.
    pub accept_review: bool,
}

#[derive(Debug, Clone)]
struct LinkedEpisodeRow {
    id: String,
    season: Option<i32>,
    episode: Option<i32>,
    title: Option<String>,
}

/// How an import says a file is one the library already has.
const ALREADY_IN_LIBRARY: &str = "Already in the library";
/// How an import says the library has another copy of a file's track, and keeps it.
const KEEPS_OWN_COPY: &str = "The library keeps its own copy";

/// A file the import left alone because the library already holds its place.
fn held_by_library(reason: &str) -> bool {
    reason.starts_with(ALREADY_IN_LIBRARY) || reason.starts_with(KEEPS_OWN_COPY)
}

#[derive(Debug, Clone)]
struct LinkedTrackRow {
    id: String,
    title: String,
    track_number: Option<i32>,
    disc_number: Option<i32>,
    seconds: Option<f64>,
    /// The size of the file already on this track, if there is one.
    held_size: Option<i64>,
    /// Where that file is.
    held_path: Option<String>,
}

#[derive(Debug, Clone)]
struct ScanJob {
    library_id: String,
    scan_run_id: String,
    scope: ScanScope,
}

/// What one scan looks at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScanScope {
    /// The whole library.
    Full,
    /// Only these paths (files or folders, each one under the library's root) and the records
    /// under them: what a folder watch saw change (`docs/design.md` Q116).
    Changes(Vec<String>),
}

impl ScanScope {
    fn is_full(&self) -> bool {
        matches!(self, Self::Full)
    }

    /// Whether a catalogued path is one this scan looks at.
    fn covers(&self, path: &str) -> bool {
        match self {
            Self::Full => true,
            Self::Changes(targets) => targets
                .iter()
                .any(|target| path_is_at_or_under(path, target)),
        }
    }
}

/// `path` is `target` or something inside it.
fn path_is_at_or_under(path: &str, target: &str) -> bool {
    let target = target.trim_end_matches('/');
    path == target
        || path
            .strip_prefix(target)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Whether a scan takes a file of this name for media of the library: not hidden, and with one
/// of its extensions. `name` may be a whole path; only its last part counts.
fn is_reported_media_name(name: &str, allowed_extensions: &[&str]) -> bool {
    let name = name.rsplit('/').next().unwrap_or(name);
    if name.starts_with('.') {
        return false;
    }
    let extension = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    allowed_extensions
        .iter()
        .any(|allowed| *allowed == extension)
}

/// A walked media file as the scan takes it, with its path relative to the library `root`.
fn discovered_media_file(
    path: StoragePath,
    meta: Option<crate::services::library_storage::EntryMeta>,
    root: &StoragePath,
) -> DiscoveredMediaFile {
    let extension = path
        .extension()
        .map(|extension| extension.to_ascii_lowercase())
        .unwrap_or_default();
    let relative_path = path.relative_to(root);
    DiscoveredMediaFile {
        absolute_path: path.into_string(),
        relative_path,
        extension,
        size: meta
            .as_ref()
            .and_then(|meta| i64::try_from(meta.size).ok())
            .unwrap_or(i64::MAX),
        modified_at: meta
            .and_then(|meta| meta.modified)
            .map(DateTime::<Utc>::from)
            .map(|value| value.to_rfc3339_opts(SecondsFormat::Nanos, true)),
    }
}

/// Why the walk of a library could not read an entry.
#[derive(Debug)]
struct DiscoveryError {
    message: String,
    /// The storage itself could not be reached (a share that is down), not one entry unreadable.
    unavailable: bool,
}

#[derive(Debug)]
struct DiscoveredMediaFile {
    absolute_path: String,
    relative_path: Option<String>,
    extension: String,
    size: i64,
    modified_at: Option<String>,
}

struct ScanMediaFileCreate<'a> {
    path: &'a str,
    relative_path: Option<&'a str>,
    size: i64,
    modified_at: Option<&'a str>,
    content_type: Option<&'a str>,
}

#[derive(Debug, Clone)]
struct DuplicateCandidate {
    media_file_id: String,
    path: String,
    size: i64,
    analysis_state: &'static str,
    quality_status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DuplicateIssueDetails {
    kind: String,
    duplicate_path: String,
    keeper_path: String,
    size: i64,
    sha256: String,
}

#[derive(Debug, Clone)]
struct AnalyzeJob {
    media_file_id: String,
    path: String,
    scan_run_id: Option<String>,
    retry_issue_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QueueScanResult {
    pub queued: bool,
    pub scan_run_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DuplicateTrashResult {
    pub old_path: String,
    /// Where the removed copy went; `None` when the issue closed without moving anything.
    pub trash_path: Option<String>,
    /// Why the issue closed without moving anything: one of the copies had already gone.
    pub no_longer_duplicate: Option<String>,
}

/// What settling every open duplicate in a library did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DuplicateSettlement {
    /// Pairs settled by moving a copy to the trash.
    pub settled: usize,
    /// Issues closed because a copy had already gone, so nothing was left to settle.
    pub closed: usize,
    /// Pairs that need a person.
    pub left: usize,
}

/// Which of a duplicate pair to keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateKeep {
    /// The copy the scan chose.
    Keeper,
    /// The copy the issue is about.
    Duplicate,
    /// Whichever a title plays.
    Best,
}

/// When an issue was first recorded, in seconds; rows store Unix seconds or RFC 3339.
fn issue_seen_at(issue: &LibraryScanIssue) -> i64 {
    issue.created_at.parse::<i64>().unwrap_or_else(|_| {
        DateTime::parse_from_rfc3339(&issue.created_at)
            .map(|date| date.timestamp())
            .unwrap_or_default()
    })
}

/// How a duplicate issue closed because a copy had gone starts. A scan that finds the pair again
/// reopens it, unlike an issue a person settled.
pub const NO_LONGER_DUPLICATE: &str = "No longer a duplicate";

/// Why keeping the better copy needs a person.
pub const BOTH_COPIES_IN_USE: &str = "Both copies are played by titles, so choose which to keep";

#[derive(Debug, Clone)]
struct PendingProviderFallback {
    media_file_id: String,
    media_path: String,
    match_source: String,
}

#[derive(Debug, Clone, Copy, Default)]
struct ProviderFallbackSummary {
    pending_files: usize,
    matched_files: usize,
    unmatched_files: usize,
    error_files: usize,
    configuration_blocked: bool,
}

impl ProviderFallbackSummary {
    fn merge_outcome(&mut self, outcome: Self) {
        self.matched_files += outcome.matched_files;
        self.unmatched_files += outcome.unmatched_files;
        self.error_files += outcome.error_files;
        self.configuration_blocked |= outcome.configuration_blocked;
    }

    fn has_issues(&self) -> bool {
        self.configuration_blocked || self.unmatched_files > 0 || self.error_files > 0
    }
}

#[derive(Debug, Clone)]
struct MovieCandidateRow {
    id: String,
    title: String,
    year: Option<i32>,
    wanted: bool,
    has_file: bool,
}

#[derive(Debug, Clone)]
struct EpisodeCandidateRow {
    id: String,
    show_name: String,
    show_year: Option<i32>,
    season: i32,
    episode: i32,
    wanted: bool,
    has_file: bool,
}

#[derive(Debug, Clone)]
struct TrackCandidateRow {
    id: String,
    title: String,
    album_name: String,
    wanted: bool,
    has_file: bool,
}

#[derive(Debug, Clone)]
struct ChapterCandidateRow {
    id: String,
    title: String,
    audiobook_title: String,
    author_name: Option<String>,
    chapter_number: Option<i32>,
    wanted: bool,
    has_file: bool,
}

type TvShowFolderRow = (String, Option<String>, Option<i32>, Vec<i32>);

#[derive(Debug)]
struct Runtime {
    cancel: CancellationToken,
    scheduler_handle: JoinHandle<()>,
    scan_worker_handle: JoinHandle<()>,
    watch_handle: JoinHandle<()>,
    analyze_worker_handles: Vec<JoinHandle<()>>,
}

const WORKER_RESTART_LIMIT: usize = 3;

async fn supervise_worker<F, Fut>(worker_name: &'static str, cancel: CancellationToken, run: F)
where
    F: Fn(CancellationToken) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let mut restart_count = 0_usize;
    loop {
        let outcome = std::panic::AssertUnwindSafe(run(cancel.child_token()))
            .catch_unwind()
            .await;
        if cancel.is_cancelled() {
            return;
        }

        restart_count += 1;
        match outcome {
            Ok(()) => error!(
                service = "library_scan",
                worker = worker_name,
                restart_count,
                "Library scan worker exited unexpectedly"
            ),
            Err(_) => error!(
                service = "library_scan",
                worker = worker_name,
                restart_count,
                "Library scan worker panicked"
            ),
        }
        if restart_count >= WORKER_RESTART_LIMIT {
            error!(
                service = "library_scan",
                worker = worker_name,
                restart_count,
                "Library scan worker restart limit reached; service health is degraded"
            );
            return;
        }

        let backoff = Duration::from_secs(1_u64 << (restart_count - 1));
        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = tokio::time::sleep(backoff) => {}
        }
    }
}

#[derive(Debug, Clone)]
struct LibraryRow {
    id: String,
    user_id: String,
    name: String,
    path: String,
    library_type: String,
    auto_organize: bool,
    scan_interval_minutes: i32,
    scanning: bool,
    naming_pattern: String,
}

#[derive(Debug, Clone)]
struct MediaFileRow {
    id: String,
    library_id: Option<String>,
    path: String,
    original_name: Option<String>,
    episode_id: Option<String>,
    movie_id: Option<String>,
    track_id: Option<String>,
    chapter_id: Option<String>,
    match_type: Option<String>,
    size: i64,
    analyzed_at: Option<String>,
    file_modified_at: Option<String>,
    quality_status: Option<String>,
}

#[derive(Debug)]
struct ExistingMediaFileRow {
    id: String,
    path: String,
}

/// Where a catalogued file sits relative to its library: a scan's walk finds only the first kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordPlace {
    /// Under the library's folder, where the walk looks.
    Library,
    /// In the library's own trash folder, which the walk skips.
    Trash,
    /// Not under the library's folder at all, as after the library moved.
    OutsideRoot,
}

impl RecordPlace {
    fn of(path: &str, root: &StoragePath) -> Self {
        let Some(relative) = StoragePath::new(path).relative_to(root) else {
            return Self::OutsideRoot;
        };
        let first = relative.split('/').next().unwrap_or_default();
        // The trash is a folder, so a file sitting directly in the root with that name is not in it.
        if first == LIBRARY_TRASH_DIR && relative.contains('/') {
            Self::Trash
        } else {
            Self::Library
        }
    }
}

/// What one scan's reconciliation did with the records of files that were not found.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct MissingRecordSummary {
    removed_in_library: usize,
    removed_in_trash: usize,
    removed_outside_root: usize,
    /// Gone files whose record could not be removed.
    delete_failed: usize,
    /// Records kept because the storage could not say whether the file is there.
    kept_unchecked: usize,
}

impl MissingRecordSummary {
    fn removed(&self) -> usize {
        self.removed_in_library + self.removed_in_trash + self.removed_outside_root
    }
}

/// A file already linked to a library item, with what organizing needs to know about the link.
#[derive(Debug, Clone)]
struct MatchedFile {
    media_file_id: String,
    path: String,
    target_type: &'static str,
    target_id: String,
    match_type: Option<String>,
}

impl MatchedFile {
    /// The same item organizing plans from: a movie first, then an episode, track or chapter.
    fn of(file: &MediaFileRow) -> Option<Self> {
        let (target_type, target_id) = file
            .movie_id
            .as_deref()
            .map(|id| ("Movie", id))
            .or_else(|| file.episode_id.as_deref().map(|id| ("Episode", id)))
            .or_else(|| file.track_id.as_deref().map(|id| ("Track", id)))
            .or_else(|| file.chapter_id.as_deref().map(|id| ("Chapter", id)))?;
        Some(Self {
            media_file_id: file.id.clone(),
            path: file.path.clone(),
            target_type,
            target_id: target_id.to_string(),
            match_type: file.match_type.clone(),
        })
    }
}

#[derive(Debug, Clone, Default)]
struct ParsedMovieHint {
    title: Option<String>,
    year: Option<i32>,
}

#[derive(Debug, Clone, Default)]
struct ParsedEpisodeHint {
    show_name: Option<String>,
    season: Option<i32>,
    episode: Option<i32>,
    year: Option<i32>,
}

#[derive(Debug, Clone, Default)]
struct ParsedTrackHint {
    artist_name: Option<String>,
    album_name: Option<String>,
    title: Option<String>,
    track_number: Option<i32>,
}

#[derive(Debug, Clone, Default)]
struct ParsedChapterHint {
    author_name: Option<String>,
    audiobook_title: Option<String>,
    chapter_title: Option<String>,
    chapter_number: Option<i32>,
}

type ReleaseProbe = (
    Option<f64>,
    release_match::FileTags,
    Vec<release_match::EmbeddedChapter>,
);

pub struct LibraryScanService {
    manager: Arc<ServicesManager>,
    config: LibraryScanServiceConfig,
    runtime: RwLock<Option<Runtime>>,
    ffprobe_available: RwLock<bool>,
    /// Length, tags and chapter markers of files in downloads, by path and size.
    release_probe_cache: Mutex<HashMap<(String, i64), ReleaseProbe>>,
    /// Jev's answers, since previews ask again and each question costs something.
    jev_group_cache: Mutex<HashMap<String, Option<(usize, f64)>>>,
    /// Downloads waiting to be identified, and what asked for the most thorough look.
    identify_pending: std::sync::Mutex<HashMap<String, IdentifyTrigger>>,
    identify_active: Mutex<HashSet<String>>,
    /// One import of a download at a time. Finishing, identification and a person can all start one,
    /// and two at once can each add a record for the same file.
    torrent_import_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    identify_slots: Arc<tokio::sync::Semaphore>,
    /// Listening is the slowest thing identification does, so one book at a time.
    listen_slot: Arc<tokio::sync::Semaphore>,
    scan_tx: mpsc::Sender<ScanJob>,
    scan_rx: Arc<Mutex<mpsc::Receiver<ScanJob>>>,
    analyze_tx: mpsc::Sender<AnalyzeJob>,
    analyze_rx: Arc<Mutex<mpsc::Receiver<AnalyzeJob>>>,
    in_progress_scans: Arc<Mutex<HashSet<String>>>,
    queued_analysis: Arc<Mutex<HashSet<String>>>,
    scan_run_update_lock: Arc<Mutex<()>>,
    movie_candidate_cache: Arc<RwLock<HashMap<String, Vec<MovieCandidateRow>>>>,
    episode_candidate_cache: Arc<RwLock<HashMap<String, Vec<EpisodeCandidateRow>>>>,
    track_candidate_cache: Arc<RwLock<HashMap<String, Vec<TrackCandidateRow>>>>,
    chapter_candidate_cache: Arc<RwLock<HashMap<String, Vec<ChapterCandidateRow>>>>,
}

/// Finds an unanswered action notification about this same thing: the same action on the same
/// file, for the same reason. A different upgrade found for that file is still news.
fn open_action_notification_lookup(
    owner_id: &str,
    action_type: &str,
    media_file_id: Option<&str>,
    message: &str,
) -> serde_json::Value {
    let mut filter = serde_json::json!({
        "userId": { "eq": owner_id },
        "actionType": { "eq": action_type },
        "message": { "eq": message },
        "resolvedAt": { "isNull": true }
    });
    if let Some(media_file_id) = media_file_id {
        filter["mediaFileId"] = serde_json::json!({ "eq": media_file_id });
    }
    serde_json::json!({ "where": filter, "page": { "limit": 1, "offset": 0 } })
}

fn notifications_found(data: &serde_json::Value) -> bool {
    data.get("Notifications")
        .and_then(|notifications| notifications.get("Edges"))
        .and_then(|edges| edges.as_array())
        .is_some_and(|edges| !edges.is_empty())
}

impl LibraryScanService {
    const AUTO_ORGANIZE_MOVIE_CONFIDENCE_GUARD: f64 = 0.93;
    const AUTO_ORGANIZE_MOVIE_TITLE_SIMILARITY_GUARD: f64 = 0.86;
    const SCAN_QUEUE_CAPACITY: usize = 64;
    const ANALYSIS_QUEUE_CAPACITY: usize = 256;
    /// Must not exceed what a query on the raw pool returns: the ORM clamps an explicit limit to its
    /// default maximum without saying so, and these loops stop at the first short page. At 1,000 they
    /// read only 100 records, so a scan took every file past those for new and added a record for it.
    const ENTITY_PAGE_SIZE: i64 = 100;
    const MAX_SCAN_RUN_HISTORY_PER_LIBRARY: usize = 50;
    const RESOLVED_ISSUE_RETENTION_DAYS: i64 = 30;

    fn normalize_library_type(library_type: &str) -> String {
        let lower = library_type.trim().to_ascii_lowercase();
        match lower.as_str() {
            "movie" | "movies" => "movies".to_string(),
            "tv" | "show" | "shows" => "tv".to_string(),
            "music" | "album" | "albums" => "music".to_string(),
            "audiobook" | "audiobooks" => "audiobooks".to_string(),
            _ => lower,
        }
    }

    fn is_tv_library_type(library_type: &str) -> bool {
        Self::normalize_library_type(library_type) == "tv"
    }

    fn fallback_naming_pattern(library_type: &str) -> &'static str {
        match Self::normalize_library_type(library_type).as_str() {
            "tv" => "{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}",
            "movies" => "{title} ({year})/{title} ({year}).{ext}",
            "music" => "{artist}/{album} ({year})/{track:02} - {title}.{ext}",
            "audiobooks" => "{author}/{title}/{chapter:02} - {chapter_title}.{ext}",
            _ => "{original}.{ext}",
        }
    }

    pub fn new(manager: Arc<ServicesManager>, config: LibraryScanServiceConfig) -> Self {
        let (scan_tx, scan_rx) = mpsc::channel(Self::SCAN_QUEUE_CAPACITY);
        let (analyze_tx, analyze_rx) = mpsc::channel(Self::ANALYSIS_QUEUE_CAPACITY);

        Self {
            manager,
            config,
            runtime: RwLock::new(None),
            ffprobe_available: RwLock::new(true),
            release_probe_cache: Mutex::new(HashMap::new()),
            jev_group_cache: Mutex::new(HashMap::new()),
            identify_pending: std::sync::Mutex::new(HashMap::new()),
            identify_active: Mutex::new(HashSet::new()),
            torrent_import_locks: Mutex::new(HashMap::new()),
            identify_slots: Arc::new(tokio::sync::Semaphore::new(2)),
            listen_slot: Arc::new(tokio::sync::Semaphore::new(1)),
            scan_tx,
            scan_rx: Arc::new(Mutex::new(scan_rx)),
            analyze_tx,
            analyze_rx: Arc::new(Mutex::new(analyze_rx)),
            in_progress_scans: Arc::new(Mutex::new(HashSet::new())),
            queued_analysis: Arc::new(Mutex::new(HashSet::new())),
            scan_run_update_lock: Arc::new(Mutex::new(())),
            movie_candidate_cache: Arc::new(RwLock::new(HashMap::new())),
            episode_candidate_cache: Arc::new(RwLock::new(HashMap::new())),
            track_candidate_cache: Arc::new(RwLock::new(HashMap::new())),
            chapter_candidate_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn graphql_schema(&self) -> Result<LibrarianSchema> {
        let graphql = self
            .manager
            .get_graphql()
            .await
            .ok_or_else(|| anyhow::anyhow!("GraphQL service not available"))?;

        graphql
            .schema()
            .await
            .ok_or_else(|| anyhow::anyhow!("GraphQL schema not available"))
    }

    async fn execute_mutation(
        &self,
        auth_user: &AuthUser,
        mutation: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        if let Some(owner) = variables.get("input").and_then(|input| input.get("userId")) {
            let owner = owner
                .as_str()
                .context("New record ownership must be explicit")?;
            Uuid::parse_str(owner).context(
                "New record ownership is unknown; system actors cannot own user records",
            )?;
        }
        let data = self.execute_graphql(auth_user, mutation, variables).await?;
        check_mutation_payloads(&data)?;
        Ok(data)
    }

    async fn execute_graphql(
        &self,
        auth_user: &AuthUser,
        document: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let _ = SCAN_GRAPHQL_ENTITY_OPERATIONS.try_with(|count| {
            count.set(count.get().saturating_add(1));
        });
        let schema = self.graphql_schema().await?;
        let request = Request::new(document)
            .variables(Variables::from_json(variables))
            .data(auth_user.clone())
            .data(AuthSubject::from_parts(
                auth_user.user_id.clone(),
                auth_user.role.iter().cloned().collect(),
                Vec::new(),
                None,
            ))
            .data(auth_user.user_id.clone());
        let response = schema.execute(request).await;
        if !response.errors.is_empty() {
            let msg = response
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join("; ");
            anyhow::bail!(msg);
        }
        Ok(serde_json::to_value(&response.data)?)
    }

    /// Page a top-level GraphQL connection to exhaustion while preserving the
    /// same response shape expected by the scan pipeline.
    async fn execute_graphql_paged(
        &self,
        auth_user: &AuthUser,
        document: &str,
        mut variables: serde_json::Value,
        connection_name: &str,
    ) -> Result<serde_json::Value> {
        const PAGE_SIZE: usize = 1_000;
        let variables = variables
            .as_object_mut()
            .context("Paged GraphQL variables must be an object")?;
        let mut offset = 0_usize;
        let mut merged_edges = Vec::new();
        let mut merged = None;

        loop {
            variables.insert(
                "page".to_string(),
                serde_json::json!({ "limit": PAGE_SIZE, "offset": offset }),
            );
            let page = self
                .execute_graphql(
                    auth_user,
                    document,
                    serde_json::Value::Object(variables.clone()),
                )
                .await?;
            let edges = page
                .get(connection_name)
                .and_then(|value| value.get("Edges"))
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default();
            let page_len = edges.len();
            merged_edges.extend(edges);
            if merged.is_none() {
                merged = Some(page);
            }
            if page_len < PAGE_SIZE {
                break;
            }
            offset = offset.saturating_add(PAGE_SIZE);
        }

        let mut merged = merged.unwrap_or_else(|| serde_json::json!({}));
        if let Some(edges) = merged
            .get_mut(connection_name)
            .and_then(|value| value.get_mut("Edges"))
        {
            *edges = serde_json::Value::Array(merged_edges);
        }
        Ok(merged)
    }

    async fn check_ffprobe_available(&self) -> bool {
        match Command::new("ffprobe").arg("-version").output().await {
            Ok(output) if output.status.success() => true,
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!(
                    status = ?output.status.code(),
                    stderr = %stderr,
                    "ffprobe startup check failed: command returned non-zero status"
                );
                false
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "ffprobe startup check failed: command not executable or not found"
                );
                false
            }
        }
    }

    async fn ensure_ffprobe_missing_notification(&self) -> Result<()> {
        let Some(bootstrap_auth_user) = self.try_system_auth_user(None).await? else {
            warn!("ffprobe is unavailable and no users exist to notify");
            return Ok(());
        };
        let users_data = self
            .execute_graphql_paged(
                &bootstrap_auth_user,
                r#"query NotificationUsers($page: PageInput) {
                    Users: users(page: $page) {
                        Edges: edges { Node: node { Id: id } }
                    }
                }"#,
                serde_json::json!({}),
                "Users",
            )
            .await?;

        let users: Vec<String> = users_data
            .get("Users")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                edge.get("Node")
                    .and_then(|n| n.get("Id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .collect();

        if users.is_empty() {
            warn!("ffprobe is unavailable and no users exist to notify");
            return Ok(());
        }

        for user_id in users {
            let auth_user = AuthUser::system_admin();

            let existing = self
                .execute_graphql(
                    &auth_user,
                    r#"query ExistingFfprobeNotification($where: NotificationWhereInput, $page: PageInput) {
                        Notifications: notifications(where: $where, page: $page) {
                            Edges: edges { Node: node { Id: id } }
                        }
                    }"#,
                    serde_json::json!({
                        "where": {
                            "userId": { "eq": user_id },
                            "category": { "eq": "CONFIGURATION" },
                            "title": { "eq": "ffprobe is not installed" }
                        },
                        "page": { "limit": 1, "offset": 0 }
                    }),
                )
                .await?;

            let already_exists = existing
                .get("Notifications")
                .and_then(|n| n.get("Edges"))
                .and_then(|e| e.as_array())
                .map(|edges| !edges.is_empty())
                .unwrap_or(false);

            if already_exists {
                continue;
            }

            let _ = self
                .execute_mutation(
                    &auth_user,
                    r#"mutation CreateFfprobeMissingNotification($input: CreateNotificationInput!) {
                        CreateNotification: createNotification(input: $input) {
                            Success: success
                            Error: error
                        }
                    }"#,
                    serde_json::json!({
                        "input": {
                            "userId": user_id,
                            "notificationType": "ERROR",
                            "category": "CONFIGURATION",
                            "title": "ffprobe is not installed",
                            "message": "Library analysis is unavailable because ffprobe is missing from PATH. Install ffmpeg/ffprobe and restart Librarian."
                        }
                    }),
                )
                .await?;
        }

        Ok(())
    }

    async fn create_notification(
        &self,
        auth_user: &AuthUser,
        owner_id: &str,
        notification_type: &str,
        category: &str,
        title: &str,
        message: &str,
    ) {
        // Repeated scans can encounter the same unresolved conflict. Keep its
        // original read state instead of producing a new warning every hour.
        let existing = async {
            use crate::graphql::entities::{Notification, NotificationWhereInput};
            use graphql_orm::graphql::filters::DateFilter;
            let database = self.database().await?;
            let rows = Notification::query(database.pool().pool())
                .filter(NotificationWhereInput {
                    user_id: Some(StringFilter {
                        eq: Some(owner_id.to_string()),
                        ..Default::default()
                    }),
                    category: Some(StringFilter {
                        eq: Some(category.to_string()),
                        ..Default::default()
                    }),
                    title: Some(StringFilter {
                        eq: Some(title.to_string()),
                        ..Default::default()
                    }),
                    resolved_at: Some(DateFilter {
                        is_null: Some(true),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .fetch_all()
                .await?;
            Ok::<_, anyhow::Error>(rows.iter().any(|row| row.message == message))
        }
        .await;
        match existing {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => {
                warn!(user_id = %owner_id, title, error = %error, "Failed to check existing scan notification");
                return;
            }
        }
        if let Err(e) = self
            .execute_mutation(
                auth_user,
                r#"mutation CreateLibraryScanNotification($input: CreateNotificationInput!) {
                    CreateNotification: createNotification(input: $input) {
                        Success: success
                        Error: error
                    }
                }"#,
                serde_json::json!({
                    "input": {
                        "userId": owner_id,
                        "notificationType": notification_type,
                        "category": category,
                        "title": title,
                        "message": message
                    }
                }),
            )
            .await
        {
            warn!(
                user_id = %owner_id,
                error = %e,
                "Failed to create notification: user_id={}, title={}, error={}",
                owner_id,
                title,
                e
            );
        }
    }

    async fn ensure_tmdb_scan_notification(
        &self,
        auth_user: &AuthUser,
        library: &LibraryRow,
        affected_files: usize,
    ) {
        const TITLE: &str = "TMDB API key required for movie scanning";

        let existing = self
            .execute_graphql(
                auth_user,
                r#"query ExistingTmdbScanNotification($where: NotificationWhereInput, $page: PageInput) {
                    Notifications: notifications(where: $where, page: $page) {
                        Edges: edges { Node: node { Id: id } }
                    }
                }"#,
                serde_json::json!({
                    "where": {
                        "userId": { "eq": library.user_id },
                        "libraryId": { "eq": library.id },
                        "category": { "eq": "CONFIGURATION" },
                        "title": { "eq": TITLE },
                        "resolvedAt": { "isNull": true }
                    },
                    "page": { "limit": 1, "offset": 0 }
                }),
            )
            .await;

        match existing {
            Ok(data)
                if data
                    .get("Notifications")
                    .and_then(|notifications| notifications.get("Edges"))
                    .and_then(|edges| edges.as_array())
                    .is_some_and(|edges| !edges.is_empty()) =>
            {
                return;
            }
            Ok(_) => {}
            Err(error) => {
                warn!(
                    user_id = %library.user_id,
                    library_id = %library.id,
                    error = %error,
                    "Failed to check for an existing TMDB scan notification"
                );
                return;
            }
        }

        let message = format!(
            "Librarian found {affected_files} movie file(s) in '{}', but cannot identify or add \
             them as movies because no TMDB API key is configured. Add the key in Settings > \
             Metadata, then scan this library again.",
            library.name
        );
        if let Err(error) = self
            .execute_mutation(
                auth_user,
                r#"mutation CreateTmdbScanNotification($input: CreateNotificationInput!) {
                    CreateNotification: createNotification(input: $input) {
                        Success: success
                        Error: error
                    }
                }"#,
                serde_json::json!({
                    "input": {
                        "userId": library.user_id,
                        "notificationType": "ACTION_REQUIRED",
                        "category": "CONFIGURATION",
                        "title": TITLE,
                        "message": message,
                        "libraryId": library.id
                    }
                }),
            )
            .await
        {
            warn!(
                user_id = %library.user_id,
                library_id = %library.id,
                error = %error,
                "Failed to create TMDB scan notification"
            );
        }
    }

    async fn resolve_tmdb_configuration_feedback(
        &self,
        auth_user: &AuthUser,
        library: &LibraryRow,
    ) {
        const TITLE: &str = "TMDB API key required for movie scanning";
        let now = Utc::now().to_rfc3339();
        let result = self
            .execute_mutation(
                auth_user,
                r#"mutation ResolveTmdbScanNotifications($where: NotificationWhereInput, $input: UpdateNotificationInput!) {
                    Updated: updateNotifications(where: $where, input: $input) {
                        Success: success
                        Error: error
                    }
                }"#,
                serde_json::json!({
                    "where": {
                        "userId": { "eq": library.user_id },
                        "libraryId": { "eq": library.id },
                        "category": { "eq": "CONFIGURATION" },
                        "title": { "eq": TITLE },
                        "resolvedAt": { "isNull": true }
                    },
                    "input": {
                        "resolvedAt": now,
                        "resolution": "TMDB is configured; a subsequent scan retried movie matching."
                    }
                }),
            )
            .await;
        if let Err(error) = result {
            warn!(
                library_id = %library.id,
                error = %error,
                "Failed to resolve obsolete TMDB configuration notifications"
            );
        }

        let Ok(database) = self.database().await else {
            return;
        };
        let issues = LibraryScanIssue::query(database.pool().pool())
            .filter(LibraryScanIssueWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                issue_code: Some(StringFilter {
                    eq: Some("TMDB_NOT_CONFIGURED".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await;
        let Ok(issues) = issues else {
            return;
        };
        for issue in issues
            .into_iter()
            .filter(|issue| issue.resolved_at.is_none())
        {
            let _ = self
                .update_scan_issue(
                    &issue.id,
                    UpdateLibraryScanIssueInput {
                        resolved_at: Some(Some(now.clone())),
                        resolution: Some(Some("TMDB configured and matching retried".to_string())),
                        ..Default::default()
                    },
                )
                .await;
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn ensure_auto_organize_issue_notification(
        &self,
        auth_user: &AuthUser,
        library: &LibraryRow,
        media_file_id: &str,
        media_path: &str,
        title: &str,
        attempted_operation: &str,
        sanitized_reason: &str,
    ) {
        let existing = self
            .execute_graphql(
                auth_user,
                r#"query ExistingAutoOrganizeIssue($where: NotificationWhereInput, $page: PageInput) {
                    Notifications: notifications(where: $where, page: $page) {
                        Edges: edges { Node: node { Id: id } }
                    }
                }"#,
                serde_json::json!({
                    "where": {
                        "userId": { "eq": library.user_id },
                        "libraryId": { "eq": library.id },
                        "mediaFileId": { "eq": media_file_id },
                        "category": { "eq": "ORGANIZATION" },
                        "title": { "eq": title },
                        "resolvedAt": { "isNull": true }
                    },
                    "page": { "limit": 1, "offset": 0 }
                }),
            )
            .await;

        match existing {
            Ok(data)
                if data
                    .get("Notifications")
                    .and_then(|notifications| notifications.get("Edges"))
                    .and_then(|edges| edges.as_array())
                    .is_some_and(|edges| !edges.is_empty()) =>
            {
                return;
            }
            Ok(_) => {}
            Err(error) => {
                warn!(
                    user_id = %library.user_id,
                    library_id = %library.id,
                    media_file_id = %media_file_id,
                    attempted_operation,
                    error = %error,
                    "Failed to check for an existing auto-organize issue notification"
                );
                return;
            }
        }

        let message = format!(
            "Auto-organize stopped without approving a file move. Library ID: {}. Media file ID: \
             {}. Current path: '{}'. Attempted operation: {}. Reason: {}",
            library.id, media_file_id, media_path, attempted_operation, sanitized_reason
        );
        if let Err(error) = self
            .execute_mutation(
                auth_user,
                r#"mutation CreateAutoOrganizeIssue($input: CreateNotificationInput!) {
                    CreateNotification: createNotification(input: $input) {
                        Success: success
                        Error: error
                    }
                }"#,
                serde_json::json!({
                    "input": {
                        "userId": library.user_id,
                        "notificationType": "ERROR",
                        "category": "ORGANIZATION",
                        "title": title,
                        "message": message,
                        "libraryId": library.id,
                        "mediaFileId": media_file_id
                    }
                }),
            )
            .await
        {
            warn!(
                user_id = %library.user_id,
                library_id = %library.id,
                media_file_id = %media_file_id,
                attempted_operation,
                error = %error,
                "Failed to create auto-organize issue notification"
            );
        }
    }

    /// Like [`create_notification`](Self::create_notification), but also sets
    /// `libraryId`/`mediaFileId`/`actionType`/`actionData` so the frontend can
    /// render an actionable notification (Q38's "Upgrade"/"Keep Current"
    /// pattern) and a mutation can later look up what to do when approved.
    #[allow(clippy::too_many_arguments)]
    async fn create_action_notification(
        &self,
        auth_user: &AuthUser,
        owner_id: &str,
        notification_type: &str,
        category: &str,
        title: &str,
        message: &str,
        library_id: Option<&str>,
        media_file_id: Option<&str>,
        pending_match_id: Option<&str>,
        action_type: &str,
        action_data: &str,
    ) {
        // Whatever finds this keeps running until the question is answered, so ask it once.
        let open = self
            .execute_graphql(
                auth_user,
                r#"query OpenActionNotification($where: NotificationWhereInput, $page: PageInput) {
                    Notifications: notifications(where: $where, page: $page) {
                        Edges: edges { Node: node { Id: id } }
                    }
                }"#,
                open_action_notification_lookup(owner_id, action_type, media_file_id, message),
            )
            .await;
        match open {
            Ok(data) if notifications_found(&data) => return,
            Ok(_) => {}
            Err(error) => {
                // Better to skip one than to risk another run of duplicates.
                warn!(user_id = %owner_id, title, error = %error, "Failed to check for an open action notification");
                return;
            }
        }
        if let Err(e) = self
            .execute_mutation(
                auth_user,
                r#"mutation CreateQualityUpgradeNotification($input: CreateNotificationInput!) {
                    CreateNotification: createNotification(input: $input) {
                        Success: success
                        Error: error
                    }
                }"#,
                serde_json::json!({
                    "input": {
                        "userId": owner_id,
                        "notificationType": notification_type,
                        "category": category,
                        "title": title,
                        "message": message,
                        "libraryId": library_id,
                        "mediaFileId": media_file_id,
                        "pendingMatchId": pending_match_id,
                        "actionType": action_type,
                        "actionData": action_data,
                    }
                }),
            )
            .await
        {
            warn!(
                user_id = %owner_id,
                error = %e,
                "Failed to create action notification: user_id={}, title={}, error={}",
                owner_id,
                title,
                e
            );
        }
    }

    async fn get_movie_title_year(
        &self,
        auth_user: &AuthUser,
        movie_id: &str,
    ) -> Result<Option<(String, Option<i32>)>> {
        let data = self
            .execute_graphql(
                auth_user,
                r#"query MovieTitleYear($id: String!) {
                    Movie: movie(id: $id) { Id: id Title: title Year: year }
                }"#,
                serde_json::json!({ "id": movie_id }),
            )
            .await?;

        let movie = data.get("Movie");
        let title = movie
            .and_then(|m| m.get("Title"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let year = movie
            .and_then(|m| m.get("Year"))
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        Ok(title.map(|t| (t, year)))
    }

    async fn should_skip_auto_organize_for_match(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        media_file_id: &str,
        media_path: &str,
        match_result: &MatchResult,
    ) -> Result<Option<String>> {
        if match_result.matched_type.as_deref() != Some("Movie") {
            return Ok(None);
        }

        let Some(movie_id) = match_result.matched_id.as_deref() else {
            return Ok(None);
        };

        let Some((matched_title, matched_year)) =
            self.get_movie_title_year(auth_user, movie_id).await?
        else {
            return Ok(None);
        };

        let parsed = Self::parse_movie_hint(media_path);
        let Some(parsed_title) = parsed.title.as_deref() else {
            return Ok(None);
        };
        let Some(parsed_year) = parsed.year else {
            return Ok(None);
        };
        let Some(matched_year) = matched_year else {
            return Ok(None);
        };

        if parsed_year == matched_year {
            return Ok(None);
        }

        let similarity = jaro_winkler(
            &Self::normalize_for_match(parsed_title),
            &Self::normalize_for_match(&matched_title),
        );

        if similarity >= Self::AUTO_ORGANIZE_MOVIE_TITLE_SIMILARITY_GUARD
            && match_result.confidence < Self::AUTO_ORGANIZE_MOVIE_CONFIDENCE_GUARD
        {
            let reason = format!(
                "Skipped auto-organize due to potential year-conflict match: media_file_id={}, library_id={}, path={}, parsed_title='{}', parsed_year={}, matched_movie_id={}, matched_title='{}', matched_year={}, confidence={:.3}, title_similarity={:.3}",
                media_file_id,
                library.id,
                media_path,
                parsed_title,
                parsed_year,
                movie_id,
                matched_title,
                matched_year,
                match_result.confidence,
                similarity
            );
            return Ok(Some(reason));
        }

        Ok(None)
    }

    /// Worker authority never depends on the first account in the database. A supplied
    /// owner validates an explicit scope taken from a Library/target/caller. The actor
    /// remains system; each new owned record names its recipient/owner separately.
    async fn try_system_auth_user(&self, owner_id: Option<&str>) -> Result<Option<AuthUser>> {
        if let Some(owner) = owner_id {
            Uuid::parse_str(owner).context("Worker target has no valid owner")?;
        }
        Ok(Some(AuthUser::system_admin()))
    }

    async fn system_auth_user(&self, owner_id: Option<&str>) -> Result<AuthUser> {
        self.try_system_auth_user(owner_id)
            .await?
            .context("System scan authority is unavailable")
    }

    async fn get_library(&self, library_id: &str) -> Result<LibraryRow> {
        let auth_user = self.system_auth_user(None).await?;
        let data = self
            .execute_graphql(
                &auth_user,
                r#"query GetLibraryForScan($id: String!) {
                    Library: library(id: $id) {
                        Id: id
                        UserId: userId
                        Name: name
                        Path: path
                        LibraryType: libraryType
                        AutoOrganize: autoOrganize
                        ScanIntervalMinutes: scanIntervalMinutes
                        Scanning: scanning
                        NamingPattern: namingPattern
                    }
                }"#,
                serde_json::json!({ "id": library_id }),
            )
            .await?;

        let library = data
            .get("Library")
            .ok_or_else(|| anyhow::anyhow!("Library not found"))?;

        Ok(LibraryRow {
            id: library
                .get("Id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Library.Id missing"))?
                .to_string(),
            user_id: library
                .get("UserId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Library.UserId missing"))?
                .to_string(),
            name: library
                .get("Name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Library.Name missing"))?
                .to_string(),
            path: library
                .get("Path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Library.Path missing"))?
                .to_string(),
            library_type: library
                .get("LibraryType")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Library.LibraryType missing"))?
                .to_string(),
            auto_organize: library
                .get("AutoOrganize")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            scan_interval_minutes: library
                .get("ScanIntervalMinutes")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
            scanning: library
                .get("Scanning")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            naming_pattern: library
                .get("NamingPattern")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
        })
    }

    async fn resolve_library_naming_pattern(&self, library: &LibraryRow) -> Result<String> {
        if !library.naming_pattern.trim().is_empty() {
            validate_relative_library_path(Path::new(&library.naming_pattern))?;
            return Ok(library.naming_pattern.clone());
        }

        let normalized_type = Self::normalize_library_type(&library.library_type);
        let auth_user = self.system_auth_user(Some(&library.user_id)).await?;
        let data = self
            .execute_graphql(
                &auth_user,
                r#"query ResolveNamingPattern($libraryType: String!) {
                    NamingPatterns: namingPatterns(
                        where: { libraryType: { eq: $libraryType }, isDefault: { eq: true } }
                        page: { limit: 100 }
                    ) {
                        Edges: edges {
                            Node: node {
                                Pattern: pattern
                                IsSystem: isSystem
                                CreatedAt: createdAt
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "libraryType": normalized_type }),
            )
            .await?;

        let mut patterns: Vec<(String, bool, String)> = data
            .get("NamingPatterns")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                let node = edge.get("Node")?;
                let pattern = node.get("Pattern")?.as_str()?.to_string();
                let is_system = node
                    .get("IsSystem")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let created_at = node
                    .get("CreatedAt")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                Some((pattern, is_system, created_at))
            })
            .collect();
        patterns.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));

        let pattern = patterns
            .first()
            .map(|(pattern, _, _)| pattern.clone())
            .filter(|pattern| !pattern.trim().is_empty())
            .unwrap_or_else(|| Self::fallback_naming_pattern(&normalized_type).to_string());
        validate_relative_library_path(Path::new(&pattern))?;
        Ok(pattern)
    }

    /// How often a library gets a whole-library scan: its own schedule, and for a watched library
    /// at least hourly, the safety net for a change no notification told (Q116).
    fn full_scan_interval_minutes(auto_scan: bool, watched: bool, interval: i32) -> Option<i32> {
        const WATCH_SAFETY_NET_MINUTES: i32 = 60;
        match (auto_scan && interval > 0, watched) {
            (true, true) => Some(interval.min(WATCH_SAFETY_NET_MINUTES)),
            (true, false) => Some(interval),
            (false, true) => Some(WATCH_SAFETY_NET_MINUTES),
            (false, false) => None,
        }
    }

    async fn get_due_autoscan_libraries(&self) -> Result<Vec<String>> {
        let Some(auth_user) = self.try_system_auth_user(None).await? else {
            debug!("Skipping autoscan schedule evaluation because no users exist yet");
            return Ok(Vec::new());
        };
        let data = self
            .execute_graphql_paged(
                &auth_user,
                r#"query DueAutoscanLibraries($page: PageInput) {
                    Libraries: libraries(
                        where: { scanning: { eq: false } }
                        page: $page
                    ) {
                        Edges: edges {
                            Node: node {
                                Id: id
                                AutoScan: autoScan
                                WatchForChanges: watchForChanges
                                LastScannedAt: lastScannedAt
                                ScanIntervalMinutes: scanIntervalMinutes
                            }
                        }
                    }
                }"#,
                serde_json::json!({}),
                "Libraries",
            )
            .await?;

        let now = Utc::now();
        let mut due = Vec::new();
        let edges = data
            .get("Libraries")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        for edge in edges {
            let node = edge.get("Node").cloned().unwrap_or_default();
            let Some(id) = node.get("Id").and_then(|v| v.as_str()) else {
                continue;
            };
            let last_scanned_at = node
                .get("LastScannedAt")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let interval_minutes = node
                .get("ScanIntervalMinutes")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
            let auto_scan = node
                .get("AutoScan")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let watched = node
                .get("WatchForChanges")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let Some(interval_minutes) =
                Self::full_scan_interval_minutes(auto_scan, watched, interval_minutes)
            else {
                continue;
            };

            let Some(last) = last_scanned_at else {
                due.push(id.to_string());
                continue;
            };

            let parsed = chrono::DateTime::parse_from_rfc3339(&last)
                .map(|dt| dt.with_timezone(&Utc))
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&last, "%Y-%m-%d %H:%M:%S")
                        .map(|naive| naive.and_utc())
                });

            match parsed {
                Ok(last_dt) => {
                    if now.signed_duration_since(last_dt).num_minutes() >= interval_minutes as i64 {
                        due.push(id.to_string());
                    }
                }
                Err(_) => due.push(id.to_string()),
            }
        }

        Ok(due)
    }

    async fn clear_stale_library_scanning_state_on_startup(&self) -> Result<()> {
        let Some(auth_user) = self.try_system_auth_user(None).await? else {
            info!("Skipping startup scan-state reconciliation because no users exist yet");
            return Ok(());
        };
        let mut offset = 0usize;
        let limit = 500usize;
        let mut libraries_reset = 0usize;

        info!(
            page_limit = limit,
            "Starting startup scan-state reconciliation for libraries with Scanning=true: page_limit={}",
            limit
        );

        loop {
            let data = self
                .execute_graphql(
                    &auth_user,
                    r#"query LibrariesMarkedScanningOnStartup($where: LibraryWhereInput, $page: PageInput) {
                        Libraries: libraries(where: $where, page: $page) {
                            Edges: edges { Node: node { Id: id Name: name Scanning: scanning } }
                        }
                    }"#,
                    serde_json::json!({
                        "where": { "scanning": { "eq": true } },
                        "page": { "limit": limit, "offset": offset }
                    }),
                )
                .await?;

            let edges = data
                .get("Libraries")
                .and_then(|v| v.get("Edges"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if edges.is_empty() {
                break;
            }

            for edge in &edges {
                let node = edge.get("Node").unwrap_or(&serde_json::Value::Null);
                let library_id = node.get("Id").and_then(|v| v.as_str()).unwrap_or_default();
                let library_name = node
                    .get("Name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                if library_id.is_empty() {
                    continue;
                }

                self.set_library_scanning(&auth_user, library_id, false, false)
                    .await?;
                libraries_reset += 1;
                info!(
                    library_id = %library_id,
                    library_name = %library_name,
                    "Cleared stale library scanning state on startup: library_id={}, library_name={}",
                    library_id,
                    library_name
                );
            }

            if edges.len() < limit {
                break;
            }
            offset += limit;
        }

        info!(
            reset_libraries = libraries_reset,
            "Completed startup scan-state reconciliation: reset_libraries={}", libraries_reset
        );

        self.backfill_sort_keys().await;
        self.retire_stale_upgrade_notices().await;

        Ok(())
    }

    /// Upgrade decisions that can't or shouldn't be taken: ones from before upgrades carried what
    /// they need to be applied safely, and ones that aren't upgrades by today's rules (a film
    /// cropped to 1920x800 is as 1080p as the download that was offered in its place).
    async fn retire_stale_upgrade_notices(&self) {
        use crate::graphql::entities::{MediaFile, Notification, NotificationWhereInput};
        use crate::services::quality::{profile, scoring};
        use graphql_orm::graphql::filters::DateFilter;
        let Ok(database) = self.database().await else {
            return;
        };
        let pool = database.pool().pool();
        let rows = match Notification::query(pool)
            .filter(NotificationWhereInput {
                action_type: Some(StringFilter {
                    eq: Some("quality_upgrade".to_string()),
                    ..Default::default()
                }),
                resolved_at: Some(DateFilter {
                    is_null: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
        {
            Ok(rows) => rows,
            Err(error) => {
                warn!(error = %error, "Could not read pending upgrade decisions: {}", error);
                return;
            }
        };
        let mut retired = 0usize;
        for row in rows {
            let action: serde_json::Value = row
                .action_data
                .as_deref()
                .and_then(|text| serde_json::from_str(text).ok())
                .unwrap_or_default();
            let stale = if row.pending_match_id.is_none() {
                true
            } else {
                let source = action["sourcePath"].as_str().unwrap_or_default();
                let target = match row.media_file_id.as_deref() {
                    Some(id) => MediaFile::get(pool, &id.to_string()).await.ok().flatten(),
                    None => None,
                };
                match target {
                    Some(target) => {
                        let name = std::path::Path::new(source)
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or(source);
                        !scoring::is_upgrade(
                            &scoring::parse_release(name),
                            Some(&profile::parsed_release_for_media_file(&target)),
                        )
                    }
                    None => true,
                }
            };
            if !stale {
                continue;
            }
            let Ok(auth_user) = self.system_auth_user(Some(&row.user_id)).await else {
                continue;
            };
            let done = self
                .execute_mutation(
                    &auth_user,
                    r#"mutation RetireUpgrade($id: String!, $input: UpdateNotificationInput!) {
                        UpdateNotification: updateNotification(id: $id, input: $input) { Success: success Error: error }
                    }"#,
                    serde_json::json!({
                        "id": row.id,
                        "input": { "resolvedAt": chrono::Utc::now().to_rfc3339() },
                    }),
                )
                .await;
            match done {
                Ok(_) => retired += 1,
                Err(error) => {
                    warn!(notification_id = %row.id, error = %error, "Could not retire a stale upgrade decision: {}", error)
                }
            }
        }
        if retired > 0 {
            info!(
                retired,
                "Retired {} upgrade decision(s) that were stale or not upgrades", retired
            );
        }
    }

    /// Records from before sort keys were written get them here. Anyone's own choice stays.
    async fn backfill_sort_keys(&self) {
        use crate::services::graphql::entities::{
            Album, Artist, Audiobook, Collection, Movie, Show,
        };

        let Ok(database) = self.database().await else {
            return;
        };
        let pool = database.pool().pool();
        let mut changes: Vec<(&str, &str, String, String)> = Vec::new();
        let mut note = |entity, field, id: String, title: &str, key: Option<&str>| {
            let target = crate::services::write_rules::sort_key(title);
            let stale = match key.map(str::trim) {
                None | Some("") => true,
                Some(key) => key == title.trim(),
            };
            if stale && key != Some(target.as_str()) {
                changes.push((entity, field, id, target));
            }
        };
        for row in Movie::query(pool).fetch_all().await.unwrap_or_default() {
            note(
                "Movie",
                "sortTitle",
                row.id,
                &row.title,
                row.sort_title.as_deref(),
            );
        }
        for row in Audiobook::query(pool).fetch_all().await.unwrap_or_default() {
            note(
                "Audiobook",
                "sortTitle",
                row.id,
                &row.title,
                row.sort_title.as_deref(),
            );
        }
        for row in Show::query(pool).fetch_all().await.unwrap_or_default() {
            note(
                "Show",
                "sortName",
                row.id,
                &row.name,
                row.sort_name.as_deref(),
            );
        }
        for row in Album::query(pool).fetch_all().await.unwrap_or_default() {
            note(
                "Album",
                "sortName",
                row.id,
                &row.name,
                row.sort_name.as_deref(),
            );
        }
        for row in Artist::query(pool).fetch_all().await.unwrap_or_default() {
            note(
                "Artist",
                "sortName",
                row.id,
                &row.name,
                row.sort_name.as_deref(),
            );
        }
        for row in Collection::query(pool)
            .fetch_all()
            .await
            .unwrap_or_default()
        {
            note(
                "Collection",
                "sortName",
                row.id,
                &row.name,
                row.sort_name.as_deref(),
            );
        }
        let total = changes.len();
        for (entity, field, id, key) in changes {
            let mutation = format!(
                "mutation Backfill($id: String!, $input: Update{entity}Input!) {{ Result: update{entity}(id: $id, input: $input) {{ Success: success }} }}"
            );
            if let Err(error) = self
                .execute_mutation(
                    &AuthUser::system_admin(),
                    &mutation,
                    serde_json::json!({ "id": id, "input": { field: key } }),
                )
                .await
            {
                warn!(entity, id, error = %error, "Could not set a sort key");
            }
        }
        if total > 0 {
            info!(total, "Set sort keys on existing records");
        }
    }

    fn extensions_for_library(library_type: &str) -> &'static [&'static str] {
        match Self::normalize_library_type(library_type).as_str() {
            "movies" | "tv" => &[
                "mkv", "mp4", "avi", "m4v", "mov", "wmv", "flv", "webm", "mpeg", "mpg", "ts",
                "m2ts",
            ],
            "music" => &[
                "mp3", "flac", "m4a", "aac", "ogg", "opus", "wav", "wma", "aiff", "alac", "ape",
                "dsf", "dff",
            ],
            "audiobooks" => &["mp3", "m4a", "m4b", "aac", "ogg", "opus", "flac", "wav"],
            _ => &[
                "mkv", "mp4", "avi", "m4v", "mov", "wmv", "flv", "webm", "mpeg", "mpg", "ts",
                "m2ts",
            ],
        }
    }

    /// What a scan walk reports: the library's media, and for video libraries their sidecar
    /// subtitles too (Q118), which are kept apart from media files.
    fn walk_extensions_for_library(library_type: &str) -> &'static [&'static str] {
        match Self::normalize_library_type(library_type).as_str() {
            "music" | "audiobooks" => Self::extensions_for_library(library_type),
            _ => &[
                "mkv", "mp4", "avi", "m4v", "mov", "wmv", "flv", "webm", "mpeg", "mpg", "ts",
                "m2ts", "srt", "vtt", "ass", "ssa", "sub", "idx",
            ],
        }
    }

    fn discover_media_files(
        root: PathBuf,
        allowed_extensions: &'static [&'static str],
        cancel: CancellationToken,
    ) -> (
        mpsc::Receiver<std::result::Result<DiscoveredMediaFile, DiscoveryError>>,
        JoinHandle<()>,
    ) {
        let (tx, rx) = mpsc::channel(128);
        let handle = tokio::spawn(async move {
            let root = root.into_storage_path();
            // The library's own trash holds files a person removed as duplicates. They are not
            // library files: finding them again would match and organize them back out of it.
            let prune: WalkFilter =
                Arc::new(|entry| entry.depth == 1 && entry.name == LIBRARY_TRASH_DIR);
            let report: WalkFilter = Arc::new(move |entry| {
                entry.kind == EntryKind::File
                    && is_reported_media_name(entry.name, allowed_extensions)
            });
            let walk = library_storage()
                .walk(
                    &root,
                    WalkOptions {
                        prune: Some(prune),
                        report: Some(report),
                        with_metadata: true,
                    },
                    cancel.clone(),
                )
                .await;
            let mut walk = match walk {
                Ok(walk) => walk,
                Err(error) => {
                    let _ = tx
                        .send(Err(DiscoveryError {
                            message: format!("Unable to read library entry: {error}"),
                            unavailable: error.kind() == StorageErrorKind::Unavailable,
                        }))
                        .await;
                    return;
                }
            };
            while let Some(result) = walk.entries.recv().await {
                if cancel.is_cancelled() {
                    break;
                }
                let sent = match result {
                    Ok(entry) => {
                        tx.send(Ok(discovered_media_file(entry.path, entry.meta, &root)))
                            .await
                    }
                    Err(error) => {
                        tx.send(Err(DiscoveryError {
                            message: format!("Unable to read library entry: {error}"),
                            unavailable: error.kind() == StorageErrorKind::Unavailable,
                        }))
                        .await
                    }
                };
                if sent.is_err() {
                    break;
                }
            }
            // Dropping the receiver stops the walk; wait for it to finish.
            drop(walk.entries);
            let _ = walk.task.await;
        });
        (rx, handle)
    }

    /// Store the sidecar subtitles a scan walked past (Q118): each placed with its video, and
    /// rows of files no longer there removed. Failures are logged; they never fail a scan.
    async fn sync_scan_sidecars(
        &self,
        library: &LibraryRow,
        videos: &[(String, String)],
        subtitles: &[String],
    ) {
        use crate::services::subtitles::{sidecar, store};
        let Ok(schema) = self.graphql_schema().await else {
            return;
        };
        let Ok(database) = self.database().await else {
            return;
        };
        let db = database.pool();
        let ids_by_path: HashMap<&str, &str> = videos
            .iter()
            .map(|(path, id)| (path.as_str(), id.as_str()))
            .collect();
        let index = sidecar::VideoIndex::new(videos.iter().map(|(path, _)| path.as_str()));
        let mut found: HashMap<String, Vec<sidecar::SidecarFile>> = HashMap::new();
        for (video, file) in sidecar::assign(&index, subtitles) {
            if let Some(id) = ids_by_path.get(video.as_str()) {
                found.entry((*id).to_owned()).or_default().push(file);
            }
        }
        // Files that had sidecar rows but none now, as well as files with sidecars.
        let mut with_rows: HashSet<String> = HashSet::new();
        let ids = videos.iter().map(|(_, id)| id.clone()).collect::<Vec<_>>();
        for chunk in ids.chunks(500) {
            match Subtitle::query(db.pool())
                .filter(SubtitleWhereInput {
                    media_file_id: Some(StringFilter {
                        in_list: Some(chunk.to_vec()),
                        ..Default::default()
                    }),
                    source_type: Some(StringFilter {
                        in_list: Some(vec![
                            store::SOURCE_SIDECAR.to_owned(),
                            store::SOURCE_DOWNLOADED.to_owned(),
                        ]),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .fetch_all()
                .await
            {
                Ok(rows) => with_rows.extend(rows.into_iter().map(|row| row.media_file_id)),
                Err(error) => {
                    warn!(library_id = %library.id, %error, "Could not read sidecar subtitles");
                    return;
                }
            }
        }
        let mut counts = store::SyncCounts::default();
        let mut files = found.keys().cloned().collect::<HashSet<_>>();
        files.extend(with_rows);
        for media_file_id in files {
            let list = found.remove(&media_file_id).unwrap_or_default();
            match store::sync_rows(db, &schema, &media_file_id, &list).await {
                Ok(changed) => counts.add(changed),
                Err(error) => {
                    warn!(library_id = %library.id, %media_file_id, %error, "Could not store sidecar subtitles");
                }
            }
        }
        if counts != store::SyncCounts::default() {
            info!(
                library_id = %library.id,
                library_name = %library.name,
                created = counts.created,
                updated = counts.updated,
                removed = counts.removed,
                "Sidecar subtitles updated"
            );
        }
    }

    /// Store one video's sidecars, found on disk now (after analysis, an import or a move).
    async fn sync_video_sidecars(&self, media_file_id: &str, video_path: &str) {
        let (Ok(schema), Ok(database)) = (self.graphql_schema().await, self.database().await)
        else {
            return;
        };
        if let Err(error) = crate::services::subtitles::store::sync_video(
            database.pool(),
            &schema,
            media_file_id,
            video_path,
        )
        .await
        {
            debug!(%media_file_id, %error, "Could not look for sidecar subtitles");
        }
    }

    /// Move or copy a video's sidecars to sit beside it at `new_video`, renamed after it (Q118).
    /// `rows` are the sidecar rows to point at the new files after a move; on a copy (an import)
    /// there are none, and the rows come from the next look at the new file.
    async fn carry_sidecars(
        &self,
        library_root: &Path,
        old_video: &Path,
        new_video: &Path,
        sidecars: Vec<(
            Option<String>,
            crate::services::subtitles::sidecar::SidecarFile,
        )>,
        remove_source: bool,
        source_is_library: bool,
    ) {
        use crate::services::subtitles::sidecar::{carry_names, file_stem};
        let schema = self.graphql_schema().await.ok();
        let (Some(old_name), Some(new_name), Some(new_folder)) = (
            old_video
                .file_name()
                .map(|name| name.to_string_lossy().into_owned()),
            new_video
                .file_name()
                .map(|name| name.to_string_lossy().into_owned()),
            new_video.parent(),
        ) else {
            return;
        };
        let (old_stem, new_stem) = (
            file_stem(&old_name).to_owned(),
            file_stem(&new_name).to_owned(),
        );
        let files = sidecars
            .iter()
            .map(|(_, file)| file.clone())
            .collect::<Vec<_>>();
        let names = carry_names(&old_stem, &new_stem, &files);
        for ((row_id, sidecar), target_name) in sidecars.into_iter().zip(names) {
            let from = PathBuf::from(&sidecar.path);
            let to = new_folder.join(&target_name);
            if to == from {
                continue;
            }
            let mut pairs = vec![(from.clone(), to.clone())];
            if sidecar.bitmap {
                // A VobSub index travels with its pictures.
                pairs.push((from.with_extension("sub"), to.with_extension("sub")));
            }
            let mut placed = true;
            for (source, target) in pairs {
                match place_in_library(
                    library_root.to_path_buf(),
                    source.clone(),
                    target.clone(),
                    remove_source,
                    source_is_library,
                )
                .await
                {
                    Ok(NoClobberOutcome::Placed) => {}
                    Ok(NoClobberOutcome::TargetExists) => {
                        debug!(source = %source.display(), target = %target.display(), "A subtitle is already there; left as it is");
                        placed = false;
                    }
                    Err(error) => {
                        warn!(source = %source.display(), target = %target.display(), %error, "Could not carry a subtitle with its video");
                        placed = false;
                    }
                }
            }
            if placed
                && let (Some(id), Some(schema)) = (row_id.as_deref(), schema.as_ref())
                && let Err(error) =
                    crate::services::subtitles::store::move_row(schema, id, &to.to_string_lossy())
                        .await
            {
                warn!(subtitle_id = %id, %error, "Could not record a moved subtitle");
            }
        }
    }

    async fn hash_duplicate_candidate(
        candidate: DuplicateCandidate,
        cancel: CancellationToken,
    ) -> Result<(DuplicateCandidate, String)> {
        let mut file = library_storage()
            .open_read(&candidate.path)
            .await
            .with_context(|| "Unable to open duplicate candidate")?
            .reader;
        let mut digest = Sha256::new();
        let mut buffer = vec![0_u8; 1024 * 1024];
        loop {
            if cancel.is_cancelled() {
                anyhow::bail!("SCAN_CANCELLED");
            }
            let read = file
                .read(&mut buffer)
                .await
                .with_context(|| "Unable to read duplicate candidate")?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
        }
        Ok((candidate, hex::encode(digest.finalize())))
    }

    /// Whether a title plays this file: it's the file a movie, episode, track or chapter points at,
    /// not a second copy beside it.
    async fn file_in_use(&self, file: &MediaFile) -> bool {
        let Ok(database) = self.database().await else {
            return true;
        };
        let pool = database.pool().pool();
        let points_here = |held: Option<String>| held.as_deref() == Some(file.id.as_str());
        if let Some(id) = &file.track_id {
            return Track::get(pool, id)
                .await
                .ok()
                .flatten()
                .is_some_and(|row| points_here(row.media_file_id));
        }
        if let Some(id) = &file.episode_id {
            return Episode::get(pool, id)
                .await
                .ok()
                .flatten()
                .is_some_and(|row| points_here(row.media_file_id));
        }
        if let Some(id) = &file.movie_id {
            return crate::graphql::entities::Movie::get(pool, id)
                .await
                .ok()
                .flatten()
                .is_some_and(|row| points_here(row.media_file_id));
        }
        file.chapter_id.is_some()
    }

    /// Best copy to keep first: one a title plays, then one analysis found optimal, then the
    /// shorter path, then by name, so the choice is the same every time.
    async fn order_for_keeping(&self, copies: &mut [DuplicateCandidate]) {
        let mut in_use: HashMap<String, bool> = HashMap::new();
        if let Ok(database) = self.database().await {
            for copy in copies.iter() {
                if let Ok(Some(file)) =
                    MediaFile::get(database.pool().pool(), &copy.media_file_id).await
                {
                    in_use.insert(copy.media_file_id.clone(), self.file_in_use(&file).await);
                }
            }
        }
        copies.sort_by(|a, b| {
            let rank = |copy: &DuplicateCandidate| {
                (
                    !in_use.get(&copy.media_file_id).copied().unwrap_or(false),
                    copy.quality_status.as_deref() != Some("optimal"),
                    copy.path.len(),
                )
            };
            rank(a).cmp(&rank(b)).then_with(|| a.path.cmp(&b.path))
        });
    }

    /// Confirm same-size candidates with a full SHA-256 before reporting.
    /// This is deliberately review-only: scans never delete either path.
    async fn detect_duplicate_files(
        &self,
        scan_run_id: &str,
        library: &LibraryRow,
        candidates: Vec<DuplicateCandidate>,
        cancel: CancellationToken,
    ) -> Result<usize> {
        let mut by_size: HashMap<i64, Vec<DuplicateCandidate>> = HashMap::new();
        for candidate in candidates
            .into_iter()
            .filter(|candidate| candidate.size > 0)
        {
            by_size.entry(candidate.size).or_default().push(candidate);
        }

        let mut duplicate_count = 0_usize;
        for same_size in by_size.into_values().filter(|group| group.len() > 1) {
            let mut by_hash: HashMap<String, Vec<DuplicateCandidate>> = HashMap::new();
            for candidate in same_size {
                if cancel.is_cancelled() {
                    anyhow::bail!("SCAN_CANCELLED");
                }
                match Self::hash_duplicate_candidate(candidate.clone(), cancel.child_token()).await
                {
                    Ok((candidate, digest)) => {
                        by_hash.entry(digest).or_default().push(candidate);
                    }
                    Err(error) if error.to_string().contains("SCAN_CANCELLED") => {
                        anyhow::bail!("SCAN_CANCELLED");
                    }
                    Err(error) => {
                        warn!(
                            library_id = %library.id,
                            media_file_id = %candidate.media_file_id,
                            error = %error,
                            "Unable to verify a same-size duplicate candidate"
                        );
                        self.record_scan_issue(
                            scan_run_id,
                            library,
                            Some(&candidate.media_file_id),
                            "DUPLICATE_REVIEW",
                            "DUPLICATE_HASH_FAILED",
                            "WARNING",
                            "A same-size duplicate candidate could not be content-verified.",
                            Some("Check file permissions and retry the scan."),
                        )
                        .await?;
                    }
                }
            }

            for (digest, mut identical) in by_hash.into_iter().filter(|(_, group)| group.len() > 1)
            {
                // The same copy has to be the keeper every scan, or each scan reports the pair anew.
                self.order_for_keeping(&mut identical).await;
                let keeper = &identical[0];
                for duplicate in identical.iter().skip(1) {
                    let storage_relationship = if same_file::is_same_file(
                        &duplicate.path,
                        &keeper.path,
                    )
                    .unwrap_or(false)
                    {
                        "HARDLINK"
                    } else {
                        "COPY"
                    };
                    duplicate_count += 1;
                    self.record_scan_issue_with_details(
                        scan_run_id,
                        library,
                        Some(&duplicate.media_file_id),
                        "DUPLICATE_REVIEW",
                        "BYTE_IDENTICAL_DUPLICATE",
                        "WARNING",
                        &format!(
                            "Byte-identical duplicate detected: '{}' duplicates '{}'.",
                            duplicate.path, keeper.path
                        ),
                        Some(
                            "Review both paths and explicitly remove one only if it is not a wanted edition or seeding source.",
                        ),
                        Some(
                            serde_json::json!({
                                "kind": "byte-identical-duplicate",
                                "duplicatePath": duplicate.path,
                                "keeperPath": keeper.path,
                                "size": duplicate.size,
                                "sha256": digest.as_str(),
                                "storageRelationship": storage_relationship,
                                "duplicateAnalysisState": duplicate.analysis_state,
                                "keeperAnalysisState": keeper.analysis_state,
                                "duplicateQualityStatus": duplicate.quality_status,
                                "keeperQualityStatus": keeper.quality_status,
                            })
                            .to_string(),
                        ),
                    )
                    .await?;
                }
            }
        }
        Ok(duplicate_count)
    }

    async fn set_library_scanning(
        &self,
        auth_user: &AuthUser,
        library_id: &str,
        scanning: bool,
        set_last_scanned: bool,
    ) -> Result<()> {
        let mut input = serde_json::json!({
            "scanning": scanning,
        });

        if set_last_scanned {
            input["lastScannedAt"] = serde_json::json!(Utc::now().to_rfc3339());
        }

        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation UpdateLibraryForScan($id: String!, $input: UpdateLibraryInput!) {
                    UpdateLibrary: updateLibrary(id: $id, input: $input) { Success: success Error: error }
                }"#,
                serde_json::json!({
                    "id": library_id,
                    "input": input,
                }),
            )
            .await?;

        let success = data
            .get("UpdateLibrary")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !success {
            let err = data
                .get("UpdateLibrary")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to update library scan state");
            anyhow::bail!(err.to_string());
        }

        Ok(())
    }

    async fn create_media_file(
        &self,
        auth_user: &AuthUser,
        library_id: &str,
        input: ScanMediaFileCreate<'_>,
    ) -> Result<String> {
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation CreateMediaFileFromScan($input: CreateMediaFileInput!) {
                    CreateMediaFile: createMediaFile(input: $input) {
                        Success: success
                        Error: error
                        MediaFile: mediaFile { Id: id }
                    }
                }"#,
                serde_json::json!({
                    "input": {
                        "libraryId": library_id,
                        "path": input.path,
                        "relativePath": input.relative_path,
                        "originalName": Path::new(input.path).file_name().and_then(|n| n.to_str()),
                        "size": input.size,
                        "fileModifiedAt": input.modified_at,
                        "isHdr": false,
                        "contentType": input.content_type,
                        "addedAt": Utc::now().to_rfc3339(),
                    }
                }),
            )
            .await?;

        let success = data
            .get("CreateMediaFile")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !success {
            let err = data
                .get("CreateMediaFile")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to create media file");
            anyhow::bail!(err.to_string());
        }

        let id = data
            .get("CreateMediaFile")
            .and_then(|v| v.get("MediaFile"))
            .and_then(|v| v.get("Id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("CreateMediaFile did not return MediaFile.Id"))?;

        Ok(id.to_string())
    }

    fn media_file_version_changed(
        existing_size: i64,
        existing_modified_at: Option<&str>,
        discovered_size: i64,
        discovered_modified_at: Option<&str>,
    ) -> bool {
        if existing_size != discovered_size {
            return true;
        }
        match (existing_modified_at, discovered_modified_at) {
            (Some(existing), Some(discovered)) => existing != discovered,
            // Populate the new marker for legacy rows once so subsequent scans
            // can prove that the file remained unchanged.
            (None, Some(_)) => true,
            // If the filesystem cannot report mtime, size remains the only
            // available version signal and we avoid invalidating every scan.
            _ => false,
        }
    }

    async fn invalidate_changed_media_file(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        size: i64,
        modified_at: Option<&str>,
    ) -> Result<()> {
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation InvalidateChangedMediaFile($id: String!, $input: UpdateMediaFileInput!) {
                    UpdateMediaFile: updateMediaFile(id: $id, input: $input) {
                        Success: success
                        Error: error
                    }
                }"#,
                serde_json::json!({
                    "id": media_file_id,
                    "input": {
                        "size": size,
                        "fileModifiedAt": modified_at,
                        "container": null,
                        "videoCodec": null,
                        "audioCodec": null,
                        "width": null,
                        "height": null,
                        "duration": null,
                        "bitrate": null,
                        "resolution": null,
                        "isHdr": false,
                        "hdrType": null,
                        "audioChannels": null,
                        "metadata": null,
                        "analyzedAt": null,
                        "qualityStatus": null,
                        // design.md Q122/Q123: the previews were made from the old contents.
                        "pictureCrop": null,
                        "storyboard": null,
                        "previewsCheckedAt": null
                    }
                }),
            )
            .await?;
        if data
            .get("UpdateMediaFile")
            .and_then(|value| value.get("Success"))
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            let message = data
                .get("UpdateMediaFile")
                .and_then(|value| value.get("Error"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Failed to invalidate stale media analysis");
            anyhow::bail!(message.to_string());
        }
        Ok(())
    }

    /// The record to use for a file landing at `target`: the library's own if it already has one, in
    /// which case the unassigned record just made for the download is removed.
    async fn settle_on_library_record(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        library_id: &str,
        target: &Path,
    ) -> Result<String> {
        let database = self.database().await?;
        let pool = database.pool().pool();
        let existing = MediaFile::query(pool)
            .filter(MediaFileWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library_id.to_string()),
                    ..Default::default()
                }),
                path: Some(StringFilter {
                    eq: Some(target.to_string_lossy().to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .limit(1)
            .fetch_all()
            .await?
            .into_iter()
            .next();
        let Some(existing) = existing.filter(|row| row.id != media_file_id) else {
            return Ok(media_file_id.to_string());
        };
        let is_unassigned = MediaFile::get(pool, &media_file_id.to_string())
            .await?
            .is_some_and(|row| {
                row.library_id.is_none()
                    && row.episode_id.is_none()
                    && row.movie_id.is_none()
                    && row.track_id.is_none()
                    && row.chapter_id.is_none()
            });
        if is_unassigned {
            self.execute_mutation(
                auth_user,
                r#"mutation RemoveSourceRecord($id: String!) {
                    Result: deleteMediaFile(id: $id) { Success: success }
                }"#,
                serde_json::json!({ "id": media_file_id }),
            )
            .await?;
        }
        Ok(existing.id)
    }

    /// Re-importing a download used to add a new record for the same file every time, each with its
    /// own streams. One record per file stays: the one linked to a title, else the newest analyzed.
    /// Anything pointing at the others moves to it first, and no file on disk is touched.
    async fn collapse_duplicate_media_files(&self) -> Result<usize> {
        use crate::graphql::entities::{
            Notification, NotificationWhereInput, PlaybackProgress, PlaybackProgressWhereInput,
            PlaybackSession, PlaybackSessionWhereInput,
        };
        const BATCH: usize = 200;
        let database = self.database().await?;
        let pool = database.pool().pool();
        let owner = AuthUser::system_admin();
        // Library by library, like the scan's own lookup. Records with no library aren't visible to
        // this read, and one short page would look like the end of the table.
        let mut rows = Vec::new();
        for library in crate::graphql::entities::Library::query(pool)
            .fetch_all()
            .await?
        {
            let mut offset = 0_i64;
            loop {
                let page = MediaFile::query(pool)
                    .filter(MediaFileWhereInput {
                        library_id: Some(StringFilter {
                            eq: Some(library.id.clone()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                    .limit(Self::ENTITY_PAGE_SIZE)
                    .offset(offset)
                    .fetch_all()
                    .await?;
                let page_len = page.len();
                rows.extend(page.into_iter().map(|file| {
                    let linked = file.episode_id.is_some()
                        || file.movie_id.is_some()
                        || file.track_id.is_some()
                        || file.chapter_id.is_some();
                    let rank = (linked, file.analyzed_at.is_some(), file.added_at.clone());
                    (file.path, rank, file.id)
                }));
                if page_len < Self::ENTITY_PAGE_SIZE as usize {
                    break;
                }
                offset = offset.saturating_add(Self::ENTITY_PAGE_SIZE);
            }
        }
        let mut by_file: HashMap<_, Vec<_>> = HashMap::new();
        for (key, rank, id) in rows {
            by_file.entry(key).or_default().push((rank, id));
        }
        let mut keeper_of: HashMap<String, String> = HashMap::new();
        for (_, mut records) in by_file {
            if records.len() < 2 {
                continue;
            }
            records.sort_by(|a, b| b.0.cmp(&a.0));
            let keeper = records[0].1.clone();
            for (_, id) in records.into_iter().skip(1) {
                keeper_of.insert(id, keeper.clone());
            }
        }
        if keeper_of.is_empty() {
            return Ok(0);
        }
        let total = keeper_of.len();
        warn!(
            duplicates = total,
            "Collapsing duplicate media file records"
        );
        let doomed: Vec<String> = keeper_of.keys().cloned().collect();
        for ids in doomed.chunks(BATCH) {
            let filter = StringFilter {
                in_list: Some(ids.to_vec()),
                ..Default::default()
            };
            macro_rules! repoint {
                ($entity:ty, $where:ident, $update:literal, $input:literal) => {
                    for row in <$entity>::query(pool)
                        .filter($where {
                            media_file_id: Some(filter.clone()),
                            ..Default::default()
                        })
                        .fetch_all()
                        .await?
                    {
                        let Some(keeper) = row
                            .media_file_id
                            .as_deref()
                            .and_then(|id| keeper_of.get(id))
                        else {
                            continue;
                        };
                        self.execute_mutation(
                            &owner,
                            concat!(
                                "mutation Repoint($id: String!, $input: ",
                                $input,
                                "!) { Result: ",
                                $update,
                                "(id: $id, input: $input) { Success: success } }"
                            ),
                            serde_json::json!({ "id": row.id, "input": { "mediaFileId": keeper } }),
                        )
                        .await?;
                    }
                };
            }
            repoint!(
                PlaybackProgress,
                PlaybackProgressWhereInput,
                "updatePlaybackProgress",
                "UpdatePlaybackProgressInput"
            );
            repoint!(
                PlaybackSession,
                PlaybackSessionWhereInput,
                "updatePlaybackSession",
                "UpdatePlaybackSessionInput"
            );
            repoint!(
                Notification,
                NotificationWhereInput,
                "updateNotification",
                "UpdateNotificationInput"
            );
            let deleted = self
                .execute_mutation(
                    &owner,
                    r#"mutation RemoveDuplicateRecords($where: MediaFileWhereInput) {
                        Result: deleteMediaFiles(where: $where) { Success: success Error: error }
                    }"#,
                    serde_json::json!({ "where": { "id": { "inList": ids } } }),
                )
                .await?;
            if deleted
                .pointer("/Result/Success")
                .and_then(serde_json::Value::as_bool)
                != Some(true)
            {
                anyhow::bail!(
                    "{}",
                    deleted
                        .pointer("/Result/Error")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("Could not remove duplicate media file records")
                );
            }
        }
        info!(removed = total, "Collapsed duplicate media file records");
        Ok(total)
    }

    async fn load_media_files_by_path(
        &self,
        library_id: &str,
    ) -> Result<HashMap<String, MediaFileRow>> {
        let database = self.database().await?;
        let mut offset = 0_i64;
        let mut rows = HashMap::new();
        loop {
            let page = MediaFile::query(database.pool().pool())
                .filter(MediaFileWhereInput {
                    library_id: Some(StringFilter {
                        eq: Some(library_id.to_string()),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .limit(Self::ENTITY_PAGE_SIZE)
                .offset(offset)
                .fetch_all()
                .await
                .context("Failed to page media files for library scan")?;
            let page_len = page.len();
            for file in page {
                rows.insert(
                    file.path.clone(),
                    MediaFileRow {
                        id: file.id,
                        library_id: file.library_id,
                        path: file.path,
                        original_name: file.original_name,
                        episode_id: file.episode_id,
                        movie_id: file.movie_id,
                        track_id: file.track_id,
                        chapter_id: file.chapter_id,
                        match_type: file.match_type,
                        size: file.size,
                        analyzed_at: file.analyzed_at,
                        file_modified_at: file.file_modified_at,
                        quality_status: file.quality_status,
                    },
                );
            }
            if page_len < Self::ENTITY_PAGE_SIZE as usize {
                break;
            }
            offset = offset.saturating_add(Self::ENTITY_PAGE_SIZE);
        }
        Ok(rows)
    }

    async fn load_episodes_for_show_ids(&self, show_ids: &[String]) -> Result<Vec<Episode>> {
        let database = self.database().await?;
        let mut rows = Vec::new();
        for ids in show_ids.chunks(500) {
            let mut offset = 0_i64;
            loop {
                let page = Episode::query(database.pool().pool())
                    .filter(EpisodeWhereInput {
                        show_id: Some(StringFilter {
                            in_list: Some(ids.to_vec()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                    .limit(Self::ENTITY_PAGE_SIZE)
                    .offset(offset)
                    .fetch_all()
                    .await
                    .context("Failed to page episodes for library scan")?;
                let page_len = page.len();
                rows.extend(page);
                if page_len < Self::ENTITY_PAGE_SIZE as usize {
                    break;
                }
                offset = offset.saturating_add(Self::ENTITY_PAGE_SIZE);
            }
        }
        Ok(rows)
    }

    async fn load_tracks_for_library(&self, library_id: &str) -> Result<Vec<Track>> {
        let database = self.database().await?;
        let mut rows = Vec::new();
        let mut offset = 0_i64;
        loop {
            let page = Track::query(database.pool().pool())
                .filter(TrackWhereInput {
                    library_id: Some(StringFilter {
                        eq: Some(library_id.to_string()),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .limit(Self::ENTITY_PAGE_SIZE)
                .offset(offset)
                .fetch_all()
                .await
                .context("Failed to page tracks for library scan")?;
            let page_len = page.len();
            rows.extend(page);
            if page_len < Self::ENTITY_PAGE_SIZE as usize {
                break;
            }
            offset = offset.saturating_add(Self::ENTITY_PAGE_SIZE);
        }
        Ok(rows)
    }

    async fn load_chapters_for_audiobook_ids(
        &self,
        audiobook_ids: &[String],
    ) -> Result<Vec<Chapter>> {
        let database = self.database().await?;
        let mut rows = Vec::new();
        for ids in audiobook_ids.chunks(500) {
            let mut offset = 0_i64;
            loop {
                let page = Chapter::query(database.pool().pool())
                    .filter(ChapterWhereInput {
                        audiobook_id: Some(StringFilter {
                            in_list: Some(ids.to_vec()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                    .limit(Self::ENTITY_PAGE_SIZE)
                    .offset(offset)
                    .fetch_all()
                    .await
                    .context("Failed to page audiobook chapters for library scan")?;
                let page_len = page.len();
                rows.extend(page);
                if page_len < Self::ENTITY_PAGE_SIZE as usize {
                    break;
                }
                offset = offset.saturating_add(Self::ENTITY_PAGE_SIZE);
            }
        }
        Ok(rows)
    }

    async fn ensure_original_name(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        current_original_name: Option<&str>,
        path: &str,
    ) -> Result<()> {
        if current_original_name
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
        {
            return Ok(());
        }

        let Some(original_name) = Path::new(path).file_name().and_then(|n| n.to_str()) else {
            return Ok(());
        };

        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation EnsureMediaFileOriginalName($id: String!, $input: UpdateMediaFileInput!) {
                    UpdateMediaFile: updateMediaFile(id: $id, input: $input) { Success: success Error: error }
                }"#,
                serde_json::json!({
                    "id": media_file_id,
                    "input": {
                        "originalName": original_name,
                    }
                }),
            )
            .await?;

        let success = data
            .get("UpdateMediaFile")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !success {
            let err = data
                .get("UpdateMediaFile")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("failed to update media file original name");
            anyhow::bail!(err.to_string());
        }

        info!(
            media_file_id = %media_file_id,
            original_name = %original_name,
            path = %path,
            "Backfilled missing media file original name: media_file_id={}, original_name={}, path={}",
            media_file_id,
            original_name,
            path
        );
        Ok(())
    }

    fn content_type_for_ext(ext: &str, library_type: &str) -> Option<&'static str> {
        match Self::normalize_library_type(library_type).as_str() {
            "movies" | "tv" => Some("video"),
            "music" | "audiobooks" => Some("audio"),
            _ => {
                let video = [
                    "mkv", "mp4", "avi", "m4v", "mov", "wmv", "flv", "webm", "mpeg", "mpg", "ts",
                    "m2ts",
                ];
                if video.contains(&ext) {
                    Some("video")
                } else {
                    Some("audio")
                }
            }
        }
    }

    fn usize_count(value: usize) -> i32 {
        i32::try_from(value).unwrap_or(i32::MAX)
    }

    async fn wait_for_analysis_settlement(
        &self,
        media_file_ids: &[String],
        deadline: Duration,
    ) -> bool {
        if media_file_ids.is_empty() {
            return true;
        }
        let expected = media_file_ids.iter().collect::<HashSet<_>>();
        let started = tokio::time::Instant::now();
        loop {
            let pending = {
                let queued = self.queued_analysis.lock().await;
                queued.iter().any(|id| expected.contains(id))
            };
            if !pending {
                return true;
            }
            if started.elapsed() >= deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn queue_scan(&self, library_id: &str) -> Result<QueueScanResult> {
        {
            let in_progress = self.in_progress_scans.lock().await;
            if in_progress.contains(library_id) {
                return Ok(QueueScanResult {
                    queued: false,
                    scan_run_id: None,
                });
            }
        }
        self.queue_scan_with(library_id, ScanScope::Full).await
    }

    /// Queue a scan of `scope`. Unlike [`queue_scan`](Self::queue_scan) this also queues while
    /// the library is being scanned: the one running may have walked past the change already, and
    /// scans of one library run one after another.
    async fn queue_scan_with(&self, library_id: &str, scope: ScanScope) -> Result<QueueScanResult> {
        let library = self.get_library(library_id).await?;
        if let Err(error) = self.compact_scan_history(library_id).await {
            warn!(
                library_id,
                error = %error,
                "Failed to compact old library scan history"
            );
        }
        let scan_run = self.create_scan_run(&library, &scope).await?;
        if let Err(error) = self
            .scan_tx
            .send(ScanJob {
                library_id: library_id.to_string(),
                scan_run_id: scan_run.id.clone(),
                scope,
            })
            .await
        {
            let _ = self
                .update_scan_run(
                    &scan_run.id,
                    UpdateLibraryScanRunInput {
                        status: Some("FAILED".to_string()),
                        current_stage: Some("QUEUE".to_string()),
                        finished_at: Some(Some(Utc::now().to_rfc3339())),
                        error_code: Some(Some("QUEUE_UNAVAILABLE".to_string())),
                        summary: Some(Some(
                            "The scan worker queue was unavailable; retry the scan.".to_string(),
                        )),
                        ..Default::default()
                    },
                )
                .await;
            return Err(anyhow::anyhow!("failed to queue scan job: {error}"));
        }
        let scan_queue_depth = self
            .scan_tx
            .max_capacity()
            .saturating_sub(self.scan_tx.capacity());
        info!(
            library_id,
            scan_run_id = %scan_run.id,
            scan_queue_depth,
            scan_queue_capacity = self.scan_tx.max_capacity(),
            "Queued library scan"
        );

        Ok(QueueScanResult {
            queued: true,
            scan_run_id: Some(scan_run.id),
        })
    }

    async fn compact_scan_history(&self, library_id: &str) -> Result<()> {
        let database = self.database().await?;
        let library_filter = StringFilter {
            eq: Some(library_id.to_string()),
            ..Default::default()
        };
        let mut runs = LibraryScanRun::query(database.pool().pool())
            .filter(
                crate::services::graphql::entities::LibraryScanRunWhereInput {
                    library_id: Some(library_filter.clone()),
                    ..Default::default()
                },
            )
            .fetch_all()
            .await
            .context("Failed to read scan history for compaction")?;
        runs.sort_by(|left, right| right.created_at.cmp(&left.created_at));

        for run in runs
            .into_iter()
            .skip(Self::MAX_SCAN_RUN_HISTORY_PER_LIBRARY)
        {
            // Deleting the run removes its issues in the same transaction
            // (the LibraryScanRun hook in `AppMutationHook`).
            self.delete_scan_run(&run.id)
                .await
                .context("Failed to delete compacted scan run")?;
        }

        let cutoff = Utc::now() - chrono::Duration::days(Self::RESOLVED_ISSUE_RETENTION_DAYS);
        let resolved_issues = LibraryScanIssue::query(database.pool().pool())
            .filter(LibraryScanIssueWhereInput {
                library_id: Some(library_filter),
                ..Default::default()
            })
            .fetch_all()
            .await
            .context("Failed to read resolved scan issues for compaction")?;
        for issue in resolved_issues {
            let expired = issue
                .resolved_at
                .as_deref()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .is_some_and(|resolved_at| resolved_at.with_timezone(&Utc) < cutoff);
            if expired {
                self.delete_scan_issue(&issue.id)
                    .await
                    .context("Failed to delete expired resolved scan issue")?;
            }
        }
        Ok(())
    }

    pub async fn queue_analyze_job(&self, media_file_id: &str, path: &str) -> Result<bool> {
        self.queue_analyze_job_with_context(media_file_id, path, None, None)
            .await
    }

    async fn queue_analyze_job_for_run(
        &self,
        media_file_id: &str,
        path: &str,
        scan_run_id: Option<&str>,
    ) -> Result<bool> {
        self.queue_analyze_job_with_context(media_file_id, path, scan_run_id, None)
            .await
    }

    async fn queue_analyze_job_with_context(
        &self,
        media_file_id: &str,
        path: &str,
        scan_run_id: Option<&str>,
        retry_issue_id: Option<&str>,
    ) -> Result<bool> {
        if !*self.ffprobe_available.read().await {
            warn!(
                media_file_id = %media_file_id,
                path = %path,
                "Skipping analyze job because ffprobe is unavailable: media_file_id={}, path={}",
                media_file_id,
                path
            );
            return Ok(false);
        }

        let already_analyzed = match self.media_file_has_been_analyzed(media_file_id).await {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    media_file_id = %media_file_id,
                    path = %path,
                    error = %e,
                    "Failed to check existing analysis state; proceeding to queue analyze job: media_file_id={}, path={}, error={}",
                    media_file_id,
                    path,
                    e
                );
                false
            }
        };
        if already_analyzed {
            debug!(
                media_file_id = %media_file_id,
                path = %path,
                "Skipping analyze job because media file is already analyzed: media_file_id={}, path={}",
                media_file_id,
                path
            );
            return Ok(false);
        }

        {
            let mut queued = self.queued_analysis.lock().await;
            if queued.contains(media_file_id) {
                info!(
                    media_file_id = %media_file_id,
                    path = %path,
                    "Analyze job already queued; skipping duplicate enqueue: media_file_id={}, path={}",
                    media_file_id,
                    path
                );
                return Ok(false);
            }
            queued.insert(media_file_id.to_string());
        }

        if let Err(error) = self
            .analyze_tx
            .send(AnalyzeJob {
                media_file_id: media_file_id.to_string(),
                path: path.to_string(),
                scan_run_id: scan_run_id.map(str::to_string),
                retry_issue_id: retry_issue_id.map(str::to_string),
            })
            .await
        {
            self.queued_analysis.lock().await.remove(media_file_id);
            anyhow::bail!("failed to queue analyze job: {error}");
        }
        let analysis_queue_depth = self
            .analyze_tx
            .max_capacity()
            .saturating_sub(self.analyze_tx.capacity());

        info!(
            media_file_id = %media_file_id,
            path = %path,
            analysis_queue_depth,
            analysis_queue_capacity = self.analyze_tx.max_capacity(),
            "Queued analyze job: media_file_id={}, path={}",
            media_file_id,
            path
        );

        Ok(true)
    }

    pub async fn retry_analysis_issue(&self, issue_id: &str) -> Result<bool> {
        let database = self.database().await?;
        let issue = LibraryScanIssue::get(database.pool().pool(), &issue_id.to_string())
            .await
            .context("Failed to read scan issue")?
            .context("Scan issue not found")?;
        if issue.resolved_at.is_some() {
            anyhow::bail!("Scan issue is already resolved");
        }
        if issue.stage != "ANALYSIS" {
            anyhow::bail!("Only analysis issues can be retried");
        }
        let media_file_id = issue
            .media_file_id
            .as_deref()
            .context("Analysis issue is not linked to a media file")?;
        let media_file = MediaFile::get(database.pool().pool(), &media_file_id.to_string())
            .await
            .context("Failed to read media file for analysis retry")?
            .context("Media file for analysis retry no longer exists")?;
        if media_file.library_id.as_deref() != Some(issue.library_id.as_str()) {
            anyhow::bail!("Analysis issue no longer matches its media file library");
        }
        self.queue_analyze_job_with_context(media_file_id, &media_file.path, None, Some(issue_id))
            .await
    }

    async fn record_analysis_retry_outcome(
        &self,
        issue_id: &str,
        error: Option<&anyhow::Error>,
    ) -> Result<()> {
        let _guard = self.scan_run_update_lock.lock().await;
        let database = self.database().await?;
        let Some(issue) =
            LibraryScanIssue::get(database.pool().pool(), &issue_id.to_string()).await?
        else {
            return Ok(());
        };
        let input = if let Some(error) = error {
            let (code, message, remediation) = classify_analysis_error(error);
            UpdateLibraryScanIssueInput {
                issue_code: Some(code.to_string()),
                message: Some(message.to_string()),
                remediation: Some(Some(remediation.to_string())),
                details_json: Some(Some(
                    serde_json::json!({
                        "lastError": Self::bounded_issue_text(&error.to_string(), 4_000),
                        "lastAttemptAt": Utc::now().to_rfc3339(),
                    })
                    .to_string(),
                )),
                occurrence_count: Some(issue.occurrence_count.saturating_add(1)),
                ..Default::default()
            }
        } else {
            UpdateLibraryScanIssueInput {
                resolved_at: Some(Some(Utc::now().to_rfc3339())),
                resolution: Some(Some("Analysis retry completed successfully.".to_string())),
                ..Default::default()
            }
        };
        self.update_scan_issue(&issue.id, input)
            .await
            .context("Failed to persist analysis retry outcome")?;
        Ok(())
    }

    /// Settle a duplicate by keeping the better copy: the one a title plays. Neither being played
    /// keeps the one the scan chose; both being played is a person's call.
    pub async fn trash_duplicate_issue_keeping(
        &self,
        issue_id: &str,
        keep: DuplicateKeep,
    ) -> Result<DuplicateTrashResult> {
        let database = self.database().await?;
        let issue = LibraryScanIssue::get(database.pool().pool(), &issue_id.to_string())
            .await?
            .context("Duplicate issue not found")?;
        let keep = match keep {
            DuplicateKeep::Best => {
                let details: DuplicateIssueDetails = serde_json::from_str(
                    issue
                        .details_json
                        .as_deref()
                        .context("Duplicate issue has no verification context")?,
                )?;
                let duplicate = match issue.media_file_id.as_deref() {
                    Some(id) => MediaFile::get(database.pool().pool(), &id.to_string()).await?,
                    None => None,
                };
                let keeper = self
                    .library_file_at(&issue.library_id, &details.keeper_path)
                    .await?;
                let keeper_used = match &keeper {
                    Some(file) => self.file_in_use(file).await,
                    None => false,
                };
                let duplicate_used = match &duplicate {
                    Some(file) => self.file_in_use(file).await,
                    None => false,
                };
                match (keeper_used, duplicate_used) {
                    (true, true) => anyhow::bail!(BOTH_COPIES_IN_USE),
                    (false, true) => DuplicateKeep::Duplicate,
                    _ => DuplicateKeep::Keeper,
                }
            }
            other => other,
        };
        self.trash_duplicate_copy(issue_id, keep == DuplicateKeep::Duplicate)
            .await
    }

    /// For a library set to settle duplicates itself: keep the copy each pair's title plays and
    /// move the other to the recoverable trash. Pairs it can't decide stay open for a person.
    async fn settle_duplicates_if_asked(&self, library: &LibraryRow) {
        let Ok(database) = self.database().await else {
            return;
        };
        let asked = crate::graphql::entities::Library::get(database.pool().pool(), &library.id)
            .await
            .ok()
            .flatten()
            .and_then(|row| row.auto_resolve_duplicates)
            .unwrap_or(false);
        if !asked {
            return;
        }
        if let Ok(outcome) = self.settle_duplicates(&library.id).await
            && outcome.settled + outcome.left > 0
        {
            info!(
                library_id = %library.id,
                settled = outcome.settled,
                left = outcome.left,
                "Settled duplicates after a scan: library_id={}, settled={}, left={}",
                library.id,
                outcome.settled,
                outcome.left
            );
        }
    }

    /// Settle every open duplicate in a library by keeping the copy its title plays.
    pub async fn settle_duplicates(&self, library_id: &str) -> Result<DuplicateSettlement> {
        let open = self.open_duplicate_issues(library_id).await?;
        let mut outcome = DuplicateSettlement::default();
        for issue in open {
            match self
                .trash_duplicate_issue_keeping(&issue.id, DuplicateKeep::Best)
                .await
            {
                Ok(result) if result.no_longer_duplicate.is_some() => outcome.closed += 1,
                Ok(_) => outcome.settled += 1,
                Err(error) => {
                    outcome.left += 1;
                    debug!(issue_id = %issue.id, error = %error, "Left a duplicate for a person to settle");
                }
            }
        }
        Ok(outcome)
    }

    /// Close the open duplicate issues whose copies are no longer both there, so issues from
    /// earlier scans can't outlive the files they were about. Moves nothing.
    async fn close_stale_duplicates(&self, library: &LibraryRow) {
        // An unreachable library root says nothing about the files: leave the issues alone.
        if resolve_library_path(&library.path).await.is_err() {
            return;
        }
        let Ok(open) = self.open_duplicate_issues(&library.id).await else {
            return;
        };
        let mut closed = 0_usize;
        for issue in open {
            let Some(details) = issue
                .details_json
                .as_deref()
                .and_then(|json| serde_json::from_str::<DuplicateIssueDetails>(json).ok())
            else {
                continue;
            };
            match self.duplicate_gone(&issue, &details, false).await {
                Ok(Some(reason)) => {
                    match self.close_no_longer_duplicate(&issue.id, &reason).await {
                        Ok(()) => closed += 1,
                        Err(error) => {
                            warn!(scan_issue_id = %issue.id, error = %error, "Failed to close a stale duplicate issue");
                        }
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    debug!(scan_issue_id = %issue.id, error = %error, "Could not check a duplicate issue's copies");
                }
            }
        }
        if closed > 0 {
            info!(
                library_id = %library.id,
                closed,
                "Closed duplicate issues whose copies have gone: library_id={}, closed={}",
                library.id,
                closed
            );
        }
    }

    /// Albums whose cover lookup failed when they were created (the archive was down) ask again,
    /// a few per scan so a large library doesn't hammer the archive.
    async fn backfill_album_covers(&self, library: &LibraryRow) {
        const PER_SCAN: usize = 10;
        let Ok(database) = self.database().await else {
            return;
        };
        let missing = match crate::graphql::entities::Album::query(database.pool().pool())
            .filter(crate::graphql::entities::AlbumWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                musicbrainz_id: Some(StringFilter {
                    is_null: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
        {
            Ok(albums) => albums
                .into_iter()
                .filter(|album| album.cover_url.as_deref().is_none_or(str::is_empty))
                .take(PER_SCAN)
                .collect::<Vec<_>>(),
            Err(error) => {
                debug!(library_id = %library.id, error = %error, "Could not list albums missing covers");
                return;
            }
        };
        if missing.is_empty() {
            return;
        }
        let Ok(metadata) = self.metadata_service().await else {
            return;
        };
        let Ok(auth_user) = self.system_auth_user(Some(&library.user_id)).await else {
            return;
        };
        for album in missing {
            let Some(release_group) = album
                .musicbrainz_id
                .as_deref()
                .and_then(|id| Uuid::parse_str(id).ok())
            else {
                continue;
            };
            let cover = match metadata.album_cover(release_group).await {
                Ok(Some(cover)) => cover,
                Ok(None) => continue,
                Err(error) => {
                    debug!(album_id = %album.id, error = %error, "Cover Art Archive still unavailable");
                    continue;
                }
            };
            match self
                .execute_mutation(
                    &auth_user,
                    r#"mutation BackfillAlbumCover($id: String!, $input: UpdateAlbumInput!) {
                        Saved: updateAlbum(id: $id, input: $input) { Success: success Error: error }
                    }"#,
                    serde_json::json!({ "id": album.id, "input": { "coverUrl": cover } }),
                )
                .await
            {
                Ok(data) if data["Saved"]["Success"].as_bool() == Some(true) => info!(
                    library_id = %library.id,
                    album_id = %album.id,
                    album = %album.name,
                    "Filled in an album cover that was missing"
                ),
                Ok(data) => {
                    warn!(album_id = %album.id, error = %data["Saved"]["Error"], "Failed to save a backfilled album cover");
                }
                Err(error) => {
                    warn!(album_id = %album.id, error = %error, "Failed to save a backfilled album cover");
                }
            }
        }
    }

    /// Artists with no picture ask for one: Wikimedia Commons through their MusicBrainz id, else
    /// Deezer by exact name. A few per scan, those with an id first, so a large library doesn't
    /// hammer the sites. A picture the user set is never replaced, and an artist nothing has a
    /// picture for takes its turn at random rather than holding up the ones behind it.
    async fn backfill_artist_images(&self, library: &LibraryRow) {
        use rand::seq::SliceRandom;
        const PER_SCAN: usize = 10;
        let Ok(database) = self.database().await else {
            return;
        };
        let mut missing = match crate::graphql::entities::Artist::query(database.pool().pool())
            .filter(crate::graphql::entities::ArtistWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
        {
            Ok(artists) => artists
                .into_iter()
                .filter(|artist| artist.image_url.as_deref().is_none_or(str::is_empty))
                .collect::<Vec<_>>(),
            Err(error) => {
                debug!(library_id = %library.id, error = %error, "Could not list artists missing pictures");
                return;
            }
        };
        if missing.is_empty() {
            return;
        }
        missing.shuffle(&mut rand::rng());
        missing.sort_by_key(|artist| {
            artist
                .musicbrainz_id
                .as_deref()
                .is_none_or(|id| Uuid::parse_str(id).is_err())
        });
        missing.truncate(PER_SCAN);
        let Ok(metadata) = self.metadata_service().await else {
            return;
        };
        let Ok(auth_user) = self.system_auth_user(Some(&library.user_id)).await else {
            return;
        };
        for artist in missing {
            let musicbrainz_id = artist
                .musicbrainz_id
                .as_deref()
                .and_then(|id| Uuid::parse_str(id).ok());
            let image = match metadata.artist_image(musicbrainz_id, &artist.name).await {
                Ok(Some(image)) => image,
                Ok(None) => continue,
                Err(error) => {
                    debug!(
                        artist_id = %artist.id,
                        error = %error,
                        "Could not find a picture for artist '{}' yet: {}",
                        artist.name,
                        error
                    );
                    continue;
                }
            };
            // The lookup is slow; the user may have set a picture meanwhile.
            match crate::graphql::entities::Artist::get(database.pool().pool(), &artist.id).await {
                Ok(Some(current)) if current.image_url.as_deref().is_none_or(str::is_empty) => {}
                Ok(_) => continue,
                Err(error) => {
                    debug!(artist_id = %artist.id, error = %error, "Could not re-read artist '{}' before saving its picture", artist.name);
                    continue;
                }
            }
            match self
                .execute_mutation(
                    &auth_user,
                    r#"mutation BackfillArtistImage($id: String!, $input: UpdateArtistInput!) {
                        Saved: updateArtist(id: $id, input: $input) { Success: success Error: error }
                    }"#,
                    serde_json::json!({ "id": artist.id, "input": { "imageUrl": image } }),
                )
                .await
            {
                Ok(data) if data["Saved"]["Success"].as_bool() == Some(true) => info!(
                    library_id = %library.id,
                    artist_id = %artist.id,
                    artist = %artist.name,
                    image_url = %image,
                    "Filled in a picture for artist '{}'",
                    artist.name
                ),
                Ok(data) => {
                    warn!(artist_id = %artist.id, error = %data["Saved"]["Error"], "Failed to save a picture for artist '{}'", artist.name);
                }
                Err(error) => {
                    warn!(artist_id = %artist.id, error = %error, "Failed to save a picture for artist '{}'", artist.name);
                }
            }
        }
    }

    /// Why a duplicate pair no longer needs settling, or `None` while both copies are there.
    /// `remove_keeper` asks about the pair the other way round, where the kept copy goes and so
    /// must still be a library file.
    async fn duplicate_gone(
        &self,
        issue: &LibraryScanIssue,
        details: &DuplicateIssueDetails,
        remove_keeper: bool,
    ) -> Result<Option<String>> {
        let database = self.database().await?;
        let issue_file = match issue.media_file_id.as_deref() {
            Some(id) => MediaFile::get(database.pool().pool(), &id.to_string()).await?,
            None => None,
        };
        if issue_file.is_none() {
            return Ok(Some(
                "the duplicate copy was removed from the library".to_string(),
            ));
        }
        if !library_storage().exists(&details.duplicate_path).await? {
            return Ok(Some("the duplicate copy is no longer on disk".to_string()));
        }
        if !library_storage().exists(&details.keeper_path).await? {
            return Ok(Some(
                "the copy that was kept is no longer on disk".to_string(),
            ));
        }
        if remove_keeper
            && self
                .library_file_at(&issue.library_id, &details.keeper_path)
                .await?
                .is_none()
        {
            return Ok(Some(
                "the copy that was kept is no longer in the library".to_string(),
            ));
        }
        Ok(None)
    }

    async fn close_no_longer_duplicate(&self, issue_id: &str, reason: &str) -> Result<()> {
        self.update_scan_issue(
            issue_id,
            UpdateLibraryScanIssueInput {
                resolved_at: Some(Some(Utc::now().to_rfc3339())),
                resolution: Some(Some(format!(
                    "{NO_LONGER_DUPLICATE}: {reason}. Nothing was moved."
                ))),
                ..Default::default()
            },
        )
        .await
        .context("Failed to close the duplicate issue")?;
        Ok(())
    }

    async fn open_duplicate_issues(&self, library_id: &str) -> Result<Vec<LibraryScanIssue>> {
        let database = self.database().await?;
        let open = LibraryScanIssue::query(database.pool().pool())
            .filter(LibraryScanIssueWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library_id.to_string()),
                    ..Default::default()
                }),
                issue_code: Some(StringFilter {
                    eq: Some("BYTE_IDENTICAL_DUPLICATE".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?
            .into_iter()
            .filter(|issue| issue.resolved_at.is_none())
            .collect::<Vec<_>>();
        Ok(open)
    }

    /// The library's record of the file at a path.
    async fn library_file_at(&self, library_id: &str, path: &str) -> Result<Option<MediaFile>> {
        let database = self.database().await?;
        Ok(MediaFile::query(database.pool().pool())
            .filter(MediaFileWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library_id.to_string()),
                    ..Default::default()
                }),
                path: Some(StringFilter {
                    eq: Some(path.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?
            .into_iter()
            .next())
    }

    pub async fn trash_duplicate_issue(&self, issue_id: &str) -> Result<DuplicateTrashResult> {
        self.trash_duplicate_copy(issue_id, false).await
    }

    /// Move one of a verified duplicate pair to the library's trash: the duplicate, or with
    /// `remove_keeper` the copy the scan chose to keep.
    async fn trash_duplicate_copy(
        &self,
        issue_id: &str,
        remove_keeper: bool,
    ) -> Result<DuplicateTrashResult> {
        let database = self.database().await?;
        let issue = LibraryScanIssue::get(database.pool().pool(), &issue_id.to_string())
            .await
            .context("Failed to read duplicate issue")?
            .context("Duplicate issue not found")?;
        if issue.resolved_at.is_some() {
            anyhow::bail!("Duplicate issue is already resolved");
        }
        if issue.issue_code != "BYTE_IDENTICAL_DUPLICATE" {
            anyhow::bail!("The selected issue is not a verified duplicate");
        }
        let details: DuplicateIssueDetails = serde_json::from_str(
            issue
                .details_json
                .as_deref()
                .context("Duplicate issue has no verification context")?,
        )
        .context("Duplicate verification context is invalid")?;
        if details.kind != "byte-identical-duplicate" {
            anyhow::bail!("Duplicate verification context has an unexpected kind");
        }
        // A copy that has gone since the scan leaves nothing to settle: close the issue and move
        // nothing, as long as the library itself is reachable.
        let library = self.get_library(&issue.library_id).await?;
        resolve_library_path(&library.path)
            .await
            .context("Library root is unavailable")?;
        if let Some(reason) = self.duplicate_gone(&issue, &details, remove_keeper).await? {
            self.close_no_longer_duplicate(issue_id, &reason).await?;
            info!(
                scan_issue_id = %issue_id,
                library_id = %issue.library_id,
                reason = %reason,
                "Closed a duplicate issue whose copies are no longer both there"
            );
            return Ok(DuplicateTrashResult {
                old_path: details.duplicate_path,
                trash_path: None,
                no_longer_duplicate: Some(reason),
            });
        }
        let issue_file_id = issue
            .media_file_id
            .as_deref()
            .context("Duplicate issue is not linked to a media file")?;
        let issue_file = MediaFile::get(database.pool().pool(), &issue_file_id.to_string())
            .await
            .context("Failed to read duplicate media file")?
            .context("Duplicate media file no longer exists")?;
        if issue_file.library_id.as_deref() != Some(issue.library_id.as_str())
            || issue_file.path != details.duplicate_path
            || issue_file.size != details.size
        {
            anyhow::bail!("Duplicate file changed since it was verified; rescan before removal");
        }
        // Removing the keeper swaps the two: the issue's own file stays.
        let details = if remove_keeper {
            DuplicateIssueDetails {
                duplicate_path: details.keeper_path.clone(),
                keeper_path: details.duplicate_path.clone(),
                ..details
            }
        } else {
            details
        };
        let media_file = if remove_keeper {
            self.library_file_at(&issue.library_id, &details.duplicate_path)
                .await?
                .context("The kept copy is no longer in the library; rescan before removal")?
        } else {
            issue_file
        };
        if media_file.size != details.size {
            anyhow::bail!("Duplicate file changed since it was verified; rescan before removal");
        }
        let media_file_id = media_file.id.as_str();
        let root = resolve_library_path(&library.path)
            .await
            .context("Library root is unavailable")?;
        let duplicate_path = resolve_library_path(&details.duplicate_path)
            .await
            .context("Duplicate path is unavailable")?;
        let keeper_path = resolve_library_path(&details.keeper_path)
            .await
            .context("Keeper path is unavailable")?;
        if !duplicate_path.starts_with(&root) || !keeper_path.starts_with(&root) {
            anyhow::bail!("Duplicate paths must remain inside the library root");
        }

        let duplicate_hash = Self::hash_duplicate_candidate(
            DuplicateCandidate {
                media_file_id: media_file_id.to_string(),
                path: duplicate_path.to_string_lossy().to_string(),
                size: details.size,
                analysis_state: "UNKNOWN",
                quality_status: None,
            },
            CancellationToken::new(),
        )
        .await?
        .1;
        let keeper_hash = Self::hash_duplicate_candidate(
            DuplicateCandidate {
                media_file_id: media_file_id.to_string(),
                path: keeper_path.to_string_lossy().to_string(),
                size: details.size,
                analysis_state: "UNKNOWN",
                quality_status: None,
            },
            CancellationToken::new(),
        )
        .await?
        .1;
        if duplicate_hash != details.sha256
            || keeper_hash != details.sha256
            || duplicate_hash != keeper_hash
        {
            anyhow::bail!("Duplicate content changed since it was verified; rescan before removal");
        }

        let trash_dir = create_library_directory_in(&root, &root.join(LIBRARY_TRASH_DIR))
            .await
            .context("Failed to create the contained library trash directory")?;
        let original_name = duplicate_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("media");
        let trash_path = trash_dir.join(format!("{}-{original_name}", Uuid::new_v4()));
        checked_library_path(&root, &trash_path)?;
        library_storage()
            .rename(&duplicate_path, &trash_path, PlaceMode::Replace)
            .await
            .context("Failed to move duplicate to recoverable trash")?;

        let auth_user = self.system_auth_user(Some(&library.user_id)).await?;
        let delete_result = self
            .execute_mutation(
                &auth_user,
                r#"mutation TrashDuplicateMediaFile($id: String!) {
                    DeleteMediaFile: deleteMediaFile(id: $id) { Success: success Error: error }
                }"#,
                serde_json::json!({ "id": media_file_id }),
            )
            .await;
        let deleted = delete_result
            .as_ref()
            .ok()
            .and_then(|data| data.get("DeleteMediaFile"))
            .and_then(|result| result.get("Success"))
            .and_then(|success| success.as_bool())
            .unwrap_or(false);
        if !deleted {
            if let Err(restore_error) = library_storage()
                .rename(&trash_path, &duplicate_path, PlaceMode::Replace)
                .await
            {
                error!(
                    library_id = %library.id,
                    media_file_id,
                    original_path = %duplicate_path.display(),
                    trash_path = %trash_path.display(),
                    error = %restore_error,
                    "Duplicate entity deletion failed and filesystem compensation also failed"
                );
                anyhow::bail!(
                    "The database update failed and the file could not be restored from trash; inspect correlated server logs"
                );
            }
            let reason = delete_result
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "entity deletion was rejected".to_string());
            anyhow::bail!("Duplicate was restored because {reason}");
        }

        let old_path = duplicate_path.to_string_lossy().to_string();
        let trash_path_string = trash_path.to_string_lossy().to_string();
        self.update_scan_issue(
            issue_id,
            UpdateLibraryScanIssueInput {
                resolved_at: Some(Some(Utc::now().to_rfc3339())),
                resolution: Some(Some(format!(
                    "Moved duplicate to recoverable trash at '{trash_path_string}'."
                ))),
                ..Default::default()
            },
        )
        .await
        .context("Failed to resolve scan issue")?;
        info!(
            user_id = %library.user_id,
            library_id = %library.id,
            media_file_id,
            old_path,
            trash_path = %trash_path_string,
            "Moved user-confirmed byte-identical duplicate to recoverable library trash"
        );
        Ok(DuplicateTrashResult {
            old_path,
            trash_path: Some(trash_path_string),
            no_longer_duplicate: None,
        })
    }

    async fn database(&self) -> Result<Arc<crate::services::DatabaseService>> {
        self.manager
            .get_database()
            .await
            .ok_or_else(|| anyhow::anyhow!("Database service not available"))
    }

    /// Narrow trusted scan action boundary. Ownership comes from a persisted library,
    /// never from worker authority; all related writes stay on one generated context.
    async fn owned_scan_write<T, F>(
        &self,
        library_id: String,
        owner_id: String,
        operation: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: for<'tx> FnOnce(
                &'tx mut graphql_orm::graphql::orm::MutationContext<'_>,
            ) -> futures::future::BoxFuture<
                'tx,
                std::result::Result<T, OrmPublicError>,
            > + Send
            + 'static,
    {
        Uuid::parse_str(&owner_id).context("Scan record ownership is unknown")?;
        let database = self.database().await?;
        let authority = DbAuthContext {
            user_id: Some(owner_id.clone()),
            subject: Some("system".to_string()),
            actor_id: Some("system".to_string()),
            roles: vec!["admin".to_string()],
            ..Default::default()
        };
        database
            .pool()
            .transaction_with_auth(TransactionMode::StateMachine, Some(&authority), move |tx| {
                Box::pin(async move {
                    let library = tx
                        .find_by_id::<crate::graphql::entities::Library>(&library_id)
                        .await?
                        .ok_or_else(OrmPublicError::forbidden)?;
                    if library.user_id != owner_id
                        || tx.find_by_id::<User>(&owner_id).await?.is_none()
                    {
                        return Err(OrmPublicError::forbidden());
                    }
                    operation(tx).await
                })
            })
            .await
            .context("Scoped scan entity transaction failed")
    }

    async fn update_scan_issue(
        &self,
        id: &str,
        input: UpdateLibraryScanIssueInput,
    ) -> Result<Option<LibraryScanIssue>> {
        let database = self.database().await?;
        let Some(issue) = LibraryScanIssue::get(database.pool().pool(), &id.to_string()).await?
        else {
            return Ok(None);
        };
        let library_id = issue.library_id.clone();
        let owner_id = issue.user_id.clone();
        self.owned_scan_write(library_id.clone(), owner_id.clone(), move |tx| {
            Box::pin(async move {
                let Some(current) = tx.find_by_id::<LibraryScanIssue>(&issue.id).await? else {
                    return Ok(None);
                };
                if current.library_id != library_id
                    || current.user_id != owner_id
                    || input
                        .library_id
                        .as_ref()
                        .is_some_and(|id| id != &library_id)
                    || input.user_id.as_ref().is_some_and(|id| id != &owner_id)
                {
                    return Err(OrmPublicError::forbidden());
                }
                tx.update_by_id::<LibraryScanIssue>(&issue.id, input)
                    .await
                    .map_err(Into::into)
            })
        })
        .await
    }

    async fn delete_scan_issue(&self, id: &str) -> Result<bool> {
        let database = self.database().await?;
        let Some(issue) = LibraryScanIssue::get(database.pool().pool(), &id.to_string()).await?
        else {
            return Ok(false);
        };
        let library_id = issue.library_id.clone();
        let owner_id = issue.user_id.clone();
        self.owned_scan_write(library_id.clone(), owner_id.clone(), move |tx| {
            Box::pin(async move {
                let Some(current) = tx.find_by_id::<LibraryScanIssue>(&issue.id).await? else {
                    return Ok(false);
                };
                if current.library_id != library_id || current.user_id != owner_id {
                    return Err(OrmPublicError::forbidden());
                }
                tx.delete_by_id::<LibraryScanIssue>(&issue.id)
                    .await
                    .map_err(Into::into)
            })
        })
        .await
    }

    async fn delete_scan_run(&self, id: &str) -> Result<bool> {
        let database = self.database().await?;
        let Some(run) = LibraryScanRun::get(database.pool().pool(), &id.to_string()).await? else {
            return Ok(false);
        };
        let library_id = run.library_id.clone();
        let owner_id = run.user_id.clone();
        self.owned_scan_write(library_id.clone(), owner_id.clone(), move |tx| {
            Box::pin(async move {
                let Some(current) = tx.find_by_id::<LibraryScanRun>(&run.id).await? else {
                    return Ok(false);
                };
                if current.library_id != library_id || current.user_id != owner_id {
                    return Err(OrmPublicError::forbidden());
                }
                tx.delete_by_id::<LibraryScanRun>(&run.id)
                    .await
                    .map_err(Into::into)
            })
        })
        .await
    }

    async fn create_scan_run(
        &self,
        library: &LibraryRow,
        scope: &ScanScope,
    ) -> Result<LibraryScanRun> {
        let input = CreateLibraryScanRunInput {
            user_id: library.user_id.clone(),
            library_id: library.id.clone(),
            status: "QUEUED".to_string(),
            current_stage: "QUEUE".to_string(),
            started_at: None,
            finished_at: None,
            discovered_count: 0,
            existing_count: 0,
            matched_count: 0,
            unmatched_count: 0,
            provider_blocked_count: 0,
            analysis_queued_count: 0,
            analysis_succeeded_count: 0,
            analysis_failed_count: 0,
            organization_succeeded_count: 0,
            organization_failed_count: 0,
            missing_count: 0,
            reconciled_count: 0,
            summary: Some(match scope {
                ScanScope::Full => format!("Scan queued for {}", library.name),
                ScanScope::Changes(targets) => format!(
                    "Changes queued for {}: {}",
                    library.name,
                    describe_change_targets(&library.path, targets)
                ),
            }),
            error_code: None,
        };
        self.owned_scan_write(library.id.clone(), library.user_id.clone(), move |tx| {
            Box::pin(async move { tx.insert::<LibraryScanRun>(input).await.map_err(Into::into) })
        })
        .await
        .context("Failed to create durable library scan run")
    }

    async fn update_scan_run(
        &self,
        scan_run_id: &str,
        input: UpdateLibraryScanRunInput,
    ) -> Result<LibraryScanRun> {
        let database = self.database().await?;
        let run = LibraryScanRun::get(database.pool().pool(), &scan_run_id.to_string())
            .await?
            .context("Library scan run no longer exists")?;
        let library_id = run.library_id.clone();
        let owner_id = run.user_id.clone();
        self.owned_scan_write(library_id.clone(), owner_id.clone(), move |tx| {
            Box::pin(async move {
                let current = tx
                    .find_by_id::<LibraryScanRun>(&run.id)
                    .await?
                    .ok_or_else(OrmPublicError::forbidden)?;
                if current.library_id != library_id
                    || current.user_id != owner_id
                    || input
                        .library_id
                        .as_ref()
                        .is_some_and(|id| id != &library_id)
                    || input.user_id.as_ref().is_some_and(|id| id != &owner_id)
                {
                    return Err(OrmPublicError::forbidden());
                }
                tx.update_by_id::<LibraryScanRun>(&run.id, input)
                    .await?
                    .ok_or_else(|| {
                        OrmPublicError::with_message(OrmErrorCode::Conflict, "Scan run disappeared")
                    })
            })
        })
        .await
    }

    fn bounded_issue_text(input: &str, max_chars: usize) -> String {
        input.chars().take(max_chars).collect()
    }

    #[allow(clippy::too_many_arguments)]
    async fn record_scan_issue(
        &self,
        scan_run_id: &str,
        library: &LibraryRow,
        media_file_id: Option<&str>,
        stage: &str,
        issue_code: &str,
        severity: &str,
        message: &str,
        remediation: Option<&str>,
    ) -> Result<()> {
        self.record_scan_issue_with_details(
            scan_run_id,
            library,
            media_file_id,
            stage,
            issue_code,
            severity,
            message,
            remediation,
            None,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn record_scan_issue_with_details(
        &self,
        scan_run_id: &str,
        library: &LibraryRow,
        media_file_id: Option<&str>,
        stage: &str,
        issue_code: &str,
        severity: &str,
        message: &str,
        remediation: Option<&str>,
        details_json: Option<String>,
    ) -> Result<()> {
        let _guard = self.scan_run_update_lock.lock().await;
        let database = self.database().await?;
        let existing = LibraryScanIssue::query(database.pool().pool())
            .filter(LibraryScanIssueWhereInput {
                scan_run_id: Some(StringFilter {
                    eq: Some(scan_run_id.to_string()),
                    ..Default::default()
                }),
                issue_code: Some(StringFilter {
                    eq: Some(issue_code.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
            .context("Failed to query scan issues for aggregation")?
            .into_iter()
            .find(|issue| issue.media_file_id.as_deref() == media_file_id);

        if let Some(existing) = existing {
            self.update_scan_issue(
                &existing.id,
                UpdateLibraryScanIssueInput {
                    occurrence_count: Some(existing.occurrence_count.saturating_add(1)),
                    details_json: details_json
                        .map(|value| Some(Self::bounded_issue_text(&value, 8_000))),
                    ..Default::default()
                },
            )
            .await
            .context("Failed to aggregate scan issue")?;
            return Ok(());
        }

        // A later scan finding the same problem is the same issue, seen once more: it moves to
        // this scan and counts up, rather than becoming another row to deal with. Copies left by
        // older scans fold into it.
        let earlier: Vec<LibraryScanIssue> = LibraryScanIssue::query(database.pool().pool())
            .filter(LibraryScanIssueWhereInput {
                user_id: Some(StringFilter {
                    eq: Some(library.user_id.clone()),
                    ..Default::default()
                }),
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                issue_code: Some(StringFilter {
                    eq: Some(issue_code.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?
            .into_iter()
            .filter(|issue| issue.media_file_id.as_deref() == media_file_id)
            .collect();
        let mut open: Vec<&LibraryScanIssue> = earlier
            .iter()
            .filter(|issue| issue.resolved_at.is_none())
            .collect();
        open.sort_by_key(|issue| std::cmp::Reverse(issue_seen_at(issue)));
        if let Some((current, older)) = open.split_first() {
            let folded: i32 = older.iter().map(|issue| issue.occurrence_count).sum();
            self.update_scan_issue(
                &current.id,
                UpdateLibraryScanIssueInput {
                    scan_run_id: Some(scan_run_id.to_string()),
                    severity: Some(severity.to_string()),
                    message: Some(Self::bounded_issue_text(message, 1_000)),
                    remediation: Some(
                        remediation.map(|value| Self::bounded_issue_text(value, 1_000)),
                    ),
                    details_json: Some(
                        details_json.map(|value| Self::bounded_issue_text(&value, 8_000)),
                    ),
                    occurrence_count: Some(
                        current
                            .occurrence_count
                            .saturating_add(folded)
                            .saturating_add(1),
                    ),
                    ..Default::default()
                },
            )
            .await
            .context("Failed to carry a scan issue forward")?;
            for issue in older {
                self.delete_scan_issue(&issue.id).await?;
            }
            return Ok(());
        }
        // Someone settled this already, as it still is: finding it again doesn't reopen it. An
        // issue that closed only because a copy had gone isn't a decision, so it doesn't count.
        let bounded_message = Self::bounded_issue_text(message, 1_000);
        if earlier.iter().any(|issue| {
            issue.resolved_at.is_some()
                && issue.message == bounded_message
                && !issue
                    .resolution
                    .as_deref()
                    .is_some_and(|resolution| resolution.starts_with(NO_LONGER_DUPLICATE))
        }) {
            return Ok(());
        }
        let read_at = None;

        let input = CreateLibraryScanIssueInput {
            user_id: library.user_id.clone(),
            scan_run_id: scan_run_id.to_string(),
            library_id: library.id.clone(),
            media_file_id: media_file_id.map(str::to_string),
            stage: stage.to_string(),
            issue_code: issue_code.to_string(),
            severity: severity.to_string(),
            message: Self::bounded_issue_text(message, 1_000),
            remediation: remediation.map(|value| Self::bounded_issue_text(value, 1_000)),
            details_json: details_json.map(|value| Self::bounded_issue_text(&value, 8_000)),
            occurrence_count: 1,
            read_at,
            resolved_at: None,
            resolution: None,
        };
        self.owned_scan_write(library.id.clone(), library.user_id.clone(), move |tx| {
            Box::pin(async move {
                tx.insert::<LibraryScanIssue>(input)
                    .await
                    .map_err(Into::into)
            })
        })
        .await
        .context("Failed to persist scan issue")?;
        Ok(())
    }

    async fn record_analysis_outcome(&self, scan_run_id: &str, succeeded: bool) -> Result<()> {
        let _guard = self.scan_run_update_lock.lock().await;
        let database = self.database().await?;
        let Some(run) = LibraryScanRun::get(database.pool().pool(), &scan_run_id.to_string())
            .await
            .context("Failed to read scan run for analysis outcome")?
        else {
            return Ok(());
        };
        let input = if succeeded {
            UpdateLibraryScanRunInput {
                analysis_succeeded_count: Some(run.analysis_succeeded_count.saturating_add(1)),
                ..Default::default()
            }
        } else {
            UpdateLibraryScanRunInput {
                analysis_failed_count: Some(run.analysis_failed_count.saturating_add(1)),
                ..Default::default()
            }
        };
        self.update_scan_run(&run.id, input)
            .await
            .context("Failed to persist scan analysis outcome")?;
        Ok(())
    }

    async fn media_file_has_been_analyzed(&self, media_file_id: &str) -> Result<bool> {
        let auth_user = self.system_auth_user(None).await?;
        let data = self
            .execute_graphql(
                &auth_user,
                r#"query MediaFileAnalyzeState($id: String!) {
                    MediaFile: mediaFile(id: $id) {
                        AnalyzedAt: analyzedAt
                    }
                }"#,
                serde_json::json!({ "id": media_file_id }),
            )
            .await?;

        Ok(data
            .get("MediaFile")
            .and_then(|v| v.get("AnalyzedAt"))
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false))
    }

    async fn scan_library_inner(
        self: &Arc<Self>,
        library_id: &str,
        scan_run_id: &str,
        scope: &ScanScope,
        cancel: CancellationToken,
    ) -> Result<()> {
        let scan_started = Instant::now();
        let library = self.get_library(library_id).await?;
        let normalized_library_type = Self::normalize_library_type(&library.library_type);
        // A library being moved is not scanned: its records and its folder disagree until done.
        self.ensure_library_not_moving(library_id).await?;

        if library.scanning {
            debug!(
                library_id = %library_id,
                library_name = %library.name,
                "Skipping scan because library is already marked scanning: library_id={}, library_name={}",
                library_id,
                library.name
            );
            return Ok(());
        }

        let auth_user = self.system_auth_user(Some(&library.user_id)).await?;
        self.update_scan_run(
            scan_run_id,
            UpdateLibraryScanRunInput {
                status: Some("RUNNING".to_string()),
                current_stage: Some("DISCOVERY".to_string()),
                started_at: Some(Some(Utc::now().to_rfc3339())),
                summary: Some(Some(match scope {
                    ScanScope::Full => format!("Scanning {}", library.name),
                    ScanScope::Changes(targets) => format!(
                        "Checking changes in {}: {}",
                        library.name,
                        describe_change_targets(&library.path, targets)
                    ),
                })),
                ..Default::default()
            },
        )
        .await?;

        self.set_library_scanning(&auth_user, library_id, true, false)
            .await?;
        info!(
            library_id = %library_id,
            library_name = %library.name,
            library_type = %library.library_type,
            normalized_library_type = %normalized_library_type,
            library_path = %library.path,
            auto_organize = library.auto_organize,
            scan_interval_minutes = library.scan_interval_minutes,
            full = scope.is_full(),
            "Starting library scan: library_id={}, name={}, type={}, path={}, auto_organize={}, scan_interval_minutes={}, scope={}",
            library_id,
            library.name,
            library.library_type,
            library.path,
            library.auto_organize,
            library.scan_interval_minutes,
            match scope {
                ScanScope::Full => "everything".to_string(),
                ScanScope::Changes(targets) => describe_change_targets(&library.path, targets),
            }
        );

        let root = PathBuf::from(&library.path);
        let remote_library = is_smb_uri(&library.path);
        if remote_library {
            // A library on a share: say why it cannot be read, and touch nothing in the catalogue.
            if let Err(error) = library_storage().stat(&library.path).await {
                let unavailable = error.kind() != StorageErrorKind::NotFound;
                warn!(
                    library_id = %library_id,
                    library_name = %library.name,
                    path = %library.path,
                    error = %error,
                    "Skipping library scan because its share cannot be read: library_id={}, name={}, path={}, error={}",
                    library_id,
                    library.name,
                    library.path,
                    error
                );
                self.record_scan_issue(
                    scan_run_id,
                    &library,
                    None,
                    "DISCOVERY",
                    if unavailable {
                        "LIBRARY_STORAGE_UNAVAILABLE"
                    } else {
                        "LIBRARY_ROOT_UNAVAILABLE"
                    },
                    "ERROR",
                    &format!("The library's network share cannot be read: {error}"),
                    Some(
                        "No files were removed or marked missing. Check that the file server is reachable and the saved sign-in still works, then retry the scan.",
                    ),
                )
                .await?;
                anyhow::bail!("Library storage is unavailable");
            }
        } else if !root.exists() {
            warn!(
                library_id = %library_id,
                library_name = %library.name,
                path = %library.path,
                "Skipping library scan because path does not exist: library_id={}, name={}, path={}",
                library_id,
                library.name,
                library.path
            );
            self.record_scan_issue(
                scan_run_id,
                &library,
                None,
                "DISCOVERY",
                "LIBRARY_ROOT_UNAVAILABLE",
                "ERROR",
                "The configured library root is unavailable.",
                Some("Restore or correct the library path, then retry the scan."),
            )
            .await?;
            anyhow::bail!("Library root is unavailable");
        }

        let allowed_ext = Self::walk_extensions_for_library(&normalized_library_type);
        let wants_sidecars = !matches!(normalized_library_type.as_str(), "music" | "audiobooks");
        let mut sidecar_paths: Vec<String> = Vec::new();
        let mut scanned_videos: Vec<(String, String)> = Vec::new();
        let mut scanned_count: usize = 0;
        let mut match_pipeline_error_count: usize = 0;
        let mut existing_count: usize = 0;
        let mut matched_count: usize = 0;
        let mut analysis_queued_count: usize = 0;
        let mut unchanged_skipped_count: usize = 0;
        let mut run_analysis_ids: Vec<String> = Vec::new();
        let mut discovered_paths: HashSet<String> = HashSet::new();
        let mut discovery_errors: usize = 0;
        let mut storage_unavailable = false;
        let mut pending_provider_fallback: Vec<PendingProviderFallback> = Vec::new();
        let mut duplicate_candidates: Vec<DuplicateCandidate> = Vec::new();
        let mut organize_candidates: Vec<MatchedFile> = Vec::new();
        let mut media_files_by_path = self.load_media_files_by_path(library_id).await?;
        let reconciliation_rows = media_files_by_path
            .values()
            .filter(|file| scope.covers(&file.path))
            .map(|file| ExistingMediaFileRow {
                id: file.id.clone(),
                path: file.path.clone(),
            })
            .collect::<Vec<_>>();

        let (mut discovered_rx, discovery_handle) = match scope {
            ScanScope::Full => {
                Self::discover_media_files(root.clone(), allowed_ext, cancel.child_token())
            }
            ScanScope::Changes(targets) => Self::discover_changed_media_files(
                root.clone(),
                targets.clone(),
                allowed_ext,
                cancel.child_token(),
            ),
        };
        let mut discovery_handle = Some(discovery_handle);
        loop {
            let discovered = tokio::select! {
                _ = cancel.cancelled() => {
                    drop(discovered_rx);
                    if let Some(handle) = discovery_handle.take() {
                        let _ = handle.await;
                    }
                    anyhow::bail!("SCAN_CANCELLED");
                }
                discovered = discovered_rx.recv() => discovered,
            };
            let Some(discovered) = discovered else {
                break;
            };
            let discovered = match discovered {
                Ok(discovered) => discovered,
                Err(failure) => {
                    let message = failure.message;
                    discovery_errors += 1;
                    warn!(
                        library_id = %library_id,
                        library_name = %library.name,
                        library_path = %library.path,
                        error = %message,
                        "Library traversal could not read an entry"
                    );
                    if failure.unavailable {
                        storage_unavailable = true;
                        self.record_scan_issue(
                            scan_run_id,
                            &library,
                            None,
                            "DISCOVERY",
                            "LIBRARY_STORAGE_UNAVAILABLE",
                            "ERROR",
                            &message,
                            Some(
                                "The share stopped answering during the scan. No files were removed or marked missing; retry the scan when it is back.",
                            ),
                        )
                        .await?;
                    } else {
                        self.record_scan_issue(
                            scan_run_id,
                            &library,
                            None,
                            "DISCOVERY",
                            "FILE_DISCOVERY_ERROR",
                            "WARNING",
                            &message,
                            Some("Check library filesystem permissions and retry the scan."),
                        )
                        .await?;
                    }
                    continue;
                }
            };

            let abs_path = discovered.absolute_path;
            if wants_sidecars
                && crate::services::subtitles::sidecar::is_sidecar_extension(&discovered.extension)
            {
                sidecar_paths.push(abs_path);
                continue;
            }
            discovered_paths.insert(abs_path.clone());
            let rel_path = discovered.relative_path;
            let match_source = rel_path.clone().unwrap_or_else(|| {
                Path::new(&abs_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&abs_path)
                    .to_string()
            });
            let size = discovered.size;
            let modified_at = discovered.modified_at;

            let media_file = media_files_by_path.remove(&abs_path);
            let mut matched_link: Option<MatchedFile> = None;
            let (
                media_file_id,
                is_new_media_file,
                was_unmatched_media_file,
                analysis_state,
                quality_status,
                version_changed,
            ) = if let Some(existing) = media_file {
                existing_count += 1;
                let is_unmatched = existing.movie_id.is_none()
                    && existing.episode_id.is_none()
                    && existing.track_id.is_none()
                    && existing.chapter_id.is_none();
                let version_changed = Self::media_file_version_changed(
                    existing.size,
                    existing.file_modified_at.as_deref(),
                    size,
                    modified_at.as_deref(),
                );
                self.ensure_original_name(
                    &auth_user,
                    &existing.id,
                    existing.original_name.as_deref(),
                    &abs_path,
                )
                .await?;
                if version_changed {
                    self.invalidate_changed_media_file(
                        &auth_user,
                        &existing.id,
                        size,
                        modified_at.as_deref(),
                    )
                    .await?;
                }
                matched_link = MatchedFile::of(&existing);
                let analysis_state = if !version_changed && existing.analyzed_at.is_some() {
                    "ANALYZED"
                } else {
                    "NOT_ANALYZED"
                };
                let quality_status = if version_changed {
                    None
                } else {
                    existing.quality_status
                };
                (
                    existing.id,
                    false,
                    is_unmatched,
                    analysis_state,
                    quality_status,
                    version_changed,
                )
            } else {
                (
                    self.create_media_file(
                        &auth_user,
                        library_id,
                        ScanMediaFileCreate {
                            path: &abs_path,
                            relative_path: rel_path.as_deref(),
                            size,
                            modified_at: modified_at.as_deref(),
                            content_type: Self::content_type_for_ext(
                                &discovered.extension,
                                &normalized_library_type,
                            ),
                        },
                    )
                    .await?,
                    true,
                    true,
                    "NOT_ANALYZED",
                    None,
                    false,
                )
            };

            if wants_sidecars {
                scanned_videos.push((abs_path.clone(), media_file_id.clone()));
            }
            duplicate_candidates.push(DuplicateCandidate {
                media_file_id: media_file_id.clone(),
                path: abs_path.clone(),
                size,
                analysis_state,
                quality_status,
            });

            if is_new_media_file {
                debug!(
                    library_id = %library_id,
                    library_name = %library.name,
                    media_file_id = %media_file_id,
                    media_file_path = %abs_path,
                    "Discovered new media file during scan: library_id={}, media_file_id={}, path={}",
                    library_id,
                    media_file_id,
                    abs_path
                );
            } else {
                debug!(
                    library_id = %library_id,
                    library_name = %library.name,
                    media_file_id = %media_file_id,
                    media_file_path = %abs_path,
                    unmatched = was_unmatched_media_file,
                    version_changed,
                    "Re-evaluating previously known media file during scan: library_id={}, media_file_id={}, path={}, unmatched={}",
                    library_id,
                    media_file_id,
                    abs_path,
                    was_unmatched_media_file
                );
            }

            // Queue ffprobe analysis during scans as well.
            // queue_analyze_job is idempotent and will skip when:
            // - ffprobe is unavailable
            // - file is already analyzed
            // - file is already queued
            match self
                .queue_analyze_job_for_run(&media_file_id, &abs_path, Some(scan_run_id))
                .await
            {
                Ok(true) => {
                    analysis_queued_count += 1;
                    run_analysis_ids.push(media_file_id.clone());
                }
                Ok(false) => {}
                Err(e) => {
                    warn!(
                        library_id = %library_id,
                        library_name = %library.name,
                        media_file_id = %media_file_id,
                        media_file_path = %abs_path,
                        error = %e,
                        "Failed to queue analyze job during scan"
                    );
                    self.record_scan_issue(
                        scan_run_id,
                        &library,
                        Some(&media_file_id),
                        "ANALYSIS",
                        "ANALYSIS_QUEUE_FAILED",
                        "WARNING",
                        "Media analysis could not be queued.",
                        Some("Retry analysis for this file from the scan issue list."),
                    )
                    .await?;
                }
            }

            if !is_new_media_file && !version_changed && !was_unmatched_media_file {
                unchanged_skipped_count += 1;
                matched_count += 1;
                scanned_count += 1;
                // Matching is settled, but whether the file is where the library's naming pattern
                // puts it is not: it may have been matched while organizing was off, or the
                // pattern or the title may have changed since. Checked together after the walk.
                if library.auto_organize
                    && let Some(link) = matched_link
                {
                    organize_candidates.push(link);
                }
                debug!(
                    library_id = %library_id,
                    media_file_id = %media_file_id,
                    media_file_path = %abs_path,
                    "Skipped match/provider work for an unchanged, already-linked media file"
                );
                continue;
            }

            // Music is worked out a release at a time, once every file has been seen. A track's name
            // alone could match any album in the library.
            if Self::normalize_library_type(&library.library_type) == "music"
                && was_unmatched_media_file
            {
                pending_provider_fallback.push(PendingProviderFallback {
                    media_file_id: media_file_id.clone(),
                    media_path: abs_path.clone(),
                    match_source: match_source.clone(),
                });
                scanned_count += 1;
                continue;
            }

            let match_started = Instant::now();
            let match_result = self
                .match_media_file(MatchRequest {
                    media_file_id: media_file_id.clone(),
                    library_id: Some(library_id.to_string()),
                    methods: vec![MatchMethod::Filename, MatchMethod::Metadata],
                    force: false,
                    auto_match: true,
                    candidate_limit: 10,
                    allow_provider_fallback: false,
                    wanted_policy: MatchWantedPolicy::PreferWanted,
                    ..Default::default()
                })
                .await;

            match match_result {
                Ok(m) => {
                    if m.success {
                        matched_count += 1;
                    }
                    self.maybe_auto_organize_after_scan_match(
                        &library,
                        &auth_user,
                        &media_file_id,
                        &abs_path,
                        &m,
                    )
                    .await;
                    if !m.success && was_unmatched_media_file {
                        pending_provider_fallback.push(PendingProviderFallback {
                            media_file_id: media_file_id.clone(),
                            media_path: abs_path.clone(),
                            match_source: match_source.clone(),
                        });
                    }
                    debug!(
                        library_id = %library_id,
                        media_file_id = %media_file_id,
                        media_file_path = %abs_path,
                        matched = m.success,
                        elapsed_ms = match_started.elapsed().as_millis() as u64,
                        "Completed scan match evaluation for media file: library_id={}, media_file_id={}, path={}, matched={}, elapsed_ms={}",
                        library_id,
                        media_file_id,
                        abs_path,
                        m.success,
                        match_started.elapsed().as_millis() as u64
                    );
                }
                Err(e) => {
                    match_pipeline_error_count += 1;
                    warn!(
                        library_id = %library_id,
                        library_name = %library.name,
                        media_file_id = %media_file_id,
                        media_path = %abs_path,
                        error = %e,
                        "Match pipeline failed for scanned media file: library_id={}, library_name={}, media_file_id={}, path={}, error={}",
                        library_id,
                        library.name,
                        media_file_id,
                        abs_path,
                        e
                    );
                    self.record_scan_issue(
                        scan_run_id,
                        &library,
                        Some(&media_file_id),
                        "MATCH",
                        "MATCH_PIPELINE_FAILED",
                        "WARNING",
                        "The file could not be evaluated by the matching pipeline.",
                        Some(
                            "Review the filename and provider configuration, then retry matching.",
                        ),
                    )
                    .await?;
                }
            }

            scanned_count += 1;
        }
        if let Some(handle) = discovery_handle.take() {
            handle.await.context("Library discovery worker failed")?;
        }
        // Only a complete walk says a sidecar is gone.
        if wants_sidecars && !storage_unavailable && discovery_errors == 0 {
            self.sync_scan_sidecars(&library, &scanned_videos, &sidecar_paths)
                .await;
        }
        let duplicate_count = self
            .detect_duplicate_files(
                scan_run_id,
                &library,
                duplicate_candidates,
                cancel.child_token(),
            )
            .await?;

        self.update_scan_run(
            scan_run_id,
            UpdateLibraryScanRunInput {
                current_stage: Some("PROVIDER_MATCH".to_string()),
                discovered_count: Some(Self::usize_count(scanned_count)),
                existing_count: Some(Self::usize_count(existing_count)),
                matched_count: Some(Self::usize_count(matched_count)),
                analysis_queued_count: Some(Self::usize_count(analysis_queued_count)),
                ..Default::default()
            },
        )
        .await?;

        let provider_summary = self
            .process_provider_fallback_batch(&library, &auth_user, pending_provider_fallback)
            .await;
        matched_count = matched_count.saturating_add(provider_summary.matched_files);
        if provider_summary.configuration_blocked {
            self.record_scan_issue(
                scan_run_id,
                &library,
                None,
                "PROVIDER_MATCH",
                "TMDB_NOT_CONFIGURED",
                "WARNING",
                "Movie matching is blocked because TMDB is not configured.",
                Some("Add and test a TMDB API key in Settings > Metadata, then rescan."),
            )
            .await?;
        } else {
            if normalized_library_type == "movies" {
                self.resolve_tmdb_configuration_feedback(&auth_user, &library)
                    .await;
            }
            if provider_summary.error_files > 0 {
                self.record_scan_issue(
                    scan_run_id,
                    &library,
                    None,
                    "PROVIDER_MATCH",
                    "PROVIDER_REQUEST_FAILED",
                    "WARNING",
                    "One or more provider matching requests failed.",
                    Some("Check provider health and retry the failed scan."),
                )
                .await?;
            }
        }

        self.update_scan_run(
            scan_run_id,
            UpdateLibraryScanRunInput {
                current_stage: Some("RECONCILIATION".to_string()),
                matched_count: Some(Self::usize_count(matched_count)),
                unmatched_count: Some(Self::usize_count(provider_summary.unmatched_files)),
                provider_blocked_count: Some(if provider_summary.configuration_blocked {
                    Self::usize_count(provider_summary.pending_files)
                } else {
                    0
                }),
                ..Default::default()
            },
        )
        .await?;
        // A catalogue is only trimmed to what a complete walk found. A share that stopped
        // answering, or answered with nothing for a library that had files, says nothing about
        // them: keep every record.
        // A scan of changed paths finding nothing under a removed folder is the point of it, so
        // only a whole-library walk counts "found nothing" as a failed walk.
        let keep_every_record = storage_unavailable
            || (remote_library
                && (discovery_errors > 0
                    || (scope.is_full()
                        && discovered_paths.is_empty()
                        && !reconciliation_rows.is_empty())));
        let removed_count = if keep_every_record {
            if !storage_unavailable {
                warn!(
                    library_id = %library_id,
                    library_name = %library.name,
                    discovery_errors,
                    catalogued_files = reconciliation_rows.len(),
                    "Kept every file record of '{}': the walk of its share was incomplete or found nothing",
                    library.name
                );
            }
            info!(
                library_id = %library_id,
                library_name = %library.name,
                removed = 0,
                "Removed 0 records of missing files from '{}': the library could not be read completely",
                library.name
            );
            0
        } else {
            let summary = self
                .reconcile_missing_media_files(
                    &auth_user,
                    &library,
                    &discovered_paths,
                    &reconciliation_rows,
                )
                .await?;
            info!(
                library_id = %library_id,
                library_name = %library.name,
                removed = summary.removed(),
                in_library = summary.removed_in_library,
                in_trash = summary.removed_in_trash,
                outside_root = summary.removed_outside_root,
                delete_failed = summary.delete_failed,
                kept_unchecked = summary.kept_unchecked,
                "Removed {} records of missing files from '{}': {} in the library, {} in its trash, {} outside its folder ({} could not be removed, {} kept because their file could not be checked)",
                summary.removed(),
                library.name,
                summary.removed_in_library,
                summary.removed_in_trash,
                summary.removed_outside_root,
                summary.delete_failed,
                summary.kept_unchecked
            );
            summary.removed()
        };

        let organized_misplaced_count = if library.auto_organize {
            self.organize_unchanged_matched_files(&library, &auth_user, organize_candidates)
                .await
        } else {
            0
        };

        // Library-wide tidying is left to whole-library scans; a scan of changes stays as small
        // as the change.
        if scope.is_full() {
            self.reconcile_movie_collections_for_library(&library).await;

            if Self::is_tv_library_type(&normalized_library_type) {
                self.ensure_tv_folder_structure(&library).await?;
            }

            if library.auto_organize {
                self.cleanup_empty_folders(&library).await?;
            }
        }

        self.update_scan_run(
            scan_run_id,
            UpdateLibraryScanRunInput {
                current_stage: Some("ANALYSIS_SETTLE".to_string()),
                ..Default::default()
            },
        )
        .await?;
        let analysis_settled = self
            .wait_for_analysis_settlement(&run_analysis_ids, Duration::from_secs(120))
            .await;
        if !analysis_settled {
            self.record_scan_issue(
                scan_run_id,
                &library,
                None,
                "ANALYSIS",
                "ANALYSIS_SETTLE_TIMEOUT",
                "WARNING",
                "Some analysis jobs did not settle before the scan deadline.",
                Some("Use Retry analysis for the remaining failed or queued files."),
            )
            .await?;
        }

        // Only a whole-library walk counts as "scanned" for the schedule, which is the watch's
        // safety net.
        self.set_library_scanning(&auth_user, library_id, false, scope.is_full())
            .await?;
        let database = self.database().await?;
        let analysis_failed = LibraryScanRun::get(database.pool().pool(), &scan_run_id.to_string())
            .await?
            .map(|run| run.analysis_failed_count)
            .unwrap_or_default();
        let completed_with_issues = match_pipeline_error_count > 0
            || provider_summary.has_issues()
            || !analysis_settled
            || analysis_failed > 0
            || duplicate_count > 0;
        let terminal_status = if completed_with_issues {
            "COMPLETED_WITH_ISSUES"
        } else {
            "COMPLETED"
        };
        let elapsed = scan_started.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;
        let files_per_second = if elapsed.is_zero() {
            scanned_count as f64
        } else {
            scanned_count as f64 / elapsed.as_secs_f64()
        };
        let graphql_entity_operations = SCAN_GRAPHQL_ENTITY_OPERATIONS
            .try_with(Cell::get)
            .unwrap_or_default();
        let graphql_entity_operations_per_file = if scanned_count == 0 {
            0.0
        } else {
            graphql_entity_operations as f64 / scanned_count as f64
        };
        self.update_scan_run(
            scan_run_id,
            UpdateLibraryScanRunInput {
                status: Some(terminal_status.to_string()),
                current_stage: Some("COMPLETE".to_string()),
                finished_at: Some(Some(Utc::now().to_rfc3339())),
                discovered_count: Some(Self::usize_count(scanned_count)),
                existing_count: Some(Self::usize_count(existing_count)),
                matched_count: Some(Self::usize_count(matched_count)),
                unmatched_count: Some(Self::usize_count(provider_summary.unmatched_files)),
                provider_blocked_count: Some(if provider_summary.configuration_blocked {
                    Self::usize_count(provider_summary.pending_files)
                } else {
                    0
                }),
                analysis_queued_count: Some(Self::usize_count(analysis_queued_count)),
                summary: Some(Some(if let (true, ScanScope::Changes(targets)) =
                    (completed_with_issues, scope)
                {
                    format!(
                        "Changes checked in {} with issues: {} found, {} matched, {} removed, {} analysis failures, {} byte-identical duplicates",
                        describe_change_targets(&library.path, targets),
                        scanned_count,
                        matched_count,
                        removed_count,
                        analysis_failed,
                        duplicate_count
                    )
                } else if completed_with_issues {
                    format!(
                        "Scan completed with issues: {} discovered, {} matched, {} unmatched, {} analysis failures, {} byte-identical duplicates",
                        scanned_count,
                        matched_count,
                        provider_summary.unmatched_files,
                        analysis_failed,
                        duplicate_count
                    )
                } else if let ScanScope::Changes(targets) = scope {
                    format!(
                        "Changes checked in {}: {} found, {} matched, {} removed",
                        describe_change_targets(&library.path, targets),
                        scanned_count,
                        matched_count,
                        removed_count
                    )
                } else {
                    format!(
                        "Scan completed: {} discovered and {} matched",
                        scanned_count, matched_count
                    )
                })),
                ..Default::default()
            },
        )
        .await?;
        if completed_with_issues {
            warn!(
                library_id = %library_id,
                library_name = %library.name,
                library_type = %library.library_type,
                scanned_count,
                discovered_path_count = discovered_paths.len(),
                match_pipeline_error_count,
                provider_pending_files = provider_summary.pending_files,
                provider_matched_files = provider_summary.matched_files,
                provider_unmatched_files = provider_summary.unmatched_files,
                provider_error_files = provider_summary.error_files,
                provider_configuration_blocked = provider_summary.configuration_blocked,
                duplicate_count,
                unchanged_skipped_count,
                organized_misplaced_count,
                auto_organize = library.auto_organize,
                elapsed_ms,
                files_per_second,
                graphql_entity_operations,
                graphql_entity_operations_per_file,
                "Library scan completed with issues: library_id={}, name={}, type={}, scanned_files={}, match_errors={}, provider_pending_files={}, provider_matched_files={}, provider_unmatched_files={}, provider_error_files={}, provider_configuration_blocked={}, duplicate_count={}, auto_organize={}",
                library_id,
                library.name,
                library.library_type,
                scanned_count,
                match_pipeline_error_count,
                provider_summary.pending_files,
                provider_summary.matched_files,
                provider_summary.unmatched_files,
                provider_summary.error_files,
                provider_summary.configuration_blocked,
                duplicate_count,
                library.auto_organize
            );
        } else {
            info!(
                library_id = %library_id,
                library_name = %library.name,
                library_type = %library.library_type,
                scanned_count,
                discovered_path_count = discovered_paths.len(),
                provider_pending_files = provider_summary.pending_files,
                provider_matched_files = provider_summary.matched_files,
                unchanged_skipped_count,
                organized_misplaced_count,
                auto_organize = library.auto_organize,
                elapsed_ms,
                files_per_second,
                graphql_entity_operations,
                graphql_entity_operations_per_file,
                "Library scan completed: library_id={}, name={}, type={}, scanned_files={}, discovered_paths={}, provider_matched_files={}, auto_organize={}",
                library_id,
                library.name,
                library.library_type,
                scanned_count,
                discovered_paths.len(),
                provider_summary.matched_files,
                library.auto_organize
            );
        }

        self.clear_candidate_caches_for_library(library_id).await;

        self.close_stale_duplicates(&library).await;
        self.settle_duplicates_if_asked(&library).await;
        if library.library_type == "music"
            && (scope.is_full() || scanned_count > unchanged_skipped_count)
        {
            self.backfill_album_covers(&library).await;
            self.backfill_artist_images(&library).await;
        }

        // Downloads compared with this library before may now be in it, or have more to add.
        if scanned_count > unchanged_skipped_count || removed_count > 0 {
            self.recheck_identified(&library.user_id, &library.library_type);
        }

        Ok(())
    }

    async fn run_scan_worker(self: Arc<Self>, cancel: CancellationToken) {
        loop {
            if cancel.is_cancelled() {
                break;
            }

            let job = tokio::time::timeout(Duration::from_secs(1), async {
                let mut rx = self.scan_rx.lock().await;
                rx.recv().await
            })
            .await
            .ok()
            .flatten();

            let Some(job) = job else {
                // No job available right now (or timed out waiting); keep worker alive.
                continue;
            };

            info!(
                library_id = %job.library_id,
                "Scan worker received queued library scan job: library_id={}",
                job.library_id
            );

            let mut should_run = true;
            {
                let mut in_progress = self.in_progress_scans.lock().await;
                if in_progress.contains(&job.library_id) {
                    should_run = false;
                } else {
                    in_progress.insert(job.library_id.clone());
                }
            }

            if !should_run {
                let _ = self
                    .update_scan_run(
                        &job.scan_run_id,
                        UpdateLibraryScanRunInput {
                            status: Some("CANCELLED".to_string()),
                            current_stage: Some("QUEUE".to_string()),
                            finished_at: Some(Some(Utc::now().to_rfc3339())),
                            error_code: Some(Some("DUPLICATE_SCAN".to_string())),
                            summary: Some(Some(
                                "A scan was already active for this library.".to_string(),
                            )),
                            ..Default::default()
                        },
                    )
                    .await;
                continue;
            }

            if let Err(e) = SCAN_GRAPHQL_ENTITY_OPERATIONS
                .scope(
                    Cell::new(0),
                    self.scan_library_inner(
                        &job.library_id,
                        &job.scan_run_id,
                        &job.scope,
                        cancel.child_token(),
                    ),
                )
                .await
            {
                error!(
                    library_id = %job.library_id,
                    scan_run_id = %job.scan_run_id,
                    error = %format!("{e:#}"),
                    "Library scan failed: library_id={}, error={:#}",
                    job.library_id,
                    e
                );
                if let Ok(auth_user) = self.system_auth_user(None).await {
                    let _ = self
                        .set_library_scanning(&auth_user, &job.library_id, false, false)
                        .await;
                }
                let cancelled = cancel.is_cancelled() || e.to_string() == "SCAN_CANCELLED";
                let _ = self
                    .update_scan_run(
                        &job.scan_run_id,
                        UpdateLibraryScanRunInput {
                            status: Some(
                                if cancelled { "CANCELLED" } else { "FAILED" }.to_string(),
                            ),
                            finished_at: Some(Some(Utc::now().to_rfc3339())),
                            error_code: Some(Some(
                                if cancelled {
                                    "SCAN_CANCELLED"
                                } else {
                                    "SCAN_FAILED"
                                }
                                .to_string(),
                            )),
                            summary: Some(Some(
                                if cancelled {
                                    "The scan was cancelled during shutdown."
                                } else {
                                    "The scan could not complete. Review its issues and retry."
                                }
                                .to_string(),
                            )),
                            ..Default::default()
                        },
                    )
                    .await;
            }
            self.clear_candidate_caches_for_library(&job.library_id)
                .await;

            let mut in_progress = self.in_progress_scans.lock().await;
            in_progress.remove(&job.library_id);
        }
    }

    async fn run_scheduler(self: Arc<Self>, cancel: CancellationToken) {
        let mut interval = tokio::time::interval(self.config.autoscan_poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = interval.tick() => {
                    match self.get_due_autoscan_libraries().await {
                        Ok(ids) => {
                            for id in ids {
                                if let Err(e) = self.queue_scan(&id).await {
                                    warn!(
                                        library_id = %id,
                                        error = %e,
                                        "Failed to queue autoscan job: library_id={}, error={}",
                                        id,
                                        e
                                    );
                                }
                            }
                        }
                        Err(e) => warn!(
                            error = %e,
                            "Failed to evaluate autoscan schedule: error={}",
                            e
                        ),
                    }
                }
            }
        }
    }

    async fn analyze_media_file_inner(&self, media_file_id: &str, path: &str) -> Result<()> {
        info!(
            media_file_id = %media_file_id,
            media_file_path = %path,
            "Starting media file analysis with ffprobe: media_file_id={}, path={}",
            media_file_id,
            path
        );
        let actor = AuthUser::system_admin();
        let file = self.get_media_file(media_file_id).await?;
        let library_id = file
            .library_id
            .as_deref()
            .context("Cannot analyze a file with unknown ownership")?;
        let library = self.get_library(library_id).await?;
        let target = AnalysisTarget::capture(&file, &library, path)?;
        let analysis = ffprobe_analyze(path).await?;
        let database = self.database().await?;
        replace_media_analysis(database.pool(), &actor, target, analysis.clone()).await?;
        if analysis.video_codec.is_some()
            && crate::services::subtitles::store::is_video_extension(
                path.rsplit_once('.')
                    .map(|(_, ext)| ext)
                    .unwrap_or_default(),
            )
        {
            self.sync_video_sidecars(media_file_id, path).await;
            // Preferred languages it still lacks, when automatic downloads are on (Q119).
            let manager = self.manager.clone();
            let schema = self.graphql_schema().await.ok();
            let file = MediaFile::get(database.pool().pool(), &media_file_id.to_owned())
                .await
                .ok()
                .flatten();
            if let (Some(schema), Some(file)) = (schema, file) {
                let database = database.clone();
                tokio::spawn(async move {
                    crate::services::subtitles::provider::auto_fetch(
                        database.pool(),
                        &schema,
                        &manager,
                        &file,
                    )
                    .await;
                });
            }
        }

        // design.md Q23/Q24: the MediaFile hook (`requalify_media_file`) recomputes
        // `quality_status` against the resolved profile as this write commits, so the
        // verified resolution/codec/HDR/audio columns are judged without a second path.

        info!(
            media_file_id = %media_file_id,
            media_file_path = %path,
            container = ?analysis.container,
            video_codec = ?analysis.video_codec,
            audio_codec = ?analysis.audio_codec,
            width = ?analysis.width,
            height = ?analysis.height,
            duration = ?analysis.duration,
            bitrate = ?analysis.bitrate,
            resolution = ?analysis.resolution,
            is_hdr = analysis.is_hdr,
            hdr_type = ?analysis.hdr_type,
            audio_channels = ?analysis.audio_channels,
            video_stream_count = analysis.video_streams.len(),
            audio_stream_count = analysis.audio_streams.len(),
            subtitle_count = analysis.subtitles.len(),
            chapter_count = analysis.chapters.len(),
            "Media file analysis completed and persisted: media_file_id={}, path={}, container={:?}, video_codec={:?}, audio_codec={:?}, resolution={:?}",
            media_file_id,
            path,
            analysis.container,
            analysis.video_codec,
            analysis.audio_codec,
            analysis.resolution
        );

        Ok(())
    }

    async fn run_analyze_worker(self: Arc<Self>, cancel: CancellationToken, worker_idx: usize) {
        loop {
            if cancel.is_cancelled() {
                break;
            }

            let job = tokio::time::timeout(Duration::from_secs(1), async {
                let mut rx = self.analyze_rx.lock().await;
                rx.recv().await
            })
            .await
            .ok()
            .flatten();

            let Some(job) = job else {
                // No analyze job currently queued; keep worker alive.
                continue;
            };

            if !*self.ffprobe_available.read().await {
                warn!(
                    worker = worker_idx,
                    media_file_id = %job.media_file_id,
                    media_file_path = %job.path,
                    "Analyze worker skipped job because ffprobe is unavailable: worker={}, media_file_id={}, path={}",
                    worker_idx,
                    job.media_file_id,
                    job.path
                );
                let mut queued = self.queued_analysis.lock().await;
                queued.remove(&job.media_file_id);
                drop(queued);
                if let Some(scan_run_id) = job.scan_run_id.as_deref() {
                    let _ = self.record_analysis_outcome(scan_run_id, false).await;
                    let _ = self
                        .record_analysis_issue(
                            scan_run_id,
                            &job.media_file_id,
                            "FFPROBE_UNAVAILABLE",
                            "ffprobe is unavailable on the server.",
                            "Install or configure ffprobe, then retry analysis.",
                            None,
                        )
                        .await;
                }
                if let Some(issue_id) = job.retry_issue_id.as_deref() {
                    let error = anyhow::Error::new(MediaAnalysisError::ExecutableUnavailable);
                    let _ = self
                        .record_analysis_retry_outcome(issue_id, Some(&error))
                        .await;
                }
                continue;
            }

            info!(
                worker = worker_idx,
                media_file_id = %job.media_file_id,
                path = %job.path,
                "Analyze worker received queued job: worker={}, media_file_id={}, path={}",
                worker_idx,
                job.media_file_id,
                job.path
            );

            let analysis_result = self
                .analyze_media_file_inner(&job.media_file_id, &job.path)
                .await;
            if let Err(e) = &analysis_result {
                warn!(
                    worker = worker_idx,
                    media_file_id = %job.media_file_id,
                    media_file_path = %job.path,
                    error = %e,
                    "Analyze worker failed for media file: worker={}, media_file_id={}, path={}, error={}",
                    worker_idx,
                    job.media_file_id,
                    job.path,
                    e
                );
            } else {
                // design.md Q124: black bars and the storyboard follow analysis, on their own queue.
                if let Some(previews) = crate::services::video_preview::video_previews() {
                    previews.queue_after_analysis(&job.media_file_id).await;
                }
                info!(
                    worker = worker_idx,
                    media_file_id = %job.media_file_id,
                    media_file_path = %job.path,
                    "Analyze worker completed job successfully: worker={}, media_file_id={}, path={}",
                    worker_idx,
                    job.media_file_id,
                    job.path
                );
            }

            let mut queued = self.queued_analysis.lock().await;
            queued.remove(&job.media_file_id);
            drop(queued);
            if let Some(scan_run_id) = job.scan_run_id.as_deref() {
                let succeeded = analysis_result.is_ok();
                if let Err(error) = self.record_analysis_outcome(scan_run_id, succeeded).await {
                    warn!(
                        scan_run_id,
                        media_file_id = %job.media_file_id,
                        error = %error,
                        "Failed to persist scan analysis outcome"
                    );
                }
                if let Err(error) = &analysis_result {
                    let (code, message, remediation) = classify_analysis_error(error);
                    let _ = self
                        .record_analysis_issue(
                            scan_run_id,
                            &job.media_file_id,
                            code,
                            message,
                            remediation,
                            Some(
                                serde_json::json!({
                                    "diagnostic": Self::bounded_issue_text(&error.to_string(), 4_000),
                                    "attemptCount": 1,
                                })
                                .to_string(),
                            ),
                        )
                        .await;
                }
            }
            if let Some(issue_id) = job.retry_issue_id.as_deref()
                && let Err(error) = self
                    .record_analysis_retry_outcome(issue_id, analysis_result.as_ref().err())
                    .await
            {
                warn!(
                    scan_issue_id = issue_id,
                    media_file_id = %job.media_file_id,
                    error = %error,
                    "Failed to persist analysis retry outcome"
                );
            }
        }
    }

    async fn record_analysis_issue(
        &self,
        scan_run_id: &str,
        media_file_id: &str,
        code: &str,
        message: &str,
        remediation: &str,
        details_json: Option<String>,
    ) -> Result<()> {
        let database = self.database().await?;
        let Some(run) =
            LibraryScanRun::get(database.pool().pool(), &scan_run_id.to_string()).await?
        else {
            return Ok(());
        };
        let library = self.get_library(&run.library_id).await?;
        self.record_scan_issue_with_details(
            scan_run_id,
            &library,
            Some(media_file_id),
            "ANALYSIS",
            code,
            "WARNING",
            message,
            Some(remediation),
            details_json,
        )
        .await
    }

    async fn get_media_file(&self, media_file_id: &str) -> Result<MediaFileRow> {
        let auth_user = self.system_auth_user(None).await?;
        let data = self
            .execute_graphql(
                &auth_user,
                r#"query MediaFileById($id: String!) {
                    MediaFile: mediaFile(id: $id) {
                        Id: id
                        LibraryId: libraryId
                        Path: path
                        OriginalName: originalName
                        EpisodeId: episodeId
                        MovieId: movieId
                        TrackId: trackId
                        ChapterId: chapterId
                        MatchType: matchType
                        Size: size
                        AnalyzedAt: analyzedAt
                        FileModifiedAt: fileModifiedAt
                        QualityStatus: qualityStatus
                    }
                }"#,
                serde_json::json!({ "id": media_file_id }),
            )
            .await?;

        let row = data
            .get("MediaFile")
            .ok_or_else(|| anyhow::anyhow!("Media file not found"))?;

        Ok(MediaFileRow {
            id: row
                .get("Id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            library_id: row
                .get("LibraryId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            path: row
                .get("Path")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            original_name: row
                .get("OriginalName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            episode_id: row
                .get("EpisodeId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            movie_id: row
                .get("MovieId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            track_id: row
                .get("TrackId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            chapter_id: row
                .get("ChapterId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            match_type: row
                .get("MatchType")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            size: row.get("Size").and_then(|v| v.as_i64()).unwrap_or_default(),
            analyzed_at: row
                .get("AnalyzedAt")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            file_modified_at: row
                .get("FileModifiedAt")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            quality_status: row
                .get("QualityStatus")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }

    /// Apply a match link to a media file. `match_type` must be "manual" (explicit
    /// user-selected target) or "auto" (ranked-candidate/provider-fallback target) —
    /// see design.md Q9b. Manual matches record `matched_by_user_id` and
    /// `match_confirmed_at`; automatic matches leave those unset.
    #[allow(clippy::too_many_arguments)]
    async fn apply_media_file_match_update(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        library_id: &str,
        target_type: &str,
        target_id: &str,
        path_update: Option<(&str, Option<&str>)>,
        match_type: &str,
        matched_by_user_id: Option<&str>,
    ) -> Result<()> {
        let (path, relative_path) = path_update.unwrap_or(("", None));
        let match_confirmed_at = if match_type == "manual" {
            Some(
                chrono::Utc::now()
                    .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                    .to_string(),
            )
        } else {
            None
        };
        let mut media_input = serde_json::json!({
            "libraryId": library_id,
            "movieId": if target_type == "Movie" { serde_json::Value::String(target_id.to_string()) } else { serde_json::Value::Null },
            "episodeId": if target_type == "Episode" { serde_json::Value::String(target_id.to_string()) } else { serde_json::Value::Null },
            "trackId": if target_type == "Track" { serde_json::Value::String(target_id.to_string()) } else { serde_json::Value::Null },
            "chapterId": if target_type == "Chapter" { serde_json::Value::String(target_id.to_string()) } else { serde_json::Value::Null },
            "matchType": match_type,
            "matchedByUserId": matched_by_user_id,
            "matchConfirmedAt": match_confirmed_at,
        });
        if !path.is_empty() {
            media_input["path"] = serde_json::Value::String(path.to_string());
            media_input["relativePath"] = relative_path
                .map(|value| serde_json::Value::String(value.to_string()))
                .unwrap_or(serde_json::Value::Null);
        }

        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation ApplyMediaFileMatch($mediaFileId: String!, $mediaInput: UpdateMediaFileInput!) {
                    UpdateMediaFile: updateMediaFile(id: $mediaFileId, input: $mediaInput) { Success: success Error: error }
                }"#,
                serde_json::json!({
                    "mediaFileId": media_file_id,
                    "mediaInput": media_input
                }),
            )
            .await?;

        let media_ok = data
            .get("UpdateMediaFile")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !media_ok {
            let err = data
                .get("UpdateMediaFile")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("failed to update media file match");
            anyhow::bail!(err.to_string());
        }

        Ok(())
    }

    async fn link_movie(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        library_id: &str,
        movie_id: &str,
        match_type: &str,
        matched_by_user_id: Option<&str>,
    ) -> Result<()> {
        self.apply_media_file_match_update(
            auth_user,
            media_file_id,
            library_id,
            "Movie",
            movie_id,
            None,
            match_type,
            matched_by_user_id,
        )
        .await
    }

    async fn link_episode(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        library_id: &str,
        episode_id: &str,
        match_type: &str,
        matched_by_user_id: Option<&str>,
    ) -> Result<()> {
        self.apply_media_file_match_update(
            auth_user,
            media_file_id,
            library_id,
            "Episode",
            episode_id,
            None,
            match_type,
            matched_by_user_id,
        )
        .await?;
        Ok(())
    }

    async fn link_track(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        library_id: &str,
        track_id: &str,
        match_type: &str,
        matched_by_user_id: Option<&str>,
    ) -> Result<()> {
        self.apply_media_file_match_update(
            auth_user,
            media_file_id,
            library_id,
            "Track",
            track_id,
            None,
            match_type,
            matched_by_user_id,
        )
        .await?;
        Ok(())
    }

    async fn link_chapter(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        library_id: &str,
        chapter_id: &str,
        match_type: &str,
        matched_by_user_id: Option<&str>,
    ) -> Result<()> {
        self.apply_media_file_match_update(
            auth_user,
            media_file_id,
            library_id,
            "Chapter",
            chapter_id,
            None,
            match_type,
            matched_by_user_id,
        )
        .await?;
        Ok(())
    }

    fn normalize_for_match(s: &str) -> String {
        let lower = s.to_ascii_lowercase();
        let no_ext = lower
            .rsplit_once('.')
            .map(|(base, _)| base.to_string())
            .unwrap_or(lower);

        let patterns = [
            r"\b(2160p|1080p|720p|480p)\b",
            r"\b(x264|x265|h264|h265|hevc|bluray|brrip|webrip|web-dl|dvdrip)\b",
            r"\b(aac|dts|ddp?5?\.?1?|flac|atmos)\b",
            r"\b(telesync|telecine|hdts|hdcam|ts|tc)\b",
            r"\b(sample|trailer|preview)\b",
        ];

        let mut cleaned = no_ext;
        for p in patterns {
            if let Ok(re) = Regex::new(p) {
                cleaned = re.replace_all(&cleaned, " ").to_string();
            }
        }

        cleaned
            .replace(['.', '_', '-', '[', ']', '(', ')'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn extract_title_sequence_number(s: &str) -> Option<i32> {
        let re = Regex::new(r"\b([0-9]{1,2})\b").ok()?;
        re.captures(&Self::normalize_for_match(s))
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<i32>().ok())
    }

    /// Score how well an already-normalized filename hint matches a candidate
    /// show name, for episode matching. `normalized_query` must already be
    /// `normalize_for_match`-ed by the caller; `show_name` is raw.
    ///
    /// Deliberately has NO floor: unlike season/episode number matches, the
    /// show-name similarity is what actually distinguishes "the right show" from
    /// any other show with a matching S/E number, so a floor here would let
    /// completely unrelated shows reach the auto-link threshold (audit #3).
    fn score_episode_show_candidate(
        normalized_query: &str,
        show_name: &str,
        hint_year: Option<i32>,
        show_year: Option<i32>,
    ) -> f64 {
        let show_norm = Self::normalize_for_match(show_name);
        let mut score = if normalized_query.contains(&show_norm) {
            0.95
        } else {
            jaro_winkler(normalized_query, &show_norm) * 0.9
        };
        if let Some(parsed_year) = hint_year
            && let Some(y) = show_year
            && y == parsed_year
        {
            score += 0.05;
        }
        score
    }

    /// Manual-match protection guard (design.md Q9b): automatic matching/rematch
    /// (ranked-candidate or provider-fallback — i.e. not an explicit target id from
    /// the user) must never be applied on top of an existing manual match, even when
    /// `force: true`. Explicit-id requests are allowed through unconditionally here —
    /// replacing a manual match via an explicit pick is the user's own intent.
    fn should_block_automatic_match(
        is_explicit_target: bool,
        current_match_type: Option<&str>,
    ) -> bool {
        !is_explicit_target && current_match_type == Some("manual")
    }

    fn parse_year_hint(s: &str) -> Option<i32> {
        let year_re = Regex::new(r"\b(19\d{2}|20\d{2})\b").ok()?;
        year_re
            .captures(s)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<i32>().ok())
    }

    fn parse_movie_year_hint(file_name: &str, parent_name: &str) -> Option<i32> {
        // Strongest signal: explicit bracketed year, usually "(1990)" or "[1990]".
        let bracket_year_re = Regex::new(r"[\[\(](19\d{2}|20\d{2})[\]\)]").ok();
        if let Some(re) = &bracket_year_re {
            if let Some(year) = re
                .captures(file_name)
                .and_then(|c| c.get(1))
                .and_then(|m| m.as_str().parse::<i32>().ok())
            {
                return Some(year);
            }
            if let Some(year) = re
                .captures(parent_name)
                .and_then(|c| c.get(1))
                .and_then(|m| m.as_str().parse::<i32>().ok())
            {
                return Some(year);
            }
        }

        let year_re = Regex::new(r"\b(19\d{2}|20\d{2})\b").ok()?;
        let combined = format!("{} {}", file_name, parent_name);
        let current_year = Utc::now().year();
        let mut all_years: Vec<i32> = year_re
            .captures_iter(&combined)
            .filter_map(|c| c.get(1))
            .filter_map(|m| m.as_str().parse::<i32>().ok())
            .collect();
        if all_years.is_empty() {
            return None;
        }

        // Prefer plausible release years (not far-future). If there are multiple, use the last one.
        all_years.retain(|y| *y >= 1900 && *y <= current_year + 1);
        if let Some(y) = all_years.last() {
            return Some(*y);
        }

        // Fallback to last seen year if everything looked implausible.
        year_re
            .captures_iter(&combined)
            .filter_map(|c| c.get(1))
            .filter_map(|m| m.as_str().parse::<i32>().ok())
            .last()
    }

    fn strip_release_tokens(s: &str) -> String {
        let mut out = s.to_string();
        if let Ok(re) = Regex::new(r"(?i)(?:-[a-z0-9]{2,})+$") {
            out = re.replace(&out, "").to_string();
        }
        if let Ok(re) = Regex::new(r"(?i)(?:\s+-\s*[a-z0-9]{2,})+$") {
            out = re.replace(&out, "").to_string();
        }
        out = out.replace(['.', '_'], " ");
        let patterns = [
            r"(?i)\[[a-z]{2,3}\]",
            r"(?i)^\s*\[[^\]]+\]\s*",
            r"(?i)\b(2160p|1080p|960p|720p|576p|480p|360p|4k|uhd)\b",
            r"(?i)\b(x[ ._-]?264|x[ ._-]?265|h[ ._-]?264|h[ ._-]?265|hevc|av1|xvid|10bit|8bit)\b",
            r"(?i)\b(bluray|blu-ray|brrip|bdrip|webrip|web-dl|hdtv|dvdrip|dvdscr|vhsrip|hdrip|cam|web|sdr|hdr10\+?|hdr|dovi|dv|telesync|telecine|hdts|hdcam|ts|tc)\b",
            r"(?i)\b(ddp\s*\.?\s*5\s*\.?\s*1|dd\s*\.?\s*5\s*\.?\s*1|aac\s*\.?\s*5\s*\.?\s*1|ddpa\s*\.?\s*5\s*\.?\s*1)\b",
            r"(?i)\b(atmos|truehd|dts[- ]?x|dts-hd|dts|ddp?a?\d*|aac\d*|ac3|flac|ma|6ch)\b",
            r"(?i)\b(proper|repack|internal|extended|unrated|remux|sample|hybrid|screener|read|readnfo|nfo|hq|shq|hc|v[2-9])\b",
            r"(?i)\b(collective|pirates|syncup)\b",
            r"(?i)\b(hive|cm8|nhd)\b",
            r"(?i)\b(directors?\s+cut|theatrical\s+cut|extended\s+edition|edition|cut)\b",
            r"(?i)\b(amzn|atvp|hmax|webios|nf|dsnp|pcok|ptv|it|retail|korsub|rosubbed)\b",
            r"(?i)\b(new\s+source)\b",
            r"(?i)\b(eng|english|multi|multilang)\b",
            r"(?i)\b[1-9]\s*[\. ]\s*[0-9]\b",
        ];
        for pattern in patterns {
            if let Ok(re) = Regex::new(pattern) {
                out = re.replace_all(&out, " ").to_string();
            }
        }
        out.replace(['.', '_', '-', '[', ']', '(', ')'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string()
    }

    fn parse_movie_hint(path: &str) -> ParsedMovieHint {
        let raw_file_name = Path::new(path)
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        let without_group = Regex::new(r"-[A-Za-z0-9]{2,}$")
            .ok()
            .map(|re| re.replace(raw_file_name, "").to_string())
            .unwrap_or_else(|| raw_file_name.to_string());
        let file_name = Regex::new(r"^\s*\[[^\]]+\]\s*")
            .ok()
            .map(|re| re.replace(&without_group, "").to_string())
            .unwrap_or(without_group);
        let parent_name = Path::new(path)
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or(&file_name);

        let year = Self::parse_movie_year_hint(&file_name, parent_name);
        let base = if let Some(y) = year {
            let year_re = Regex::new(&format!(r"\b{}\b", y))
                .ok()
                .map(|re| re.replace_all(&file_name, " ").to_string())
                .unwrap_or_else(|| file_name.clone());
            Self::strip_release_tokens(&year_re)
        } else {
            Self::strip_release_tokens(&file_name)
        };

        ParsedMovieHint {
            title: if base.is_empty() { None } else { Some(base) },
            year,
        }
    }

    fn parse_episode_hint(path: &str) -> ParsedEpisodeHint {
        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        let cleaned = file_name.replace(['.', '_', '-'], " ");
        let season_episode_re = Regex::new(r"(?i)s(\d{1,2})e(\d{1,3})|(\d{1,2})x(\d{1,3})").ok();

        let (season, episode, title_part) = if let Some(re) = season_episode_re {
            if let Some(c) = re.captures(&cleaned) {
                let season = c
                    .get(1)
                    .or_else(|| c.get(3))
                    .and_then(|m| m.as_str().parse::<i32>().ok());
                let episode = c
                    .get(2)
                    .or_else(|| c.get(4))
                    .and_then(|m| m.as_str().parse::<i32>().ok());
                let idx = c.get(0).map(|m| m.start()).unwrap_or(cleaned.len());
                (season, episode, cleaned[..idx].trim().to_string())
            } else {
                (None, None, cleaned)
            }
        } else {
            (None, None, cleaned)
        };

        let mut show_name = Self::strip_release_tokens(&title_part);
        if show_name.is_empty() {
            show_name = Self::episode_show_name_from_path(path);
        }
        ParsedEpisodeHint {
            show_name: if show_name.is_empty() {
                None
            } else {
                Some(show_name)
            },
            season,
            episode,
            year: Self::parse_year_hint(file_name),
        }
    }

    fn episode_show_name_from_path(path: &str) -> String {
        let path_obj = Path::new(path);
        let Some(parent) = path_obj.parent() else {
            return String::new();
        };

        let parent_name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let season_dir_re = Regex::new(r"(?i)^(season|series|s)\s*0*\d{1,2}$|^s0*\d{1,2}$").ok();
        let show_dir = if season_dir_re
            .as_ref()
            .map(|re| re.is_match(parent_name.trim()))
            .unwrap_or(false)
        {
            parent.parent().and_then(|p| p.file_name())
        } else {
            parent.file_name()
        };

        show_dir
            .and_then(|n| n.to_str())
            .map(Self::strip_release_tokens)
            .unwrap_or_default()
    }

    fn parse_track_hint(path: &str) -> ParsedTrackHint {
        let path_obj = Path::new(path);
        let file_stem = path_obj
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        let album_name_raw = path_obj
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let artist_name_raw = path_obj
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let track_re = Regex::new(r"(?i)^(?:track\s*)?(\d{1,3})[\s\-._]+(.+)$").ok();
        let (track_number, title_raw) = if let Some(re) = track_re {
            if let Some(c) = re.captures(file_stem) {
                (
                    c.get(1).and_then(|m| m.as_str().parse::<i32>().ok()),
                    c.get(2).map(|m| m.as_str()).unwrap_or(file_stem),
                )
            } else {
                (None, file_stem)
            }
        } else {
            (None, file_stem)
        };

        let title = Self::strip_release_tokens(title_raw);
        let album_name = Self::strip_release_tokens(album_name_raw);
        let artist_name = Self::strip_release_tokens(artist_name_raw);

        ParsedTrackHint {
            artist_name: if artist_name.is_empty() {
                None
            } else {
                Some(artist_name)
            },
            album_name: if album_name.is_empty() {
                None
            } else {
                Some(album_name)
            },
            title: if title.is_empty() { None } else { Some(title) },
            track_number,
        }
    }

    fn parse_chapter_hint(path: &str) -> ParsedChapterHint {
        let path_obj = Path::new(path);
        let file_stem = path_obj
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        let audiobook_title_raw = path_obj
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let author_name_raw = path_obj
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let chapter_re = Regex::new(r"(?i)^(?:ch(?:apter)?|track)?\s*(\d{1,3})[\s\-._]+(.+)$").ok();
        let (chapter_number, chapter_title_raw) = if let Some(re) = chapter_re {
            if let Some(c) = re.captures(file_stem) {
                (
                    c.get(1).and_then(|m| m.as_str().parse::<i32>().ok()),
                    c.get(2).map(|m| m.as_str()).unwrap_or(file_stem),
                )
            } else {
                (None, file_stem)
            }
        } else {
            (None, file_stem)
        };

        let chapter_title = Self::strip_release_tokens(chapter_title_raw);
        let audiobook_title = Self::strip_release_tokens(audiobook_title_raw);
        let author_name = Self::strip_release_tokens(author_name_raw);

        ParsedChapterHint {
            author_name: if author_name.is_empty() {
                None
            } else {
                Some(author_name)
            },
            audiobook_title: if audiobook_title.is_empty() {
                None
            } else {
                Some(audiobook_title)
            },
            chapter_title: if chapter_title.is_empty() {
                None
            } else {
                Some(chapter_title)
            },
            chapter_number,
        }
    }

    async fn find_episode_match(
        &self,
        library_id: &str,
        path: &str,
    ) -> Result<Option<(String, f64)>> {
        let auth_user = self.system_auth_user(None).await?;

        let hint = Self::parse_episode_hint(path);
        let (season, episode) = match (hint.season, hint.episode) {
            (Some(s), Some(e)) => (s, e),
            _ => return Ok(None),
        };

        let normalized = hint
            .show_name
            .as_deref()
            .map(Self::normalize_for_match)
            .unwrap_or_else(|| Self::normalize_for_match(path));

        let data = self
            .execute_graphql_paged(
                &auth_user,
                r#"query FindEpisodeMatch($libraryId: String!, $season: Int!, $episode: Int!, $page: PageInput) {
                    Shows: shows(
                        where: { libraryId: { eq: $libraryId } }
                        page: $page
                    ) {
                        Edges: edges {
                            Node: node {
                                Name: name
                                Year: year
                                Episodes: episodes(
                                    where: {
                                        season: { eq: $season }
                                        episode: { eq: $episode }
                                    }
                                    page: { limit: 100 }
                                ) {
                                    Edges: edges { Node: node { Id: id } }
                                }
                            }
                        }
                    }
                }"#,
                serde_json::json!({
                    "libraryId": library_id,
                    "season": season,
                    "episode": episode,
                }),
                "Shows",
            )
            .await?;
        let rows: Vec<(String, String, Option<i32>)> = data
            .get("Shows")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .flat_map(|edge| {
                let Some(show) = edge.get("Node") else {
                    return Vec::new();
                };
                let show_name = show
                    .get("Name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let show_year = show.get("Year").and_then(|v| v.as_i64()).map(|v| v as i32);
                show.get("Episodes")
                    .and_then(|v| v.get("Edges"))
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(move |episode_edge| {
                        let node = episode_edge.get("Node")?;
                        Some((
                            node.get("Id")?.as_str()?.to_string(),
                            show_name.clone(),
                            show_year,
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        if rows.is_empty() {
            return Ok(None);
        }

        let mut best: Option<(String, f64)> = None;
        for (id, show_name, show_year) in rows {
            let score =
                Self::score_episode_show_candidate(&normalized, &show_name, hint.year, show_year);
            if score > best.as_ref().map(|(_, s)| *s).unwrap_or(0.0) {
                best = Some((id, score));
            }
        }

        // Require a minimum show-name similarity before returning a match at all —
        // otherwise a file whose S/E numbers happen to line up with a completely
        // unrelated show would still be returned as "the" match (audit #3).
        Ok(best.filter(|(_, score)| *score >= 0.70))
    }

    async fn metadata_service(&self) -> Result<MetadataService> {
        let db_svc = self
            .manager
            .get_database()
            .await
            .ok_or_else(|| anyhow::anyhow!("Database service not available"))?;
        Ok(MetadataService::new_default(
            db_svc.pool().clone(),
            self.manager.clone(),
        ))
    }

    async fn reconcile_movie_collections_for_library(&self, library: &LibraryRow) {
        match (
            Uuid::parse_str(&library.id),
            Uuid::parse_str(&library.user_id),
        ) {
            (Ok(library_uuid), Ok(user_uuid)) => match self.metadata_service().await {
                Ok(metadata) => {
                    if let Err(error) = metadata
                        .ensure_movie_collections_for_library(library_uuid, user_uuid)
                        .await
                    {
                        warn!(
                            library_id = %library.id,
                            library_name = %library.name,
                            user_id = %library.user_id,
                            error = %error,
                            "Movie collection reconciliation failed for library scan workflow"
                        );
                    }
                }
                Err(error) => {
                    warn!(
                        library_id = %library.id,
                        library_name = %library.name,
                        user_id = %library.user_id,
                        error = %error,
                        "Metadata service unavailable for collection reconciliation in library scan workflow"
                    );
                }
            },
            (library_parse, user_parse) => {
                warn!(
                    library_id = %library.id,
                    user_id = %library.user_id,
                    library_id_parse_ok = library_parse.is_ok(),
                    user_id_parse_ok = user_parse.is_ok(),
                    "Skipping library scan collection reconciliation because library/user IDs are invalid UUIDs"
                );
            }
        }
    }

    fn movie_provider_fingerprint(match_source: &str) -> Option<String> {
        let hint = Self::parse_movie_hint(match_source);
        let title = hint.title?;
        if title.len() < 2 {
            return None;
        }
        let normalized = Self::normalize_for_match(&title);
        if normalized.is_empty() {
            return None;
        }
        Some(match hint.year {
            Some(year) => format!("{}|{}", normalized, year),
            None => normalized,
        })
    }

    fn tv_provider_fingerprint(match_source: &str) -> Option<String> {
        let hint = Self::parse_episode_hint(match_source);
        let show_name = hint.show_name?;
        if show_name.len() < 2 {
            return None;
        }
        let normalized = Self::normalize_for_match(&show_name);
        if normalized.is_empty() {
            return None;
        }
        Some(match hint.year {
            Some(year) => format!("{}|{}", normalized, year),
            None => normalized,
        })
    }

    fn auto_organize_guard_allows_move<E>(
        guard_result: &std::result::Result<Option<String>, E>,
    ) -> bool {
        matches!(guard_result, Ok(None))
    }

    fn auto_organize_result_is_success(result: &Result<OrganizeResult>) -> bool {
        matches!(result, Ok(outcome) if outcome.success)
    }

    fn sanitized_organize_failure_reason(reason: Option<&str>) -> String {
        match reason {
            Some("Media file has no library association yet")
            | Some("Source file does not exist")
            | Some("Media file is not matched to a library item")
            | Some(ARTIST_UNKNOWN_REASON)
            | Some("Target path already exists; skipped to avoid overwrite") => {
                reason.unwrap_or_default().to_string()
            }
            _ => "The organize operation could not produce a safe completed move. Review the server log for the correlated internal error.".to_string(),
        }
    }

    async fn maybe_auto_organize_after_scan_match(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        media_file_id: &str,
        media_path: &str,
        match_result: &MatchResult,
    ) {
        if !match_result.success || !library.auto_organize {
            return;
        }

        let guard_result = self
            .should_skip_auto_organize_for_match(
                library,
                auth_user,
                media_file_id,
                media_path,
                match_result,
            )
            .await;
        if !Self::auto_organize_guard_allows_move(&guard_result) {
            match guard_result {
                Ok(Some(reason)) => {
                    warn!(
                        library_id = %library.id,
                        library_name = %library.name,
                        media_file_id = %media_file_id,
                        media_file_path = %media_path,
                        matched_type = ?match_result.matched_type,
                        matched_id = ?match_result.matched_id,
                        confidence = match_result.confidence,
                        "{}",
                        reason
                    );
                    self.create_notification(
                        auth_user,
                        &library.user_id,
                        "WARNING",
                        "SCAN",
                        "Auto-organize skipped due to potential mismatch",
                        &reason,
                    )
                    .await;
                }
                Ok(None) => {
                    warn!(
                        library_id = %library.id,
                        library_name = %library.name,
                        media_file_id = %media_file_id,
                        media_file_path = %media_path,
                        "Auto-organize stopped because the safety guard returned no decision"
                    );
                }
                Err(error) => {
                    warn!(
                        library_id = %library.id,
                        library_name = %library.name,
                        media_file_id = %media_file_id,
                        media_file_path = %media_path,
                        matched_type = ?match_result.matched_type,
                        matched_id = ?match_result.matched_id,
                        error = %error,
                        "Auto-organize stopped because the safety guard could not be evaluated"
                    );
                    self.ensure_auto_organize_issue_notification(
                        auth_user,
                        library,
                        media_file_id,
                        media_path,
                        "Auto-organize safety check failed",
                        "safety validation",
                        "The safety validation could not be completed. The file was left in place; review the server log for the correlated internal error.",
                    )
                    .await;
                }
            }
            return;
        }

        let organize_result = self.move_media_file_to_pattern(media_file_id).await;
        if Self::auto_organize_result_is_success(&organize_result) {
            return;
        }

        let sanitized_reason = match &organize_result {
            Ok(result) => Self::sanitized_organize_failure_reason(result.reason.as_deref()),
            Err(_) => "The file operation or database update did not complete safely. Review the server log for the correlated internal error.".to_string(),
        };
        match &organize_result {
            Ok(result) => warn!(
                library_id = %library.id,
                library_name = %library.name,
                media_file_id = %media_file_id,
                media_file_path = %media_path,
                target_path = result.new_path.as_deref().unwrap_or(""),
                reason = result.reason.as_deref().unwrap_or("unspecified"),
                "Auto-organize completed without moving the matched file"
            ),
            Err(error) => error!(
                library_id = %library.id,
                library_name = %library.name,
                media_file_id = %media_file_id,
                media_file_path = %media_path,
                error = %error,
                "Auto-organize failed for a matched file"
            ),
        }
        self.ensure_auto_organize_issue_notification(
            auth_user,
            library,
            media_file_id,
            media_path,
            "Auto-organize failed",
            "organize matched media file",
            &sanitized_reason,
        )
        .await;
    }

    async fn process_provider_fallback_batch(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        pending: Vec<PendingProviderFallback>,
    ) -> ProviderFallbackSummary {
        let pending_files = pending.len();
        let mut summary = ProviderFallbackSummary {
            pending_files,
            ..Default::default()
        };
        if pending.is_empty() {
            return summary;
        }

        info!(
            library_id = %library.id,
            library_name = %library.name,
            library_type = %library.library_type,
            pending_files = pending.len(),
            "Starting deferred provider fallback batch for scan: library_id={}, library_name={}, library_type={}, pending_files={}",
            library.id,
            library.name,
            library.library_type,
            pending.len()
        );

        let normalized_library_type = Self::normalize_library_type(&library.library_type);
        let fallback_workers = self.config.analyze_workers.clamp(2, 8);

        match normalized_library_type.as_str() {
            "movies" => {
                match self.metadata_service().await {
                    Ok(metadata) if !metadata.has_tmdb().await => {
                        summary.unmatched_files = pending_files;
                        summary.configuration_blocked = true;
                        self.ensure_tmdb_scan_notification(
                            auth_user,
                            library,
                            summary.unmatched_files,
                        )
                        .await;
                        warn!(
                            library_id = %library.id,
                            library_name = %library.name,
                            affected_files = summary.unmatched_files,
                            "Movie provider fallback skipped because TMDB is not configured: library_id={}, library_name={}, affected_files={}. Configure a TMDB API key in Settings > Metadata and scan again",
                            library.id,
                            library.name,
                            summary.unmatched_files
                        );
                        return summary;
                    }
                    Ok(_) => {}
                    Err(error) => {
                        summary.unmatched_files = pending_files;
                        summary.error_files = pending_files;
                        warn!(
                            library_id = %library.id,
                            library_name = %library.name,
                            affected_files = pending_files,
                            error = %error,
                            "Movie provider fallback could not initialize metadata service: library_id={}, library_name={}, affected_files={}, error={}",
                            library.id,
                            library.name,
                            pending_files,
                            error
                        );
                        return summary;
                    }
                }

                let mut grouped: HashMap<String, Vec<PendingProviderFallback>> = HashMap::new();
                let mut skipped_no_fingerprint = 0usize;
                for item in pending {
                    if let Some(fingerprint) = Self::movie_provider_fingerprint(&item.match_source)
                    {
                        grouped.entry(fingerprint).or_default().push(item);
                    } else {
                        skipped_no_fingerprint += 1;
                    }
                }

                if skipped_no_fingerprint > 0 {
                    debug!(
                        library_id = %library.id,
                        skipped = skipped_no_fingerprint,
                        "Skipped deferred movie provider fallback for files with insufficient fingerprint hints: library_id={}, skipped={}",
                        library.id,
                        skipped_no_fingerprint
                    );
                    summary.unmatched_files += skipped_no_fingerprint;
                }

                let grouped_items: Vec<Vec<PendingProviderFallback>> =
                    grouped.into_values().collect();
                let outcomes = stream::iter(grouped_items)
                    .map(|group| async move {
                        let mut outcome = ProviderFallbackSummary::default();
                        if group.is_empty() {
                            return outcome;
                        }
                        let group_len = group.len();
                        let started = Instant::now();
                        let representative = &group[0];
                        match self
                            .try_provider_create_and_match_movie(
                                library,
                                auth_user,
                                &representative.media_file_id,
                                &representative.media_path,
                                &representative.match_source,
                            )
                            .await
                        {
                            Ok(Some(result))
                                if result.matched_type.as_deref() == Some("Movie")
                                    && result.matched_id.is_some() =>
                            {
                                self.maybe_auto_organize_after_scan_match(
                                    library,
                                    auth_user,
                                    &representative.media_file_id,
                                    &representative.media_path,
                                    &result,
                                )
                                .await;
                                outcome.matched_files = 1;

                                let movie_id = result.matched_id.clone().unwrap_or_default();
                                for sibling in group.iter().skip(1) {
                                    if let Err(error) = self
                                        .link_movie(
                                            auth_user,
                                            &sibling.media_file_id,
                                            &library.id,
                                            &movie_id,
                                            "auto",
                                            None,
                                        )
                                        .await
                                    {
                                        warn!(
                                            media_file_id = %sibling.media_file_id,
                                            media_path = %sibling.media_path,
                                            movie_id = %movie_id,
                                            error = %error,
                                            "Deferred provider fallback failed to link sibling media file: media_file_id={}, path={}, movie_id={}, error={}",
                                            sibling.media_file_id,
                                            sibling.media_path,
                                            movie_id,
                                            error
                                        );
                                        outcome.unmatched_files += 1;
                                        outcome.error_files += 1;
                                        continue;
                                    }
                                    outcome.matched_files += 1;
                                    let sibling_result = MatchResult {
                                        success: true,
                                        auto_matched: true,
                                        already_matched: false,
                                        matched_type: Some("Movie".to_string()),
                                        matched_id: Some(movie_id.clone()),
                                        confidence: result.confidence,
                                        reason: Some(
                                            "Linked by deferred grouped movie provider fallback"
                                                .to_string(),
                                        ),
                                        candidates: Vec::new(),
                                    };
                                    self.maybe_auto_organize_after_scan_match(
                                        library,
                                        auth_user,
                                        &sibling.media_file_id,
                                        &sibling.media_path,
                                        &sibling_result,
                                    )
                                    .await;
                                }
                            }
                            Ok(Some(_)) | Ok(None) => {
                                outcome.unmatched_files += group_len;
                            }
                            Err(error) => {
                                outcome.unmatched_files += group_len;
                                outcome.error_files += group_len;
                                warn!(
                                    media_file_id = %representative.media_file_id,
                                    media_path = %representative.media_path,
                                    error = %error,
                                    "Deferred movie provider fallback failed: media_file_id={}, path={}, error={}",
                                    representative.media_file_id,
                                    representative.media_path,
                                    error
                                );
                            }
                        }
                        info!(
                            media_file_id = %representative.media_file_id,
                            media_path = %representative.media_path,
                            grouped_files = group.len(),
                            elapsed_ms = started.elapsed().as_millis() as u64,
                            "Deferred movie provider fallback group processed: media_file_id={}, path={}, grouped_files={}, elapsed_ms={}",
                            representative.media_file_id,
                            representative.media_path,
                            group.len(),
                            started.elapsed().as_millis() as u64
                        );
                        outcome
                    })
                    .buffer_unordered(fallback_workers)
                    .collect::<Vec<_>>()
                    .await;
                for outcome in outcomes {
                    summary.merge_outcome(outcome);
                }
            }
            "tv" => {
                let mut grouped: HashMap<String, Vec<PendingProviderFallback>> = HashMap::new();
                let mut ungrouped = Vec::new();
                for item in pending {
                    if let Some(fingerprint) = Self::tv_provider_fingerprint(&item.match_source) {
                        grouped.entry(fingerprint).or_default().push(item);
                    } else {
                        ungrouped.push(item);
                    }
                }

                let mut groups: Vec<Vec<PendingProviderFallback>> = grouped.into_values().collect();
                groups.extend(ungrouped.into_iter().map(|item| vec![item]));

                let outcomes = stream::iter(groups)
                    .map(|group| async move {
                        let mut outcome = ProviderFallbackSummary::default();
                        for item in group {
                            match self
                                .try_provider_create_and_match_episode(
                                    library,
                                    auth_user,
                                    &item.media_file_id,
                                    &item.media_path,
                                    &item.match_source,
                                )
                                .await
                            {
                                Ok(Some(match_result)) => {
                                    if match_result.success {
                                        outcome.matched_files += 1;
                                        self.maybe_auto_organize_after_scan_match(
                                            library,
                                            auth_user,
                                            &item.media_file_id,
                                            &item.media_path,
                                            &match_result,
                                        )
                                        .await;
                                    } else {
                                        outcome.unmatched_files += 1;
                                    }
                                }
                                Ok(None) => {
                                    outcome.unmatched_files += 1;
                                }
                                Err(error) => {
                                    outcome.unmatched_files += 1;
                                    outcome.error_files += 1;
                                    warn!(
                                        media_file_id = %item.media_file_id,
                                        media_path = %item.media_path,
                                        error = %error,
                                        "Deferred TV provider fallback failed: media_file_id={}, path={}, error={}",
                                        item.media_file_id,
                                        item.media_path,
                                        error
                                    );
                                }
                            }
                        }
                        outcome
                    })
                    .buffer_unordered(fallback_workers)
                    .collect::<Vec<_>>()
                    .await;
                for outcome in outcomes {
                    summary.merge_outcome(outcome);
                }
            }
            "music" => {
                summary.merge_outcome(self.link_music_files(library, auth_user, &pending).await);
            }
            "audiobooks" => {
                let outcomes = stream::iter(pending)
                    .map(|item| {
                        async move {
                            let mut outcome = ProviderFallbackSummary::default();
                            let result = self
                                .try_provider_create_and_match_chapter(
                                    library,
                                    auth_user,
                                    &item.media_file_id,
                                    &item.media_path,
                                    &item.match_source,
                                )
                                .await;
                            match result {
                                Ok(Some(match_result)) => {
                                    if match_result.success {
                                        outcome.matched_files = 1;
                                        self.maybe_auto_organize_after_scan_match(
                                            library,
                                            auth_user,
                                            &item.media_file_id,
                                            &item.media_path,
                                            &match_result,
                                        )
                                        .await;
                                    } else {
                                        outcome.unmatched_files = 1;
                                    }
                                }
                                Ok(None) => {
                                    outcome.unmatched_files = 1;
                                }
                                Err(error) => {
                                    outcome.unmatched_files = 1;
                                    outcome.error_files = 1;
                                    warn!(
                                        media_file_id = %item.media_file_id,
                                        media_path = %item.media_path,
                                        error = %error,
                                        "Deferred provider fallback failed: media_file_id={}, path={}, error={}",
                                        item.media_file_id,
                                        item.media_path,
                                        error
                                    );
                                }
                            }
                            outcome
                        }
                    })
                    .buffer_unordered(fallback_workers)
                    .collect::<Vec<_>>()
                    .await;
                for outcome in outcomes {
                    summary.merge_outcome(outcome);
                }
            }
            _ => {
                summary.unmatched_files = pending_files;
            }
        }

        summary
    }

    async fn try_provider_create_and_match_movie(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        media_file_id: &str,
        media_path: &str,
        match_source: &str,
    ) -> Result<Option<MatchResult>> {
        let hint = Self::parse_movie_hint(match_source);
        let Some(title) = hint.title.as_ref().filter(|t| t.len() >= 2) else {
            return Ok(None);
        };

        let metadata = self.metadata_service().await?;
        let mut attempts: Vec<(String, Option<i32>)> = Vec::new();
        if let Some(year) = hint.year {
            attempts.push((title.to_string(), Some(year)));
        }
        attempts.push((title.to_string(), None));

        let mut candidates = Vec::new();
        let mut attempted_labels: Vec<String> = Vec::new();
        let mut successful_searches = 0usize;
        let mut provider_errors: Vec<String> = Vec::new();
        for (query, year) in &attempts {
            attempted_labels.push(match year {
                Some(y) => format!("query='{}',year={}", query, y),
                None => format!("query='{}'", query),
            });
            match metadata.search_movies(query, *year).await {
                Ok(found) => {
                    successful_searches += 1;
                    if !found.is_empty() {
                        candidates = found;
                        break;
                    }
                }
                Err(e) => {
                    provider_errors.push(e.to_string());
                    warn!(
                        media_file_id = %media_file_id,
                        library_id = %library.id,
                        media_path = %media_path,
                        title = %title,
                        attempted_query = %query,
                        attempted_year = ?year,
                        error = %e,
                        "Provider movie search attempt failed during scan fallback: media_file_id={}, library_id={}, path={}, parsed_title={}, attempted_query={}, attempted_year={:?}, error={}",
                        media_file_id,
                        library.id,
                        media_path,
                        title,
                        query,
                        year,
                        e
                    );
                }
            }
        }

        info!(
            media_file_id = %media_file_id,
            library_id = %library.id,
            media_path = %media_path,
            title = %title,
            parsed_year = ?hint.year,
            candidate_count = candidates.len(),
            "Movie provider search candidates: media_file_id={}, library_id={}, path={}, parsed_title={}, parsed_year={:?}, candidates={}",
            media_file_id,
            library.id,
            media_path,
            title,
            hint.year,
            candidates.len()
        );

        let wanted_norm = Self::normalize_for_match(title);
        let wanted_seq = Self::extract_title_sequence_number(title);
        let score_candidate = |provider_title: &str, provider_year: Option<i32>| -> f64 {
            let mut score = jaro_winkler(&wanted_norm, &Self::normalize_for_match(provider_title));
            if hint.year.is_some() && provider_year == hint.year {
                score += 0.1;
            } else if hint.year.is_some() && provider_year.is_some() {
                score -= 0.2;
            }
            let provider_seq = Self::extract_title_sequence_number(provider_title);
            if wanted_seq != provider_seq {
                score -= 0.2;
            }
            score
        };
        let best = candidates
            .into_iter()
            .map(|candidate| {
                let score = score_candidate(&candidate.title, candidate.year);
                (candidate, score)
            })
            .max_by(|(_, a_score), (_, b_score)| {
                a_score
                    .partial_cmp(b_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let Some((candidate, candidate_score)) = best else {
            if successful_searches == 0 && !provider_errors.is_empty() {
                anyhow::bail!(
                    "all movie metadata searches failed for '{}' ({} attempt(s)): {}",
                    title,
                    provider_errors.len(),
                    provider_errors.join(" | ")
                );
            }
            warn!(
                media_file_id = %media_file_id,
                library_id = %library.id,
                media_path = %media_path,
                match_source = %match_source,
                title = %title,
                parsed_year = ?hint.year,
                attempts = %attempted_labels.join(" | "),
                "Movie provider search produced no candidate to select: media_file_id={}, library_id={}, path={}, match_source={}, parsed_title={}, parsed_year={:?}, attempts={}",
                media_file_id,
                library.id,
                media_path,
                match_source,
                title,
                hint.year,
                attempted_labels.join(" | ")
            );
            return Ok(None);
        };

        if candidate_score < 0.78 {
            warn!(
                media_file_id = %media_file_id,
                library_id = %library.id,
                media_path = %media_path,
                match_source = %match_source,
                title = %title,
                parsed_year = ?hint.year,
                provider_id = candidate.provider_id,
                provider_title = %candidate.title,
                provider_year = ?candidate.year,
                candidate_score = candidate_score,
                "Movie provider candidate rejected due to low confidence: media_file_id={}, library_id={}, path={}, match_source={}, parsed_title={}, parsed_year={:?}, provider_id={}, provider_title={}, provider_year={:?}, candidate_score={:.3}",
                media_file_id,
                library.id,
                media_path,
                match_source,
                title,
                hint.year,
                candidate.provider_id,
                candidate.title,
                candidate.year,
                candidate_score
            );
            return Ok(None);
        }

        info!(
            media_file_id = %media_file_id,
            library_id = %library.id,
            media_path = %media_path,
            match_source = %match_source,
            provider_id = candidate.provider_id,
            provider_title = %candidate.title,
            provider_year = ?candidate.year,
            candidate_score = candidate_score,
            "Movie provider candidate selected: media_file_id={}, library_id={}, path={}, match_source={}, provider_id={}, provider_title={}, provider_year={:?}, candidate_score={:.3}",
            media_file_id,
            library.id,
            media_path,
            match_source,
            candidate.provider_id,
            candidate.title,
            candidate.year,
            candidate_score
        );

        let library_uuid = Uuid::parse_str(&library.id)?;
        let user_uuid = Uuid::parse_str(&library.user_id)?;
        let movie = metadata
            .add_movie_from_provider(AddMovieOptions {
                provider: MetadataProvider::Tmdb,
                provider_id: candidate.provider_id,
                library_id: library_uuid,
                user_id: user_uuid,
                monitored: true,
            })
            .await?;
        self.link_movie(
            auth_user,
            media_file_id,
            &library.id,
            &movie.id,
            "auto",
            None,
        )
        .await?;

        let confidence =
            jaro_winkler(&wanted_norm, &Self::normalize_for_match(&movie.title)).max(0.7);
        info!(
            media_file_id = %media_file_id,
            library_id = %library.id,
            media_path = %media_path,
            movie_id = %movie.id,
            movie_title = %movie.title,
            confidence,
            "Scan fallback created and linked movie from provider: media_file_id={}, library_id={}, path={}, movie_id={}, movie_title={}, confidence={:.3}",
            media_file_id,
            library.id,
            media_path,
            movie.id,
            movie.title,
            confidence
        );

        Ok(Some(MatchResult {
            success: true,
            auto_matched: true,
            already_matched: false,
            matched_type: Some("Movie".to_string()),
            matched_id: Some(movie.id),
            confidence,
            reason: Some("Created movie from metadata provider during scan fallback".to_string()),
            candidates: Vec::new(),
        }))
    }

    async fn try_provider_create_and_match_episode(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        media_file_id: &str,
        media_path: &str,
        match_source: &str,
    ) -> Result<Option<MatchResult>> {
        let hint = Self::parse_episode_hint(match_source);
        let Some(show_name) = hint.show_name.as_ref().filter(|s| s.len() >= 2) else {
            return Ok(None);
        };
        let (season, episode) = match (hint.season, hint.episode) {
            (Some(s), Some(e)) => (s, e),
            _ => return Ok(None),
        };

        let metadata = self.metadata_service().await?;
        let candidates = match metadata.search_tv_shows(show_name).await {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    media_file_id = %media_file_id,
                    library_id = %library.id,
                    media_path = %media_path,
                    show_name = %show_name,
                    error = %e,
                    "Provider TV search failed during scan fallback: media_file_id={}, library_id={}, path={}, parsed_show={}, error={}",
                    media_file_id,
                    library.id,
                    media_path,
                    show_name,
                    e
                );
                return Ok(None);
            }
        };

        let wanted_norm = Self::normalize_for_match(show_name);
        let score_candidate = |name: &str, year: Option<i32>| -> f64 {
            let mut s = jaro_winkler(&wanted_norm, &Self::normalize_for_match(name));
            if hint.year.is_some() && year == hint.year {
                s += 0.1;
            }
            s
        };
        let best = candidates
            .into_iter()
            .map(|candidate| {
                let score = score_candidate(&candidate.name, candidate.year);
                (candidate, score)
            })
            .max_by(|(_, a_score), (_, b_score)| {
                a_score
                    .partial_cmp(b_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let Some((candidate, candidate_score)) = best else {
            return Ok(None);
        };

        if candidate_score < 0.78 {
            debug!(
                media_file_id = %media_file_id,
                library_id = %library.id,
                media_path = %media_path,
                show_name = %show_name,
                provider_id = candidate.provider_id,
                provider_name = %candidate.name,
                candidate_score = candidate_score,
                "TV provider candidate rejected due to low confidence: media_file_id={}, library_id={}, path={}, parsed_show={}, provider_id={}, provider_name={}, candidate_score={:.3}",
                media_file_id,
                library.id,
                media_path,
                show_name,
                candidate.provider_id,
                candidate.name,
                candidate_score
            );
            return Ok(None);
        }

        let library_uuid = Uuid::parse_str(&library.id)?;
        let user_uuid = Uuid::parse_str(&library.user_id)?;
        let show = metadata
            .add_tv_show_from_provider(AddTvShowOptions {
                provider: MetadataProvider::Tvmaze,
                provider_id: candidate.provider_id,
                library_id: library_uuid,
                user_id: user_uuid,
                monitor_type: AutoDownloadMode::Wanted,
                path: None,
            })
            .await?;

        if let Some((episode_id, score)) = self.find_episode_match(&library.id, media_path).await? {
            self.link_episode(
                auth_user,
                media_file_id,
                &library.id,
                &episode_id,
                "auto",
                None,
            )
            .await?;
            info!(
                media_file_id = %media_file_id,
                library_id = %library.id,
                media_path = %media_path,
                show_id = %show.id,
                show_name = %show.name,
                season,
                episode,
                episode_id = %episode_id,
                confidence = score,
                "Scan fallback created show and linked episode: media_file_id={}, library_id={}, path={}, show_id={}, show_name={}, season={}, episode={}, episode_id={}, confidence={:.3}",
                media_file_id,
                library.id,
                media_path,
                show.id,
                show.name,
                season,
                episode,
                episode_id,
                score
            );
            return Ok(Some(MatchResult {
                success: true,
                auto_matched: true,
                already_matched: false,
                matched_type: Some("Episode".to_string()),
                matched_id: Some(episode_id),
                confidence: score,
                reason: Some("Created show from metadata provider and matched episode".to_string()),
                candidates: Vec::new(),
            }));
        }

        Ok(None)
    }

    async fn create_minimal_chapter_for_audiobook(
        &self,
        auth_user: &AuthUser,
        audiobook_id: &str,
        hint: &ParsedChapterHint,
    ) -> Result<String> {
        let chapter_number = hint.chapter_number.unwrap_or(1).max(1);
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation CreateChapterFromScan($input: CreateChapterInput!) {
                    CreateChapter: createChapter(input: $input) { Success: success Error: error Chapter: chapter { Id: id } }
                }"#,
                serde_json::json!({
                    "input": {
                        "audiobookId": audiobook_id,
                        "chapterNumber": chapter_number,
                        "title": hint.chapter_title.clone().unwrap_or_else(|| format!("Chapter: chapter {}", chapter_number)),
                        "startTimeSecs": 0.0,
                        "wanted": true
                    }
                }),
            )
            .await?;

        let success = data
            .get("CreateChapter")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !success {
            let err = data
                .get("CreateChapter")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("failed to create fallback chapter");
            anyhow::bail!(err.to_string());
        }

        data.get("CreateChapter")
            .and_then(|v| v.get("Chapter"))
            .and_then(|v| v.get("Id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("CreateChapter did not return Chapter.Id"))
    }

    async fn try_provider_create_and_match_track(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        media_file_id: &str,
        media_path: &str,
        match_source: &str,
    ) -> Result<Option<MatchResult>> {
        self.link_one_music_file(library, auth_user, media_file_id, media_path, match_source)
            .await
    }

    async fn stored_media_tags(&self, media_file_id: &str) -> release_match::FileTags {
        use crate::graphql::entities::MediaFile;
        let Ok(database) = self.database().await else {
            return Default::default();
        };
        let Ok(Some(file)) =
            MediaFile::get(database.pool().pool(), &media_file_id.to_string()).await
        else {
            return Default::default();
        };
        let Some(probe) = file
            .metadata
            .as_deref()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        else {
            return Default::default();
        };
        let audio_tags = probe
            .get("streams")
            .and_then(serde_json::Value::as_array)
            .and_then(|streams| {
                streams.iter().find(|stream| {
                    stream.get("codec_type").and_then(serde_json::Value::as_str) == Some("audio")
                })
            })
            .and_then(|stream| stream.get("tags"));
        let pairs = [probe.pointer("/format/tags"), audio_tags]
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_object)
            .flatten()
            .filter_map(|(key, value)| Some((key.as_str(), value.as_str()?)));
        release_match::tags_from_pairs(pairs)
    }

    /// What a download looks like against the library, before or after anything has moved. It
    /// shares its plan with the import, so what it shows is what importing would do.
    pub async fn torrent_match_preview(
        &self,
        auth_user: &AuthUser,
        info_hash: &str,
        files: Vec<SourceFileImport>,
    ) -> Result<TorrentMatchPreview> {
        let database = self.database().await?;
        let import =
            Self::authorize_torrent_import(database.pool(), auth_user, info_hash, None).await?;
        let owner = AuthUser::system_admin();
        let audio: Vec<&SourceFileImport> = files
            .iter()
            .filter(|file| Self::is_audio_import_path(&file.source_path))
            .collect();
        let video_count = files
            .iter()
            .filter(|file| Self::is_video_import_path(&file.source_path))
            .count();
        let link = &import.link;
        if let Some(book_id) = link.audiobook_id.as_deref() {
            return self.book_preview(book_id, &audio, &files).await;
        }
        if link.has_target() {
            let import_files: Vec<ImportFile> = files
                .iter()
                .cloned()
                .map(|file| ImportFile { file })
                .collect();
            let (usable, _) = Self::filter_import_candidates(import_files);
            let plan = self
                .plan_linked_import_targets(&owner, link, &usable, None)
                .await?;
            let mut targets_seen: HashMap<String, usize> = HashMap::new();
            for target in plan.targets.values() {
                *targets_seen.entry(target.target_id.clone()).or_default() += 1;
            }
            let previews = files
                .iter()
                .filter(|file| Self::is_supported_import_extension(&file.source_path))
                .map(|file| {
                    let target = plan.targets.get(&file.source_path);
                    PreviewFile {
                        path: file.source_path.clone(),
                        group_key: None,
                        status: if target.is_some() { "match" } else { "none" },
                        target: target.and_then(|target| target.target_name.clone()),
                        detail: plan.unmatched_reasons.get(&file.source_path).cloned(),
                        warning: target
                            .filter(|target| {
                                targets_seen.get(&target.target_id).copied().unwrap_or(0) > 1
                            })
                            .map(|_| "More than one file matches this".to_string()),
                    }
                })
                .collect::<Vec<_>>();
            let matched = plan.targets.len();
            let total = previews.len();
            return Ok(TorrentMatchPreview {
                status: if matched > 0 { "ready" } else { "review" },
                summary: Some(format!(
                    "{matched} of {total} files match {}",
                    plan.target_label
                        .as_deref()
                        .unwrap_or("the title it was grabbed for")
                )),
                complete_facts: files
                    .iter()
                    .all(|file| file.downloaded_bytes >= file.file_size),
                files: previews,
                ..Default::default()
            });
        }
        if audio.is_empty() || video_count > audio.len() {
            return Ok(TorrentMatchPreview {
                status: "none",
                ..Default::default()
            });
        }
        // Music is albums, not books: name each file's album, disc and track.
        if self
            .library_holds_music(database.pool(), link.library_id.as_deref())
            .await
        {
            let mut facts = self.release_facts_for(&audio).await;
            music_rules::fill_in_discs(&mut facts);
            let held = self.held_albums(&import.owner_id).await?;
            return Ok(music::describe_music_release(&facts, &held));
        }
        // Linked to nothing: see whether its files look like books already in the library.
        self.unlinked_audio_preview(&import.owner_id, &audio, &files)
            .await
    }

    /// Whether a torrent's library is a music library, so its audio is albums and not books.
    async fn library_holds_music(
        &self,
        db: &crate::db::Database,
        library_id: Option<&str>,
    ) -> bool {
        let Some(id) = library_id else {
            return false;
        };
        match crate::graphql::entities::Library::get(db.pool(), &id.to_string()).await {
            Ok(Some(library)) => Self::normalize_library_type(&library.library_type) == "music",
            _ => false,
        }
    }

    async fn release_facts_for(
        &self,
        audio: &[&SourceFileImport],
    ) -> Vec<release_match::ReleaseFile> {
        self.release_file_facts(
            audio
                .iter()
                .map(|file| {
                    (
                        file.source_path.clone(),
                        file.relative_path.clone(),
                        file.file_size,
                        file.file_size > 0 && file.downloaded_bytes >= file.file_size,
                    )
                })
                .collect(),
        )
        .await
    }

    async fn book_preview(
        &self,
        book_id: &str,
        audio: &[&SourceFileImport],
        all_files: &[SourceFileImport],
    ) -> Result<TorrentMatchPreview> {
        let (book, _, facts, reference) = self.book_reference(book_id).await?;
        if audio.is_empty() {
            let names: Vec<String> = all_files
                .iter()
                .map(|file| file.relative_path.clone())
                .collect();
            return Ok(TorrentMatchPreview {
                status: "review",
                summary: release_match::payload_mismatch(true, false, &names)
                    .map(|text| text.replace(" Nothing was downloaded.", "")),
                ..Default::default()
            });
        }
        let release_files = self.release_facts_for(audio).await;
        let plan = self
            .plan_book_release(&book.id, &release_files, &facts, &reference)
            .await;
        Ok(Self::describe_book_plan(
            &plan,
            &release_files,
            &facts,
            &book.id,
            &book.title,
        ))
    }

    fn describe_book_plan(
        plan: &release_match::ReleasePlan,
        files: &[release_match::ReleaseFile],
        facts: &release_match::BookFacts,
        book_id: &str,
        book_title: &str,
    ) -> TorrentMatchPreview {
        let (layout, reason) = match &plan.outcome {
            release_match::BookOutcome::Ready(layout) => (Some(layout), None),
            release_match::BookOutcome::Review(review) => {
                (review.fallback.as_ref(), Some(review.reason.clone()))
            }
        };
        let groups = plan
            .groups
            .iter()
            .enumerate()
            .map(|(index, group)| PreviewGroup {
                key: group.key.clone(),
                label: group.label.clone(),
                file_count: group.files.len() as i32,
                seconds: group
                    .files
                    .iter()
                    .map(|file| files[*file].duration)
                    .sum::<Option<f64>>(),
                is_target: Some(index) == plan.chosen,
                book_id: (Some(index) == plan.chosen).then(|| book_id.to_string()),
                book_title: (Some(index) == plan.chosen).then(|| book_title.to_string()),
                author: group.files.first().and_then(|file| {
                    let tags = &files[*file].tags;
                    tags.album_artist.clone().or_else(|| tags.artist.clone())
                }),
            })
            .collect();
        let mut previews = Vec::with_capacity(files.len());
        for (index, group) in plan.groups.iter().enumerate() {
            let chosen = Some(index) == plan.chosen;
            let held = layout
                .filter(|_| chosen)
                .map(|layout| release_match::chapter_of_each_file(layout, group.files.len()));
            for (position, file) in group.files.iter().enumerate() {
                let (status, target, detail) = if chosen {
                    let detail = layout.map(|layout| match layout.shape {
                        release_match::FileShape::WholeBook => {
                            format!("Holds all {} chapters", layout.chapters.len())
                        }
                        release_match::FileShape::PerChapter => held
                            .as_ref()
                            .and_then(|held| held[position])
                            .and_then(|chapter| layout.chapters.get(chapter))
                            .map(|chapter| chapter.title.clone())
                            .unwrap_or_default(),
                        release_match::FileShape::Parts => {
                            let starting: Vec<&str> = layout
                                .chapters
                                .iter()
                                .filter(|chapter| chapter.file == Some(position))
                                .map(|chapter| chapter.title.as_str())
                                .collect();
                            let part = format!("Part {} of {}", position + 1, group.files.len());
                            match starting.as_slice() {
                                [] => format!("{part}, inside a longer chapter"),
                                [only] => format!("{part}, starts {only}"),
                                [first, .., last] => format!("{part}, starts {first} to {last}"),
                            }
                        }
                    });
                    (
                        if reason.is_some() { "review" } else { "match" },
                        Some(book_title.to_string()),
                        detail,
                    )
                } else {
                    (
                        "other",
                        Some(group.label.clone()),
                        Some("Another book in this release".to_string()),
                    )
                };
                previews.push(PreviewFile {
                    path: files[*file].key.clone(),
                    group_key: Some(group.key.clone()),
                    status,
                    target,
                    detail,
                    warning: None,
                });
            }
        }
        TorrentMatchPreview {
            status: if reason.is_some() { "review" } else { "ready" },
            summary: Some(release_match::describe(plan, facts)),
            reason,
            can_accept: matches!(&plan.outcome, release_match::BookOutcome::Review(review) if review.fallback.is_some()),
            chapter_source: layout.map(|layout| layout.source.as_str().to_string()),
            file_shape: layout.map(|layout| layout.shape.as_str().to_string()),
            evidence: plan
                .evidence
                .iter()
                .chain(layout.iter().flat_map(|layout| layout.evidence.iter()))
                .cloned()
                .collect(),
            complete_facts: plan.complete_facts,
            groups,
            files: previews,
        }
    }

    async fn unlinked_audio_preview(
        &self,
        owner_id: &str,
        audio: &[&SourceFileImport],
        all_files: &[SourceFileImport],
    ) -> Result<TorrentMatchPreview> {
        use crate::graphql::entities::{Audiobook, AudiobookWhereInput};
        let database = self.database().await?;
        let books = Audiobook::query(database.pool().pool())
            .filter(AudiobookWhereInput {
                user_id: Some(StringFilter {
                    eq: Some(owner_id.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        let release_files = self.release_facts_for(audio).await;
        let groups = release_match::group_files(&release_files);
        if groups.is_empty() {
            return Ok(TorrentMatchPreview {
                status: "none",
                ..Default::default()
            });
        }
        // The first book it looks like gets the full plan. The others are named on their groups.
        let mut known: HashMap<usize, (&Audiobook, f64)> = HashMap::new();
        for (index, group) in groups.iter().enumerate() {
            for book in &books {
                let facts = release_match::BookFacts {
                    title: book.title.clone(),
                    author: book.author_name.clone(),
                    ..Default::default()
                };
                let album = group
                    .files
                    .first()
                    .and_then(|file| release_files[*file].tags.album.as_deref())
                    .unwrap_or(group.label.as_str());
                let score = release_match::title_score(&group.label, &facts)
                    .max(release_match::title_score(album, &facts));
                if score >= 0.75 && known.get(&index).is_none_or(|(_, best)| score > *best) {
                    known.insert(index, (book, score));
                }
            }
        }
        let mut preview = match known.iter().min_by_key(|(index, _)| **index) {
            Some((_, (book, _))) => {
                let mut preview = self.book_preview(&book.id, audio, all_files).await?;
                preview.status = "unlinked";
                preview
            }
            None => TorrentMatchPreview {
                status: "unlinked",
                summary: Some(format!(
                    "{} {} not in the library.",
                    groups.len(),
                    if groups.len() == 1 { "book" } else { "books" }
                )),
                complete_facts: release_files.iter().all(|file| file.duration.is_some()),
                groups: groups
                    .iter()
                    .map(|group| PreviewGroup {
                        key: group.key.clone(),
                        label: group.label.clone(),
                        file_count: group.files.len() as i32,
                        ..Default::default()
                    })
                    .collect(),
                files: Vec::new(),
                ..Default::default()
            },
        };
        for group in &mut preview.groups {
            let index = groups
                .iter()
                .position(|known_group| known_group.key == group.key);
            if let Some((book, _)) = index.and_then(|index| known.get(&index)) {
                group.book_id = Some(book.id.clone());
                group.book_title = Some(book.title.clone());
            }
            if group.author.is_none()
                && let Some(found) = index.map(|index| &groups[index])
            {
                let tags = found.files.first().map(|file| &release_files[*file].tags);
                group.author =
                    tags.and_then(|tags| tags.album_artist.clone().or_else(|| tags.artist.clone()));
            }
        }
        if preview.files.is_empty() {
            for group in &groups {
                for file in &group.files {
                    preview.files.push(PreviewFile {
                        path: release_files[*file].key.clone(),
                        group_key: Some(group.key.clone()),
                        status: "other",
                        target: Some(group.label.clone()),
                        detail: None,
                        warning: None,
                    });
                }
            }
        }
        Ok(preview)
    }

    /// Put the provider's chapters back after a release's own replaced them, placed on the book's
    /// files by time where those files allow it.
    pub async fn restore_provider_chapters(&self, audiobook_id: &str) -> Result<()> {
        let owner = AuthUser::system_admin();
        let (book, chapters, facts, reference) = self.book_reference(audiobook_id).await?;
        anyhow::ensure!(
            !reference.is_empty(),
            "This book has no provider chapters to restore"
        );
        let files = self.book_files_in_order(&chapters).await?;
        let durations: Vec<f64> = files
            .iter()
            .map(|file| file.duration.map_or(0.0, f64::from))
            .collect();
        let layout = release_match::provider_layout(&durations, &reference, facts.seconds);
        let new_ids = self
            .rewrite_book_chapters(&owner, &book, &chapters, &reference, &layout, true)
            .await?;
        self.relink_book_files(&files, &layout, &new_ids).await?;
        let placements = layout
            .chapters
            .iter()
            .enumerate()
            .filter_map(|(index, chapter)| {
                Some((
                    new_ids.get(index)?.clone(),
                    files.get(chapter.file?)?.id.clone(),
                    chapter.offset,
                ))
            })
            .collect();
        let imported: HashMap<String, String> = files
            .iter()
            .map(|file| (file.id.clone(), file.id.clone()))
            .collect();
        self.write_book_chapters(
            &owner,
            &BookWrites {
                audiobook_id: book.id.clone(),
                placements,
                source: "provider",
                shape: layout.shape.as_str(),
            },
            &imported,
        )
        .await?;
        self.update_audiobook_fields(
            &owner,
            &book.id,
            serde_json::json!({
                "providerChapters": serde_json::Value::Null,
                "chapterCheckState": serde_json::Value::Null,
                "chapterCheck": serde_json::Value::Null,
            }),
        )
        .await
    }

    async fn try_provider_create_and_match_chapter(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        media_file_id: &str,
        media_path: &str,
        match_source: &str,
    ) -> Result<Option<MatchResult>> {
        let mut hint = Self::parse_chapter_hint(match_source);
        // What a file says about itself beats what its folders suggest.
        let tags = self.stored_media_tags(media_file_id).await;
        if let Some(album) = tags
            .album
            .as_deref()
            .filter(|album| !album.trim().is_empty())
        {
            hint.audiobook_title = Some(album.to_string());
        }
        if let Some(author) = tags.album_artist.as_deref().or(tags.artist.as_deref()) {
            hint.author_name = Some(author.to_string());
        }
        let query = hint
            .audiobook_title
            .clone()
            .or_else(|| hint.chapter_title.clone())
            .filter(|q| q.len() >= 2);
        let Some(query) = query else {
            return Ok(None);
        };

        let metadata = self.metadata_service().await?;
        let candidates = metadata.search_audiobooks(&query).await.unwrap_or_default();
        let wanted_title_norm = hint
            .audiobook_title
            .as_deref()
            .map(Self::normalize_for_match)
            .unwrap_or_else(|| Self::normalize_for_match(&query));
        let wanted_author_norm = hint.author_name.as_deref().map(Self::normalize_for_match);

        let score_candidate = |title: &str, author: Option<&str>| -> f64 {
            let mut s = jaro_winkler(&wanted_title_norm, &Self::normalize_for_match(title));
            if let (Some(wanted_author), Some(candidate_author)) =
                (wanted_author_norm.as_deref(), author)
            {
                s += (jaro_winkler(wanted_author, &Self::normalize_for_match(candidate_author))
                    * 0.25)
                    .min(0.25);
            }
            s
        };
        let best = candidates
            .into_iter()
            .map(|candidate| {
                let score = score_candidate(&candidate.title, candidate.author_name.as_deref());
                (candidate, score)
            })
            .max_by(|(_, a_score), (_, b_score)| {
                a_score
                    .partial_cmp(b_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let Some((candidate, candidate_score)) = best else {
            return Ok(None);
        };

        if candidate_score < 0.78 {
            debug!(
                media_file_id = %media_file_id,
                library_id = %library.id,
                media_path = %media_path,
                provider_id = candidate.provider_id,
                provider_title = %candidate.title,
                candidate_score = candidate_score,
                "Audiobook provider candidate rejected due to low confidence: media_file_id={}, library_id={}, path={}, provider_id={}, provider_title={}, candidate_score={:.3}",
                media_file_id,
                library.id,
                media_path,
                candidate.provider_id,
                candidate.title,
                candidate_score
            );
            return Ok(None);
        }

        let library_uuid = Uuid::parse_str(&library.id)?;
        let user_uuid = Uuid::parse_str(&library.user_id)?;
        let audiobook = metadata
            .add_audiobook_from_provider(AddAudiobookOptions {
                provider: crate::services::metadata::providers::audiobook_provider_for_id(
                    &candidate.provider_id,
                ),
                provider_id: candidate.provider_id,
                library_id: library_uuid,
                user_id: user_uuid,
                monitor_type: AutoDownloadMode::None,
            })
            .await?;

        if let Some((chapter_id, score)) = self.find_chapter_match(&library.id, media_path).await? {
            self.link_chapter(
                auth_user,
                media_file_id,
                &library.id,
                &chapter_id,
                "auto",
                None,
            )
            .await?;
            return Ok(Some(MatchResult {
                success: true,
                auto_matched: true,
                already_matched: false,
                matched_type: Some("Chapter".to_string()),
                matched_id: Some(chapter_id),
                confidence: score,
                reason: Some(
                    "Created audiobook from metadata provider and matched chapter".to_string(),
                ),
                candidates: Vec::new(),
            }));
        }

        // A file as long as the whole book belongs to the chapters the book already has.
        let chapters = self.book_chapters(&audiobook.id).await?;
        let first_chapter = chapters
            .iter()
            .filter(|chapter| chapter.media_file_id.is_none())
            .min_by_key(|chapter| chapter.chapter_number)
            .map(|chapter| chapter.id.clone());
        if let Some(first) = first_chapter.filter(|_| {
            chapters.len() > 1
                && chapters
                    .iter()
                    .all(|chapter| chapter.media_file_id.is_none())
        }) && self
            .file_is_whole_book(&audiobook.id, &chapters, media_file_id)
            .await?
        {
            self.link_chapter(auth_user, media_file_id, &library.id, &first, "auto", None)
                .await?;
            self.link_whole_book(auth_user, &first, media_file_id)
                .await?;
            return Ok(Some(MatchResult {
                success: true,
                auto_matched: true,
                already_matched: false,
                matched_type: Some("Chapter".to_string()),
                matched_id: Some(first),
                confidence: candidate_score.min(1.0),
                reason: Some("Linked the book's one file to all of its chapters".to_string()),
                candidates: Vec::new(),
            }));
        }

        let chapter_id = self
            .create_minimal_chapter_for_audiobook(auth_user, &audiobook.id, &hint)
            .await?;
        self.link_chapter(
            auth_user,
            media_file_id,
            &library.id,
            &chapter_id,
            "auto",
            None,
        )
        .await?;
        let confidence = 0.7;
        info!(
            media_file_id = %media_file_id,
            library_id = %library.id,
            media_path = %media_path,
            audiobook_id = %audiobook.id,
            audiobook_title = %audiobook.title,
            chapter_id = %chapter_id,
            confidence,
            "Scan fallback created audiobook and fallback chapter link: media_file_id={}, library_id={}, path={}, audiobook_id={}, audiobook_title={}, chapter_id={}, confidence={:.3}",
            media_file_id,
            library.id,
            media_path,
            audiobook.id,
            audiobook.title,
            chapter_id,
            confidence
        );
        Ok(Some(MatchResult {
            success: true,
            auto_matched: true,
            already_matched: false,
            matched_type: Some("Chapter".to_string()),
            matched_id: Some(chapter_id),
            confidence,
            reason: Some("Created audiobook and fallback chapter during scan".to_string()),
            candidates: Vec::new(),
        }))
    }

    async fn find_chapter_match(
        &self,
        library_id: &str,
        path: &str,
    ) -> Result<Option<(String, f64)>> {
        let auth_user = self.system_auth_user(None).await?;

        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);

        let normalized = Self::normalize_for_match(file_name);
        let chapter_re = Regex::new(r"(?i)\b(ch(?:apter)?\s*|track\s*)(\d{1,3})\b")?;
        let chapter_num = chapter_re
            .captures(file_name)
            .and_then(|c| c.get(2))
            .and_then(|m| m.as_str().parse::<i32>().ok());

        let data = self
            .execute_graphql_paged(
                &auth_user,
                r#"query FindChapterMatch($libraryId: String!, $page: PageInput) {
                    Audiobooks: audiobooks(
                        where: { libraryId: { eq: $libraryId } }
                        page: $page
                    ) {
                        Edges: edges {
                            Node: node {
                                Title: title
                                AuthorName: authorName
                                Chapters: chapters(page: { limit: 1000 }) {
                                    Edges: edges {
                                        Node: node {
                                            Id: id
                                            Title: title
                                            ChapterNumber: chapterNumber
                                        }
                                    }
                                }
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "libraryId": library_id }),
                "Audiobooks",
            )
            .await?;
        type ChapterMatchRow = (String, Option<String>, i32, String, Option<String>);
        let rows: Vec<ChapterMatchRow> = data
            .get("Audiobooks")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .flat_map(|edge| {
                let book = edge.get("Node")?;
                let book_title = book.get("Title")?.as_str()?.to_string();
                let author_name = book
                    .get("AuthorName")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let chapters = book
                    .get("Chapters")
                    .and_then(|v| v.get("Edges"))
                    .and_then(|v| v.as_array())?;
                Some(
                    chapters
                        .iter()
                        .filter_map(move |chapter_edge| {
                            let chapter = chapter_edge.get("Node")?;
                            Some((
                                chapter.get("Id")?.as_str()?.to_string(),
                                chapter
                                    .get("Title")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                chapter.get("ChapterNumber")?.as_i64()? as i32,
                                book_title.clone(),
                                author_name.clone(),
                            ))
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect();

        let mut best: Option<(String, f64)> = None;
        for (id, chapter_title, chapter_number, book_title, author_name) in rows {
            let mut score = 0.0;
            if let Some(num) = chapter_num
                && num == chapter_number
            {
                score += 0.5;
            }

            let book_norm = Self::normalize_for_match(&book_title);
            if normalized.contains(&book_norm) {
                score += 0.25;
            }
            if let Some(author) = author_name {
                let author_norm = Self::normalize_for_match(&author);
                if normalized.contains(&author_norm) {
                    score += 0.15;
                }
            }
            if let Some(ct) = chapter_title {
                let title_norm = Self::normalize_for_match(&ct);
                score += (jaro_winkler(&normalized, &title_norm) * 0.1).min(0.1);
            }

            if score > best.as_ref().map(|(_, s)| *s).unwrap_or(0.0) {
                best = Some((id, score.min(1.0)));
            }
        }

        Ok(best.filter(|(_, s)| *s >= 0.6))
    }

    fn adjust_candidate_score(
        base_score: f64,
        wanted: bool,
        has_existing_file: bool,
        wanted_policy: MatchWantedPolicy,
    ) -> f64 {
        let mut score = base_score;
        if matches!(wanted_policy, MatchWantedPolicy::PreferWanted) {
            if wanted {
                score += 0.05;
            }
            if has_existing_file && !wanted {
                score -= 0.08;
            }
        }
        score.clamp(0.0, 1.0)
    }

    fn should_include_candidate(wanted: bool, wanted_policy: MatchWantedPolicy) -> bool {
        match wanted_policy {
            MatchWantedPolicy::PreferWanted | MatchWantedPolicy::All => true,
            MatchWantedPolicy::WantedOnly => wanted,
        }
    }

    fn normalized_candidate_limit(limit: usize) -> usize {
        if limit == 0 { 10 } else { limit.min(50) }
    }

    fn sort_and_trim_candidates(
        mut candidates: Vec<MatchCandidate>,
        limit: usize,
    ) -> Vec<MatchCandidate> {
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(Self::normalized_candidate_limit(limit));
        candidates
    }

    async fn clear_candidate_caches_for_library(&self, library_id: &str) {
        self.movie_candidate_cache.write().await.remove(library_id);
        self.episode_candidate_cache
            .write()
            .await
            .remove(library_id);
        self.track_candidate_cache.write().await.remove(library_id);
        self.chapter_candidate_cache
            .write()
            .await
            .remove(library_id);
    }

    async fn get_or_load_movie_candidate_rows(
        &self,
        auth_user: &AuthUser,
        library_id: &str,
    ) -> Result<Vec<MovieCandidateRow>> {
        if let Some(rows) = self
            .movie_candidate_cache
            .read()
            .await
            .get(library_id)
            .cloned()
        {
            return Ok(rows);
        }

        let data = self
            .execute_graphql_paged(
                auth_user,
                r#"query MatchMovieCandidates($libraryId: String!, $page: PageInput) {
                    Movies: movies(
                        where: { libraryId: { eq: $libraryId } }
                        page: $page
                    ) {
                        Edges: edges { Node: node { Id: id Title: title Year: year Wanted: wanted HasFile: hasFile } }
                    }
                }"#,
                serde_json::json!({ "libraryId": library_id }),
                "Movies",
            )
            .await?;

        let mut rows = Vec::new();
        for edge in data
            .get("Movies")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            let Some(node) = edge.get("Node") else {
                continue;
            };
            let Some(id) = node.get("Id").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(title) = node.get("Title").and_then(|v| v.as_str()) else {
                continue;
            };

            rows.push(MovieCandidateRow {
                id: id.to_string(),
                title: title.to_string(),
                year: node.get("Year").and_then(|v| v.as_i64()).map(|v| v as i32),
                wanted: node
                    .get("Wanted")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                has_file: node
                    .get("HasFile")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            });
        }

        self.movie_candidate_cache
            .write()
            .await
            .insert(library_id.to_string(), rows.clone());

        Ok(rows)
    }

    async fn get_or_load_episode_candidate_rows(
        &self,
        auth_user: &AuthUser,
        library_id: &str,
    ) -> Result<Vec<EpisodeCandidateRow>> {
        if let Some(rows) = self
            .episode_candidate_cache
            .read()
            .await
            .get(library_id)
            .cloned()
        {
            return Ok(rows);
        }

        let data = self
            .execute_graphql_paged(
                auth_user,
                r#"query MatchEpisodeCandidates($libraryId: String!, $page: PageInput) {
                    Shows: shows(
                        where: { libraryId: { eq: $libraryId } }
                        page: $page
                    ) {
                        Edges: edges {
                            Node: node {
                                Id: id
                                Name: name
                                Year: year
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "libraryId": library_id }),
                "Shows",
            )
            .await?;

        let mut shows = HashMap::new();
        for show_edge in data
            .get("Shows")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            let Some(show) = show_edge.get("Node") else {
                continue;
            };
            let Some(show_id) = show.get("Id").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(show_name) = show.get("Name").and_then(|v| v.as_str()) else {
                continue;
            };
            let show_year = show.get("Year").and_then(|v| v.as_i64()).map(|v| v as i32);
            shows.insert(show_id.to_string(), (show_name.to_string(), show_year));
        }
        let show_ids = shows.keys().cloned().collect::<Vec<_>>();
        let rows = self
            .load_episodes_for_show_ids(&show_ids)
            .await?
            .into_iter()
            .filter_map(|episode| {
                let (show_name, show_year) = shows.get(&episode.show_id)?;
                Some(EpisodeCandidateRow {
                    id: episode.id,
                    show_name: show_name.clone(),
                    show_year: *show_year,
                    season: episode.season,
                    episode: episode.episode,
                    wanted: episode.wanted,
                    has_file: episode.media_file_id.is_some(),
                })
            })
            .collect::<Vec<_>>();

        self.episode_candidate_cache
            .write()
            .await
            .insert(library_id.to_string(), rows.clone());

        Ok(rows)
    }

    async fn get_or_load_track_candidate_rows(
        &self,
        auth_user: &AuthUser,
        library_id: &str,
    ) -> Result<Vec<TrackCandidateRow>> {
        if let Some(rows) = self
            .track_candidate_cache
            .read()
            .await
            .get(library_id)
            .cloned()
        {
            return Ok(rows);
        }

        let data = self
            .execute_graphql_paged(
                auth_user,
                r#"query MatchTrackCandidates($libraryId: String!, $page: PageInput) {
                    Albums: albums(
                        where: { libraryId: { eq: $libraryId } }
                        page: $page
                    ) {
                        Edges: edges {
                            Node: node {
                                Id: id
                                Name: name
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "libraryId": library_id }),
                "Albums",
            )
            .await?;

        let mut albums = HashMap::new();
        for album_edge in data
            .get("Albums")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            let Some(album) = album_edge.get("Node") else {
                continue;
            };
            let Some(album_id) = album.get("Id").and_then(|v| v.as_str()) else {
                continue;
            };
            let album_name = album
                .get("Name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            albums.insert(album_id.to_string(), album_name);
        }
        let rows = self
            .load_tracks_for_library(library_id)
            .await?
            .into_iter()
            .filter_map(|track| {
                Some(TrackCandidateRow {
                    id: track.id,
                    title: track.title,
                    album_name: albums.get(&track.album_id)?.clone(),
                    wanted: track.wanted,
                    has_file: track.media_file_id.is_some(),
                })
            })
            .collect::<Vec<_>>();

        self.track_candidate_cache
            .write()
            .await
            .insert(library_id.to_string(), rows.clone());

        Ok(rows)
    }

    async fn get_or_load_chapter_candidate_rows(
        &self,
        auth_user: &AuthUser,
        library_id: &str,
    ) -> Result<Vec<ChapterCandidateRow>> {
        if let Some(rows) = self
            .chapter_candidate_cache
            .read()
            .await
            .get(library_id)
            .cloned()
        {
            return Ok(rows);
        }

        let data = self
            .execute_graphql_paged(
                auth_user,
                r#"query MatchChapterCandidates($libraryId: String!, $page: PageInput) {
                    Audiobooks: audiobooks(
                        where: { libraryId: { eq: $libraryId } }
                        page: $page
                    ) {
                        Edges: edges {
                            Node: node {
                                Id: id
                                Title: title
                                AuthorName: authorName
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "libraryId": library_id }),
                "Audiobooks",
            )
            .await?;

        let mut audiobooks = HashMap::new();
        for book_edge in data
            .get("Audiobooks")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            let Some(book) = book_edge.get("Node") else {
                continue;
            };
            let Some(audiobook_id) = book.get("Id").and_then(|v| v.as_str()) else {
                continue;
            };
            let audiobook_title = book
                .get("Title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let author_name = book
                .get("AuthorName")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            audiobooks.insert(audiobook_id.to_string(), (audiobook_title, author_name));
        }
        let audiobook_ids = audiobooks.keys().cloned().collect::<Vec<_>>();
        let rows = self
            .load_chapters_for_audiobook_ids(&audiobook_ids)
            .await?
            .into_iter()
            .filter_map(|chapter| {
                let (audiobook_title, author_name) = audiobooks.get(&chapter.audiobook_id)?;
                Some(ChapterCandidateRow {
                    id: chapter.id,
                    title: chapter.title.unwrap_or_default(),
                    audiobook_title: audiobook_title.clone(),
                    author_name: author_name.clone(),
                    chapter_number: Some(chapter.chapter_number),
                    wanted: chapter.wanted,
                    has_file: chapter.media_file_id.is_some(),
                })
            })
            .collect::<Vec<_>>();

        self.chapter_candidate_cache
            .write()
            .await
            .insert(library_id.to_string(), rows.clone());

        Ok(rows)
    }

    async fn collect_movie_candidates(
        &self,
        library_id: &str,
        path: &str,
        limit: usize,
        wanted_policy: MatchWantedPolicy,
    ) -> Result<Vec<MatchCandidate>> {
        let auth_user = self.system_auth_user(None).await?;
        let rows = self
            .get_or_load_movie_candidate_rows(&auth_user, library_id)
            .await?;
        let hint = Self::parse_movie_hint(path);
        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        let normalized = hint
            .title
            .as_deref()
            .map(Self::normalize_for_match)
            .unwrap_or_else(|| Self::normalize_for_match(file_name));
        let hint_seq = hint
            .title
            .as_deref()
            .and_then(Self::extract_title_sequence_number);

        let mut out = Vec::new();
        for row in rows {
            let target_id = row.id;
            let title = row.title;
            let year = row.year;
            let wanted = row.wanted;
            let has_file = row.has_file;
            if !Self::should_include_candidate(wanted, wanted_policy) {
                continue;
            }

            let title_norm = Self::normalize_for_match(&title);
            let mut score = if normalized.contains(&title_norm) || title_norm.contains(&normalized)
            {
                0.85
            } else {
                jaro_winkler(&normalized, &title_norm)
            };

            if let Some(y) = year {
                if hint.year == Some(y) {
                    score += 0.1;
                } else if hint.year.is_some() {
                    score -= 0.35;
                }
            }
            let title_seq = Self::extract_title_sequence_number(&title);
            if hint_seq != title_seq {
                score -= 0.25;
            }

            let adjusted =
                Self::adjust_candidate_score(score.min(1.0), wanted, has_file, wanted_policy);
            if adjusted >= 0.4 {
                let display_name = match year {
                    Some(y) => format!("{} ({})", title, y),
                    None => title.to_string(),
                };
                out.push(MatchCandidate {
                    target_type: "Movie".to_string(),
                    target_id,
                    target_name: Some(display_name),
                    score: adjusted,
                    reason: Some("movie title/year heuristic".to_string()),
                    wanted: Some(wanted),
                });
            }
        }

        Ok(Self::sort_and_trim_candidates(out, limit))
    }

    async fn collect_episode_candidates(
        &self,
        library_id: &str,
        path: &str,
        limit: usize,
        wanted_policy: MatchWantedPolicy,
    ) -> Result<Vec<MatchCandidate>> {
        let auth_user = self.system_auth_user(None).await?;
        let rows = self
            .get_or_load_episode_candidate_rows(&auth_user, library_id)
            .await?;
        let hint = Self::parse_episode_hint(path);
        let (season, episode) = match (hint.season, hint.episode) {
            (Some(s), Some(e)) => (s, e),
            _ => return Ok(Vec::new()),
        };
        let normalized = hint
            .show_name
            .as_deref()
            .map(Self::normalize_for_match)
            .unwrap_or_else(|| Self::normalize_for_match(path));

        let mut out = Vec::new();
        for row in rows {
            if row.season != season || row.episode != episode {
                continue;
            }
            let target_id = row.id;
            let show_name = row.show_name;
            let show_year = row.show_year;
            let wanted = row.wanted;
            let has_file = row.has_file;
            if !Self::should_include_candidate(wanted, wanted_policy) {
                continue;
            }

            // No floor here: show-name similarity must actually gate the score,
            // otherwise any file whose S/E numbers line up with a wanted episode
            // of a completely different show would still clear the auto-link
            // threshold (audit #3).
            let score =
                Self::score_episode_show_candidate(&normalized, &show_name, hint.year, show_year);
            let adjusted =
                Self::adjust_candidate_score(score.min(1.0), wanted, has_file, wanted_policy);
            if adjusted >= 0.45 {
                let display_name = format!("{} S{:02}E{:02}", show_name, season, episode);
                out.push(MatchCandidate {
                    target_type: "Episode".to_string(),
                    target_id,
                    target_name: Some(display_name),
                    score: adjusted,
                    reason: Some("episode season/number + show similarity".to_string()),
                    wanted: Some(wanted),
                });
            }
        }

        Ok(Self::sort_and_trim_candidates(out, limit))
    }

    async fn collect_track_candidates(
        &self,
        library_id: &str,
        path: &str,
        limit: usize,
        wanted_policy: MatchWantedPolicy,
    ) -> Result<Vec<MatchCandidate>> {
        let auth_user = self.system_auth_user(None).await?;
        let rows = self
            .get_or_load_track_candidate_rows(&auth_user, library_id)
            .await?;
        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        let normalized = Self::normalize_for_match(file_name);

        let mut out = Vec::new();
        for row in rows {
            let target_id = row.id;
            let title = row.title;
            let album = row.album_name;
            let wanted = row.wanted;
            let has_file = row.has_file;
            if !Self::should_include_candidate(wanted, wanted_policy) {
                continue;
            }

            let title_norm = Self::normalize_for_match(&title);
            let album_norm = Self::normalize_for_match(&album);
            let mut score = jaro_winkler(&normalized, &title_norm);
            if !album_norm.is_empty() && normalized.contains(&album_norm) {
                score += 0.1;
            }

            let adjusted =
                Self::adjust_candidate_score(score.min(1.0), wanted, has_file, wanted_policy);
            if adjusted >= 0.5 {
                let display_name = if album.is_empty() {
                    title.to_string()
                } else {
                    format!("{} — {}", album, title)
                };
                out.push(MatchCandidate {
                    target_type: "Track".to_string(),
                    target_id,
                    target_name: Some(display_name),
                    score: adjusted,
                    reason: Some("track title + album heuristic".to_string()),
                    wanted: Some(wanted),
                });
            }
        }

        Ok(Self::sort_and_trim_candidates(out, limit))
    }

    async fn collect_chapter_candidates(
        &self,
        library_id: &str,
        path: &str,
        limit: usize,
        wanted_policy: MatchWantedPolicy,
    ) -> Result<Vec<MatchCandidate>> {
        let auth_user = self.system_auth_user(None).await?;
        let rows = self
            .get_or_load_chapter_candidate_rows(&auth_user, library_id)
            .await?;
        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        let normalized = Self::normalize_for_match(file_name);
        let chapter_re = Regex::new(r"(?i)\b(ch(?:apter)?\s*|track\s*)(\d{1,3})\b")?;
        let chapter_num = chapter_re
            .captures(file_name)
            .and_then(|c| c.get(2))
            .and_then(|m| m.as_str().parse::<i32>().ok());

        let mut out = Vec::new();
        for row in rows {
            let target_id = row.id;
            let chapter_number = row.chapter_number.unwrap_or_default();
            let book_title = row.audiobook_title;
            let chapter_title = row.title;
            let author_name = row.author_name;
            let wanted = row.wanted;
            let has_file = row.has_file;
            if !Self::should_include_candidate(wanted, wanted_policy) {
                continue;
            }

            let mut score = 0.0;
            if let Some(num) = chapter_num
                && num == chapter_number
            {
                score += 0.5;
            }
            let book_norm = Self::normalize_for_match(&book_title);
            if normalized.contains(&book_norm) {
                score += 0.25;
            }
            if let Some(author) = author_name {
                let author_norm = Self::normalize_for_match(&author);
                if normalized.contains(&author_norm) {
                    score += 0.15;
                }
            }
            let title_norm = Self::normalize_for_match(&chapter_title);
            score += (jaro_winkler(&normalized, &title_norm) * 0.1).min(0.1);
            let adjusted =
                Self::adjust_candidate_score(score.min(1.0), wanted, has_file, wanted_policy);
            if adjusted >= 0.4 {
                let display_name = if book_title.is_empty() {
                    format!("Chapter: chapter {}", chapter_number)
                } else {
                    format!("{} — Ch. {}", book_title, chapter_number)
                };
                out.push(MatchCandidate {
                    target_type: "Chapter".to_string(),
                    target_id,
                    target_name: Some(display_name),
                    score: adjusted,
                    reason: Some("chapter number/title heuristic".to_string()),
                    wanted: Some(wanted),
                });
            }
        }

        Ok(Self::sort_and_trim_candidates(out, limit))
    }

    async fn collect_candidates_for_library_type(
        &self,
        library_id: &str,
        normalized_library_type: &str,
        match_source: &str,
        limit: usize,
        wanted_policy: MatchWantedPolicy,
    ) -> Result<Vec<MatchCandidate>> {
        match normalized_library_type {
            "movies" => {
                self.collect_movie_candidates(library_id, match_source, limit, wanted_policy)
                    .await
            }
            "tv" => {
                self.collect_episode_candidates(library_id, match_source, limit, wanted_policy)
                    .await
            }
            "music" => {
                self.collect_track_candidates(library_id, match_source, limit, wanted_policy)
                    .await
            }
            "audiobooks" => {
                self.collect_chapter_candidates(library_id, match_source, limit, wanted_policy)
                    .await
            }
            _ => Ok(Vec::new()),
        }
    }

    fn merge_candidates(
        mut base: Vec<MatchCandidate>,
        mut extra: Vec<MatchCandidate>,
        limit: usize,
    ) -> Vec<MatchCandidate> {
        for candidate in extra.drain(..) {
            if let Some(existing) = base.iter_mut().find(|existing| {
                existing.target_type == candidate.target_type
                    && existing.target_id == candidate.target_id
            }) {
                if candidate.score > existing.score {
                    *existing = candidate;
                }
            } else {
                base.push(candidate);
            }
        }
        Self::sort_and_trim_candidates(base, limit)
    }

    fn parse_setting_bool(value: Option<&str>, fallback: bool) -> bool {
        value
            .and_then(|value| serde_json::from_str::<bool>(value).ok())
            .unwrap_or(fallback)
    }

    fn parse_setting_f64(value: Option<&str>, fallback: f64) -> f64 {
        value
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(fallback)
    }

    fn parse_setting_u32(value: Option<&str>, fallback: u32) -> u32 {
        value
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(fallback)
    }

    fn parse_setting_string(value: Option<&str>, fallback: &str) -> String {
        value
            .and_then(|value| {
                serde_json::from_str::<String>(value)
                    .ok()
                    .or_else(|| Some(value.to_string()))
            })
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty() && value != "null")
            .unwrap_or_else(|| fallback.to_string())
    }

    pub async fn load_ollama_settings(&self, library_type: &str) -> Result<OllamaParserSettings> {
        let auth_user = self.system_auth_user(None).await?;
        let data = self
            .execute_graphql(
                &auth_user,
                r#"query LlmSettingsForMatch {
                    AppSettings: appSettings(
                        where: { category: { eq: "llm" } }
                        page: { limit: 100 }
                    ) {
                        Edges: edges { Node: node { Key: key Value: value } }
                    }
                }"#,
                serde_json::json!({}),
            )
            .await?;

        let mut settings = HashMap::<String, String>::new();
        if let Some(edges) = data
            .get("AppSettings")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
        {
            for edge in edges {
                if let Some(node) = edge.get("Node")
                    && let (Some(key), Some(value)) = (
                        node.get("Key").and_then(|v| v.as_str()),
                        node.get("Value").and_then(|v| v.as_str()),
                    )
                {
                    settings.insert(key.to_string(), value.to_string());
                }
            }
        }

        let default = OllamaParserSettings::default();
        let model_key = format!("llm.model.{library_type}");
        let prompt_key = format!("llm.prompt.{library_type}");
        let model = Self::parse_setting_string(
            settings
                .get(&model_key)
                .map(String::as_str)
                .or_else(|| settings.get("llm.ollama_model").map(String::as_str)),
            &default.model,
        );
        let prompt_template = settings
            .get(&prompt_key)
            .or_else(|| settings.get("llm.prompt_template"))
            .and_then(|value| {
                let parsed = Self::parse_setting_string(Some(value), "");
                if parsed.is_empty() {
                    None
                } else {
                    Some(parsed)
                }
            });

        Ok(OllamaParserSettings {
            enabled: Self::parse_setting_bool(
                settings.get("llm.enabled").map(String::as_str),
                default.enabled,
            ),
            ollama_url: Self::parse_setting_string(
                settings.get("llm.ollama_url").map(String::as_str),
                &default.ollama_url,
            ),
            model,
            timeout_seconds: Self::parse_setting_u32(
                settings.get("llm.timeout_seconds").map(String::as_str),
                default.timeout_seconds as u32,
            ) as u64,
            temperature: Self::parse_setting_f64(
                settings.get("llm.temperature").map(String::as_str),
                default.temperature,
            ),
            max_tokens: Self::parse_setting_u32(
                settings.get("llm.max_tokens").map(String::as_str),
                default.max_tokens,
            ),
            max_retries: Self::parse_setting_u32(
                settings.get("llm.max_retries").map(String::as_str),
                default.max_retries,
            ),
            confidence_threshold: Self::parse_setting_f64(
                settings.get("llm.confidence_threshold").map(String::as_str),
                default.confidence_threshold,
            ),
            prompt_template,
        })
    }

    pub fn deterministic_parser_preview(library_type: &str, filename: &str) -> Option<String> {
        match Self::normalize_library_type(library_type).as_str() {
            "movies" => {
                let hint = Self::parse_movie_hint(filename);
                hint.title.map(|title| match hint.year {
                    Some(year) => format!("{title} ({year})"),
                    None => title,
                })
            }
            "tv" => {
                let hint = Self::parse_episode_hint(filename);
                hint.show_name.map(|show| {
                    let episode = match (hint.season, hint.episode) {
                        (Some(season), Some(episode)) => format!(" S{season:02}E{episode:02}"),
                        _ => String::new(),
                    };
                    format!("{show}{episode}")
                })
            }
            "music" => {
                let hint = Self::parse_track_hint(filename);
                match (hint.artist_name, hint.album_name, hint.title) {
                    (Some(artist), Some(album), Some(track)) => {
                        Some(format!("{artist} - {album} - {track}"))
                    }
                    (Some(artist), _, Some(track)) => Some(format!("{artist} - {track}")),
                    (_, _, Some(track)) => Some(track),
                    _ => None,
                }
            }
            "audiobooks" => {
                let hint = Self::parse_chapter_hint(filename);
                match (hint.author_name, hint.audiobook_title, hint.chapter_title) {
                    (Some(author), Some(book), Some(chapter)) => {
                        Some(format!("{author} - {book} - {chapter}"))
                    }
                    (Some(author), Some(book), None) => Some(format!("{author} - {book}")),
                    (_, Some(book), Some(chapter)) => Some(format!("{book} - {chapter}")),
                    (_, Some(book), None) => Some(book),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub async fn test_ollama_parser(
        &self,
        library_type: &str,
        filename: &str,
    ) -> Result<OllamaParsedHint> {
        let normalized = Self::normalize_library_type(library_type);
        let settings = self.load_ollama_settings(&normalized).await?;
        OllamaClient::default()
            .parse_filename(&settings, &normalized, filename)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn collect_ollama_fallback_candidates(
        &self,
        media_file_id: &str,
        library_id: &str,
        normalized_library_type: &str,
        match_source: &str,
        limit: usize,
        wanted_policy: MatchWantedPolicy,
        deterministic_best_score: f64,
    ) -> Result<Vec<MatchCandidate>> {
        let settings = self.load_ollama_settings(normalized_library_type).await?;
        if !settings.enabled || deterministic_best_score >= settings.confidence_threshold {
            return Ok(Vec::new());
        }

        let file_name = Path::new(match_source)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(match_source);
        let hint = match OllamaClient::default()
            .parse_filename(&settings, normalized_library_type, file_name)
            .await
        {
            Ok(hint) => hint,
            Err(error) => {
                debug!(
                    media_file_id = %media_file_id,
                    library_id = %library_id,
                    library_type = %normalized_library_type,
                    match_source = %match_source,
                    error = %error,
                    "Ollama fallback parser failed for media_file_id={}, library_id={}, source='{}': {}",
                    media_file_id,
                    library_id,
                    match_source,
                    error
                );
                return Ok(Vec::new());
            }
        };

        if hint.confidence() < settings.confidence_threshold {
            debug!(
                media_file_id = %media_file_id,
                library_id = %library_id,
                library_type = %normalized_library_type,
                match_source = %match_source,
                confidence = hint.confidence(),
                threshold = settings.confidence_threshold,
                "Ollama fallback parse below confidence threshold for media_file_id={}, library_id={}, source='{}'",
                media_file_id,
                library_id,
                match_source
            );
            return Ok(Vec::new());
        }

        let Some(ollama_match_source) = hint.to_match_source(normalized_library_type) else {
            return Ok(Vec::new());
        };

        let mut candidates = self
            .collect_candidates_for_library_type(
                library_id,
                normalized_library_type,
                &ollama_match_source,
                limit,
                wanted_policy,
            )
            .await?;
        for candidate in &mut candidates {
            candidate.score = (candidate.score + (hint.confidence() * 0.05)).min(1.0);
            candidate.reason = Some(format!(
                "ollama fallback parse + {}",
                candidate.reason.as_deref().unwrap_or("ranked candidate")
            ));
        }
        Ok(candidates)
    }

    async fn apply_link_for_candidate(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        library_id: &str,
        candidate: &MatchCandidate,
    ) -> Result<()> {
        // Candidate-ranking matches are always "auto" — manual matches only come from
        // explicit user-selected IDs (design.md Q9b).
        match candidate.target_type.as_str() {
            "Movie" => {
                self.link_movie(
                    auth_user,
                    media_file_id,
                    library_id,
                    &candidate.target_id,
                    "auto",
                    None,
                )
                .await
            }
            "Episode" => {
                self.link_episode(
                    auth_user,
                    media_file_id,
                    library_id,
                    &candidate.target_id,
                    "auto",
                    None,
                )
                .await
            }
            "Track" => {
                self.link_track(
                    auth_user,
                    media_file_id,
                    library_id,
                    &candidate.target_id,
                    "auto",
                    None,
                )
                .await
            }
            "Chapter" => {
                self.link_chapter(
                    auth_user,
                    media_file_id,
                    library_id,
                    &candidate.target_id,
                    "auto",
                    None,
                )
                .await
            }
            other => anyhow::bail!("unsupported candidate target type: {}", other),
        }
    }

    pub async fn unmatch_media_file(&self, media_file_id: &str) -> Result<()> {
        let auth_user = self.system_auth_user(None).await?;

        let data = self
            .execute_mutation(
                &auth_user,
                r#"mutation unmatchMediaFileLinks($mediaFileId: String!, $input: UpdateMediaFileInput!) {
                    UpdateMediaFile: updateMediaFile(id: $mediaFileId, input: $input) { Success: success Error: error }
                }"#,
                serde_json::json!({
                    "mediaFileId": media_file_id,
                    "input": {
                        "movieId": null,
                        "episodeId": null,
                        "trackId": null,
                        "chapterId": null,
                        "matchType": null,
                        "matchedByUserId": null,
                        "matchConfirmedAt": null,
                    }
                }),
            )
            .await?;
        let ok = data
            .get("UpdateMediaFile")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !ok {
            let err = data
                .get("UpdateMediaFile")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("failed to clear media file links");
            anyhow::bail!(err.to_string());
        }

        Ok(())
    }

    /// Delete a media file for good: its record through the generated mutation, then the file.
    ///
    /// The file goes last because that step cannot be undone. If it fails, the record is already
    /// gone and the file is still safe on disk, where the next scan finds it again. The checks
    /// run first, so a file outside its library's root loses neither.
    pub async fn delete_media_file_from_disk(&self, media_file_id: &str) -> Result<()> {
        // A sandbox copies the real database, so its paths are the real library's files.
        if std::env::var_os("LIBRARIAN_DISABLE_DISK_DELETE").is_some() {
            anyhow::bail!("Deleting files from disk is disabled on this server");
        }
        let file = self.get_media_file(media_file_id).await?;
        let library_id = file
            .library_id
            .as_deref()
            .context("This file does not belong to a library, so it cannot be deleted from disk")?;
        let library = self.get_library(library_id).await?;
        let root = PathBuf::from(&library.path);
        let path = PathBuf::from(&file.path);
        let target = deletable_media_file(&root, &path).await?;

        let auth_user = self.system_auth_user(None).await?;
        let data = self
            .execute_mutation(
                &auth_user,
                r#"mutation deleteMediaFileRecord($id: String!) {
                    DeleteMediaFile: deleteMediaFile(id: $id) { Success: success Error: error }
                }"#,
                serde_json::json!({ "id": media_file_id }),
            )
            .await?;
        if !data
            .get("DeleteMediaFile")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            let error = data
                .get("DeleteMediaFile")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("failed to delete the media file record");
            anyhow::bail!(error.to_string());
        }

        let Some(target) = target else {
            return Ok(());
        };
        remove_media_file_in(&root, &target)
            .await
            .with_context(|| {
                format!(
                    "{} left the library, but the file could not be deleted from disk. The next scan will find it again",
                    file.original_name.as_deref().unwrap_or(&file.path)
                )
            })
    }

    pub async fn match_media_file(&self, mut request: MatchRequest) -> Result<MatchResult> {
        let media_file = self.get_media_file(&request.media_file_id).await?;
        let library_id = request
            .library_id
            .clone()
            .or_else(|| media_file.library_id.clone())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Media file is unmatched (no LibraryId); provide LibraryId when matching"
                )
            })?;
        let library = self.get_library(&library_id).await?;
        let normalized_library_type = Self::normalize_library_type(&library.library_type);
        let auth_user = self.system_auth_user(Some(&library.user_id)).await?;

        if request.methods.is_empty() {
            request.methods = vec![MatchMethod::Filename, MatchMethod::Metadata];
        }

        let mut method_set: HashSet<MatchMethod> = request.methods.into_iter().collect();
        let ollama_requested = method_set.remove(&MatchMethod::Ollama);

        let candidate_limit = Self::normalized_candidate_limit(request.candidate_limit);
        let wanted_policy = request.wanted_policy;
        let existing_link = if let Some(id) = media_file.movie_id.as_ref() {
            Some(("Movie".to_string(), id.clone()))
        } else if let Some(id) = media_file.episode_id.as_ref() {
            Some(("Episode".to_string(), id.clone()))
        } else if let Some(id) = media_file.track_id.as_ref() {
            Some(("Track".to_string(), id.clone()))
        } else {
            media_file
                .chapter_id
                .as_ref()
                .map(|id| ("Chapter".to_string(), id.clone()))
        };

        let match_source = media_file
            .original_name
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&media_file.path);

        let is_explicit_target = request.movie_id.is_some()
            || request.episode_id.is_some()
            || request.track_id.is_some()
            || request.chapter_id.is_some();

        // Manual-match protection (design.md Q9b): a manually-matched file must never
        // be overwritten by automatic matching/rescan, even with force:true. Explicit
        // target-id requests (the user picking a specific match) may still replace a
        // manual match — that's the user's own intent — but ranked-candidate and
        // provider-fallback matching must back off entirely, before any of that logic
        // runs.
        if Self::should_block_automatic_match(is_explicit_target, media_file.match_type.as_deref())
        {
            warn!(
                media_file_id = %request.media_file_id,
                library_id = %library_id,
                force = request.force,
                "Skipping automatic match: media file is manually matched and manual matches are never overwritten by automatic matching (design.md Q9b): media_file_id={}, library_id={}, force={}",
                request.media_file_id,
                library_id,
                request.force
            );
            let (target_type, target_id) = existing_link.clone().unwrap_or_default();
            return Ok(MatchResult {
                success: false,
                auto_matched: false,
                already_matched: true,
                matched_type: if target_type.is_empty() {
                    None
                } else {
                    Some(target_type)
                },
                matched_id: if target_id.is_empty() {
                    None
                } else {
                    Some(target_id)
                },
                confidence: 1.0,
                reason: Some(
                    "Media file is manually matched; automatic matching is skipped to protect \
                     the manual match. Unmatch first, or provide an explicit target id, to \
                     change it."
                        .to_string(),
                ),
                candidates: Vec::new(),
            });
        }

        if existing_link.is_some() && !request.force {
            let (target_type, target_id) = existing_link.unwrap_or_default();
            return Ok(MatchResult {
                success: true,
                auto_matched: false,
                already_matched: true,
                matched_type: Some(target_type.clone()),
                matched_id: Some(target_id.clone()),
                confidence: 1.0,
                reason: Some("Media file already matched. Use Force=true to rematch.".to_string()),
                candidates: vec![MatchCandidate {
                    target_type,
                    target_id,
                    target_name: None,
                    score: 1.0,
                    reason: Some("Existing link".to_string()),
                    wanted: None,
                }],
            });
        }

        if let Some(movie_id) = request.movie_id.as_deref() {
            self.link_movie(
                &auth_user,
                &request.media_file_id,
                &library_id,
                movie_id,
                "manual",
                request.requested_by_user_id.as_deref(),
            )
            .await?;
            info!(
                media_file_id = %request.media_file_id,
                library_id = %library_id,
                movie_id = %movie_id,
                "Manual match applied: media_file_id={} linked to movie_id={} in library_id={}",
                request.media_file_id,
                movie_id,
                library_id
            );
            return Ok(MatchResult {
                success: true,
                auto_matched: false,
                already_matched: false,
                matched_type: Some("Movie".to_string()),
                matched_id: Some(movie_id.to_string()),
                confidence: 1.0,
                reason: Some("Manually matched to explicit MovieId".to_string()),
                candidates: vec![MatchCandidate {
                    target_type: "Movie".to_string(),
                    target_id: movie_id.to_string(),
                    target_name: None,
                    score: 1.0,
                    reason: Some("Explicit MovieId".to_string()),
                    wanted: None,
                }],
            });
        }

        if let Some(episode_id) = request.episode_id.as_deref() {
            self.link_episode(
                &auth_user,
                &request.media_file_id,
                &library_id,
                episode_id,
                "manual",
                request.requested_by_user_id.as_deref(),
            )
            .await?;
            info!(
                media_file_id = %request.media_file_id,
                library_id = %library_id,
                episode_id = %episode_id,
                "Manual match applied: media_file_id={} linked to episode_id={} in library_id={}",
                request.media_file_id,
                episode_id,
                library_id
            );
            return Ok(MatchResult {
                success: true,
                auto_matched: false,
                already_matched: false,
                matched_type: Some("Episode".to_string()),
                matched_id: Some(episode_id.to_string()),
                confidence: 1.0,
                reason: Some("Manually matched to explicit EpisodeId".to_string()),
                candidates: vec![MatchCandidate {
                    target_type: "Episode".to_string(),
                    target_id: episode_id.to_string(),
                    target_name: None,
                    score: 1.0,
                    reason: Some("Explicit EpisodeId".to_string()),
                    wanted: None,
                }],
            });
        }

        if let Some(track_id) = request.track_id.as_deref() {
            self.link_track(
                &auth_user,
                &request.media_file_id,
                &library_id,
                track_id,
                "manual",
                request.requested_by_user_id.as_deref(),
            )
            .await?;
            info!(
                media_file_id = %request.media_file_id,
                library_id = %library_id,
                track_id = %track_id,
                "Manual match applied: media_file_id={} linked to track_id={} in library_id={}",
                request.media_file_id,
                track_id,
                library_id
            );
            return Ok(MatchResult {
                success: true,
                auto_matched: false,
                already_matched: false,
                matched_type: Some("Track".to_string()),
                matched_id: Some(track_id.to_string()),
                confidence: 1.0,
                reason: Some("Manually matched to explicit TrackId".to_string()),
                candidates: vec![MatchCandidate {
                    target_type: "Track".to_string(),
                    target_id: track_id.to_string(),
                    target_name: None,
                    score: 1.0,
                    reason: Some("Explicit TrackId".to_string()),
                    wanted: None,
                }],
            });
        }

        if let Some(chapter_id) = request.chapter_id.as_deref() {
            self.link_chapter(
                &auth_user,
                &request.media_file_id,
                &library_id,
                chapter_id,
                "manual",
                request.requested_by_user_id.as_deref(),
            )
            .await?;
            info!(
                media_file_id = %request.media_file_id,
                library_id = %library_id,
                chapter_id = %chapter_id,
                "Manual match applied: media_file_id={} linked to chapter_id={} in library_id={}",
                request.media_file_id,
                chapter_id,
                library_id
            );
            return Ok(MatchResult {
                success: true,
                auto_matched: false,
                already_matched: false,
                matched_type: Some("Chapter".to_string()),
                matched_id: Some(chapter_id.to_string()),
                confidence: 1.0,
                reason: Some("Manually matched to explicit ChapterId".to_string()),
                candidates: vec![MatchCandidate {
                    target_type: "Chapter".to_string(),
                    target_id: chapter_id.to_string(),
                    target_name: None,
                    score: 1.0,
                    reason: Some("Explicit ChapterId".to_string()),
                    wanted: None,
                }],
            });
        }

        let mut candidates = Vec::new();
        if method_set.contains(&MatchMethod::Filename)
            || method_set.contains(&MatchMethod::Metadata)
        {
            candidates = self
                .collect_candidates_for_library_type(
                    &library_id,
                    normalized_library_type.as_str(),
                    match_source,
                    candidate_limit,
                    wanted_policy,
                )
                .await?;
        }

        if ollama_requested {
            let deterministic_best_score = candidates.first().map(|c| c.score).unwrap_or(0.0);
            let ollama_candidates = self
                .collect_ollama_fallback_candidates(
                    &request.media_file_id,
                    &library_id,
                    normalized_library_type.as_str(),
                    match_source,
                    candidate_limit,
                    wanted_policy,
                    deterministic_best_score,
                )
                .await?;
            candidates = Self::merge_candidates(candidates, ollama_candidates, candidate_limit);
        }

        if request.auto_match
            && let Some(best) = candidates.first()
            && best.score >= 0.7
        {
            self.apply_link_for_candidate(&auth_user, &request.media_file_id, &library_id, best)
                .await?;
            return Ok(MatchResult {
                success: true,
                auto_matched: true,
                already_matched: false,
                matched_type: Some(best.target_type.clone()),
                matched_id: Some(best.target_id.clone()),
                confidence: best.score,
                reason: Some("Auto matched from ranked candidates".to_string()),
                candidates,
            });
        }

        if request.allow_provider_fallback
            && (method_set.contains(&MatchMethod::Metadata)
                || method_set.contains(&MatchMethod::Filename))
        {
            let provider_result = match normalized_library_type.as_str() {
                "movies" => {
                    self.try_provider_create_and_match_movie(
                        &library,
                        &auth_user,
                        &request.media_file_id,
                        &media_file.path,
                        match_source,
                    )
                    .await?
                }
                "tv" => {
                    self.try_provider_create_and_match_episode(
                        &library,
                        &auth_user,
                        &request.media_file_id,
                        &media_file.path,
                        match_source,
                    )
                    .await?
                }
                "music" => {
                    self.try_provider_create_and_match_track(
                        &library,
                        &auth_user,
                        &request.media_file_id,
                        &media_file.path,
                        match_source,
                    )
                    .await?
                }
                "audiobooks" => {
                    self.try_provider_create_and_match_chapter(
                        &library,
                        &auth_user,
                        &request.media_file_id,
                        &media_file.path,
                        match_source,
                    )
                    .await?
                }
                _ => None,
            };
            if let Some(mut result) = provider_result {
                result.auto_matched = true;
                if result.candidates.is_empty() {
                    result.candidates = candidates;
                }
                return Ok(result);
            }
        }

        warn!(
            media_file_id = %request.media_file_id,
            library_id = %library_id,
            methods = ?method_set,
            "No match found for media file: media_file_id={}, library_id={}, methods={:?}",
            request.media_file_id,
            library_id,
            method_set
        );
        Ok(MatchResult {
            success: false,
            auto_matched: false,
            already_matched: false,
            matched_type: None,
            matched_id: None,
            confidence: 0.0,
            reason: Some("No match found".to_string()),
            candidates,
        })
    }

    async fn plan_media_target_path(
        &self,
        auth_user: &AuthUser,
        library: &LibraryRow,
        target_type: &str,
        target_id: &str,
        source_path: &Path,
    ) -> Result<PathBuf> {
        let normalized_library_type = Self::normalize_library_type(&library.library_type);
        let naming_pattern = self.resolve_library_naming_pattern(library).await?;
        let ext = source_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mkv");
        let original_filename = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");

        let relative_path = match (normalized_library_type.as_str(), target_type) {
            ("movies", "Movie") => {
                let data = self
                    .execute_graphql(
                        auth_user,
                        r#"query OrganizeMovieInfo($id: String!) {
                            Movie: movie(id: $id) { Title: title Year: year }
                        }"#,
                        serde_json::json!({ "id": target_id }),
                    )
                    .await?;
                let movie = data.get("Movie");
                let Some(title) = movie.and_then(|m| m.get("Title")).and_then(|v| v.as_str())
                else {
                    anyhow::bail!("Movie relation not found for organizing");
                };
                let year = movie
                    .and_then(|m| m.get("Year"))
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32);
                TargetFacts::Movie {
                    title: title.to_string(),
                    year,
                }
                .relative_path(&naming_pattern, original_filename, ext)?
            }
            ("tv", "Episode") => {
                let data = self
                    .execute_graphql(
                        auth_user,
                        r#"query OrganizeEpisodeInfo($id: String!) {
                            Episode: episode(id: $id) {
                                ShowId: showId
                                Season: season
                                Episode: episode
                                Title: title
                            }
                        }"#,
                        serde_json::json!({ "id": target_id }),
                    )
                    .await?;
                let episode_node = data.get("Episode");
                let (Some(show_id), Some(season), Some(episode)) = (
                    episode_node
                        .and_then(|e| e.get("ShowId"))
                        .and_then(|v| v.as_str()),
                    episode_node
                        .and_then(|e| e.get("Season"))
                        .and_then(|v| v.as_i64()),
                    episode_node
                        .and_then(|e| e.get("Episode"))
                        .and_then(|v| v.as_i64()),
                ) else {
                    anyhow::bail!("Episode relation not found for organizing");
                };
                let show_data = self
                    .execute_graphql(
                        auth_user,
                        r#"query OrganizeEpisodeShowInfo($id: String!) {
                            Show: show(id: $id) { Name: name }
                        }"#,
                        serde_json::json!({ "id": show_id }),
                    )
                    .await?;
                let Some(show_name) = show_data
                    .get("Show")
                    .and_then(|s| s.get("Name"))
                    .and_then(|v| v.as_str())
                else {
                    anyhow::bail!("Show relation not found for organizing");
                };
                let episode_title = episode_node
                    .and_then(|e| e.get("Title"))
                    .and_then(|v| v.as_str());
                TargetFacts::Episode {
                    show_name: show_name.to_string(),
                    season: season as i32,
                    episode: episode as i32,
                    title: episode_title.map(str::to_string),
                }
                .relative_path(&naming_pattern, original_filename, ext)?
            }
            ("music", "Track") => {
                let data = self
                    .execute_graphql(
                        auth_user,
                        r#"query OrganizeTrackInfo($id: String!) {
                            Track: track(id: $id) {
                                Title: title
                                TrackNumber: trackNumber
                                DiscNumber: discNumber
                                AlbumId: albumId
                            }
                        }"#,
                        serde_json::json!({ "id": target_id }),
                    )
                    .await?;
                let track = data.get("Track");
                let album_id = track
                    .and_then(|t| t.get("AlbumId"))
                    .and_then(|v| v.as_str());
                let album_data = if let Some(album_id) = album_id {
                    self.execute_graphql(
                        auth_user,
                        r#"query OrganizeTrackAlbumInfo($id: String!) {
                            Album: album(id: $id) {
                                Name: name
                                Year: year
                                Artist: artist { Name: name }
                                Tracks: tracks(page: { limit: 1000 }) {
                                    Edges: edges { Node: node { DiscNumber: discNumber } }
                                }
                            }
                        }"#,
                        serde_json::json!({ "id": album_id }),
                    )
                    .await?
                } else {
                    serde_json::Value::Null
                };
                let album = album_data.get("Album");
                let (Some(track_title), Some(album_name), Some(track_number)) = (
                    track.and_then(|t| t.get("Title")).and_then(|v| v.as_str()),
                    album.and_then(|a| a.get("Name")).and_then(|v| v.as_str()),
                    track
                        .and_then(|t| t.get("TrackNumber"))
                        .and_then(|v| v.as_i64()),
                ) else {
                    anyhow::bail!("Track relation not found for organizing");
                };
                let album_year = album
                    .and_then(|a| a.get("Year"))
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32);
                let disc_number = track
                    .and_then(|t| t.get("DiscNumber"))
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32);
                // An album stays in one folder, whoever guests on each track: the folder is named
                // from the album's Artist record and nothing a track says about itself.
                let artist_name = album
                    .and_then(|a| a.pointer("/Artist/Name"))
                    .and_then(|v| v.as_str());
                let discs = album
                    .and_then(|a| a.pointer("/Tracks/Edges"))
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flatten()
                    .filter_map(|edge| edge.pointer("/Node/DiscNumber")?.as_i64())
                    .max()
                    .unwrap_or(1) as i32;
                TargetFacts::Track {
                    title: track_title.to_string(),
                    album_name: album_name.to_string(),
                    album_year,
                    track_number: track_number as i32,
                    disc_number,
                    discs,
                    artist_name: artist_name.map(str::to_string),
                }
                .relative_path(&naming_pattern, original_filename, ext)?
            }
            ("audiobooks", "Chapter") => {
                let data = self
                    .execute_graphql(
                        auth_user,
                        r#"query OrganizeChapterInfo($id: String!) {
                            Chapter: chapter(id: $id) {
                                ChapterNumber: chapterNumber
                                Title: title
                                AudiobookId: audiobookId
                            }
                        }"#,
                        serde_json::json!({ "id": target_id }),
                    )
                    .await?;
                let chapter_node = data.get("Chapter");
                let audiobook_id = chapter_node
                    .and_then(|c| c.get("AudiobookId"))
                    .and_then(|v| v.as_str());
                let audiobook_data = if let Some(audiobook_id) = audiobook_id {
                    self.execute_graphql(
                        auth_user,
                        r#"query OrganizeChapterAudiobookInfo($id: String!) {
                            Audiobook: audiobook(id: $id) { Title: title AuthorName: authorName }
                        }"#,
                        serde_json::json!({ "id": audiobook_id }),
                    )
                    .await?
                } else {
                    serde_json::Value::Null
                };
                let audiobook = audiobook_data.get("Audiobook");
                let (Some(chapter_number), Some(book_title)) = (
                    chapter_node
                        .and_then(|c| c.get("ChapterNumber"))
                        .and_then(|v| v.as_i64()),
                    audiobook
                        .and_then(|a| a.get("Title"))
                        .and_then(|v| v.as_str()),
                ) else {
                    anyhow::bail!("Media file is not matched to an audiobook chapter");
                };
                let chapter_title = chapter_node
                    .and_then(|c| c.get("Title"))
                    .and_then(|v| v.as_str());
                let author_name = audiobook
                    .and_then(|a| a.get("AuthorName"))
                    .and_then(|v| v.as_str());
                TargetFacts::Chapter {
                    author_name: author_name.map(str::to_string),
                    book_title: book_title.to_string(),
                    chapter_number: chapter_number as i32,
                    chapter_title: chapter_title.map(str::to_string),
                }
                .relative_path(&naming_pattern, original_filename, ext)?
            }
            _ => anyhow::bail!("Unsupported library type/target for organizing"),
        };

        validate_relative_library_path(&relative_path)?;
        let mut target = PathBuf::from(&library.path).join(relative_path);
        if target_type == "Chapter" {
            if self.book_is_in_parts(target_id).await? {
                // Parts don't line up with chapters, so a chapter's name would be wrong on them.
                if let Some(name) = source_path.file_name() {
                    target = target.with_file_name(name);
                }
            } else if self.chapter_shares_a_file(target_id).await? {
                target = self.named_after_book(target_id, target).await?;
            }
        }
        checked_library_path(Path::new(&library.path), &target)?;
        Ok(target)
    }

    /// One file holding a whole book takes the book's name, not its first chapter's.
    async fn named_after_book(&self, chapter_id: &str, path: PathBuf) -> Result<PathBuf> {
        use crate::graphql::entities::{Audiobook, Chapter};
        let database = self.database().await?;
        let pool = database.pool().pool();
        let chapter = Chapter::get(pool, &chapter_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("That chapter no longer exists"))?;
        let book = Audiobook::get(pool, &chapter.audiobook_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("That audiobook no longer exists"))?;
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("m4b")
            .to_string();
        Ok(path.with_file_name(format!(
            "{}.{extension}",
            sanitize_for_filename(&book.title)
        )))
    }

    async fn book_chapters(
        &self,
        audiobook_id: &str,
    ) -> Result<Vec<crate::graphql::entities::Chapter>> {
        use crate::graphql::entities::{Chapter, ChapterWhereInput};
        let database = self.database().await?;
        Ok(Chapter::query(database.pool().pool())
            .filter(ChapterWhereInput {
                audiobook_id: Some(graphql_orm::graphql::filters::StringFilter {
                    eq: Some(audiobook_id.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?)
    }

    /// Whether a file is about as long as the whole of the book these chapters belong to.
    async fn file_is_whole_book(
        &self,
        audiobook_id: &str,
        chapters: &[crate::graphql::entities::Chapter],
        media_file_id: &str,
    ) -> Result<bool> {
        use crate::graphql::entities::{Audiobook, MediaFile};
        let database = self.database().await?;
        let pool = database.pool().pool();
        let Some(file_seconds) = MediaFile::get(pool, &media_file_id.to_string())
            .await?
            .and_then(|file| file.duration)
            .map(i64::from)
        else {
            return Ok(false);
        };
        let listed: f64 = chapters
            .iter()
            .filter_map(|chapter| chapter.duration_secs)
            .map(f64::from)
            .sum();
        let book_seconds = Audiobook::get(pool, &audiobook_id.to_string())
            .await?
            .and_then(|book| book.total_duration_secs)
            .map_or(listed, f64::from);
        Ok(file_covers_book(file_seconds, book_seconds))
    }

    async fn book_is_in_parts(&self, chapter_id: &str) -> Result<bool> {
        use crate::graphql::entities::{Audiobook, Chapter};
        let database = self.database().await?;
        let pool = database.pool().pool();
        let Some(chapter) = Chapter::get(pool, &chapter_id.to_string()).await? else {
            return Ok(false);
        };
        Ok(Audiobook::get(pool, &chapter.audiobook_id)
            .await?
            .is_some_and(|book| book.file_shape.as_deref() == Some("parts")))
    }

    async fn chapter_shares_a_file(&self, chapter_id: &str) -> Result<bool> {
        use crate::graphql::entities::Chapter;
        let database = self.database().await?;
        let Some(chapter) = Chapter::get(database.pool().pool(), &chapter_id.to_string()).await?
        else {
            return Ok(false);
        };
        let chapters = self.book_chapters(&chapter.audiobook_id).await?;
        let mut files = chapters
            .iter()
            .map(|chapter| chapter.media_file_id.as_deref());
        let first = files.next().flatten();
        Ok(chapters.len() > 1 && first.is_some() && files.all(|file| file == first))
    }

    /// Point every chapter of the book `chapter_id` belongs to at one file.
    async fn link_whole_book(
        &self,
        auth_user: &AuthUser,
        chapter_id: &str,
        media_file_id: &str,
    ) -> Result<()> {
        use crate::graphql::entities::Chapter;
        let database = self.database().await?;
        let chapter = Chapter::get(database.pool().pool(), &chapter_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("That chapter no longer exists"))?;
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation LinkWholeBook($where: ChapterWhereInput, $input: UpdateChapterInput!) {
                    UpdateChapters: updateChapters(where: $where, input: $input) {
                        Success: success
                        Error: error
                    }
                }"#,
                serde_json::json!({
                    "where": { "audiobookId": { "eq": chapter.audiobook_id } },
                    "input": { "mediaFileId": media_file_id, "wanted": false }
                }),
            )
            .await?;
        if data
            .pointer("/UpdateChapters/Success")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            anyhow::bail!(
                "{}",
                data.pointer("/UpdateChapters/Error")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Could not update the book's chapters")
            );
        }
        Ok(())
    }

    /// Books already imported with one file linked to their first chapter only: link the rest,
    /// and, where the library organizes its files, give the file the book's name. Safe to repeat.
    pub async fn repair_whole_file_audiobooks(&self) -> Result<usize> {
        use crate::graphql::entities::Audiobook;
        let database = self.database().await?;
        let books = Audiobook::query(database.pool().pool()).fetch_all().await?;
        let owner = AuthUser::system_admin();
        let mut repaired = 0;
        for book in books {
            let chapters = self.book_chapters(&book.id).await?;
            if chapters.len() < 2 {
                continue;
            }
            let files: HashSet<&str> = chapters
                .iter()
                .filter_map(|chapter| chapter.media_file_id.as_deref())
                .collect();
            let (1, Some(file_id)) = (files.len(), files.iter().next().copied()) else {
                continue;
            };
            let linked_everywhere = chapters
                .iter()
                .all(|chapter| chapter.media_file_id.is_some());
            let file = self.get_media_file(file_id).await?;
            if !self
                .file_is_whole_book(&book.id, &chapters, file_id)
                .await?
            {
                continue;
            }
            if !linked_everywhere {
                let first = chapters
                    .iter()
                    .find(|chapter| chapter.media_file_id.is_some())
                    .map(|chapter| chapter.id.clone())
                    .unwrap_or_default();
                self.link_whole_book(&owner, &first, file_id).await?;
                repaired += 1;
                info!(audiobook_id = %book.id, title = %book.title, "Linked every chapter to the book's one file");
            }
            let library = self.get_library(&book.library_id).await?;
            let expected_name = format!("{}.", sanitize_for_filename(&book.title));
            let named_for_book = Path::new(&file.path)
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&expected_name));
            if library.auto_organize && !named_for_book {
                let organized = self.organize_media_file(file_id).await?;
                if !organized.success {
                    warn!(media_file_id = %file_id, reason = ?organized.reason, "Could not rename a whole-book file");
                }
            }
        }
        Ok(repaired)
    }

    /// Move one matched file to where the library's naming pattern puts it, then remove the
    /// folders that left empty. A scan organizes through `move_media_file_to_pattern` instead and
    /// clears empty folders once at the end, which costs less than doing so after every move.
    pub async fn organize_media_file(&self, media_file_id: &str) -> Result<OrganizeResult> {
        let result = self.move_media_file_to_pattern(media_file_id).await?;
        if result.success
            && let (Some(old_path), Some(new_path)) = (&result.old_path, &result.new_path)
            && old_path != new_path
        {
            self.remove_folders_emptied_by_move(media_file_id, Path::new(old_path))
                .await;
        }
        Ok(result)
    }

    /// Remove the folder a file was moved out of, and each parent that leaves empty, stopping at
    /// the library root. Hidden folders, the trash and any folder a library item expects stay.
    async fn remove_folders_emptied_by_move(&self, media_file_id: &str, old_path: &Path) {
        let Some(old_dir) = old_path.parent() else {
            return;
        };
        let library = match self.get_media_file(media_file_id).await {
            Ok(file) => match file.library_id {
                Some(library_id) => self.get_library(&library_id).await,
                None => return,
            },
            Err(error) => Err(error),
        };
        let library = match library {
            Ok(library) => library,
            Err(error) => {
                warn!(
                    media_file_id = %media_file_id,
                    old_path = %old_path.display(),
                    error = %error,
                    "Could not tidy the folder '{}' a file moved out of: {}",
                    old_dir.display(),
                    error
                );
                return;
            }
        };
        let root = PathBuf::from(&library.path);
        // Only a folder this move emptied needs work, and knowing what the library expects costs
        // reads of every item, so the common move that leaves a folder in use stops here.
        let emptied = library_storage()
            .is_dir_empty(old_dir)
            .await
            .unwrap_or(false);
        if !emptied || old_dir == root || !old_dir.starts_with(&root) {
            return;
        }
        let protected = match self.build_protected_paths(&library).await {
            Ok(protected) => protected,
            Err(error) => {
                warn!(
                    library_id = %library.id,
                    old_path = %old_path.display(),
                    error = %error,
                    "Could not tidy the folder '{}' a file moved out of: {}",
                    old_dir.display(),
                    error
                );
                return;
            }
        };
        let removed = if is_remote_path(&root) {
            remove_empty_folders_upward_remote(&root, old_dir, &protected).await
        } else {
            // The folders above it are covered too: a folder watch ignores them (Q116).
            let _own = own_write([&StoragePath::new(old_dir.to_string_lossy().into_owned())]);
            let (root, start, protected) = (root, old_dir.to_path_buf(), protected);
            tokio::task::spawn_blocking(move || {
                remove_empty_folders_upward(&root, &start, &protected)
            })
            .await
            .unwrap_or_default()
        };
        for folder in removed {
            debug!(
                library_id = %library.id,
                folder_path = %folder.display(),
                "Removed folder '{}': organizing left it empty",
                folder.display()
            );
        }
    }

    async fn move_media_file_to_pattern(&self, media_file_id: &str) -> Result<OrganizeResult> {
        let media_file = self.get_media_file(media_file_id).await?;
        let Some(media_library_id) = media_file.library_id.clone() else {
            return Ok(OrganizeResult {
                success: false,
                old_path: Some(media_file.path),
                new_path: None,
                reason: Some("Media file has no library association yet".to_string()),
            });
        };
        let library = self.get_library(&media_library_id).await?;
        self.ensure_library_not_moving(&library.id).await?;
        let auth_user = self.system_auth_user(Some(&library.user_id)).await?;

        let old_path = PathBuf::from(&media_file.path);
        checked_library_path(Path::new(&library.path), &old_path)?;
        if !library_storage().exists(&old_path).await.unwrap_or(false) {
            return Ok(OrganizeResult {
                success: false,
                old_path: Some(media_file.path),
                new_path: None,
                reason: Some("Source file does not exist".to_string()),
            });
        }

        let Some((target_type, target_id)) = media_file
            .movie_id
            .as_deref()
            .map(|id| ("Movie", id))
            .or_else(|| media_file.episode_id.as_deref().map(|id| ("Episode", id)))
            .or_else(|| media_file.track_id.as_deref().map(|id| ("Track", id)))
            .or_else(|| media_file.chapter_id.as_deref().map(|id| ("Chapter", id)))
        else {
            return Ok(OrganizeResult {
                success: false,
                old_path: Some(media_file.path),
                new_path: None,
                reason: Some("Media file is not matched to a library item".to_string()),
            });
        };
        let target_path = match self
            .plan_media_target_path(&auth_user, &library, target_type, target_id, &old_path)
            .await
        {
            Ok(path) => path,
            Err(error) => {
                return Ok(OrganizeResult {
                    success: false,
                    old_path: Some(media_file.path),
                    new_path: None,
                    reason: Some(error.to_string()),
                });
            }
        };

        if old_path == target_path {
            return Ok(OrganizeResult {
                success: true,
                old_path: Some(old_path.to_string_lossy().to_string()),
                new_path: Some(target_path.to_string_lossy().to_string()),
                reason: Some("Already organized".to_string()),
            });
        }

        // The video's subtitles go with it (Q118): what is on disk now, as rows to repoint.
        let sidecars = self.sidecars_to_carry(media_file_id, &old_path).await;

        // Atomic no-clobber move (audit #4): `hard_link` + `remove` instead of the
        // previous `exists()`-then-`rename()`, which races under concurrent
        // organize/import and can silently replace an existing target file.
        let move_outcome = place_in_library(
            PathBuf::from(&library.path),
            old_path.clone(),
            target_path.clone(),
            true,
            true,
        )
        .await?;

        if move_outcome == NoClobberOutcome::TargetExists {
            if paths_refer_to_same_file(&old_path, &target_path).await {
                return Ok(OrganizeResult {
                    success: true,
                    old_path: Some(old_path.to_string_lossy().to_string()),
                    new_path: Some(target_path.to_string_lossy().to_string()),
                    reason: Some("Already organized (same canonical target)".to_string()),
                });
            }

            let message = format!(
                "Organize conflict detected and skipped to prevent overwrite: media_file_id={}, library_id={}, source='{}', target='{}'",
                media_file_id,
                library.id,
                old_path.to_string_lossy(),
                target_path.to_string_lossy()
            );
            warn!(
                media_file_id = %media_file_id,
                library_id = %library.id,
                source_path = %old_path.to_string_lossy(),
                target_path = %target_path.to_string_lossy(),
                "{}", message
            );
            // A retried organize meets the same conflict every time.
            let already_said = self
                .execute_graphql(
                    &auth_user,
                    r#"query OpenOrganizeConflict($where: NotificationWhereInput, $page: PageInput) {
                        Notifications: notifications(where: $where, page: $page) {
                            Edges: edges { Node: node { Id: id } }
                        }
                    }"#,
                    serde_json::json!({
                        "where": {
                            "userId": { "eq": library.user_id },
                            "title": { "eq": "Organize conflict skipped" },
                            "message": { "eq": message },
                            "resolvedAt": { "isNull": true }
                        },
                        "page": { "limit": 1, "offset": 0 }
                    }),
                )
                .await
                .is_ok_and(|data| notifications_found(&data));
            if !already_said {
                self.create_notification(
                    &auth_user,
                    &library.user_id,
                    "WARNING",
                    "ORGANIZATION",
                    "Organize conflict skipped",
                    &message,
                )
                .await;
            }

            return Ok(OrganizeResult {
                success: false,
                old_path: Some(old_path.to_string_lossy().to_string()),
                new_path: Some(target_path.to_string_lossy().to_string()),
                reason: Some("Target path already exists; skipped to avoid overwrite".to_string()),
            });
        }

        // The file is now at `target_path`; `old_path` no longer exists. Update the
        // DB only after the file operation fully succeeded, and update both `path`
        // and `relativePath` (previously only `path` was updated here, leaving
        // `relativePath` permanently stale — inconsistent with the torrent-import
        // path, which updates both).
        let relative_path = target_path
            .strip_prefix(&library.path)
            .ok()
            .map(|p| p.to_string_lossy().to_string());

        let update_outcome = self
            .execute_mutation(
                &auth_user,
                r#"mutation UpdateMediaFilePath($id: String!, $input: UpdateMediaFileInput!) {
                    UpdateMediaFile: updateMediaFile(id: $id, input: $input) { Success: success Error: error }
                }"#,
                serde_json::json!({
                    "id": media_file_id,
                    "input": {
                        "path": target_path.to_string_lossy().to_string(),
                        "relativePath": relative_path,
                    }
                }),
            )
            .await;

        let db_error = match update_outcome {
            Ok(data) => {
                let success = data
                    .get("UpdateMediaFile")
                    .and_then(|v| v.get("Success"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if success {
                    None
                } else {
                    Some(
                        data.get("UpdateMediaFile")
                            .and_then(|v| v.get("Error"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("Failed to update organized media path")
                            .to_string(),
                    )
                }
            }
            Err(error) => Some(error.to_string()),
        };

        if let Some(db_error) = db_error {
            // Stale-path guard: the file already moved but the DB still points at
            // `old_path`. Rather than leave that inconsistency (the audit found the
            // reconciler deletes MediaFile rows whose on-disk path is gone), try to
            // move the file back so disk and DB agree again, then report the error.
            error!(
                media_file_id = %media_file_id,
                old_path = %old_path.display(),
                target_path = %target_path.display(),
                error = %db_error,
                "Organize: DB update failed after file move; attempting to move the file back: media_file_id={}, old_path={}, target_path={}, error={}",
                media_file_id,
                old_path.display(),
                target_path.display(),
                db_error
            );

            match place_in_library(
                PathBuf::from(&library.path),
                target_path.clone(),
                old_path.clone(),
                true,
                true,
            )
            .await
            {
                Ok(NoClobberOutcome::Placed) => {
                    anyhow::bail!(
                        "Failed to update organized media path in database ({}); the file was moved back to its original location",
                        db_error
                    );
                }
                Ok(NoClobberOutcome::TargetExists) => {
                    error!(
                        media_file_id = %media_file_id,
                        old_path = %old_path.display(),
                        target_path = %target_path.display(),
                        "Organize: move-back after failed DB update also failed (original path is occupied); the file is stranded at target_path with a stale DB record: media_file_id={}, old_path={}, target_path={}",
                        media_file_id,
                        old_path.display(),
                        target_path.display()
                    );
                    anyhow::bail!(
                        "Failed to update organized media path in database ({}); moving the file back also failed because the original path is occupied — the file remains at '{}' but the database still points at '{}'",
                        db_error,
                        target_path.display(),
                        old_path.display()
                    );
                }
                Err(move_back_error) => {
                    error!(
                        media_file_id = %media_file_id,
                        old_path = %old_path.display(),
                        target_path = %target_path.display(),
                        move_back_error = %move_back_error,
                        "Organize: move-back after failed DB update also failed; the file is stranded at target_path with a stale DB record: media_file_id={}, old_path={}, target_path={}, move_back_error={}",
                        media_file_id,
                        old_path.display(),
                        target_path.display(),
                        move_back_error
                    );
                    anyhow::bail!(
                        "Failed to update organized media path in database ({}); moving the file back also failed ({}) — the file remains at '{}' but the database still points at '{}'",
                        db_error,
                        move_back_error,
                        target_path.display(),
                        old_path.display()
                    );
                }
            }
        }

        if !sidecars.is_empty() {
            self.carry_sidecars(
                Path::new(&library.path),
                &old_path,
                &target_path,
                sidecars,
                true,
                true,
            )
            .await;
        }

        Ok(OrganizeResult {
            success: true,
            old_path: Some(old_path.to_string_lossy().to_string()),
            new_path: Some(target_path.to_string_lossy().to_string()),
            reason: None,
        })
    }

    /// A video's sidecar and downloaded subtitles, looked for on disk first so ones no scan has
    /// seen yet are carried too. Nothing for audio.
    async fn sidecars_to_carry(
        &self,
        media_file_id: &str,
        video: &Path,
    ) -> Vec<(
        Option<String>,
        crate::services::subtitles::sidecar::SidecarFile,
    )> {
        use crate::services::subtitles::{is_bitmap_codec, sidecar, store};
        let video = video.to_string_lossy().into_owned();
        let extension = video
            .rsplit_once('.')
            .map(|(_, ext)| ext)
            .unwrap_or_default();
        if !store::is_video_extension(extension) {
            return Vec::new();
        }
        self.sync_video_sidecars(media_file_id, &video).await;
        let Ok(database) = self.database().await else {
            return Vec::new();
        };
        store::file_rows(database.pool(), media_file_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| {
                let path = row.file_path.clone()?;
                let codec = row.codec.clone().unwrap_or_default();
                Some((
                    Some(row.id.clone()),
                    sidecar::SidecarFile {
                        path,
                        codec: "subrip",
                        bitmap: is_bitmap_codec(&codec),
                        facts: sidecar::NameFacts {
                            language: row
                                .language
                                .as_deref()
                                .and_then(crate::services::subtitles::language::normalize),
                            forced: row.is_forced,
                            hearing_impaired: row.is_hearing_impaired,
                            default: row.is_default,
                            title: if row.source_type == store::SOURCE_SIDECAR {
                                row.title.clone()
                            } else {
                                None
                            },
                        },
                    },
                ))
            })
            .collect()
    }

    pub async fn process_torrent_source_files(
        &self,
        auth_user: &AuthUser,
        info_hash: &str,
        library_override: Option<String>,
        files: Vec<SourceFileImport>,
    ) -> SourceProcessSummary {
        self.run_torrent_import(
            auth_user,
            info_hash,
            library_override,
            files,
            None,
            None,
            None,
        )
        .await
    }

    /// Import a release, or one book of a pack, as a person decided after seeing its plan.
    pub async fn import_torrent_as_book(
        &self,
        auth_user: &AuthUser,
        info_hash: &str,
        audiobook_id: Option<&str>,
        files: Vec<SourceFileImport>,
        choice: BookImportChoice,
    ) -> SourceProcessSummary {
        self.run_torrent_import(
            auth_user,
            info_hash,
            None,
            files,
            None,
            Some((audiobook_id, &choice)),
            None,
        )
        .await
    }

    /// Import some of a download's files as a library item a person or identification chose.
    pub(self) async fn import_torrent_with_link(
        &self,
        auth_user: &AuthUser,
        info_hash: &str,
        library_id: Option<String>,
        files: Vec<SourceFileImport>,
        patch: ImportLinkPatch,
    ) -> SourceProcessSummary {
        self.run_torrent_import(
            auth_user,
            info_hash,
            library_id,
            files,
            None,
            None,
            Some(&patch),
        )
        .await
    }

    /// Import one file of a download as what a person says it is. The target must be in the
    /// chosen library; the same checks as any import then apply, and an existing library file is
    /// still never overwritten.
    pub async fn import_torrent_file_as(
        &self,
        auth_user: &AuthUser,
        info_hash: &str,
        file: SourceFileImport,
        target: ManualFileTarget,
    ) -> SourceProcessSummary {
        match self.manual_target_library(&target).await {
            Ok(library_id) if library_id == target.library_id => {}
            Ok(_) => {
                return Self::failed_source_summary(anyhow::anyhow!(
                    "That title is not in the chosen library"
                ));
            }
            Err(error) => return Self::failed_source_summary(error),
        }
        self.run_torrent_import(
            auth_user,
            info_hash,
            Some(target.library_id.clone()),
            vec![file],
            Some(&target),
            None,
            None,
        )
        .await
    }

    /// What the matcher thinks one downloaded file could be, to choose from. Changes no match.
    pub async fn torrent_file_candidates(
        &self,
        auth_user: &AuthUser,
        info_hash: &str,
        file: &SourceFileImport,
        library_id: &str,
    ) -> Result<MatchResult> {
        let database = self.database().await?;
        let import =
            Self::authorize_torrent_import(database.pool(), auth_user, info_hash, Some(library_id))
                .await?;
        let owner = AuthUser::system_admin();
        let media_file_id = self
            .ensure_source_media_file(&owner, &import.owner_id, info_hash, file)
            .await?;
        self.match_media_file(MatchRequest {
            media_file_id,
            library_id: Some(library_id.to_string()),
            episode_id: None,
            movie_id: None,
            track_id: None,
            chapter_id: None,
            methods: Vec::new(),
            requested_by_user_id: None,
            force: false,
            auto_match: false,
            candidate_limit: 12,
            allow_provider_fallback: false,
            wanted_policy: MatchWantedPolicy::All,
        })
        .await
    }

    async fn manual_target_library(&self, target: &ManualFileTarget) -> Result<String> {
        use crate::graphql::entities::{Audiobook, Chapter, Episode, Movie, Show, Track};
        let database = self.database().await?;
        let pool = database.pool().pool();
        let id = &target.target_id;
        let missing = || anyhow::anyhow!("That title no longer exists");
        Ok(match target.target_type.as_str() {
            "Movie" => Movie::get(pool, id).await?.ok_or_else(missing)?.library_id,
            "Track" => Track::get(pool, id).await?.ok_or_else(missing)?.library_id,
            "Episode" => {
                let episode = Episode::get(pool, id).await?.ok_or_else(missing)?;
                Show::get(pool, &episode.show_id)
                    .await?
                    .ok_or_else(missing)?
                    .library_id
            }
            "Chapter" => {
                let chapter = Chapter::get(pool, id).await?.ok_or_else(missing)?;
                Audiobook::get(pool, &chapter.audiobook_id)
                    .await?
                    .ok_or_else(missing)?
                    .library_id
            }
            other => anyhow::bail!("Unsupported match target: {other}"),
        })
    }

    /// After one file was imported by hand: finished if nothing else is waiting, otherwise a
    /// count of what is, since the file names in the previous message are out of date.
    async fn status_after_manual_import(
        &self,
        import: &AuthorizedTorrentImport,
    ) -> Result<(&'static str, Option<String>)> {
        let owner = AuthUser::system_admin();
        let data = self
            .execute_graphql(
                &owner,
                r#"query WaitingSourceFiles($where: PendingFileMatchWhereInput, $page: PageInput) {
                    PendingFileMatches: pendingFileMatches(where: $where, page: $page) {
                        PageInfo: pageInfo { TotalCount: totalCount }
                    }
                }"#,
                serde_json::json!({
                    "where": {
                        "sourceId": { "eq": import.info_hash },
                        "userId": { "eq": import.owner_id },
                        "copiedAt": { "isNull": true }
                    },
                    "page": { "limit": 1, "offset": 0 }
                }),
            )
            .await?;
        let waiting = data
            .pointer("/PendingFileMatches/PageInfo/TotalCount")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        Ok(if waiting == 0 {
            ("completed", None)
        } else {
            (
                "partial",
                Some(format!("{waiting} file(s) did not match any library item")),
            )
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_torrent_import(
        &self,
        auth_user: &AuthUser,
        info_hash: &str,
        library_override: Option<String>,
        files: Vec<SourceFileImport>,
        manual: Option<&ManualFileTarget>,
        book_choice: Option<(Option<&str>, &BookImportChoice)>,
        patch: Option<&ImportLinkPatch>,
    ) -> SourceProcessSummary {
        let lock = self
            .torrent_import_locks
            .lock()
            .await
            .entry(info_hash.to_string())
            .or_default()
            .clone();
        let _one_at_a_time = lock.lock().await;
        let admission = async {
            let database = self.database().await?;
            Self::authorize_torrent_import(
                database.pool(),
                auth_user,
                info_hash,
                library_override.as_deref(),
            )
            .await
        }
        .await;
        let import = match admission {
            Ok(import) => import,
            Err(error) => return Self::failed_source_summary(error),
        };
        match self
            .process_torrent_source_files_inner(
                &import,
                library_override,
                files,
                manual,
                book_choice,
                patch,
            )
            .await
        {
            Ok(summary) => summary,
            Err(error) => {
                let message = error.to_string();
                if let Err(status_error) = self
                    .update_torrent_post_process_status(&import, "failed", Some(&message), 0, 1)
                    .await
                {
                    warn!(info_hash = %import.info_hash, torrent_id = %import.id, error = %status_error,
                        "Failed to record authorized torrent post-processing failure");
                }
                Self::failed_source_summary(error)
            }
        }
    }

    fn failed_source_summary(error: anyhow::Error) -> SourceProcessSummary {
        let message = error.to_string();
        SourceProcessSummary {
            success: false,
            files_processed: 0,
            files_failed: 1,
            messages: vec![message.clone()],
            error: Some(message),
        }
    }

    async fn authorize_torrent_import(
        db: &crate::db::Database,
        actor: &AuthUser,
        info_hash: &str,
        library_override: Option<&str>,
    ) -> Result<AuthorizedTorrentImport> {
        let actor = if actor.user_id == "system" && actor.is_admin() {
            actor.clone()
        } else {
            crate::services::graphql::entities::torrent::current_torrent_user(db, actor)
                .await
                .map_err(|error| anyhow::anyhow!(error.message))?
        };
        let mut rows = crate::graphql::entities::Torrent::query(db.pool())
            .filter(crate::graphql::entities::TorrentWhereInput {
                info_hash: Some(StringFilter {
                    eq: Some(info_hash.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .limit(2)
            .fetch_all()
            .await?;
        if rows.len() != 1 {
            anyhow::bail!("Torrent not found or access denied");
        }
        let row = rows.pop().unwrap();
        let link = TorrentImportLink::from_torrent(&row);
        let owner_id = Self::torrent_import_owner(&actor, &link)?;
        if User::get(db.pool(), &owner_id).await?.is_none() {
            anyhow::bail!("Torrent import owner is unavailable");
        }
        for id in [row.library_id.as_deref(), library_override]
            .into_iter()
            .flatten()
        {
            let library = crate::graphql::entities::Library::get(db.pool(), &id.to_string())
                .await?
                .context("Torrent import library is unavailable")?;
            if library.user_id != owner_id {
                anyhow::bail!(
                    "Torrent import library does not belong to the persisted source owner"
                );
            }
        }
        Ok(AuthorizedTorrentImport {
            id: row.id,
            info_hash: row.info_hash,
            owner_id,
            actor,
            link,
        })
    }

    fn torrent_import_owner(caller: &AuthUser, link: &TorrentImportLink) -> Result<String> {
        let owner = link
            .user_id
            .as_deref()
            .context("Torrent import ownership is unknown")?;
        Uuid::parse_str(owner).context("Torrent import has no valid persisted owner")?;
        if !caller.has_role(crate::services::graphql::auth::Role::Member)
            || (!caller.is_admin() && caller.user_id != owner)
        {
            anyhow::bail!("Torrent not found or access denied");
        }
        Ok(owner.to_string())
    }

    async fn process_torrent_source_files_inner(
        &self,
        import: &AuthorizedTorrentImport,
        library_override: Option<String>,
        files: Vec<SourceFileImport>,
        manual: Option<&ManualFileTarget>,
        book_choice: Option<(Option<&str>, &BookImportChoice)>,
        patch: Option<&ImportLinkPatch>,
    ) -> Result<SourceProcessSummary> {
        let info_hash = import.info_hash.as_str();
        let link = &import.link;
        let owner_id = import.owner_id.clone();
        let owner_actor = AuthUser::system_admin();
        let auth_user = &owner_actor;
        let torrent_library_id = match library_override {
            Some(id) => Some(id),
            None => link.library_id.clone(),
        };

        if let Some(library_id) = torrent_library_id.as_deref() {
            let library = self.get_library(library_id).await?;
            if library.user_id != owner_id {
                anyhow::bail!(
                    "Torrent import library does not belong to the persisted source owner"
                );
            }
        }

        let mut diagnostics = ImportDiagnostics::default();
        let mut processed = 0i32;
        let mut failed = 0i32;
        let mut messages = Vec::new();

        // ------------------------------------------------------------------
        // Phase 0 — archives (design.md "Archive Extraction").
        // Scene releases ship media inside .rar/.zip/.7z. Unpack them into a
        // bounded staging directory next to the download and feed the results
        // through exactly the same import path as plain files.
        // ------------------------------------------------------------------
        let PartitionedImportFiles {
            media,
            archives,
            ignored_extensions,
        } = Self::partition_import_files(files);
        diagnostics.ignored_extensions = ignored_extensions;

        let mut import_files: Vec<ImportFile> =
            media.into_iter().map(|file| ImportFile { file }).collect();
        let mut staging_dirs: Vec<PathBuf> = Vec::new();

        if !archives.is_empty() {
            let limits = self.load_extraction_limits().await;
            for archive in &archives {
                let archive_path = PathBuf::from(&archive.source_path);
                if !extract::is_extraction_entry_point(&archive_path) {
                    debug!(
                        archive = %archive.source_path,
                        "Skipping continuation archive volume (extraction starts at the first volume): archive='{}'",
                        archive.source_path
                    );
                    continue;
                }
                // An archive on a share is unpacked on this machine: its volumes are copied to a
                // local scratch folder first (the tools read local files), the output stays local,
                // and the copies are removed afterwards. The download itself is never changed.
                let (extracted, fetched) = if is_remote_path(&archive_path) {
                    match Self::fetch_share_archive(&archive_path, info_hash).await {
                        Ok((local, staging_root, fetched)) => (
                            extract::extract_archive(&local, &staging_root, limits).await,
                            Some(fetched),
                        ),
                        Err(reason) => (
                            Err(extract::ExtractError::Io {
                                archive: Self::display_file_name(&archive.source_path),
                                path: archive.source_path.clone(),
                                reason,
                            }),
                            None,
                        ),
                    }
                } else {
                    let staging_root = Self::extraction_staging_root(&archive_path, info_hash);
                    (
                        extract::extract_archive(&archive_path, &staging_root, limits).await,
                        None,
                    )
                };
                if let Some(fetched) = fetched {
                    let _ = tokio::fs::remove_dir_all(&fetched).await;
                }
                match extracted {
                    Ok(outcome) => {
                        let mut usable = 0usize;
                        for extracted in &outcome.files {
                            let path_string = extracted.path.to_string_lossy().to_string();
                            if !Self::is_supported_import_extension(&path_string) {
                                continue;
                            }
                            usable += 1;
                            import_files.push(ImportFile {
                                file: SourceFileImport {
                                    source_path: path_string,
                                    relative_path: extracted.relative_path.clone(),
                                    file_size: extracted.size as i64,
                                    downloaded_bytes: extracted.size as i64,
                                    file_index: None,
                                },
                            });
                        }
                        info!(
                            info_hash = %info_hash,
                            archive = %archive.source_path,
                            staging_dir = %outcome.staging_dir.display(),
                            extracted = outcome.files.len(),
                            usable,
                            reused = outcome.reused,
                            "Archive '{}' unpacked for import: extracted={}, importable_media={}, staging_dir={}, reused={}",
                            archive.source_path,
                            outcome.files.len(),
                            usable,
                            outcome.staging_dir.display(),
                            outcome.reused
                        );
                        if usable == 0 {
                            diagnostics.archive_failures.push(format!(
                                "'{}' extracted successfully but contained no importable media files",
                                Self::display_file_name(&archive.source_path)
                            ));
                        }
                        staging_dirs.push(outcome.staging_dir);
                    }
                    Err(error) => {
                        failed += 1;
                        let message = error.to_string();
                        error!(
                            info_hash = %info_hash,
                            archive = %archive.source_path,
                            error = %message,
                            "Archive extraction failed for torrent import: info_hash={}, archive='{}', error={}",
                            info_hash,
                            archive.source_path,
                            message
                        );
                        diagnostics.archive_failures.push(message.clone());
                        messages.push(format!(
                            "Extraction failed for '{}': {}",
                            archive.source_path, message
                        ));
                        let _ = self
                            .upsert_pending_file_match(
                                auth_user,
                                &owner_id,
                                info_hash,
                                archive,
                                None,
                                Some(&message),
                                None,
                                None,
                            )
                            .await;
                    }
                }
            }
        }

        // ------------------------------------------------------------------
        // Phase 1 — drop samples, proofs, extras and undersized junk before
        // anything is matched or copied.
        // ------------------------------------------------------------------
        let (import_files, skipped) = Self::filter_import_candidates(import_files);
        for skip in &skipped {
            debug!(
                info_hash = %info_hash,
                source_path = %skip.source_path,
                reason = %skip.reason,
                "Skipping torrent file before import: source_path='{}', reason={}",
                skip.source_path,
                skip.reason
            );
        }
        diagnostics.skipped = skipped;

        // ------------------------------------------------------------------
        // Phase 2 — targeted import. The Torrent row records which wanted item
        // the grab was for; use it instead of re-deriving the target from the
        // filename (design.md Q53).
        // ------------------------------------------------------------------
        let chosen_link;
        let link = match (
            book_choice.and_then(|(audiobook_id, _)| audiobook_id),
            patch,
        ) {
            (Some(audiobook_id), _) => {
                let mut changed = link.clone();
                changed.audiobook_id = Some(audiobook_id.to_string());
                changed.chapter_id = None;
                chosen_link = changed;
                &chosen_link
            }
            (None, Some(patch)) => {
                // What was chosen replaces whatever the grab was for.
                chosen_link = TorrentImportLink {
                    movie_id: patch.movie_id.clone(),
                    show_id: patch.show_id.clone(),
                    season: patch.season,
                    album_id: patch.album_id.clone(),
                    better_copies: patch.better_copies,
                    episode_id: None,
                    track_id: None,
                    chapter_id: None,
                    audiobook_id: None,
                    ..link.clone()
                };
                &chosen_link
            }
            (None, None) => link,
        };
        let linked = if link.has_target() {
            match self
                .plan_linked_import_targets(
                    auth_user,
                    link,
                    &import_files,
                    book_choice.map(|(_, choice)| choice),
                )
                .await
            {
                Ok(plan) => plan,
                Err(error) => {
                    warn!(
                        info_hash = %info_hash,
                        error = %error,
                        "Failed to plan linkage-based import targets, falling back to generic matching: info_hash={}, error={}",
                        info_hash,
                        error
                    );
                    LinkedImportPlan::default()
                }
            }
        } else {
            LinkedImportPlan::default()
        };
        let mut linked = linked;
        if !link.has_target()
            && manual.is_none()
            && book_choice.is_none()
            && let Some(name) = link.name.as_deref()
            && self
                .looks_like_audiobooks(&owner_id, name, &import_files)
                .await
        {
            for import in &import_files {
                if Self::is_audio_import_path(&import.file.source_path) {
                    linked.unmatched_reasons.insert(
                        import.file.source_path.clone(),
                        "Identification will work out which book this is.".to_string(),
                    );
                }
            }
        }
        diagnostics.notes.extend(linked.notes.iter().cloned());
        diagnostics.target_label = linked.target_label.clone();

        let mut library_cache: HashMap<String, LibraryRow> = HashMap::new();
        let mut imported_files: HashMap<String, String> = HashMap::new();

        for import in &import_files {
            let file = &import.file;
            if file.file_size <= 0 || file.downloaded_bytes < file.file_size {
                continue;
            }
            if linked.left_alone.contains(&file.source_path) {
                continue;
            }

            let mut media_file_id = match self
                .ensure_source_media_file(auth_user, &owner_id, info_hash, file)
                .await
            {
                Ok(id) => id,
                Err(error) => {
                    failed += 1;
                    messages.push(format!(
                        "Failed to create source media file for '{}': {}",
                        file.source_path, error
                    ));
                    continue;
                }
            };

            // Manual-match protection (design.md Q9b): `ensure_source_media_file` can
            // resolve to a pre-existing MediaFile row (matched by source path), so this
            // torrent-import path — which writes directly via
            // `apply_media_file_match_update` rather than through `match_media_file` —
            // must not silently overwrite a user's manual match. This applies to
            // linkage-based matches too: a recorded grab target never overrides a
            // human decision.
            match self.get_media_file(&media_file_id).await {
                Ok(existing)
                    if manual.is_none() && existing.match_type.as_deref() == Some("manual") =>
                {
                    debug!(
                        media_file_id = %media_file_id,
                        source_path = %file.source_path,
                        "Skipping torrent import match: media file is manually matched: media_file_id={}, source_path={}",
                        media_file_id,
                        file.source_path
                    );
                    messages.push(format!(
                        "Skipped '{}': media file is manually matched and protected from automatic import matching",
                        file.source_path
                    ));
                    continue;
                }
                Ok(_) => {}
                Err(error) => {
                    failed += 1;
                    messages.push(format!(
                        "Failed to load media file for '{}': {}",
                        file.source_path, error
                    ));
                    continue;
                }
            }

            let selected = if let Some(manual) = manual {
                Some(SelectedImportMatch {
                    library: self.get_library(&manual.library_id).await?,
                    candidate: MatchCandidate {
                        target_type: manual.target_type.clone(),
                        target_id: manual.target_id.clone(),
                        target_name: None,
                        score: 1.0,
                        reason: Some(MANUAL_MATCH_TYPE.to_string()),
                        wanted: None,
                    },
                })
            } else if let Some(target) = linked.targets.get(&file.source_path) {
                let library = match library_cache.get(&target.library_id) {
                    Some(library) => library.clone(),
                    None => match self.get_library(&target.library_id).await {
                        Ok(library) => {
                            library_cache.insert(target.library_id.clone(), library.clone());
                            library
                        }
                        Err(error) => {
                            failed += 1;
                            let reason = format!(
                                "Failed to load library {} for linked import target: {error}",
                                target.library_id
                            );
                            diagnostics.unmatched.push(UnmatchedImportFile {
                                source_path: file.source_path.clone(),
                                reason: reason.clone(),
                            });
                            let _ = self
                                .upsert_pending_file_match(
                                    auth_user,
                                    &owner_id,
                                    info_hash,
                                    file,
                                    None,
                                    Some(&reason),
                                    None,
                                    None,
                                )
                                .await;
                            continue;
                        }
                    },
                };
                info!(
                    media_file_id = %media_file_id,
                    source_path = %file.source_path,
                    library_id = %library.id,
                    target_type = %target.target_type,
                    target_id = %target.target_id,
                    match_type = target.match_type,
                    "Using recorded torrent linkage for import: source='{}', target_type={}, target='{}', match_type={}",
                    file.source_path,
                    target.target_type,
                    target.target_name.as_deref().unwrap_or(&target.target_id),
                    target.match_type
                );
                Some(SelectedImportMatch {
                    library,
                    candidate: MatchCandidate {
                        target_type: target.target_type.clone(),
                        target_id: target.target_id.clone(),
                        target_name: target.target_name.clone(),
                        score: 1.0,
                        reason: Some(target.reason.clone()),
                        wanted: None,
                    },
                })
            } else if linked.unmatched_reasons.contains_key(&file.source_path) {
                // The torrent's recorded linkage covers this file's release but
                // could not place this particular file. Falling back to the
                // generic matcher here would only invite a cross-item mismatch,
                // so report the specific reason instead (design.md Q71/Q73).
                None
            } else {
                let libraries = match self
                    .candidate_libraries_for_import(
                        auth_user,
                        &owner_id,
                        torrent_library_id.as_deref(),
                        file,
                    )
                    .await
                {
                    Ok(libraries) => libraries,
                    Err(error) => {
                        failed += 1;
                        let reason = format!("Failed to resolve candidate libraries: {error}");
                        diagnostics.unmatched.push(UnmatchedImportFile {
                            source_path: file.source_path.clone(),
                            reason: reason.clone(),
                        });
                        let _ = self
                            .upsert_pending_file_match(
                                auth_user,
                                &owner_id,
                                info_hash,
                                file,
                                None,
                                Some(&reason),
                                None,
                                None,
                            )
                            .await;
                        continue;
                    }
                };

                self.select_import_match(
                    auth_user,
                    &media_file_id,
                    file,
                    libraries,
                    torrent_library_id.is_some(),
                )
                .await?
            };

            let Some(selected) = selected else {
                let reason = linked
                    .unmatched_reasons
                    .get(&file.source_path)
                    .cloned()
                    .unwrap_or_else(|| "No safe automatic match found".to_string());
                diagnostics.unmatched.push(UnmatchedImportFile {
                    source_path: file.source_path.clone(),
                    reason: reason.clone(),
                });
                let _ = self
                    .upsert_pending_file_match(
                        auth_user,
                        &owner_id,
                        info_hash,
                        file,
                        None,
                        Some(&reason),
                        None,
                        None,
                    )
                    .await;
                messages.push(format!("{} for '{}'", reason, file.source_path));
                continue;
            };

            if selected.library.user_id != owner_id {
                anyhow::bail!(
                    "Selected import target does not belong to the persisted source owner"
                );
            }

            let source_path = PathBuf::from(&file.source_path);
            let whole_book = selected
                .candidate
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains(WHOLE_BOOK_REASON));
            let planned_path = self
                .plan_media_target_path(
                    auth_user,
                    &selected.library,
                    &selected.candidate.target_type,
                    &selected.candidate.target_id,
                    &source_path,
                )
                .await;
            let planned_path = match planned_path {
                Ok(path) if whole_book => {
                    self.named_after_book(&selected.candidate.target_id, path)
                        .await
                }
                other => other,
            };
            let planned_path = match (
                planned_path,
                linked
                    .targets
                    .get(&file.source_path)
                    .and_then(|target| target.file_stem.as_deref()),
            ) {
                (Ok(path), Some(stem)) if manual.is_none() => {
                    let extension = source_path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .unwrap_or("mp3");
                    Ok(path.with_file_name(format!("{stem}.{extension}")))
                }
                (other, _) => other,
            };
            // A pattern like `{author}/{title}.{ext}` names every file of a book the same.
            let planned_path = match (planned_path, linked.book_title.as_deref()) {
                (Ok(path), Some(title))
                    if manual.is_none()
                        && path.file_stem().and_then(|stem| stem.to_str()).is_some_and(
                            |stem| {
                                release_match::normalize_words(stem)
                                    == release_match::normalize_words(title)
                            },
                        ) =>
                {
                    match linked.book_file_numbers.get(&file.source_path) {
                        Some(number) => {
                            let extension = path
                                .extension()
                                .and_then(|extension| extension.to_str())
                                .unwrap_or("mp3")
                                .to_string();
                            Ok(path.with_file_name(format!(
                                "{} - Part {number:02}.{extension}",
                                sanitize_for_filename(title)
                            )))
                        }
                        None => Ok(path),
                    }
                }
                (other, _) => other,
            };
            let target_path = match planned_path {
                Ok(path) => path,
                Err(error) => {
                    failed += 1;
                    diagnostics.unmatched.push(UnmatchedImportFile {
                        source_path: file.source_path.clone(),
                        reason: format!("failed to plan target path: {error}"),
                    });
                    let _ = self
                        .upsert_pending_file_match(
                            auth_user,
                            &owner_id,
                            info_hash,
                            file,
                            Some(&selected.candidate),
                            Some(&format!("Failed to plan target path: {error}")),
                            None,
                            None,
                        )
                        .await;
                    continue;
                }
            };

            // A first import moves its record from the download's path to the library's, so importing
            // again finds nothing at the download's path and starts a new record. If the library
            // already has a record for this file, that is the one: without this, every re-import
            // of a download added another record for the same file.
            match self
                .settle_on_library_record(
                    &owner_actor,
                    &media_file_id,
                    &selected.library.id,
                    &target_path,
                )
                .await
            {
                Ok(existing) => media_file_id = existing,
                Err(error) => {
                    warn!(media_file_id = %media_file_id, error = %error, "Could not check for an existing library record");
                }
            }

            // Keep the complete extraction intact until the batch succeeds.
            // A conflict can create an approval referencing a staged source,
            // and retries must be able to reuse its completed manifest.
            // Persist the selected source identity before a conflict can produce an
            // approval notification. Approval rederives it from this row and TorrentFile.
            self.upsert_pending_file_match(
                auth_user,
                &owner_id,
                info_hash,
                file,
                Some(&selected.candidate),
                None,
                None,
                None,
            )
            .await?;
            if let Err(error) = self
                .materialize_torrent_file(
                    &source_path,
                    &target_path,
                    auth_user,
                    &selected.library,
                    false,
                )
                .await
            {
                failed += 1;
                diagnostics.unmatched.push(UnmatchedImportFile {
                    source_path: file.source_path.clone(),
                    reason: error.to_string(),
                });
                let _ = self
                    .upsert_pending_file_match(
                        auth_user,
                        &owner_id,
                        info_hash,
                        file,
                        Some(&selected.candidate),
                        Some(&error.to_string()),
                        None,
                        None,
                    )
                    .await;
                continue;
            }

            let relative_path = target_path
                .strip_prefix(&selected.library.path)
                .ok()
                .map(|path| path.to_string_lossy().to_string())
                .or_else(|| {
                    target_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.to_string())
                });
            let target_path_string = target_path.to_string_lossy().to_string();
            let match_type = Self::import_match_type(&selected.candidate);
            if let Err(error) = self
                .apply_media_file_match_update(
                    auth_user,
                    &media_file_id,
                    &selected.library.id,
                    &selected.candidate.target_type,
                    &selected.candidate.target_id,
                    Some((&target_path_string, relative_path.as_deref())),
                    match_type,
                    None,
                )
                .await
            {
                failed += 1;
                let _ = self
                    .upsert_pending_file_match(
                        auth_user,
                        &owner_id,
                        info_hash,
                        file,
                        Some(&selected.candidate),
                        Some(&format!("Failed to update imported media file: {error}")),
                        None,
                        None,
                    )
                    .await;
                continue;
            }

            if let Err(error) = self
                .queue_analyze_job(&media_file_id, &target_path_string)
                .await
            {
                warn!(
                    media_file_id = %media_file_id,
                    source_path = %file.source_path,
                    target_path = %target_path_string,
                    error = %error,
                    "Failed to queue analysis for imported torrent file: media_file_id={}, target='{}', error={}",
                    media_file_id,
                    target_path_string,
                    error
                );
            }

            if whole_book
                && let Err(error) = self
                    .link_whole_book(auth_user, &selected.candidate.target_id, &media_file_id)
                    .await
            {
                warn!(media_file_id = %media_file_id, error = %error, "Could not link every chapter of a whole-book file");
            }
            imported_files.insert(file.source_path.clone(), media_file_id.clone());

            self.upsert_pending_file_match(
                auth_user,
                &owner_id,
                info_hash,
                file,
                Some(&selected.candidate),
                None,
                Some(
                    &chrono::Utc::now()
                        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                        .to_string(),
                ),
                None,
            )
            .await?;
            processed += 1;
            messages.push(format!(
                "Imported '{}' to '{}'",
                file.source_path, target_path_string
            ));
        }

        if let Some(writes) = linked.book_writes.as_ref()
            && !imported_files.is_empty()
            && let Err(error) = self
                .write_book_chapters(auth_user, writes, &imported_files)
                .await
        {
            failed += 1;
            warn!(info_hash = %info_hash, error = %error, "Could not place chapters on imported files");
            messages.push(format!("Could not place chapters on the files: {error}"));
        }

        // Pending approvals/retries still need the verified extracted files.
        // Only discard our temporary copies after the whole batch succeeds;
        // the original archives and torrent payloads are always preserved.
        if failed == 0 {
            for staging_dir in &staging_dirs {
                extract::cleanup_staging_dir(staging_dir).await;
            }
        }

        let status = if processed > 0 && failed == 0 {
            "completed"
        } else if processed > 0 {
            "partial"
        } else {
            "unmatched"
        };
        let mut error = Self::summarize_import_outcome(&diagnostics, processed, failed);
        // A count of failures doesn't help anyone fix them.
        if failed > 0
            && let Some(first) = messages
                .iter()
                .find(|message| !message.starts_with("Imported"))
                .cloned()
                .or_else(|| {
                    diagnostics
                        .unmatched
                        .first()
                        .map(|file| format!("{}: {}", file.source_path, file.reason))
                })
        {
            warn!(info_hash = %info_hash, failed, first = %first, "Some files could not be imported");
            error = error.map(|summary| format!("{summary}. The first: {first}"));
        }
        let mut status = status;
        if let Some(reason) = linked.hard_failure.as_ref() {
            status = "failed";
            error = Some(reason.clone());
        } else if let Some(reason) = linked.review.as_ref() {
            status = "needs_review";
            error = Some(reason.clone());
        } else if let Some(summary) = linked.summary.as_ref()
            && processed > 0
        {
            // A book that landed in full is done, however many other books came with it, but the
            // others are still worth saying.
            if failed == 0 && diagnostics.unmatched.is_empty() {
                error = None;
            } else {
                status = "partial";
                error = Some(summary.clone());
            }
        }
        // Everything here is already held: nothing to add isn't a failure.
        let all_held = processed == 0
            && failed == 0
            && !diagnostics.unmatched.is_empty()
            && diagnostics
                .unmatched
                .iter()
                .all(|file| held_by_library(&file.reason))
            && linked.hard_failure.is_none()
            && linked.review.is_none();
        if all_held {
            status = "completed";
            error = None;
            messages.push(format!(
                "Nothing to add: the library already has all {} of these",
                diagnostics.unmatched.len()
            ));
        }
        if manual.is_some() {
            if processed == 0 {
                // One file not landing says nothing about the rest of the download.
                return Ok(SourceProcessSummary {
                    success: false,
                    files_processed: 0,
                    files_failed: failed,
                    messages,
                    error,
                });
            }
            (status, error) = self.status_after_manual_import(import).await?;
        }
        self.update_torrent_post_process_status(
            import,
            status,
            error.as_deref(),
            processed,
            failed,
        )
        .await?;

        Ok(SourceProcessSummary {
            success: all_held || (failed == 0 && processed > 0),
            files_processed: processed,
            files_failed: failed,
            messages,
            error,
        })
    }

    /// Staging root for archives belonging to one torrent:
    /// `<archive dir>/.librarian-extract/<info hash>/`.
    /// Where archives from a share are unpacked: a scratch folder on this machine, per torrent.
    fn share_extraction_root(info_hash: &str) -> PathBuf {
        let cache = std::env::var("CACHE_PATH").unwrap_or_else(|_| "./data/cache".to_string());
        let short_hash: String = info_hash.chars().take(40).collect();
        PathBuf::from(cache).join("share-extract").join(short_hash)
    }

    /// Copy an archive set on a share (the entry volume and its other volumes) to a local scratch
    /// folder. Returns the local entry volume, the local staging root to unpack into, and the
    /// folder of copies to remove afterwards. Nothing is copied when the archive was unpacked
    /// completely before.
    async fn fetch_share_archive(
        archive_path: &Path,
        info_hash: &str,
    ) -> std::result::Result<(PathBuf, PathBuf, PathBuf), String> {
        let root = Self::share_extraction_root(info_hash);
        let staging_root = root.join("unpacked");
        let fetched = root.join("archive");
        let name = archive_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .ok_or_else(|| "The archive has no file name".to_string())?;
        let local_entry = fetched.join(&name);
        if extract::has_completed_extraction(&local_entry, &staging_root) {
            return Ok((local_entry, staging_root, fetched));
        }
        let folder = archive_path
            .parent()
            .ok_or_else(|| "The archive has no folder".to_string())?;
        let storage = library_storage();
        let listing = storage.list_dir(folder).await.map_err(|error| {
            format!("The archive's folder on the share cannot be read: {error}")
        })?;
        let siblings: Vec<String> = listing.iter().map(|entry| entry.name.clone()).collect();
        let set = extract::archive_set(&name, &siblings);
        let bytes: u64 = listing
            .iter()
            .filter(|entry| set.contains(&entry.name))
            .filter_map(|entry| entry.meta.as_ref().map(|meta| meta.size))
            .sum();
        let _ = tokio::fs::remove_dir_all(&fetched).await;
        tokio::fs::create_dir_all(&fetched)
            .await
            .map_err(|error| format!("Could not make '{}': {error}", fetched.display()))?;
        if let Some(free) = extract::available_space_bytes(&fetched)
            && free < bytes.saturating_mul(2)
        {
            return Err(format!(
                "Not enough space on this server to unpack an archive from the share: it needs about {} GB free in '{}' (the archive and what it holds), and {} GB are",
                bytes.saturating_mul(2) / 1_000_000_000 + 1,
                root.display(),
                free / 1_000_000_000
            ));
        }
        for volume in &set {
            if let Err(error) = storage
                .download_to_local(folder.join(volume), &fetched.join(volume))
                .await
            {
                let _ = tokio::fs::remove_dir_all(&fetched).await;
                return Err(format!(
                    "Could not copy '{volume}' from the share to unpack it: {error}"
                ));
            }
        }
        Ok((local_entry, staging_root, fetched))
    }

    fn extraction_staging_root(archive_path: &Path, info_hash: &str) -> PathBuf {
        let parent = archive_path
            .parent()
            .map(|path| path.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        let short_hash: String = info_hash.chars().take(40).collect();
        parent.join(extract::STAGING_DIR_NAME).join(short_hash)
    }

    /// Read `extract.*` app settings, falling back to the documented defaults.
    async fn load_extraction_limits(&self) -> extract::ExtractionLimits {
        let Ok(auth_user) = self.system_auth_user(None).await else {
            return extract::ExtractionLimits::default();
        };
        let data = match self
            .execute_graphql(
                &auth_user,
                r#"query ExtractionSettings {
                    AppSettings: appSettings(
                        where: { category: { eq: "extract" } }
                        page: { limit: 50 }
                    ) {
                        Edges: edges { Node: node { Key: key Value: value } }
                    }
                }"#,
                serde_json::json!({}),
            )
            .await
        {
            Ok(data) => data,
            Err(error) => {
                debug!(
                    error = %error,
                    "Extraction settings unavailable, using defaults (max_unpacked_gb={}, max_entries={}): error={}",
                    extract::DEFAULT_MAX_UNPACKED_GB,
                    extract::DEFAULT_MAX_ENTRIES,
                    error
                );
                return extract::ExtractionLimits::default();
            }
        };

        let mut settings = HashMap::<String, String>::new();
        if let Some(edges) = data
            .get("AppSettings")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
        {
            for edge in edges {
                if let Some(node) = edge.get("Node")
                    && let (Some(key), Some(value)) = (
                        node.get("Key").and_then(|v| v.as_str()),
                        node.get("Value").and_then(|v| v.as_str()),
                    )
                {
                    settings.insert(key.to_string(), value.to_string());
                }
            }
        }

        extract::ExtractionLimits::from_settings(
            settings
                .get("extract.max_unpacked_gb")
                .and_then(|value| value.trim().parse::<u64>().ok()),
            settings
                .get("extract.max_entries")
                .and_then(|value| value.trim().parse::<usize>().ok()),
        )
    }

    /// Split the torrent's files into importable media, archives to unpack,
    /// and everything else (nfo/srt/jpg/…), which is simply not media.
    fn partition_import_files(files: Vec<SourceFileImport>) -> PartitionedImportFiles {
        let mut media = Vec::new();
        let mut archives = Vec::new();
        let mut ignored_extensions: BTreeSet<String> = BTreeSet::new();
        for file in files {
            if Self::is_supported_import_extension(&file.source_path) {
                media.push(file);
            } else if extract::is_archive_path(&file.source_path) {
                archives.push(file);
            } else {
                ignored_extensions.insert(
                    Path::new(&file.source_path)
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| format!(".{}", ext.to_ascii_lowercase()))
                        .unwrap_or_else(|| "(no extension)".to_string()),
                );
            }
        }
        PartitionedImportFiles {
            media,
            archives,
            ignored_extensions,
        }
    }

    /// Sample/proof/extras and undersized-file filtering (design.md Q4/Q54).
    ///
    /// Pure so it can be unit tested: a release's own contents decide what is
    /// "small", which is why a single-file 20 MB webisode release is kept
    /// while a 20 MB `sample.mkv` sitting next to a 4 GB main file is not.
    fn filter_import_candidates(
        files: Vec<ImportFile>,
    ) -> (Vec<ImportFile>, Vec<SkippedImportFile>) {
        let largest_video = files
            .iter()
            .filter(|item| Self::is_video_import_path(&item.file.source_path))
            .map(|item| item.file.file_size)
            .max()
            .unwrap_or(0);
        let largest_audio = files
            .iter()
            .filter(|item| Self::is_audio_import_path(&item.file.source_path))
            .map(|item| item.file.file_size)
            .max()
            .unwrap_or(0);

        let mut kept = Vec::new();
        let mut skipped = Vec::new();
        for item in files {
            let path = item.file.source_path.clone();
            let is_video = Self::is_video_import_path(&path);
            let largest_of_kind = if is_video {
                largest_video
            } else {
                largest_audio
            };

            if let Some(reason) = Self::sample_or_extras_directory(&path) {
                skipped.push(SkippedImportFile {
                    source_path: path,
                    reason: reason.to_string(),
                });
                continue;
            }

            if Self::file_name_has_sample_token(&path) && item.file.file_size < largest_of_kind {
                skipped.push(SkippedImportFile {
                    source_path: path,
                    reason: "named as a sample and a larger main file is present in the release"
                        .to_string(),
                });
                continue;
            }

            if is_video
                && item.file.file_size < MIN_IMPORT_VIDEO_BYTES
                && largest_video >= MIN_IMPORT_VIDEO_BYTES
            {
                skipped.push(SkippedImportFile {
                    source_path: path,
                    reason: format!(
                        "video file is only {} bytes, below the {} MB import floor, while a larger main video file is present",
                        item.file.file_size,
                        MIN_IMPORT_VIDEO_BYTES / (1024 * 1024)
                    ),
                });
                continue;
            }

            kept.push(item);
        }
        (kept, skipped)
    }

    /// True when any *directory* on the path is a sample/proof/extras folder.
    fn sample_or_extras_directory(path: &str) -> Option<&'static str> {
        let path_obj = Path::new(path);
        let parent = path_obj.parent()?;
        for component in parent.components() {
            let Component::Normal(part) = component else {
                continue;
            };
            let Some(name) = part.to_str() else {
                continue;
            };
            let normalized = name.trim().to_ascii_lowercase();
            if matches!(
                normalized.as_str(),
                "sample"
                    | "samples"
                    | "proof"
                    | "proofs"
                    | "extra"
                    | "extras"
                    | "featurette"
                    | "featurettes"
                    | "bonus"
                    | "screens"
                    | "screenshot"
                    | "screenshots"
                    | "trailer"
                    | "trailers"
            ) {
                return Some("file is inside a sample/proof/extras directory");
            }
        }
        None
    }

    /// `sample` / `proof` / `trailer` as a standalone token in the file name.
    fn file_name_has_sample_token(path: &str) -> bool {
        let Some(stem) = Path::new(path).file_stem().and_then(|value| value.to_str()) else {
            return false;
        };
        let lowered = stem.to_ascii_lowercase();
        lowered
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|token| matches!(token, "sample" | "proof" | "trailer"))
    }

    fn is_video_import_path(path: &str) -> bool {
        Self::compatible_library_types_for_path(path).contains("movies")
    }

    fn is_audio_import_path(path: &str) -> bool {
        Self::compatible_library_types_for_path(path).contains("music")
    }

    fn display_file_name(path: &str) -> String {
        Path::new(path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(path)
            .to_string()
    }

    /// `MediaFile.match_type` / `PendingFileMatch.match_type` value that makes
    /// the source of an import decision auditable after the fact.
    fn import_match_type(candidate: &MatchCandidate) -> &'static str {
        let reason = candidate.reason.as_deref().unwrap_or_default();
        if reason.starts_with(MANUAL_MATCH_TYPE) {
            MANUAL_MATCH_TYPE
        } else if reason.starts_with(TORRENT_LINK_PARSED_MATCH_TYPE) {
            TORRENT_LINK_PARSED_MATCH_TYPE
        } else if reason.starts_with(TORRENT_LINK_MATCH_TYPE) {
            TORRENT_LINK_MATCH_TYPE
        } else if reason.contains("ollama") {
            "ollama"
        } else {
            "auto"
        }
    }

    /// Build the `Torrent.post_process_error` text for a finished import so
    /// the UI can explain *why* nothing (or only some things) landed
    /// (design.md Q55).
    fn summarize_import_outcome(
        diagnostics: &ImportDiagnostics,
        processed: i32,
        failed: i32,
    ) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();

        if !diagnostics.archive_failures.is_empty() {
            parts.push(format!(
                "archive extraction failed: {}",
                diagnostics.archive_failures.join("; ")
            ));
        }

        // Files the library already has aren't failures: say how many, apart from the rest.
        let (held, unmatched): (Vec<&UnmatchedImportFile>, Vec<&UnmatchedImportFile>) = diagnostics
            .unmatched
            .iter()
            .partition(|item| held_by_library(&item.reason));
        let same = held
            .iter()
            .filter(|item| item.reason.starts_with(ALREADY_IN_LIBRARY))
            .count();
        if same > 0 {
            parts.push(format!("{same} file(s) already in the library"));
        }
        if held.len() > same {
            parts.push(format!(
                "{} file(s) left alone: the library keeps its own copies",
                held.len() - same
            ));
        }
        if !unmatched.is_empty() {
            let names = unmatched
                .iter()
                .take(5)
                .map(|item| Self::display_file_name(&item.source_path))
                .collect::<Vec<_>>()
                .join(", ");
            let overflow = unmatched.len().saturating_sub(5);
            let target = diagnostics
                .target_label
                .as_deref()
                .map(|label| format!(" any item of {label}"))
                .unwrap_or_else(|| " any library item".to_string());
            let mut reasons: Vec<String> = Vec::new();
            for item in &unmatched {
                if !reasons.contains(&item.reason) {
                    reasons.push(item.reason.clone());
                }
            }
            parts.push(format!(
                "{} file(s) did not match{}: {}{} ({})",
                unmatched.len(),
                target,
                names,
                if overflow > 0 {
                    format!(" and {overflow} more")
                } else {
                    String::new()
                },
                reasons.into_iter().take(3).collect::<Vec<_>>().join("; ")
            ));
        }

        if processed == 0 && !diagnostics.skipped.is_empty() {
            let names = diagnostics
                .skipped
                .iter()
                .take(5)
                .map(|item| Self::display_file_name(&item.source_path))
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!(
                "{} file(s) skipped as sample/extras/undersized: {}",
                diagnostics.skipped.len(),
                names
            ));
        }

        // Informational notes ("8 of 12 chapters have a file") only belong on
        // the torrent when something actually needs explaining; a clean import
        // must leave `post_process_error` null (Q39: partial fulfilment is OK).
        if processed == 0 || failed > 0 || !diagnostics.unmatched.is_empty() {
            for note in &diagnostics.notes {
                parts.push(note.clone());
            }
        }

        if processed == 0
            && parts.is_empty()
            && failed == 0
            && !diagnostics.ignored_extensions.is_empty()
        {
            parts.push(format!(
                "no supported media files: this torrent only contains {}",
                diagnostics
                    .ignored_extensions
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        if failed > 0 {
            parts.push(format!(
                "{failed} file(s) failed during torrent post-processing"
            ));
        }

        if parts.is_empty() && processed == 0 {
            parts.push("no supported media files were found in this torrent".to_string());
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("; "))
        }
    }

    /// Build per-file import targets from the linkage recorded on the Torrent
    /// row (design.md Q53). Returns an empty plan when the linkage cannot be
    /// resolved so the caller falls back to the generic matcher.
    async fn plan_linked_import_targets(
        &self,
        auth_user: &AuthUser,
        link: &TorrentImportLink,
        files: &[ImportFile],
        book_choice: Option<&BookImportChoice>,
    ) -> Result<LinkedImportPlan> {
        let mut plan = LinkedImportPlan::default();
        let mut video: Vec<&ImportFile> = files
            .iter()
            .filter(|item| Self::is_video_import_path(&item.file.source_path))
            .collect();
        let mut audio: Vec<&ImportFile> = files
            .iter()
            .filter(|item| Self::is_audio_import_path(&item.file.source_path))
            .collect();
        video.sort_by(|a, b| {
            Self::natural_sort_key(&a.file.source_path)
                .cmp(&Self::natural_sort_key(&b.file.source_path))
        });
        audio.sort_by(|a, b| {
            Self::natural_sort_key(&a.file.source_path)
                .cmp(&Self::natural_sort_key(&b.file.source_path))
        });

        if let Some(movie_id) = link.movie_id.as_deref() {
            self.plan_linked_movie(auth_user, movie_id, &video, &mut plan)
                .await?;
        } else if link.show_id.is_some() || link.episode_id.is_some() {
            self.plan_linked_episodes(auth_user, link, &video, &mut plan)
                .await?;
        } else if let Some(album_id) = link.album_id.as_deref() {
            self.plan_linked_album(auth_user, album_id, &audio, &mut plan, link.better_copies)
                .await?;
        } else if let Some(audiobook_id) = link.audiobook_id.as_deref() {
            self.plan_linked_audiobook(auth_user, audiobook_id, &audio, &mut plan, book_choice)
                .await?;
        } else if let Some(track_id) = link.track_id.as_deref() {
            // Single-track grab with no album context.
            self.plan_linked_single(auth_user, "Track", track_id, &audio, &mut plan)
                .await?;
        } else if let Some(chapter_id) = link.chapter_id.as_deref() {
            self.plan_linked_single(auth_user, "Chapter", chapter_id, &audio, &mut plan)
                .await?;
        }

        Ok(plan)
    }

    async fn plan_linked_movie(
        &self,
        auth_user: &AuthUser,
        movie_id: &str,
        video: &[&ImportFile],
        plan: &mut LinkedImportPlan,
    ) -> Result<()> {
        let data = self
            .execute_graphql(
                auth_user,
                r#"query ImportLinkMovie($id: String!) {
                    Movie: movie(id: $id) { Id: id Title: title LibraryId: libraryId }
                }"#,
                serde_json::json!({ "id": movie_id }),
            )
            .await?;
        let node = data.get("Movie");
        let (Some(title), Some(library_id)) = (
            node.and_then(|m| m.get("Title")).and_then(|v| v.as_str()),
            node.and_then(|m| m.get("LibraryId"))
                .and_then(|v| v.as_str()),
        ) else {
            plan.notes.push(format!(
                "torrent is linked to movie {movie_id} but that movie no longer exists"
            ));
            return Ok(());
        };
        plan.target_label = Some(format!("'{title}'"));

        // A movie release has exactly one feature file: the largest video.
        let Some(main) = video.iter().max_by_key(|item| item.file.file_size) else {
            plan.notes
                .push(format!("no video file found for movie '{title}'"));
            return Ok(());
        };
        if video.len() > 1 {
            debug!(
                movie_id = %movie_id,
                candidates = video.len(),
                chosen = %main.file.source_path,
                "Movie-linked torrent has {} video files; importing the largest as '{}': chosen='{}'",
                video.len(),
                title,
                main.file.source_path
            );
        }
        for item in video {
            if item.file.source_path == main.file.source_path {
                continue;
            }
            plan.unmatched_reasons.insert(
                item.file.source_path.clone(),
                format!("not the main feature file for movie '{title}'"),
            );
        }
        plan.targets.insert(
            main.file.source_path.clone(),
            LinkedImportTarget {
                library_id: library_id.to_string(),
                target_type: "Movie".to_string(),
                target_id: movie_id.to_string(),
                target_name: Some(title.to_string()),
                file_stem: None,
                match_type: TORRENT_LINK_MATCH_TYPE,
                reason: format!(
                    "{TORRENT_LINK_MATCH_TYPE}: torrent was grabbed for movie '{title}'"
                ),
            },
        );
        Ok(())
    }

    async fn plan_linked_episodes(
        &self,
        auth_user: &AuthUser,
        link: &TorrentImportLink,
        video: &[&ImportFile],
        plan: &mut LinkedImportPlan,
    ) -> Result<()> {
        // Resolve the show: either recorded directly (season packs) or via the
        // linked episode.
        let mut show_id = link.show_id.clone();
        let mut linked_episode: Option<LinkedEpisodeRow> = None;
        if let Some(episode_id) = link.episode_id.as_deref() {
            let data = self
                .execute_graphql(
                    auth_user,
                    r#"query ImportLinkEpisode($id: String!) {
                        Episode: episode(id: $id) {
                            Id: id
                            Title: title
                            Season: season
                            Episode: episode
                            ShowId: showId
                        }
                    }"#,
                    serde_json::json!({ "id": episode_id }),
                )
                .await?;
            if let Some(node) = data.get("Episode") {
                if show_id.is_none() {
                    show_id = node
                        .get("ShowId")
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string());
                }
                linked_episode = Some(LinkedEpisodeRow {
                    id: episode_id.to_string(),
                    season: node
                        .get("Season")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32),
                    episode: node
                        .get("Episode")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32),
                    title: node
                        .get("Title")
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string()),
                });
            }
        }

        let Some(show_id) = show_id else {
            plan.notes.push(
                "torrent is linked to a TV target whose show could not be resolved".to_string(),
            );
            return Ok(());
        };

        let data = self
            .execute_graphql(
                auth_user,
                r#"query ImportLinkShow($id: String!) {
                    Show: show(id: $id) {
                        Id: id
                        Name: name
                        LibraryId: libraryId
                        Episodes: episodes(page: { limit: 2000 }) {
                            Edges: edges {
                                Node: node {
                                    Id: id
                                    Season: season
                                    Episode: episode
                                    Title: title
                                }
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "id": show_id }),
            )
            .await?;
        let show = data.get("Show");
        let (Some(show_name), Some(library_id)) = (
            show.and_then(|s| s.get("Name")).and_then(|v| v.as_str()),
            show.and_then(|s| s.get("LibraryId"))
                .and_then(|v| v.as_str()),
        ) else {
            plan.notes.push(format!(
                "torrent is linked to show {show_id} but that show no longer exists"
            ));
            return Ok(());
        };
        plan.target_label = Some(format!("'{show_name}'"));

        let mut episodes: Vec<LinkedEpisodeRow> = Vec::new();
        if let Some(edges) = show
            .and_then(|s| s.get("Episodes"))
            .and_then(|e| e.get("Edges"))
            .and_then(|v| v.as_array())
        {
            for edge in edges {
                let Some(node) = edge.get("Node") else {
                    continue;
                };
                let Some(id) = node.get("Id").and_then(|v| v.as_str()) else {
                    continue;
                };
                episodes.push(LinkedEpisodeRow {
                    id: id.to_string(),
                    season: node
                        .get("Season")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32),
                    episode: node
                        .get("Episode")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32),
                    title: node
                        .get("Title")
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string()),
                });
            }
        }

        if video.is_empty() {
            plan.notes
                .push(format!("no video files found for show '{show_name}'"));
            return Ok(());
        }

        // Single video file + a recorded episode: that file IS that episode,
        // regardless of what its name claims.
        if video.len() == 1
            && let Some(episode) = linked_episode.as_ref()
        {
            let label = Self::episode_display_name(show_name, episode);
            plan.targets.insert(
                video[0].file.source_path.clone(),
                LinkedImportTarget {
                    library_id: library_id.to_string(),
                    target_type: "Episode".to_string(),
                    target_id: episode.id.clone(),
                    target_name: Some(label.clone()),
                    file_stem: None,
                    match_type: TORRENT_LINK_MATCH_TYPE,
                    reason: format!("{TORRENT_LINK_MATCH_TYPE}: torrent was grabbed for {label}"),
                },
            );
            return Ok(());
        }

        // Season pack / multi-episode release: parse each file and associate.
        let fallback_season = link
            .season
            .or_else(|| linked_episode.as_ref().and_then(|episode| episode.season))
            .or_else(|| {
                link.name
                    .as_deref()
                    .and_then(Self::parse_season_from_release_name)
            });

        let mut used: HashSet<String> = HashSet::new();
        for item in video {
            let hint = Self::parse_episode_hint(&item.file.source_path);
            let season = hint.season.or(fallback_season);
            let (Some(season), Some(episode_number)) = (season, hint.episode) else {
                plan.unmatched_reasons.insert(
                    item.file.source_path.clone(),
                    format!(
                        "could not parse a season/episode number from the filename for show '{show_name}'"
                    ),
                );
                continue;
            };
            let Some(episode) = episodes
                .iter()
                .find(|row| row.season == Some(season) && row.episode == Some(episode_number))
            else {
                plan.unmatched_reasons.insert(
                    item.file.source_path.clone(),
                    format!("'{show_name}' has no episode S{season:02}E{episode_number:02}"),
                );
                continue;
            };
            if !used.insert(episode.id.clone()) {
                plan.unmatched_reasons.insert(
                    item.file.source_path.clone(),
                    format!(
                        "another file in this release already matched '{}'",
                        Self::episode_display_name(show_name, episode)
                    ),
                );
                continue;
            }
            let label = Self::episode_display_name(show_name, episode);
            plan.targets.insert(
                item.file.source_path.clone(),
                LinkedImportTarget {
                    library_id: library_id.to_string(),
                    target_type: "Episode".to_string(),
                    target_id: episode.id.clone(),
                    target_name: Some(label.clone()),
                    file_stem: None,
                    match_type: TORRENT_LINK_PARSED_MATCH_TYPE,
                    reason: format!(
                        "{TORRENT_LINK_PARSED_MATCH_TYPE}: torrent is linked to '{show_name}' and the filename parsed as S{season:02}E{episode_number:02}"
                    ),
                },
            );
        }

        Ok(())
    }

    fn episode_display_name(show_name: &str, episode: &LinkedEpisodeRow) -> String {
        match (episode.season, episode.episode) {
            (Some(season), Some(number)) => match episode.title.as_deref() {
                Some(title) if !title.is_empty() => {
                    format!("{show_name} S{season:02}E{number:02} - {title}")
                }
                _ => format!("{show_name} S{season:02}E{number:02}"),
            },
            _ => show_name.to_string(),
        }
    }

    /// Pull a season number out of a release name (`...S03...`, `Season 3`).
    fn parse_season_from_release_name(name: &str) -> Option<i32> {
        let cleaned = name.replace(['.', '_'], " ");
        let re = Regex::new(r"(?i)\bs(?:eason)?\s*0*(\d{1,2})\b").ok()?;
        re.captures(&cleaned)
            .and_then(|caps| caps.get(1))
            .and_then(|value| value.as_str().parse::<i32>().ok())
    }

    async fn plan_linked_album(
        &self,
        auth_user: &AuthUser,
        album_id: &str,
        audio: &[&ImportFile],
        plan: &mut LinkedImportPlan,
        better_copies: bool,
    ) -> Result<()> {
        let data = self
            .execute_graphql(
                auth_user,
                r#"query ImportLinkAlbum($id: String!) {
                    Album: album(id: $id) {
                        Id: id
                        Name: name
                        LibraryId: libraryId
                        Tracks: tracks(page: { limit: 1000 }) {
                            Edges: edges {
                                Node: node {
                                    Id: id
                                    Title: title
                                    TrackNumber: trackNumber
                                    DiscNumber: discNumber
                                    DurationSecs: durationSecs
                                    MediaFile: mediaFile { Size: size Path: path }
                                }
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "id": album_id }),
            )
            .await?;
        let album = data.get("Album");
        let (Some(album_name), Some(library_id)) = (
            album.and_then(|a| a.get("Name")).and_then(|v| v.as_str()),
            album
                .and_then(|a| a.get("LibraryId"))
                .and_then(|v| v.as_str()),
        ) else {
            plan.notes.push(format!(
                "torrent is linked to album {album_id} but that album no longer exists"
            ));
            return Ok(());
        };
        plan.target_label = Some(format!("'{album_name}'"));

        let mut tracks: Vec<LinkedTrackRow> = Vec::new();
        if let Some(edges) = album
            .and_then(|a| a.get("Tracks"))
            .and_then(|t| t.get("Edges"))
            .and_then(|v| v.as_array())
        {
            for edge in edges {
                let Some(node) = edge.get("Node") else {
                    continue;
                };
                let Some(id) = node.get("Id").and_then(|v| v.as_str()) else {
                    continue;
                };
                tracks.push(LinkedTrackRow {
                    id: id.to_string(),
                    title: node
                        .get("Title")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    track_number: node
                        .get("TrackNumber")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32),
                    disc_number: node
                        .get("DiscNumber")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32),
                    seconds: node.get("DurationSecs").and_then(|v| v.as_f64()),
                    held_size: node
                        .get("MediaFile")
                        .and_then(|file| file.get("Size"))
                        .and_then(|v| v.as_i64()),
                    held_path: node
                        .get("MediaFile")
                        .and_then(|file| file.get("Path"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                });
            }
        }
        if tracks.is_empty() {
            plan.notes.push(format!(
                "album '{album_name}' has no track rows to import files into"
            ));
            return Ok(());
        }
        tracks.sort_by_key(|track| {
            (
                track.disc_number.unwrap_or(1),
                track.track_number.unwrap_or(0),
            )
        });

        let release_files = self
            .release_file_facts(
                audio
                    .iter()
                    .map(|item| {
                        (
                            item.file.source_path.clone(),
                            item.file.relative_path.clone(),
                            item.file.file_size,
                            item.file.downloaded_bytes >= item.file.file_size,
                        )
                    })
                    .collect(),
            )
            .await;
        let mut release_files = release_files;
        music_rules::fill_in_discs(&mut release_files);
        let names: Vec<Option<String>> = audio
            .iter()
            .map(|item| Self::parse_track_hint(&item.file.source_path).title)
            .collect();
        // Before planning, let the album take in what a bigger edition of it holds. Only for files
        // tagged as this album: a pack's other albums aren't bonus tracks.
        let belongs: Vec<usize> = (0..release_files.len())
            .filter(|index| {
                release_files[*index]
                    .tags
                    .album
                    .as_deref()
                    .is_some_and(|album| music_rules::same_album_title(album, album_name))
            })
            .collect();
        if !belongs.is_empty()
            && let Ok(database) = self.database().await
            && let Ok(Some(row)) =
                crate::graphql::entities::Album::get(database.pool().pool(), &album_id.to_string())
                    .await
        {
            let members: Vec<_> = belongs
                .iter()
                .map(|index| release_files[*index].clone())
                .collect();
            let member_names: Vec<_> = belongs.iter().map(|index| names[*index].clone()).collect();
            match self
                .place_on_album(auth_user, &row, &members, &member_names)
                .await
            {
                Ok(placed) => {
                    let held: HashMap<String, (Option<i64>, Option<String>)> = tracks
                        .iter()
                        .map(|track| (track.id.clone(), (track.held_size, track.held_path.clone())))
                        .collect();
                    tracks = placed
                        .into_iter()
                        .flatten()
                        .map(|track| LinkedTrackRow {
                            id: track.id,
                            title: track.title,
                            track_number: Some(track.track_number),
                            disc_number: track.disc_number,
                            seconds: track.duration_secs.map(f64::from),
                            held_size: None,
                            held_path: None,
                        })
                        .chain(tracks.drain(..))
                        .fold(Vec::new(), |mut rows: Vec<LinkedTrackRow>, track| {
                            if rows.iter().all(|row| row.id != track.id) {
                                rows.push(track);
                            }
                            rows
                        });
                    for track in &mut tracks {
                        if let Some((size, path)) = held.get(&track.id) {
                            track.held_size = *size;
                            track.held_path = path.clone();
                        }
                    }
                    tracks.sort_by_key(|track| {
                        (
                            track.disc_number.unwrap_or(1),
                            track.track_number.unwrap_or(0),
                        )
                    });
                }
                Err(error) => {
                    warn!(album_id = %album_id, error = %error, "Could not extend an album for its files: {}", error)
                }
            }
        }
        let track_facts: Vec<release_match::TrackFacts> = tracks
            .iter()
            .map(|track| release_match::TrackFacts {
                title: track.title.clone(),
                number: track.track_number,
                disc: track.disc_number,
                seconds: track.seconds,
            })
            .collect();
        // Tags, names and lengths together first, so a clear match isn't taken by a guess.
        let matched = release_match::match_tracks(&release_files, &names, &track_facts);
        let mut matched = matched;
        Self::ask_jev_about_tracks(
            &self.manager,
            &mut matched,
            &release_files,
            &track_facts,
            album_name,
        )
        .await;
        let mut used: HashSet<String> = matched
            .iter()
            .flatten()
            .filter_map(|(track, _)| tracks.get(*track).map(|track| track.id.clone()))
            .collect();
        let wanted_album = release_match::normalize_words(&music_rules::album_title(album_name));
        let positional = audio.len() == tracks.len()
            && release_files.iter().zip(&tracks).all(|(file, track)| {
                match (file.duration, track.seconds) {
                    (Some(found), Some(wanted)) => (found - wanted).abs() <= 15.0,
                    _ => true,
                }
            });
        for (index, item) in audio.iter().enumerate() {
            let facts = &release_files[index];
            // A file tagged as some other album doesn't belong, whatever its number says.
            if matched[index].is_none()
                && let Some(album) = facts.tags.album.as_deref()
                && !wanted_album.is_empty()
                && !music_rules::same_album_title(album, album_name)
                && jaro_winkler(&release_match::normalize_words(album), &wanted_album) < 0.8
            {
                plan.unmatched_reasons.insert(
                    item.file.source_path.clone(),
                    format!("tagged as album '{album}', not '{album_name}'"),
                );
                continue;
            }
            let hint = Self::parse_track_hint(&item.file.source_path);
            let title_hint = facts.tags.title.clone().or(hint.title);
            let chosen = matched[index]
                .and_then(|(track, _)| tracks.get(track))
                .or_else(|| {
                    Self::choose_album_track(
                        &tracks,
                        &used,
                        facts.tags.track.or(hint.track_number),
                        title_hint.as_deref(),
                    )
                })
                .or_else(|| {
                    // Tags and filenames were ambiguous. Audio fingerprinting
                    // (services/fingerprint.rs) would be the ideal tiebreaker
                    // but it needs the Chromaprint `fpcalc` binary plus an
                    // AcoustID key and is not wired into the service graph, so
                    // fall back to release order when the counts line up
                    // exactly and no length disagrees (design.md Q53).
                    if positional {
                        tracks.get(index).filter(|track| !used.contains(&track.id))
                    } else {
                        None
                    }
                });
            let Some(track) = chosen else {
                plan.unmatched_reasons.insert(
                    item.file.source_path.clone(),
                    format!("no unmatched track on album '{album_name}' matched this file"),
                );
                continue;
            };
            used.insert(track.id.clone());
            // A track whose file is still on disk is already in the library: the same size is this
            // file imported before, another size is a second copy, and which to keep is a person's
            // call. Only a file that has gone from disk is imported again.
            let held_on_disk = match track.held_path.as_deref() {
                Some(path) => library_storage().exists(path).await.unwrap_or(true),
                None => track.held_size.is_some(),
            };
            // Unless a person chose this download's better copies: then a better one takes the
            // track, and the file it had stays on disk as a second copy.
            let replaces = better_copies
                && track.held_size != Some(item.file.file_size)
                && track.held_path.as_deref().is_some_and(|held| {
                    let format = |path: &str| {
                        Path::new(path)
                            .extension()
                            .and_then(|extension| extension.to_str())
                            .unwrap_or_default()
                            .to_ascii_lowercase()
                    };
                    let (new_format, old_format) = (format(&item.file.source_path), format(held));
                    music_rules::better_copy(
                        music_rules::Copy {
                            format: &new_format,
                            kbps: music_rules::kbps(item.file.file_size, facts.duration),
                        },
                        music_rules::Copy {
                            format: &old_format,
                            kbps: track
                                .held_size
                                .and_then(|size| music_rules::kbps(size, track.seconds)),
                        },
                    )
                });
            if held_on_disk && !replaces {
                let reason = if track.held_size == Some(item.file.file_size) {
                    format!("{ALREADY_IN_LIBRARY} as '{}'", track.title)
                } else {
                    format!("{KEEPS_OWN_COPY} of '{}' on '{album_name}'", track.title)
                };
                plan.unmatched_reasons
                    .insert(item.file.source_path.clone(), reason);
                continue;
            }
            let label = format!("{album_name} - {}", track.title);
            plan.targets.insert(
                item.file.source_path.clone(),
                LinkedImportTarget {
                    library_id: library_id.to_string(),
                    target_type: "Track".to_string(),
                    target_id: track.id.clone(),
                    target_name: Some(label.clone()),
                    match_type: TORRENT_LINK_PARSED_MATCH_TYPE,
                    file_stem: None,
                    reason: format!(
                        "{TORRENT_LINK_PARSED_MATCH_TYPE}: torrent is linked to album '{album_name}' and this file matched track '{}'",
                        track.title
                    ),
                },
            );
        }

        Ok(())
    }

    /// For files the rules couldn't place, ask Jev which of the album's unclaimed tracks each is. An
    /// answer only counts if Jev is sure and no length contradicts it.
    async fn ask_jev_about_tracks(
        manager: &ServicesManager,
        matched: &mut [Option<(usize, f64)>],
        files: &[release_match::ReleaseFile],
        tracks: &[release_match::TrackFacts],
        album: &str,
    ) {
        const ENOUGH: f64 = 0.8;
        let waiting: Vec<usize> = (0..matched.len())
            .filter(|index| matched[*index].is_none())
            .collect();
        if waiting.is_empty() || files.iter().any(|file| file.duration.is_none()) {
            return;
        }
        let Some(jev) = crate::services::jev::Jev::configured(manager).await else {
            return;
        };
        let taken: HashSet<usize> = matched.iter().flatten().map(|(track, _)| *track).collect();
        let open: Vec<usize> = (0..tracks.len())
            .filter(|index| !taken.contains(index))
            .collect();
        if open.is_empty() {
            return;
        }
        let choices: Vec<String> = open
            .iter()
            .map(|index| tracks[*index].title.clone())
            .collect();
        let items: Vec<String> = waiting
            .iter()
            .map(|index| {
                let file = &files[*index];
                format!(
                    "{} (tagged title: {}, track: {})",
                    file.path.rsplit('/').next().unwrap_or_default(),
                    file.tags.title.as_deref().unwrap_or("none"),
                    file.tags
                        .track
                        .map_or("none".to_string(), |number| number.to_string()),
                )
            })
            .collect();
        let state = serde_json::json!({ "album": album, "tracks": choices });
        let answers = match jev
            .choose(
                state,
                "Which track of this album is this file?",
                &choices,
                &items,
            )
            .await
        {
            Ok(answers) => answers,
            Err(error) => {
                warn!(error = %error, "Jev could not help with an album");
                return;
            }
        };
        let mut claimed = taken;
        for (position, index) in waiting.iter().enumerate() {
            let Some((choice, confidence)) = answers.get(&items[position]) else {
                continue;
            };
            if *confidence < ENOUGH {
                continue;
            }
            let Some(track) = open
                .iter()
                .copied()
                .find(|track| tracks[*track].title == *choice && !claimed.contains(track))
            else {
                continue;
            };
            let fits = match (files[*index].duration, tracks[track].seconds) {
                (Some(found), Some(wanted)) => (found - wanted).abs() <= 15.0,
                _ => true,
            };
            if fits {
                claimed.insert(track);
                matched[*index] = Some((track, *confidence));
            }
        }
    }

    /// Pick the best still-unused track for a file: exact track number first,
    /// then a strong title similarity.
    fn choose_album_track<'a>(
        tracks: &'a [LinkedTrackRow],
        used: &HashSet<String>,
        track_number: Option<i32>,
        title: Option<&str>,
    ) -> Option<&'a LinkedTrackRow> {
        if let Some(number) = track_number {
            let mut numbered = tracks
                .iter()
                .filter(|track| track.track_number == Some(number) && !used.contains(&track.id));
            if let Some(first) = numbered.next()
                && numbered.next().is_none()
            {
                return Some(first);
            }
        }
        let title = title?.trim().to_ascii_lowercase();
        if title.is_empty() {
            return None;
        }
        let mut best: Option<(&LinkedTrackRow, f64)> = None;
        for track in tracks {
            if used.contains(&track.id) {
                continue;
            }
            let score = jaro_winkler(&title, &track.title.to_ascii_lowercase());
            if best.map(|(_, current)| score > current).unwrap_or(true) {
                best = Some((track, score));
            }
        }
        best.filter(|(_, score)| *score >= 0.90)
            .map(|(track, _)| track)
    }

    /// Read what files say about themselves: length, tags and chapter markers. Only finished files
    /// can be read. Results are kept, because previews ask again and a pack can be hundreds of files.
    async fn release_file_facts(
        &self,
        files: Vec<(String, String, i64, bool)>,
    ) -> Vec<release_match::ReleaseFile> {
        use futures::stream::StreamExt;
        const PROBES_AT_ONCE: usize = 6;
        futures::stream::iter(files)
            .map(|(key, path, size, complete)| async move {
                let mut facts = release_match::ReleaseFile {
                    key: key.clone(),
                    path,
                    size,
                    ..Default::default()
                };
                if !complete {
                    return facts;
                }
                let cached = self
                    .release_probe_cache
                    .lock()
                    .await
                    .get(&(key.clone(), size))
                    .cloned();
                let probed = match cached {
                    Some(probed) => Some(probed),
                    None => match ffprobe_analyze(&key).await {
                        Ok(analysis) => {
                            let probed = (
                                analysis.duration_secs,
                                analysis.tags,
                                analysis
                                    .chapters
                                    .iter()
                                    .map(|chapter| release_match::EmbeddedChapter {
                                        title: chapter.title.clone(),
                                        start: chapter.start_secs,
                                        end: chapter.end_secs,
                                    })
                                    .collect::<Vec<_>>(),
                            );
                            let mut cache = self.release_probe_cache.lock().await;
                            if cache.len() >= 4096 {
                                cache.clear();
                            }
                            cache.insert((key.clone(), size), probed.clone());
                            Some(probed)
                        }
                        Err(error) => {
                            debug!(path = %key, error = %error, "Could not read a release file's tags");
                            None
                        }
                    },
                };
                if let Some((duration, tags, embedded)) = probed {
                    facts.duration = duration;
                    facts.tags = tags;
                    facts.embedded = embedded;
                }
                facts
            })
            .buffered(PROBES_AT_ONCE)
            .collect()
            .await
    }

    /// The book as a release is compared with it. The reference list is the provider's, even when a
    /// previous release's chapters have replaced it on the book.
    async fn book_reference(
        &self,
        audiobook_id: &str,
    ) -> Result<(
        crate::graphql::entities::Audiobook,
        Vec<crate::graphql::entities::Chapter>,
        release_match::BookFacts,
        Vec<release_match::ReferenceChapter>,
    )> {
        use crate::graphql::entities::Audiobook;
        let database = self.database().await?;
        let book = Audiobook::get(database.pool().pool(), &audiobook_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("That audiobook no longer exists"))?;
        let mut chapters = self.book_chapters(audiobook_id).await?;
        chapters.sort_by_key(|chapter| chapter.chapter_number);
        let reference: Vec<release_match::ReferenceChapter> = book
            .provider_chapters
            .as_deref()
            .and_then(|saved| serde_json::from_str(saved).ok())
            .unwrap_or_else(|| {
                chapters
                    .iter()
                    .map(|chapter| release_match::ReferenceChapter {
                        title: chapter.title.clone(),
                        start: chapter.start_time_secs,
                        seconds: chapter.duration_secs.map(f64::from),
                    })
                    .collect()
            });
        let facts = release_match::BookFacts {
            title: book.title.clone(),
            author: book.author_name.clone(),
            narrator: book.narrator_name.clone(),
            asin: book.asin.clone().or_else(|| book.audible_id.clone()),
            seconds: book.total_duration_secs.map(f64::from),
            series: book.series_name.clone(),
        };
        Ok((book, chapters, facts, reference))
    }

    /// The rules' answer for a release, with Jev asked to settle which book it is if they can't and
    /// a key is configured. Lengths still decide whether the files fit.
    async fn plan_book_release(
        &self,
        book_id: &str,
        files: &[release_match::ReleaseFile],
        facts: &release_match::BookFacts,
        reference: &[release_match::ReferenceChapter],
    ) -> release_match::ReleasePlan {
        let mut plan = release_match::plan_release(files, facts, reference);
        let undecided = plan.chosen.is_none() || plan.evidence.is_empty();
        let unsettled = matches!(plan.outcome, release_match::BookOutcome::Review(_));
        if !(undecided && unsettled && plan.complete_facts) || plan.groups.is_empty() {
            return plan;
        }
        let Some(jev) = crate::services::jev::Jev::configured(&self.manager).await else {
            return plan;
        };
        const ENOUGH: f64 = 0.85;
        let key = format!(
            "{book_id}|{}|{}",
            facts.title,
            plan.groups
                .iter()
                .map(|group| format!("{}:{}", group.key, group.files.len()))
                .collect::<Vec<_>>()
                .join("|")
        );
        let cached = self.jev_group_cache.lock().await.get(&key).cloned();
        let answer = match cached {
            Some(answer) => answer,
            None => {
                let answer = Self::ask_jev_which_group(&jev, &plan.groups, files, facts).await;
                let mut cache = self.jev_group_cache.lock().await;
                if cache.len() >= 512 {
                    cache.clear();
                }
                cache.insert(key, answer);
                answer
            }
        };
        if let Some((index, probability)) = answer
            && probability >= ENOUGH
            && let Some(group) = plan.groups.get(index)
        {
            plan.chosen = Some(index);
            plan.outcome = release_match::layout_book(files, group, facts, reference);
            plan.evidence.push(format!(
                "Jev is {:.0}% sure these files are this book",
                probability * 100.0
            ));
        }
        plan
    }

    /// The group Jev thinks most likely to be this book, and how likely. `None` if it can't say.
    async fn ask_jev_which_group(
        jev: &crate::services::jev::Jev,
        groups: &[release_match::BookGroup],
        files: &[release_match::ReleaseFile],
        facts: &release_match::BookFacts,
    ) -> Option<(usize, f64)> {
        const SAMPLE_FILES: usize = 5;
        const MAX_GROUPS: usize = 20;
        let described: Vec<serde_json::Value> = groups
            .iter()
            .take(MAX_GROUPS)
            .map(|group| {
                let tags = group.files.first().map(|file| &files[*file].tags);
                serde_json::json!({
                    "name": group.label,
                    "album_tag": tags.and_then(|tags| tags.album.clone()),
                    "artist_tag": tags.and_then(|tags| tags.artist.clone()),
                    "file_count": group.files.len(),
                    "example_files": group.files.iter().take(SAMPLE_FILES)
                        .map(|file| files[*file].path.rsplit('/').next().unwrap_or_default())
                        .collect::<Vec<_>>(),
                })
            })
            .collect();
        let state = serde_json::json!({
            "wanted_book": {
                "title": facts.title,
                "author": facts.author,
                "narrator": facts.narrator,
                "series": facts.series,
            },
            "groups_of_files": described,
        });
        let questions = (0..described.len())
            .map(|index| {
                (
                    format!("group_{index}"),
                    format!(
                        "Is the group of files at position {index} in groups_of_files the wanted book, and not a different book by the same author?"
                    ),
                )
            })
            .collect();
        match jev.yes_no(state, &questions).await {
            Ok(answers) => {
                // Sibling books score surprisingly close, so the winner has to stand clear of them.
                const CLEAR_GAP: f64 = 0.1;
                let mut ranked: Vec<(usize, f64)> = answers
                    .into_iter()
                    .filter_map(|(name, probability)| {
                        Some((
                            name.strip_prefix("group_")?.parse::<usize>().ok()?,
                            probability,
                        ))
                    })
                    .collect();
                ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                let best = *ranked.first()?;
                let clear = ranked
                    .get(1)
                    .is_none_or(|second| best.1 - second.1 >= CLEAR_GAP);
                clear.then_some(best)
            }
            Err(error) => {
                warn!(error = %error, "Jev could not help with a release");
                None
            }
        }
    }

    /// Make a book's chapter rows say what a release's plan says, reusing existing rows so anything
    /// pointing at them survives. The provider's list is kept, once, so this can be undone.
    async fn rewrite_book_chapters(
        &self,
        auth_user: &AuthUser,
        book: &crate::graphql::entities::Audiobook,
        existing: &[crate::graphql::entities::Chapter],
        reference: &[release_match::ReferenceChapter],
        layout: &release_match::BookLayout,
        always: bool,
    ) -> Result<Vec<String>> {
        let keep_rows = !always
            && layout.source == release_match::ChapterSource::Provider
            && book
                .chapter_source
                .as_deref()
                .is_none_or(|source| source == "provider")
            && existing.len() == layout.chapters.len();
        if keep_rows {
            return Ok(existing.iter().map(|chapter| chapter.id.clone()).collect());
        }
        if book.provider_chapters.is_none() && !reference.is_empty() {
            let saved = serde_json::to_string(reference)?;
            self.update_audiobook_fields(
                auth_user,
                &book.id,
                serde_json::json!({ "providerChapters": saved }),
            )
            .await?;
        }
        let mut ids = Vec::with_capacity(layout.chapters.len());
        for (index, planned) in layout.chapters.iter().enumerate() {
            let number = index as i32 + 1;
            let duration = planned
                .end
                .map(|end| (end - planned.start).max(0.0).round() as i32);
            if let Some(row) = existing.get(index) {
                let data = self
                    .execute_mutation(
                        auth_user,
                        r#"mutation RewriteChapter($id: String!, $input: UpdateChapterInput!) {
                            UpdateChapter: updateChapter(id: $id, input: $input) { Success: success Error: error }
                        }"#,
                        serde_json::json!({ "id": row.id, "input": {
                            "chapterNumber": number,
                            "title": planned.title,
                            "startTimeSecs": planned.start,
                            "endTimeSecs": planned.end,
                            "durationSecs": duration,
                            "fileOffsetSecs": serde_json::Value::Null,
                        }}),
                    )
                    .await?;
                if data
                    .pointer("/UpdateChapter/Success")
                    .and_then(serde_json::Value::as_bool)
                    != Some(true)
                {
                    anyhow::bail!("Could not update a chapter from this release");
                }
                ids.push(row.id.clone());
            } else {
                let data = self
                    .execute_mutation(
                        auth_user,
                        r#"mutation AddChapter($input: CreateChapterInput!) {
                            CreateChapter: createChapter(input: $input) {
                                Success: success
                                Error: error
                                Chapter: chapter { Id: id }
                            }
                        }"#,
                        serde_json::json!({ "input": {
                            "audiobookId": book.id,
                            "chapterNumber": number,
                            "title": planned.title,
                            "startTimeSecs": planned.start,
                            "endTimeSecs": planned.end,
                            "durationSecs": duration,
                            "wanted": planned.file.is_none(),
                        }}),
                    )
                    .await?;
                let id = data
                    .pointer("/CreateChapter/Chapter/Id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| anyhow::anyhow!("Could not add a chapter from this release"))?;
                ids.push(id.to_string());
            }
        }
        for extra in existing.iter().skip(layout.chapters.len()) {
            let data = self
                .execute_mutation(
                    auth_user,
                    r#"mutation RemoveChapter($id: String!) {
                        DeleteChapter: deleteChapter(id: $id) { Success: success Error: error }
                    }"#,
                    serde_json::json!({ "id": extra.id }),
                )
                .await?;
            if data
                .pointer("/DeleteChapter/Success")
                .and_then(serde_json::Value::as_bool)
                != Some(true)
            {
                anyhow::bail!("Could not remove a chapter this release doesn't have");
            }
        }
        Ok(ids)
    }

    async fn update_audiobook_fields(
        &self,
        auth_user: &AuthUser,
        audiobook_id: &str,
        input: serde_json::Value,
    ) -> Result<()> {
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation SetBookFields($id: String!, $input: UpdateAudiobookInput!) {
                    UpdateAudiobook: updateAudiobook(id: $id, input: $input) { Success: success Error: error }
                }"#,
                serde_json::json!({ "id": audiobook_id, "input": input }),
            )
            .await?;
        if data
            .pointer("/UpdateAudiobook/Success")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            anyhow::bail!("Could not update the audiobook");
        }
        Ok(())
    }

    /// Point chapters at the files that landed, with where each starts in its file.
    async fn write_book_chapters(
        &self,
        auth_user: &AuthUser,
        writes: &BookWrites,
        imported: &HashMap<String, String>,
    ) -> Result<()> {
        for (chapter_id, source_path, offset) in &writes.placements {
            let Some(media_file_id) = imported.get(source_path) else {
                continue;
            };
            let data = self
                .execute_mutation(
                    auth_user,
                    r#"mutation PlaceChapter($id: String!, $input: UpdateChapterInput!) {
                        UpdateChapter: updateChapter(id: $id, input: $input) { Success: success Error: error }
                    }"#,
                    serde_json::json!({ "id": chapter_id, "input": {
                        "mediaFileId": media_file_id,
                        "fileOffsetSecs": offset,
                        "wanted": false,
                    }}),
                )
                .await?;
            if data
                .pointer("/UpdateChapter/Success")
                .and_then(serde_json::Value::as_bool)
                != Some(true)
            {
                anyhow::bail!("Could not point a chapter at its file");
            }
        }
        self.update_audiobook_fields(
            auth_user,
            &writes.audiobook_id,
            serde_json::json!({ "chapterSource": writes.source, "fileShape": writes.shape }),
        )
        .await
    }

    /// Decide what a release is for a book, and get the chapters ready to receive it.
    ///
    /// Rules are in `release_match`. No chapter is ever filled by position alone: a release that
    /// doesn't fit asks a person, and other books in a pack are left where they are.
    async fn plan_linked_audiobook(
        &self,
        auth_user: &AuthUser,
        audiobook_id: &str,
        audio: &[&ImportFile],
        plan: &mut LinkedImportPlan,
        choice: Option<&BookImportChoice>,
    ) -> Result<()> {
        let Ok((book, chapters, facts, reference)) = self.book_reference(audiobook_id).await else {
            plan.notes.push(format!(
                "torrent is linked to audiobook {audiobook_id} but that audiobook no longer exists"
            ));
            return Ok(());
        };
        let title = book.title.clone();
        plan.target_label = Some(format!("'{title}'"));

        if audio.is_empty() {
            let reason = format!(
                "This release has no audio files, so it can't be \u{201c}{title}\u{201d}. It may be an ebook."
            );
            plan.notes.push(reason.clone());
            plan.hard_failure = Some(reason);
            return Ok(());
        }

        let release_files = self
            .release_file_facts(
                audio
                    .iter()
                    .map(|item| {
                        (
                            item.file.source_path.clone(),
                            item.file.relative_path.clone(),
                            item.file.file_size,
                            item.file.downloaded_bytes >= item.file.file_size,
                        )
                    })
                    .collect(),
            )
            .await;
        let mut release = self
            .plan_book_release(&book.id, &release_files, &facts, &reference)
            .await;
        // Sizes are only good enough to preview with. Nothing moves on an estimate.
        if !release.complete_facts && !choice.is_some_and(|choice| choice.accept_review) {
            let reason = "Couldn't read the lengths of these files, so they can't be placed yet.";
            release.outcome = release_match::BookOutcome::Review(release_match::ReviewNeeded {
                reason: reason.to_string(),
                fallback: match release.outcome {
                    release_match::BookOutcome::Ready(layout) => Some(layout),
                    release_match::BookOutcome::Review(review) => review.fallback,
                },
            });
        }

        // A person pointing at one book of a pack settles which files it is.
        if let Some(key) = choice.and_then(|choice| choice.group_key.as_deref())
            && let Some(index) = release.groups.iter().position(|group| group.key == key)
        {
            release.chosen = Some(index);
            release.outcome = release_match::layout_book(
                &release_files,
                &release.groups[index],
                &facts,
                &reference,
            );
        }
        let accept = choice.is_some_and(|choice| choice.accept_review);
        let layout = match release.outcome.clone() {
            release_match::BookOutcome::Ready(layout) => layout,
            release_match::BookOutcome::Review(review) => match review.fallback {
                Some(layout) if accept => layout,
                _ => {
                    // A book's problem is its own: other books in a pack keep what they have.
                    let own: HashSet<&String> = release
                        .chosen
                        .and_then(|index| release.groups.get(index))
                        .map(|group| {
                            group
                                .files
                                .iter()
                                .map(|file| &release_files[*file].key)
                                .collect()
                        })
                        .unwrap_or_default();
                    for item in audio {
                        let path = &item.file.source_path;
                        if own.is_empty() || own.contains(path) {
                            plan.unmatched_reasons
                                .insert(path.clone(), review.reason.clone());
                        } else {
                            plan.left_alone.insert(path.clone());
                        }
                    }
                    plan.review = Some(release_match::describe(&release, &facts));
                    return Ok(());
                }
            },
        };
        let group = &release.groups[release.chosen.unwrap_or(0)];

        // Other books in the pack stay out of this one. A later choice imports them.
        for (index, other) in release.groups.iter().enumerate() {
            if Some(index) == release.chosen {
                continue;
            }
            for file in &other.files {
                let path = &release_files[*file].key;
                if choice.is_some() {
                    plan.left_alone.insert(path.clone());
                } else {
                    plan.unmatched_reasons.insert(
                        path.clone(),
                        format!(
                            "Belongs to \u{201c}{}\u{201d}, another book in this release",
                            other.label
                        ),
                    );
                }
            }
        }

        let ids = self
            .rewrite_book_chapters(auth_user, &book, &chapters, &reference, &layout, false)
            .await?;
        let library_id = book.library_id.clone();
        let file_paths: Vec<String> = group
            .files
            .iter()
            .map(|index| release_files[*index].key.clone())
            .collect();
        let holder_of_file = release_match::chapter_of_each_file(&layout, file_paths.len());
        let width = file_paths.len().to_string().len().max(2);
        for (position, path) in file_paths.iter().enumerate() {
            let chapter_index = holder_of_file[position].unwrap_or(0);
            let Some(chapter_id) = ids.get(chapter_index) else {
                continue;
            };
            let chapter_title = layout
                .chapters
                .get(chapter_index)
                .map_or("", |chapter| chapter.title.as_str());
            let (reason, file_stem) = match layout.shape {
                release_match::FileShape::WholeBook => (
                    format!(
                        "{TORRENT_LINK_PARSED_MATCH_TYPE}: holds the whole of '{title}'; {WHOLE_BOOK_REASON}"
                    ),
                    None,
                ),
                release_match::FileShape::PerChapter => (
                    format!(
                        "{TORRENT_LINK_PARSED_MATCH_TYPE}: chapter '{chapter_title}' of '{title}'"
                    ),
                    None,
                ),
                release_match::FileShape::Parts => (
                    format!(
                        "{TORRENT_LINK_PARSED_MATCH_TYPE}: part {} of {} of '{title}'",
                        position + 1,
                        file_paths.len()
                    ),
                    Some(format!(
                        "{} - Part {:0width$}",
                        sanitize_for_filename(&title),
                        position + 1
                    )),
                ),
            };
            plan.targets.insert(
                path.clone(),
                LinkedImportTarget {
                    library_id: library_id.clone(),
                    target_type: "Chapter".to_string(),
                    target_id: chapter_id.clone(),
                    target_name: Some(if layout.shape == release_match::FileShape::Parts {
                        format!("{title} - part {}", position + 1)
                    } else {
                        format!("{title} - {chapter_title}")
                    }),
                    match_type: TORRENT_LINK_PARSED_MATCH_TYPE,
                    reason,
                    file_stem,
                },
            );
        }
        plan.book_writes = Some(BookWrites {
            audiobook_id: book.id.clone(),
            placements: layout
                .chapters
                .iter()
                .enumerate()
                .filter_map(|(index, chapter)| {
                    Some((
                        ids.get(index)?.clone(),
                        file_paths.get(chapter.file?)?.clone(),
                        chapter.offset,
                    ))
                })
                .collect(),
            source: layout.source.as_str(),
            shape: layout.shape.as_str(),
        });
        plan.summary = Some(release_match::describe(&release, &facts));
        if file_paths.len() > 1 {
            plan.book_title = Some(title.clone());
            plan.book_file_numbers = file_paths
                .iter()
                .enumerate()
                .map(|(position, path)| (path.clone(), position + 1))
                .collect();
        }
        Ok(())
    }

    /// Single-target linkage (`trackId`/`chapterId` with no parent recorded):
    /// only usable when the release contains exactly one audio file.
    async fn plan_linked_single(
        &self,
        auth_user: &AuthUser,
        target_type: &str,
        target_id: &str,
        audio: &[&ImportFile],
        plan: &mut LinkedImportPlan,
    ) -> Result<()> {
        if audio.len() != 1 {
            return Ok(());
        }
        let query = if target_type == "Track" {
            r#"query ImportLinkTrack($id: String!) {
                Node: track(id: $id) { Id: id Title: title LibraryId: libraryId }
            }"#
        } else {
            r#"query ImportLinkChapter($id: String!) {
                Node: chapter(id: $id) { Id: id Title: title Audiobook: audiobook { LibraryId: libraryId } }
            }"#
        };
        let data = self
            .execute_graphql(auth_user, query, serde_json::json!({ "id": target_id }))
            .await?;
        let node = data.get("Node");
        let library_id = node
            .and_then(|n| n.get("LibraryId"))
            .and_then(|v| v.as_str())
            .or_else(|| {
                node.and_then(|n| n.get("Audiobook"))
                    .and_then(|a| a.get("LibraryId"))
                    .and_then(|v| v.as_str())
            });
        let Some(library_id) = library_id else {
            plan.notes.push(format!(
                "torrent is linked to {target_type} {target_id} but its library could not be resolved"
            ));
            return Ok(());
        };
        let name = node
            .and_then(|n| n.get("Title"))
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        plan.target_label = name.as_ref().map(|value| format!("'{value}'"));
        plan.targets.insert(
            audio[0].file.source_path.clone(),
            LinkedImportTarget {
                library_id: library_id.to_string(),
                target_type: target_type.to_string(),
                target_id: target_id.to_string(),
                target_name: name.clone(),
                file_stem: None,
                match_type: TORRENT_LINK_MATCH_TYPE,
                reason: format!(
                    "{TORRENT_LINK_MATCH_TYPE}: torrent was grabbed for {} '{}'",
                    target_type.to_ascii_lowercase(),
                    name.unwrap_or_else(|| target_id.to_string())
                ),
            },
        );
        Ok(())
    }

    /// Sort key that orders `track2` before `track10` (digit runs are
    /// zero-padded so they compare numerically).
    fn natural_sort_key(path: &str) -> String {
        let mut key = String::with_capacity(path.len() + 16);
        let mut digits = String::new();
        for ch in path.to_ascii_lowercase().chars() {
            if ch.is_ascii_digit() {
                digits.push(ch);
            } else {
                if !digits.is_empty() {
                    key.push_str(&format!("{:0>12}", digits));
                    digits.clear();
                }
                key.push(ch);
            }
        }
        if !digits.is_empty() {
            key.push_str(&format!("{:0>12}", digits));
        }
        key
    }

    async fn ensure_source_media_file(
        &self,
        auth_user: &AuthUser,
        owner_id: &str,
        info_hash: &str,
        file: &SourceFileImport,
    ) -> Result<String> {
        if let Some(id) = self
            .find_media_file_id_by_path(auth_user, owner_id, info_hash, &file.source_path)
            .await?
        {
            return Ok(id);
        }

        let added_at = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        let original_name = Path::new(&file.source_path)
            .file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.to_string());
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation CreateSourceMediaFile($input: CreateMediaFileInput!) {
                    createMediaFile(input: $input) {
                        success
                        error
                        mediaFile { Id: id }
                    }
                }"#,
                serde_json::json!({
                    "input": {
                        "libraryId": serde_json::Value::Null,
                        "path": file.source_path.clone(),
                        "relativePath": file.relative_path.clone(),
                        "originalName": original_name,
                        "size": file.file_size,
                        "isHdr": false,
                        "addedAt": added_at,
                        "metadata": serde_json::json!({
                            "sourceType": "torrent",
                            "sourceId": info_hash,
                            "sourceOwnerId": owner_id,
                            "unmatchedReason": "Completed torrent file awaiting match"
                        }).to_string()
                    }
                }),
            )
            .await?;
        let create = data
            .get("createMediaFile")
            .ok_or_else(|| anyhow::anyhow!("GraphQL response missing createMediaFile"))?;
        if !create
            .get("success")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            anyhow::bail!(
                "{}",
                create
                    .get("error")
                    .and_then(|value| value.as_str())
                    .unwrap_or("createMediaFile failed")
            );
        }
        create
            .get("mediaFile")
            .and_then(|value| value.get("Id"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_string())
            .ok_or_else(|| anyhow::anyhow!("createMediaFile response missing mediaFile.id"))
    }

    async fn find_media_file_id_by_path(
        &self,
        auth_user: &AuthUser,
        owner_id: &str,
        info_hash: &str,
        path: &str,
    ) -> Result<Option<String>> {
        let data = self
            .execute_graphql(
                auth_user,
                r#"query MediaFileByPathForImport($path: String!) {
                    MediaFiles: mediaFiles(
                        where: { path: { eq: $path } }
                        page: { limit: 1 }
                    ) {
                        Edges: edges { Node: node { Id: id LibraryId: libraryId Metadata: metadata Library: library { UserId: userId } } }
                    }
                }"#,
                serde_json::json!({ "path": path }),
            )
            .await?;
        let node = data
            .get("MediaFiles")
            .and_then(|value| value.get("Edges"))
            .and_then(serde_json::Value::as_array)
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"));
        let Some(node) = node else { return Ok(None) };
        let assigned_owner = node
            .get("Library")
            .and_then(|library| library.get("UserId"))
            .and_then(serde_json::Value::as_str);
        let source = node
            .get("Metadata")
            .and_then(serde_json::Value::as_str)
            .and_then(|value| serde_json::from_str::<serde_json::Value>(value).ok());
        let owned_unassigned_source = node.get("LibraryId").is_none_or(serde_json::Value::is_null)
            && source
                .as_ref()
                .is_some_and(|source| source_row_is_this_import(source, owner_id, info_hash));
        if assigned_owner != Some(owner_id) && !owned_unassigned_source {
            anyhow::bail!(
                "Existing source media file ownership is unknown or belongs to another user"
            );
        }
        Ok(node
            .get("Id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string))
    }

    async fn candidate_libraries_for_import(
        &self,
        auth_user: &AuthUser,
        owner_id: &str,
        explicit_library_id: Option<&str>,
        file: &SourceFileImport,
    ) -> Result<Vec<LibraryRow>> {
        let compatible_types = Self::compatible_library_types_for_path(&file.source_path);
        if compatible_types.is_empty() {
            return Ok(Vec::new());
        }

        if let Some(library_id) = explicit_library_id {
            let library = self.get_library(library_id).await?;
            if library.user_id != owner_id {
                anyhow::bail!("Import library does not belong to the source owner");
            }
            if compatible_types.contains(&Self::normalize_library_type(&library.library_type)) {
                return Ok(vec![library]);
            }
            return Ok(Vec::new());
        }

        let data = self
            .execute_graphql(
                auth_user,
                r#"query CompatibleLibrariesForImport($userId: String!) {
                    Libraries: libraries(
                        where: { userId: { eq: $userId } }
                        page: { limit: 1000 }
                    ) {
                        Edges: edges {
                            Node: node {
                                Id: id
                                UserId: userId
                                Name: name
                                Path: path
                                LibraryType: libraryType
                                AutoOrganize: autoOrganize
                                ScanIntervalMinutes: scanIntervalMinutes
                                Scanning: scanning
                                NamingPattern: namingPattern
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "userId": owner_id }),
            )
            .await?;

        let mut libraries = Vec::new();
        if let Some(edges) = data
            .get("Libraries")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
        {
            for edge in edges {
                let Some(node) = edge.get("Node") else {
                    continue;
                };
                let library_type = node
                    .get("LibraryType")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                if !compatible_types.contains(&Self::normalize_library_type(&library_type)) {
                    continue;
                }
                libraries.push(LibraryRow {
                    id: node
                        .get("Id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    user_id: node
                        .get("UserId")
                        .and_then(|v| v.as_str())
                        .filter(|id| *id == owner_id)
                        .context("Import library ownership is missing or inconsistent")?
                        .to_string(),
                    name: node
                        .get("Name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    path: node
                        .get("Path")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    library_type,
                    auto_organize: node
                        .get("AutoOrganize")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    scan_interval_minutes: node
                        .get("ScanIntervalMinutes")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32,
                    scanning: node
                        .get("Scanning")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    naming_pattern: node
                        .get("NamingPattern")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                });
            }
        }

        Ok(libraries)
    }

    async fn select_import_match(
        &self,
        _auth_user: &AuthUser,
        media_file_id: &str,
        file: &SourceFileImport,
        libraries: Vec<LibraryRow>,
        explicit_library: bool,
    ) -> Result<Option<SelectedImportMatch>> {
        let mut ranked = Vec::<SelectedImportMatch>::new();
        for library in libraries {
            let normalized_type = Self::normalize_library_type(&library.library_type);
            let result = self
                .match_media_file(MatchRequest {
                    media_file_id: media_file_id.to_string(),
                    library_id: Some(library.id.clone()),
                    methods: vec![
                        MatchMethod::Filename,
                        MatchMethod::Metadata,
                        MatchMethod::Ollama,
                    ],
                    force: true,
                    auto_match: false,
                    candidate_limit: 10,
                    allow_provider_fallback: false,
                    wanted_policy: MatchWantedPolicy::PreferWanted,
                    ..Default::default()
                })
                .await?;
            for candidate in result.candidates {
                if Self::candidate_matches_library_type(&candidate.target_type, &normalized_type) {
                    ranked.push(SelectedImportMatch {
                        library: library.clone(),
                        candidate,
                    });
                }
            }
        }

        ranked.sort_by(|a, b| {
            b.candidate
                .score
                .partial_cmp(&a.candidate.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let Some(best) = ranked.first().cloned() else {
            return Ok(None);
        };

        let second_score = ranked
            .get(1)
            .map(|item| item.candidate.score)
            .unwrap_or(0.0);
        let accepted = if explicit_library {
            best.candidate.score >= 0.70
        } else {
            best.candidate.score >= 0.90 && best.candidate.score - second_score >= 0.10
        };

        if accepted {
            info!(
                media_file_id = %media_file_id,
                source_path = %file.source_path,
                library_id = %best.library.id,
                target_type = %best.candidate.target_type,
                target_id = %best.candidate.target_id,
                score = best.candidate.score,
                explicit_library,
                "Selected torrent import match: source='{}', library_id={}, target_type={}, target_id={}, score={:.3}",
                file.source_path,
                best.library.id,
                best.candidate.target_type,
                best.candidate.target_id,
                best.candidate.score
            );
            Ok(Some(best))
        } else {
            Ok(None)
        }
    }

    /// Place `source_path` at `target_path`.
    ///
    /// `allow_move` is true only for files Librarian itself unpacked into a
    /// staging directory: those are our own copies, cannot be hardlinked to a
    /// seeding source, and moving them avoids doubling disk usage. Actual
    /// torrent payload files are never moved (design.md Q10).
    async fn materialize_torrent_file(
        &self,
        source_path: &Path,
        target_path: &Path,
        auth_user: &AuthUser,
        library: &LibraryRow,
        allow_move: bool,
    ) -> Result<()> {
        self.ensure_library_not_moving(&library.id).await?;
        // Through storage: a download can be on a share, and a share that is down is not a
        // missing file.
        match library_storage().exists(source_path).await {
            Ok(true) => {}
            Ok(false) => anyhow::bail!("Source file does not exist: {}", source_path.display()),
            Err(error) => anyhow::bail!(
                "The download '{}' cannot be read now: {error}",
                source_path.display()
            ),
        }

        // No-clobber placement (audit #4): for torrent payload files `allow_move`
        // is false, so this is always a copy/hardlink, never a move, and the
        // seeding source file is never modified. It is only true for files
        // Librarian unpacked into its own staging directory, which the torrent
        // client does not know about. This also fixes the previous bug where ANY hard_link
        // error — including a genuine `AlreadyExists` conflict race — fell through
        // to a raw `copy`, which could truncate a seeding source hardlink shared
        // with the torrent client. Now only a cross-device error falls back to
        // copy (via the no-clobber copy-to-temp-then-hardlink path); an
        // `AlreadyExists` race is handled as the pre-existing conflict path below,
        // and any other error propagates instead of being papered over.
        let outcome = place_in_library(
            PathBuf::from(&library.path),
            source_path.to_path_buf(),
            target_path.to_path_buf(),
            allow_move,
            false,
        )
        .await?;

        if outcome == NoClobberOutcome::TargetExists {
            if paths_refer_to_same_file(source_path, target_path).await {
                return Ok(());
            }

            // Q44: before treating this purely as a conflict to skip, check
            // whether the new file would actually be a quality upgrade over
            // what's already there. We never auto-replace (Q38) — if it is
            // an upgrade, this creates an actionable notification the user
            // can approve (`approveQualityUpgrade` mutation) in addition to
            // the generic conflict notification below.
            self.maybe_notify_quality_upgrade(auth_user, source_path, target_path, library)
                .await;

            let message = format!(
                "Torrent import conflict skipped to prevent overwrite: source='{}', target='{}'",
                source_path.display(),
                target_path.display()
            );
            warn!(
                source_path = %source_path.display(),
                target_path = %target_path.display(),
                library_id = %library.id,
                "{}", message
            );
            self.create_notification(
                auth_user,
                &library.user_id,
                "WARNING",
                "ORGANIZATION",
                "Torrent import conflict skipped",
                &message,
            )
            .await;
            anyhow::bail!("Target path already exists; skipped to avoid overwrite");
        }

        // A downloaded video's subtitles come with it, renamed after it (Q118). The download's
        // own files are never moved, so its torrent keeps seeding.
        let source = source_path.to_string_lossy().into_owned();
        let extension = source
            .rsplit_once('.')
            .map(|(_, ext)| ext)
            .unwrap_or_default();
        if crate::services::subtitles::store::is_video_extension(extension) {
            match crate::services::subtitles::store::discover(&source).await {
                Ok(found) if !found.is_empty() => {
                    let sidecars = found.into_iter().map(|file| (None, file)).collect();
                    self.carry_sidecars(
                        Path::new(&library.path),
                        source_path,
                        target_path,
                        sidecars,
                        allow_move,
                        false,
                    )
                    .await;
                }
                Ok(_) => {}
                Err(error) => {
                    debug!(source = %source_path.display(), %error, "Could not look for a download's subtitles");
                }
            }
        }

        Ok(())
    }

    /// Q44/Q38: when a second torrent's file targets the same library path as
    /// an already-imported `MediaFile`, check whether it would be a quality
    /// upgrade (`scoring::is_upgrade`) over the existing file. If so, create
    /// an actionable notification (`quality::mutations::QUALITY_UPGRADE_ACTION_TYPE`)
    /// rather than silently discarding the option — the user approves via
    /// `approveQualityUpgrade` or dismisses via the generic `updateNotification`
    /// mutation. Never replaces the file itself (that only happens once
    /// approved). Best-effort: failures here must not affect the caller's
    /// existing skip-as-duplicate behavior.
    async fn maybe_notify_quality_upgrade(
        &self,
        auth_user: &AuthUser,
        source_path: &Path,
        target_path: &Path,
        library: &LibraryRow,
    ) {
        if is_remote_path(Path::new(&library.path)) {
            // An upgrade replaces a file in place after proving, from its device, inode and change
            // time, that it is the file that was approved. A share cannot give that proof, so the
            // better copy is not offered as an upgrade there; it stays in the downloads.
            info!(
                library_id = %library.id,
                library_name = %library.name,
                target_path = %target_path.display(),
                "Not offering a quality upgrade for '{}': libraries on a network share do not support in-place upgrades",
                target_path.display()
            );
            return;
        }
        if is_remote_path(source_path) {
            // The same proof is taken of the better copy, which a download on a share cannot give.
            info!(
                library_id = %library.id,
                source_path = %source_path.display(),
                target_path = %target_path.display(),
                "Not offering a quality upgrade for '{}': the better copy is a download on a network share, which in-place upgrades do not support",
                target_path.display()
            );
            return;
        }
        let Some(db_service) = self.manager.get_database().await else {
            return;
        };
        let db = db_service.pool().clone();

        let target_str = target_path.to_string_lossy().to_string();
        let existing = match crate::graphql::entities::MediaFile::query(db.pool())
            .filter(crate::graphql::entities::MediaFileWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                path: Some(graphql_orm::graphql::filters::StringFilter {
                    eq: Some(target_str.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .limit(2)
            .fetch_all()
            .await
        {
            Ok(mut rows) if rows.len() == 1 => rows.pop(),
            Ok(_) => None,
            Err(e) => {
                warn!(error = %e, "Failed to look up existing media file for upgrade check");
                return;
            }
        };
        let Some(existing) = existing else {
            return;
        };

        let new_filename = source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let new_parsed = scoring::parse_release(new_filename);
        let existing_parsed = profile::parsed_release_for_media_file(&existing);

        if !scoring::is_upgrade(&new_parsed, Some(&existing_parsed)) {
            return;
        }

        let existing_filename = target_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let message = format!(
            "A higher quality version was found for an existing file: '{}' may be an upgrade over '{}'. Approve to replace it.",
            new_filename, existing_filename
        );
        let (pending_match_id, action_data) =
            match crate::services::graphql::mutations::quality::build_quality_upgrade_action_data(
                &db,
                &library.user_id,
                &library.id,
                &existing.id,
                source_path,
                target_path,
            )
            .await
            {
                Ok(data) => data,
                Err(error) => {
                    warn!(library_id = %library.id, source_path = %source_path.display(), target_path = %target_path.display(), error = %error,
                    "Cannot create quality upgrade notification without verified file identities");
                    return;
                }
            };

        self.create_action_notification(
            auth_user,
            &library.user_id,
            "ACTION_REQUIRED",
            "QUALITY",
            "Possible quality upgrade found",
            &message,
            Some(&library.id),
            Some(&existing.id),
            Some(&pending_match_id),
            crate::services::graphql::mutations::quality::QUALITY_UPGRADE_ACTION_TYPE,
            &action_data,
        )
        .await;
    }

    #[allow(clippy::too_many_arguments)]
    async fn upsert_pending_file_match(
        &self,
        auth_user: &AuthUser,
        owner_id: &str,
        info_hash: &str,
        file: &SourceFileImport,
        candidate: Option<&MatchCandidate>,
        unmatched_reason: Option<&str>,
        copied_at: Option<&str>,
        copy_error: Option<&str>,
    ) -> Result<()> {
        let existing = self
            .find_pending_file_match(
                auth_user,
                owner_id,
                info_hash,
                file.file_index,
                &file.source_path,
            )
            .await?;

        // A file that reached the library stays recorded as having done so. A later look at the same
        // download, for another book in it, has nothing to say about this one.
        if candidate.is_none()
            && copied_at.is_none()
            && let Some((id, _, _)) = existing.as_ref()
        {
            let database = self.database().await?;
            if crate::graphql::entities::PendingFileMatch::get(database.pool().pool(), id)
                .await?
                .is_some_and(|row| row.copied_at.is_some())
            {
                return Ok(());
            }
        }

        let input = serde_json::json!({
            "userId": owner_id,
            "sourcePath": file.source_path.clone(),
            "sourceType": "torrent",
            "sourceId": info_hash,
            "sourceFileIndex": file.file_index,
            "fileSize": file.file_size,
            "movieId": candidate.filter(|c| c.target_type == "Movie").map(|c| c.target_id.clone()),
            "episodeId": candidate.filter(|c| c.target_type == "Episode").map(|c| c.target_id.clone()),
            "trackId": candidate.filter(|c| c.target_type == "Track").map(|c| c.target_id.clone()),
            "chapterId": candidate.filter(|c| c.target_type == "Chapter").map(|c| c.target_id.clone()),
            "unmatchedReason": unmatched_reason,
            "matchType": candidate.map(Self::import_match_type).or(Some("unmatched")),
            "matchConfidence": candidate.map(|c| c.score),
            "matchAttempts": existing.as_ref().map(|e| e.1 + 1).unwrap_or(1),
            "verificationStatus": if candidate.is_some() { Some("matched") } else { Some("unmatched") },
            "verificationReason": unmatched_reason,
            "copiedAt": copied_at,
            "copyError": copy_error.or(unmatched_reason),
            "copyAttempts": existing.as_ref().map(|e| e.2 + if copy_error.is_some() { 1 } else { 0 }).unwrap_or(if copy_error.is_some() { 1 } else { 0 }),
            // `createdAt`/`updatedAt` are ORM-managed and are not part of the
            // generated Create/Update inputs. Sending them made every single
            // torrent import fail with an "unknown field" GraphQL error, which
            // surfaced as `postProcessStatus = "failed"` on every torrent.
        });

        if let Some((id, _, _)) = existing {
            let data = self
                .execute_mutation(
                    auth_user,
                    r#"mutation UpdatePendingFileMatch($id: String!, $input: UpdatePendingFileMatchInput!) {
                        updatePendingFileMatch(id: $id, input: $input) { success error }
                    }"#,
                    serde_json::json!({
                        "id": id,
                        "input": input
                    }),
                )
                .await?;
            if !data
                .get("updatePendingFileMatch")
                .and_then(|v| v.get("success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                anyhow::bail!("updatePendingFileMatch failed");
            }
        } else {
            let create_input = input;
            let data = self
                .execute_mutation(
                    auth_user,
                    r#"mutation CreatePendingFileMatch($input: CreatePendingFileMatchInput!) {
                        createPendingFileMatch(input: $input) { success error }
                    }"#,
                    serde_json::json!({ "input": create_input }),
                )
                .await?;
            if !data
                .get("createPendingFileMatch")
                .and_then(|v| v.get("success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                anyhow::bail!("createPendingFileMatch failed");
            }
        }
        Ok(())
    }

    async fn find_pending_file_match(
        &self,
        auth_user: &AuthUser,
        owner_id: &str,
        info_hash: &str,
        file_index: Option<i32>,
        source_path: &str,
    ) -> Result<Option<(String, i32, i32)>> {
        let where_input = if let Some(index) = file_index {
            serde_json::json!({
                "sourceType": { "eq": "torrent" },
                "sourceId": { "eq": info_hash },
                "sourceFileIndex": { "eq": index }
            })
        } else {
            serde_json::json!({
                "sourceType": { "eq": "torrent" },
                "sourceId": { "eq": info_hash },
                "sourcePath": { "eq": source_path }
            })
        };
        let mut where_input = where_input;
        where_input["userId"] = serde_json::json!({ "eq": owner_id });
        let data = self
            .execute_graphql(
                auth_user,
                r#"query PendingFileMatchForSource($where: PendingFileMatchWhereInput!) {
                    PendingFileMatches: pendingFileMatches(where: $where, page: { limit: 2 }) {
                        Edges: edges {
                            Node: node {
                                Id: id
                                MatchAttempts: matchAttempts
                                CopyAttempts: copyAttempts
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "where": where_input }),
            )
            .await?;
        if data
            .get("PendingFileMatches")
            .and_then(|value| value.get("Edges"))
            .and_then(|value| value.as_array())
            .is_some_and(|rows| rows.len() > 1)
        {
            anyhow::bail!("Pending source identity is ambiguous");
        }
        Ok(data
            .get("PendingFileMatches")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| {
                Some((
                    node.get("Id")?.as_str()?.to_string(),
                    node.get("MatchAttempts")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32,
                    node.get("CopyAttempts")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32,
                ))
            }))
    }

    async fn update_torrent_post_process_status(
        &self,
        import: &AuthorizedTorrentImport,
        status: &str,
        error: Option<&str>,
        _processed: i32,
        _failed: i32,
    ) -> Result<()> {
        let database = self.database().await?;
        Self::record_torrent_import_status(database.pool(), import, status, error).await
    }

    async fn record_torrent_import_status(
        db: &crate::db::Database,
        import: &AuthorizedTorrentImport,
        status: &str,
        error: Option<&str>,
    ) -> Result<()> {
        use crate::graphql::entities::{Torrent, UpdateTorrentInput};
        let import = import.clone();
        let status = status.to_string();
        let error = error.map(str::to_string);
        let authority = DbAuthContext {
            user_id: Some(import.owner_id.clone()),
            actor_id: Some(import.actor.user_id.clone()),
            subject: Some(import.actor.user_id.clone()),
            roles: import.actor.role.iter().cloned().collect(),
            ..Default::default()
        };
        db.transaction_with_auth(TransactionMode::StateMachine, Some(&authority), move |tx| {
            Box::pin(async move {
                let row = tx
                    .find_by_id::<Torrent>(&import.id)
                    .await?
                    .ok_or_else(OrmPublicError::forbidden)?;
                if row.info_hash != import.info_hash
                    || row.user_id != import.owner_id
                    || row.library_id != import.link.library_id
                {
                    return Err(OrmPublicError::forbidden());
                }
                if import.actor.user_id != "system" || !import.actor.is_admin() {
                    let actor = tx
                        .find_by_id::<User>(&import.actor.user_id)
                        .await?
                        .ok_or_else(OrmPublicError::forbidden)?;
                    let current_admin = import.actor.is_admin()
                        && crate::services::graphql::auth::Role::parse(&actor.role)
                            == Some(crate::services::graphql::auth::Role::Admin);
                    if !actor.is_active
                        || (!current_admin && actor.id != import.owner_id)
                        || crate::services::graphql::auth::Role::parse(&actor.role).is_none()
                    {
                        return Err(OrmPublicError::forbidden());
                    }
                }
                tx.update_by_id::<Torrent>(
                    &import.id,
                    UpdateTorrentInput {
                        post_process_status: Some(Some(status)),
                        post_process_error: Some(error),
                        processed_at: Some(Some(Utc::now().to_rfc3339())),
                        ..Default::default()
                    },
                )
                .await?
                .ok_or_else(OrmPublicError::forbidden)?;
                Ok(())
            })
        })
        .await
        .context("Authorized torrent import status update failed")
    }

    fn is_supported_import_extension(path: &str) -> bool {
        !Self::compatible_library_types_for_path(path).is_empty()
    }

    fn compatible_library_types_for_path(path: &str) -> HashSet<String> {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|value| value.to_ascii_lowercase());
        match ext.as_deref() {
            Some(
                "mkv" | "mp4" | "avi" | "m4v" | "mov" | "wmv" | "flv" | "webm" | "mpeg" | "mpg"
                | "ts" | "m2ts",
            ) => HashSet::from(["movies".to_string(), "tv".to_string()]),
            Some(
                "mp3" | "flac" | "m4a" | "m4b" | "aac" | "ogg" | "opus" | "wav" | "wma" | "aiff"
                | "alac" | "ape" | "dsf" | "dff",
            ) => HashSet::from(["music".to_string(), "audiobooks".to_string()]),
            _ => HashSet::new(),
        }
    }

    fn candidate_matches_library_type(target_type: &str, normalized_library_type: &str) -> bool {
        matches!(
            (normalized_library_type, target_type),
            ("movies", "Movie")
                | ("tv", "Episode")
                | ("music", "Track")
                | ("audiobooks", "Chapter")
        )
    }

    /// Remove the records of files that are gone: any record the walk did not find whose file the
    /// storage says is not there. That covers a record the walk can never find again, one in the
    /// library's own trash or one outside the library's folder (left behind by a move), as well as
    /// a file deleted from inside the library. A record whose file cannot be checked stays, so a
    /// share or mount that is down never costs a record. Nothing on disk is touched, and each
    /// removal is the entity delete, so links to tracks and other items cascade as they always do.
    async fn reconcile_missing_media_files(
        &self,
        auth_user: &AuthUser,
        library: &LibraryRow,
        discovered_paths: &HashSet<String>,
        rows: &[ExistingMediaFileRow],
    ) -> Result<MissingRecordSummary> {
        let root = PathBuf::from(&library.path).into_storage_path();
        let mut summary = MissingRecordSummary::default();
        for row in rows {
            if discovered_paths.contains(&row.path) {
                continue;
            }
            // A file that cannot be checked is kept: only a storage that answers "not there" counts.
            match library_storage().exists(&row.path).await {
                Ok(false) => {}
                Ok(true) => continue,
                Err(error) => {
                    summary.kept_unchecked += 1;
                    debug!(
                        library_id = %library.id,
                        path = %row.path,
                        error = %error,
                        "Kept the record of '{}': its file could not be checked",
                        row.path
                    );
                    continue;
                }
            }
            let place = RecordPlace::of(&row.path, &root);
            let deleted = self
                .execute_mutation(
                    auth_user,
                    r#"mutation DeleteMissingMediaFile($id: String!) {
                        DeleteMediaFile: deleteMediaFile(id: $id) { Success: success Error: error }
                    }"#,
                    serde_json::json!({ "id": row.id }),
                )
                .await
                .ok()
                .and_then(|data| data.get("DeleteMediaFile")?.get("Success")?.as_bool())
                .unwrap_or(false);
            if !deleted {
                summary.delete_failed += 1;
                warn!(
                    library_id = %library.id,
                    media_file_id = %row.id,
                    path = %row.path,
                    "Could not remove the record of '{}' (its file is gone)",
                    row.path
                );
                continue;
            }
            match place {
                RecordPlace::Library => summary.removed_in_library += 1,
                RecordPlace::Trash => summary.removed_in_trash += 1,
                RecordPlace::OutsideRoot => summary.removed_outside_root += 1,
            }
        }

        Ok(summary)
    }

    async fn get_tv_shows_for_library(
        &self,
        auth_user: &AuthUser,
        library_id: &str,
    ) -> Result<Vec<TvShowFolderRow>> {
        let data = self
            .execute_graphql_paged(
                auth_user,
                r#"query TvShowsForLibrary($libraryId: String!, $page: PageInput) {
                    Shows: shows(
                        where: { libraryId: { eq: $libraryId } }
                        page: $page
                    ) {
                        Edges: edges {
                            Node: node {
                                Id: id
                                Name: name
                                Path: path
                                Year: year
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "libraryId": library_id }),
                "Shows",
            )
            .await?;

        let shows = data
            .get("Shows")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                let node = edge.get("Node")?;
                let show_id = node.get("Id")?.as_str()?.to_string();
                let show_name = node.get("Name")?.as_str()?.to_string();
                let show_path = node
                    .get("Path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let show_year = node.get("Year").and_then(|v| v.as_i64()).map(|v| v as i32);
                Some((show_id, show_name, show_path, show_year))
            })
            .collect::<Vec<_>>();
        let show_ids = shows
            .iter()
            .map(|(show_id, _, _, _)| show_id.clone())
            .collect::<Vec<_>>();
        let mut seasons_by_show: HashMap<String, Vec<i32>> = HashMap::new();
        for episode in self.load_episodes_for_show_ids(&show_ids).await? {
            seasons_by_show
                .entry(episode.show_id)
                .or_default()
                .push(episode.season);
        }

        Ok(shows
            .into_iter()
            .map(|(show_id, name, path, year)| {
                let mut seasons = seasons_by_show.remove(&show_id).unwrap_or_default();
                seasons.sort_unstable();
                seasons.dedup();
                (name, path, year, seasons)
            })
            .collect())
    }

    async fn ensure_tv_folder_structure(&self, library: &LibraryRow) -> Result<()> {
        let naming_pattern = self.resolve_library_naming_pattern(library).await?;
        let auth_user = self.system_auth_user(Some(&library.user_id)).await?;
        let shows = self
            .get_tv_shows_for_library(&auth_user, &library.id)
            .await?;

        for (show_name, show_path, show_year, seasons) in shows {
            if let Some(path) = show_path {
                create_library_directory_in(Path::new(&library.path), Path::new(&path)).await?;
            }

            if seasons.is_empty() {
                let (show_dir, _) = derive_tv_dirs_from_pattern(
                    &library.path,
                    &naming_pattern,
                    &show_name,
                    1,
                    show_year,
                )?;
                create_library_directory_in(Path::new(&library.path), &show_dir).await?;
                continue;
            }

            for season in seasons {
                let (show_dir, season_dir) = derive_tv_dirs_from_pattern(
                    &library.path,
                    &naming_pattern,
                    &show_name,
                    season,
                    show_year,
                )?;
                create_library_directory_in(Path::new(&library.path), &show_dir).await?;
                create_library_directory_in(Path::new(&library.path), &season_dir).await?;
            }
        }

        Ok(())
    }

    /// What every library item's path is made from, for the whole library in a few paged reads, so
    /// a scan can tell which files are out of place without asking about each one.
    async fn load_target_facts(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
    ) -> Result<HashMap<String, TargetFacts>> {
        let mut facts = HashMap::new();
        let nodes = |data: &serde_json::Value, connection: &str| -> Vec<serde_json::Value> {
            data.get(connection)
                .and_then(|value| value.get("Edges"))
                .and_then(|value| value.as_array())
                .into_iter()
                .flatten()
                .filter_map(|edge| edge.get("Node").cloned())
                .collect()
        };
        let text = |node: &serde_json::Value, key: &str| -> Option<String> {
            node.get(key)
                .and_then(|value| value.as_str())
                .map(str::to_string)
        };
        let variables = serde_json::json!({ "libraryId": library.id });
        match Self::normalize_library_type(&library.library_type).as_str() {
            "movies" => {
                let data = self
                    .execute_graphql_paged(
                        auth_user,
                        r#"query OrganizeLibraryMovies($libraryId: String!, $page: PageInput) {
                            Movies: movies(where: { libraryId: { eq: $libraryId } }, page: $page) {
                                Edges: edges { Node: node { Id: id Title: title Year: year } }
                            }
                        }"#,
                        variables,
                        "Movies",
                    )
                    .await?;
                for node in nodes(&data, "Movies") {
                    let (Some(id), Some(title)) = (text(&node, "Id"), text(&node, "Title")) else {
                        continue;
                    };
                    let year = node.get("Year").and_then(|v| v.as_i64()).map(|v| v as i32);
                    facts.insert(id, TargetFacts::Movie { title, year });
                }
            }
            "tv" => {
                let data = self
                    .execute_graphql_paged(
                        auth_user,
                        r#"query OrganizeLibraryShows($libraryId: String!, $page: PageInput) {
                            Shows: shows(where: { libraryId: { eq: $libraryId } }, page: $page) {
                                Edges: edges { Node: node { Id: id Name: name } }
                            }
                        }"#,
                        variables,
                        "Shows",
                    )
                    .await?;
                let shows: HashMap<String, String> = nodes(&data, "Shows")
                    .iter()
                    .filter_map(|node| Some((text(node, "Id")?, text(node, "Name")?)))
                    .collect();
                let show_ids: Vec<String> = shows.keys().cloned().collect();
                for episode in self.load_episodes_for_show_ids(&show_ids).await? {
                    let Some(show_name) = shows.get(&episode.show_id) else {
                        continue;
                    };
                    facts.insert(
                        episode.id,
                        TargetFacts::Episode {
                            show_name: show_name.clone(),
                            season: episode.season,
                            episode: episode.episode,
                            title: episode.title,
                        },
                    );
                }
            }
            "music" => {
                let data = self
                    .execute_graphql_paged(
                        auth_user,
                        r#"query OrganizeLibraryAlbums($libraryId: String!, $page: PageInput) {
                            Albums: albums(where: { libraryId: { eq: $libraryId } }, page: $page) {
                                Edges: edges {
                                    Node: node { Id: id Name: name Year: year Artist: artist { Name: name } }
                                }
                            }
                        }"#,
                        variables,
                        "Albums",
                    )
                    .await?;
                let albums: HashMap<String, (String, Option<i32>, Option<String>)> =
                    nodes(&data, "Albums")
                        .iter()
                        .filter_map(|node| {
                            Some((
                                text(node, "Id")?,
                                (
                                    text(node, "Name")?,
                                    node.get("Year").and_then(|v| v.as_i64()).map(|v| v as i32),
                                    node.pointer("/Artist/Name")
                                        .and_then(|v| v.as_str())
                                        .map(str::to_string),
                                ),
                            ))
                        })
                        .collect();
                let tracks = self.load_tracks_for_library(&library.id).await?;
                let mut discs: HashMap<String, i32> = HashMap::new();
                for track in &tracks {
                    let most = discs.entry(track.album_id.clone()).or_insert(1);
                    *most = (*most).max(track.disc_number.unwrap_or(1));
                }
                for track in tracks {
                    let Some((album_name, album_year, artist_name)) = albums.get(&track.album_id)
                    else {
                        continue;
                    };
                    facts.insert(
                        track.id,
                        TargetFacts::Track {
                            title: track.title,
                            album_name: album_name.clone(),
                            album_year: *album_year,
                            track_number: track.track_number,
                            disc_number: track.disc_number,
                            discs: discs.get(&track.album_id).copied().unwrap_or(1),
                            artist_name: artist_name.clone(),
                        },
                    );
                }
            }
            "audiobooks" => {
                let data = self
                    .execute_graphql_paged(
                        auth_user,
                        r#"query OrganizeLibraryBooks($libraryId: String!, $page: PageInput) {
                            Audiobooks: audiobooks(where: { libraryId: { eq: $libraryId } }, page: $page) {
                                Edges: edges { Node: node { Id: id Title: title AuthorName: authorName } }
                            }
                        }"#,
                        variables,
                        "Audiobooks",
                    )
                    .await?;
                let books: HashMap<String, (String, Option<String>)> = nodes(&data, "Audiobooks")
                    .iter()
                    .filter_map(|node| {
                        Some((
                            text(node, "Id")?,
                            (text(node, "Title")?, text(node, "AuthorName")),
                        ))
                    })
                    .collect();
                let book_ids: Vec<String> = books.keys().cloned().collect();
                for chapter in self.load_chapters_for_audiobook_ids(&book_ids).await? {
                    let Some((book_title, author_name)) = books.get(&chapter.audiobook_id) else {
                        continue;
                    };
                    facts.insert(
                        chapter.id,
                        TargetFacts::Chapter {
                            author_name: author_name.clone(),
                            book_title: book_title.clone(),
                            chapter_number: chapter.chapter_number,
                            chapter_title: chapter.title,
                        },
                    );
                }
            }
            _ => {}
        }
        Ok(facts)
    }

    /// Organize files a scan skipped as unchanged but that are not where the naming pattern puts
    /// them. Nothing is re-matched or re-read: where each file belongs is worked out from the
    /// library's own records in bulk, and only a file that is somewhere else goes through
    /// `organize_media_file`, which re-plans it, never overwrites, and records a failure. Returns
    /// how many files moved.
    async fn organize_unchanged_matched_files(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        files: Vec<MatchedFile>,
    ) -> usize {
        if files.is_empty() {
            return 0;
        }
        let naming_pattern = match self.resolve_library_naming_pattern(library).await {
            Ok(pattern) => pattern,
            Err(error) => {
                warn!(
                    library_id = %library.id,
                    library_name = %library.name,
                    error = %error,
                    "Could not check library '{}' for misplaced files: no usable naming pattern: {}",
                    library.name,
                    error
                );
                return 0;
            }
        };
        let facts = match self.load_target_facts(library, auth_user).await {
            Ok(facts) => facts,
            Err(error) => {
                warn!(
                    library_id = %library.id,
                    library_name = %library.name,
                    error = %error,
                    "Could not check library '{}' for misplaced files: reading its items failed: {}",
                    library.name,
                    error
                );
                return 0;
            }
        };
        let root = PathBuf::from(&library.path);
        let mut moved = 0;
        for file in files {
            let Some(facts) = facts.get(&file.target_id) else {
                continue;
            };
            let current = Path::new(&file.path);
            let original_filename = current
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("file");
            let ext = current
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("mkv");
            let planned = match facts
                .relative_path(&naming_pattern, original_filename, ext)
                .and_then(|relative| {
                    validate_relative_library_path(&relative)?;
                    Ok(root.join(relative))
                }) {
                Ok(planned) => planned,
                Err(error) => {
                    debug!(
                        library_id = %library.id,
                        media_file_id = %file.media_file_id,
                        media_file_path = %file.path,
                        reason = %error,
                        "Left '{}' where it is: no path to plan for it: {}",
                        file.path,
                        error
                    );
                    continue;
                }
            };
            // A book split in parts, or sharing one file, is named by `organize_media_file` after
            // its parts, so only the folder is compared for chapters.
            let in_place = match facts {
                TargetFacts::Chapter { .. } => current.parent() == planned.parent(),
                _ => current == planned,
            };
            if in_place {
                continue;
            }
            // Same guard as for a new match, without its notification: the person was told then.
            if file.target_type == "Movie" && file.match_type.as_deref() != Some(MANUAL_MATCH_TYPE)
            {
                let verdict = self
                    .should_skip_auto_organize_for_match(
                        library,
                        auth_user,
                        &file.media_file_id,
                        &file.path,
                        &MatchResult {
                            success: true,
                            matched_type: Some(file.target_type.to_string()),
                            matched_id: Some(file.target_id.clone()),
                            ..Default::default()
                        },
                    )
                    .await;
                if !matches!(verdict, Ok(None)) {
                    debug!(
                        library_id = %library.id,
                        media_file_id = %file.media_file_id,
                        media_file_path = %file.path,
                        "Left '{}' where it is: its name and its matched movie disagree on the year",
                        file.path
                    );
                    continue;
                }
            }
            let outcome = self.move_media_file_to_pattern(&file.media_file_id).await;
            match &outcome {
                Ok(result) if result.success => {
                    if result.new_path.as_deref() != Some(file.path.as_str()) {
                        moved += 1;
                        info!(
                            library_id = %library.id,
                            media_file_id = %file.media_file_id,
                            old_path = %file.path,
                            new_path = result.new_path.as_deref().unwrap_or(""),
                            "Moved '{}' to '{}': it was not where the naming pattern puts it",
                            file.path,
                            result.new_path.as_deref().unwrap_or("")
                        );
                    }
                }
                _ => {
                    let failure = match &outcome {
                        Ok(result) => {
                            Self::sanitized_organize_failure_reason(result.reason.as_deref())
                        }
                        Err(_) => "The file operation or database update did not complete safely. Review the server log for the correlated internal error.".to_string(),
                    };
                    warn!(
                        library_id = %library.id,
                        media_file_id = %file.media_file_id,
                        media_file_path = %file.path,
                        planned_path = %planned.display(),
                        "Could not move '{}' to '{}': {}",
                        file.path,
                        planned.display(),
                        match &outcome {
                            Ok(result) => result.reason.clone().unwrap_or_default(),
                            Err(error) => error.to_string(),
                        }
                    );
                    self.ensure_auto_organize_issue_notification(
                        auth_user,
                        library,
                        &file.media_file_id,
                        &file.path,
                        "Auto-organize failed",
                        "organize matched media file",
                        &failure,
                    )
                    .await;
                }
            }
        }
        moved
    }

    async fn cleanup_empty_folders(&self, library: &LibraryRow) -> Result<()> {
        let library_root = PathBuf::from(&library.path);
        if !library_root.exists() {
            return Ok(());
        }

        let protected = self.build_protected_paths(library).await?;
        Self::cleanup_empty_unprotected_folders(&library_root, &protected).await
    }

    /// Remove every empty folder under the library root that no library item expects, a whole
    /// empty tree at a time. Never the root, never a folder with anything in it (hidden files
    /// included), and never a hidden folder such as the library's `.librarian-trash` or a sync
    /// tool's marker, which are not this scan's to judge.
    async fn cleanup_empty_unprotected_folders(
        library_root: &Path,
        protected: &HashSet<PathBuf>,
    ) -> Result<()> {
        if !library_storage()
            .exists(library_root)
            .await
            .unwrap_or(false)
        {
            return Ok(());
        }

        // Hidden folders (the library's trash, a sync tool's markers) are never entered.
        let prune: WalkFilter =
            Arc::new(|entry| entry.depth > 0 && is_hidden_name(std::ffi::OsStr::new(entry.name)));
        let report: WalkFilter = Arc::new(|entry| entry.kind == EntryKind::Directory);
        let mut walk = library_storage()
            .walk(
                library_root,
                WalkOptions {
                    prune: Some(prune),
                    report: Some(report),
                    with_metadata: false,
                },
                CancellationToken::new(),
            )
            .await?;
        let mut dirs = Vec::new();
        while let Some(entry) = walk.entries.recv().await {
            // An entry that cannot be read is left out, as it always was.
            if let Ok(entry) = entry {
                dirs.push(PathBuf::from(entry.path.into_string()));
            }
        }
        walk.task
            .await
            .context("Empty-folder traversal worker failed")?;

        // Deepest first, so a folder whose last child was just removed is empty by its turn.
        dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));

        for dir in dirs {
            if dir == library_root || protected.contains(&dir) {
                continue;
            }
            // `remove_dir` only ever removes an empty directory, so a file that lands here after
            // the walk is never lost.
            match library_storage().remove_dir(&dir).await {
                Ok(()) => debug!(
                    folder_path = %dir.to_string_lossy(),
                    library_root = %library_root.to_string_lossy(),
                    "Removed empty unprotected library folder: folder_path={}, library_root={}",
                    dir.to_string_lossy(),
                    library_root.to_string_lossy()
                ),
                Err(error) if error.kind() == StorageErrorKind::NotEmpty => {}
                Err(error) => debug!(
                    folder_path = %dir.to_string_lossy(),
                    library_root = %library_root.to_string_lossy(),
                    error = %error,
                    "Failed to remove empty unprotected library folder: folder_path={}, library_root={}, error={}",
                    dir.to_string_lossy(),
                    library_root.to_string_lossy(),
                    error
                ),
            }
        }

        Ok(())
    }

    fn build_tv_protected_paths_from_rows(
        library_path: &str,
        naming_pattern: &str,
        shows: &[TvShowFolderRow],
    ) -> Result<HashSet<PathBuf>> {
        let mut protected = HashSet::new();
        let root = PathBuf::from(library_path);
        protected.insert(root.clone());

        for (show_name, show_path, show_year, seasons) in shows {
            if let Some(path) = show_path {
                checked_library_path(&root, Path::new(path))?;
                protected.insert(PathBuf::from(path));
            }

            if seasons.is_empty() {
                let (show_dir, _) = derive_tv_dirs_from_pattern(
                    library_path,
                    naming_pattern,
                    show_name,
                    1,
                    *show_year,
                )?;
                protected.insert(show_dir);
            } else {
                for season in seasons {
                    let (show_dir, season_dir) = derive_tv_dirs_from_pattern(
                        library_path,
                        naming_pattern,
                        show_name,
                        *season,
                        *show_year,
                    )?;
                    protected.insert(show_dir);
                    protected.insert(season_dir);
                }
            }
        }

        Ok(protected)
    }

    async fn build_tv_protected_paths(&self, library: &LibraryRow) -> Result<HashSet<PathBuf>> {
        let auth_user = self.system_auth_user(Some(&library.user_id)).await?;
        let naming_pattern = self.resolve_library_naming_pattern(library).await?;

        let shows = self
            .get_tv_shows_for_library(&auth_user, &library.id)
            .await?;
        Self::build_tv_protected_paths_from_rows(&library.path, &naming_pattern, &shows)
    }

    fn protect_path_and_ancestors(
        &self,
        protected: &mut HashSet<PathBuf>,
        root: &Path,
        path: PathBuf,
    ) {
        let mut current = Some(path);
        while let Some(dir) = current {
            if !dir.starts_with(root) {
                break;
            }
            protected.insert(dir.clone());
            if dir == root {
                break;
            }
            current = dir.parent().map(|p| p.to_path_buf());
        }
    }

    async fn build_movie_protected_paths(&self, library: &LibraryRow) -> Result<HashSet<PathBuf>> {
        let naming_pattern = self.resolve_library_naming_pattern(library).await?;
        let auth_user = self.system_auth_user(Some(&library.user_id)).await?;

        let mut protected = HashSet::new();
        let root = PathBuf::from(&library.path);
        protected.insert(root.clone());

        let data = self
            .execute_graphql_paged(
                &auth_user,
                r#"query ProtectedMoviePaths($libraryId: String!, $page: PageInput) {
                    Movies: movies(where: { libraryId: { eq: $libraryId } }, page: $page) {
                        Edges: edges {
                            Node: node {
                                Title: title
                                Year: year
                            }
                        }
                    }
                }"#,
                serde_json::json!({
                    "libraryId": library.id,
                }),
                "Movies",
            )
            .await?;

        let edges = data
            .get("Movies")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for edge in edges {
            let node = edge.get("Node").cloned().unwrap_or_default();
            let Some(title) = node.get("Title").and_then(|v| v.as_str()) else {
                continue;
            };
            let year = node.get("Year").and_then(|v| v.as_i64()).map(|v| v as i32);
            let rel = apply_movie_naming_pattern(&naming_pattern, title, year, "dummy.mkv", "mkv");
            validate_relative_library_path(&rel)?;
            let target = root.join(rel);
            checked_library_path(&root, &target)?;
            if let Some(parent) = target.parent().map(|p| p.to_path_buf()) {
                self.protect_path_and_ancestors(&mut protected, &root, parent);
            }
        }

        Ok(protected)
    }

    async fn build_music_protected_paths(&self, library: &LibraryRow) -> Result<HashSet<PathBuf>> {
        let naming_pattern = self.resolve_library_naming_pattern(library).await?;
        let auth_user = self.system_auth_user(Some(&library.user_id)).await?;

        let mut protected = HashSet::new();
        let root = PathBuf::from(&library.path);
        protected.insert(root.clone());

        let data = self
            .execute_graphql_paged(
                &auth_user,
                r#"query ProtectedMusicPaths($libraryId: String!, $page: PageInput) {
                    Albums: albums(where: { libraryId: { eq: $libraryId } }, page: $page) {
                        Edges: edges {
                            Node: node {
                                Id: id
                                Name: name
                                Year: year
                                Artist: artist { Name: name }
                            }
                        }
                    }
                }"#,
                serde_json::json!({
                    "libraryId": library.id,
                }),
                "Albums",
            )
            .await?;

        let artists: HashMap<String, String> = data
            .pointer("/Albums/Edges")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|edge| {
                let node = edge.get("Node")?;
                Some((
                    node.get("Id")?.as_str()?.to_string(),
                    node.pointer("/Artist/Name")?.as_str()?.to_string(),
                ))
            })
            .collect();
        let albums = data
            .get("Albums")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                let node = edge.get("Node")?;
                Some((
                    node.get("Id")?.as_str()?.to_string(),
                    (
                        node.get("Name")?.as_str()?.to_string(),
                        node.get("Year")
                            .and_then(|value| value.as_i64())
                            .map(|value| value as i32),
                    ),
                ))
            })
            .collect::<HashMap<_, _>>();
        let media_paths = self
            .load_media_files_by_path(&library.id)
            .await?
            .into_values()
            .map(|file| (file.id, file.path))
            .collect::<HashMap<_, _>>();
        let tracks = self.load_tracks_for_library(&library.id).await?;
        let mut album_discs: HashMap<String, i32> = HashMap::new();
        for track in &tracks {
            let discs = album_discs.entry(track.album_id.clone()).or_insert(1);
            *discs = (*discs).max(track.disc_number.unwrap_or(1));
        }
        for track in tracks {
            let Some((album_name, album_year)) = albums.get(&track.album_id) else {
                continue;
            };
            let file_path = track
                .media_file_id
                .as_ref()
                .and_then(|id| media_paths.get(id))
                .map(String::as_str)
                .unwrap_or("track.mp3");
            let original_filename = Path::new(file_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("track.mp3");
            // The same facts organizing uses; a track with no real artist has no folder to keep.
            let Ok(rel) = (TargetFacts::Track {
                title: track.title.clone(),
                album_name: album_name.clone(),
                album_year: *album_year,
                track_number: track.track_number,
                disc_number: track.disc_number,
                discs: album_discs.get(&track.album_id).copied().unwrap_or(1),
                artist_name: artists.get(&track.album_id).cloned(),
            })
            .relative_path(&naming_pattern, original_filename, "mp3") else {
                continue;
            };
            validate_relative_library_path(&rel)?;
            let target = root.join(rel);
            checked_library_path(&root, &target)?;
            if let Some(parent) = target.parent().map(PathBuf::from) {
                self.protect_path_and_ancestors(&mut protected, &root, parent);
            }
        }

        Ok(protected)
    }

    async fn build_audiobook_protected_paths(
        &self,
        library: &LibraryRow,
    ) -> Result<HashSet<PathBuf>> {
        let naming_pattern = self.resolve_library_naming_pattern(library).await?;
        let auth_user = self.system_auth_user(Some(&library.user_id)).await?;

        let mut protected = HashSet::new();
        let root = PathBuf::from(&library.path);
        protected.insert(root.clone());

        let data = self
            .execute_graphql_paged(
                &auth_user,
                r#"query ProtectedAudiobookPaths($libraryId: String!, $page: PageInput) {
                    Audiobooks: audiobooks(where: { libraryId: { eq: $libraryId } }, page: $page) {
                        Edges: edges {
                            Node: node {
                                Id: id
                                Title: title
                                AuthorName: authorName
                            }
                        }
                    }
                }"#,
                serde_json::json!({
                    "libraryId": library.id,
                }),
                "Audiobooks",
            )
            .await?;

        let audiobooks = data
            .get("Audiobooks")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                let node = edge.get("Node")?;
                Some((
                    node.get("Id")?.as_str()?.to_string(),
                    (
                        node.get("Title")?.as_str()?.to_string(),
                        node.get("AuthorName")
                            .and_then(|value| value.as_str())
                            .map(str::to_string),
                    ),
                ))
            })
            .collect::<HashMap<_, _>>();
        let audiobook_ids = audiobooks.keys().cloned().collect::<Vec<_>>();
        let media_paths = self
            .load_media_files_by_path(&library.id)
            .await?
            .into_values()
            .map(|file| (file.id, file.path))
            .collect::<HashMap<_, _>>();
        for chapter in self.load_chapters_for_audiobook_ids(&audiobook_ids).await? {
            let Some((book_title, author_name)) = audiobooks.get(&chapter.audiobook_id) else {
                continue;
            };
            let file_path = chapter
                .media_file_id
                .as_ref()
                .and_then(|id| media_paths.get(id))
                .map(String::as_str)
                .unwrap_or("chapter.m4b");
            let original_filename = Path::new(file_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("chapter.m4b");
            let rel = apply_audiobook_naming_pattern(
                &naming_pattern,
                author_name.as_deref().unwrap_or("Unknown Author"),
                book_title,
                chapter.chapter_number,
                chapter.title.as_deref(),
                original_filename,
                "m4b",
            );
            validate_relative_library_path(&rel)?;
            let target = root.join(rel);
            checked_library_path(&root, &target)?;
            if let Some(parent) = target.parent().map(PathBuf::from) {
                self.protect_path_and_ancestors(&mut protected, &root, parent);
            }
        }

        Ok(protected)
    }

    async fn build_protected_paths(&self, library: &LibraryRow) -> Result<HashSet<PathBuf>> {
        match Self::normalize_library_type(&library.library_type).as_str() {
            "tv" => self.build_tv_protected_paths(library).await,
            "movies" => self.build_movie_protected_paths(library).await,
            "music" => self.build_music_protected_paths(library).await,
            "audiobooks" => self.build_audiobook_protected_paths(library).await,
            _ => {
                let mut set = HashSet::new();
                set.insert(PathBuf::from(&library.path));
                Ok(set)
            }
        }
    }
}

#[async_trait]
impl Service for LibraryScanService {
    fn name(&self) -> &str {
        "library_scan"
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["database".to_string(), "graphql".to_string()]
    }

    async fn start(&self) -> Result<()> {
        if self.config.autoscan_poll_interval.is_zero() {
            anyhow::bail!("Library scan autoscan poll interval must be greater than zero");
        }
        if self.config.analyze_workers == 0 {
            anyhow::bail!("Library scan analyze worker count must be greater than zero");
        }
        self.clear_stale_library_scanning_state_on_startup().await?;

        let ffprobe_available = self.check_ffprobe_available().await;
        {
            let mut guard = self.ffprobe_available.write().await;
            *guard = ffprobe_available;
        }

        if ffprobe_available {
            info!("ffprobe startup check passed: media analysis is enabled");
        } else {
            warn!(
                "ffprobe startup check failed: media analysis jobs will fail until ffprobe is installed and backend is restarted"
            );
            if let Err(e) = self.ensure_ffprobe_missing_notification().await {
                warn!(
                    error = %e,
                    "Failed to create ffprobe-missing startup notification"
                );
            }
        }

        info!(
            service = "library_scan",
            autoscan_poll_interval_secs = self.config.autoscan_poll_interval.as_secs(),
            analyze_workers = self.config.analyze_workers.max(1),
            "Starting library scan service: autoscan_poll_interval_secs={}, analyze_workers={}",
            self.config.autoscan_poll_interval.as_secs(),
            self.config.analyze_workers.max(1)
        );

        let cancel = CancellationToken::new();
        let this = self
            .manager
            .get_library_scan_unchecked()
            .await
            .ok_or_else(|| anyhow::anyhow!("library_scan service handle not registered"))?;

        this.clear_interrupted_chapter_checks().await;

        // Also before any scan: earlier versions could add a new record for a file on every scan.
        if let Err(error) = this.collapse_duplicate_media_files().await {
            warn!(error = %error, "Could not collapse duplicate media file records");
        }

        // Before any scan can start: it may rename a file, and a scan that sees the new name before
        // the database does takes it for a new file and drops the old record.
        if let Err(error) = this.repair_whole_file_audiobooks().await {
            warn!(error = %error, "Could not repair whole-file audiobooks");
        }

        let scheduler = this.clone();
        let scheduler_handle = tokio::spawn(supervise_worker(
            "scheduler",
            cancel.child_token(),
            move |worker_cancel| scheduler.clone().run_scheduler(worker_cancel),
        ));
        let scan_worker = this.clone();
        let scan_worker_handle = tokio::spawn(supervise_worker(
            "scan",
            cancel.child_token(),
            move |worker_cancel| scan_worker.clone().run_scan_worker(worker_cancel),
        ));

        let watcher = this.clone();
        let watch_handle = tokio::spawn(supervise_worker(
            "watch",
            cancel.child_token(),
            move |worker_cancel| watcher.clone().run_watch_supervisor(worker_cancel),
        ));

        let mut analyze_worker_handles = Vec::new();
        for idx in 0..self.config.analyze_workers {
            let analyze_worker = this.clone();
            analyze_worker_handles.push(tokio::spawn(supervise_worker(
                "analysis",
                cancel.child_token(),
                move |worker_cancel| {
                    analyze_worker
                        .clone()
                        .run_analyze_worker(worker_cancel, idx)
                },
            )));
        }

        *self.runtime.write().await = Some(Runtime {
            cancel,
            scheduler_handle,
            scan_worker_handle,
            watch_handle,
            analyze_worker_handles,
        });

        info!(
            service = "library_scan",
            analyze_workers = self.config.analyze_workers.max(1),
            "Library scan service started: analyze_workers={}",
            self.config.analyze_workers.max(1)
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        info!(
            service = "library_scan",
            "Stopping library scan service: canceling scheduler, scan worker, and analyze workers"
        );

        if let Some(runtime) = self.runtime.write().await.take() {
            runtime.cancel.cancel();
            let _ = runtime.scheduler_handle.await;
            let _ = runtime.scan_worker_handle.await;
            let _ = runtime.watch_handle.await;
            for h in runtime.analyze_worker_handles {
                let _ = h.await;
            }
        }

        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        let worker_stopped = {
            let runtime = self.runtime.read().await;
            let Some(runtime) = runtime.as_ref() else {
                return Ok(ServiceHealth::degraded("library scan runtime not running"));
            };
            runtime.scheduler_handle.is_finished()
                || runtime.scan_worker_handle.is_finished()
                || runtime.watch_handle.is_finished()
                || runtime
                    .analyze_worker_handles
                    .iter()
                    .any(JoinHandle::is_finished)
        };
        if worker_stopped {
            return Ok(ServiceHealth::degraded(
                "one or more library scan workers stopped after exhausting restarts",
            ));
        }
        if *self.ffprobe_available.read().await {
            Ok(ServiceHealth::healthy())
        } else {
            Ok(ServiceHealth::degraded(
                "ffprobe is unavailable; media analysis jobs will fail",
            ))
        }
    }
}

#[derive(Debug, Deserialize)]
struct FfprobeRoot {
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
    #[serde(default)]
    chapters: Vec<FfprobeChapter>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    format_name: Option<String>,
    duration: Option<String>,
    bit_rate: Option<String>,
    #[serde(default)]
    tags: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    #[serde(default)]
    index: Option<i32>,
    codec_type: Option<String>,
    codec_name: Option<String>,
    #[serde(default)]
    codec_long_name: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    #[serde(default)]
    display_aspect_ratio: Option<String>,
    #[serde(default)]
    r_frame_rate: Option<String>,
    #[serde(default)]
    avg_frame_rate: Option<String>,
    #[serde(default)]
    bit_rate: Option<String>,
    #[serde(default)]
    pix_fmt: Option<String>,
    #[serde(default)]
    bits_per_raw_sample: Option<String>,
    #[serde(default)]
    bits_per_sample: Option<i32>,
    #[serde(default)]
    color_space: Option<String>,
    #[serde(default)]
    color_transfer: Option<String>,
    #[serde(default)]
    color_primaries: Option<String>,
    #[serde(default)]
    channels: Option<i32>,
    #[serde(default)]
    channel_layout: Option<String>,
    #[serde(default)]
    sample_rate: Option<String>,
    #[serde(default)]
    disposition: Option<FfprobeDisposition>,
    #[serde(default)]
    tags: Option<HashMap<String, String>>,
    /// `1` when the video carries EIA-608/708 captions in its picture stream.
    #[serde(default)]
    closed_captions: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct FfprobeDisposition {
    #[serde(default)]
    default: Option<i32>,
    #[serde(default)]
    forced: Option<i32>,
    #[serde(default)]
    hearing_impaired: Option<i32>,
    #[serde(default)]
    commentary: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct FfprobeChapter {
    // Container chapter IDs are opaque (Matroska commonly uses signed 64-bit
    // values). Ignore them; the application uses sequential chapter positions.
    #[serde(default)]
    start_time: Option<String>,
    #[serde(default)]
    end_time: Option<String>,
    #[serde(default)]
    tags: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Default)]
struct VideoStreamAnalysis {
    stream_index: i32,
    codec: String,
    codec_long_name: Option<String>,
    width: i32,
    height: i32,
    aspect_ratio: Option<String>,
    frame_rate: Option<String>,
    avg_frame_rate: Option<String>,
    bitrate: Option<i32>,
    pixel_format: Option<String>,
    color_space: Option<String>,
    color_transfer: Option<String>,
    color_primaries: Option<String>,
    hdr_type: Option<String>,
    bit_depth: Option<i32>,
    language: Option<String>,
    title: Option<String>,
    is_default: bool,
    metadata: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct AudioStreamAnalysis {
    stream_index: i32,
    codec: String,
    codec_long_name: Option<String>,
    channels: i32,
    channel_layout: Option<String>,
    sample_rate: Option<i32>,
    bitrate: Option<i32>,
    bit_depth: Option<i32>,
    language: Option<String>,
    title: Option<String>,
    is_default: bool,
    is_commentary: bool,
    metadata: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct SubtitleAnalysis {
    source_type: String,
    stream_index: Option<i32>,
    codec: Option<String>,
    codec_long_name: Option<String>,
    language: Option<String>,
    title: Option<String>,
    is_default: bool,
    is_forced: bool,
    is_hearing_impaired: bool,
    metadata: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ChapterAnalysis {
    chapter_index: i32,
    start_secs: f64,
    end_secs: f64,
    title: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ProbeAnalysis {
    /// Closed captions inside a video stream (Q119): counted as subtitles that are present.
    closed_captions: bool,
    container: Option<String>,
    video_codec: Option<String>,
    audio_codec: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    duration: Option<i32>,
    bitrate: Option<i32>,
    resolution: Option<String>,
    is_hdr: bool,
    hdr_type: Option<String>,
    audio_channels: Option<String>,
    metadata: Option<String>,
    video_streams: Vec<VideoStreamAnalysis>,
    audio_streams: Vec<AudioStreamAnalysis>,
    subtitles: Vec<SubtitleAnalysis>,
    chapters: Vec<ChapterAnalysis>,
    /// Exact length, since `duration` is rounded.
    duration_secs: Option<f64>,
    /// Tags from the container, then the first audio stream. Ogg and Opus keep theirs on the stream.
    tags: crate::services::release_match::FileTags,
}

/// Private proof of the authorized target before ffprobe. Recheck it on the pinned
/// transaction so moved/reowned/replaced files cannot receive stale analysis.
#[derive(Debug, Clone)]
struct AnalysisTarget {
    file_id: String,
    library_id: String,
    owner_id: String,
    path: String,
    size: i64,
    file_modified_at: Option<String>,
}

impl AnalysisTarget {
    fn capture(file: &MediaFileRow, library: &LibraryRow, path: &str) -> Result<Self> {
        Uuid::parse_str(&library.user_id).context("Analysis library has no valid owner")?;
        if file.id.is_empty()
            || file.library_id.as_deref() != Some(library.id.as_str())
            || file.path != path
        {
            anyhow::bail!("Analysis target path or library changed before probing");
        }
        Ok(Self {
            file_id: file.id.clone(),
            library_id: library.id.clone(),
            owner_id: library.user_id.clone(),
            path: file.path.clone(),
            size: file.size,
            file_modified_at: file.file_modified_at.clone(),
        })
    }
}

/// Authorized worker action: top-level ffprobe columns and all owned detail rows commit
/// together. These generated context writes preserve existing hooks and postcommit witnesses.
/// The explicit DB auth context keeps execution actor distinct from the target owner.
async fn replace_media_analysis(
    db: &crate::db::Database,
    actor: &AuthUser,
    target: AnalysisTarget,
    analysis: ProbeAnalysis,
) -> Result<()> {
    use crate::graphql::entities::{
        AudioStream, AudioStreamWhereInput, CreateAudioStreamInput, CreateMediaChapterInput,
        CreateSubtitleInput, CreateVideoStreamInput, Library, MediaChapter, MediaChapterWhereInput,
        Subtitle, SubtitleWhereInput, UpdateMediaFileInput, VideoStream, VideoStreamWhereInput,
    };
    if !actor.has_role(crate::services::graphql::auth::Role::Member)
        || (!actor.is_admin() && actor.user_id != target.owner_id)
    {
        anyhow::bail!("Analysis target not found or access denied");
    }
    let authority = DbAuthContext {
        user_id: Some(target.owner_id.clone()),
        subject: Some(actor.user_id.clone()),
        actor_id: Some(actor.user_id.clone()),
        roles: actor.role.iter().cloned().collect(),
        ..Default::default()
    };
    db.transaction_with_auth(TransactionMode::StateMachine, Some(&authority), move |tx| {
        Box::pin(async move {
            let conflict = || {
                OrmPublicError::with_message(
                    OrmErrorCode::Conflict,
                    "Analysis target changed; retry analysis",
                )
            };
            let file = tx
                .find_by_id::<MediaFile>(&target.file_id)
                .await?
                .ok_or_else(conflict)?;
            let library = tx
                .find_by_id::<Library>(&target.library_id)
                .await?
                .ok_or_else(conflict)?;
            if file.library_id.as_deref() != Some(target.library_id.as_str())
                || library.user_id != target.owner_id
                || file.path != target.path
                || file.size != target.size
                || file.file_modified_at != target.file_modified_at
                || tx.find_by_id::<User>(&target.owner_id).await?.is_none()
            {
                return Err(conflict());
            }
            let eq = || StringFilter {
                eq: Some(target.file_id.clone()),
                ..Default::default()
            };
            tx.delete_where::<VideoStream>(VideoStreamWhereInput {
                media_file_id: Some(eq()),
                ..Default::default()
            })
            .await?;
            tx.delete_where::<AudioStream>(AudioStreamWhereInput {
                media_file_id: Some(eq()),
                ..Default::default()
            })
            .await?;
            // Sidecar/downloaded subtitles belong to their own workflows, not this probe.
            tx.delete_where::<Subtitle>(SubtitleWhereInput {
                media_file_id: Some(eq()),
                source_type: Some(StringFilter {
                    eq: Some("embedded".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .await?;
            tx.delete_where::<MediaChapter>(MediaChapterWhereInput {
                media_file_id: Some(eq()),
                ..Default::default()
            })
            .await?;
            for stream in analysis.video_streams {
                tx.insert::<VideoStream>(CreateVideoStreamInput {
                    media_file_id: target.file_id.clone(),
                    stream_index: stream.stream_index,
                    codec: stream.codec,
                    codec_long_name: stream.codec_long_name,
                    width: stream.width,
                    height: stream.height,
                    aspect_ratio: stream.aspect_ratio,
                    frame_rate: stream.frame_rate,
                    avg_frame_rate: stream.avg_frame_rate,
                    bitrate: stream.bitrate,
                    pixel_format: stream.pixel_format,
                    color_space: stream.color_space,
                    color_transfer: stream.color_transfer,
                    color_primaries: stream.color_primaries,
                    hdr_type: stream.hdr_type,
                    bit_depth: stream.bit_depth,
                    language: stream.language,
                    title: stream.title,
                    is_default: stream.is_default,
                    metadata: stream.metadata,
                })
                .await?;
            }
            for stream in analysis.audio_streams {
                tx.insert::<AudioStream>(CreateAudioStreamInput {
                    media_file_id: target.file_id.clone(),
                    stream_index: stream.stream_index,
                    codec: stream.codec,
                    codec_long_name: stream.codec_long_name,
                    channels: stream.channels,
                    channel_layout: stream.channel_layout,
                    sample_rate: stream.sample_rate,
                    bitrate: stream.bitrate,
                    bit_depth: stream.bit_depth,
                    language: stream.language,
                    title: stream.title,
                    is_default: stream.is_default,
                    is_commentary: stream.is_commentary,
                    metadata: stream.metadata,
                })
                .await?;
            }
            for stream in analysis.subtitles {
                tx.insert::<Subtitle>(CreateSubtitleInput {
                    media_file_id: target.file_id.clone(),
                    source_type: stream.source_type,
                    stream_index: stream.stream_index,
                    codec: stream.codec,
                    codec_long_name: stream.codec_long_name,
                    language: stream.language,
                    title: stream.title,
                    is_default: stream.is_default,
                    is_forced: stream.is_forced,
                    is_hearing_impaired: stream.is_hearing_impaired,
                    metadata: stream.metadata,
                    file_path: None,
                    opensubtitles_id: None,
                    downloaded_at: None,
                })
                .await?;
            }
            for stream in analysis.chapters {
                tx.insert::<MediaChapter>(CreateMediaChapterInput {
                    media_file_id: target.file_id.clone(),
                    chapter_index: stream.chapter_index,
                    start_secs: stream.start_secs,
                    end_secs: stream.end_secs,
                    title: stream.title,
                })
                .await?;
            }
            tx.update_by_id::<MediaFile>(
                &target.file_id,
                UpdateMediaFileInput {
                    container: Some(analysis.container),
                    video_codec: Some(analysis.video_codec),
                    audio_codec: Some(analysis.audio_codec),
                    width: Some(analysis.width),
                    height: Some(analysis.height),
                    duration: Some(analysis.duration),
                    bitrate: Some(analysis.bitrate),
                    resolution: Some(analysis.resolution),
                    is_hdr: Some(analysis.is_hdr),
                    hdr_type: Some(analysis.hdr_type),
                    closed_captions: Some(Some(analysis.closed_captions)),
                    audio_channels: Some(analysis.audio_channels),
                    metadata: Some(analysis.metadata),
                    analyzed_at: Some(Some(Utc::now().to_rfc3339())),
                    ..Default::default()
                },
            )
            .await?
            .ok_or_else(conflict)?;
            Ok(())
        })
    })
    .await
    .context("Failed to atomically replace media analysis")
}

const FFPROBE_TIMEOUT: Duration = Duration::from_secs(60);
const FFPROBE_STDOUT_LIMIT: u64 = 16 * 1024 * 1024;
const FFPROBE_STDERR_LIMIT: u64 = 64 * 1024;

#[derive(Debug, thiserror::Error)]
enum MediaAnalysisError {
    #[error("ffprobe executable is unavailable")]
    ExecutableUnavailable,
    #[error("ffprobe timed out after {0} seconds")]
    Timeout(u64),
    #[error("ffprobe output exceeded the allowed size")]
    OutputTooLarge,
    #[error("ffprobe exited with code {exit_code:?}: {stderr}")]
    ProbeFailed {
        exit_code: Option<i32>,
        stderr: String,
    },
    #[error("ffprobe returned unsupported JSON: {0}")]
    MalformedJson(serde_json::Error),
    #[error("ffprobe process failed")]
    Process,
}

fn classify_analysis_error(error: &anyhow::Error) -> (&'static str, &'static str, &'static str) {
    match error.downcast_ref::<MediaAnalysisError>() {
        Some(MediaAnalysisError::ExecutableUnavailable) => (
            "FFPROBE_UNAVAILABLE",
            "Media analysis is unavailable because ffprobe could not be started.",
            "Install or configure ffprobe, then retry analysis.",
        ),
        Some(MediaAnalysisError::Timeout(_)) => (
            "FFPROBE_TIMEOUT",
            "Media analysis exceeded its execution deadline.",
            "Check whether the file or storage is responsive, then retry analysis.",
        ),
        Some(MediaAnalysisError::MalformedJson(_)) => (
            "FFPROBE_MALFORMED_OUTPUT",
            "ffprobe returned output that Librarian could not parse.",
            "Verify the ffprobe version and retry; include the scan reference when reporting a bug.",
        ),
        Some(MediaAnalysisError::OutputTooLarge) => (
            "FFPROBE_OUTPUT_TOO_LARGE",
            "ffprobe returned more analysis data than the safety limit permits.",
            "Review the media file for excessive streams or metadata before retrying.",
        ),
        Some(MediaAnalysisError::ProbeFailed { stderr, .. })
            if stderr.to_ascii_lowercase().contains("permission denied") =>
        {
            (
                "MEDIA_PERMISSION_DENIED",
                "ffprobe could not read this media file.",
                "Grant the Librarian service account read access, then retry analysis.",
            )
        }
        Some(MediaAnalysisError::ProbeFailed { .. }) => (
            "MEDIA_UNSUPPORTED_OR_CORRUPT",
            "ffprobe could not analyze this media file.",
            "Verify that the file is complete and playable, then retry analysis.",
        ),
        Some(MediaAnalysisError::Process) | None => (
            "ANALYSIS_INTERNAL_ERROR",
            "Media analysis failed unexpectedly.",
            "Retry analysis; if it repeats, use the correlated server log for diagnosis.",
        ),
    }
}

async fn read_limited<R>(reader: R, limit: u64) -> std::io::Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut output = Vec::new();
    reader
        .take(limit.saturating_add(1))
        .read_to_end(&mut output)
        .await?;
    Ok(output)
}

async fn ffprobe_analyze(path: &str) -> Result<ProbeAnalysis> {
    // A file on a share is read by ffprobe through a loopback URL; a local file by its path.
    let input = tool_argument(path)
        .await
        .with_context(|| format!("Cannot give ffprobe access to '{path}'"))?;
    let mut command = Command::new("ffprobe");
    command.arg("-v").arg("error");
    if is_tool_url(&input) {
        command
            .arg("-protocol_whitelist")
            .arg(TOOL_PROTOCOL_WHITELIST);
    }
    let mut child = command
        .arg("-show_streams")
        .arg("-show_chapters")
        .arg("-show_format")
        .arg("-print_format")
        .arg("json")
        .arg(&input)
        .kill_on_drop(true)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                MediaAnalysisError::ExecutableUnavailable
            } else {
                MediaAnalysisError::Process
            }
        })?;
    let stdout = child.stdout.take().ok_or(MediaAnalysisError::Process)?;
    let stderr = child.stderr.take().ok_or(MediaAnalysisError::Process)?;
    let completed = tokio::time::timeout(FFPROBE_TIMEOUT, async {
        let (stdout, stderr, status) = tokio::join!(
            read_limited(stdout, FFPROBE_STDOUT_LIMIT),
            read_limited(stderr, FFPROBE_STDERR_LIMIT),
            child.wait()
        );
        Ok::<_, MediaAnalysisError>((
            stdout.map_err(|_| MediaAnalysisError::Process)?,
            stderr.map_err(|_| MediaAnalysisError::Process)?,
            status.map_err(|_| MediaAnalysisError::Process)?,
        ))
    })
    .await;
    let (stdout, stderr, status) = match completed {
        Ok(result) => result?,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(MediaAnalysisError::Timeout(FFPROBE_TIMEOUT.as_secs()).into());
        }
    };
    if stdout.len() > FFPROBE_STDOUT_LIMIT as usize || stderr.len() > FFPROBE_STDERR_LIMIT as usize
    {
        return Err(MediaAnalysisError::OutputTooLarge.into());
    }

    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr)
            .chars()
            .take(4_000)
            .collect();
        return Err(MediaAnalysisError::ProbeFailed {
            exit_code: status.code(),
            stderr,
        }
        .into());
    }

    parse_ffprobe_output(&stdout)
}

fn parse_ffprobe_output(stdout: &[u8]) -> Result<ProbeAnalysis> {
    let metadata_json = String::from_utf8_lossy(stdout).to_string();

    let parsed: FfprobeRoot =
        serde_json::from_slice(stdout).map_err(MediaAnalysisError::MalformedJson)?;

    let mut analysis = ProbeAnalysis {
        metadata: Some(metadata_json),
        ..Default::default()
    };

    let stream_tags = parsed
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("audio"))
        .and_then(|stream| stream.tags.clone())
        .unwrap_or_default();
    if let Some(format) = parsed.format {
        analysis.container = format.format_name;
        analysis.duration_secs = format
            .duration
            .as_deref()
            .and_then(|d| d.parse::<f64>().ok());
        analysis.duration = analysis.duration_secs.map(|d| d.round() as i32);
        analysis.tags = crate::services::release_match::tags_from_pairs(
            format
                .tags
                .iter()
                .flatten()
                .chain(stream_tags.iter())
                .map(|(key, value)| (key.as_str(), value.as_str())),
        );
        analysis.bitrate = format
            .bit_rate
            .as_deref()
            .and_then(|b| b.parse::<i64>().ok())
            .map(|b| b.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
    }

    for stream in parsed.streams {
        let tags_json = stream
            .tags
            .as_ref()
            .and_then(|t| serde_json::to_string(t).ok());
        let language = stream
            .tags
            .as_ref()
            .and_then(|t| get_tag_ci(t, "language"))
            .map(|s| s.to_string());
        let title = stream
            .tags
            .as_ref()
            .and_then(|t| get_tag_ci(t, "title"))
            .map(|s| s.to_string());
        let is_default = stream
            .disposition
            .as_ref()
            .and_then(|d| d.default)
            .unwrap_or(0)
            > 0;
        let is_forced = stream
            .disposition
            .as_ref()
            .and_then(|d| d.forced)
            .unwrap_or(0)
            > 0;
        let is_hearing_impaired = stream
            .disposition
            .as_ref()
            .and_then(|d| d.hearing_impaired)
            .unwrap_or(0)
            > 0;
        let is_commentary = stream
            .disposition
            .as_ref()
            .and_then(|d| d.commentary)
            .unwrap_or(0)
            > 0;

        match stream.codec_type.as_deref() {
            Some("video") => {
                // ffprobe flags EIA-608/708 captions with `closed_captions=1`, and names them in
                // the stream's description ("…, Closed Captions").
                if stream.closed_captions == Some(1)
                    || stream
                        .codec_long_name
                        .as_deref()
                        .is_some_and(|name| name.to_ascii_lowercase().contains("closed captions"))
                {
                    analysis.closed_captions = true;
                }
                let stream_hdr_type = stream.color_transfer.as_ref().and_then(|transfer| {
                    let t = transfer.to_ascii_lowercase();
                    if t.contains("smpte2084") {
                        Some("HDR10".to_string())
                    } else if t.contains("arib-std-b67") || t.contains("arib") {
                        Some("HLG".to_string())
                    } else {
                        None
                    }
                });

                if analysis.video_codec.is_none() {
                    analysis.video_codec = stream.codec_name.clone();
                    analysis.width = stream.width;
                    analysis.height = stream.height;
                    analysis.resolution = match (analysis.width, analysis.height) {
                        (Some(w), Some(h)) => Some(format!("{}x{}", w, h)),
                        _ => None,
                    };
                    if let Some(hdr_type) = stream_hdr_type.clone() {
                        analysis.is_hdr = true;
                        analysis.hdr_type = Some(hdr_type);
                    }
                }

                analysis.video_streams.push(VideoStreamAnalysis {
                    stream_index: stream.index.unwrap_or(analysis.video_streams.len() as i32),
                    codec: stream.codec_name.unwrap_or_else(|| "unknown".to_string()),
                    codec_long_name: stream.codec_long_name,
                    width: stream.width.unwrap_or(0),
                    height: stream.height.unwrap_or(0),
                    aspect_ratio: stream.display_aspect_ratio,
                    frame_rate: stream.r_frame_rate,
                    avg_frame_rate: stream.avg_frame_rate,
                    bitrate: parse_opt_i32(&stream.bit_rate),
                    pixel_format: stream.pix_fmt,
                    color_space: stream.color_space,
                    color_transfer: stream.color_transfer,
                    color_primaries: stream.color_primaries,
                    hdr_type: stream_hdr_type,
                    bit_depth: parse_bit_depth(stream.bits_per_sample, stream.bits_per_raw_sample),
                    language,
                    title,
                    is_default,
                    metadata: tags_json,
                });
            }
            Some("audio") => {
                if analysis.audio_codec.is_none() {
                    analysis.audio_codec = stream.codec_name.clone();
                    if let Some(layout) = stream.channel_layout.clone() {
                        analysis.audio_channels = Some(layout);
                    } else if let Some(ch) = stream.channels {
                        analysis.audio_channels = Some(ch.to_string());
                    }
                }

                analysis.audio_streams.push(AudioStreamAnalysis {
                    stream_index: stream.index.unwrap_or(analysis.audio_streams.len() as i32),
                    codec: stream.codec_name.unwrap_or_else(|| "unknown".to_string()),
                    codec_long_name: stream.codec_long_name,
                    channels: stream.channels.unwrap_or(0),
                    channel_layout: stream.channel_layout,
                    sample_rate: parse_opt_i32(&stream.sample_rate),
                    bitrate: parse_opt_i32(&stream.bit_rate),
                    bit_depth: parse_bit_depth(stream.bits_per_sample, stream.bits_per_raw_sample),
                    language,
                    title,
                    is_default,
                    is_commentary,
                    metadata: tags_json,
                });
            }
            Some("subtitle") => {
                analysis.subtitles.push(SubtitleAnalysis {
                    source_type: "embedded".to_string(),
                    stream_index: stream.index,
                    codec: stream.codec_name,
                    codec_long_name: stream.codec_long_name,
                    language,
                    title,
                    is_default,
                    is_forced,
                    is_hearing_impaired,
                    metadata: tags_json,
                });
            }
            _ => {}
        }
    }

    for (idx, chapter) in parsed.chapters.into_iter().enumerate() {
        let chapter_index = idx as i32;
        let start_secs = parse_opt_f64(&chapter.start_time).unwrap_or(0.0);
        let end_secs = parse_opt_f64(&chapter.end_time).unwrap_or(start_secs);
        let title = chapter
            .tags
            .as_ref()
            .and_then(|t| get_tag_ci(t, "title"))
            .map(|s| s.to_string());
        analysis.chapters.push(ChapterAnalysis {
            chapter_index,
            start_secs,
            end_secs,
            title,
        });
    }

    Ok(analysis)
}

fn parse_opt_i32(input: &Option<String>) -> Option<i32> {
    input
        .as_deref()
        .and_then(|s| s.parse::<i64>().ok())
        .map(|v| v.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

fn parse_opt_f64(input: &Option<String>) -> Option<f64> {
    input.as_deref().and_then(|s| s.parse::<f64>().ok())
}

fn parse_bit_depth(
    bits_per_sample: Option<i32>,
    bits_per_raw_sample: Option<String>,
) -> Option<i32> {
    bits_per_sample.or_else(|| parse_opt_i32(&bits_per_raw_sample))
}

fn get_tag_ci<'a>(tags: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    tags.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
}

/// Turns a title or name into one folder or file name component. This is the only place names
/// become path text, so a name has exactly one spelling on disk: typographic apostrophes
/// (`\u{2019}`, `\u{2018}`, `\u{02BC}`) become the plain `'`, because the same artist or title is
/// credited with either by different sources and must not get a folder for each.
fn sanitize_for_filename(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => out.push('_'),
            '\u{2018}' | '\u{2019}' | '\u{02BC}' => out.push('\''),
            _ => out.push(c),
        }
    }
    out.trim().trim_end_matches('.').to_string()
}

fn jaro_winkler(a: &str, b: &str) -> f64 {
    strsim::jaro_winkler(a, b)
}

fn replace_number_token(mut pattern: String, token: &str, value: i32) -> String {
    let Ok(re) = Regex::new(&format!(r"\{{{}(?::(\d+))?\}}", regex::escape(token))) else {
        return pattern;
    };
    pattern = re
        .replace_all(&pattern, |caps: &regex::Captures| {
            if let Some(width) = caps.get(1) {
                let w: usize = width.as_str().parse().unwrap_or(2);
                format!("{:0>width$}", value, width = w)
            } else {
                value.to_string()
            }
        })
        .to_string();
    pattern
}

fn apply_tv_naming_pattern(
    pattern: &str,
    show_name: &str,
    season: i32,
    episode: i32,
    episode_title: Option<&str>,
    ext: &str,
) -> PathBuf {
    let mut out = pattern.to_string();
    out = out.replace("{show}", &sanitize_for_filename(show_name));
    out = out.replace(
        "{title}",
        &sanitize_for_filename(
            episode_title
                .unwrap_or(&format!("Episode: episode {}", episode))
                .trim(),
        ),
    );
    out = out.replace("{ext}", ext.trim_start_matches('.'));
    out = replace_number_token(out, "season", season);
    out = replace_number_token(out, "episode", episode);
    PathBuf::from(out)
}

fn extract_quality_info(filename: &str) -> String {
    // Delegates to the shared release parser (`services::quality::scoring`)
    // instead of duplicating the resolution-tag regex/token list — see
    // `docs/tier1-features-plan.md` §2 "refactor... into this shared parser".
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);
    scoring::parse_release(stem)
        .resolution
        .unwrap_or_else(|| "unknown".to_string())
}

fn apply_movie_naming_pattern(
    pattern: &str,
    title: &str,
    year: Option<i32>,
    original_filename: &str,
    ext: &str,
) -> PathBuf {
    let mut out = pattern.to_string();
    let safe_title = sanitize_for_filename(title);
    out = out.replace("{title}", &safe_title);
    out = out.replace("{year}", &year.map(|y| y.to_string()).unwrap_or_default());
    out = out.replace("{quality}", &extract_quality_info(original_filename));
    out = out.replace(
        "{original}",
        Path::new(original_filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(original_filename),
    );
    out = out.replace("{ext}", ext.trim_start_matches('.'));
    PathBuf::from(out)
}

#[allow(clippy::too_many_arguments)]
/// Whether a library-less source row belongs to this download and its owner. Rows from before
/// sources were recorded only say they came from a torrent. The path being imported is on this
/// download's own file list and its owner has been checked, so such a row is theirs to take. A row
/// naming another download or another owner never is.
fn source_row_is_this_import(source: &serde_json::Value, owner_id: &str, info_hash: &str) -> bool {
    match (
        source["sourceId"].as_str(),
        source["sourceOwnerId"].as_str(),
    ) {
        (None, None) => source["sourceType"].as_str() == Some("torrent"),
        (id, owner) => id == Some(info_hash) && owner.is_none_or(|owner| owner == owner_id),
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_music_naming_pattern(
    pattern: &str,
    artist_name: &str,
    album_name: &str,
    album_year: Option<i32>,
    track_number: i32,
    disc_number: Option<i32>,
    track_title: &str,
    original_filename: &str,
    ext: &str,
) -> PathBuf {
    let mut out = pattern.to_string();
    out = out.replace("{artist}", &sanitize_for_filename(artist_name));
    out = out.replace("{album}", &sanitize_for_filename(album_name));
    out = out.replace(
        "{year}",
        &album_year.map(|y| y.to_string()).unwrap_or_default(),
    );
    out = out.replace("{title}", &sanitize_for_filename(track_title));
    out = out.replace(
        "{original}",
        Path::new(original_filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(original_filename),
    );
    out = out.replace("{ext}", ext.trim_start_matches('.'));
    out = replace_number_token(out, "track", track_number);
    out = replace_number_token(out, "disc", disc_number.unwrap_or(1));
    PathBuf::from(out)
}

/// Why a track has no folder to go to. A placeholder artist is never a folder: files put under
/// one are what a person later has to find and move by hand.
const ARTIST_UNKNOWN_REASON: &str =
    "The album's artist is not known, so there is no artist folder to organize into";

/// The artist folder for a track's album: the library's Artist record, sanitised like every other
/// name, or `None` when the record has no real name.
fn music_artist_folder(artist_name: Option<&str>) -> Option<String> {
    let name = artist_name?.trim();
    if name.is_empty() || name.eq_ignore_ascii_case("unknown artist") {
        return None;
    }
    let folder = sanitize_for_filename(name);
    (!folder.is_empty()).then_some(folder)
}

/// Everything a matched file's library path is made from. A file is planned from these alone, so
/// organizing one file and checking a whole library at once agree on where each file belongs.
#[derive(Debug, Clone)]
enum TargetFacts {
    Movie {
        title: String,
        year: Option<i32>,
    },
    Episode {
        show_name: String,
        season: i32,
        episode: i32,
        title: Option<String>,
    },
    Track {
        title: String,
        album_name: String,
        album_year: Option<i32>,
        track_number: i32,
        disc_number: Option<i32>,
        /// The most discs any track of the album is on.
        discs: i32,
        /// The name on the album's Artist record.
        artist_name: Option<String>,
    },
    Chapter {
        author_name: Option<String>,
        book_title: String,
        chapter_number: i32,
        chapter_title: Option<String>,
    },
}

impl TargetFacts {
    /// Where the file goes, relative to the library root, under the library's naming pattern.
    fn relative_path(
        &self,
        naming_pattern: &str,
        original_filename: &str,
        ext: &str,
    ) -> Result<PathBuf> {
        Ok(match self {
            Self::Movie { title, year } => {
                apply_movie_naming_pattern(naming_pattern, title, *year, original_filename, ext)
            }
            Self::Episode {
                show_name,
                season,
                episode,
                title,
            } => apply_tv_naming_pattern(
                naming_pattern,
                show_name,
                *season,
                *episode,
                title.as_deref(),
                ext,
            ),
            Self::Track {
                title,
                album_name,
                album_year,
                track_number,
                disc_number,
                discs,
                artist_name,
            } => {
                let Some(artist_folder) = music_artist_folder(artist_name.as_deref()) else {
                    anyhow::bail!(ARTIST_UNKNOWN_REASON);
                };
                apply_music_naming_pattern(
                    &music_rules::pattern_for_discs(naming_pattern, *discs),
                    &artist_folder,
                    album_name,
                    *album_year,
                    *track_number,
                    *disc_number,
                    title,
                    original_filename,
                    ext,
                )
            }
            Self::Chapter {
                author_name,
                book_title,
                chapter_number,
                chapter_title,
            } => apply_audiobook_naming_pattern(
                naming_pattern,
                author_name.as_deref().unwrap_or("Unknown Author"),
                book_title,
                *chapter_number,
                chapter_title.as_deref(),
                original_filename,
                ext,
            ),
        })
    }
}

fn apply_audiobook_naming_pattern(
    pattern: &str,
    author_name: &str,
    book_title: &str,
    chapter_number: i32,
    chapter_title: Option<&str>,
    original_filename: &str,
    ext: &str,
) -> PathBuf {
    let mut out = pattern.to_string();
    out = out.replace("{author}", &sanitize_for_filename(author_name));
    out = out.replace("{title}", &sanitize_for_filename(book_title));
    out = out.replace(
        "{chapter_title}",
        &sanitize_for_filename(
            chapter_title
                .unwrap_or(&format!("Chapter: chapter {}", chapter_number))
                .trim(),
        ),
    );
    out = out.replace("{series}", "");
    out = out.replace("{series_position}", "");
    out = out.replace("{narrator}", "");
    out = out.replace(
        "{original}",
        Path::new(original_filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(original_filename),
    );
    out = out.replace("{ext}", ext.trim_start_matches('.'));
    out = replace_number_token(out, "chapter", chapter_number);
    PathBuf::from(out)
}

fn derive_tv_dirs_from_pattern(
    library_path: &str,
    naming_pattern: &str,
    show_name: &str,
    season: i32,
    show_year: Option<i32>,
) -> Result<(PathBuf, PathBuf)> {
    validate_relative_library_path(Path::new(naming_pattern))?;
    let safe_show = show_year
        .map(|year| format!("{} ({})", sanitize_for_filename(show_name), year))
        .unwrap_or_else(|| sanitize_for_filename(show_name));

    let relative = apply_tv_naming_pattern(
        naming_pattern,
        &safe_show,
        season,
        1,
        Some("Episode"),
        "mkv",
    );
    validate_relative_library_path(&relative)?;
    let root = Path::new(library_path);
    let absolute = root.join(relative);
    checked_library_path(root, &absolute)?;

    let season_dir = absolute
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(library_path));
    let show_dir = season_dir
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(library_path));

    // A flat/custom layout can have fewer than two directory levels.
    let show_dir = if show_dir.starts_with(root) {
        show_dir
    } else {
        root.to_path_buf()
    };
    Ok((show_dir, season_dir))
}

/// Patterns and their expansions are always relative names, on every host platform.
/// Check both: metadata tokens can introduce traversal even in an otherwise safe template.
pub(crate) fn validate_relative_library_path(path: &Path) -> Result<()> {
    let text = path.to_string_lossy();
    if text.is_empty()
        || text.contains('\\')
        || text.contains('\0')
        || text.as_bytes().get(1) == Some(&b':')
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
        || !path
            .components()
            .any(|component| matches!(component, Component::Normal(_)))
    {
        anyhow::bail!(
            "Library naming paths must be relative and contain no parent traversal or platform prefixes"
        );
    }
    Ok(())
}

/// A library path as it is held for comparing and moving: resolved through symlinks on this
/// machine, or as written on a share once the share has answered that it is there.
async fn resolve_library_path(path: &str) -> Result<PathBuf> {
    if is_smb_uri(path) {
        library_storage().stat(path).await?;
        return Ok(PathBuf::from(path));
    }
    Ok(tokio::fs::canonicalize(path).await?)
}

/// Whether a path names a file or folder on a network share rather than on this machine.
pub(crate) fn is_remote_path(path: &Path) -> bool {
    path.to_str().is_some_and(is_smb_uri)
}

/// `create_library_directory`, for either kind of library.
async fn create_library_directory_in(root: &Path, directory: &Path) -> Result<PathBuf> {
    if is_remote_path(root) {
        checked_library_path(root, directory)?;
        library_storage()
            .create_dir_all(directory)
            .await
            .with_context(|| {
                format!("Cannot create library directory '{}'", directory.display())
            })?;
        return Ok(directory.to_path_buf());
    }
    let _own = own_write([&StoragePath::new(directory.to_string_lossy().into_owned())]);
    let (root, directory) = (root.to_path_buf(), directory.to_path_buf());
    tokio::task::spawn_blocking(move || create_library_directory(&root, &directory))
        .await
        .context("Library directory task failed")?
}

/// `deletable_media_file_path`, for either kind of library.
async fn deletable_media_file(root: &Path, file: &Path) -> Result<Option<PathBuf>> {
    if !is_remote_path(root) {
        let (root, file) = (root.to_path_buf(), file.to_path_buf());
        return tokio::task::spawn_blocking(move || deletable_media_file_path(&root, &file))
            .await?;
    }
    let parent = file.parent().context("Media file path has no parent")?;
    checked_library_path(root, parent)?;
    match library_storage().stat(file).await {
        Ok(meta) if meta.is_dir() => anyhow::bail!("Media file path is a directory, not a file"),
        Ok(_) => Ok(Some(file.to_path_buf())),
        Err(error) if error.is_not_found() => Ok(None),
        Err(error) => Err(error).context("Cannot inspect the media file"),
    }
}

/// `remove_media_file`, for either kind of library.
async fn remove_media_file_in(root: &Path, target: &Path) -> Result<()> {
    let _own = own_write([&StoragePath::new(target.to_string_lossy().into_owned())]);
    if !is_remote_path(root) {
        let (root, target) = (root.to_path_buf(), target.to_path_buf());
        return tokio::task::spawn_blocking(move || remove_media_file(&root, &target)).await?;
    }
    match library_storage().remove_file(target).await {
        Ok(()) => {}
        Err(error) if error.is_not_found() => return Ok(()),
        Err(error) => return Err(error.into()),
    }
    if let Some(parent) = target.parent()
        && parent != root
        && parent.starts_with(root)
    {
        // Refused by the share while anything is left in the folder (subtitles, artwork).
        let _ = library_storage().remove_dir(parent).await;
    }
    Ok(())
}

/// Resolve existing ancestors, including symlinks, without creating anything. Missing
/// suffixes are allowed for planning; dangling links and escapes fail closed.
pub(crate) fn checked_library_path(root: &Path, destination: &Path) -> Result<PathBuf> {
    let relative = destination
        .strip_prefix(root)
        .context("Destination is outside the library root")?;
    if !relative.as_os_str().is_empty() {
        validate_relative_library_path(relative)?;
    }
    // A share has no symlinks to resolve and no working directory to canonicalize against: the
    // containment that counts is the name check above, and `..` is refused there.
    if is_remote_path(root) {
        return Ok(destination.to_path_buf());
    }
    let canonical_root = std::fs::canonicalize(root).context("Library root is unavailable")?;
    if !canonical_root.is_dir() {
        anyhow::bail!("Library root is not a directory");
    }
    let mut resolved = canonical_root.clone();
    for component in relative.components() {
        if component == Component::CurDir {
            continue;
        }
        resolved.push(component.as_os_str());
        match std::fs::symlink_metadata(&resolved) {
            Ok(_) => {
                resolved = std::fs::canonicalize(&resolved)
                    .context("Destination contains an unresolved symlink")?;
                if !resolved.starts_with(&canonical_root) {
                    anyhow::bail!("Destination symlink escapes the library root");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("Cannot inspect library destination"),
        }
    }
    Ok(resolved)
}

/// The directory entry to unlink for a media file, or `None` when it is already gone.
///
/// Only the parent is resolved, so a symlink in the library loses the link and never what it
/// points at. The parent's real location must be inside the library root.
fn deletable_media_file_path(root: &Path, file: &Path) -> Result<Option<PathBuf>> {
    let name = file
        .file_name()
        .context("Media file path has no file name")?;
    let parent = file.parent().context("Media file path has no parent")?;
    let target = checked_library_path(root, parent)?.join(name);
    match std::fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.is_dir() => {
            anyhow::bail!("Media file path is a directory, not a file")
        }
        Ok(_) => Ok(Some(target)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).context("Cannot inspect the media file"),
    }
}

/// Unlink a checked media file, then its folder if that leaves it empty. The library root stays,
/// and `remove_dir` refuses a folder with anything left in it (subtitles, artwork).
fn remove_media_file(root: &Path, target: &Path) -> Result<()> {
    match std::fs::remove_file(target) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    }
    if let (Ok(root), Some(parent)) = (std::fs::canonicalize(root), target.parent())
        && parent != root
        && parent.starts_with(&root)
    {
        let _ = std::fs::remove_dir(parent);
    }
    Ok(())
}

/// Whether a folder or file name is hidden. Hidden folders are never removed as empty: the
/// library's own `.librarian-trash` is one, and sync tools keep markers in others.
fn is_hidden_name(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

/// Remove `start`, then each parent in turn, while they are empty, stopping before the library
/// root, a hidden folder or a folder in `protected`. Returns the folders removed.
fn remove_empty_folders_upward(
    root: &Path,
    start: &Path,
    protected: &HashSet<PathBuf>,
) -> Vec<PathBuf> {
    let mut removed = Vec::new();
    let mut current = start.to_path_buf();
    while current != root
        && current.starts_with(root)
        && !protected.contains(&current)
        && !current.file_name().is_some_and(is_hidden_name)
    {
        match std::fs::remove_dir(&current) {
            Ok(()) => removed.push(current.clone()),
            // Already gone: its parent may still be empty.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => break,
        }
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent.to_path_buf();
    }
    removed
}

/// `remove_empty_folders_upward` on a share: the share refuses to remove a folder that has
/// anything in it, so a file that landed there a moment ago is never lost.
async fn remove_empty_folders_upward_remote(
    root: &Path,
    start: &Path,
    protected: &HashSet<PathBuf>,
) -> Vec<PathBuf> {
    let mut removed = Vec::new();
    let mut current = start.to_path_buf();
    while current != root
        && current.starts_with(root)
        && !protected.contains(&current)
        && !current.file_name().is_some_and(is_hidden_name)
    {
        match library_storage().remove_dir(&current).await {
            Ok(()) => removed.push(current.clone()),
            // Already gone: its parent may still be empty.
            Err(error) if error.is_not_found() => {}
            Err(_) => break,
        }
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent.to_path_buf();
    }
    removed
}

fn create_library_directory(root: &Path, directory: &Path) -> Result<PathBuf> {
    checked_library_path(root, directory)?;
    let relative = directory.strip_prefix(root)?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        let checked = checked_library_path(root, &current)?;
        match std::fs::create_dir(&checked) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error).context("Cannot create library directory"),
        }
        if !checked_library_path(root, &current)?.is_dir() {
            anyhow::bail!("Library destination parent is not a directory");
        }
    }
    checked_library_path(root, directory)
}

/// All active library placement paths pass through this final containment boundary.
/// The raw no-clobber primitive remains usable for its isolated filesystem tests.
async fn place_in_library(
    root: PathBuf,
    source: PathBuf,
    target: PathBuf,
    remove_source: bool,
    source_is_library: bool,
) -> Result<NoClobberOutcome> {
    // Imports and organize renames are Librarian's own: a folder watch must not scan them (Q116).
    let _own = own_write([
        &StoragePath::new(source.to_string_lossy().into_owned()),
        &StoragePath::new(target.to_string_lossy().into_owned()),
    ]);
    if is_remote_path(&root) || is_remote_path(&target) {
        return place_in_remote_library(root, source, target, remove_source, source_is_library)
            .await;
    }
    if is_remote_path(&source) {
        return place_from_share_in_local_library(root, source, target, remove_source).await;
    }
    tokio::task::spawn_blocking(move || {
        if source_is_library {
            checked_library_path(&root, &source)?;
            if std::fs::symlink_metadata(&source)?.file_type().is_symlink() {
                anyhow::bail!("Cannot organize a symlinked media file");
            }
        }
        checked_library_path(&root, &target)?;
        let parent = target
            .parent()
            .context("Library destination has no parent")?;
        // A scan tidying empty folders can remove the new folder between making it and linking
        // into it. The source is still there then, so make the folder again and retry.
        let mut attempts = 0;
        loop {
            create_library_directory(&root, parent)?;
            let target = checked_library_path(&root, &target)?;
            let source = if source_is_library {
                checked_library_path(&root, &source)?
            } else {
                source.clone()
            };
            match place_no_clobber_blocking(&source, &target, remove_source) {
                Err(error)
                    if attempts < 3
                        && source.exists()
                        && error.chain().any(|cause| {
                            cause
                                .downcast_ref::<std::io::Error>()
                                .is_some_and(|io| io.kind() == std::io::ErrorKind::NotFound)
                        }) =>
                {
                    attempts += 1;
                }
                other => return other,
            }
        }
    })
    .await
    .context("Contained library placement task failed")?
}

/// `place_in_library` for a download on a share going into a library on this server: the file
/// is copied down (to a temporary name, then linked into place without replacing anything), and
/// the download is never touched, so its torrent keeps seeding.
async fn place_from_share_in_local_library(
    root: PathBuf,
    source: PathBuf,
    target: PathBuf,
    remove_source: bool,
) -> Result<NoClobberOutcome> {
    if remove_source {
        anyhow::bail!(
            "A file on a share is copied into a library on this server, never moved: '{}'",
            source.display()
        );
    }
    let mut attempts = 0;
    loop {
        let (root_for_dir, target_for_dir) = (root.clone(), target.clone());
        tokio::task::spawn_blocking(move || -> Result<()> {
            checked_library_path(&root_for_dir, &target_for_dir)?;
            let parent = target_for_dir
                .parent()
                .context("Library destination has no parent")?;
            create_library_directory(&root_for_dir, parent)?;
            Ok(())
        })
        .await
        .context("Library folder task failed")??;
        let placed = library_storage()
            .copy(&source, &target, PlaceMode::NoClobber)
            .await
            .map_err(anyhow::Error::from);
        match placed {
            // The new folder was tidied away between making it and copying into it.
            Err(error)
                if attempts < 3
                    && error
                        .downcast_ref::<crate::services::library_storage::StorageError>()
                        .is_some_and(|error| error.is_not_found())
                    && library_storage().exists(&source).await.unwrap_or(false) =>
            {
                attempts += 1;
            }
            other => return other,
        }
    }
}

/// `place_in_library` for a library on a share. There are no hard links there: a file arriving
/// from a download is copied (the download is left alone, so a torrent keeps seeding), and a file
/// already on the share is renamed, both without replacing anything that is there.
async fn place_in_remote_library(
    root: PathBuf,
    source: PathBuf,
    target: PathBuf,
    remove_source: bool,
    source_is_library: bool,
) -> Result<NoClobberOutcome> {
    if !is_remote_path(&root) {
        anyhow::bail!(
            "A file cannot be placed on a share inside the local library '{}'",
            root.display()
        );
    }
    checked_library_path(&root, &target)?;
    let parent = target
        .parent()
        .context("Library destination has no parent")?;
    let storage = library_storage();
    let mut attempts = 0;
    loop {
        create_library_directory_in(&root, parent).await?;
        let outcome = if is_remote_path(&source) {
            if remove_source {
                // Organizing: a rename on the library's own share.
                checked_library_path(&root, &source)?;
                storage
                    .rename(&source, &target, PlaceMode::NoClobber)
                    .await
                    .map_err(anyhow::Error::from)
            } else {
                // A download on a share: a server-side copy when it is the library's share,
                // streamed through Librarian from another one. The download is left alone.
                if source_is_library {
                    checked_library_path(&root, &source)?;
                }
                storage
                    .copy(&source, &target, PlaceMode::NoClobber)
                    .await
                    .map_err(anyhow::Error::from)
            }
        } else {
            let placed = storage
                .put_from_local(&source, &target, PlaceMode::NoClobber)
                .await
                .map_err(anyhow::Error::from);
            if matches!(placed, Ok(NoClobberOutcome::Placed)) && remove_source {
                tokio::fs::remove_file(&source).await.with_context(|| {
                    format!(
                        "failed to remove source file after move: {}",
                        source.display()
                    )
                })?;
            }
            placed
        };
        match outcome {
            // A scan tidying empty folders can remove the new folder between making it and
            // placing into it; the source is still there, so make it again and retry.
            Err(error)
                if attempts < 3
                    && error
                        .downcast_ref::<crate::services::library_storage::StorageError>()
                        .is_some_and(|error| error.is_not_found())
                    && library_storage().exists(&source).await.unwrap_or(false) =>
            {
                attempts += 1;
            }
            other => return other,
        }
    }
}

async fn paths_refer_to_same_file(left: &Path, right: &Path) -> bool {
    let storage = library_storage();
    let (Ok(left_meta), Ok(right_meta)) = (storage.stat(left).await, storage.stat(right).await)
    else {
        return false;
    };
    match (left_meta.identity, right_meta.identity) {
        (Some(left), Some(right)) => left == right,
        // No identity (a share, or a platform without one): the same file is the same path,
        // or two local paths that resolve to one.
        _ => match (
            tokio::fs::canonicalize(left).await.ok(),
            tokio::fs::canonicalize(right).await.ok(),
        ) {
            (Some(left), Some(right)) => left == right,
            _ => left == right,
        },
    }
}

// Outcome of `place_in_library`: `Placed`, or `TargetExists` (nothing was touched, so the caller
// treats it as a conflict). The no-clobber primitive itself, `place_no_clobber_blocking`, is the
// local storage's (`services/library_storage/local.rs`); its reasoning is documented there.
use crate::services::library_storage::PlaceOutcome as NoClobberOutcome;

/// Place `src` at `dst` without ever clobbering an existing `dst`; see
/// [`place_no_clobber_blocking`] for how.
///
/// This replaces the `exists()`-then-`rename()`/`copy()` pattern that races under
/// concurrent organize/import (audit #4 — TOCTOU: another task can create `dst`
/// between the check and the write, and `rename`/`copy` silently replace it,
/// permanently destroying the previous file). `std::fs::hard_link` is used instead:
/// it is a single kernel syscall that atomically fails with `AlreadyExists` if `dst`
/// is already present, giving true no-clobber semantics on same-filesystem moves.
///
/// If `remove_source_on_success` is `true`, `src` is removed once `dst` is in place
/// (a "move"); if `false`, `src` is left untouched (a "copy" — e.g. so a seeding
/// torrent source file is never modified).
///
/// Falls back to copy-to-temp-then-no-clobber-hardlink when `src` and `dst` are on
/// different filesystems (`hard_link` cannot cross devices). The temp file is
/// created next to `dst` so the final placement hardlink is same-filesystem and
/// atomic; the temp file is always cleaned up, and `src` is left untouched unless
/// the placement fully succeeds.
///
/// `dst`'s parent directory must already exist. Runs on a blocking thread pool
/// since these are synchronous syscalls.
#[cfg(test)]
async fn no_clobber_place(
    src: PathBuf,
    dst: PathBuf,
    remove_source_on_success: bool,
) -> Result<NoClobberOutcome> {
    tokio::task::spawn_blocking(move || {
        place_no_clobber_blocking(&src, &dst, remove_source_on_success)
    })
    .await
    .context("no-clobber file placement task panicked")?
}

#[cfg(test)]
mod tests {

    use super::{
        ImportDiagnostics, ImportFile, LinkedTrackRow, MatchCandidate, RecordPlace,
        SkippedImportFile, SourceFileImport, TORRENT_LINK_MATCH_TYPE,
        TORRENT_LINK_PARSED_MATCH_TYPE, TorrentImportLink, UnmatchedImportFile,
    };
    use crate::services::library_storage::StoragePath;

    fn import_file(path: &str, size_mb: i64) -> ImportFile {
        ImportFile {
            file: SourceFileImport {
                source_path: path.to_string(),
                relative_path: path.to_string(),
                file_size: size_mb * 1024 * 1024,
                downloaded_bytes: size_mb * 1024 * 1024,
                file_index: None,
            },
        }
    }

    fn kept_paths(files: Vec<ImportFile>) -> (Vec<String>, Vec<String>) {
        let (kept, skipped) = LibraryScanService::filter_import_candidates(files);
        (
            kept.into_iter().map(|item| item.file.source_path).collect(),
            skipped.into_iter().map(|item| item.source_path).collect(),
        )
    }

    #[test]
    fn import_filter_drops_samples_extras_and_undersized_video() {
        let (kept, skipped) = kept_paths(vec![
            import_file("/dl/Show.S01E01/Show.S01E01.1080p.mkv", 4000),
            import_file("/dl/Show.S01E01/Sample/show-sample.mkv", 40),
            import_file("/dl/Show.S01E01/sample.mkv", 30),
            import_file("/dl/Show.S01E01/Extras/behind-the-scenes.mkv", 900),
            import_file("/dl/Show.S01E01/teaser.mkv", 20),
        ]);
        assert_eq!(kept, vec!["/dl/Show.S01E01/Show.S01E01.1080p.mkv"]);
        assert_eq!(skipped.len(), 4);
    }

    #[test]
    fn import_filter_keeps_small_format_releases() {
        // Every video in the release is small, so the release itself is
        // small-format and nothing is dropped for size.
        let (kept, skipped) = kept_paths(vec![
            import_file("/dl/Webisodes/Web.S01E01.mkv", 20),
            import_file("/dl/Webisodes/Web.S01E02.mkv", 22),
        ]);
        assert_eq!(kept.len(), 2);
        assert!(skipped.is_empty());
    }

    #[test]
    fn import_filter_keeps_audio_below_the_video_floor() {
        let (kept, skipped) = kept_paths(vec![
            import_file("/dl/Album/01 - One.flac", 30),
            import_file("/dl/Album/02 - Two.flac", 28),
        ]);
        assert_eq!(kept.len(), 2, "audio has no 50 MB floor");
        assert!(skipped.is_empty());
    }

    #[test]
    fn natural_sort_key_orders_numbers_numerically() {
        let mut names = vec!["part10.mp3", "part2.mp3", "part1.mp3"];
        names.sort_by_key(|name| LibraryScanService::natural_sort_key(name));
        assert_eq!(names, vec!["part1.mp3", "part2.mp3", "part10.mp3"]);
    }

    #[test]
    fn season_is_parsed_from_release_names() {
        assert_eq!(
            LibraryScanService::parse_season_from_release_name("The.Show.S03.1080p.WEB-DL"),
            Some(3)
        );
        assert_eq!(
            LibraryScanService::parse_season_from_release_name("The Show Season 12 Complete"),
            Some(12)
        );
        assert_eq!(
            LibraryScanService::parse_season_from_release_name("Some Movie 2019 1080p"),
            None
        );
    }

    #[test]
    fn album_track_choice_prefers_track_number_then_title() {
        let tracks = vec![
            LinkedTrackRow {
                seconds: None,
                id: "t1".to_string(),
                title: "Welcome to the Jungle".to_string(),
                track_number: Some(1),
                disc_number: Some(1),
                held_size: None,
                held_path: None,
            },
            LinkedTrackRow {
                seconds: None,
                id: "t2".to_string(),
                title: "It's So Easy".to_string(),
                track_number: Some(2),
                disc_number: Some(1),
                held_size: None,
                held_path: None,
            },
        ];
        let used = HashSet::new();
        assert_eq!(
            LibraryScanService::choose_album_track(&tracks, &used, Some(2), None)
                .map(|track| track.id.as_str()),
            Some("t2")
        );
        assert_eq!(
            LibraryScanService::choose_album_track(&tracks, &used, None, Some("its so easy"))
                .map(|track| track.id.as_str()),
            Some("t2")
        );
        assert!(
            LibraryScanService::choose_album_track(&tracks, &used, None, Some("unrelated song"))
                .is_none()
        );
        let used_all: HashSet<String> = ["t1".to_string(), "t2".to_string()].into_iter().collect();
        assert!(
            LibraryScanService::choose_album_track(&tracks, &used_all, Some(2), None).is_none()
        );
    }

    #[test]
    fn import_match_type_is_auditable() {
        let linked = MatchCandidate {
            target_type: "Movie".to_string(),
            target_id: "m1".to_string(),
            target_name: None,
            score: 1.0,
            reason: Some(format!("{TORRENT_LINK_MATCH_TYPE}: grabbed for 'Dune'")),
            wanted: None,
        };
        assert_eq!(
            LibraryScanService::import_match_type(&linked),
            "torrent_link"
        );

        let parsed = MatchCandidate {
            reason: Some(format!("{TORRENT_LINK_PARSED_MATCH_TYPE}: parsed S01E02")),
            ..linked.clone()
        };
        assert_eq!(
            LibraryScanService::import_match_type(&parsed),
            "torrent_link_parsed"
        );

        let ollama = MatchCandidate {
            reason: Some("matched via ollama hint".to_string()),
            ..linked.clone()
        };
        assert_eq!(LibraryScanService::import_match_type(&ollama), "ollama");

        let plain = MatchCandidate {
            reason: None,
            ..linked
        };
        assert_eq!(LibraryScanService::import_match_type(&plain), "auto");
    }

    #[test]
    fn torrent_link_detects_recorded_targets() {
        assert!(!TorrentImportLink::default().has_target());
        assert!(
            TorrentImportLink {
                show_id: Some("s1".to_string()),
                ..Default::default()
            }
            .has_target()
        );
    }

    #[test]
    fn unmatched_torrents_get_an_explanatory_post_process_error() {
        let diagnostics = ImportDiagnostics {
            unmatched: vec![
                UnmatchedImportFile {
                    source_path: "/dl/pack/a.mkv".to_string(),
                    reason: "'The Show' has no episode S02E11".to_string(),
                },
                UnmatchedImportFile {
                    source_path: "/dl/pack/b.mkv".to_string(),
                    reason: "'The Show' has no episode S02E12".to_string(),
                },
            ],
            target_label: Some("'The Show'".to_string()),
            ..Default::default()
        };
        let message = LibraryScanService::summarize_import_outcome(&diagnostics, 0, 0).unwrap();
        assert!(message.contains("2 file(s) did not match"), "{message}");
        assert!(message.contains("'The Show'"), "{message}");
        assert!(message.contains("a.mkv"), "{message}");
        assert!(message.contains("no episode S02E11"), "{message}");
    }

    #[test]
    fn archive_failures_are_reported_on_the_torrent() {
        let diagnostics = ImportDiagnostics {
            archive_failures: vec!["'release.rar' is password protected (RAR archive)".to_string()],
            ..Default::default()
        };
        let message = LibraryScanService::summarize_import_outcome(&diagnostics, 0, 1).unwrap();
        assert!(
            message.starts_with("archive extraction failed:"),
            "{message}"
        );
        assert!(message.contains("password protected"), "{message}");
        assert!(message.contains("1 file(s) failed"), "{message}");
    }

    #[test]
    fn torrents_with_no_media_explain_what_they_contained() {
        let diagnostics = ImportDiagnostics {
            ignored_extensions: [".nfo".to_string(), ".srt".to_string()]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let message = LibraryScanService::summarize_import_outcome(&diagnostics, 0, 0).unwrap();
        assert!(message.contains("no supported media files"), "{message}");
        assert!(message.contains(".nfo"), "{message}");
    }

    #[test]
    fn skipped_only_torrents_say_files_were_filtered() {
        let diagnostics = ImportDiagnostics {
            skipped: vec![SkippedImportFile {
                source_path: "/dl/x/sample.mkv".to_string(),
                reason: "sample".to_string(),
            }],
            ..Default::default()
        };
        let message = LibraryScanService::summarize_import_outcome(&diagnostics, 0, 0).unwrap();
        assert!(
            message.contains("skipped as sample/extras/undersized"),
            "{message}"
        );
        assert!(message.contains("sample.mkv"), "{message}");
    }

    #[test]
    fn clean_imports_do_not_surface_informational_notes() {
        let diagnostics = ImportDiagnostics {
            notes: vec!["audiobook 'X': 3 of 12 chapters have a file".to_string()],
            ..Default::default()
        };
        assert!(LibraryScanService::summarize_import_outcome(&diagnostics, 3, 0).is_none());
        let message = LibraryScanService::summarize_import_outcome(&diagnostics, 0, 0).unwrap();
        assert!(message.contains("3 of 12 chapters"), "{message}");
    }

    #[test]
    fn successful_imports_have_no_post_process_error() {
        let diagnostics = ImportDiagnostics::default();
        assert!(LibraryScanService::summarize_import_outcome(&diagnostics, 3, 0).is_none());
    }

    #[test]
    fn extraction_staging_root_is_bounded_to_the_download_directory() {
        let root = LibraryScanService::extraction_staging_root(
            std::path::Path::new("/downloads/Some.Release/release.rar"),
            "abcd1234",
        );
        assert_eq!(
            root,
            std::path::PathBuf::from("/downloads/Some.Release/.librarian-extract/abcd1234")
        );
    }

    #[test]
    fn archives_are_partitioned_away_from_media_and_junk() {
        let partitioned = LibraryScanService::partition_import_files(vec![
            SourceFileImport {
                source_path: "/dl/r/movie.mkv".to_string(),
                relative_path: "movie.mkv".to_string(),
                file_size: 1,
                downloaded_bytes: 1,
                file_index: Some(0),
            },
            SourceFileImport {
                source_path: "/dl/r/movie.rar".to_string(),
                relative_path: "movie.rar".to_string(),
                file_size: 1,
                downloaded_bytes: 1,
                file_index: Some(1),
            },
            SourceFileImport {
                source_path: "/dl/r/movie.r00".to_string(),
                relative_path: "movie.r00".to_string(),
                file_size: 1,
                downloaded_bytes: 1,
                file_index: Some(2),
            },
            SourceFileImport {
                source_path: "/dl/r/movie.nfo".to_string(),
                relative_path: "movie.nfo".to_string(),
                file_size: 1,
                downloaded_bytes: 1,
                file_index: Some(3),
            },
        ]);
        assert_eq!(partitioned.media.len(), 1);
        assert_eq!(partitioned.archives.len(), 2);
        assert!(partitioned.ignored_extensions.contains(".nfo"));
    }
    #[test]
    fn ffprobe_closed_captions_are_noticed() {
        let flagged = super::parse_ffprobe_output(br#"{
            "streams": [{"index": 0, "codec_type": "video", "codec_name": "h264", "closed_captions": 1}],
            "chapters": []
        }"#).unwrap();
        assert!(flagged.closed_captions);
        let described = super::parse_ffprobe_output(
            br#"{
            "streams": [{"index": 0, "codec_type": "video", "codec_name": "mpeg2video",
                "codec_long_name": "MPEG-2 video, Closed Captions"}],
            "chapters": []
        }"#,
        )
        .unwrap();
        assert!(described.closed_captions);
        let plain = super::parse_ffprobe_output(br#"{
            "streams": [{"index": 0, "codec_type": "video", "codec_name": "h264", "closed_captions": 0}],
            "chapters": []
        }"#).unwrap();
        assert!(!plain.closed_captions);
    }

    #[test]
    fn notification_audit_ffprobe_accepts_large_opaque_chapter_ids() {
        let analysis = super::parse_ffprobe_output(br#"{
            "streams": [],
            "chapters": [
                {"id": -7101387783505086327, "start_time": "0.000000", "end_time": "122.500000", "tags": {"title": "Opening"}},
                {"id": 18446744073709551615, "start_time": "122.500000", "end_time": "245.000000"},
                {"start_time": "245.000000", "end_time": "300.000000"}
            ]
        }"#).unwrap();
        assert_eq!(
            analysis
                .chapters
                .iter()
                .map(|chapter| chapter.chapter_index)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(analysis.chapters[0].title.as_deref(), Some("Opening"));
        assert_eq!(analysis.chapters[1].start_secs, 122.5);
        assert_eq!(analysis.chapters[2].end_secs, 300.0);
        let error = super::parse_ffprobe_output(b"{broken").unwrap_err();
        assert_eq!(
            super::classify_analysis_error(&error).0,
            "FFPROBE_MALFORMED_OUTPUT"
        );
        assert!(error.to_string().contains("line 1"));
    }

    use super::{
        DuplicateIssueDetails, DuplicateKeep, LibraryScanService, LibraryScanServiceConfig,
        MatchWantedPolicy, NO_LONGER_DUPLICATE, NoClobberOutcome, OrganizeResult,
        ProviderFallbackSummary, no_clobber_place, supervise_worker,
    };
    use quick_xml::{Reader, events::Event};
    use regex::Regex;
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn panicking_worker_stops_after_bounded_restarts() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let worker_attempts = Arc::clone(&attempts);

        supervise_worker(
            "panic-fixture",
            CancellationToken::new(),
            move |_worker_cancel| {
                let worker_attempts = Arc::clone(&worker_attempts);
                async move {
                    worker_attempts.fetch_add(1, Ordering::SeqCst);
                    panic!("injected worker panic");
                }
            },
        )
        .await;

        assert_eq!(attempts.load(Ordering::SeqCst), super::WORKER_RESTART_LIMIT);
    }

    /// Manual scalability harness. Run with:
    /// `LIBRARIAN_SCAN_BENCH_FILES=100000 cargo test synthetic_large_tree_discovery -- --ignored --nocapture`
    #[tokio::test]
    #[ignore = "creates a synthetic 10k/100k-file tree for manual qualification"]
    async fn synthetic_large_tree_discovery() {
        let file_count = std::env::var("LIBRARIAN_SCAN_BENCH_FILES")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(10_000);
        assert!(
            matches!(file_count, 10_000 | 100_000),
            "benchmark accepts only the planned 10k or 100k fixture sizes"
        );

        let root = tempfile::tempdir().expect("benchmark tempdir should create");
        for index in 0..file_count {
            let directory = root.path().join(format!("bucket-{:04}", index / 1_000));
            fs::create_dir_all(&directory).expect("benchmark directory should create");
            fs::write(directory.join(format!("movie-{index:06}.mkv")), [])
                .expect("benchmark file should create");
        }

        let started = std::time::Instant::now();
        let (mut discovered, handle) = LibraryScanService::discover_media_files(
            root.path().to_path_buf(),
            LibraryScanService::extensions_for_library("movies"),
            CancellationToken::new(),
        );
        let mut discovered_count = 0usize;
        while let Some(result) = discovered.recv().await {
            result.expect("benchmark traversal should not fail");
            discovered_count += 1;
        }
        handle
            .await
            .expect("benchmark traversal worker should join");

        let elapsed = started.elapsed();
        eprintln!(
            "synthetic scan benchmark: files={file_count}, elapsed_ms={}, files_per_second={:.2}",
            elapsed.as_millis(),
            file_count as f64 / elapsed.as_secs_f64()
        );
        assert_eq!(discovered_count, file_count);
    }

    #[test]
    fn provider_fallback_summary_preserves_issue_counts() {
        let mut summary = ProviderFallbackSummary {
            pending_files: 4,
            ..Default::default()
        };
        summary.merge_outcome(ProviderFallbackSummary {
            matched_files: 2,
            unmatched_files: 2,
            error_files: 1,
            ..Default::default()
        });

        assert_eq!(summary.pending_files, 4);
        assert_eq!(summary.matched_files, 2);
        assert_eq!(summary.unmatched_files, 2);
        assert_eq!(summary.error_files, 1);
        assert!(summary.has_issues());
    }

    #[test]
    fn auto_organize_guard_error_never_allows_a_move() {
        let guard_result: std::result::Result<Option<String>, &str> =
            Err("simulated safety lookup failure");
        assert!(
            !LibraryScanService::auto_organize_guard_allows_move(&guard_result),
            "a safety-check error must fail closed"
        );
    }

    #[test]
    fn auto_organize_failure_is_not_treated_as_success() {
        let operation_error: anyhow::Result<OrganizeResult> =
            Err(anyhow::anyhow!("simulated organization failure"));
        let incomplete = Ok(OrganizeResult {
            success: false,
            reason: Some("Source file does not exist".to_string()),
            ..Default::default()
        });
        let completed = Ok(OrganizeResult {
            success: true,
            ..Default::default()
        });

        assert!(!LibraryScanService::auto_organize_result_is_success(
            &operation_error
        ));
        assert!(!LibraryScanService::auto_organize_result_is_success(
            &incomplete
        ));
        assert!(LibraryScanService::auto_organize_result_is_success(
            &completed
        ));
    }

    fn rss_titles_from_xml(xml: &str) -> anyhow::Result<Vec<String>> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();
        let mut in_item = false;
        let mut in_title = false;
        let mut titles = Vec::new();

        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(event) => match event.name().as_ref() {
                    "item" => in_item = true,
                    "title" if in_item => in_title = true,
                    _ => {}
                },
                Event::Text(event) if in_item && in_title => {
                    titles.push(quick_xml::escape::unescape(&event)?.into_owned());
                }
                Event::End(event) => match event.name().as_ref() {
                    "title" if in_item => in_title = false,
                    "item" => {
                        in_item = false;
                        in_title = false;
                    }
                    _ => {}
                },
                Event::Eof => {
                    if in_item || in_title {
                        anyhow::bail!("RSS XML ended inside an item/title element");
                    }
                    break;
                }
                _ => {}
            }
            buf.clear();
        }

        Ok(titles)
    }

    #[test]
    fn parse_movie_hint_extracts_title_and_year() {
        let hint = LibraryScanService::parse_movie_hint(
            "/data/Movies/The.Hunt.for.Red.October.1990.1080p.BluRay.x264-GROUP.mkv",
        );
        assert_eq!(hint.title.as_deref(), Some("The Hunt for Red October"));
        assert_eq!(hint.year, Some(1990));
    }

    #[test]
    fn parse_episode_hint_extracts_show_and_numbers() {
        let hint = LibraryScanService::parse_episode_hint(
            "/data/TV/Chicago.Fire.S14E08.1080p.WEB.h264-ETHEL.mkv",
        );
        assert_eq!(hint.show_name.as_deref(), Some("Chicago Fire"));
        assert_eq!(hint.season, Some(14));
        assert_eq!(hint.episode, Some(8));
    }

    #[test]
    fn parse_episode_hint_uses_show_folder_when_filename_has_only_episode_numbers() {
        let hint = LibraryScanService::parse_episode_hint(
            "/data/TV/Chicago Fire/Season 14/S14E08.1080p.WEB.h264-ETHEL.mkv",
        );
        assert_eq!(hint.show_name.as_deref(), Some("Chicago Fire"));
        assert_eq!(hint.season, Some(14));
        assert_eq!(hint.episode, Some(8));
    }

    #[test]
    fn parse_episode_hint_uses_parent_folder_without_season_folder() {
        let hint = LibraryScanService::parse_episode_hint(
            "/data/TV/Chicago Fire/S14E08.1080p.WEB.h264-ETHEL.mkv",
        );
        assert_eq!(hint.show_name.as_deref(), Some("Chicago Fire"));
        assert_eq!(hint.season, Some(14));
        assert_eq!(hint.episode, Some(8));
    }

    #[test]
    fn parse_episode_hint_handles_show_folder_year_and_episode_title() {
        let hint = LibraryScanService::parse_episode_hint(
            "/data/media/TV Shows/A Knight of the Seven Kingdoms (2026)/Season 01/A Knight of the Seven Kingdoms - S01E01 - The Hedge Knight.mkv",
        );
        assert_eq!(
            hint.show_name.as_deref(),
            Some("A Knight of the Seven Kingdoms")
        );
        assert_eq!(hint.season, Some(1));
        assert_eq!(hint.episode, Some(1));
    }

    #[test]
    fn episode_show_score_rejects_completely_different_show() {
        // audit #3: a file whose S/E numbers happen to match a wanted episode of
        // some other show must NOT reach the ~0.70 auto-link threshold just
        // because the numbers line up.
        let hint = LibraryScanService::parse_episode_hint("/data/TV/The.Bear.S01E05.mkv");
        let normalized = hint
            .show_name
            .as_deref()
            .map(LibraryScanService::normalize_for_match)
            .expect("show name should parse from filename");

        let score =
            LibraryScanService::score_episode_show_candidate(&normalized, "Severance", None, None);

        assert!(
            score < 0.70,
            "expected 'The Bear' vs 'Severance' to score below the auto-link threshold, got {score}"
        );
    }

    #[test]
    fn episode_show_score_accepts_matching_show() {
        let hint = LibraryScanService::parse_episode_hint("/data/TV/Severance.S01E05.mkv");
        let normalized = hint
            .show_name
            .as_deref()
            .map(LibraryScanService::normalize_for_match)
            .expect("show name should parse from filename");

        let score =
            LibraryScanService::score_episode_show_candidate(&normalized, "Severance", None, None);

        assert!(
            score >= 0.70,
            "expected a correct show-name match to clear the auto-link threshold, got {score}"
        );
        // Sanity-check the audit's claim that a correct match scores well above the
        // 0.70 threshold (jaro_winkler ~0.95+ * 0.9 ~= 0.85+, or 0.95 via substring match).
        assert!(score >= 0.85, "expected a strong match score, got {score}");
    }

    #[test]
    fn manual_match_guard_blocks_automatic_rematch_of_manual_match() {
        // audit #5 / design.md Q9b: automatic matching (no explicit target id) must
        // never be applied over an existing manual match, regardless of force.
        assert!(LibraryScanService::should_block_automatic_match(
            false,
            Some("manual")
        ));
    }

    #[test]
    fn manual_match_guard_allows_explicit_target_to_replace_manual_match() {
        // Explicit-id requests are the user's own intent and may replace a manual match.
        assert!(!LibraryScanService::should_block_automatic_match(
            true,
            Some("manual")
        ));
    }

    #[test]
    fn manual_match_guard_allows_automatic_rematch_of_auto_or_unmatched_file() {
        assert!(!LibraryScanService::should_block_automatic_match(
            false,
            Some("auto")
        ));
        assert!(!LibraryScanService::should_block_automatic_match(
            false, None
        ));
    }

    #[test]
    fn tv_provider_fingerprint_groups_episodes_from_same_show() {
        let first = LibraryScanService::tv_provider_fingerprint(
            "Fallout (2024)/Season 01/Fallout - S01E01 - The End.mkv",
        );
        let second = LibraryScanService::tv_provider_fingerprint(
            "Fallout (2024)/Season 02/Fallout - S02E04 - The Demon in the Snow.mkv",
        );

        assert_eq!(first, Some("fallout".to_string()));
        assert_eq!(first, second);
    }

    #[test]
    fn normalize_library_type_handles_uppercase() {
        assert_eq!(
            LibraryScanService::normalize_library_type("MOVIES"),
            "movies"
        );
        assert_eq!(LibraryScanService::normalize_library_type("TV"), "tv");
        assert_eq!(
            LibraryScanService::normalize_library_type("AUDIOBOOKS"),
            "audiobooks"
        );
    }

    #[test]
    fn media_file_version_change_uses_size_and_mtime() {
        assert!(!LibraryScanService::media_file_version_changed(
            10,
            Some("2026-07-30T00:00:00Z"),
            10,
            Some("2026-07-30T00:00:00Z"),
        ));
        assert!(LibraryScanService::media_file_version_changed(
            10,
            Some("2026-07-30T00:00:00Z"),
            11,
            Some("2026-07-30T00:00:00Z"),
        ));
        assert!(LibraryScanService::media_file_version_changed(
            10,
            Some("2026-07-30T00:00:00Z"),
            10,
            Some("2026-07-30T00:00:01Z"),
        ));
        assert!(LibraryScanService::media_file_version_changed(
            10,
            None,
            10,
            Some("2026-07-30T00:00:00Z"),
        ));
        assert!(!LibraryScanService::media_file_version_changed(
            10, None, 10, None,
        ));
    }

    #[test]
    fn tv_protected_paths_do_not_include_stale_season_like_folders() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let root = temp_dir.path();
        let stale_season = root.join("Example Show").join("Season 99");
        fs::create_dir_all(&stale_season)?;

        let shows = vec![("Example Show".to_string(), None, None, vec![1])];
        let protected = LibraryScanService::build_tv_protected_paths_from_rows(
            &root.to_string_lossy(),
            "{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}",
            &shows,
        )?;

        assert!(protected.contains(&root.to_path_buf()));
        assert!(protected.contains(&root.join("Example Show")));
        assert!(protected.contains(&root.join("Example Show").join("Season 01")));
        assert!(!protected.contains(&stale_season));

        Ok(())
    }

    #[tokio::test]
    async fn cleanup_empty_unprotected_folders_removes_nested_empty_tree() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let root = temp_dir.path();
        let stale_tree = root.join("Old Layout").join("Nested").join("Leaf");
        let protected_show = root.join("Expected Show");
        let protected_season = protected_show.join("Season 01");
        tokio::fs::create_dir_all(&stale_tree).await?;
        tokio::fs::create_dir_all(&protected_season).await?;

        let protected = HashSet::from([
            root.to_path_buf(),
            protected_show.clone(),
            protected_season.clone(),
        ]);

        LibraryScanService::cleanup_empty_unprotected_folders(root, &protected).await?;

        assert!(!root.join("Old Layout").exists());
        assert!(protected_show.exists());
        assert!(protected_season.exists());

        Ok(())
    }

    #[tokio::test]
    async fn library_scan_service_starts_on_fresh_database_without_users() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let database_path = temp_dir.path().join("librarian.db");
        let database_url = format!("sqlite://{}", database_path.display());

        let services = crate::services::ServicesManager::builder()
            .add_service(crate::services::DatabaseServiceConfig {
                database_url,
                connect_timeout: Duration::from_secs(5),
            })
            .add_service(crate::services::AuthConfig::for_tests())
            .add_service(crate::services::GraphqlServiceConfig { server_port: 0 })
            .add_service(LibraryScanServiceConfig {
                autoscan_poll_interval: Duration::from_secs(3600),
                analyze_workers: 1,
            })
            .start()
            .await?;

        assert!(services.get_library_scan().await.is_some());
        services.stop_all().await?;

        Ok(())
    }

    #[test]
    fn movie_scan_without_tmdb_persists_one_actionable_configuration_issue() -> anyhow::Result<()> {
        // The generated entity futures make this integration scenario larger
        // than Rust's default test-thread stack in debug builds. Production
        // uses the same 8 MiB Tokio stack size (see main.rs), so exercise it
        // under that real runtime constraint instead of relying on
        // RUST_MIN_STACK in CI.
        std::thread::Builder::new()
            .name("movie-scan-no-tmdb-test".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(|| {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(movie_scan_without_tmdb_case())
            })?
            .join()
            .map_err(|_| anyhow::anyhow!("movie scan integration test thread panicked"))?
    }

    async fn movie_scan_without_tmdb_case() -> anyhow::Result<()> {
        use crate::graphql::entities::{
            AppSetting, AppSettingWhereInput, CreateLibraryInput, CreateMovieInput,
            CreateUserInput, Library, LibraryScanIssue, LibraryScanIssueWhereInput, LibraryScanRun,
            MediaFile, MediaFileWhereInput, Movie, Notification, NotificationWhereInput,
            UpdateAppSettingInput, User,
        };
        use graphql_orm::graphql::filters::{DateFilter, StringFilter};

        let temp_dir = tempfile::tempdir()?;
        let library_root = temp_dir.path().join("movies");
        std::fs::create_dir_all(&library_root)?;
        let movie_path = library_root.join("Example.Movie.2024.mkv");
        std::fs::write(&movie_path, b"not a valid media container")?;
        let database_url = format!(
            "sqlite://{}",
            temp_dir.path().join("librarian.db").display()
        );

        let services = crate::services::ServicesManager::builder()
            .add_service(crate::services::DatabaseServiceConfig {
                database_url,
                connect_timeout: Duration::from_secs(5),
            })
            .add_service(crate::services::AuthConfig::for_tests())
            .add_service(crate::services::GraphqlServiceConfig { server_port: 0 })
            .add_service(LibraryScanServiceConfig {
                autoscan_poll_interval: Duration::from_secs(3600),
                analyze_workers: 1,
            })
            .start()
            .await?;
        let database = services
            .get_database()
            .await
            .ok_or_else(|| anyhow::anyhow!("database service missing"))?;
        let user = User::insert(
            database.pool(),
            CreateUserInput {
                username: "scan-owner".to_string(),
                email: Some("scan-owner@example.test".to_string()),
                password_hash: "test-only".to_string(),
                role: "Member".to_string(),
                display_name: None,
                avatar_url: None,
                is_active: true,
                last_login_at: None,
            },
        )
        .await?;
        let library = Library::insert(
            database.pool(),
            CreateLibraryInput {
                user_id: user.id.clone(),
                name: "Movies without TMDB".to_string(),
                path: library_root.to_string_lossy().to_string(),
                library_type: "movies".to_string(),
                icon: None,
                color: None,
                auto_scan: false,
                auto_organize: false,
                auto_resolve_duplicates: None,
                naming_pattern: String::new(),
                scan_interval_minutes: 60,
                watch_for_changes: false,
                watch_status: None,
                watch_detail: None,
                watch_status_since: None,
                scanning: false,
                last_scanned_at: None,
                quality_profile_id: None,
                storage_volume_id: None,
            },
        )
        .await?;
        let scanner = services
            .get_library_scan()
            .await
            .ok_or_else(|| anyhow::anyhow!("library scan service missing"))?;

        for _ in 0..2 {
            let queued = scanner.queue_scan(&library.id).await?;
            let run_id = queued
                .scan_run_id
                .ok_or_else(|| anyhow::anyhow!("scan did not return a run id"))?;
            let mut terminal = None;
            for _ in 0..300 {
                terminal = LibraryScanRun::get(database.pool().pool(), &run_id).await?;
                if terminal.as_ref().is_some_and(|run| {
                    matches!(
                        run.status.as_str(),
                        "COMPLETED" | "COMPLETED_WITH_ISSUES" | "FAILED" | "CANCELLED"
                    )
                }) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            let terminal =
                terminal.ok_or_else(|| anyhow::anyhow!("scan run did not complete in time"))?;
            assert_eq!(terminal.status, "COMPLETED_WITH_ISSUES");
            assert!(terminal.provider_blocked_count >= 1);
        }

        let media_files = MediaFile::query(database.pool().pool())
            .filter(MediaFileWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        assert_eq!(media_files.len(), 1);
        assert!(media_files[0].movie_id.is_none());

        let issues = LibraryScanIssue::query(database.pool().pool())
            .filter(LibraryScanIssueWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                issue_code: Some(StringFilter {
                    eq: Some("TMDB_NOT_CONFIGURED".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        // The second scan finds the same problem: it's the same issue, seen twice, not a new row.
        assert_eq!(issues.len(), 1, "a repeated problem stays one issue");
        assert_eq!(issues[0].occurrence_count, 2);
        assert!(issues[0].resolved_at.is_none());

        let notifications = Notification::query(database.pool().pool())
            .filter(NotificationWhereInput {
                user_id: Some(StringFilter {
                    eq: Some(user.id.clone()),
                    ..Default::default()
                }),
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                title: Some(StringFilter {
                    eq: Some("TMDB API key required for movie scanning".to_string()),
                    ..Default::default()
                }),
                resolved_at: Some(DateFilter {
                    is_null: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        assert_eq!(notifications.len(), 1);

        let tmdb_settings = AppSetting::query(database.pool().pool())
            .filter(AppSettingWhereInput {
                key: Some(StringFilter {
                    eq: Some("metadata.tmdb_api_key".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        let tmdb_setting = tmdb_settings
            .first()
            .ok_or_else(|| anyhow::anyhow!("TMDB setting was not seeded"))?;
        AppSetting::update_by_id(
            database.pool(),
            &tmdb_setting.id,
            UpdateAppSettingInput {
                key: None,
                value: Some("fake-provider-key".to_string()),
                description: None,
                category: None,
            },
        )
        .await?;
        let movie = Movie::insert(
            database.pool(),
            CreateMovieInput {
                locked_fields: None,
                library_id: library.id.clone(),
                user_id: user.id.clone(),
                title: "Example Movie".to_string(),
                sort_title: None,
                original_title: None,
                year: Some(2024),
                tmdb_id: Some(12345),
                imdb_id: None,
                overview: None,
                tagline: None,
                runtime: None,
                genres: Vec::new(),
                director: None,
                cast_names: Vec::new(),
                production_countries: Vec::new(),
                spoken_languages: Vec::new(),
                tmdb_rating: None,
                tmdb_vote_count: None,
                poster_url: None,
                backdrop_url: None,
                collection_id: None,
                collection_name: None,
                collection_poster_url: None,
                release_date: None,
                certification: None,
                digital_release_date: None,
                physical_release_date: None,
                monitored: true,
                tmdb_status: Some("Released".to_string()),
                wanted: true,
                ignored: None,
                download_status: None,
                has_file: false,
                media_file_id: None,
                quality_profile_id: None,
            },
        )
        .await?;

        let configured_run_id = scanner
            .queue_scan(&library.id)
            .await?
            .scan_run_id
            .ok_or_else(|| anyhow::anyhow!("configured scan did not return a run id"))?;
        let mut configured_terminal = None;
        // Full-suite contention can make the database-backed worker materially slower than an
        // isolated run. Keep a bounded one-minute deadline so this remains a reliable regression
        // test without masking a genuinely stuck scan.
        for _ in 0..1200 {
            configured_terminal =
                LibraryScanRun::get(database.pool().pool(), &configured_run_id).await?;
            if configured_terminal.as_ref().is_some_and(|run| {
                matches!(
                    run.status.as_str(),
                    "COMPLETED" | "COMPLETED_WITH_ISSUES" | "FAILED" | "CANCELLED"
                )
            }) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        configured_terminal
            .ok_or_else(|| anyhow::anyhow!("configured scan did not complete in time"))?;

        let linked_files = MediaFile::query(database.pool().pool())
            .filter(MediaFileWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        assert_eq!(linked_files.len(), 1);
        assert_eq!(linked_files[0].movie_id.as_deref(), Some(movie.id.as_str()));

        let open_notifications = Notification::query(database.pool().pool())
            .filter(NotificationWhereInput {
                user_id: Some(StringFilter {
                    eq: Some(user.id),
                    ..Default::default()
                }),
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                title: Some(StringFilter {
                    eq: Some("TMDB API key required for movie scanning".to_string()),
                    ..Default::default()
                }),
                resolved_at: Some(DateFilter {
                    is_null: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        assert!(
            open_notifications.is_empty(),
            "successful matching should resolve the configuration notification"
        );
        let unresolved_tmdb_issues = LibraryScanIssue::query(database.pool().pool())
            .filter(LibraryScanIssueWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id),
                    ..Default::default()
                }),
                issue_code: Some(StringFilter {
                    eq: Some("TMDB_NOT_CONFIGURED".to_string()),
                    ..Default::default()
                }),
                resolved_at: Some(DateFilter {
                    is_null: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        assert!(unresolved_tmdb_issues.is_empty());

        services.stop_all().await?;
        Ok(())
    }

    #[test]
    fn a_record_is_placed_in_the_library_its_trash_or_outside_it() {
        let local = StoragePath::new("/data/media/Music");
        let place = |path: &str, root: &StoragePath| RecordPlace::of(path, root);
        assert_eq!(
            place("/data/media/Music/Artist/Album/01.flac", &local),
            RecordPlace::Library
        );
        assert_eq!(
            place(
                "/data/media/Music/.librarian-trash/a-b-02 - Dust n’ Bones.flac",
                &local
            ),
            RecordPlace::Trash
        );
        // A file merely named like the trash, or a folder of the same name deeper down, is not it.
        assert_eq!(
            place("/data/media/Music/.librarian-trash", &local),
            RecordPlace::Library
        );
        assert_eq!(
            place("/data/media/Music/Artist/.librarian-trash/x.flac", &local),
            RecordPlace::Library
        );
        assert_eq!(
            place("/data/media/Music2/Artist/01.flac", &local),
            RecordPlace::OutsideRoot
        );
        // After a move to a share, the old local paths are outside the library, trash or not.
        let share = StoragePath::new("smb://10.0.1.190/media/Music");
        assert_eq!(
            place("/data/media/Music/.librarian-trash/x.flac", &share),
            RecordPlace::OutsideRoot
        );
        assert_eq!(
            place(
                "smb://10.0.1.190/media/Music/.librarian-trash/x.flac",
                &share
            ),
            RecordPlace::Trash
        );
        assert_eq!(
            place("smb://10.0.1.190/media/Music/Artist/01.flac", &share),
            RecordPlace::Library
        );
        assert_eq!(
            place("smb://10.0.1.190/media/Movies/Film.mkv", &share),
            RecordPlace::OutsideRoot
        );
    }

    #[test]
    fn scan_removes_records_for_gone_files_the_walk_cannot_see() -> anyhow::Result<()> {
        std::thread::Builder::new()
            .name("unseen-record-scan-test".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(|| {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(scan_removes_unseen_gone_records_case())
            })?
            .join()
            .map_err(|_| anyhow::anyhow!("unseen record scan test thread panicked"))?
    }

    async fn scan_removes_unseen_gone_records_case() -> anyhow::Result<()> {
        use crate::graphql::entities::{
            CreateLibraryInput, CreateMediaFileInput, CreateUserInput, Library, LibraryScanRun,
            MediaFile, MediaFileWhereInput, User,
        };
        use anyhow::Context;
        use graphql_orm::graphql::filters::StringFilter;

        let temp_dir = tempfile::tempdir()?;
        let library_root = temp_dir.path().join("music");
        let trash = library_root.join(super::LIBRARY_TRASH_DIR);
        let elsewhere = temp_dir.path().join("elsewhere");
        std::fs::create_dir_all(&trash)?;
        std::fs::create_dir_all(&elsewhere)?;
        std::fs::write(library_root.join("Kept.mp3"), b"a track the walk sees")?;
        // Trashed files the person has not emptied yet, and a file outside the root that is still
        // there: their records stay.
        std::fs::write(trash.join("still-in-trash.flac"), b"recoverable")?;
        std::fs::write(elsewhere.join("outside.flac"), b"not in the library folder")?;

        let services = crate::services::ServicesManager::builder()
            .add_service(crate::services::DatabaseServiceConfig {
                database_url: format!(
                    "sqlite://{}",
                    temp_dir.path().join("librarian.db").display()
                ),
                connect_timeout: Duration::from_secs(5),
            })
            .add_service(crate::services::AuthConfig::for_tests())
            .add_service(crate::services::GraphqlServiceConfig { server_port: 0 })
            .add_service(LibraryScanServiceConfig {
                autoscan_poll_interval: Duration::from_secs(3600),
                analyze_workers: 1,
            })
            .start()
            .await?;
        let database = services
            .get_database()
            .await
            .context("database service missing")?;
        let user = User::insert(
            database.pool(),
            CreateUserInput {
                username: "unseen-owner".to_string(),
                email: Some("unseen-owner@example.test".to_string()),
                password_hash: "test-only".to_string(),
                role: "Member".to_string(),
                display_name: None,
                avatar_url: None,
                is_active: true,
                last_login_at: None,
            },
        )
        .await?;
        let library = Library::insert(
            database.pool(),
            CreateLibraryInput {
                user_id: user.id,
                name: "Unseen test".to_string(),
                path: library_root.to_string_lossy().to_string(),
                library_type: "music".to_string(),
                icon: None,
                color: None,
                auto_scan: false,
                auto_organize: false,
                auto_resolve_duplicates: None,
                naming_pattern: String::new(),
                scan_interval_minutes: 60,
                watch_for_changes: false,
                watch_status: None,
                watch_detail: None,
                watch_status_since: None,
                scanning: false,
                last_scanned_at: None,
                quality_profile_id: None,
                storage_volume_id: None,
            },
        )
        .await?;
        let record = |path: String| CreateMediaFileInput {
            library_id: Some(library.id.clone()),
            episode_id: None,
            movie_id: None,
            track_id: None,
            chapter_id: None,
            path,
            relative_path: None,
            original_name: None,
            size: 10,
            container: None,
            video_codec: None,
            audio_codec: None,
            width: None,
            height: None,
            duration: None,
            bitrate: None,
            resolution: None,
            is_hdr: false,
            hdr_type: None,
            closed_captions: None,
            audio_channels: None,
            metadata: None,
            content_type: None,
            added_at: "2026-10-26T00:00:00Z".to_string(),
            analyzed_at: None,
            file_modified_at: None,
            match_type: None,
            matched_by_user_id: None,
            match_confirmed_at: None,
            quality_status: None,
            picture_crop: None,
            storyboard: None,
            previews_checked_at: None,
        };
        let path_text = |path: PathBuf| path.to_string_lossy().to_string();
        let gone_from_trash = path_text(trash.join("emptied.flac"));
        let still_in_trash = path_text(trash.join("still-in-trash.flac"));
        let gone_hidden_deeper = path_text(library_root.join(".sync").join("old.flac"));
        let gone_old_root = "/definitely/not/a/library/old-root/Music/gone.flac".to_string();
        let outside_present = path_text(elsewhere.join("outside.flac"));
        let outside_gone = path_text(elsewhere.join("gone.flac"));
        for path in [
            &gone_from_trash,
            &still_in_trash,
            &gone_hidden_deeper,
            &gone_old_root,
            &outside_present,
            &outside_gone,
        ] {
            MediaFile::insert(database.pool(), record(path.clone())).await?;
        }

        let scanner = services
            .get_library_scan()
            .await
            .context("library scan service missing")?;
        let run_id = scanner
            .queue_scan(&library.id)
            .await?
            .scan_run_id
            .context("scan did not return a run id")?;
        let mut terminal = None;
        for _ in 0..1200 {
            terminal = LibraryScanRun::get(database.pool().pool(), &run_id).await?;
            if terminal.as_ref().is_some_and(|run| {
                matches!(
                    run.status.as_str(),
                    "COMPLETED" | "COMPLETED_WITH_ISSUES" | "FAILED" | "CANCELLED"
                )
            }) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let terminal = terminal.context("scan did not complete")?;
        assert_ne!(terminal.status, "FAILED", "{terminal:?}");

        let remaining = MediaFile::query(database.pool().pool())
            .filter(MediaFileWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?
            .into_iter()
            .map(|file| file.path)
            .collect::<HashSet<_>>();
        for gone in [
            &gone_from_trash,
            &gone_hidden_deeper,
            &gone_old_root,
            &outside_gone,
        ] {
            assert!(!remaining.contains(gone), "{gone} should be removed");
        }
        for kept in [&still_in_trash, &outside_present] {
            assert!(remaining.contains(kept), "{kept} should stay");
        }
        assert!(
            remaining.contains(&path_text(library_root.join("Kept.mp3"))),
            "the walked file stays"
        );
        assert!(trash.join("still-in-trash.flac").exists());
        assert!(elsewhere.join("outside.flac").exists());

        services.stop_all().await?;
        Ok(())
    }

    #[test]
    fn scan_reports_byte_identical_duplicates_without_deleting() -> anyhow::Result<()> {
        std::thread::Builder::new()
            .name("duplicate-scan-test".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(|| {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(scan_reports_byte_identical_duplicates_case())
            })?
            .join()
            .map_err(|_| anyhow::anyhow!("duplicate scan integration test thread panicked"))?
    }

    async fn scan_reports_byte_identical_duplicates_case() -> anyhow::Result<()> {
        use crate::graphql::entities::{
            CreateLibraryInput, CreateUserInput, Library, LibraryScanIssue,
            LibraryScanIssueWhereInput, LibraryScanRun, MediaFile, MediaFileWhereInput,
            UpdateMediaFileInput, User,
        };
        use anyhow::Context;
        use graphql_orm::graphql::filters::StringFilter;

        let temp_dir = tempfile::tempdir()?;
        let library_root = temp_dir.path().join("music");
        std::fs::create_dir_all(&library_root)?;
        let first_path = library_root.join("Track One.mp3");
        let second_path = library_root.join("Track One Copy.mp3");
        let bytes = b"identical test media payload";
        std::fs::write(&first_path, bytes)?;
        std::fs::write(&second_path, bytes)?;
        let database_url = format!(
            "sqlite://{}",
            temp_dir.path().join("librarian.db").display()
        );
        let services = crate::services::ServicesManager::builder()
            .add_service(crate::services::DatabaseServiceConfig {
                database_url,
                connect_timeout: Duration::from_secs(5),
            })
            .add_service(crate::services::AuthConfig::for_tests())
            .add_service(crate::services::GraphqlServiceConfig { server_port: 0 })
            .add_service(LibraryScanServiceConfig {
                autoscan_poll_interval: Duration::from_secs(3600),
                analyze_workers: 1,
            })
            .start()
            .await?;
        let database = services
            .get_database()
            .await
            .ok_or_else(|| anyhow::anyhow!("database service missing"))?;
        let user = User::insert(
            database.pool(),
            CreateUserInput {
                username: "duplicate-owner".to_string(),
                email: Some("duplicate-owner@example.test".to_string()),
                password_hash: "test-only".to_string(),
                role: "Member".to_string(),
                display_name: None,
                avatar_url: None,
                is_active: true,
                last_login_at: None,
            },
        )
        .await?;
        let library = Library::insert(
            database.pool(),
            CreateLibraryInput {
                user_id: user.id,
                name: "Duplicate test".to_string(),
                path: library_root.to_string_lossy().to_string(),
                library_type: "music".to_string(),
                icon: None,
                color: None,
                auto_scan: false,
                auto_organize: false,
                auto_resolve_duplicates: None,
                naming_pattern: String::new(),
                scan_interval_minutes: 60,
                watch_for_changes: false,
                watch_status: None,
                watch_detail: None,
                watch_status_since: None,
                scanning: false,
                last_scanned_at: None,
                quality_profile_id: None,
                storage_volume_id: None,
            },
        )
        .await?;
        let scanner = services
            .get_library_scan()
            .await
            .ok_or_else(|| anyhow::anyhow!("library scan service missing"))?;
        let run_id = scanner
            .queue_scan(&library.id)
            .await?
            .scan_run_id
            .context("scan did not return a run id")?;
        let mut terminal = None;
        // Full-suite contention can make the database-backed worker materially slower than an
        // isolated run. Keep a bounded one-minute deadline so this remains a reliable regression
        // test without masking a genuinely stuck scan.
        for _ in 0..1200 {
            terminal = LibraryScanRun::get(database.pool().pool(), &run_id).await?;
            if terminal.as_ref().is_some_and(|run| {
                matches!(
                    run.status.as_str(),
                    "COMPLETED" | "COMPLETED_WITH_ISSUES" | "FAILED" | "CANCELLED"
                )
            }) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(
            terminal.context("duplicate scan did not complete")?.status,
            "COMPLETED_WITH_ISSUES"
        );

        let media_files = MediaFile::query(database.pool().pool())
            .filter(MediaFileWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        assert_eq!(media_files.len(), 2);
        assert!(first_path.exists());
        assert!(second_path.exists());

        let issues = LibraryScanIssue::query(database.pool().pool())
            .filter(LibraryScanIssueWhereInput {
                scan_run_id: Some(StringFilter {
                    eq: Some(run_id),
                    ..Default::default()
                }),
                issue_code: Some(StringFilter {
                    eq: Some("BYTE_IDENTICAL_DUPLICATE".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        assert_eq!(issues.len(), 1);
        let details: serde_json::Value = serde_json::from_str(
            issues[0]
                .details_json
                .as_deref()
                .context("duplicate issue details missing")?,
        )?;
        assert_eq!(details["kind"], "byte-identical-duplicate");
        assert_eq!(details["size"], bytes.len() as i64);
        assert_eq!(details["sha256"].as_str().map(str::len), Some(64));
        assert_eq!(details["storageRelationship"], "COPY");
        assert_eq!(details["duplicateAnalysisState"], "NOT_ANALYZED");
        assert_eq!(details["keeperAnalysisState"], "NOT_ANALYZED");
        assert!(details["duplicateQualityStatus"].is_null());
        assert!(details["keeperQualityStatus"].is_null());

        let first_file = media_files
            .iter()
            .find(|file| file.path == first_path.to_string_lossy())
            .context("first media file missing")?;
        let second_file = media_files
            .iter()
            .find(|file| file.path == second_path.to_string_lossy())
            .context("second media file missing")?;
        let synthetic_analysis_time = "2026-07-30T00:00:00Z".to_string();
        MediaFile::update_by_id(
            database.pool(),
            &first_file.id,
            UpdateMediaFileInput {
                analyzed_at: Some(Some(synthetic_analysis_time.clone())),
                quality_status: Some(Some("optimal".to_string())),
                ..Default::default()
            },
        )
        .await?;
        MediaFile::update_by_id(
            database.pool(),
            &second_file.id,
            UpdateMediaFileInput {
                analyzed_at: Some(Some(synthetic_analysis_time.clone())),
                file_modified_at: Some(Some("2000-01-01T00:00:00Z".to_string())),
                quality_status: Some(Some("optimal".to_string())),
                ..Default::default()
            },
        )
        .await?;

        let rescan_id = scanner
            .queue_scan(&library.id)
            .await?
            .scan_run_id
            .context("rescan did not return a run id")?;
        let mut rescan_terminal = None;
        for _ in 0..1200 {
            rescan_terminal = LibraryScanRun::get(database.pool().pool(), &rescan_id).await?;
            if rescan_terminal.as_ref().is_some_and(|run| {
                matches!(
                    run.status.as_str(),
                    "COMPLETED" | "COMPLETED_WITH_ISSUES" | "FAILED" | "CANCELLED"
                )
            }) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let rescan_terminal = rescan_terminal.context("duplicate rescan did not complete")?;
        assert!(
            matches!(
                rescan_terminal.status.as_str(),
                "COMPLETED" | "COMPLETED_WITH_ISSUES" | "FAILED" | "CANCELLED"
            ),
            "duplicate rescan remained non-terminal: {}",
            rescan_terminal.status
        );

        let rescanned_files = MediaFile::query(database.pool().pool())
            .filter(MediaFileWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        let unchanged = rescanned_files
            .iter()
            .find(|file| file.id == first_file.id)
            .context("unchanged file missing after rescan")?;
        let invalidated = rescanned_files
            .iter()
            .find(|file| file.id == second_file.id)
            .context("changed file missing after rescan")?;
        assert_eq!(
            unchanged.analyzed_at.as_deref(),
            Some(synthetic_analysis_time.as_str())
        );
        assert_eq!(unchanged.quality_status.as_deref(), Some("optimal"));
        assert!(invalidated.analyzed_at.is_none());
        assert!(invalidated.quality_status.is_none());
        assert_ne!(
            invalidated.file_modified_at.as_deref(),
            Some("2000-01-01T00:00:00Z")
        );

        // Settling a pair whose kept copy has gone closes the issue and moves nothing: the copy
        // left is the only one.
        let library_id = unchanged
            .library_id
            .clone()
            .context("media file has no library")?;
        let open = scanner.open_duplicate_issues(&library_id).await?;
        assert_eq!(open.len(), 1, "one open issue for the pair");
        let pair: DuplicateIssueDetails = serde_json::from_str(
            open[0]
                .details_json
                .as_deref()
                .context("duplicate issue details missing")?,
        )?;
        let away = temp_dir.path().join("kept-copy-away.mp3");
        std::fs::rename(&pair.keeper_path, &away)?;
        let settled = scanner
            .trash_duplicate_issue_keeping(&open[0].id, DuplicateKeep::Best)
            .await?;
        assert!(settled.trash_path.is_none(), "nothing may move");
        assert!(
            settled
                .no_longer_duplicate
                .as_deref()
                .is_some_and(|reason| reason.contains("no longer on disk")),
            "closed as no longer a duplicate: {:?}",
            settled.no_longer_duplicate
        );
        assert!(std::path::Path::new(&pair.duplicate_path).exists());
        let closed = LibraryScanIssue::get(database.pool().pool(), &open[0].id)
            .await?
            .context("issue vanished")?;
        assert!(closed.resolved_at.is_some());
        assert!(
            closed
                .resolution
                .as_deref()
                .is_some_and(|text| text.starts_with(NO_LONGER_DUPLICATE))
        );

        // The kept copy coming back makes them duplicates again, and the closure must not
        // silence that.
        std::fs::rename(&away, &pair.keeper_path)?;
        let reopen_id = scanner
            .queue_scan(&library_id)
            .await?
            .scan_run_id
            .context("rescan did not return a run id")?;
        for _ in 0..1200 {
            if LibraryScanRun::get(database.pool().pool(), &reopen_id)
                .await?
                .is_some_and(|run| {
                    matches!(
                        run.status.as_str(),
                        "COMPLETED" | "COMPLETED_WITH_ISSUES" | "FAILED" | "CANCELLED"
                    )
                })
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let reopened = scanner.open_duplicate_issues(&library_id).await?;
        assert_eq!(reopened.len(), 1, "the pair is reported again");

        // A copy disappearing between scans closes its issue on the next scan by itself.
        let reopened_pair: DuplicateIssueDetails = serde_json::from_str(
            reopened[0]
                .details_json
                .as_deref()
                .context("duplicate issue details missing")?,
        )?;
        std::fs::remove_file(&reopened_pair.duplicate_path)?;
        let sweep_id = scanner
            .queue_scan(&library_id)
            .await?
            .scan_run_id
            .context("rescan did not return a run id")?;
        for _ in 0..1200 {
            if LibraryScanRun::get(database.pool().pool(), &sweep_id)
                .await?
                .is_some_and(|run| {
                    matches!(
                        run.status.as_str(),
                        "COMPLETED" | "COMPLETED_WITH_ISSUES" | "FAILED" | "CANCELLED"
                    )
                })
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            scanner.open_duplicate_issues(&library_id).await?.is_empty(),
            "an issue whose copy has gone must not stay open"
        );
        assert!(std::path::Path::new(&reopened_pair.keeper_path).exists());

        services.stop_all().await?;
        Ok(())
    }

    #[test]
    fn parse_movie_hint_handles_language_tags_and_cut_labels() {
        let hint = LibraryScanService::parse_movie_hint(
            "[German] Fabian and the Deadly Wedding 2026 HDR 2160p WEB h265-EDITH",
        );
        assert_eq!(hint.title.as_deref(), Some("Fabian and the Deadly Wedding"));
        assert_eq!(hint.year, Some(2026));
    }

    #[test]
    fn parse_movie_hint_from_rss_samples_produces_clean_search_titles() {
        let xml = include_str!("../../tests/fixtures/rss/iptorrents_movies.xml");
        let bad_token_re = Regex::new(
            r"(?i)\b(2160p|1080p|720p|480p|4k|uhd|x264|x265|h264|h265|hevc|bluray|blu-ray|webrip|web-dl|hdr10?|dovi|atmos|ddp|dts|truehd|remux|repack|proper|internal|screener)\b",
        )
        .expect("bad token regex should compile");

        let mut checked = 0usize;
        let mut failures: Vec<String> = Vec::new();

        for raw_title in rss_titles_from_xml(xml).expect("RSS fixture should parse") {
            let hint = LibraryScanService::parse_movie_hint(&raw_title);
            checked += 1;

            let Some(search_title) = hint.title else {
                failures.push(format!("empty title from raw='{}'", raw_title));
                continue;
            };

            if bad_token_re.is_match(&search_title) {
                failures.push(format!(
                    "unclean title='{}' from raw='{}'",
                    search_title, raw_title
                ));
            }
            if search_title.starts_with('[') || search_title.ends_with(']') {
                failures.push(format!(
                    "bracket residue title='{}' from raw='{}'",
                    search_title, raw_title
                ));
            }
            if search_title.trim().len() < 2 {
                failures.push(format!(
                    "too short title='{}' from raw='{}'",
                    search_title, raw_title
                ));
            }
        }

        assert!(checked > 0, "no sample titles were parsed from RSS XML");
        assert!(
            failures.is_empty(),
            "found {} bad parsed movie search titles out of {} samples. first failures:\n{}",
            failures.len(),
            checked,
            failures
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn malformed_rss_fixture_reports_xml_parse_error() {
        let xml = include_str!("../../tests/fixtures/rss/malformed_partial.xml");
        assert!(rss_titles_from_xml(xml).is_err());
    }

    #[test]
    fn parse_movie_hint_strips_common_release_noise_from_logs() {
        let cases = vec![
            (
                "/mnt/z/Kids Movies/Tomorrowland.2015.HDRip.XviD.AC3-EVO.avi",
                "Tomorrowland",
                Some(2015),
            ),
            (
                "/mnt/z/Kids Movies/Earth.Star.Voyager  VHSRip 1988.avi",
                "Earth Star Voyager",
                Some(1988),
            ),
            (
                "/mnt/z/Kids Movies/Space.Camp.1986.Xvid.[Eng].DvdRip.avi",
                "Space Camp",
                Some(1986),
            ),
            (
                "/mnt/z/Kids Movies/Lilo and Stitch 2025 1080p WEBRip x264 READ NFO-SyncUp.mkv",
                "Lilo and Stitch",
                Some(2025),
            ),
            (
                "/mnt/z/Kids Movies/Sonic.the.Hedgehog.2.2022.1080p.AMZN.WEB-DL.DDP.5.1.H.264-PiRaTeS.mkv",
                "Sonic the Hedgehog 2",
                Some(2022),
            ),
            (
                "/mnt/z/Kids Movies/Sonic.the.Hedgehog.3.2024.1080p.TELESYNC.x264.COLLECTiVE.mkv",
                "Sonic the Hedgehog 3",
                Some(2024),
            ),
            (
                "Guardians.of.the.Galaxy.2014.V2.RETAIL.DVDRip.XviD.AC3-EVO.avi",
                "Guardians of the Galaxy",
                Some(2014),
            ),
            (
                "Top Gun Maverick 2022 2160p ATVP WEB-DL DDPA 5 1 H 265-PiRaTeS.mkv",
                "Top Gun Maverick",
                Some(2022),
            ),
            (
                "X-Men Days of Future Past 2014 KORSUB HDRip READNFO x264 AC3-MiLLENiUM.mkv",
                "X Men Days of Future Past",
                Some(2014),
            ),
            (
                "The Matrix 1999 720p BRRIP XVID AC3 - 26k.avi",
                "The Matrix",
                Some(1999),
            ),
        ];

        for (path, expected_title, expected_year) in cases {
            let hint = LibraryScanService::parse_movie_hint(path);
            assert_eq!(hint.title.as_deref(), Some(expected_title), "path={}", path);
            assert_eq!(hint.year, expected_year, "path={}", path);
        }
    }

    #[test]
    fn parse_movie_hint_from_movies_file_list_produces_clean_search_titles() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../docs/sample-data/movies-file-list.txt");
        let Ok(content) = fs::read_to_string(&sample_path) else {
            eprintln!(
                "skipping movie file-list parser test; missing optional fixture {}",
                sample_path.display()
            );
            return;
        };

        let bad_token_re = Regex::new(
            r"(?i)\b(2160p|1080p|720p|480p|4k|uhd|x264|x265|h264|h265|hevc|xvid|bluray|blu-ray|brrip|bdrip|webrip|web-dl|hdrip|dvdrip|dvdscr|hdts|hdcam|telesync|atmos|ddp|ddpa|dts|truehd|ac3|aac|flac|remux|repack|proper|internal|readnfo|nfo|korsub|retail|v[2-9]|hq|shq|hmax|atvp|amzn|webios|rosubbed|collective|pirates)\b",
        )
        .expect("bad token regex should compile");

        let mut checked = 0usize;
        let mut skipped_hidden = 0usize;
        let mut failures: Vec<String> = Vec::new();

        for raw_path in content.lines().map(str::trim).filter(|l| !l.is_empty()) {
            let raw_path_buf = PathBuf::from(raw_path);
            let file_name = raw_path_buf
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if file_name.starts_with('.') || file_name.starts_with("._") {
                skipped_hidden += 1;
                continue;
            }

            let hint = LibraryScanService::parse_movie_hint(raw_path);
            checked += 1;

            let Some(search_title) = hint.title else {
                failures.push(format!("empty title from path='{}'", raw_path));
                continue;
            };

            if search_title.trim().len() < 2 {
                failures.push(format!(
                    "too short title='{}' from path='{}'",
                    search_title, raw_path
                ));
            }
            if bad_token_re.is_match(&search_title) {
                failures.push(format!(
                    "unclean title='{}' from path='{}'",
                    search_title, raw_path
                ));
            }
        }

        assert!(
            checked > 0,
            "no movie filenames were checked from {}",
            sample_path.display()
        );
        assert!(
            failures.is_empty(),
            "found {} bad parsed movie search titles out of {} checked filenames (skipped_hidden={}). first failures:\n{}",
            failures.len(),
            checked,
            skipped_hidden,
            failures
                .iter()
                .take(20)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn parse_movie_hint_handles_numeric_titles_and_release_noise() {
        let cases = vec![
            (
                "Interstellar.2014.DVDScr.XVID.AC3.HQ.Hive-CM8.avi",
                "Interstellar",
                Some(2014),
            ),
            (
                "The.Martian.2015.HC.HDRip.X264.AC3-EVO.mkv",
                "The Martian",
                Some(2015),
            ),
            (
                "Dune.2021.2160p.HMAX.WEB-DL.DDP5.1.Atmos.HDR.HEVC-EVO.mkv",
                "Dune",
                Some(2021),
            ),
            (
                "Blade Runner 2049 (2017).mkv",
                "Blade Runner 2049",
                Some(2017),
            ),
            (
                "2001 A Space Odyssey (1968).mkv",
                "2001 A Space Odyssey",
                Some(1968),
            ),
            ("2012 (2009).mkv", "2012", Some(2009)),
        ];

        for (path, expected_title, expected_year) in cases {
            let hint = LibraryScanService::parse_movie_hint(path);
            assert_eq!(hint.title.as_deref(), Some(expected_title), "path={}", path);
            assert_eq!(hint.year, expected_year, "path={}", path);
        }
    }

    #[test]
    fn wanted_policy_filters_candidates_as_expected() {
        assert!(LibraryScanService::should_include_candidate(
            true,
            MatchWantedPolicy::PreferWanted
        ));
        assert!(LibraryScanService::should_include_candidate(
            false,
            MatchWantedPolicy::PreferWanted
        ));
        assert!(LibraryScanService::should_include_candidate(
            true,
            MatchWantedPolicy::WantedOnly
        ));
        assert!(!LibraryScanService::should_include_candidate(
            false,
            MatchWantedPolicy::WantedOnly
        ));
        assert!(LibraryScanService::should_include_candidate(
            true,
            MatchWantedPolicy::All
        ));
        assert!(LibraryScanService::should_include_candidate(
            false,
            MatchWantedPolicy::All
        ));
    }

    #[test]
    fn wanted_policy_affects_score_adjustment() {
        let prefer_wanted = LibraryScanService::adjust_candidate_score(
            0.70,
            true,
            false,
            MatchWantedPolicy::PreferWanted,
        );
        let prefer_non_wanted_with_file = LibraryScanService::adjust_candidate_score(
            0.70,
            false,
            true,
            MatchWantedPolicy::PreferWanted,
        );
        let all_policy_with_file =
            LibraryScanService::adjust_candidate_score(0.70, false, true, MatchWantedPolicy::All);

        assert!(
            prefer_wanted > 0.70,
            "prefer_wanted should boost wanted candidates"
        );
        assert!(
            prefer_non_wanted_with_file < 0.70,
            "prefer_wanted should penalize non-wanted candidates that already have files"
        );
        assert_eq!(
            all_policy_with_file, 0.70,
            "all policy should not apply wanted bias"
        );
    }

    #[tokio::test]
    async fn no_clobber_place_move_succeeds_and_removes_source() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let src = temp_dir.path().join("source.mkv");
        let dst = temp_dir.path().join("target.mkv");
        fs::write(&src, b"original bytes")?;

        let outcome = no_clobber_place(src.clone(), dst.clone(), true).await?;

        assert_eq!(outcome, NoClobberOutcome::Placed);
        assert!(!src.exists(), "source should be removed after a move");
        assert_eq!(fs::read(&dst)?, b"original bytes");

        Ok(())
    }

    #[tokio::test]
    async fn no_clobber_place_copy_keeps_source() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let src = temp_dir.path().join("source.mkv");
        let dst = temp_dir.path().join("target.mkv");
        fs::write(&src, b"seeding bytes")?;

        let outcome = no_clobber_place(src.clone(), dst.clone(), false).await?;

        assert_eq!(outcome, NoClobberOutcome::Placed);
        assert!(
            src.exists(),
            "source must survive a copy (e.g. seeding torrent file)"
        );
        assert_eq!(fs::read(&dst)?, b"seeding bytes");

        Ok(())
    }

    #[tokio::test]
    async fn no_clobber_place_never_overwrites_existing_target() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let src = temp_dir.path().join("source.mkv");
        let dst = temp_dir.path().join("target.mkv");
        fs::write(&src, b"new bytes")?;
        fs::write(&dst, b"existing target bytes -- must not be touched")?;

        let outcome = no_clobber_place(src.clone(), dst.clone(), true).await?;

        assert_eq!(outcome, NoClobberOutcome::TargetExists);
        assert!(src.exists(), "source must be left untouched on conflict");
        assert_eq!(fs::read(&src)?, b"new bytes");
        assert_eq!(
            fs::read(&dst)?,
            b"existing target bytes -- must not be touched",
            "existing target content must never be clobbered"
        );

        Ok(())
    }
}

#[cfg(test)]
mod atomic_analysis_tests {
    use super::*;
    use crate::db::Database;
    use crate::graphql::entities::*;
    use graphql_orm::DbPool;
    use graphql_orm::graphql::orm::{
        ChangeAction, MutationContext, MutationEvent, MutationHook, MutationPhase,
    };
    use graphql_orm::graphql::subscriptions::SubscriptionEvent;
    use std::sync::atomic::{AtomicBool, Ordering};

    async fn database() -> crate::db::Database {
        let db = crate::db::Database::new(DbPool::connect("sqlite::memory:").await.unwrap());
        crate::services::database::bootstrap_schema(&db)
            .await
            .unwrap();
        crate::services::DatabaseService::new(db).pool().clone()
    }
    async fn user(db: &Database) -> User {
        User::insert(
            db,
            CreateUserInput {
                username: uuid::Uuid::new_v4().to_string(),
                email: None,
                password_hash: "fixture password_hash".to_string(),
                role: "member".to_string(),
                display_name: None,
                avatar_url: None,
                is_active: true,
                last_login_at: None,
            },
        )
        .await
        .unwrap()
    }

    async fn library(db: &Database, user: &User) -> Library {
        Library::insert(
            db,
            CreateLibraryInput {
                user_id: user.id.clone(),
                name: "fixture library".to_string(),
                path: format!("/fixture/{}", uuid::Uuid::new_v4()),
                library_type: "music".to_string(),
                icon: None,
                color: None,
                auto_scan: false,
                auto_organize: false,
                auto_resolve_duplicates: None,
                naming_pattern: "fixture naming_pattern".to_string(),
                scan_interval_minutes: 0,
                watch_for_changes: false,
                watch_status: None,
                watch_detail: None,
                watch_status_since: None,
                scanning: false,
                last_scanned_at: None,
                quality_profile_id: None,
                storage_volume_id: None,
            },
        )
        .await
        .unwrap()
    }

    async fn media_file(
        db: &Database,
        library: &Library,
        size: i64,
        track_id: Option<&str>,
        chapter_id: Option<&str>,
    ) -> MediaFile {
        MediaFile::insert(
            db,
            CreateMediaFileInput {
                library_id: Some(library.id.clone()),
                episode_id: None,
                movie_id: None,
                track_id: track_id.map(str::to_string),
                chapter_id: chapter_id.map(str::to_string),
                path: format!("/fixture/{}.bin", uuid::Uuid::new_v4()),
                relative_path: None,
                original_name: None,
                size,
                container: None,
                video_codec: None,
                audio_codec: None,
                width: None,
                height: None,
                duration: None,
                bitrate: None,
                resolution: None,
                is_hdr: false,
                hdr_type: None,
                closed_captions: None,
                audio_channels: None,
                metadata: None,
                content_type: None,
                added_at: "2026-10-26T00:00:00Z".to_string(),
                analyzed_at: None,
                file_modified_at: None,
                match_type: None,
                matched_by_user_id: None,
                match_confirmed_at: None,
                quality_status: None,
                picture_crop: None,
                storyboard: None,
                previews_checked_at: None,
            },
        )
        .await
        .unwrap()
    }
    fn target(file: &MediaFile, library: &Library) -> AnalysisTarget {
        AnalysisTarget {
            file_id: file.id.clone(),
            library_id: library.id.clone(),
            owner_id: library.user_id.clone(),
            path: file.path.clone(),
            size: file.size,
            file_modified_at: file.file_modified_at.clone(),
        }
    }

    fn analysis(codec: &str) -> ProbeAnalysis {
        ProbeAnalysis {
            container: Some("matroska".to_string()),
            video_codec: Some(codec.to_string()),
            audio_codec: Some(format!("{codec}-audio")),
            width: Some(1920),
            height: Some(1080),
            video_streams: vec![VideoStreamAnalysis {
                stream_index: 0,
                codec: codec.to_string(),
                width: 1920,
                height: 1080,
                ..Default::default()
            }],
            audio_streams: vec![AudioStreamAnalysis {
                stream_index: 1,
                codec: format!("{codec}-audio"),
                channels: 2,
                ..Default::default()
            }],
            subtitles: vec![SubtitleAnalysis {
                source_type: "embedded".to_string(),
                stream_index: Some(2),
                codec: Some("subrip".to_string()),
                ..Default::default()
            }],
            chapters: vec![ChapterAnalysis {
                chapter_index: 0,
                start_secs: 0.0,
                end_secs: 30.0,
                title: Some(codec.to_string()),
            }],
            ..Default::default()
        }
    }

    struct FailAudioOnce {
        previous: Arc<dyn MutationHook>,
        enabled: Arc<AtomicBool>,
    }
    impl MutationHook for FailAudioOnce {
        fn on_mutation<'a>(
            &'a self,
            ctx: Option<&'a async_graphql::Context<'_>>,
            tx: &'a mut MutationContext<'_>,
            event: &'a MutationEvent,
        ) -> futures::future::BoxFuture<'a, async_graphql::Result<()>> {
            Box::pin(async move {
                self.previous.on_mutation(ctx, tx, event).await?;
                if event.entity_name == "AudioStream"
                    && event.phase == MutationPhase::After
                    && event.action == ChangeAction::Created
                    && self.enabled.swap(false, Ordering::AcqRel)
                {
                    return Err(async_graphql::Error::new(
                        "injected audio persistence failure",
                    ));
                }
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn analysis_failure_rolls_back_and_retry_commits_a_complete_stream_snapshot() {
        let mut db = database().await;
        let owner = user(&db).await;
        let lib = library(&db, &owner).await;
        let file = media_file(&db, &lib, 42, None, None).await;
        let expected = target(&file, &lib);
        replace_media_analysis(
            &db,
            &AuthUser::system_admin(),
            expected.clone(),
            analysis("old"),
        )
        .await
        .unwrap();
        let sidecar = Subtitle::insert(
            &db,
            CreateSubtitleInput {
                media_file_id: file.id.clone(),
                source_type: "downloaded".to_string(),
                stream_index: None,
                file_path: Some("/fixture/external.srt".to_string()),
                codec: Some("subrip".to_string()),
                codec_long_name: None,
                language: Some("en".to_string()),
                title: None,
                is_default: false,
                is_forced: false,
                is_hearing_impaired: false,
                opensubtitles_id: None,
                downloaded_at: None,
                metadata: None,
            },
        )
        .await
        .unwrap();
        let old_file = MediaFile::get(db.pool(), &file.id).await.unwrap().unwrap();
        let old_video = VideoStream::query(db.pool()).fetch_all().await.unwrap();
        let old_audio = AudioStream::query(db.pool()).fetch_all().await.unwrap();
        let old_chapters = MediaChapter::query(db.pool()).fetch_all().await.unwrap();
        let mut events = db
            .ensure_event_sender::<SubscriptionEvent<MediaFileChangedEvent>>()
            .subscribe();
        let mut stream_events = db
            .ensure_event_sender::<SubscriptionEvent<VideoStreamChangedEvent>>()
            .subscribe();
        let previous = db.mutation_hook().unwrap().clone();
        db.set_mutation_hook(FailAudioOnce {
            previous,
            enabled: Arc::new(AtomicBool::new(true)),
        });
        assert!(
            replace_media_analysis(
                &db,
                &AuthUser::system_admin(),
                expected.clone(),
                analysis("new")
            )
            .await
            .is_err()
        );
        let unchanged = MediaFile::get(db.pool(), &file.id).await.unwrap().unwrap();
        assert_eq!(unchanged.video_codec, old_file.video_codec);
        assert_eq!(unchanged.analyzed_at, old_file.analyzed_at);
        assert_eq!(
            VideoStream::query(db.pool()).fetch_all().await.unwrap()[0].id,
            old_video[0].id
        );
        assert_eq!(
            AudioStream::query(db.pool()).fetch_all().await.unwrap()[0].id,
            old_audio[0].id
        );
        assert_eq!(
            MediaChapter::query(db.pool()).fetch_all().await.unwrap()[0].id,
            old_chapters[0].id
        );
        assert!(
            Subtitle::get(db.pool(), &sidecar.id)
                .await
                .unwrap()
                .is_some()
        );
        assert!(events.try_recv().is_err());
        assert!(stream_events.try_recv().is_err());

        replace_media_analysis(&db, &AuthUser::system_admin(), expected, analysis("new"))
            .await
            .unwrap();
        assert!(events.try_recv().is_ok());
        assert!(stream_events.try_recv().is_ok());
        let restored = MediaFile::get(db.pool(), &file.id).await.unwrap().unwrap();
        assert_eq!(restored.video_codec.as_deref(), Some("new"));
        assert_eq!(
            (restored.path, restored.size, restored.library_id),
            (file.path, file.size, Some(lib.id.clone()))
        );
        assert_eq!(
            VideoStream::query(db.pool()).fetch_all().await.unwrap()[0].codec,
            "new"
        );
        assert_eq!(
            AudioStream::query(db.pool()).fetch_all().await.unwrap()[0].codec,
            "new-audio"
        );
        assert_eq!(
            MediaChapter::query(db.pool()).fetch_all().await.unwrap()[0]
                .title
                .as_deref(),
            Some("new")
        );
        let subtitles = Subtitle::query(db.pool()).fetch_all().await.unwrap();
        assert_eq!(subtitles.len(), 2);
        assert!(subtitles.iter().any(|row| row.id == sidecar.id));
        assert_eq!(
            Library::get(db.pool(), &lib.id)
                .await
                .unwrap()
                .unwrap()
                .user_id,
            owner.id
        );
    }

    #[tokio::test]
    async fn analysis_rejects_foreign_owners_forged_scope_and_stale_file_versions() {
        let db = database().await;
        let owner = user(&db).await;
        let other = user(&db).await;
        let lib = library(&db, &owner).await;
        let file = media_file(&db, &lib, 42, None, None).await;
        let scope = target(&file, &lib);
        let foreign = AuthUser {
            user_id: other.id.clone(),
            email: None,
            role: Some("member".to_string()),
        };
        assert!(
            replace_media_analysis(&db, &foreign, scope.clone(), analysis("bad"))
                .await
                .is_err()
        );
        let mut forged = scope.clone();
        forged.owner_id = other.id;
        assert!(
            replace_media_analysis(&db, &foreign, forged, analysis("bad"))
                .await
                .is_err()
        );
        MediaFile::update_by_id(
            &db,
            &file.id,
            UpdateMediaFileInput {
                size: Some(43),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let mut events = db
            .ensure_event_sender::<SubscriptionEvent<MediaFileChangedEvent>>()
            .subscribe();
        assert!(
            replace_media_analysis(&db, &AuthUser::system_admin(), scope, analysis("bad"))
                .await
                .is_err()
        );
        assert!(events.try_recv().is_err());
        assert!(
            VideoStream::query(db.pool())
                .fetch_all()
                .await
                .unwrap()
                .is_empty()
        );
        let unchanged = MediaFile::get(db.pool(), &file.id).await.unwrap().unwrap();
        assert_eq!(unchanged.analyzed_at, None);
        assert_eq!(unchanged.size, 43);
    }

    #[tokio::test]
    async fn scanner_uses_system_authority_without_selecting_an_account_and_rejects_unknown_owners()
    {
        let scanner = LibraryScanService::new(
            Arc::new(ServicesManager::new()),
            LibraryScanServiceConfig::default(),
        );
        let actor = scanner.try_system_auth_user(None).await.unwrap().unwrap();
        assert_eq!(actor.user_id, "system");
        let owner = Uuid::new_v4().to_string();
        assert_eq!(
            scanner
                .try_system_auth_user(Some(&owner))
                .await
                .unwrap()
                .unwrap()
                .user_id,
            "system"
        );
        assert!(actor.is_admin());
        assert!(
            scanner
                .try_system_auth_user(Some("unknown-owner"))
                .await
                .is_err()
        );
        let result = scanner
            .execute_mutation(
                &actor,
                "mutation { ignored }",
                serde_json::json!({"input": {"userId": "system"}}),
            )
            .await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("ownership is unknown")
        );
    }

    #[test]
    fn a_source_row_is_only_adopted_by_its_own_download_and_owner() {
        let row = serde_json::json!({ "sourceId": "hash", "sourceOwnerId": "me" });
        assert!(source_row_is_this_import(&row, "me", "hash"));
        assert!(!source_row_is_this_import(&row, "them", "hash"));
        assert!(!source_row_is_this_import(&row, "me", "other"));
        // From before sources were recorded.
        let legacy = serde_json::json!({ "sourceType": "torrent", "unmatchedReason": "waiting" });
        assert!(source_row_is_this_import(&legacy, "me", "hash"));
        assert!(!source_row_is_this_import(
            &serde_json::json!({ "sourceType": "scan" }),
            "me",
            "hash"
        ));
        let no_owner = serde_json::json!({ "sourceType": "torrent", "sourceId": "hash" });
        assert!(source_row_is_this_import(&no_owner, "me", "hash"));
        assert!(!source_row_is_this_import(&no_owner, "me", "other"));
        assert!(!source_row_is_this_import(
            &serde_json::json!({}),
            "me",
            "hash"
        ));
    }

    #[test]
    fn torrent_import_uses_the_persisted_source_owner_and_rejects_unknown_or_foreign_scope() {
        let owner = Uuid::new_v4().to_string();
        let caller = AuthUser {
            user_id: owner.clone(),
            email: None,
            role: Some("member".to_string()),
        };
        assert!(
            LibraryScanService::torrent_import_owner(&caller, &TorrentImportLink::default())
                .is_err()
        );
        let link = TorrentImportLink {
            user_id: Some(owner.clone()),
            ..Default::default()
        };
        assert_eq!(
            LibraryScanService::torrent_import_owner(&AuthUser::system_admin(), &link).unwrap(),
            owner
        );
        let foreign = AuthUser {
            user_id: Uuid::new_v4().to_string(),
            email: None,
            role: Some("member".to_string()),
        };
        assert!(LibraryScanService::torrent_import_owner(&foreign, &link).is_err());
    }

    #[test]
    fn scanner_mutation_failures_are_checked_even_without_transport_errors() {
        assert!(
            check_mutation_payloads(
                &serde_json::json!({"updateMediaFile": {"success": false, "error": "denied"}})
            )
            .is_err()
        );
        assert!(
            check_mutation_payloads(
                &serde_json::json!({"UpdateMediaFile": {"Success": true, "Error": null}})
            )
            .is_ok()
        );
        assert!(check_mutation_payloads(&serde_json::json!({"UpdateMediaFile": null})).is_err());
    }
    fn import_torrent_input(user: &AuthUser, hash: &str) -> CreateTorrentInput {
        CreateTorrentInput {
            user_id: user.user_id.clone(),
            info_hash: hash.into(),
            magnet_uri: None,
            name: "persisted name".into(),
            state: "paused".into(),
            progress: 0.1,
            total_bytes: 100,
            downloaded_bytes: 10,
            uploaded_bytes: 0,
            uploaded_bytes_total: 0,
            delete_files_on_remove: false,
            save_path: "/test/downloads".into(),
            download_path: None,
            source_url: None,
            source_feed_id: None,
            source_indexer_id: None,
            library_id: None,
            identification_kind: None,
            identification_status: None,
            identification_summary: None,
            post_process_status: None,
            post_process_error: None,
            processed_at: None,
            episode_id: None,
            movie_id: None,
            track_id: None,
            chapter_id: None,
            show_id: None,
            album_id: None,
            audiobook_id: None,
            minimum_ratio: None,
            minimum_seed_time_minutes: None,
            season: None,
            excluded_files: vec![],
            added_at: "2026-10-24T00:00:00Z".into(),
            completed_at: None,
        }
    }

    #[tokio::test]
    async fn rejected_source_requests_cannot_write_foreign_status_or_emit_changes() {
        // The GraphQL service here only builds its schema; no HTTP listener or
        // torrent/scan worker is started, and all storage is in memory.
        let manager = ServicesManager::builder()
            .add_service(crate::services::DatabaseServiceConfig {
                database_url: "sqlite::memory:".to_string(),
                connect_timeout: Duration::from_secs(5),
            })
            .add_service(crate::services::AuthConfig::for_tests())
            .add_service(crate::services::GraphqlServiceConfig { server_port: 0 })
            .start()
            .await
            .unwrap();
        let db = manager.get_database().await.unwrap().pool().clone();
        let first = user(&db).await;
        let second = user(&db).await;
        let first_library = library(&db, &first).await;
        let second_library = library(&db, &second).await;
        let first_actor = AuthUser {
            user_id: first.id.clone(),
            email: None,
            role: Some("member".to_string()),
        };
        let second_actor = AuthUser {
            user_id: second.id.clone(),
            email: None,
            role: Some("member".to_string()),
        };
        let hash = "b".repeat(40);
        let mut input = import_torrent_input(&second_actor, &hash);
        input.library_id = Some(second_library.id.clone());
        input.post_process_status = Some("unmatched".to_string());
        input.post_process_error = Some("existing outcome".to_string());
        let foreign = Torrent::insert(&db, input).await.unwrap();
        let scanner = LibraryScanService::new(manager.clone(), LibraryScanServiceConfig::default());
        let mut changes = db
            .ensure_event_sender::<SubscriptionEvent<TorrentChangedEvent>>()
            .subscribe();
        let forged_admin = AuthUser::system_admin_for(&first.id);
        for (actor, target) in [
            (&first_actor, None),
            (&forged_admin, None),
            (&second_actor, Some(first_library.id.clone())),
        ] {
            let rejected = scanner
                .process_torrent_source_files(actor, &hash, target, vec![])
                .await;
            assert!(!rejected.success);
            let row = Torrent::get(db.pool(), &foreign.id).await.unwrap().unwrap();
            assert_eq!(row.post_process_status, foreign.post_process_status);
            assert_eq!(row.post_process_error, foreign.post_process_error);
            assert_eq!(row.processed_at, foreign.processed_at);
        }
        assert!(changes.try_recv().is_err());
        assert!(
            PendingFileMatch::query(db.pool())
                .fetch_all()
                .await
                .unwrap()
                .is_empty()
        );
        let proof = LibraryScanService::authorize_torrent_import(&db, &second_actor, &hash, None)
            .await
            .unwrap();
        LibraryScanService::record_torrent_import_status(
            &db,
            &proof,
            "failed",
            Some("authorized operational failure"),
        )
        .await
        .unwrap();
        let row = Torrent::get(db.pool(), &foreign.id).await.unwrap().unwrap();
        assert_eq!(row.user_id, second.id);
        assert_eq!(row.post_process_status.as_deref(), Some("failed"));
        assert_eq!(
            row.post_process_error.as_deref(),
            Some("authorized operational failure")
        );
        assert!(changes.try_recv().is_ok());
        manager.stop_all().await.unwrap();
    }

    #[tokio::test]
    async fn import_status_cannot_follow_an_owner_change_or_revoked_actor() {
        let db = database().await;
        let owner = user(&db).await;
        let other = user(&db).await;
        let actor = AuthUser {
            user_id: owner.id.clone(),
            email: None,
            role: Some("member".to_string()),
        };
        let hash = "c".repeat(40);
        let row = Torrent::insert(&db, import_torrent_input(&actor, &hash))
            .await
            .unwrap();
        let proof = LibraryScanService::authorize_torrent_import(&db, &actor, &hash, None)
            .await
            .unwrap();
        Torrent::update_by_id(
            &db,
            &row.id,
            UpdateTorrentInput {
                user_id: Some(other.id),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(
            LibraryScanService::record_torrent_import_status(
                &db,
                &proof,
                "failed",
                Some("must not persist")
            )
            .await
            .is_err()
        );
        assert!(
            Torrent::get(db.pool(), &row.id)
                .await
                .unwrap()
                .unwrap()
                .post_process_status
                .is_none()
        );
        Torrent::update_by_id(
            &db,
            &row.id,
            UpdateTorrentInput {
                user_id: Some(owner.id.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        User::update_by_id(
            &db,
            &owner.id,
            UpdateUserInput {
                is_active: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(
            LibraryScanService::record_torrent_import_status(
                &db,
                &proof,
                "failed",
                Some("must not persist")
            )
            .await
            .is_err()
        );
        assert!(
            Torrent::get(db.pool(), &row.id)
                .await
                .unwrap()
                .unwrap()
                .post_process_status
                .is_none()
        );
    }
}

#[cfg(test)]
mod library_path_safety_tests {
    use super::*;

    fn library(root: &Path, pattern: &str) -> LibraryRow {
        LibraryRow {
            id: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4().to_string(),
            name: "Safe custom library".to_string(),
            path: root.to_string_lossy().to_string(),
            library_type: "movies".to_string(),
            auto_organize: true,
            scan_interval_minutes: 0,
            scanning: false,
            naming_pattern: pattern.to_string(),
        }
    }
    fn scanner() -> LibraryScanService {
        LibraryScanService::new(
            Arc::new(ServicesManager::new()),
            LibraryScanServiceConfig::default(),
        )
    }

    #[tokio::test]
    async fn relative_owner_patterns_remain_available_and_absolute_or_parent_templates_fail() {
        let temp = tempfile::tempdir().unwrap();
        let scanner = scanner();
        for pattern in [
            "/outside/{title}.{ext}",
            "../outside/{title}",
            "nested/../../{title}",
            r"C:\outside\{title}",
            r"\\server\share\{title}",
        ] {
            assert!(
                scanner
                    .resolve_library_naming_pattern(&library(temp.path(), pattern))
                    .await
                    .is_err(),
                "{pattern}"
            );
        }
        let pattern = "Custom/{title} ({year})/{original}.{ext}";
        assert_eq!(
            scanner
                .resolve_library_naming_pattern(&library(temp.path(), pattern))
                .await
                .unwrap(),
            pattern
        );
        let relative =
            apply_movie_naming_pattern(pattern, "Movie", Some(2026), "Release.mkv", "mkv");
        validate_relative_library_path(&relative).unwrap();
        checked_library_path(temp.path(), &temp.path().join(relative)).unwrap();
        let expanded =
            apply_movie_naming_pattern("{original}/{title}.{ext}", "Movie", None, "..", "mkv");
        assert!(validate_relative_library_path(&expanded).is_err());
        for relative in [
            apply_tv_naming_pattern(
                "../{show}/{title}.{ext}",
                "Show",
                1,
                1,
                Some("Title"),
                "mkv",
            ),
            apply_music_naming_pattern(
                "../{artist}/{title}.{ext}",
                "Artist",
                "Album",
                None,
                1,
                None,
                "Track",
                "track.flac",
                "flac",
            ),
            apply_audiobook_naming_pattern(
                "../{author}/{title}.{ext}",
                "Author",
                "Book",
                1,
                None,
                "book.m4b",
                "m4b",
            ),
        ] {
            assert!(validate_relative_library_path(&relative).is_err());
        }
        let (show, season) = derive_tv_dirs_from_pattern(
            &temp.path().to_string_lossy(),
            "{show}.{ext}",
            "Show",
            1,
            None,
        )
        .unwrap();
        assert_eq!(show, temp.path());
        assert_eq!(season, temp.path());
    }

    #[tokio::test]
    async fn import_effect_rejects_escape_without_creating_directories_or_removing_source() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        std::fs::create_dir(&root).unwrap();
        let source = temp.path().join("source.mkv");
        std::fs::write(&source, b"seeding payload").unwrap();
        let scanner = scanner();
        let library = library(&root, "{title}.{ext}");
        for target in [
            temp.path().join("outside/new.mkv"),
            root.join("../outside/new.mkv"),
        ] {
            assert!(
                scanner
                    .materialize_torrent_file(
                        &source,
                        &target,
                        &AuthUser::system_admin(),
                        &library,
                        true
                    )
                    .await
                    .is_err()
            );
            assert_eq!(std::fs::read(&source).unwrap(), b"seeding payload");
            assert!(!temp.path().join("outside").exists());
        }
        let target = root.join("Custom/Movie/file.mkv");
        scanner
            .materialize_torrent_file(&source, &target, &AuthUser::system_admin(), &library, false)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"seeding payload");
        assert_eq!(std::fs::read(&source).unwrap(), b"seeding payload");
        let other = temp.path().join("other.mkv");
        std::fs::write(&other, b"other payload").unwrap();
        assert!(
            scanner
                .materialize_torrent_file(
                    &other,
                    &target,
                    &AuthUser::system_admin(),
                    &library,
                    false
                )
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(&target).unwrap(), b"seeding payload");
        assert_eq!(std::fs::read(&other).unwrap(), b"other payload");
        assert!(
            place_in_library(
                root.clone(),
                other.clone(),
                root.join("illegal-move.mkv"),
                true,
                true
            )
            .await
            .is_err()
        );
        assert!(other.exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn symlink_parent_escape_and_postplanning_swap_fail_but_contained_aliases_work() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let outside = temp.path().join("outside");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&outside).unwrap();
        let source = temp.path().join("source.mkv");
        std::fs::write(&source, b"preserve source").unwrap();
        let scanner = scanner();
        let library = library(&root, "{title}.{ext}");
        symlink(&outside, root.join("escape")).unwrap();
        let target = root.join("escape/new/file.mkv");
        assert!(checked_library_path(&root, &target).is_err());
        assert!(create_library_directory(&root, &root.join("escape/new")).is_err());
        assert!(
            scanner
                .materialize_torrent_file(
                    &source,
                    &target,
                    &AuthUser::system_admin(),
                    &library,
                    true
                )
                .await
                .is_err()
        );
        assert!(!outside.join("new").exists());
        assert!(source.exists());
        symlink(outside.join("missing"), root.join("dangling")).unwrap();
        assert!(checked_library_path(&root, &root.join("dangling/file.mkv")).is_err());
        let planned_parent = root.join("planned");
        std::fs::create_dir(&planned_parent).unwrap();
        let planned = planned_parent.join("file.mkv");
        checked_library_path(&root, &planned).unwrap();
        std::fs::remove_dir(&planned_parent).unwrap();
        symlink(&outside, &planned_parent).unwrap();
        assert!(
            scanner
                .materialize_torrent_file(
                    &source,
                    &planned,
                    &AuthUser::system_admin(),
                    &library,
                    true
                )
                .await
                .is_err()
        );
        assert!(!outside.join("file.mkv").exists());
        let real_parent = root.join("real");
        std::fs::create_dir(&real_parent).unwrap();
        symlink(&real_parent, root.join("alias")).unwrap();
        let safe = root.join("alias/nested/file.mkv");
        scanner
            .materialize_torrent_file(&source, &safe, &AuthUser::system_admin(), &library, false)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(root.join("real/nested/file.mkv")).unwrap(),
            b"preserve source"
        );
        assert_eq!(std::fs::read(&source).unwrap(), b"preserve source");
    }
}

#[cfg(test)]
mod entity_paging_tests {
    use super::LibraryScanService;
    use graphql_orm::graphql::orm::PaginationConfig;

    /// A page size above the ORM's maximum reads one short page and stops, silently losing the rest.
    #[test]
    fn page_size_fits_what_a_query_returns() {
        let maximum = PaginationConfig::default().max_limit.expect("a maximum");
        assert!(LibraryScanService::ENTITY_PAGE_SIZE <= maximum);
    }
}

#[cfg(test)]
mod whole_book_tests {
    use super::file_covers_book;

    #[test]
    fn a_file_is_the_whole_book_only_if_it_is_about_as_long() {
        assert!(file_covers_book(34628, 34627.0));
        assert!(file_covers_book(30000, 34627.0));
        // The first of many files in a book split by chapter.
        assert!(!file_covers_book(64, 34627.0));
        // Nothing to compare it with.
        assert!(!file_covers_book(64, 0.0));
    }
}

#[cfg(test)]
mod media_file_disk_deletion_tests {
    use super::*;

    struct Library {
        _temp: tempfile::TempDir,
        root: PathBuf,
        outside: PathBuf,
    }

    fn library() -> Library {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let outside = temp.path().join("outside");
        std::fs::create_dir_all(root.join("Film (2020)")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        Library {
            _temp: temp,
            root,
            outside,
        }
    }

    fn delete(library: &Library, file: &Path) -> Result<()> {
        match deletable_media_file_path(&library.root, file)? {
            Some(target) => remove_media_file(&library.root, &target),
            None => Ok(()),
        }
    }

    #[test]
    fn deletes_the_file_and_the_folder_it_empties_but_never_the_root() {
        let library = library();
        let folder = library.root.join("Film (2020)");
        let film = folder.join("Film (2020).mkv");
        std::fs::write(&film, b"film").unwrap();
        delete(&library, &film).unwrap();
        assert!(!film.exists());
        assert!(!folder.exists());

        let loose = library.root.join("Loose.mkv");
        std::fs::write(&loose, b"film").unwrap();
        delete(&library, &loose).unwrap();
        assert!(!loose.exists());
        assert!(library.root.is_dir());
    }

    #[test]
    fn keeps_a_folder_that_still_holds_other_files() {
        let library = library();
        let folder = library.root.join("Film (2020)");
        let film = folder.join("Film (2020).mkv");
        let subtitles = folder.join("Film (2020).srt");
        std::fs::write(&film, b"film").unwrap();
        std::fs::write(&subtitles, b"subtitles").unwrap();
        delete(&library, &film).unwrap();
        assert!(!film.exists());
        assert!(subtitles.exists());
    }

    #[test]
    fn refuses_anything_outside_the_library_root() {
        let library = library();
        let outside = library.outside.join("Other.mkv");
        std::fs::write(&outside, b"other").unwrap();
        let traversal = library.root.join("..").join("outside").join("Other.mkv");
        for path in [&outside, &traversal] {
            assert!(delete(&library, path).is_err(), "{}", path.display());
        }
        assert!(outside.exists());
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_folder_symlink_that_escapes_the_root() {
        let library = library();
        let outside = library.outside.join("Other.mkv");
        std::fs::write(&outside, b"other").unwrap();
        let link = library.root.join("Escape");
        std::os::unix::fs::symlink(&library.outside, &link).unwrap();
        assert!(delete(&library, &link.join("Other.mkv")).is_err());
        assert!(outside.exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_file_loses_the_link_and_not_its_target() {
        let library = library();
        let outside = library.outside.join("Other.mkv");
        std::fs::write(&outside, b"other").unwrap();
        let link = library.root.join("Linked.mkv");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        delete(&library, &link).unwrap();
        assert!(std::fs::symlink_metadata(&link).is_err());
        assert!(outside.exists());
    }

    #[test]
    fn refuses_a_directory_and_accepts_a_file_already_gone() {
        let library = library();
        let folder = library.root.join("Film (2020)");
        assert!(deletable_media_file_path(&library.root, &folder).is_err());
        assert!(folder.is_dir());
        let gone = library.root.join("Gone.mkv");
        assert!(
            deletable_media_file_path(&library.root, &gone)
                .unwrap()
                .is_none()
        );
    }
}

/// Libraries on a network share: the path rules (always run) and the file operations against a
/// real share (only when `LIBRARIAN_SMB_TEST_URL` and `LIBRARIAN_SMB_TEST_PASSWORD_FILE` name one;
/// they work in a throwaway folder that is removed afterwards).
#[cfg(test)]
mod remote_library_tests {
    use super::*;
    use crate::services::library_storage::{
        SmbCredentials, SmbLocation, SmbStorage, StoragePath, WalkOptions,
    };

    #[test]
    fn a_share_path_is_contained_by_its_name_alone() {
        let root = Path::new("smb://nas/media/Movies");
        let inside = root.join("Film (2020)").join("Film.mkv");
        assert_eq!(checked_library_path(root, &inside).unwrap(), inside);
        assert_eq!(checked_library_path(root, root).unwrap(), root);
        // Outside the root, or climbing out of it, is refused without touching the share.
        assert!(checked_library_path(root, Path::new("smb://nas/media/Music/a.flac")).is_err());
        assert!(checked_library_path(root, Path::new("smb://nas/other/Movies/a.mkv")).is_err());
        assert!(checked_library_path(root, &root.join("..").join("Music")).is_err());
        assert!(checked_library_path(root, &root.join("a").join("..").join("..")).is_err());
        assert!(is_remote_path(root));
        assert!(!is_remote_path(Path::new("/data/media")));
    }

    #[tokio::test]
    async fn a_file_cannot_be_placed_on_a_share_inside_a_local_library() {
        let error = place_in_library(
            PathBuf::from("/data/media/Movies"),
            PathBuf::from("/downloads/a.mkv"),
            PathBuf::from("smb://nas/media/Movies/a.mkv"),
            false,
            false,
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("local library"), "{error}");
    }

    #[tokio::test]
    async fn a_file_on_an_unconnected_share_is_unavailable_not_missing() {
        // No provider knows this share: scans and deletes treat it as down, never as empty.
        let missing = library_storage()
            .exists("smb://unconnected.invalid/media/Movies/a.mkv")
            .await;
        assert!(missing.is_err(), "{missing:?}");
        // So reconciliation keeps the record (an unanswerable check counts as present).
        assert!(
            library_storage()
                .exists("smb://unconnected.invalid/media/Movies/a.mkv")
                .await
                .unwrap_or(true)
        );
    }

    fn share_from_env() -> Option<(SmbLocation, SmbCredentials)> {
        let url = std::env::var("LIBRARIAN_SMB_TEST_URL").ok()?;
        let file = std::env::var("LIBRARIAN_SMB_TEST_PASSWORD_FILE").ok()?;
        let password = std::fs::read_to_string(file)
            .ok()?
            .trim_end_matches(['\n', '\r'])
            .to_string();
        let user = std::env::var("LIBRARIAN_SMB_TEST_USER").unwrap_or_else(|_| "toby".into());
        Some((
            SmbLocation::parse(&url).ok()?,
            SmbCredentials {
                username: user,
                password,
            },
        ))
    }

    #[tokio::test]
    async fn placing_tidying_and_deleting_on_a_real_share() {
        let Some((base, credentials)) = share_from_env() else {
            eprintln!("skipped: no share named by LIBRARIAN_SMB_TEST_URL");
            return;
        };
        let storage = Arc::new(SmbStorage::new(base.clone(), credentials));
        library_storage().register_remote(&base.share_root().storage_path(), storage);
        let work = base.join(&format!("librarian-unit-{}", Uuid::new_v4().simple()));
        let root = PathBuf::from(work.to_uri());
        library_storage()
            .create_dir_all(&root)
            .await
            .expect("make the working folder");
        let local = tempfile::tempdir().unwrap();
        let source = local.path().join("download.mkv");
        std::fs::write(&source, b"the downloaded bytes").unwrap();

        let outcome = async {
            // Import: copied in, the download untouched, never replacing.
            let target = root.join("Film (2020)").join("Film (2020).mkv");
            let placed =
                place_in_library(root.clone(), source.clone(), target.clone(), false, false)
                    .await?;
            assert_eq!(placed, NoClobberOutcome::Placed);
            assert!(
                source.exists(),
                "a download that keeps seeding is left alone"
            );
            assert_eq!(
                library_storage().read_all(&target).await?,
                b"the downloaded bytes"
            );
            let again =
                place_in_library(root.clone(), source.clone(), target.clone(), false, false)
                    .await?;
            assert_eq!(again, NoClobberOutcome::TargetExists);

            // Organize: a file already on the share is renamed, not copied.
            let renamed = root.join("Renamed (2020)").join("Renamed (2020).mkv");
            let moved =
                place_in_library(root.clone(), target.clone(), renamed.clone(), true, true).await?;
            assert_eq!(moved, NoClobberOutcome::Placed);
            assert!(!library_storage().exists(&target).await?);
            assert!(library_storage().exists(&renamed).await?);
            // Not onto a file that is there.
            library_storage()
                .put_from_local(
                    &source,
                    &StoragePath::new(target.to_string_lossy()),
                    PlaceMode::NoClobber,
                )
                .await?;
            let refused =
                place_in_library(root.clone(), target.clone(), renamed.clone(), true, true).await?;
            assert_eq!(refused, NoClobberOutcome::TargetExists);
            assert!(library_storage().exists(&target).await?, "nothing was lost");

            // Tidy: the folder the move emptied goes; the one still in use stays.
            library_storage().remove_file(&target).await?;
            let removed = remove_empty_folders_upward_remote(
                &root,
                target.parent().unwrap(),
                &HashSet::new(),
            )
            .await;
            assert_eq!(removed, vec![target.parent().unwrap().to_path_buf()]);
            let kept = remove_empty_folders_upward_remote(
                &root,
                renamed.parent().unwrap(),
                &HashSet::new(),
            )
            .await;
            assert!(kept.is_empty(), "{kept:?}");

            // Delete from disk: the file, then its folder once empty; a missing file is fine.
            let doomed = deletable_media_file(&root, &renamed)
                .await?
                .expect("the file is there");
            remove_media_file_in(&root, &doomed).await?;
            assert!(!library_storage().exists(&renamed).await?);
            assert!(!library_storage().exists(renamed.parent().unwrap()).await?);
            assert!(deletable_media_file(&root, &renamed).await?.is_none());
            // The folder tidier finds nothing left to remove.
            LibraryScanService::cleanup_empty_unprotected_folders(&root, &HashSet::new()).await?;
            anyhow::Ok(())
        }
        .await;

        // Remove the working folder whatever happened.
        if let Ok(mut walk) = library_storage()
            .walk(&root, WalkOptions::default(), CancellationToken::new())
            .await
        {
            let mut folders = Vec::new();
            while let Some(Ok(entry)) = walk.entries.recv().await {
                if entry.kind == EntryKind::Directory {
                    folders.push(entry.path);
                } else {
                    let _ = library_storage().remove_file(&entry.path).await;
                }
            }
            let _ = walk.task.await;
            folders.sort_by_key(|path| std::cmp::Reverse(path.as_str().len()));
            for folder in folders {
                let _ = library_storage().remove_dir(&folder).await;
            }
        }
        outcome.unwrap();
    }
}
