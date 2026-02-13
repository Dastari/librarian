use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context as AnyhowContext, Result};
use async_graphql::{Request, Variables};
use async_trait::async_trait;
use chrono::{Datelike, Utc};
use futures::stream::{self, StreamExt};
use regex::Regex;
use serde::Deserialize;
use tokio::process::Command;
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::services::graphql::entities::common::AutoDownloadMode;
use crate::services::graphql::{AuthUser, LibrarianSchema};
use crate::services::manager::{Service, ServiceHealth, ServicesManager};
use crate::services::metadata::providers::{
    AddAlbumOptions, AddAudiobookOptions, AddMovieOptions, AddTvShowOptions, MetadataProvider,
    MetadataService,
};

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
struct ScanJob {
    library_id: String,
}

#[derive(Debug, Clone)]
struct AnalyzeJob {
    media_file_id: String,
    path: String,
}

#[derive(Debug, Clone)]
struct PendingProviderFallback {
    media_file_id: String,
    media_path: String,
    match_source: String,
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

#[derive(Debug)]
struct Runtime {
    cancel: CancellationToken,
    scheduler_handle: JoinHandle<()>,
    scan_worker_handle: JoinHandle<()>,
    analyze_worker_handles: Vec<JoinHandle<()>>,
}

#[derive(Debug)]
struct LibraryRow {
    id: String,
    user_id: String,
    name: String,
    path: String,
    library_type: String,
    auto_scan: bool,
    auto_organize: bool,
    scan_interval_minutes: i32,
    scanning: bool,
    last_scanned_at: Option<String>,
    naming_pattern: String,
}

#[derive(Debug)]
struct MediaFileRow {
    id: String,
    library_id: Option<String>,
    path: String,
    original_name: Option<String>,
    size: i64,
    episode_id: Option<String>,
    movie_id: Option<String>,
    track_id: Option<String>,
    chapter_id: Option<String>,
}

#[derive(Debug)]
struct ExistingMediaFileRow {
    id: String,
    path: String,
    movie_id: Option<String>,
    episode_id: Option<String>,
    track_id: Option<String>,
    chapter_id: Option<String>,
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
    scan_tx: mpsc::UnboundedSender<ScanJob>,
    scan_rx: Arc<Mutex<mpsc::UnboundedReceiver<ScanJob>>>,
    analyze_tx: mpsc::UnboundedSender<AnalyzeJob>,
    analyze_rx: Arc<Mutex<mpsc::UnboundedReceiver<AnalyzeJob>>>,
    in_progress_scans: Arc<Mutex<HashSet<String>>>,
    queued_analysis: Arc<Mutex<HashSet<String>>>,
    movie_candidate_cache: Arc<RwLock<HashMap<String, Vec<MovieCandidateRow>>>>,
    episode_candidate_cache: Arc<RwLock<HashMap<String, Vec<EpisodeCandidateRow>>>>,
    track_candidate_cache: Arc<RwLock<HashMap<String, Vec<TrackCandidateRow>>>>,
    chapter_candidate_cache: Arc<RwLock<HashMap<String, Vec<ChapterCandidateRow>>>>,
}

impl LibraryScanService {
    const AUTO_ORGANIZE_MOVIE_CONFIDENCE_GUARD: f64 = 0.93;
    const AUTO_ORGANIZE_MOVIE_TITLE_SIMILARITY_GUARD: f64 = 0.86;

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
        let (scan_tx, scan_rx) = mpsc::unbounded_channel();
        let (analyze_tx, analyze_rx) = mpsc::unbounded_channel();

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
        let schema = self.graphql_schema().await?;
        let request = Request::new(document)
            .variables(Variables::from_json(variables))
            .data(auth_user.clone());
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
        let bootstrap_auth_user = self.system_auth_user(None).await?;
        let users_data = self
            .execute_graphql(
                &bootstrap_auth_user,
                r#"query NotificationUsers {
                    Users(Page: { Limit: 10000 }) {
                        Edges { Node { Id } }
                    }
                }"#,
                serde_json::json!({}),
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
                    r#"query ExistingFfprobeNotification($Where: NotificationWhereInput, $Page: PageInput) {
                        Notifications(Where: $Where, Page: $Page) {
                            Edges { Node { Id } }
                        }
                    }"#,
                    serde_json::json!({
                        "Where": {
                            "UserId": { "Eq": user_id },
                            "Category": { "Eq": "CONFIGURATION" },
                            "Title": { "Eq": "ffprobe is not installed" }
                        },
                        "Page": { "Limit": 1, "Offset": 0 }
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
                    r#"mutation CreateFfprobeMissingNotification($Input: CreateNotificationInput!) {
                        CreateNotification(Input: $Input) {
                            Success
                            Error
                        }
                    }"#,
                    serde_json::json!({
                        "Input": {
                            "UserId": auth_user.user_id,
                            "NotificationType": "ERROR",
                            "Category": "CONFIGURATION",
                            "Title": "ffprobe is not installed",
                            "Message": "Library analysis is unavailable because ffprobe is missing from PATH. Install ffmpeg/ffprobe and restart Librarian."
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
        if let Err(e) = self
            .execute_mutation(
                auth_user,
                r#"mutation CreateLibraryScanNotification($Input: CreateNotificationInput!) {
                    CreateNotification(Input: $Input) {
                        Success
                        Error
                    }
                }"#,
                serde_json::json!({
                    "Input": {
                        "UserId": auth_user.user_id,
                        "NotificationType": notification_type,
                        "Category": category,
                        "Title": title,
                        "Message": message
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

    async fn get_movie_title_year(
        &self,
        auth_user: &AuthUser,
        movie_id: &str,
    ) -> Result<Option<(String, Option<i32>)>> {
        let data = self
            .execute_graphql(
                auth_user,
                r#"query MovieTitleYear($Id: String!) {
                    Movie(Id: $Id) { Id Title Year }
                }"#,
                serde_json::json!({ "Id": movie_id }),
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

    async fn system_auth_user(&self, fallback_user_id: Option<&str>) -> Result<AuthUser> {
        if let Some(user_id) = fallback_user_id {
            return Ok(AuthUser {
                user_id: user_id.to_string(),
                email: None,
                role: Some("admin".to_string()),
            });
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
                    Users(Page: { Limit: 1 }) {
                        Edges { Node { Id } }
                    }
                }"#,
                serde_json::json!({}),
            )
            .await
        {
            if let Some(user_id) = data
                .get("Users")
                .and_then(|v| v.get("Edges"))
                .and_then(|v| v.as_array())
                .and_then(|edges| edges.first())
                .and_then(|edge| edge.get("Node"))
                .and_then(|node| node.get("Id"))
                .and_then(|v| v.as_str())
            {
                return Ok(AuthUser {
                    user_id: user_id.to_string(),
                    email: None,
                    role: Some("admin".to_string()),
                });
            }
        }

        // Bootstrap fallback only: GraphQL needs an AuthUser, so we query users directly if bootstrap auth fails.
        let db_svc = self
            .manager
            .get_database()
            .await
            .ok_or_else(|| anyhow::anyhow!("Database service not available"))?;

        let first_user: Option<(String,)> =
            sqlx::query_as("SELECT id FROM users ORDER BY created_at ASC LIMIT 1")
                .fetch_optional(db_svc.pool())
                .await?;

        let user_id = first_user
            .map(|(id,)| id)
            .ok_or_else(|| anyhow::anyhow!("No user exists to run library scan operations"))?;

        Ok(AuthUser {
            user_id,
            email: None,
            role: Some("admin".to_string()),
        })
    }

    async fn get_library(&self, library_id: &str) -> Result<LibraryRow> {
        let auth_user = self.system_auth_user(None).await?;
        let data = self
            .execute_graphql(
                &auth_user,
                r#"query GetLibraryForScan($Id: String!) {
                    Library(Id: $Id) {
                        Id
                        UserId
                        Name
                        Path
                        LibraryType
                        AutoScan
                        AutoOrganize
                        ScanIntervalMinutes
                        Scanning
                        LastScannedAt
                        NamingPattern
                    }
                }"#,
                serde_json::json!({ "Id": library_id }),
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
            auto_scan: library
                .get("AutoScan")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
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
            last_scanned_at: library
                .get("LastScannedAt")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
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
                r#"query ResolveNamingPattern($LibraryType: String!) {
                    NamingPatterns(
                        Where: { LibraryType: { Eq: $LibraryType }, IsDefault: { Eq: true } }
                        Page: { Limit: 100 }
                    ) {
                        Edges {
                            Node {
                                Pattern
                                IsSystem
                                CreatedAt
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "LibraryType": normalized_type }),
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
        let auth_user = self.system_auth_user(None).await?;
        let data = self
            .execute_graphql(
                &auth_user,
                r#"query DueAutoscanLibraries {
                    Libraries(
                        Where: { AutoScan: { Eq: true }, Scanning: { Eq: false } }
                        Page: { Limit: 10000 }
                    ) {
                        Edges {
                            Node {
                                Id
                                LastScannedAt
                                ScanIntervalMinutes
                            }
                        }
                    }
                }"#,
                serde_json::json!({}),
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
        let auth_user = self.system_auth_user(None).await?;
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
                    r#"query LibrariesMarkedScanningOnStartup($Where: LibraryWhereInput, $Page: PageInput) {
                        Libraries(Where: $Where, Page: $Page) {
                            Edges { Node { Id Name Scanning } }
                        }
                    }"#,
                    serde_json::json!({
                        "Where": { "Scanning": { "Eq": true } },
                        "Page": { "Limit": limit, "Offset": offset }
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

    async fn set_library_scanning(
        &self,
        auth_user: &AuthUser,
        library_id: &str,
        scanning: bool,
        set_last_scanned: bool,
    ) -> Result<()> {
        let mut input = serde_json::json!({
            "Scanning": scanning,
        });

        if set_last_scanned {
            input["LastScannedAt"] = serde_json::json!(Utc::now().to_rfc3339());
        }

        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation UpdateLibraryForScan($Id: String!, $Input: UpdateLibraryInput!) {
                    UpdateLibrary(Id: $Id, Input: $Input) { Success Error }
                }"#,
                serde_json::json!({
                    "Id": library_id,
                    "Input": input,
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
        path: &str,
        relative_path: Option<&str>,
        size: i64,
        content_type: Option<&str>,
    ) -> Result<String> {
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation CreateMediaFileFromScan($Input: CreateMediaFileInput!) {
                    CreateMediaFile(Input: $Input) {
                        Success
                        Error
                        MediaFile { Id }
                    }
                }"#,
                serde_json::json!({
                    "Input": {
                        "LibraryId": library_id,
                        "Path": path,
                        "RelativePath": relative_path,
                        "OriginalName": Path::new(path).file_name().and_then(|n| n.to_str()),
                        "Size": size,
                        "IsHdr": false,
                        "ContentType": content_type,
                        "AddedAt": Utc::now().to_rfc3339(),
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

    async fn get_media_file_by_path(
        &self,
        library_id: &str,
        path: &str,
    ) -> Result<Option<MediaFileRow>> {
        let auth_user = self.system_auth_user(None).await?;
        let data = self
            .execute_graphql(
                &auth_user,
                r#"query MediaFileByPath($LibraryId: String!, $Path: String!) {
                    MediaFiles(
                        Where: { LibraryId: { Eq: $LibraryId }, Path: { Eq: $Path } }
                        Page: { Limit: 1 }
                    ) {
                        Edges {
                            Node {
                                Id
                                LibraryId
                                Path
                                OriginalName
                                Size
                                EpisodeId
                                MovieId
                                TrackId
                                ChapterId
                            }
                        }
                    }
                }"#,
                serde_json::json!({
                    "LibraryId": library_id,
                    "Path": path,
                }),
            )
            .await?;

        let node = data
            .get("MediaFiles")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"));

        Ok(node.map(|n| MediaFileRow {
            id: n
                .get("Id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            library_id: n
                .get("LibraryId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            path: n
                .get("Path")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            original_name: n
                .get("OriginalName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            size: n.get("Size").and_then(|v| v.as_i64()).unwrap_or(0),
            episode_id: n
                .get("EpisodeId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            movie_id: n
                .get("MovieId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            track_id: n
                .get("TrackId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            chapter_id: n
                .get("ChapterId")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        }))
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
                r#"mutation EnsureMediaFileOriginalName($Id: String!, $Input: UpdateMediaFileInput!) {
                    UpdateMediaFile(Id: $Id, Input: $Input) { Success Error }
                }"#,
                serde_json::json!({
                    "Id": media_file_id,
                    "Input": {
                        "OriginalName": original_name,
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

    pub async fn queue_scan(&self, library_id: &str) -> Result<bool> {
        {
            let in_progress = self.in_progress_scans.lock().await;
            if in_progress.contains(library_id) {
                return Ok(false);
            }
        }

        self.scan_tx
            .send(ScanJob {
                library_id: library_id.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("failed to queue scan job: {}", e))?;

        Ok(true)
    }

    pub async fn queue_analyze_job(&self, media_file_id: &str, path: &str) -> Result<bool> {
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

        self.analyze_tx
            .send(AnalyzeJob {
                media_file_id: media_file_id.to_string(),
                path: path.to_string(),
            })
            .map_err(|e| anyhow::anyhow!("failed to queue analyze job: {}", e))?;

        info!(
            media_file_id = %media_file_id,
            path = %path,
            "Queued analyze job: media_file_id={}, path={}",
            media_file_id,
            path
        );

        Ok(true)
    }

    async fn media_file_has_been_analyzed(&self, media_file_id: &str) -> Result<bool> {
        let auth_user = self.system_auth_user(None).await?;
        let data = self
            .execute_graphql(
                &auth_user,
                r#"query MediaFileAnalyzeState($Id: String!) {
                    MediaFile(Id: $Id) {
                        AnalyzedAt
                    }
                }"#,
                serde_json::json!({ "Id": media_file_id }),
            )
            .await?;

        Ok(data
            .get("MediaFile")
            .and_then(|v| v.get("AnalyzedAt"))
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false))
    }

    async fn scan_library_inner(self: &Arc<Self>, library_id: &str) -> Result<()> {
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
            self.reconcile_movie_collections_for_library(&library).await;
            self.set_library_scanning(&auth_user, library_id, false, true)
                .await?;
            return Ok(());
        }

        let allowed_ext = Self::extensions_for_library(&normalized_library_type);
        let mut scanned_count: usize = 0;
        let mut discovered_paths: HashSet<String> = HashSet::new();
        let mut pending_provider_fallback: Vec<PendingProviderFallback> = Vec::new();

        for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if file_name.starts_with('.') || file_name.starts_with("._") {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .unwrap_or_default();

            if !allowed_ext.iter().any(|x| *x == ext) {
                continue;
            }

            let abs_path = path.to_string_lossy().to_string();
            let match_source = Path::new(&abs_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&abs_path)
                .to_string();
            discovered_paths.insert(abs_path.clone());
            let rel_path = path
                .strip_prefix(&root)
                .ok()
                .map(|p| p.to_string_lossy().to_string());
            let size = match entry.metadata() {
                Ok(m) => m.len() as i64,
                Err(_) => 0,
            };

            let media_file = self.get_media_file_by_path(library_id, &abs_path).await?;
            let (media_file_id, is_new_media_file) = if let Some(existing) = media_file {
                self.ensure_original_name(
                    &auth_user,
                    &existing.id,
                    existing.original_name.as_deref(),
                    &abs_path,
                )
                .await?;
                (existing.id, false)
            } else {
                (
                    self.create_media_file(
                        &auth_user,
                        library_id,
                        &abs_path,
                        rel_path.as_deref(),
                        size,
                        Self::content_type_for_ext(&ext, &normalized_library_type),
                    )
                    .await?,
                    true,
                )
            };

            if is_new_media_file {
                debug!(
                    library_id = %library_id,
                    library_name = %library.name,
                    media_file_id = %media_file_id,
                    media_file_path = %abs_path,
                    "Discovered new media file during scan with deferred analysis (on-demand or torrent completion only): library_id={}, media_file_id={}, path={}",
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
                    "Skipping provider metadata fallback for previously known media file during scan: library_id={}, media_file_id={}, path={}",
                    library_id,
                    media_file_id,
                    abs_path
                );
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
                    self.maybe_auto_organize_after_scan_match(
                        &library,
                        &auth_user,
                        &media_file_id,
                        &abs_path,
                        &m,
                    )
                    .await;
                    if !m.success && is_new_media_file {
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
                }
            }

            scanned_count += 1;
        }

        self.process_provider_fallback_batch(&library, &auth_user, pending_provider_fallback)
            .await;

        self.reconcile_missing_media_files(&library, &auth_user, &discovered_paths)
            .await?;

        self.reconcile_movie_collections_for_library(&library).await;

        if Self::is_tv_library_type(&normalized_library_type) {
            self.ensure_tv_folder_structure(&library).await?;
        }

        if library.auto_organize {
            self.cleanup_empty_folders(&library).await?;
        }

        self.set_library_scanning(&auth_user, library_id, false, true)
            .await?;
        info!(
            library_id = %library_id,
            library_name = %library.name,
            library_type = %library.library_type,
            scanned_count,
            discovered_path_count = discovered_paths.len(),
            auto_organize = library.auto_organize,
            "Library scan completed: library_id={}, name={}, type={}, scanned_files={}, discovered_paths={}, auto_organize={}",
            library_id,
            library.name,
            library.library_type,
            scanned_count,
            discovered_paths.len(),
            library.auto_organize
        );

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
                continue;
            }

            if let Err(e) = self.scan_library_inner(&job.library_id).await {
                error!(
                    library_id = %job.library_id,
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
                r#"mutation UpdateMediaFileFromAnalysis($Id: String!, $Input: UpdateMediaFileInput!) {
                    UpdateMediaFile(Id: $Id, Input: $Input) {
                        Success
                        Error
                    }
                }"#,
                serde_json::json!({
                    "Id": media_file_id,
                    "Input": {
                        "Container": analysis.container,
                        "VideoCodec": analysis.video_codec,
                        "AudioCodec": analysis.audio_codec,
                        "Width": analysis.width,
                        "Height": analysis.height,
                        "Duration": analysis.duration,
                        "Bitrate": analysis.bitrate,
                        "Resolution": analysis.resolution,
                        "IsHdr": analysis.is_hdr,
                        "HdrType": analysis.hdr_type,
                        "AudioChannels": analysis.audio_channels,
                        "Metadata": analysis.metadata,
                        "AnalyzedAt": Utc::now().to_rfc3339(),
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
                r#"mutation ClearMediaFileAnalysisDetails($MediaFileId: String!) {
                    DeleteVideoStreams(Where: { MediaFileId: { Eq: $MediaFileId } }) { success error DeletedCount }
                    DeleteAudioStreams(Where: { MediaFileId: { Eq: $MediaFileId } }) { success error DeletedCount }
                    DeleteSubtitles(Where: { MediaFileId: { Eq: $MediaFileId } }) { success error DeletedCount }
                    DeleteMediaChapters(Where: { MediaFileId: { Eq: $MediaFileId } }) { success error DeletedCount }
                }"#,
                serde_json::json!({ "MediaFileId": media_file_id }),
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
                    r#"mutation CreateVideoStreamFromAnalysis($Input: CreateVideoStreamInput!) {
                        CreateVideoStream(Input: $Input) { Success Error }
                    }"#,
                    serde_json::json!({
                        "Input": {
                            "MediaFileId": media_file_id,
                            "StreamIndex": stream.stream_index,
                            "Codec": stream.codec,
                            "CodecLongName": stream.codec_long_name,
                            "Width": stream.width,
                            "Height": stream.height,
                            "AspectRatio": stream.aspect_ratio,
                            "FrameRate": stream.frame_rate,
                            "AvgFrameRate": stream.avg_frame_rate,
                            "Bitrate": stream.bitrate,
                            "PixelFormat": stream.pixel_format,
                            "ColorSpace": stream.color_space,
                            "ColorTransfer": stream.color_transfer,
                            "ColorPrimaries": stream.color_primaries,
                            "HdrType": stream.hdr_type,
                            "BitDepth": stream.bit_depth,
                            "Language": stream.language,
                            "Title": stream.title,
                            "IsDefault": stream.is_default,
                            "Metadata": stream.metadata,
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
                    r#"mutation CreateAudioStreamFromAnalysis($Input: CreateAudioStreamInput!) {
                        CreateAudioStream(Input: $Input) { Success Error }
                    }"#,
                    serde_json::json!({
                        "Input": {
                            "MediaFileId": media_file_id,
                            "StreamIndex": stream.stream_index,
                            "Codec": stream.codec,
                            "CodecLongName": stream.codec_long_name,
                            "Channels": stream.channels,
                            "ChannelLayout": stream.channel_layout,
                            "SampleRate": stream.sample_rate,
                            "Bitrate": stream.bitrate,
                            "BitDepth": stream.bit_depth,
                            "Language": stream.language,
                            "Title": stream.title,
                            "IsDefault": stream.is_default,
                            "IsCommentary": stream.is_commentary,
                            "Metadata": stream.metadata,
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
                    r#"mutation CreateSubtitleFromAnalysis($Input: CreateSubtitleInput!) {
                        CreateSubtitle(Input: $Input) { Success Error }
                    }"#,
                    serde_json::json!({
                        "Input": {
                            "MediaFileId": media_file_id,
                            "SourceType": subtitle.source_type,
                            "StreamIndex": subtitle.stream_index,
                            "Codec": subtitle.codec,
                            "CodecLongName": subtitle.codec_long_name,
                            "Language": subtitle.language,
                            "Title": subtitle.title,
                            "IsDefault": subtitle.is_default,
                            "IsForced": subtitle.is_forced,
                            "IsHearingImpaired": subtitle.is_hearing_impaired,
                            "Metadata": subtitle.metadata,
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
                    r#"mutation CreateMediaChapterFromAnalysis($Input: CreateMediaChapterInput!) {
                        CreateMediaChapter(Input: $Input) { Success Error }
                    }"#,
                    serde_json::json!({
                        "Input": {
                            "MediaFileId": media_file_id,
                            "ChapterIndex": chapter.chapter_index,
                            "StartSecs": chapter.start_secs,
                            "EndSecs": chapter.end_secs,
                            "Title": chapter.title,
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

            if let Err(e) = self
                .analyze_media_file_inner(&job.media_file_id, &job.path)
                .await
            {
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
        }
    }

    async fn get_media_file(&self, media_file_id: &str) -> Result<MediaFileRow> {
        let auth_user = self.system_auth_user(None).await?;
        let data = self
            .execute_graphql(
                &auth_user,
                r#"query MediaFileById($Id: String!) {
                    MediaFile(Id: $Id) {
                        Id
                        LibraryId
                        Path
                        OriginalName
                        Size
                        EpisodeId
                        MovieId
                        TrackId
                        ChapterId
                    }
                }"#,
                serde_json::json!({ "Id": media_file_id }),
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
            size: row.get("Size").and_then(|v| v.as_i64()).unwrap_or(0),
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
        })
    }

    async fn link_movie(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        movie_id: &str,
    ) -> Result<()> {
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation LinkMovieMediaFile($MovieId: String!, $MovieInput: UpdateMovieInput!, $MediaFileId: String!, $MediaInput: UpdateMediaFileInput!) {
                    UpdateMovie(Id: $MovieId, Input: $MovieInput) { Success Error }
                    UpdateMediaFile(Id: $MediaFileId, Input: $MediaInput) { Success Error }
                }"#,
                serde_json::json!({
                    "MovieId": movie_id,
                    "MovieInput": {
                        "MediaFileId": media_file_id,
                        "HasFile": true,
                        "Wanted": false,
                    },
                    "MediaFileId": media_file_id,
                    "MediaInput": {
                        "MovieId": movie_id,
                        "EpisodeId": null,
                        "TrackId": null,
                        "ChapterId": null,
                    }
                }),
            )
            .await?;

        let movie_ok = data
            .get("UpdateMovie")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !movie_ok {
            let err = data
                .get("UpdateMovie")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("failed to update movie link");
            anyhow::bail!(err.to_string());
        }

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
                .unwrap_or("failed to update media file movie link");
            anyhow::bail!(err.to_string());
        }

        Ok(())
    }

    async fn link_episode(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        episode_id: &str,
    ) -> Result<()> {
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation LinkEpisodeMediaFile($EpisodeId: String!, $EpisodeInput: UpdateEpisodeInput!, $MediaFileId: String!, $MediaInput: UpdateMediaFileInput!) {
                    UpdateEpisode(Id: $EpisodeId, Input: $EpisodeInput) { Success Error }
                    UpdateMediaFile(Id: $MediaFileId, Input: $MediaInput) { Success Error }
                }"#,
                serde_json::json!({
                    "EpisodeId": episode_id,
                    "EpisodeInput": {
                        "MediaFileId": media_file_id,
                        "Wanted": false,
                    },
                    "MediaFileId": media_file_id,
                    "MediaInput": {
                        "EpisodeId": episode_id,
                        "MovieId": null,
                        "TrackId": null,
                        "ChapterId": null,
                    }
                }),
            )
            .await?;

        let ep_ok = data
            .get("UpdateEpisode")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !ep_ok {
            let err = data
                .get("UpdateEpisode")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("failed to update episode link");
            anyhow::bail!(err.to_string());
        }

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
                .unwrap_or("failed to update media file episode link");
            anyhow::bail!(err.to_string());
        }

        Ok(())
    }

    async fn link_track(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        track_id: &str,
    ) -> Result<()> {
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation LinkTrackMediaFile($TrackId: String!, $TrackInput: UpdateTrackInput!, $MediaFileId: String!, $MediaInput: UpdateMediaFileInput!) {
                    UpdateTrack(Id: $TrackId, Input: $TrackInput) { Success Error }
                    UpdateMediaFile(Id: $MediaFileId, Input: $MediaInput) { Success Error }
                }"#,
                serde_json::json!({
                    "TrackId": track_id,
                    "TrackInput": {
                        "MediaFileId": media_file_id,
                        "Wanted": false,
                    },
                    "MediaFileId": media_file_id,
                    "MediaInput": {
                        "TrackId": track_id,
                        "MovieId": null,
                        "EpisodeId": null,
                        "ChapterId": null,
                    }
                }),
            )
            .await?;

        let track_ok = data
            .get("UpdateTrack")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !track_ok {
            let err = data
                .get("UpdateTrack")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("failed to update track link");
            anyhow::bail!(err.to_string());
        }

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
                .unwrap_or("failed to update media file track link");
            anyhow::bail!(err.to_string());
        }

        Ok(())
    }

    async fn link_chapter(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        chapter_id: &str,
    ) -> Result<()> {
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation LinkChapterMediaFile($ChapterId: String!, $ChapterInput: UpdateChapterInput!, $MediaFileId: String!, $MediaInput: UpdateMediaFileInput!) {
                    UpdateMediaFile(Id: $MediaFileId, Input: $MediaInput) { Success Error }
                    UpdateChapter(Id: $ChapterId, Input: $ChapterInput) { Success Error }
                }"#,
                serde_json::json!({
                    "ChapterId": chapter_id,
                    "ChapterInput": {
                        "MediaFileId": media_file_id,
                        "Wanted": false,
                    },
                    "MediaFileId": media_file_id,
                    "MediaInput": {
                        "ChapterId": chapter_id,
                        "MovieId": null,
                        "EpisodeId": null,
                        "TrackId": null,
                    }
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
                .unwrap_or("failed to update media file chapter link");
            anyhow::bail!(err.to_string());
        }

        let chapter_ok = data
            .get("UpdateChapter")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !chapter_ok {
            let err = data
                .get("UpdateChapter")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("failed to update chapter link");
            anyhow::bail!(err.to_string());
        }

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

        let show_name = Self::strip_release_tokens(&title_part);
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

    async fn find_movie_match(
        &self,
        library_id: &str,
        path: &str,
    ) -> Result<Option<(String, f64)>> {
        let auth_user = self.system_auth_user(None).await?;

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

        let data = self
            .execute_graphql(
                &auth_user,
                r#"query FindMovieMatch($LibraryId: String!) {
                    Movies(
                        Where: { LibraryId: { Eq: $LibraryId } }
                        Page: { Limit: 10000 }
                    ) {
                        Edges { Node { Id Title Year } }
                    }
                }"#,
                serde_json::json!({ "LibraryId": library_id }),
            )
            .await?;
        let rows: Vec<(String, String, Option<i32>)> = data
            .get("Movies")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                let node = edge.get("Node")?;
                Some((
                    node.get("Id")?.as_str()?.to_string(),
                    node.get("Title")?.as_str()?.to_string(),
                    node.get("Year").and_then(|v| v.as_i64()).map(|v| v as i32),
                ))
            })
            .collect();

        let mut best: Option<(String, f64)> = None;
        let hint_seq = hint
            .title
            .as_deref()
            .and_then(Self::extract_title_sequence_number);
        for (id, title, year) in rows {
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

            if score > best.as_ref().map(|(_, s)| *s).unwrap_or(0.0) {
                best = Some((id, score.min(1.0)));
            }
        }

        Ok(best.filter(|(_, s)| *s >= 0.78))
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
            .execute_graphql(
                &auth_user,
                r#"query FindEpisodeMatch($LibraryId: String!, $Season: Int!, $Episode: Int!) {
                    Episodes(
                        Where: {
                            Season: { Eq: $Season }
                            Episode: { Eq: $Episode }
                            Show: { LibraryId: { Eq: $LibraryId } }
                        }
                        Page: { Limit: 10000 }
                    ) {
                        Edges { Node { Id Show { Name Year } } }
                    }
                }"#,
                serde_json::json!({
                    "LibraryId": library_id,
                    "Season": season,
                    "Episode": episode,
                }),
            )
            .await?;
        let rows: Vec<(String, String, Option<i32>)> = data
            .get("Episodes")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                let node = edge.get("Node")?;
                let show = node.get("Show")?;
                Some((
                    node.get("Id")?.as_str()?.to_string(),
                    show.get("Name")?.as_str()?.to_string(),
                    show.get("Year").and_then(|v| v.as_i64()).map(|v| v as i32),
                ))
            })
            .collect();

        if rows.is_empty() {
            return Ok(None);
        }

        let mut best: Option<(String, f64)> = None;
        for (id, show_name, show_year) in rows {
            let show_norm = Self::normalize_for_match(&show_name);
            let mut score = if normalized.contains(&show_norm) {
                0.95
            } else {
                (jaro_winkler(&normalized, &show_norm) * 0.9).max(0.65)
            };
            if let Some(parsed_year) = hint.year {
                if let Some(y) = show_year {
                    if y == parsed_year {
                        score += 0.05;
                    }
                }
            }
            if score > best.as_ref().map(|(_, s)| *s).unwrap_or(0.0) {
                best = Some((id, score));
            }
        }

        Ok(best)
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

        match self
            .should_skip_auto_organize_for_match(
                library,
                auth_user,
                media_file_id,
                media_path,
                match_result,
            )
            .await
        {
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
                let _ = self.organize_media_file(media_file_id).await;
            }
            Err(e) => {
                warn!(
                    library_id = %library.id,
                    media_file_id = %media_file_id,
                    media_file_path = %media_path,
                    error = %e,
                    "Failed to evaluate auto-organize safety guard: library_id={}, media_file_id={}, path={}, error={}",
                    library.id,
                    media_file_id,
                    media_path,
                    e
                );
                let _ = self.organize_media_file(media_file_id).await;
            }
        }
    }

    async fn process_provider_fallback_batch(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        pending: Vec<PendingProviderFallback>,
    ) {
        if pending.is_empty() {
            return;
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
                }

                let grouped_items: Vec<Vec<PendingProviderFallback>> =
                    grouped.into_values().collect();
                stream::iter(grouped_items)
                    .map(|group| async move {
                        if group.is_empty() {
                            return;
                        }
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

                                let movie_id = result.matched_id.clone().unwrap_or_default();
                                for sibling in group.iter().skip(1) {
                                    if let Err(error) = self
                                        .link_movie(auth_user, &sibling.media_file_id, &movie_id)
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
                                        continue;
                                    }
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
                            Ok(Some(_)) | Ok(None) => {}
                            Err(error) => {
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
                    })
                    .buffer_unordered(fallback_workers)
                    .collect::<Vec<_>>()
                    .await;
            }
            "tv" | "music" | "audiobooks" => {
                let fallback_kind = normalized_library_type.clone();
                stream::iter(pending)
                    .map(|item| {
                        let fallback_kind = fallback_kind.clone();
                        async move {
                            let result = match fallback_kind.as_str() {
                            "tv" => {
                                self.try_provider_create_and_match_episode(
                                    library,
                                    auth_user,
                                    &item.media_file_id,
                                    &item.media_path,
                                    &item.match_source,
                                )
                                .await
                            }
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
                                self.maybe_auto_organize_after_scan_match(
                                    library,
                                    auth_user,
                                    &item.media_file_id,
                                    &item.media_path,
                                    &match_result,
                                )
                                .await;
                            }
                            Ok(None) => {}
                            Err(error) => {
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
                        }
                    })
                    .buffer_unordered(fallback_workers)
                    .collect::<Vec<_>>()
                    .await;
            }
            _ => {}
        }
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
        for (query, year) in &attempts {
            attempted_labels.push(match year {
                Some(y) => format!("query='{}',year={}", query, y),
                None => format!("query='{}'", query),
            });
            match metadata.search_movies(query, *year).await {
                Ok(found) => {
                    if !found.is_empty() {
                        candidates = found;
                        break;
                    }
                }
                Err(e) => {
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
        self.link_movie(auth_user, media_file_id, &movie.id).await?;

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
        let best = candidates.into_iter().max_by(|a, b| {
            let sa = {
                let mut s = jaro_winkler(&wanted_norm, &Self::normalize_for_match(&a.name));
                if hint.year.is_some() && a.year == hint.year {
                    s += 0.1;
                }
                s
            };
            let sb = {
                let mut s = jaro_winkler(&wanted_norm, &Self::normalize_for_match(&b.name));
                if hint.year.is_some() && b.year == hint.year {
                    s += 0.1;
                }
                s
            };
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        });

        let Some(candidate) = best else {
            return Ok(None);
        };

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
            self.link_episode(auth_user, media_file_id, &episode_id)
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
            .unwrap_or_else(|| format!("Track {}", track_number));
        let data = self
            .execute_mutation(
                auth_user,
                r#"mutation CreateTrackFromScan($Input: CreateTrackInput!) {
                    CreateTrack(Input: $Input) { Success Error Track { Id } }
                }"#,
                serde_json::json!({
                    "Input": {
                        "AlbumId": album_id,
                        "LibraryId": library_id,
                        "Title": title,
                        "TrackNumber": track_number,
                        "DiscNumber": 1,
                        "Explicit": false,
                        "Wanted": true
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
                r#"mutation CreateChapterFromScan($Input: CreateChapterInput!) {
                    CreateChapter(Input: $Input) { Success Error Chapter { Id } }
                }"#,
                serde_json::json!({
                    "Input": {
                        "AudiobookId": audiobook_id,
                        "ChapterNumber": chapter_number,
                        "Title": hint.chapter_title.clone().unwrap_or_else(|| format!("Chapter {}", chapter_number)),
                        "StartTimeSecs": 0.0,
                        "Wanted": true
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

        let best = candidates.into_iter().max_by(|a, b| {
            let score = |title: &str, artist: Option<&str>| {
                let mut s = jaro_winkler(&wanted_album_norm, &Self::normalize_for_match(title));
                if let (Some(wanted_artist), Some(candidate_artist)) =
                    (wanted_artist_norm.as_deref(), artist)
                {
                    s +=
                        (jaro_winkler(wanted_artist, &Self::normalize_for_match(candidate_artist))
                            * 0.25)
                            .min(0.25);
                }
                s
            };
            score(&a.title, a.artist_name.as_deref())
                .partial_cmp(&score(&b.title, b.artist_name.as_deref()))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let Some(candidate) = best else {
            return Ok(None);
        };

        let library_uuid = Uuid::parse_str(&library.id)?;
        let user_uuid = Uuid::parse_str(&auth_user.user_id)?;
        let album = metadata
            .add_album_from_provider(AddAlbumOptions {
                provider: MetadataProvider::Musicbrainz,
                provider_id: candidate.provider_id,
                library_id: library_uuid,
                user_id: user_uuid,
            })
            .await?;

        if let Some((track_id, score)) = self.find_track_match(&library.id, media_path).await? {
            self.link_track(auth_user, media_file_id, &track_id).await?;
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
        self.link_track(auth_user, media_file_id, &track_id).await?;
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

        let best = candidates.into_iter().max_by(|a, b| {
            let score = |title: &str, author: Option<&str>| {
                let mut s = jaro_winkler(&wanted_title_norm, &Self::normalize_for_match(title));
                if let (Some(wanted_author), Some(candidate_author)) =
                    (wanted_author_norm.as_deref(), author)
                {
                    s +=
                        (jaro_winkler(wanted_author, &Self::normalize_for_match(candidate_author))
                            * 0.25)
                            .min(0.25);
                }
                s
            };
            score(&a.title, a.author_name.as_deref())
                .partial_cmp(&score(&b.title, b.author_name.as_deref()))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let Some(candidate) = best else {
            return Ok(None);
        };

        let library_uuid = Uuid::parse_str(&library.id)?;
        let user_uuid = Uuid::parse_str(&auth_user.user_id)?;
        let audiobook = metadata
            .add_audiobook_from_provider(AddAudiobookOptions {
                provider: MetadataProvider::OpenLibrary,
                provider_id: candidate.provider_id,
                library_id: library_uuid,
                user_id: user_uuid,
            })
            .await?;

        if let Some((chapter_id, score)) = self.find_chapter_match(&library.id, media_path).await? {
            self.link_chapter(auth_user, media_file_id, &chapter_id)
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
        self.link_chapter(auth_user, media_file_id, &chapter_id)
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
            .execute_graphql(
                &auth_user,
                r#"query FindTrackMatch($LibraryId: String!) {
                    Tracks(
                        Where: { LibraryId: { Eq: $LibraryId } }
                        Page: { Limit: 10000 }
                    ) {
                        Edges { Node { Id Title Album { Name } } }
                    }
                }"#,
                serde_json::json!({ "LibraryId": library_id }),
            )
            .await?;
        let rows: Vec<(String, String, Option<String>)> = data
            .get("Tracks")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                let node = edge.get("Node")?;
                Some((
                    node.get("Id")?.as_str()?.to_string(),
                    node.get("Title")?.as_str()?.to_string(),
                    node.get("Album")
                        .and_then(|a| a.get("Name"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                ))
            })
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
            .execute_graphql(
                &auth_user,
                r#"query FindChapterMatch($LibraryId: String!) {
                    Chapters(
                        Where: { Audiobook: { LibraryId: { Eq: $LibraryId } } }
                        Page: { Limit: 10000 }
                    ) {
                        Edges {
                            Node {
                                Id
                                Title
                                ChapterNumber
                                Audiobook {
                                    Title
                                    AuthorName
                                }
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "LibraryId": library_id }),
            )
            .await?;
        let rows: Vec<(String, Option<String>, i32, String, Option<String>)> = data
            .get("Chapters")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                let node = edge.get("Node")?;
                let book = node.get("Audiobook")?;
                Some((
                    node.get("Id")?.as_str()?.to_string(),
                    node.get("Title")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    node.get("ChapterNumber")?.as_i64()? as i32,
                    book.get("Title")?.as_str()?.to_string(),
                    book.get("AuthorName")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                ))
            })
            .collect();

        let mut best: Option<(String, f64)> = None;
        for (id, chapter_title, chapter_number, book_title, author_name) in rows {
            let mut score = 0.0;
            if let Some(num) = chapter_num {
                if num == chapter_number {
                    score += 0.5;
                }
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
            .execute_graphql(
                auth_user,
                r#"query MatchMovieCandidates($LibraryId: String!) {
                    Movies(
                        Where: { LibraryId: { Eq: $LibraryId } }
                        Page: { Limit: 10000 }
                    ) {
                        Edges { Node { Id Title Year Wanted HasFile } }
                    }
                }"#,
                serde_json::json!({ "LibraryId": library_id }),
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
            .execute_graphql(
                auth_user,
                r#"query MatchEpisodeCandidates($LibraryId: String!) {
                    Episodes(
                        Where: { Show: { LibraryId: { Eq: $LibraryId } } }
                        Page: { Limit: 10000 }
                    ) {
                        Edges { Node { Id Season Episode Wanted MediaFileId Show { Name Year } } }
                    }
                }"#,
                serde_json::json!({ "LibraryId": library_id }),
            )
            .await?;

        let mut rows = Vec::new();
        for edge in data
            .get("Episodes")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            let Some(node) = edge.get("Node") else {
                continue;
            };
            let Some(show) = node.get("Show") else {
                continue;
            };
            let Some(id) = node.get("Id").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(show_name) = show.get("Name").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(season) = node.get("Season").and_then(|v| v.as_i64()) else {
                continue;
            };
            let Some(episode) = node.get("Episode").and_then(|v| v.as_i64()) else {
                continue;
            };

            rows.push(EpisodeCandidateRow {
                id: id.to_string(),
                show_name: show_name.to_string(),
                show_year: show.get("Year").and_then(|v| v.as_i64()).map(|v| v as i32),
                season: season as i32,
                episode: episode as i32,
                wanted: node
                    .get("Wanted")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                has_file: node
                    .get("MediaFileId")
                    .and_then(|v| v.as_str())
                    .map(|s| !s.is_empty())
                    .unwrap_or(false),
            });
        }

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
            .execute_graphql(
                auth_user,
                r#"query MatchTrackCandidates($LibraryId: String!) {
                    Tracks(
                        Where: { LibraryId: { Eq: $LibraryId } }
                        Page: { Limit: 10000 }
                    ) {
                        Edges { Node { Id Title Wanted MediaFileId Album { Name } } }
                    }
                }"#,
                serde_json::json!({ "LibraryId": library_id }),
            )
            .await?;

        let mut rows = Vec::new();
        for edge in data
            .get("Tracks")
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

            rows.push(TrackCandidateRow {
                id: id.to_string(),
                title: title.to_string(),
                album_name: node
                    .get("Album")
                    .and_then(|a| a.get("Name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                wanted: node
                    .get("Wanted")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                has_file: node
                    .get("MediaFileId")
                    .and_then(|v| v.as_str())
                    .map(|s| !s.is_empty())
                    .unwrap_or(false),
            });
        }

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
            .execute_graphql(
                auth_user,
                r#"query MatchChapterCandidates($LibraryId: String!) {
                    Chapters(
                        Where: { Audiobook: { LibraryId: { Eq: $LibraryId } } }
                        Page: { Limit: 10000 }
                    ) {
                        Edges {
                            Node {
                                Id
                                Title
                                ChapterNumber
                                Wanted
                                MediaFileId
                                Audiobook { Title AuthorName }
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "LibraryId": library_id }),
            )
            .await?;

        let mut rows = Vec::new();
        for edge in data
            .get("Chapters")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            let Some(node) = edge.get("Node") else {
                continue;
            };
            let Some(book) = node.get("Audiobook") else {
                continue;
            };
            let Some(id) = node.get("Id").and_then(|v| v.as_str()) else {
                continue;
            };

            rows.push(ChapterCandidateRow {
                id: id.to_string(),
                title: node
                    .get("Title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                audiobook_title: book
                    .get("Title")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                author_name: book
                    .get("AuthorName")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string()),
                chapter_number: node
                    .get("ChapterNumber")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32),
                wanted: node
                    .get("Wanted")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                has_file: node
                    .get("MediaFileId")
                    .and_then(|v| v.as_str())
                    .map(|s| !s.is_empty())
                    .unwrap_or(false),
            });
        }

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

            let show_norm = Self::normalize_for_match(&show_name);
            let mut score = if normalized.contains(&show_norm) {
                0.95
            } else {
                (jaro_winkler(&normalized, &show_norm) * 0.9).max(0.65)
            };
            if let Some(parsed_year) = hint.year
                && let Some(y) = show_year
                && y == parsed_year
            {
                score += 0.05;
            }
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
                    format!("Chapter {}", chapter_number)
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

    async fn apply_link_for_candidate(
        &self,
        auth_user: &AuthUser,
        media_file_id: &str,
        candidate: &MatchCandidate,
    ) -> Result<()> {
        match candidate.target_type.as_str() {
            "Movie" => {
                self.link_movie(auth_user, media_file_id, &candidate.target_id)
                    .await
            }
            "Episode" => {
                self.link_episode(auth_user, media_file_id, &candidate.target_id)
                    .await
            }
            "Track" => {
                self.link_track(auth_user, media_file_id, &candidate.target_id)
                    .await
            }
            "Chapter" => {
                self.link_chapter(auth_user, media_file_id, &candidate.target_id)
                    .await
            }
            other => anyhow::bail!("unsupported candidate target type: {}", other),
        }
    }

    pub async fn unmatch_media_file(&self, media_file_id: &str) -> Result<()> {
        let media_file = self.get_media_file(media_file_id).await?;
        let auth_user = self.system_auth_user(None).await?;

        if let Some(movie_id) = media_file.movie_id.as_deref() {
            let data = self
                .execute_mutation(
                    &auth_user,
                    r#"mutation UnmatchMovieMediaFile($MovieId: String!, $Input: UpdateMovieInput!) {
                        UpdateMovie(Id: $MovieId, Input: $Input) { Success Error }
                    }"#,
                    serde_json::json!({
                        "MovieId": movie_id,
                        "Input": {
                            "MediaFileId": null,
                            "HasFile": false,
                            "Wanted": true,
                        }
                    }),
                )
                .await?;
            let ok = data
                .get("UpdateMovie")
                .and_then(|v| v.get("Success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !ok {
                let err = data
                    .get("UpdateMovie")
                    .and_then(|v| v.get("Error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("failed to clear movie link");
                anyhow::bail!(err.to_string());
            }
        }

        if let Some(episode_id) = media_file.episode_id.as_deref() {
            let data = self
                .execute_mutation(
                    &auth_user,
                    r#"mutation UnmatchEpisodeMediaFile($EpisodeId: String!, $Input: UpdateEpisodeInput!) {
                        UpdateEpisode(Id: $EpisodeId, Input: $Input) { Success Error }
                    }"#,
                    serde_json::json!({
                        "EpisodeId": episode_id,
                        "Input": {
                            "MediaFileId": null,
                            "Wanted": true,
                        }
                    }),
                )
                .await?;
            let ok = data
                .get("UpdateEpisode")
                .and_then(|v| v.get("Success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !ok {
                let err = data
                    .get("UpdateEpisode")
                    .and_then(|v| v.get("Error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("failed to clear episode link");
                anyhow::bail!(err.to_string());
            }
        }

        if let Some(track_id) = media_file.track_id.as_deref() {
            let data = self
                .execute_mutation(
                    &auth_user,
                    r#"mutation UnmatchTrackMediaFile($TrackId: String!, $Input: UpdateTrackInput!) {
                        UpdateTrack(Id: $TrackId, Input: $Input) { Success Error }
                    }"#,
                    serde_json::json!({
                        "TrackId": track_id,
                        "Input": {
                            "MediaFileId": null,
                            "Wanted": true,
                        }
                    }),
                )
                .await?;
            let ok = data
                .get("UpdateTrack")
                .and_then(|v| v.get("Success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !ok {
                let err = data
                    .get("UpdateTrack")
                    .and_then(|v| v.get("Error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("failed to clear track link");
                anyhow::bail!(err.to_string());
            }
        }

        if let Some(chapter_id) = media_file.chapter_id.as_deref() {
            let data = self
                .execute_mutation(
                    &auth_user,
                    r#"mutation UnmatchChapterMediaFile($ChapterId: String!, $Input: UpdateChapterInput!) {
                        UpdateChapter(Id: $ChapterId, Input: $Input) { Success Error }
                    }"#,
                    serde_json::json!({
                        "ChapterId": chapter_id,
                        "Input": {
                            "MediaFileId": null,
                            "Wanted": true,
                        }
                    }),
                )
                .await?;
            let ok = data
                .get("UpdateChapter")
                .and_then(|v| v.get("Success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !ok {
                let err = data
                    .get("UpdateChapter")
                    .and_then(|v| v.get("Error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("failed to clear chapter link");
                anyhow::bail!(err.to_string());
            }
        }

        let data = self
            .execute_mutation(
                &auth_user,
                r#"mutation UnmatchMediaFile($MediaFileId: String!, $Input: UpdateMediaFileInput!) {
                    UpdateMediaFile(Id: $MediaFileId, Input: $Input) { Success Error }
                }"#,
                serde_json::json!({
                    "MediaFileId": media_file_id,
                    "Input": {
                        "MovieId": null,
                        "EpisodeId": null,
                        "TrackId": null,
                        "ChapterId": null,
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
        if method_set.remove(&MatchMethod::Ollama) {
            debug!(
                media_file_id = %request.media_file_id,
                library_id = %library_id,
                "Ollama match method requested but not enabled for media_file_id={} in library_id={}",
                request.media_file_id,
                library_id
            );
        }

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
            self.link_movie(&auth_user, &request.media_file_id, movie_id)
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
            self.link_episode(&auth_user, &request.media_file_id, episode_id)
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
            self.link_track(&auth_user, &request.media_file_id, track_id)
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
            self.link_chapter(&auth_user, &request.media_file_id, chapter_id)
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
            candidates = match normalized_library_type.as_str() {
                "movies" => {
                    self.collect_movie_candidates(
                        &library_id,
                        match_source,
                        candidate_limit,
                        wanted_policy,
                    )
                    .await?
                }
                "tv" => {
                    self.collect_episode_candidates(
                        &library_id,
                        match_source,
                        candidate_limit,
                        wanted_policy,
                    )
                    .await?
                }
                "music" => {
                    self.collect_track_candidates(
                        &library_id,
                        match_source,
                        candidate_limit,
                        wanted_policy,
                    )
                    .await?
                }
                "audiobooks" => {
                    self.collect_chapter_candidates(
                        &library_id,
                        match_source,
                        candidate_limit,
                        wanted_policy,
                    )
                    .await?
                }
                _ => Vec::new(),
            };
        }

        if request.auto_match
            && let Some(best) = candidates.first()
            && best.score >= 0.7
        {
            self.apply_link_for_candidate(&auth_user, &request.media_file_id, best)
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
        let normalized_library_type = Self::normalize_library_type(&library.library_type);
        let naming_pattern = self.resolve_library_naming_pattern(&library).await?;

        let old_path = PathBuf::from(&media_file.path);
        if !old_path.exists() {
            return Ok(OrganizeResult {
                success: false,
                old_path: Some(media_file.path),
                new_path: None,
                reason: Some("Source file does not exist".to_string()),
            });
        }

        let ext = old_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mkv");
        let original_filename = old_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");

        let target_path = match normalized_library_type.as_str() {
            "movies" => {
                if let Some(movie_id) = media_file.movie_id {
                    let data = self
                        .execute_graphql(
                            &auth_user,
                            r#"query OrganizeMovieInfo($Id: String!) {
                                Movie(Id: $Id) { Title Year }
                            }"#,
                            serde_json::json!({ "Id": movie_id }),
                        )
                        .await?;
                    let movie = data.get("Movie");
                    if let Some(title) = movie.and_then(|m| m.get("Title")).and_then(|v| v.as_str())
                    {
                        let year = movie
                            .and_then(|m| m.get("Year"))
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32);
                        PathBuf::from(&library.path).join(apply_movie_naming_pattern(
                            &naming_pattern,
                            title,
                            year,
                            original_filename,
                            ext,
                        ))
                    } else {
                        return Ok(OrganizeResult {
                            success: false,
                            old_path: Some(media_file.path),
                            new_path: None,
                            reason: Some("Movie relation not found for organizing".to_string()),
                        });
                    }
                } else {
                    return Ok(OrganizeResult {
                        success: false,
                        old_path: Some(media_file.path),
                        new_path: None,
                        reason: Some("Media file is not matched to a movie".to_string()),
                    });
                }
            }
            "tv" => {
                let episode_id = match media_file.episode_id {
                    Some(id) => id,
                    None => {
                        return Ok(OrganizeResult {
                            success: false,
                            old_path: Some(media_file.path),
                            new_path: None,
                            reason: Some("Media file is not matched to an episode".to_string()),
                        });
                    }
                };

                let data = self
                    .execute_graphql(
                        &auth_user,
                        r#"query OrganizeEpisodeInfo($Id: String!) {
                            Episode(Id: $Id) {
                                Season
                                Episode
                                Title
                                Show { Name }
                            }
                        }"#,
                        serde_json::json!({ "Id": episode_id }),
                    )
                    .await?;
                let episode_node = data.get("Episode");
                if let (Some(show_name), Some(season), Some(episode)) = (
                    episode_node
                        .and_then(|e| e.get("Show"))
                        .and_then(|s| s.get("Name"))
                        .and_then(|v| v.as_str()),
                    episode_node
                        .and_then(|e| e.get("Season"))
                        .and_then(|v| v.as_i64()),
                    episode_node
                        .and_then(|e| e.get("Episode"))
                        .and_then(|v| v.as_i64()),
                ) {
                    let episode_title = episode_node
                        .and_then(|e| e.get("Title"))
                        .and_then(|v| v.as_str());
                    PathBuf::from(&library.path).join(apply_tv_naming_pattern(
                        &naming_pattern,
                        show_name,
                        season as i32,
                        episode as i32,
                        episode_title,
                        ext,
                    ))
                } else {
                    return Ok(OrganizeResult {
                        success: false,
                        old_path: Some(media_file.path),
                        new_path: None,
                        reason: Some("Episode relation not found for organizing".to_string()),
                    });
                }
            }
            "music" => {
                let track_id = match media_file.track_id {
                    Some(id) => id,
                    None => {
                        return Ok(OrganizeResult {
                            success: false,
                            old_path: Some(media_file.path),
                            new_path: None,
                            reason: Some("Media file is not matched to a track".to_string()),
                        });
                    }
                };

                let data = self
                    .execute_graphql(
                        &auth_user,
                        r#"query OrganizeTrackInfo($Id: String!) {
                            Track(Id: $Id) {
                                Title
                                TrackNumber
                                DiscNumber
                                ArtistName
                                Album { Name Year }
                            }
                        }"#,
                        serde_json::json!({ "Id": track_id }),
                    )
                    .await?;

                let track = data.get("Track");
                if let (Some(track_title), Some(album_name), Some(track_number)) = (
                    track.and_then(|t| t.get("Title")).and_then(|v| v.as_str()),
                    track
                        .and_then(|t| t.get("Album"))
                        .and_then(|a| a.get("Name"))
                        .and_then(|v| v.as_str()),
                    track
                        .and_then(|t| t.get("TrackNumber"))
                        .and_then(|v| v.as_i64()),
                ) {
                    let album_year = track
                        .and_then(|t| t.get("Album"))
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
                    PathBuf::from(&library.path).join(apply_music_naming_pattern(
                        &naming_pattern,
                        artist_name.unwrap_or("Unknown Artist"),
                        album_name,
                        album_year,
                        track_number as i32,
                        disc_number,
                        track_title,
                        original_filename,
                        ext,
                    ))
                } else {
                    return Ok(OrganizeResult {
                        success: false,
                        old_path: Some(media_file.path),
                        new_path: None,
                        reason: Some("Track relation not found for organizing".to_string()),
                    });
                }
            }
            "audiobooks" => {
                let data = self
                    .execute_graphql(
                        &auth_user,
                        r#"query OrganizeChapterInfo($MediaFileId: String!) {
                            Chapters(
                                Where: { MediaFileId: { Eq: $MediaFileId } }
                                Page: { Limit: 1 }
                            ) {
                                Edges {
                                    Node {
                                        ChapterNumber
                                        Title
                                        Audiobook { Title AuthorName }
                                    }
                                }
                            }
                        }"#,
                        serde_json::json!({ "MediaFileId": media_file_id }),
                    )
                    .await?;

                let chapter_node = data
                    .get("Chapters")
                    .and_then(|v| v.get("Edges"))
                    .and_then(|v| v.as_array())
                    .and_then(|edges| edges.first())
                    .and_then(|edge| edge.get("Node"));

                if let (Some(chapter_number), Some(book_title)) = (
                    chapter_node
                        .and_then(|c| c.get("ChapterNumber"))
                        .and_then(|v| v.as_i64()),
                    chapter_node
                        .and_then(|c| c.get("Audiobook"))
                        .and_then(|a| a.get("Title"))
                        .and_then(|v| v.as_str()),
                ) {
                    let chapter_title = chapter_node
                        .and_then(|c| c.get("Title"))
                        .and_then(|v| v.as_str());
                    let author_name = chapter_node
                        .and_then(|c| c.get("Audiobook"))
                        .and_then(|a| a.get("AuthorName"))
                        .and_then(|v| v.as_str());
                    PathBuf::from(&library.path).join(apply_audiobook_naming_pattern(
                        &naming_pattern,
                        author_name.unwrap_or("Unknown Author"),
                        book_title,
                        chapter_number as i32,
                        chapter_title,
                        original_filename,
                        ext,
                    ))
                } else {
                    return Ok(OrganizeResult {
                        success: false,
                        old_path: Some(media_file.path),
                        new_path: None,
                        reason: Some(
                            "Media file is not matched to an audiobook chapter".to_string(),
                        ),
                    });
                }
            }
            _ => {
                return Ok(OrganizeResult {
                    success: false,
                    old_path: Some(media_file.path),
                    new_path: None,
                    reason: Some("Unsupported library type for organizing".to_string()),
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

        if target_path.exists() {
            let old_canonical = tokio::fs::canonicalize(&old_path).await.ok();
            let target_canonical = tokio::fs::canonicalize(&target_path).await.ok();
            if old_canonical.is_some() && old_canonical == target_canonical {
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

        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::rename(&old_path, &target_path).await?;

        let data = self
            .execute_mutation(
                &auth_user,
                r#"mutation UpdateMediaFilePath($Id: String!, $Input: UpdateMediaFileInput!) {
                    UpdateMediaFile(Id: $Id, Input: $Input) { Success Error }
                }"#,
                serde_json::json!({
                    "Id": media_file_id,
                    "Input": {
                        "Path": target_path.to_string_lossy().to_string(),
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
                .unwrap_or("Failed to update organized media path");
            anyhow::bail!(err.to_string());
        }

        Ok(OrganizeResult {
            success: true,
            old_path: Some(old_path.to_string_lossy().to_string()),
            new_path: Some(target_path.to_string_lossy().to_string()),
            reason: None,
        })
    }

    async fn reconcile_missing_media_files(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        discovered_paths: &HashSet<String>,
    ) -> Result<()> {
        let data = self
            .execute_graphql(
                auth_user,
                r#"query ReconcileMediaFiles($LibraryId: String!) {
                    MediaFiles(
                        Where: { LibraryId: { Eq: $LibraryId } }
                        Page: { Limit: 10000 }
                    ) {
                        Edges {
                            Node {
                                Id
                                Path
                                MovieId
                                EpisodeId
                                TrackId
                                ChapterId
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "LibraryId": library.id }),
            )
            .await?;

        let rows: Vec<ExistingMediaFileRow> = data
            .get("MediaFiles")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                let n = edge.get("Node")?;
                Some(ExistingMediaFileRow {
                    id: n.get("Id")?.as_str()?.to_string(),
                    path: n.get("Path")?.as_str()?.to_string(),
                    movie_id: n
                        .get("MovieId")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    episode_id: n
                        .get("EpisodeId")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    track_id: n
                        .get("TrackId")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    chapter_id: n
                        .get("ChapterId")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
            })
            .collect();

        for row in rows {
            if discovered_paths.contains(&row.path) || Path::new(&row.path).exists() {
                continue;
            }

            if let Some(movie_id) = &row.movie_id {
                let _ = self
                    .execute_mutation(
                        auth_user,
                        r#"mutation ClearMovieFile($Id: String!, $Input: UpdateMovieInput!) {
                            UpdateMovie(Id: $Id, Input: $Input) { Success Error }
                        }"#,
                        serde_json::json!({
                            "Id": movie_id,
                            "Input": {
                                "MediaFileId": null,
                                "HasFile": false,
                                "Wanted": true,
                            }
                        }),
                    )
                    .await;
            }

            if let Some(episode_id) = &row.episode_id {
                let _ = self
                    .execute_mutation(
                        auth_user,
                        r#"mutation ClearEpisodeFile($Id: String!, $Input: UpdateEpisodeInput!) {
                            UpdateEpisode(Id: $Id, Input: $Input) { Success Error }
                        }"#,
                        serde_json::json!({
                            "Id": episode_id,
                            "Input": {
                                "MediaFileId": null,
                                "Wanted": true,
                            }
                        }),
                    )
                    .await;
            }

            if let Some(track_id) = &row.track_id {
                let _ = self
                    .execute_mutation(
                        auth_user,
                        r#"mutation ClearTrackFile($Id: String!, $Input: UpdateTrackInput!) {
                            UpdateTrack(Id: $Id, Input: $Input) { Success Error }
                        }"#,
                        serde_json::json!({
                            "Id": track_id,
                            "Input": {
                                "MediaFileId": null,
                                "Wanted": true,
                            }
                        }),
                    )
                    .await;
            }

            if let Some(chapter_id) = &row.chapter_id {
                let _ = self
                    .execute_mutation(
                        auth_user,
                        r#"mutation ClearChapterById($Id: String!, $Input: UpdateChapterInput!) {
                            UpdateChapter(Id: $Id, Input: $Input) { Success Error }
                        }"#,
                        serde_json::json!({
                            "Id": chapter_id,
                            "Input": {
                                "MediaFileId": null,
                                "Wanted": true,
                            }
                        }),
                    )
                    .await;
            }

            let _ = self
                .execute_mutation(
                    auth_user,
                    r#"mutation ClearChapterFiles($Where: ChapterWhereInput!, $Input: UpdateChapterInput!) {
                        UpdateChapters(Where: $Where, Input: $Input) { success error affectedCount }
                    }"#,
                    serde_json::json!({
                        "Where": {
                            "MediaFileId": { "Eq": row.id }
                        },
                        "Input": {
                            "MediaFileId": null,
                            "Wanted": true
                        }
                    }),
                )
                .await;

            let _ = self
                .execute_mutation(
                    auth_user,
                    r#"mutation DeleteMissingMediaFile($Id: String!) {
                        DeleteMediaFile(Id: $Id) { Success Error }
                    }"#,
                    serde_json::json!({ "Id": row.id }),
                )
                .await;
        }

        Ok(())
    }

    async fn get_tv_shows_for_library(
        &self,
        auth_user: &AuthUser,
        library_id: &str,
    ) -> Result<Vec<(String, Option<String>, Option<i32>, Vec<i32>)>> {
        let data = self
            .execute_graphql(
                auth_user,
                r#"query TvShowsForLibrary($LibraryId: String!) {
                    Shows(
                        Where: { LibraryId: { Eq: $LibraryId } }
                        Page: { Limit: 10000 }
                    ) {
                        Edges {
                            Node {
                                Name
                                Path
                                Year
                                Episodes(Page: { Limit: 10000 }) {
                                    Edges { Node { Season } }
                                }
                            }
                        }
                    }
                }"#,
                serde_json::json!({ "LibraryId": library_id }),
            )
            .await?;

        Ok(data
            .get("Shows")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                let node = edge.get("Node")?;
                let show_name = node.get("Name")?.as_str()?.to_string();
                let show_path = node
                    .get("Path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let show_year = node.get("Year").and_then(|v| v.as_i64()).map(|v| v as i32);
                let mut seasons: Vec<i32> = node
                    .get("Episodes")
                    .and_then(|v| v.get("Edges"))
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|e| {
                        e.get("Node")
                            .and_then(|n| n.get("Season"))
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32)
                    })
                    .collect();
                seasons.sort_unstable();
                seasons.dedup();
                Some((show_name, show_path, show_year, seasons))
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

        let mut dirs: Vec<PathBuf> = WalkDir::new(&library_root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
            .map(|e| e.into_path())
            .collect();

        dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));

        for dir in dirs {
            if dir == library_root || protected.contains(&dir) {
                continue;
            }

            let is_empty = tokio::fs::read_dir(&dir)
                .await
                .map(|mut rd| async move { rd.next_entry().await.ok().flatten().is_none() })
                .ok();

            if let Some(check) = is_empty {
                if check.await {
                    let _ = tokio::fs::remove_dir(&dir).await;
                }
            }
        }

        Ok(())
    }

    async fn build_tv_protected_paths(&self, library: &LibraryRow) -> Result<HashSet<PathBuf>> {
        let auth_user = self.system_auth_user(Some(&library.user_id)).await?;
        let naming_pattern = self.resolve_library_naming_pattern(library).await?;

        let mut protected = HashSet::new();
        let root = PathBuf::from(&library.path);
        protected.insert(root.clone());

        let shows = self
            .get_tv_shows_for_library(&auth_user, &library.id)
            .await?;
        for (show_name, show_path, show_year, seasons) in shows {
            if let Some(path) = show_path {
                let show_dir = PathBuf::from(path);
                protected.insert(show_dir.clone());

                if show_dir.exists() {
                    let mut rd = match tokio::fs::read_dir(&show_dir).await {
                        Ok(rd) => rd,
                        Err(_) => continue,
                    };
                    while let Ok(Some(entry)) = rd.next_entry().await {
                        if !entry.path().is_dir() {
                            continue;
                        }
                        let folder_name = entry.file_name().to_string_lossy().to_string();
                        if folder_name.starts_with("Season ")
                            || folder_name.to_ascii_lowercase().starts_with('s')
                        {
                            protected.insert(entry.path());
                        }
                    }
                }
            }

            if seasons.is_empty() {
                let (show_dir, _) = derive_tv_dirs_from_pattern(
                    &library.path,
                    &naming_pattern,
                    &show_name,
                    1,
                    show_year,
                );
                protected.insert(show_dir);
            } else {
                for season in seasons {
                    let (show_dir, season_dir) = derive_tv_dirs_from_pattern(
                        &library.path,
                        &naming_pattern,
                        &show_name,
                        season,
                        show_year,
                    );
                    protected.insert(show_dir);
                    protected.insert(season_dir);
                }
            }
        }

        Ok(protected)
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
            .execute_graphql(
                &auth_user,
                r#"query ProtectedMoviePaths($LibraryId: String!) {
                    Movies(Where: { LibraryId: { Eq: $LibraryId } }, Page: { Limit: 10000 }) {
                        Edges {
                            Node {
                                Title
                                Year
                            }
                        }
                    }
                }"#,
                serde_json::json!({
                    "LibraryId": library.id,
                }),
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
            let rel = apply_movie_naming_pattern(&naming_pattern, &title, year, "dummy.mkv", "mkv");
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
            .execute_graphql(
                &auth_user,
                r#"query ProtectedMusicPaths($LibraryId: String!) {
                    Albums(Where: { LibraryId: { Eq: $LibraryId } }, Page: { Limit: 10000 }) {
                        Edges {
                            Node {
                                Name
                                Year
                                Tracks(Page: { Limit: 10000 }) {
                                    Edges {
                                        Node {
                                            Title
                                            TrackNumber
                                            DiscNumber
                                            ArtistName
                                            MediaFile {
                                                Path
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }"#,
                serde_json::json!({
                    "LibraryId": library.id,
                }),
            )
            .await?;

        let album_edges = data
            .get("Albums")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for album_edge in album_edges {
            let album_node = album_edge.get("Node").cloned().unwrap_or_default();
            let Some(album_name) = album_node.get("Name").and_then(|v| v.as_str()) else {
                continue;
            };
            let album_year = album_node
                .get("Year")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32);
            let track_edges = album_node
                .get("Tracks")
                .and_then(|v| v.get("Edges"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for track_edge in track_edges {
                let track_node = track_edge.get("Node").cloned().unwrap_or_default();
                let Some(track_title) = track_node.get("Title").and_then(|v| v.as_str()) else {
                    continue;
                };
                let Some(track_number) = track_node.get("TrackNumber").and_then(|v| v.as_i64())
                else {
                    continue;
                };
                let disc_number = track_node
                    .get("DiscNumber")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32);
                let artist_name = track_node.get("ArtistName").and_then(|v| v.as_str());
                let file_path = track_node
                    .get("MediaFile")
                    .and_then(|v| v.get("Path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("track.mp3");

                let original_filename = Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("track.mp3");
                let rel = apply_music_naming_pattern(
                    &naming_pattern,
                    artist_name.unwrap_or("Unknown Artist"),
                    album_name,
                    album_year,
                    track_number as i32,
                    disc_number,
                    track_title,
                    original_filename,
                    "mp3",
                );
                if let Some(parent) = root.join(rel).parent().map(|p| p.to_path_buf()) {
                    self.protect_path_and_ancestors(&mut protected, &root, parent);
                }
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
            .execute_graphql(
                &auth_user,
                r#"query ProtectedAudiobookPaths($LibraryId: String!) {
                    Audiobooks(Where: { LibraryId: { Eq: $LibraryId } }, Page: { Limit: 10000 }) {
                        Edges {
                            Node {
                                Title
                                AuthorName
                                Chapters(Page: { Limit: 10000 }) {
                                    Edges {
                                        Node {
                                            ChapterNumber
                                            Title
                                            MediaFile {
                                                Path
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }"#,
                serde_json::json!({
                    "LibraryId": library.id,
                }),
            )
            .await?;

        let audiobook_edges = data
            .get("Audiobooks")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for audiobook_edge in audiobook_edges {
            let audiobook_node = audiobook_edge.get("Node").cloned().unwrap_or_default();
            let Some(book_title) = audiobook_node.get("Title").and_then(|v| v.as_str()) else {
                continue;
            };
            let author_name = audiobook_node.get("AuthorName").and_then(|v| v.as_str());
            let chapter_edges = audiobook_node
                .get("Chapters")
                .and_then(|v| v.get("Edges"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            for chapter_edge in chapter_edges {
                let chapter_node = chapter_edge.get("Node").cloned().unwrap_or_default();
                let Some(chapter_number) =
                    chapter_node.get("ChapterNumber").and_then(|v| v.as_i64())
                else {
                    continue;
                };
                let chapter_title = chapter_node.get("Title").and_then(|v| v.as_str());
                let file_path = chapter_node
                    .get("MediaFile")
                    .and_then(|v| v.get("Path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("chapter.m4b");
                let original_filename = Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("chapter.m4b");
                let rel = apply_audiobook_naming_pattern(
                    &naming_pattern,
                    author_name.unwrap_or("Unknown Author"),
                    book_title,
                    chapter_number as i32,
                    chapter_title,
                    original_filename,
                    "m4b",
                );
                if let Some(parent) = root.join(rel).parent().map(|p| p.to_path_buf()) {
                    self.protect_path_and_ancestors(&mut protected, &root, parent);
                }
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

        let scheduler_handle = tokio::spawn(this.clone().run_scheduler(cancel.child_token()));
        let scan_worker_handle = tokio::spawn(this.clone().run_scan_worker(cancel.child_token()));

        let mut analyze_worker_handles = Vec::new();
        for idx in 0..self.config.analyze_workers.max(1) {
            analyze_worker_handles.push(tokio::spawn(
                this.clone().run_analyze_worker(cancel.child_token(), idx),
            ));
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
        if self.runtime.read().await.is_some() {
            if *self.ffprobe_available.read().await {
                Ok(ServiceHealth::healthy())
            } else {
                Ok(ServiceHealth::degraded(
                    "ffprobe is unavailable; media analysis jobs will fail",
                ))
            }
        } else {
            Ok(ServiceHealth::degraded("library scan runtime not running"))
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
    #[serde(default)]
    id: Option<i32>,
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

async fn ffprobe_analyze(path: &str) -> Result<ProbeAnalysis> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_streams")
        .arg("-show_chapters")
        .arg("-show_format")
        .arg("-print_format")
        .arg("json")
        .arg(path)
        .output()
        .await
        .with_context(|| format!("failed to execute ffprobe for {}", path))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffprobe failed: {}", stderr);
    }

    let metadata_json = String::from_utf8_lossy(&output.stdout).to_string();

    let parsed: FfprobeRoot = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("failed to parse ffprobe json for {}", path))?;

    let mut analysis = ProbeAnalysis::default();
    analysis.metadata = Some(metadata_json);

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
        let chapter_index = chapter.id.unwrap_or(idx as i32);
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
    let re = Regex::new(&format!(r"\{{{}(?::(\d+))?\}}", regex::escape(token))).unwrap();
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
                .unwrap_or(&format!("Episode {}", episode))
                .trim(),
        ),
    );
    out = out.replace("{ext}", ext.trim_start_matches('.'));
    out = replace_number_token(out, "season", season);
    out = replace_number_token(out, "episode", episode);
    PathBuf::from(out)
}

fn extract_quality_info(filename: &str) -> String {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename)
        .to_ascii_lowercase();
    for q in ["2160p", "1080p", "720p", "480p"] {
        if stem.contains(q) {
            return q.to_string();
        }
    }
    "unknown".to_string()
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
                .unwrap_or(&format!("Chapter {}", chapter_number))
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

#[cfg(test)]
mod tests {
    use super::{LibraryScanService, MatchWantedPolicy};
    use regex::Regex;
    use std::fs;
    use std::path::PathBuf;

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
    fn parse_movie_hint_handles_language_tags_and_cut_labels() {
        let hint = LibraryScanService::parse_movie_hint(
            "[German] Fabian and the Deadly Wedding 2026 HDR 2160p WEB h265-EDITH",
        );
        assert_eq!(hint.title.as_deref(), Some("Fabian and the Deadly Wedding"));
        assert_eq!(hint.year, Some(2026));
    }

    #[test]
    fn parse_movie_hint_from_rss_samples_produces_clean_search_titles() {
        let xml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../legacy/sample-data/movie-sample.xml");
        let xml = fs::read_to_string(&xml_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", xml_path.display(), e));

        let title_re = Regex::new(r"<item><title>([^<]+)</title>")
            .expect("movie sample title regex should compile");
        let bad_token_re = Regex::new(
            r"(?i)\b(2160p|1080p|720p|480p|4k|uhd|x264|x265|h264|h265|hevc|bluray|blu-ray|webrip|web-dl|hdr10?|dovi|atmos|ddp|dts|truehd|remux|repack|proper|internal|screener)\b",
        )
        .expect("bad token regex should compile");

        let mut checked = 0usize;
        let mut failures: Vec<String> = Vec::new();

        for caps in title_re.captures_iter(&xml) {
            let raw_title = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
            let hint = LibraryScanService::parse_movie_hint(raw_title);
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
            .join("../legacy/sample-data/movies-file-list.txt");
        let content = fs::read_to_string(&sample_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", sample_path.display(), e));

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
