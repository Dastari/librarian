use std::cell::Cell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::Read;
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
use graphql_orm::graphql::filters::StringFilter;
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
    MediaFileWhereInput, Track, TrackWhereInput, UpdateLibraryScanIssueInput,
    UpdateLibraryScanRunInput, User,
};
use crate::services::graphql::{AuthUser, LibrarianSchema};
use crate::services::manager::{Service, ServiceHealth, ServicesManager};
use crate::services::metadata::providers::{
    AddAlbumOptions, AddAudiobookOptions, AddMovieOptions, AddTvShowOptions, MetadataProvider,
    MetadataService,
};
use crate::services::ollama::{OllamaClient, OllamaParsedHint, OllamaParserSettings};
use crate::services::quality::{profile, scoring};

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

/// `match_type` recorded when the Torrent row named the exact target.
const TORRENT_LINK_MATCH_TYPE: &str = "torrent_link";

/// `match_type` recorded when the Torrent row named the *parent* (show, album,
/// audiobook) and the individual file was resolved by parsing/ordering.
const TORRENT_LINK_PARSED_MATCH_TYPE: &str = "torrent_link_parsed";

/// Video files smaller than this are treated as samples/extras when the same
/// release also contains a larger video file (design.md Q54).
const MIN_IMPORT_VIDEO_BYTES: i64 = 50 * 1024 * 1024;

/// One file offered to the importer, plus where it came from.
#[derive(Debug, Clone)]
struct ImportFile {
    file: SourceFileImport,
    /// True for files Librarian unpacked into its own staging directory. Those
    /// are our copies, not the seeding payload, so they may be *moved* into
    /// the library instead of hardlinked/copied.
    from_archive: bool,
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
}

impl TorrentImportLink {
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

/// A concrete import target derived from the recorded torrent linkage.
#[derive(Debug, Clone)]
struct LinkedImportTarget {
    library_id: String,
    target_type: String,
    target_id: String,
    target_name: Option<String>,
    match_type: &'static str,
    reason: String,
}

/// Per-file plan produced from the torrent linkage.
#[derive(Debug, Default, Clone)]
struct LinkedImportPlan {
    targets: HashMap<String, LinkedImportTarget>,
    /// Why a specific file could not be placed even though linkage exists.
    unmatched_reasons: HashMap<String, String>,
    notes: Vec<String>,
    target_label: Option<String>,
}

#[derive(Debug, Clone)]
struct LinkedEpisodeRow {
    id: String,
    season: Option<i32>,
    episode: Option<i32>,
    title: Option<String>,
}

#[derive(Debug, Clone)]
struct LinkedTrackRow {
    id: String,
    title: String,
    track_number: Option<i32>,
    disc_number: Option<i32>,
}

#[derive(Debug, Clone)]
struct LinkedChapterRow {
    id: String,
    number: i32,
    title: Option<String>,
}

#[derive(Debug, Clone)]
struct ScanJob {
    library_id: String,
    scan_run_id: String,
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
    pub trash_path: String,
}

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

pub struct LibraryScanService {
    manager: Arc<ServicesManager>,
    config: LibraryScanServiceConfig,
    runtime: RwLock<Option<Runtime>>,
    ffprobe_available: RwLock<bool>,
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

impl LibraryScanService {
    const AUTO_ORGANIZE_MOVIE_CONFIDENCE_GUARD: f64 = 0.93;
    const AUTO_ORGANIZE_MOVIE_TITLE_SIMILARITY_GUARD: f64 = 0.86;
    const SCAN_QUEUE_CAPACITY: usize = 64;
    const ANALYSIS_QUEUE_CAPACITY: usize = 256;
    const ENTITY_PAGE_SIZE: i64 = 1_000;
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
        self.execute_graphql(auth_user, mutation, variables).await
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
            let auth_user = AuthUser {
                user_id: user_id.clone(),
                email: None,
                role: Some("admin".to_string()),
            };

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
                            "userId": auth_user.user_id,
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
                        eq: Some(auth_user.user_id.clone()),
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
                warn!(user_id = %auth_user.user_id, title, error = %error, "Failed to check existing scan notification");
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
                        "userId": auth_user.user_id,
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
                user_id = %auth_user.user_id,
                error = %e,
                "Failed to create notification: user_id={}, title={}, error={}",
                auth_user.user_id,
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
                        "userId": { "eq": auth_user.user_id },
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
                    user_id = %auth_user.user_id,
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
                        "userId": auth_user.user_id,
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
                user_id = %auth_user.user_id,
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
                        "userId": { "eq": auth_user.user_id },
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
            let _ = LibraryScanIssue::update_by_id(
                database.pool(),
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
                        "userId": { "eq": auth_user.user_id },
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
                    user_id = %auth_user.user_id,
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
                        "userId": auth_user.user_id,
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
                user_id = %auth_user.user_id,
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
        notification_type: &str,
        category: &str,
        title: &str,
        message: &str,
        library_id: Option<&str>,
        media_file_id: Option<&str>,
        action_type: &str,
        action_data: &str,
    ) {
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
                        "userId": auth_user.user_id,
                        "notificationType": notification_type,
                        "category": category,
                        "title": title,
                        "message": message,
                        "libraryId": library_id,
                        "mediaFileId": media_file_id,
                        "actionType": action_type,
                        "actionData": action_data,
                    }
                }),
            )
            .await
        {
            warn!(
                user_id = %auth_user.user_id,
                error = %e,
                "Failed to create action notification: user_id={}, title={}, error={}",
                auth_user.user_id,
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

    async fn try_system_auth_user(
        &self,
        fallback_user_id: Option<&str>,
    ) -> Result<Option<AuthUser>> {
        if let Some(user_id) = fallback_user_id {
            return Ok(Some(AuthUser {
                user_id: user_id.to_string(),
                email: None,
                role: Some("admin".to_string()),
            }));
        }

        let bootstrap_auth = AuthUser {
            user_id: "system-bootstrap".to_string(),
            email: None,
            role: Some("admin".to_string()),
        };
        if let Ok(data) = self
            .execute_graphql(
                &bootstrap_auth,
                r#"query FirstUserForBootstrap {
                    Users: users(page: { limit: 1 }) {
                        Edges: edges { Node: node { Id: id } }
                    }
                }"#,
                serde_json::json!({}),
            )
            .await
            && let Some(user_id) = data
                .get("Users")
                .and_then(|v| v.get("Edges"))
                .and_then(|v| v.as_array())
                .and_then(|edges| edges.first())
                .and_then(|edge| edge.get("Node"))
                .and_then(|node| node.get("Id"))
                .and_then(|v| v.as_str())
        {
            return Ok(Some(AuthUser {
                user_id: user_id.to_string(),
                email: None,
                role: Some("admin".to_string()),
            }));
        }

        // Bootstrap fallback only: GraphQL needs an AuthUser, so we query users directly if bootstrap auth fails.
        let db_svc = self
            .manager
            .get_database()
            .await
            .ok_or_else(|| anyhow::anyhow!("Database service not available"))?;

        let user_id = User::query(db_svc.pool().pool())
            .fetch_all()
            .await?
            .into_iter()
            .min_by(|a, b| a.created_at.cmp(&b.created_at))
            .map(|user| user.id);

        Ok(user_id.map(|user_id| AuthUser {
            user_id,
            email: None,
            role: Some("admin".to_string()),
        }))
    }

    async fn system_auth_user(&self, fallback_user_id: Option<&str>) -> Result<AuthUser> {
        self.try_system_auth_user(fallback_user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No user exists to run library scan operations"))
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

        Ok(patterns
            .first()
            .map(|(pattern, _, _)| pattern.clone())
            .filter(|pattern| !pattern.trim().is_empty())
            .unwrap_or_else(|| Self::fallback_naming_pattern(&normalized_type).to_string()))
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
                        where: { autoScan: { eq: true }, scanning: { eq: false } }
                        page: $page
                    ) {
                        Edges: edges {
                            Node: node {
                                Id: id
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
            if interval_minutes <= 0 {
                continue;
            }

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

        Ok(())
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

    fn discover_media_files(
        root: PathBuf,
        allowed_extensions: &'static [&'static str],
        cancel: CancellationToken,
    ) -> (
        mpsc::Receiver<std::result::Result<DiscoveredMediaFile, String>>,
        JoinHandle<()>,
    ) {
        let (tx, rx) = mpsc::channel(128);
        let handle = tokio::task::spawn_blocking(move || {
            for result in WalkDir::new(&root) {
                if cancel.is_cancelled() {
                    break;
                }
                let entry = match result {
                    Ok(entry) => entry,
                    Err(error) => {
                        if tx
                            .blocking_send(Err(format!("Unable to read library entry: {error}")))
                            .is_err()
                        {
                            break;
                        }
                        continue;
                    }
                };
                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();
                let file_name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                if file_name.starts_with('.') || file_name.starts_with("._") {
                    continue;
                }
                let extension = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .map(str::to_ascii_lowercase)
                    .unwrap_or_default();
                if !allowed_extensions
                    .iter()
                    .any(|allowed| *allowed == extension)
                {
                    continue;
                }

                let metadata = entry.metadata().ok();
                let discovered = DiscoveredMediaFile {
                    absolute_path: path.to_string_lossy().to_string(),
                    relative_path: path
                        .strip_prefix(&root)
                        .ok()
                        .map(|relative| relative.to_string_lossy().to_string()),
                    extension,
                    size: metadata
                        .as_ref()
                        .and_then(|metadata| i64::try_from(metadata.len()).ok())
                        .unwrap_or(i64::MAX),
                    modified_at: metadata
                        .and_then(|metadata| metadata.modified().ok())
                        .map(DateTime::<Utc>::from)
                        .map(|value| value.to_rfc3339_opts(SecondsFormat::Nanos, true)),
                };
                if tx.blocking_send(Ok(discovered)).is_err() {
                    break;
                }
            }
        });
        (rx, handle)
    }

    async fn hash_duplicate_candidate(
        candidate: DuplicateCandidate,
        cancel: CancellationToken,
    ) -> Result<(DuplicateCandidate, String)> {
        tokio::task::spawn_blocking(move || {
            let mut file = std::fs::File::open(&candidate.path)
                .with_context(|| "Unable to open duplicate candidate")?;
            let mut digest = Sha256::new();
            let mut buffer = vec![0_u8; 1024 * 1024];
            loop {
                if cancel.is_cancelled() {
                    anyhow::bail!("SCAN_CANCELLED");
                }
                let read = file
                    .read(&mut buffer)
                    .with_context(|| "Unable to read duplicate candidate")?;
                if read == 0 {
                    break;
                }
                digest.update(&buffer[..read]);
            }
            Ok((candidate, hex::encode(digest.finalize())))
        })
        .await
        .context("Duplicate hash worker failed")?
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

            for (digest, identical) in by_hash.into_iter().filter(|(_, group)| group.len() > 1) {
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
                        "qualityStatus": null
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

        let library = self.get_library(library_id).await?;
        if let Err(error) = self.compact_scan_history(library_id).await {
            warn!(
                library_id,
                error = %error,
                "Failed to compact old library scan history"
            );
        }
        let scan_run = self.create_scan_run(&library).await?;
        if let Err(error) = self
            .scan_tx
            .send(ScanJob {
                library_id: library_id.to_string(),
                scan_run_id: scan_run.id.clone(),
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
            let issues = LibraryScanIssue::query(database.pool().pool())
                .filter(LibraryScanIssueWhereInput {
                    scan_run_id: Some(StringFilter {
                        eq: Some(run.id.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .fetch_all()
                .await
                .context("Failed to read old scan issues for compaction")?;
            for issue in issues {
                LibraryScanIssue::delete_by_id(database.pool(), &issue.id)
                    .await
                    .context("Failed to delete compacted scan issue")?;
            }
            LibraryScanRun::delete_by_id(database.pool(), &run.id)
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
                LibraryScanIssue::delete_by_id(database.pool(), &issue.id)
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
        LibraryScanIssue::update_by_id(database.pool(), &issue.id, input)
            .await
            .context("Failed to persist analysis retry outcome")?;
        Ok(())
    }

    pub async fn resolve_scan_issue(&self, issue_id: &str, resolution: &str) -> Result<()> {
        let resolution = resolution.trim();
        if resolution.is_empty() {
            anyhow::bail!("A resolution note is required");
        }
        let database = self.database().await?;
        let issue = LibraryScanIssue::get(database.pool().pool(), &issue_id.to_string())
            .await
            .context("Failed to read scan issue")?
            .context("Scan issue not found")?;
        if issue.resolved_at.is_some() {
            return Ok(());
        }
        LibraryScanIssue::update_by_id(
            database.pool(),
            &issue.id,
            UpdateLibraryScanIssueInput {
                resolved_at: Some(Some(Utc::now().to_rfc3339())),
                resolution: Some(Some(Self::bounded_issue_text(resolution, 1_000))),
                ..Default::default()
            },
        )
        .await
        .context("Failed to resolve scan issue")?;
        Ok(())
    }

    pub async fn trash_duplicate_issue(&self, issue_id: &str) -> Result<DuplicateTrashResult> {
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
        let media_file_id = issue
            .media_file_id
            .as_deref()
            .context("Duplicate issue is not linked to a media file")?;
        let media_file = MediaFile::get(database.pool().pool(), &media_file_id.to_string())
            .await
            .context("Failed to read duplicate media file")?
            .context("Duplicate media file no longer exists")?;
        if media_file.library_id.as_deref() != Some(issue.library_id.as_str())
            || media_file.path != details.duplicate_path
            || media_file.size != details.size
        {
            anyhow::bail!("Duplicate file changed since it was verified; rescan before removal");
        }
        let library = self.get_library(&issue.library_id).await?;
        let root = tokio::fs::canonicalize(&library.path)
            .await
            .context("Library root is unavailable")?;
        let duplicate_path = tokio::fs::canonicalize(&details.duplicate_path)
            .await
            .context("Duplicate path is unavailable")?;
        let keeper_path = tokio::fs::canonicalize(&details.keeper_path)
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

        let trash_dir = root.join(".librarian-trash");
        tokio::fs::create_dir_all(&trash_dir)
            .await
            .context("Failed to create the library trash directory")?;
        let original_name = duplicate_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("media");
        let trash_path = trash_dir.join(format!("{}-{original_name}", Uuid::new_v4()));
        tokio::fs::rename(&duplicate_path, &trash_path)
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
            if let Err(restore_error) = tokio::fs::rename(&trash_path, &duplicate_path).await {
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
        self.resolve_scan_issue(
            issue_id,
            &format!("Moved duplicate to recoverable trash at '{trash_path_string}'."),
        )
        .await?;
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
            trash_path: trash_path_string,
        })
    }

    async fn database(&self) -> Result<Arc<crate::services::DatabaseService>> {
        self.manager
            .get_database()
            .await
            .ok_or_else(|| anyhow::anyhow!("Database service not available"))
    }

    async fn create_scan_run(&self, library: &LibraryRow) -> Result<LibraryScanRun> {
        let database = self.database().await?;
        LibraryScanRun::insert(
            database.pool(),
            CreateLibraryScanRunInput {
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
                summary: Some(format!("Scan queued for {}", library.name)),
                error_code: None,
            },
        )
        .await
        .context("Failed to create durable library scan run")
    }

    async fn update_scan_run(
        &self,
        scan_run_id: &str,
        input: UpdateLibraryScanRunInput,
    ) -> Result<LibraryScanRun> {
        let database = self.database().await?;
        LibraryScanRun::update_by_id(database.pool(), &scan_run_id.to_string(), input)
            .await
            .context("Failed to update durable library scan run")?
            .ok_or_else(|| anyhow::anyhow!("Library scan run no longer exists"))
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
            LibraryScanIssue::update_by_id(
                database.pool(),
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

        // Keep each scan's audit record, but do not re-alert an acknowledged,
        // unchanged problem merely because another scheduled scan found it.
        let previous = LibraryScanIssue::query(database.pool().pool())
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
            .max_by_key(|issue| {
                issue.created_at.parse::<i64>().unwrap_or_else(|_| {
                    DateTime::parse_from_rfc3339(&issue.created_at)
                        .map(|date| date.timestamp())
                        .unwrap_or_default()
                })
            });
        let read_at = previous
            .filter(|issue| {
                issue.resolved_at.is_none()
                    && issue.message == Self::bounded_issue_text(message, 1_000)
            })
            .and_then(|issue| issue.read_at);

        LibraryScanIssue::insert(
            database.pool(),
            CreateLibraryScanIssueInput {
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
            },
        )
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
        LibraryScanRun::update_by_id(database.pool(), &run.id, input)
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
        cancel: CancellationToken,
    ) -> Result<()> {
        let scan_started = Instant::now();
        let library = self.get_library(library_id).await?;
        let normalized_library_type = Self::normalize_library_type(&library.library_type);

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
                summary: Some(Some(format!("Scanning {}", library.name))),
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
            "Starting library scan: library_id={}, name={}, type={}, path={}, auto_organize={}, scan_interval_minutes={}",
            library_id,
            library.name,
            library.library_type,
            library.path,
            library.auto_organize,
            library.scan_interval_minutes
        );

        let root = PathBuf::from(&library.path);
        if !root.exists() {
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

        let allowed_ext = Self::extensions_for_library(&normalized_library_type);
        let mut scanned_count: usize = 0;
        let mut match_pipeline_error_count: usize = 0;
        let mut existing_count: usize = 0;
        let mut matched_count: usize = 0;
        let mut analysis_queued_count: usize = 0;
        let mut unchanged_skipped_count: usize = 0;
        let mut run_analysis_ids: Vec<String> = Vec::new();
        let mut discovered_paths: HashSet<String> = HashSet::new();
        let mut pending_provider_fallback: Vec<PendingProviderFallback> = Vec::new();
        let mut duplicate_candidates: Vec<DuplicateCandidate> = Vec::new();
        let mut media_files_by_path = self.load_media_files_by_path(library_id).await?;
        let reconciliation_rows = media_files_by_path
            .values()
            .map(|file| ExistingMediaFileRow {
                id: file.id.clone(),
                path: file.path.clone(),
            })
            .collect::<Vec<_>>();

        let (mut discovered_rx, discovery_handle) =
            Self::discover_media_files(root.clone(), allowed_ext, cancel.child_token());
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
                Err(message) => {
                    warn!(
                        library_id = %library_id,
                        library_name = %library.name,
                        library_path = %library.path,
                        error = %message,
                        "Library traversal could not read an entry"
                    );
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
                    continue;
                }
            };

            let abs_path = discovered.absolute_path;
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
                debug!(
                    library_id = %library_id,
                    media_file_id = %media_file_id,
                    media_file_path = %abs_path,
                    "Skipped match/provider work for an unchanged, already-linked media file"
                );
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
        self.reconcile_missing_media_files(&auth_user, &discovered_paths, &reconciliation_rows)
            .await?;

        self.reconcile_movie_collections_for_library(&library).await;

        if Self::is_tv_library_type(&normalized_library_type) {
            self.ensure_tv_folder_structure(&library).await?;
        }

        if library.auto_organize {
            self.cleanup_empty_folders(&library).await?;
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

        self.set_library_scanning(&auth_user, library_id, false, true)
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
                summary: Some(Some(if completed_with_issues {
                    format!(
                        "Scan completed with issues: {} discovered, {} matched, {} unmatched, {} analysis failures, {} byte-identical duplicates",
                        scanned_count,
                        matched_count,
                        provider_summary.unmatched_files,
                        analysis_failed,
                        duplicate_count
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
                        cancel.child_token(),
                    ),
                )
                .await
            {
                error!(
                    library_id = %job.library_id,
                    scan_run_id = %job.scan_run_id,
                    error = %e,
                    "Library scan failed: library_id={}, error={}",
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
        let auth_user = self.system_auth_user(None).await?;
        let analysis = ffprobe_analyze(path).await?;

        let data = self
            .execute_mutation(
                &auth_user,
                r#"mutation UpdateMediaFileFromAnalysis($id: String!, $input: UpdateMediaFileInput!) {
                    UpdateMediaFile: updateMediaFile(id: $id, input: $input) {
                        Success: success
                        Error: error
                    }
                }"#,
                serde_json::json!({
                    "id": media_file_id,
                    "input": {
                        "container": analysis.container,
                        "videoCodec": analysis.video_codec,
                        "audioCodec": analysis.audio_codec,
                        "width": analysis.width,
                        "height": analysis.height,
                        "duration": analysis.duration,
                        "bitrate": analysis.bitrate,
                        "resolution": analysis.resolution,
                        "isHdr": analysis.is_hdr,
                        "hdrType": analysis.hdr_type,
                        "audioChannels": analysis.audio_channels,
                        "metadata": analysis.metadata,
                        "analyzedAt": Utc::now().to_rfc3339(),
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
                .unwrap_or("failed to update media file from analysis");
            anyhow::bail!(err.to_string());
        }

        self.persist_media_analysis_details(&auth_user, media_file_id, &analysis)
            .await?;

        // design.md Q23/Q24: ffprobe is authoritative for quality, so
        // (re)compute `quality_status` against the media file's resolved
        // `QualityProfile` right after analysis writes the verified
        // resolution/codec/HDR/audio columns. Best-effort: a failure here
        // (e.g. no profile seeded yet) must not fail the analysis job itself
        // (Q49 — ffprobe failures/quality gaps are logged, not fatal).
        if let Some(db_service) = self.manager.get_database().await {
            let db = db_service.pool().clone();
            if let Err(e) = profile::recompute_and_persist(&db, media_file_id).await {
                warn!(
                    media_file_id = %media_file_id,
                    error = %e,
                    "Failed to compute quality_status after analysis"
                );
            }
        }

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

    async fn persist_media_analysis_details(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        analysis: &ProbeAnalysis,
    ) -> Result<()> {
        let cleared = self
            .execute_mutation(
                auth_user,
                r#"mutation ClearMediaFileAnalysisDetails($mediaFileId: String!) {
                    DeleteVideoStreams: deleteVideoStreams(where: { mediaFileId: { eq: $mediaFileId } }) { success error DeletedCount: deletedCount }
                    DeleteAudioStreams: deleteAudioStreams(where: { mediaFileId: { eq: $mediaFileId } }) { success error DeletedCount: deletedCount }
                    DeleteSubtitles: deleteSubtitles(where: { mediaFileId: { eq: $mediaFileId } }) { success error DeletedCount: deletedCount }
                    DeleteMediaChapters: deleteMediaChapters(where: { mediaFileId: { eq: $mediaFileId } }) { success error DeletedCount: deletedCount }
                }"#,
                serde_json::json!({ "mediaFileId": media_file_id }),
            )
            .await?;

        for (op, label) in [
            ("DeleteVideoStreams", "video streams"),
            ("DeleteAudioStreams", "audio streams"),
            ("DeleteSubtitles", "subtitles"),
            ("DeleteMediaChapters", "media chapters"),
        ] {
            let ok = cleared
                .get(op)
                .and_then(|v| v.get("success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !ok {
                let err = cleared
                    .get(op)
                    .and_then(|v| v.get("error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("bulk delete failed");
                anyhow::bail!(
                    "failed clearing {} for media_file_id={}: {}",
                    label,
                    media_file_id,
                    err
                );
            }
        }

        for stream in &analysis.video_streams {
            let created = self
                .execute_mutation(
                    auth_user,
                    r#"mutation CreateVideoStreamFromAnalysis($input: CreateVideoStreamInput!) {
                        CreateVideoStream: createVideoStream(input: $input) { Success: success Error: error }
                    }"#,
                    serde_json::json!({
                        "input": {
                            "mediaFileId": media_file_id,
                            "streamIndex": stream.stream_index,
                            "codec": stream.codec,
                            "codecLongName": stream.codec_long_name,
                            "width": stream.width,
                            "height": stream.height,
                            "aspectRatio": stream.aspect_ratio,
                            "frameRate": stream.frame_rate,
                            "avgFrameRate": stream.avg_frame_rate,
                            "bitrate": stream.bitrate,
                            "pixelFormat": stream.pixel_format,
                            "colorSpace": stream.color_space,
                            "colorTransfer": stream.color_transfer,
                            "colorPrimaries": stream.color_primaries,
                            "hdrType": stream.hdr_type,
                            "bitDepth": stream.bit_depth,
                            "language": stream.language,
                            "title": stream.title,
                            "isDefault": stream.is_default,
                            "metadata": stream.metadata,
                        }
                    }),
                )
                .await?;
            let ok = created
                .get("CreateVideoStream")
                .and_then(|v| v.get("Success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !ok {
                let err = created
                    .get("CreateVideoStream")
                    .and_then(|v| v.get("Error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("create video stream failed");
                anyhow::bail!(
                    "failed creating video stream: media_file_id={}, stream_index={}, error={}",
                    media_file_id,
                    stream.stream_index,
                    err
                );
            }
        }

        for stream in &analysis.audio_streams {
            let created = self
                .execute_mutation(
                    auth_user,
                    r#"mutation CreateAudioStreamFromAnalysis($input: CreateAudioStreamInput!) {
                        CreateAudioStream: createAudioStream(input: $input) { Success: success Error: error }
                    }"#,
                    serde_json::json!({
                        "input": {
                            "mediaFileId": media_file_id,
                            "streamIndex": stream.stream_index,
                            "codec": stream.codec,
                            "codecLongName": stream.codec_long_name,
                            "channels": stream.channels,
                            "channelLayout": stream.channel_layout,
                            "sampleRate": stream.sample_rate,
                            "bitrate": stream.bitrate,
                            "bitDepth": stream.bit_depth,
                            "language": stream.language,
                            "title": stream.title,
                            "isDefault": stream.is_default,
                            "isCommentary": stream.is_commentary,
                            "metadata": stream.metadata,
                        }
                    }),
                )
                .await?;
            let ok = created
                .get("CreateAudioStream")
                .and_then(|v| v.get("Success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !ok {
                let err = created
                    .get("CreateAudioStream")
                    .and_then(|v| v.get("Error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("create audio stream failed");
                anyhow::bail!(
                    "failed creating audio stream: media_file_id={}, stream_index={}, error={}",
                    media_file_id,
                    stream.stream_index,
                    err
                );
            }
        }

        for subtitle in &analysis.subtitles {
            let created = self
                .execute_mutation(
                    auth_user,
                    r#"mutation CreateSubtitleFromAnalysis($input: CreateSubtitleInput!) {
                        CreateSubtitle: createSubtitle(input: $input) { Success: success Error: error }
                    }"#,
                    serde_json::json!({
                        "input": {
                            "mediaFileId": media_file_id,
                            "sourceType": subtitle.source_type,
                            "streamIndex": subtitle.stream_index,
                            "codec": subtitle.codec,
                            "codecLongName": subtitle.codec_long_name,
                            "language": subtitle.language,
                            "title": subtitle.title,
                            "isDefault": subtitle.is_default,
                            "isForced": subtitle.is_forced,
                            "isHearingImpaired": subtitle.is_hearing_impaired,
                            "metadata": subtitle.metadata,
                        }
                    }),
                )
                .await?;
            let ok = created
                .get("CreateSubtitle")
                .and_then(|v| v.get("Success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !ok {
                let err = created
                    .get("CreateSubtitle")
                    .and_then(|v| v.get("Error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("create subtitle failed");
                anyhow::bail!(
                    "failed creating subtitle: media_file_id={}, stream_index={:?}, error={}",
                    media_file_id,
                    subtitle.stream_index,
                    err
                );
            }
        }

        for chapter in &analysis.chapters {
            let created = self
                .execute_mutation(
                    auth_user,
                    r#"mutation CreateMediaChapterFromAnalysis($input: CreateMediaChapterInput!) {
                        CreateMediaChapter: createMediaChapter(input: $input) { Success: success Error: error }
                    }"#,
                    serde_json::json!({
                        "input": {
                            "mediaFileId": media_file_id,
                            "chapterIndex": chapter.chapter_index,
                            "startSecs": chapter.start_secs,
                            "endSecs": chapter.end_secs,
                            "title": chapter.title,
                        }
                    }),
                )
                .await?;
            let ok = created
                .get("CreateMediaChapter")
                .and_then(|v| v.get("Success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !ok {
                let err = created
                    .get("CreateMediaChapter")
                    .and_then(|v| v.get("Error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("create media chapter failed");
                anyhow::bail!(
                    "failed creating media chapter: media_file_id={}, chapter_index={}, error={}",
                    media_file_id,
                    chapter.chapter_index,
                    err
                );
            }
        }

        info!(
            media_file_id = %media_file_id,
            video_stream_count = analysis.video_streams.len(),
            audio_stream_count = analysis.audio_streams.len(),
            subtitle_count = analysis.subtitles.len(),
            chapter_count = analysis.chapters.len(),
            "Persisted detailed ffprobe rows: media_file_id={}, video_streams={}, audio_streams={}, subtitles={}, chapters={}",
            media_file_id,
            analysis.video_streams.len(),
            analysis.audio_streams.len(),
            analysis.subtitles.len(),
            analysis.chapters.len()
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

        let organize_result = self.organize_media_file(media_file_id).await;
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
            "music" | "audiobooks" => {
                let fallback_kind = normalized_library_type.clone();
                let outcomes = stream::iter(pending)
                    .map(|item| {
                        let fallback_kind = fallback_kind.clone();
                        async move {
                            let mut outcome = ProviderFallbackSummary::default();
                            let result = match fallback_kind.as_str() {
                                "music" => {
                                    self.try_provider_create_and_match_track(
                                        library,
                                        auth_user,
                                        &item.media_file_id,
                                        &item.media_path,
                                        &item.match_source,
                                    )
                                    .await
                                }
                                _ => {
                                    self.try_provider_create_and_match_chapter(
                                        library,
                                        auth_user,
                                        &item.media_file_id,
                                        &item.media_path,
                                        &item.match_source,
                                    )
                                    .await
                                }
                            };
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
        let user_uuid = Uuid::parse_str(&auth_user.user_id)?;
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
        let user_uuid = Uuid::parse_str(&auth_user.user_id)?;
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

    async fn create_minimal_track_for_album(
        &self,
        auth_user: &AuthUser,
        library_id: &str,
        album_id: &str,
        hint: &ParsedTrackHint,
    ) -> Result<String> {
        let track_number = hint.track_number.unwrap_or(1).max(1);
        let title = hint
            .title
            .clone()
            .unwrap_or_else(|| format!("Track: track {}", track_number));
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation CreateTrackFromScan($input: CreateTrackInput!) {
                    CreateTrack: createTrack(input: $input) { Success: success Error: error Track: track { Id: id } }
                }"#,
                serde_json::json!({
                    "input": {
                        "albumId": album_id,
                        "libraryId": library_id,
                        "title": title,
                        "trackNumber": track_number,
                        "discNumber": 1,
                        "explicit": false,
                        "wanted": true
                    }
                }),
            )
            .await?;

        let success = data
            .get("CreateTrack")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !success {
            let err = data
                .get("CreateTrack")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("failed to create fallback track");
            anyhow::bail!(err.to_string());
        }

        data.get("CreateTrack")
            .and_then(|v| v.get("Track"))
            .and_then(|v| v.get("Id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("CreateTrack did not return Track.Id"))
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
        let hint = Self::parse_track_hint(match_source);
        let query = hint
            .album_name
            .clone()
            .or_else(|| hint.title.clone())
            .filter(|q| q.len() >= 2);
        let Some(query) = query else {
            return Ok(None);
        };

        let metadata = self.metadata_service().await?;
        let candidates = metadata
            .search_albums(&query, true, true, true, true, true)
            .await
            .unwrap_or_default();
        let wanted_album_norm = hint
            .album_name
            .as_deref()
            .map(Self::normalize_for_match)
            .unwrap_or_else(|| Self::normalize_for_match(&query));
        let wanted_artist_norm = hint.artist_name.as_deref().map(Self::normalize_for_match);

        let score_candidate = |title: &str, artist: Option<&str>| -> f64 {
            let mut s = jaro_winkler(&wanted_album_norm, &Self::normalize_for_match(title));
            if let (Some(wanted_artist), Some(candidate_artist)) =
                (wanted_artist_norm.as_deref(), artist)
            {
                s += (jaro_winkler(wanted_artist, &Self::normalize_for_match(candidate_artist))
                    * 0.25)
                    .min(0.25);
            }
            s
        };
        let best = candidates
            .into_iter()
            .map(|candidate| {
                let score = score_candidate(&candidate.title, candidate.artist_name.as_deref());
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
                "Album provider candidate rejected due to low confidence: media_file_id={}, library_id={}, path={}, provider_id={}, provider_title={}, candidate_score={:.3}",
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
        let user_uuid = Uuid::parse_str(&auth_user.user_id)?;
        let album = metadata
            .add_album_from_provider(AddAlbumOptions {
                provider: MetadataProvider::Musicbrainz,
                provider_id: candidate.provider_id,
                library_id: library_uuid,
                user_id: user_uuid,
                // Scanner-discovered albums are not monitored: the files are
                // already on disk, there is nothing to hunt for.
                monitor_type: AutoDownloadMode::None,
            })
            .await?;

        if let Some((track_id, score)) = self.find_track_match(&library.id, media_path).await? {
            self.link_track(
                auth_user,
                media_file_id,
                &library.id,
                &track_id,
                "auto",
                None,
            )
            .await?;
            return Ok(Some(MatchResult {
                success: true,
                auto_matched: true,
                already_matched: false,
                matched_type: Some("Track".to_string()),
                matched_id: Some(track_id),
                confidence: score,
                reason: Some("Created album from metadata provider and matched track".to_string()),
                candidates: Vec::new(),
            }));
        }

        let track_id = self
            .create_minimal_track_for_album(auth_user, &library.id, &album.id, &hint)
            .await?;
        self.link_track(
            auth_user,
            media_file_id,
            &library.id,
            &track_id,
            "auto",
            None,
        )
        .await?;
        let confidence = 0.7;
        info!(
            media_file_id = %media_file_id,
            library_id = %library.id,
            media_path = %media_path,
            album_id = %album.id,
            album_name = %album.name,
            track_id = %track_id,
            confidence,
            "Scan fallback created album and fallback track link: media_file_id={}, library_id={}, path={}, album_id={}, album_name={}, track_id={}, confidence={:.3}",
            media_file_id,
            library.id,
            media_path,
            album.id,
            album.name,
            track_id,
            confidence
        );
        Ok(Some(MatchResult {
            success: true,
            auto_matched: true,
            already_matched: false,
            matched_type: Some("Track".to_string()),
            matched_id: Some(track_id),
            confidence,
            reason: Some("Created album and fallback track during scan".to_string()),
            candidates: Vec::new(),
        }))
    }

    async fn try_provider_create_and_match_chapter(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        media_file_id: &str,
        media_path: &str,
        match_source: &str,
    ) -> Result<Option<MatchResult>> {
        let hint = Self::parse_chapter_hint(match_source);
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
        let user_uuid = Uuid::parse_str(&auth_user.user_id)?;
        let audiobook = metadata
            .add_audiobook_from_provider(AddAudiobookOptions {
                provider: MetadataProvider::OpenLibrary,
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

    async fn find_track_match(
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

        let data = self
            .execute_graphql_paged(
                &auth_user,
                r#"query FindTrackMatch($libraryId: String!, $page: PageInput) {
                    Albums: albums(
                        where: { libraryId: { eq: $libraryId } }
                        page: $page
                    ) {
                        Edges: edges {
                            Node: node {
                                Name: name
                                Tracks: tracks(page: { limit: 1000 }) {
                                    Edges: edges { Node: node { Id: id Title: title } }
                                }
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "libraryId": library_id }),
                "Albums",
            )
            .await?;
        let rows: Vec<(String, String, Option<String>)> = data
            .get("Albums")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .flat_map(|edge| {
                let node = edge.get("Node")?;
                let album_name = node
                    .get("Name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let tracks = node
                    .get("Tracks")
                    .and_then(|v| v.get("Edges"))
                    .and_then(|v| v.as_array())?;
                Some(
                    tracks
                        .iter()
                        .filter_map(move |track_edge| {
                            let track = track_edge.get("Node")?;
                            Some((
                                track.get("Id")?.as_str()?.to_string(),
                                track.get("Title")?.as_str()?.to_string(),
                                album_name.clone(),
                            ))
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect();

        let mut best: Option<(String, f64)> = None;
        for (id, title, album) in rows {
            let title_norm = Self::normalize_for_match(&title);
            let album_norm = album
                .as_deref()
                .map(Self::normalize_for_match)
                .unwrap_or_default();
            let mut score = jaro_winkler(&normalized, &title_norm);
            if !album_norm.is_empty() && normalized.contains(&album_norm) {
                score += 0.1;
            }
            if score > best.as_ref().map(|(_, s)| *s).unwrap_or(0.0) {
                best = Some((id, score.min(1.0)));
            }
        }

        Ok(best.filter(|(_, s)| *s >= 0.7))
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
                apply_movie_naming_pattern(&naming_pattern, title, year, original_filename, ext)
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
                apply_tv_naming_pattern(
                    &naming_pattern,
                    show_name,
                    season as i32,
                    episode as i32,
                    episode_title,
                    ext,
                )
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
                                ArtistName: artistName
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
                            Album: album(id: $id) { Name: name Year: year }
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
                let artist_name = track
                    .and_then(|t| t.get("ArtistName"))
                    .and_then(|v| v.as_str());
                apply_music_naming_pattern(
                    &naming_pattern,
                    artist_name.unwrap_or("Unknown Artist"),
                    album_name,
                    album_year,
                    track_number as i32,
                    disc_number,
                    track_title,
                    original_filename,
                    ext,
                )
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
                apply_audiobook_naming_pattern(
                    &naming_pattern,
                    author_name.unwrap_or("Unknown Author"),
                    book_title,
                    chapter_number as i32,
                    chapter_title,
                    original_filename,
                    ext,
                )
            }
            _ => anyhow::bail!("Unsupported library type/target for organizing"),
        };

        Ok(PathBuf::from(&library.path).join(relative_path))
    }

    pub async fn organize_media_file(&self, media_file_id: &str) -> Result<OrganizeResult> {
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
        let auth_user = self.system_auth_user(Some(&library.user_id)).await?;

        let old_path = PathBuf::from(&media_file.path);
        if !old_path.exists() {
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

        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Atomic no-clobber move (audit #4): `hard_link` + `remove` instead of the
        // previous `exists()`-then-`rename()`, which races under concurrent
        // organize/import and can silently replace an existing target file.
        let move_outcome = no_clobber_place(old_path.clone(), target_path.clone(), true).await?;

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
            self.create_notification(
                &auth_user,
                "WARNING",
                "ORGANIZATION",
                "Organize conflict skipped",
                &message,
            )
            .await;

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

            match no_clobber_place(target_path.clone(), old_path.clone(), true).await {
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

        Ok(OrganizeResult {
            success: true,
            old_path: Some(old_path.to_string_lossy().to_string()),
            new_path: Some(target_path.to_string_lossy().to_string()),
            reason: None,
        })
    }

    pub async fn process_torrent_source_files(
        &self,
        auth_user: &AuthUser,
        info_hash: &str,
        library_override: Option<String>,
        files: Vec<SourceFileImport>,
    ) -> SourceProcessSummary {
        match self
            .process_torrent_source_files_inner(auth_user, info_hash, library_override, files)
            .await
        {
            Ok(summary) => summary,
            Err(error) => {
                let message = error.to_string();
                if let Err(status_error) = self
                    .update_torrent_post_process_status(info_hash, "failed", Some(&message), 0, 1)
                    .await
                {
                    warn!(
                        info_hash = %info_hash,
                        error = %status_error,
                        "Failed to mark torrent post-processing failure: info_hash={}, error={}",
                        info_hash,
                        status_error
                    );
                }
                SourceProcessSummary {
                    success: false,
                    files_processed: 0,
                    files_failed: 1,
                    messages: vec![message.clone()],
                    error: Some(message),
                }
            }
        }
    }

    async fn process_torrent_source_files_inner(
        &self,
        auth_user: &AuthUser,
        info_hash: &str,
        library_override: Option<String>,
        files: Vec<SourceFileImport>,
    ) -> Result<SourceProcessSummary> {
        let link = self.load_torrent_import_link(auth_user, info_hash).await?;
        let torrent_library_id = match library_override {
            Some(id) => Some(id),
            None => link.library_id.clone(),
        };

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

        let mut import_files: Vec<ImportFile> = media
            .into_iter()
            .map(|file| ImportFile {
                file,
                from_archive: false,
            })
            .collect();
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
                let staging_root = Self::extraction_staging_root(&archive_path, info_hash);
                match extract::extract_archive(&archive_path, &staging_root, limits).await {
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
                                from_archive: true,
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
        let linked = if link.has_target() {
            match self
                .plan_linked_import_targets(auth_user, &link, &import_files)
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
        diagnostics.notes.extend(linked.notes.iter().cloned());
        diagnostics.target_label = linked.target_label.clone();

        let mut library_cache: HashMap<String, LibraryRow> = HashMap::new();

        for import in &import_files {
            let file = &import.file;
            if file.file_size <= 0 || file.downloaded_bytes < file.file_size {
                continue;
            }

            let media_file_id = match self.ensure_source_media_file(auth_user, file).await {
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
                Ok(existing) if existing.match_type.as_deref() == Some("manual") => {
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

            let selected = if let Some(target) = linked.targets.get(&file.source_path) {
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
                    .candidate_libraries_for_import(auth_user, torrent_library_id.as_deref(), file)
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

            let source_path = PathBuf::from(&file.source_path);
            let target_path = match self
                .plan_media_target_path(
                    auth_user,
                    &selected.library,
                    &selected.candidate.target_type,
                    &selected.candidate.target_id,
                    &source_path,
                )
                .await
            {
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

            // Files unpacked from an archive are Librarian's own temporary
            // copies in the staging directory, not the seeding payload, so
            // moving them into the library is both safe and preferable (no
            // duplicate disk usage). Plain torrent files are never moved.
            if let Err(error) = self
                .materialize_torrent_file(
                    &source_path,
                    &target_path,
                    auth_user,
                    &selected.library,
                    import.from_archive,
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

            self.upsert_pending_file_match(
                auth_user,
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

        // Staging directories only ever hold copies we made; once the files
        // have been imported (or durably recorded as failures) they are dead
        // weight. The archive itself is never touched so seeding continues.
        for staging_dir in &staging_dirs {
            extract::cleanup_staging_dir(staging_dir).await;
        }

        let status = if processed > 0 && failed == 0 {
            "completed"
        } else if processed > 0 {
            "partial"
        } else {
            "unmatched"
        };
        let error = Self::summarize_import_outcome(&diagnostics, processed, failed);
        self.update_torrent_post_process_status(
            info_hash,
            status,
            error.as_deref(),
            processed,
            failed,
        )
        .await?;

        Ok(SourceProcessSummary {
            success: failed == 0 && processed > 0,
            files_processed: processed,
            files_failed: failed,
            messages,
            error,
        })
    }

    /// Staging root for archives belonging to one torrent:
    /// `<archive dir>/.librarian-extract/<info hash>/`.
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
        if reason.starts_with(TORRENT_LINK_PARSED_MATCH_TYPE) {
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

        if !diagnostics.unmatched.is_empty() {
            let names = diagnostics
                .unmatched
                .iter()
                .take(5)
                .map(|item| Self::display_file_name(&item.source_path))
                .collect::<Vec<_>>()
                .join(", ");
            let overflow = diagnostics.unmatched.len().saturating_sub(5);
            let target = diagnostics
                .target_label
                .as_deref()
                .map(|label| format!(" any item of {label}"))
                .unwrap_or_else(|| " any library item".to_string());
            let mut reasons: Vec<String> = Vec::new();
            for item in &diagnostics.unmatched {
                if !reasons.contains(&item.reason) {
                    reasons.push(item.reason.clone());
                }
            }
            parts.push(format!(
                "{} file(s) did not match{}: {}{} ({})",
                diagnostics.unmatched.len(),
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

    /// Load the wanted-target linkage stamped on the `Torrent` row at grab
    /// time (`jobs::auto_download` and manual `addTorrent` both set these).
    async fn load_torrent_import_link(
        &self,
        auth_user: &AuthUser,
        info_hash: &str,
    ) -> Result<TorrentImportLink> {
        let data = self
            .execute_graphql(
                auth_user,
                r#"query TorrentImportLinkage($infoHash: String!) {
                    Torrents: torrents(
                        where: { infoHash: { eq: $infoHash } }
                        page: { limit: 1 }
                    ) {
                        Edges: edges {
                            Node: node {
                                Id: id
                                Name: name
                                LibraryId: libraryId
                                MovieId: movieId
                                EpisodeId: episodeId
                                TrackId: trackId
                                ChapterId: chapterId
                                ShowId: showId
                                AlbumId: albumId
                                AudiobookId: audiobookId
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "infoHash": info_hash }),
            )
            .await?;
        let node = data
            .get("Torrents")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        let text = |key: &str| -> Option<String> {
            node.get(key)
                .and_then(|value| value.as_str())
                .filter(|value| !value.is_empty())
                .map(|value| value.to_string())
        };

        Ok(TorrentImportLink {
            name: text("Name"),
            library_id: text("LibraryId"),
            movie_id: text("MovieId"),
            episode_id: text("EpisodeId"),
            track_id: text("TrackId"),
            chapter_id: text("ChapterId"),
            show_id: text("ShowId"),
            album_id: text("AlbumId"),
            audiobook_id: text("AudiobookId"),
            season: self.load_torrent_season(auth_user, info_hash).await,
        })
    }

    /// `Torrent.season` is only set for season-pack grabs. It is queried
    /// separately and tolerantly so import keeps working on deployments whose
    /// schema predates the column (the season is then re-derived from the
    /// release name).
    async fn load_torrent_season(&self, auth_user: &AuthUser, info_hash: &str) -> Option<i32> {
        match self
            .execute_graphql(
                auth_user,
                r#"query TorrentImportSeason($infoHash: String!) {
                    Torrents: torrents(
                        where: { infoHash: { eq: $infoHash } }
                        page: { limit: 1 }
                    ) {
                        Edges: edges { Node: node { Season: season } } }
                }"#,
                serde_json::json!({ "infoHash": info_hash }),
            )
            .await
        {
            Ok(data) => data
                .get("Torrents")
                .and_then(|v| v.get("Edges"))
                .and_then(|v| v.as_array())
                .and_then(|edges| edges.first())
                .and_then(|edge| edge.get("Node"))
                .and_then(|node| node.get("Season"))
                .and_then(|value| value.as_i64())
                .map(|value| value as i32),
            Err(error) => {
                debug!(
                    info_hash = %info_hash,
                    error = %error,
                    "Torrent.season is unavailable; season packs will fall back to name parsing: info_hash={}, error={}",
                    info_hash,
                    error
                );
                None
            }
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
            self.plan_linked_album(auth_user, album_id, &audio, &mut plan)
                .await?;
        } else if let Some(audiobook_id) = link.audiobook_id.as_deref() {
            self.plan_linked_audiobook(auth_user, audiobook_id, &audio, &mut plan)
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

        let mut used: HashSet<String> = HashSet::new();
        let positional = audio.len() == tracks.len();
        for (index, item) in audio.iter().enumerate() {
            let hint = Self::parse_track_hint(&item.file.source_path);
            let chosen =
                Self::choose_album_track(&tracks, &used, hint.track_number, hint.title.as_deref())
                    .or_else(|| {
                        // Tags and filenames were ambiguous. Audio fingerprinting
                        // (services/fingerprint.rs) would be the ideal tiebreaker
                        // but it needs the Chromaprint `fpcalc` binary plus an
                        // AcoustID key and is not wired into the service graph, so
                        // fall back to release order when the counts line up
                        // exactly (design.md Q53).
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
            let label = format!("{album_name} - {}", track.title);
            plan.targets.insert(
                item.file.source_path.clone(),
                LinkedImportTarget {
                    library_id: library_id.to_string(),
                    target_type: "Track".to_string(),
                    target_id: track.id.clone(),
                    target_name: Some(label.clone()),
                    match_type: TORRENT_LINK_PARSED_MATCH_TYPE,
                    reason: format!(
                        "{TORRENT_LINK_PARSED_MATCH_TYPE}: torrent is linked to album '{album_name}' and this file matched track '{}'",
                        track.title
                    ),
                },
            );
        }

        Ok(())
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

    async fn plan_linked_audiobook(
        &self,
        auth_user: &AuthUser,
        audiobook_id: &str,
        audio: &[&ImportFile],
        plan: &mut LinkedImportPlan,
    ) -> Result<()> {
        let data = self
            .execute_graphql(
                auth_user,
                r#"query ImportLinkAudiobook($id: String!) {
                    Audiobook: audiobook(id: $id) {
                        Id: id
                        Title: title
                        LibraryId: libraryId
                        Chapters: chapters(page: { limit: 2000 }) {
                            Edges: edges {
                                Node: node {
                                    Id: id
                                    Title: title
                                    ChapterNumber: chapterNumber
                                }
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "id": audiobook_id }),
            )
            .await?;
        let audiobook = data.get("Audiobook");
        let (Some(title), Some(library_id)) = (
            audiobook
                .and_then(|a| a.get("Title"))
                .and_then(|v| v.as_str()),
            audiobook
                .and_then(|a| a.get("LibraryId"))
                .and_then(|v| v.as_str()),
        ) else {
            plan.notes.push(format!(
                "torrent is linked to audiobook {audiobook_id} but that audiobook no longer exists"
            ));
            return Ok(());
        };
        plan.target_label = Some(format!("'{title}'"));

        let mut chapters: Vec<LinkedChapterRow> = Vec::new();
        if let Some(edges) = audiobook
            .and_then(|a| a.get("Chapters"))
            .and_then(|c| c.get("Edges"))
            .and_then(|v| v.as_array())
        {
            for edge in edges {
                let Some(node) = edge.get("Node") else {
                    continue;
                };
                let Some(id) = node.get("Id").and_then(|v| v.as_str()) else {
                    continue;
                };
                chapters.push(LinkedChapterRow {
                    id: id.to_string(),
                    number: node
                        .get("ChapterNumber")
                        .and_then(|v| v.as_i64())
                        .map(|v| v as i32)
                        .unwrap_or(0),
                    title: node
                        .get("Title")
                        .and_then(|v| v.as_str())
                        .map(|v| v.to_string()),
                });
            }
        }
        if chapters.is_empty() {
            plan.notes.push(format!(
                "audiobook '{title}' has no chapter rows to import files into; add chapters before importing"
            ));
            return Ok(());
        }
        chapters.sort_by_key(|chapter| chapter.number);

        if audio.is_empty() {
            plan.notes
                .push(format!("no audio files found for audiobook '{title}'"));
            return Ok(());
        }

        // Q52: audiobook releases are whole-book, not per-chapter, so files map
        // onto chapters in release (filename) order. `audio` is already sorted
        // naturally by the caller.
        for (index, item) in audio.iter().enumerate() {
            let Some(chapter) = chapters.get(index) else {
                plan.unmatched_reasons.insert(
                    item.file.source_path.clone(),
                    format!(
                        "audiobook '{title}' only has {} chapter(s); this file is number {} in the release",
                        chapters.len(),
                        index + 1
                    ),
                );
                continue;
            };
            let label = match chapter.title.as_deref() {
                Some(chapter_title) if !chapter_title.is_empty() => {
                    format!("{title} - {chapter_title}")
                }
                _ => format!("{title} - chapter {}", chapter.number),
            };
            plan.targets.insert(
                item.file.source_path.clone(),
                LinkedImportTarget {
                    library_id: library_id.to_string(),
                    target_type: "Chapter".to_string(),
                    target_id: chapter.id.clone(),
                    target_name: Some(label),
                    match_type: TORRENT_LINK_PARSED_MATCH_TYPE,
                    reason: format!(
                        "{TORRENT_LINK_PARSED_MATCH_TYPE}: torrent is linked to audiobook '{title}' and this is file {} of the release",
                        index + 1
                    ),
                },
            );
        }

        if audio.len() < chapters.len() {
            plan.notes.push(format!(
                "audiobook '{title}': {} of {} chapters have a file from this release",
                audio.len(),
                chapters.len()
            ));
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
        file: &SourceFileImport,
    ) -> Result<String> {
        if let Some(id) = self
            .find_media_file_id_by_path(auth_user, &file.source_path)
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
                        Edges: edges { Node: node { Id: id } }
                    }
                }"#,
                serde_json::json!({ "path": path }),
            )
            .await?;
        Ok(data
            .get("MediaFiles")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| node.get("Id"))
            .and_then(|v| v.as_str())
            .map(|value| value.to_string()))
    }

    async fn candidate_libraries_for_import(
        &self,
        auth_user: &AuthUser,
        explicit_library_id: Option<&str>,
        file: &SourceFileImport,
    ) -> Result<Vec<LibraryRow>> {
        let compatible_types = Self::compatible_library_types_for_path(&file.source_path);
        if compatible_types.is_empty() {
            return Ok(Vec::new());
        }

        if let Some(library_id) = explicit_library_id {
            let library = self.get_library(library_id).await?;
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
                serde_json::json!({ "userId": auth_user.user_id }),
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
                        .unwrap_or(&auth_user.user_id)
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
        if !source_path.exists() {
            anyhow::bail!("Source file does not exist: {}", source_path.display());
        }

        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
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
        let outcome = no_clobber_place(
            source_path.to_path_buf(),
            target_path.to_path_buf(),
            allow_move,
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
                "WARNING",
                "ORGANIZATION",
                "Torrent import conflict skipped",
                &message,
            )
            .await;
            anyhow::bail!("Target path already exists; skipped to avoid overwrite");
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
        let Some(db_service) = self.manager.get_database().await else {
            return;
        };
        let db = db_service.pool().clone();

        let target_str = target_path.to_string_lossy().to_string();
        let existing = match crate::graphql::entities::MediaFile::query(db.pool())
            .filter(crate::graphql::entities::MediaFileWhereInput {
                path: Some(graphql_orm::graphql::filters::StringFilter {
                    eq: Some(target_str.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
        {
            Ok(rows) => rows.into_iter().next(),
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
        let action_data = serde_json::json!({
            "sourcePath": source_path.to_string_lossy(),
            "targetPath": target_str,
            "mediaFileId": existing.id,
            "newResolution": new_parsed.resolution,
            "existingResolution": existing_parsed.resolution,
        })
        .to_string();

        self.create_action_notification(
            auth_user,
            "ACTION_REQUIRED",
            "QUALITY",
            "Possible quality upgrade found",
            &message,
            Some(&library.id),
            Some(&existing.id),
            crate::services::graphql::mutations::quality::QUALITY_UPGRADE_ACTION_TYPE,
            &action_data,
        )
        .await;
    }

    #[allow(clippy::too_many_arguments)]
    async fn upsert_pending_file_match(
        &self,
        auth_user: &AuthUser,
        info_hash: &str,
        file: &SourceFileImport,
        candidate: Option<&MatchCandidate>,
        unmatched_reason: Option<&str>,
        copied_at: Option<&str>,
        copy_error: Option<&str>,
    ) -> Result<()> {
        let existing = self
            .find_pending_file_match(auth_user, info_hash, file.file_index, &file.source_path)
            .await?;

        let input = serde_json::json!({
            "userId": auth_user.user_id.clone(),
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
        let data = self
            .execute_graphql(
                auth_user,
                r#"query PendingFileMatchForSource($where: PendingFileMatchWhereInput!) {
                    PendingFileMatches: pendingFileMatches(where: $where, page: { limit: 1 }) {
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
        info_hash: &str,
        status: &str,
        error: Option<&str>,
        _processed: i32,
        _failed: i32,
    ) -> Result<()> {
        let auth_user = self.system_auth_user(None).await?;
        let data = self
            .execute_graphql(
                &auth_user,
                r#"query TorrentForPostProcessStatus($infoHash: String!) {
                    Torrents: torrents(
                        where: { infoHash: { eq: $infoHash } }
                        page: { limit: 1 }
                    ) {
                        Edges: edges { Node: node { Id: id } }
                    }
                }"#,
                serde_json::json!({ "infoHash": info_hash }),
            )
            .await?;
        let Some(torrent_id) = data
            .get("Torrents")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| node.get("Id"))
            .and_then(|v| v.as_str())
        else {
            return Ok(());
        };

        let data = self
            .execute_mutation(
                &auth_user,
                r#"mutation UpdateTorrentPostProcessStatus($id: String!, $input: UpdateTorrentInput!) {
                    updateTorrent(id: $id, input: $input) { success error }
                }"#,
                serde_json::json!({
                    "id": torrent_id,
                    "input": {
                        "postProcessStatus": status,
                        "postProcessError": error,
                        "processedAt": chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
                    }
                }),
            )
            .await?;
        if !data
            .get("updateTorrent")
            .and_then(|v| v.get("success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            anyhow::bail!("updateTorrent post-process status failed");
        }
        Ok(())
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

    async fn reconcile_missing_media_files(
        &self,
        auth_user: &AuthUser,
        discovered_paths: &HashSet<String>,
        rows: &[ExistingMediaFileRow],
    ) -> Result<()> {
        for row in rows {
            if discovered_paths.contains(&row.path) || Path::new(&row.path).exists() {
                continue;
            }

            let _ = self
                .execute_mutation(
                    auth_user,
                    r#"mutation DeleteMissingMediaFile($id: String!) {
                        DeleteMediaFile: deleteMediaFile(id: $id) { Success: success Error: error }
                    }"#,
                    serde_json::json!({ "id": row.id }),
                )
                .await;
        }

        Ok(())
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
                tokio::fs::create_dir_all(PathBuf::from(path)).await?;
            }

            if seasons.is_empty() {
                let (show_dir, _) = derive_tv_dirs_from_pattern(
                    &library.path,
                    &naming_pattern,
                    &show_name,
                    1,
                    show_year,
                );
                tokio::fs::create_dir_all(show_dir).await?;
                continue;
            }

            for season in seasons {
                let (show_dir, season_dir) = derive_tv_dirs_from_pattern(
                    &library.path,
                    &naming_pattern,
                    &show_name,
                    season,
                    show_year,
                );
                tokio::fs::create_dir_all(show_dir).await?;
                tokio::fs::create_dir_all(season_dir).await?;
            }
        }

        Ok(())
    }

    async fn cleanup_empty_folders(&self, library: &LibraryRow) -> Result<()> {
        let library_root = PathBuf::from(&library.path);
        if !library_root.exists() {
            return Ok(());
        }

        let protected = self.build_protected_paths(library).await?;
        Self::cleanup_empty_unprotected_folders(&library_root, &protected).await
    }

    async fn cleanup_empty_unprotected_folders(
        library_root: &Path,
        protected: &HashSet<PathBuf>,
    ) -> Result<()> {
        if !library_root.exists() {
            return Ok(());
        }

        let traversal_root = library_root.to_path_buf();
        let mut dirs = tokio::task::spawn_blocking(move || {
            WalkDir::new(traversal_root)
                .into_iter()
                .filter_map(std::result::Result::ok)
                .filter(|entry| entry.file_type().is_dir())
                .map(|entry| entry.into_path())
                .collect::<Vec<_>>()
        })
        .await
        .context("Empty-folder traversal worker failed")?;

        dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));

        for dir in dirs {
            if dir == library_root || protected.contains(&dir) {
                continue;
            }

            let is_empty = tokio::fs::read_dir(&dir)
                .await
                .map(|mut rd| async move { rd.next_entry().await.ok().flatten().is_none() })
                .ok();

            if let Some(check) = is_empty
                && check.await
            {
                match tokio::fs::remove_dir(&dir).await {
                    Ok(()) => {
                        debug!(
                            folder_path = %dir.to_string_lossy(),
                            library_root = %library_root.to_string_lossy(),
                            "Removed empty unprotected library folder: folder_path={}, library_root={}",
                            dir.to_string_lossy(),
                            library_root.to_string_lossy()
                        );
                    }
                    Err(error) => {
                        debug!(
                            folder_path = %dir.to_string_lossy(),
                            library_root = %library_root.to_string_lossy(),
                            error = %error,
                            "Failed to remove empty unprotected library folder: folder_path={}, library_root={}, error={}",
                            dir.to_string_lossy(),
                            library_root.to_string_lossy(),
                            error
                        );
                    }
                }
            }
        }

        Ok(())
    }

    fn build_tv_protected_paths_from_rows(
        library_path: &str,
        naming_pattern: &str,
        shows: &[TvShowFolderRow],
    ) -> HashSet<PathBuf> {
        let mut protected = HashSet::new();
        let root = PathBuf::from(library_path);
        protected.insert(root.clone());

        for (show_name, show_path, show_year, seasons) in shows {
            if let Some(path) = show_path {
                protected.insert(PathBuf::from(path));
            }

            if seasons.is_empty() {
                let (show_dir, _) = derive_tv_dirs_from_pattern(
                    library_path,
                    naming_pattern,
                    show_name,
                    1,
                    *show_year,
                );
                protected.insert(show_dir);
            } else {
                for season in seasons {
                    let (show_dir, season_dir) = derive_tv_dirs_from_pattern(
                        library_path,
                        naming_pattern,
                        show_name,
                        *season,
                        *show_year,
                    );
                    protected.insert(show_dir);
                    protected.insert(season_dir);
                }
            }
        }

        protected
    }

    async fn build_tv_protected_paths(&self, library: &LibraryRow) -> Result<HashSet<PathBuf>> {
        let auth_user = self.system_auth_user(Some(&library.user_id)).await?;
        let naming_pattern = self.resolve_library_naming_pattern(library).await?;

        let shows = self
            .get_tv_shows_for_library(&auth_user, &library.id)
            .await?;
        Ok(Self::build_tv_protected_paths_from_rows(
            &library.path,
            &naming_pattern,
            &shows,
        ))
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
            if let Some(parent) = root.join(rel).parent().map(|p| p.to_path_buf()) {
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
        for track in self.load_tracks_for_library(&library.id).await? {
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
            let rel = apply_music_naming_pattern(
                &naming_pattern,
                track.artist_name.as_deref().unwrap_or("Unknown Artist"),
                album_name,
                *album_year,
                track.track_number,
                track.disc_number,
                &track.title,
                original_filename,
                "mp3",
            );
            if let Some(parent) = root.join(rel).parent().map(PathBuf::from) {
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
            if let Some(parent) = root.join(rel).parent().map(PathBuf::from) {
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

#[derive(Debug, Default)]
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

#[derive(Debug, Default)]
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

#[derive(Debug, Default)]
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

#[derive(Debug, Default)]
struct ChapterAnalysis {
    chapter_index: i32,
    start_secs: f64,
    end_secs: f64,
    title: Option<String>,
}

#[derive(Debug, Default)]
struct ProbeAnalysis {
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
    let mut child = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_streams")
        .arg("-show_chapters")
        .arg("-show_format")
        .arg("-print_format")
        .arg("json")
        .arg(path)
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

    if let Some(format) = parsed.format {
        analysis.container = format.format_name;
        analysis.duration = format
            .duration
            .as_deref()
            .and_then(|d| d.parse::<f64>().ok())
            .map(|d| d.round() as i32);
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

fn sanitize_for_filename(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
            out.push('_');
        } else {
            out.push(c);
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
) -> (PathBuf, PathBuf) {
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
    let absolute = PathBuf::from(library_path).join(relative);

    let season_dir = absolute
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(library_path));
    let show_dir = season_dir
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(library_path));

    (show_dir, season_dir)
}

async fn paths_refer_to_same_file(left: &Path, right: &Path) -> bool {
    let Ok(left_meta) = tokio::fs::metadata(left).await else {
        return false;
    };
    let Ok(right_meta) = tokio::fs::metadata(right).await else {
        return false;
    };

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        left_meta.dev() == right_meta.dev() && left_meta.ino() == right_meta.ino()
    }

    #[cfg(not(unix))]
    {
        tokio::fs::canonicalize(left).await.ok() == tokio::fs::canonicalize(right).await.ok()
    }
}

/// Outcome of [`no_clobber_place`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoClobberOutcome {
    /// `src` (or its bytes, for a copy) is now at `dst`.
    Placed,
    /// `dst` already existed; nothing was touched (`src` is untouched, `dst` is
    /// untouched). The caller should treat this as a conflict.
    TargetExists,
}

/// Place `src` at `dst` without ever clobbering an existing `dst`.
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
async fn no_clobber_place(
    src: PathBuf,
    dst: PathBuf,
    remove_source_on_success: bool,
) -> Result<NoClobberOutcome> {
    tokio::task::spawn_blocking(move || {
        no_clobber_place_blocking(&src, &dst, remove_source_on_success)
    })
    .await
    .context("no-clobber file placement task panicked")?
}

fn no_clobber_place_blocking(
    src: &Path,
    dst: &Path,
    remove_source_on_success: bool,
) -> Result<NoClobberOutcome> {
    match std::fs::hard_link(src, dst) {
        Ok(()) => {
            if remove_source_on_success {
                std::fs::remove_file(src).with_context(|| {
                    format!("failed to remove source file after move: {}", src.display())
                })?;
            }
            Ok(NoClobberOutcome::Placed)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            Ok(NoClobberOutcome::TargetExists)
        }
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {
            cross_device_no_clobber_place(src, dst, remove_source_on_success)
        }
        Err(e) => Err(anyhow::Error::from(e).context(format!(
            "hard_link failed: {} -> {}",
            src.display(),
            dst.display()
        ))),
    }
}

fn cross_device_no_clobber_place(
    src: &Path,
    dst: &Path,
    remove_source_on_success: bool,
) -> Result<NoClobberOutcome> {
    let tmp_path = cross_device_temp_path(dst);
    let result = (|| -> Result<NoClobberOutcome> {
        std::fs::copy(src, &tmp_path).with_context(|| {
            format!(
                "cross-device copy-to-temp failed: {} -> {}",
                src.display(),
                tmp_path.display()
            )
        })?;
        match std::fs::hard_link(&tmp_path, dst) {
            Ok(()) => Ok(NoClobberOutcome::Placed),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                Ok(NoClobberOutcome::TargetExists)
            }
            Err(e) => Err(anyhow::Error::from(e).context(format!(
                "cross-device no-clobber hard_link failed: {} -> {}",
                tmp_path.display(),
                dst.display()
            ))),
        }
    })();
    // Always clean up the temp file, regardless of outcome.
    let _ = std::fs::remove_file(&tmp_path);

    let outcome = result?;
    if outcome == NoClobberOutcome::Placed && remove_source_on_success {
        std::fs::remove_file(src).with_context(|| {
            format!(
                "failed to remove source file after cross-device move: {}",
                src.display()
            )
        })?;
    }
    Ok(outcome)
}

fn cross_device_temp_path(dst: &Path) -> PathBuf {
    let file_name = dst
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "librarian-file".to_string());
    dst.with_file_name(format!("{file_name}.librarian-tmp-{}", Uuid::new_v4()))
}

#[cfg(test)]
mod tests {

    use super::{
        ImportDiagnostics, ImportFile, LinkedTrackRow, MatchCandidate, SkippedImportFile,
        SourceFileImport, TORRENT_LINK_MATCH_TYPE, TORRENT_LINK_PARSED_MATCH_TYPE,
        TorrentImportLink, UnmatchedImportFile,
    };

    fn import_file(path: &str, size_mb: i64) -> ImportFile {
        ImportFile {
            file: SourceFileImport {
                source_path: path.to_string(),
                relative_path: path.to_string(),
                file_size: size_mb * 1024 * 1024,
                downloaded_bytes: size_mb * 1024 * 1024,
                file_index: None,
            },
            from_archive: false,
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
                id: "t1".to_string(),
                title: "Welcome to the Jungle".to_string(),
                track_number: Some(1),
                disc_number: Some(1),
            },
            LinkedTrackRow {
                id: "t2".to_string(),
                title: "It's So Easy".to_string(),
                track_number: Some(2),
                disc_number: Some(1),
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
        LibraryScanService, LibraryScanServiceConfig, MatchWantedPolicy, NoClobberOutcome,
        OrganizeResult, ProviderFallbackSummary, no_clobber_place, supervise_worker,
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
        );

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
                naming_pattern: String::new(),
                scan_interval_minutes: 60,
                watch_for_changes: false,
                scanning: false,
                last_scanned_at: None,
                quality_profile_id: None,
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
        assert_eq!(issues.len(), 2, "each durable scan keeps its own issue");

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
                naming_pattern: String::new(),
                scan_interval_minutes: 60,
                watch_for_changes: false,
                scanning: false,
                last_scanned_at: None,
                quality_profile_id: None,
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
