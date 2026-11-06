//! Working out what a download is, as a series of recorded steps.
//!
//! A run starts when a torrent's files become known, each time more of them finish, when the whole
//! torrent finishes, and when a person asks. Runs are safe to repeat: results are kept by what they
//! describe, what's imported stays imported, and a step that fails is recorded and passed over. A
//! download with no finished run is picked up again after a restart. The rules are in
//! `identify_rules` and `release_match`, and imports go through the same path as everything else.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI32, Ordering};

use serde::Serialize;

use super::*;
use crate::graphql::entities::{
    Album, AlbumWhereInput, Artist, Audiobook, AudiobookWhereInput, Episode, EpisodeWhereInput,
    IdentificationEvent, IdentificationEventWhereInput, IdentifiedItem, IdentifiedItemWhereInput,
    Library, LibraryWhereInput, Movie, MovieWhereInput, PendingFileMatch,
    PendingFileMatchWhereInput, Show, ShowWhereInput, Torrent, TorrentWhereInput, Track,
    TrackWhereInput,
};
use crate::services::identify_rules::{
    self as rules, EditionFacts, EditionFit, EditionVerdict, GroupIdentity, ReleaseKind, TextBlock,
    TextFacts,
};
use crate::services::jev::Jev;
use crate::services::metadata::settings::MetadataSettings;
use crate::services::release_match::{BookFacts, BookGroup, BookOutcome, ReleaseFile, TrackFacts};
use crate::services::whisper::{self, Whisper};

/// Files can finish in quick succession, so a run waits for a pause in them.
const FILE_FINISHED_PAUSE: Duration = Duration::from_secs(30);
const RUNS_KEPT: usize = 3;
const CANDIDATES_KEPT: usize = 8;
/// The most listings to look up chapter counts for, since each is a request.
const CHAPTER_LOOKUPS: usize = 4;
const MAX_SEARCHES_PER_ITEM: usize = 4;
/// A release of many albums or singles shouldn't send hundreds of requests to a rate-limited provider.
const MAX_PROVIDER_ITEMS: usize = 40;
const MAX_TEXT_BYTES: u64 = 256 * 1024;
/// Jev is asked only among listings the rules can't separate, and has to be this sure, and clear of
/// the next one. Sibling books score close together.
const JEV_ENOUGH: f64 = 0.85;
const JEV_LEAD: f64 = 0.1;
/// A movie's or episode's runtime as listed can differ a little from a release, by cut and credits.
const RUNTIME_TOLERANCE: f64 = 0.15;
const CLEAR_TITLE: f64 = 0.9;
/// The right title by the right artist reaches this without tracks to compare.
const CLEAR_ALBUM: f64 = 0.85;
const POSSIBLE_TITLE: f64 = 0.75;
const OPENING_SECS: f64 = 60.0;
/// Enough of a book's tracks have to line up with an album's before it counts as that album.
const ENOUGH_TRACKS: f64 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifyTrigger {
    /// Its files became known to the client. Does nothing for a download already identified.
    Observed,
    FileFinished,
    Finished,
    Asked,
}

impl IdentifyTrigger {
    fn label(self) -> &'static str {
        match self {
            Self::Observed => "Files found",
            Self::FileFinished => "More files finished",
            Self::Finished => "Download finished",
            Self::Asked => "Run by hand",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Observed => 0,
            Self::FileFinished => 1,
            Self::Finished => 2,
            Self::Asked => 3,
        }
    }

    fn pause(self) -> Duration {
        match self {
            Self::FileFinished => FILE_FINISHED_PAUSE,
            Self::Observed | Self::Finished | Self::Asked => Duration::ZERO,
        }
    }
}

/// Where a piece of evidence came from, so it can be shown with others like it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Source {
    /// The download itself: names, tags, text files.
    Files,
    /// What listening to it heard.
    Heard,
    /// An online listing.
    Listing,
    /// What's already in the library, or was requested.
    Library,
    Jev,
    Other,
}

/// One thing identification found out about an item.
#[derive(Debug, Clone, Serialize)]
struct Finding {
    source: Source,
    text: String,
    /// It counts against the item, or someone should know about it before adding it.
    warning: bool,
    /// A few words for showing the warning beside the item: "2 files damaged".
    #[serde(skip_serializing_if = "Option::is_none")]
    short: Option<String>,
}

/// Something it could be. Stored as JSON on the result, and what a person chooses from.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct IdentifyCandidate {
    /// `library` or `provider`.
    pub source: String,
    pub provider: Option<String>,
    pub provider_id: Option<String>,
    pub library_item_type: Option<String>,
    pub library_item_id: Option<String>,
    pub library_id: Option<String>,
    pub title: String,
    pub creator: Option<String>,
    pub narrator: Option<String>,
    pub year: Option<i32>,
    pub cover_url: Option<String>,
    pub seconds: Option<f64>,
    pub chapters: Option<i32>,
    /// The listing's length as a percentage of the files'.
    pub length_percent: Option<f64>,
    pub fit: Option<String>,
    pub score: f64,
    pub reasons: Vec<String>,
}

/// What a person decided about a result.
#[derive(Debug, Clone, Default)]
pub struct IdentifiedChoice {
    pub item_id: String,
    pub provider_id: Option<String>,
    pub library_item_id: Option<String>,
    pub library_id: Option<String>,
    pub accept_review: bool,
}

/// A result being put together.
#[derive(Debug, Clone, Default)]
struct ItemDraft {
    key: String,
    kind: &'static str,
    status: &'static str,
    label: String,
    title: Option<String>,
    creator: Option<String>,
    year: Option<i32>,
    cover_url: Option<String>,
    paths: Vec<String>,
    finished: usize,
    seconds: Option<f64>,
    confidence: f64,
    summary: Option<String>,
    evidence: Vec<Finding>,
    candidates: Vec<IdentifyCandidate>,
    identity: GroupIdentity,
    provider_id: Option<String>,
    library_item_type: Option<&'static str>,
    library_item_id: Option<String>,
    library_id: Option<String>,
    season: Option<i32>,
    can_accept: bool,
    /// Taken from a group in the plan: which group of files a book import should use.
    group_key: Option<String>,
}

impl ItemDraft {
    fn note(&mut self, source: Source, text: impl Into<String>) {
        self.evidence.push(Finding {
            source,
            text: text.into(),
            warning: false,
            short: None,
        });
    }

    fn warn(&mut self, source: Source, text: impl Into<String>, short: Option<String>) {
        self.evidence.push(Finding {
            source,
            text: text.into(),
            warning: true,
            short,
        });
    }

    /// The reasons a listing was judged as it was. One naming a different person counts against it.
    fn note_reasons(&mut self, source: Source, reasons: impl IntoIterator<Item = String>) {
        for reason in reasons {
            if reason.starts_with("A different ") {
                self.warn(source, reason, None);
            } else {
                self.note(source, reason);
            }
        }
    }

    /// The evidence as lines, for a step in the activity feed.
    fn evidence_text(&self) -> String {
        self.evidence
            .iter()
            .map(|finding| finding.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn complete(&self) -> bool {
        !self.paths.is_empty() && self.finished >= self.paths.len()
    }
}

/// What the download was requested as.
#[derive(Debug, Clone, Default)]
struct RequestHint {
    kind: Option<ReleaseKind>,
    title: Option<String>,
    item_id: Option<String>,
}

/// Records a run's steps. Losing a line of history must never stop the run, so failures only log.
struct Feed<'a> {
    scan: &'a LibraryScanService,
    owner: String,
    hash: String,
    run: String,
    trigger: &'static str,
    sequence: AtomicI32,
}

impl Feed<'_> {
    async fn write(
        &self,
        status: &str,
        message: &str,
        detail: Option<String>,
        duration: Option<Duration>,
    ) -> Option<String> {
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        let result = self
            .scan
            .execute_mutation(
                &AuthUser::system_admin(),
                r#"mutation AddIdentificationEvent($input: CreateIdentificationEventInput!) {
                    Created: createIdentificationEvent(input: $input) {
                        Success: success
                        Node: identificationEvent { Id: id }
                    }
                }"#,
                serde_json::json!({ "input": {
                    "userId": self.owner,
                    "infoHash": self.hash,
                    "runId": self.run,
                    "trigger": self.trigger,
                    "sequence": sequence,
                    "status": status,
                    "message": message,
                    "detail": detail,
                    "durationMs": duration.map(|taken| taken.as_millis() as i64),
                }}),
            )
            .await;
        match result {
            Ok(data) => data
                .pointer("/Created/Node/Id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            Err(error) => {
                warn!(info_hash = %self.hash, error = %error, "Could not record an identification step");
                None
            }
        }
    }

    /// A step that's about to take a while, so the page can show it happening.
    async fn start(&self, message: &str) -> RunningStep {
        RunningStep {
            id: self.write("running", message, None, None).await,
            began: Instant::now(),
        }
    }

    async fn finish(&self, step: RunningStep, status: &str, message: &str, detail: Option<String>) {
        let taken = step.began.elapsed();
        let Some(id) = step.id else {
            self.write(status, message, detail, Some(taken)).await;
            return;
        };
        if let Err(error) = self
            .scan
            .execute_mutation(
                &AuthUser::system_admin(),
                r#"mutation FinishIdentificationEvent($id: String!, $input: UpdateIdentificationEventInput!) {
                    Updated: updateIdentificationEvent(id: $id, input: $input) { Success: success }
                }"#,
                serde_json::json!({ "id": id, "input": {
                    "status": status,
                    "message": message,
                    "detail": detail,
                    "durationMs": taken.as_millis() as i64,
                }}),
            )
            .await
        {
            warn!(info_hash = %self.hash, error = %error, "Could not finish an identification step");
        }
    }
}

struct RunningStep {
    id: Option<String>,
    began: Instant,
}

/// Everything a run knows, built up as it goes.
struct RunContext<'a> {
    feed: Feed<'a>,
    owner: String,
    hash: String,
    name: String,
    files: Vec<SourceFileImport>,
    hint: RequestHint,
    text: TextFacts,
    libraries: Vec<Library>,
    copied: HashSet<String>,
    jev: Option<Jev>,
    listening: Option<Whisper>,
    /// A run from seeing a torrent again, such as after a restart, only looks. Nothing old should
    /// land in a library because the server restarted.
    may_import: bool,
}

impl RunContext<'_> {
    fn library_for(&self, kinds: &[&str]) -> Option<String> {
        self.libraries
            .iter()
            .find(|library| {
                kinds
                    .iter()
                    .any(|kind| library.library_type.eq_ignore_ascii_case(kind))
            })
            .map(|library| library.id.clone())
    }
}

/// A restored torrent reads as empty while the client checks it, which can take a long time across
/// many downloads, and its progress in the database drops with it. When it finished stays recorded.
/// Padding files are part of how some torrents are laid out, and nothing ever writes them to disk.
fn settled_files(row: &Torrent, mut files: Vec<SourceFileImport>) -> Vec<SourceFileImport> {
    files.retain(|file| !is_padding(&file.relative_path));
    if row.completed_at.is_some() || row.progress >= 1.0 {
        for file in &mut files {
            file.downloaded_bytes = file.file_size;
        }
    }
    files
}

fn is_padding(relative_path: &str) -> bool {
    relative_path.starts_with(".pad/") || relative_path.starts_with(".pad\\")
}

/// The folder a file says it's in: its own, or the torrent's for a file at the top. A one-file
/// torrent has only the file's name.
fn folder_label(file: &ReleaseFile, torrent_name: &str) -> String {
    folder_name(&file.path).unwrap_or_else(|| {
        let name = Path::new(torrent_name);
        let is_file = name
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                release_match::AUDIO_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
            });
        if is_file {
            name.file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or(torrent_name)
                .to_string()
        } else {
            torrent_name.to_string()
        }
    })
}

fn dedupe(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && seen.insert(value.to_lowercase()))
        .collect()
}

fn folder_name(path: &str) -> Option<String> {
    Path::new(path)
        .parent()?
        .file_name()?
        .to_str()
        .map(str::to_string)
}

/// What a run should count as, or `None` if there's nothing to do.
///
/// A restored torrent reads as empty until the client has checked it, so its files all seem to
/// finish at once. If it already went through post-processing, nothing new has happened, and
/// that must not import anything. It may still never have been identified, though: its first
/// look can be replaced by these "finishes" while it waits for a slot. So that stays a first look.
fn settled_trigger(
    trigger: IdentifyTrigger,
    identified: bool,
    post_processed: bool,
) -> Option<IdentifyTrigger> {
    match trigger {
        IdentifyTrigger::Observed if identified => None,
        IdentifyTrigger::FileFinished if post_processed && identified => None,
        IdentifyTrigger::FileFinished if post_processed => Some(IdentifyTrigger::Observed),
        _ => Some(trigger),
    }
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

impl LibraryScanService {
    /// Ask for a download to be identified. Requests for one already waiting merge into it.
    pub fn request_identification(self: &Arc<Self>, info_hash: &str, trigger: IdentifyTrigger) {
        {
            let mut pending = self
                .identify_pending
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(waiting) = pending.get_mut(info_hash) {
                if trigger.rank() > waiting.rank() {
                    *waiting = trigger;
                }
                return;
            }
            pending.insert(info_hash.to_string(), trigger);
        }
        let this = self.clone();
        let hash = info_hash.to_string();
        tokio::spawn(async move {
            // A file finishing waits for others to settle, unless something more pressing arrives.
            let started = std::time::Instant::now();
            while started.elapsed() < trigger.pause() {
                let pressing = this
                    .identify_pending
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .get(&hash)
                    .is_some_and(|waiting| waiting.rank() > trigger.rank());
                if pressing {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            let Ok(_slot) = this.identify_slots.clone().acquire_owned().await else {
                return;
            };
            // One run per download: a request made during a run waits for it to end.
            loop {
                let mut active = this.identify_active.lock().await;
                if active.insert(hash.clone()) {
                    break;
                }
                drop(active);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            let trigger = this
                .identify_pending
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&hash)
                .unwrap_or(trigger);
            let outcome = this.identify_download(&hash, trigger).await;
            this.identify_active.lock().await.remove(&hash);
            if let Err(error) = outcome {
                warn!(info_hash = %hash, error = %error, "Could not identify a download");
                let _ = this
                    .set_identification_status(&hash, "failed", Some(&error.to_string()))
                    .await;
            }
        });
    }

    async fn torrent_row(&self, info_hash: &str) -> Result<Option<Torrent>> {
        let database = self.database().await?;
        Ok(Torrent::query(database.pool().pool())
            .filter(TorrentWhereInput {
                info_hash: Some(StringFilter {
                    eq: Some(info_hash.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .limit(1)
            .fetch_all()
            .await?
            .into_iter()
            .next())
    }

    async fn set_identification_status(
        &self,
        info_hash: &str,
        status: &str,
        summary: Option<&str>,
    ) -> Result<()> {
        let Some(row) = self.torrent_row(info_hash).await? else {
            return Ok(());
        };
        self.execute_mutation(
            &AuthUser::system_admin(),
            r#"mutation SetIdentificationStatus($id: String!, $input: UpdateTorrentInput!) {
                Updated: updateTorrent(id: $id, input: $input) { Success: success }
            }"#,
            serde_json::json!({ "id": row.id, "input": {
                "identificationStatus": status,
                "identificationSummary": summary,
            }}),
        )
        .await?;
        Ok(())
    }

    /// Say what a download is, or `auto` to go back to working it out.
    pub async fn set_identification_kind(&self, info_hash: &str, kind: &str) -> Result<()> {
        anyhow::ensure!(
            kind == "auto" || ReleaseKind::parse(kind).is_some(),
            "A download can't be treated as {kind}"
        );
        let row = self
            .torrent_row(info_hash)
            .await?
            .ok_or_else(|| anyhow::anyhow!("That download no longer exists"))?;
        let chosen = (kind != "auto").then_some(kind);
        self.execute_mutation(
            &AuthUser::system_admin(),
            r#"mutation SetIdentificationKind($id: String!, $input: UpdateTorrentInput!) {
                Updated: updateTorrent(id: $id, input: $input) { Success: success }
            }"#,
            serde_json::json!({ "id": row.id, "input": { "identificationKind": chosen } }),
        )
        .await?;
        Ok(())
    }

    /// What in the library this audio matches, for when the files alone don't settle their kind.
    async fn library_kind_of_audio(
        &self,
        owner: &str,
        torrent_name: &str,
        audio: &[ReleaseFile],
        text: &[TextBlock],
    ) -> Option<ReleaseKind> {
        let groups = release_match::group_files(audio);
        let books = self.owned_books(owner).await.unwrap_or_default();
        let is_book = groups.iter().any(|group| {
            let (identity, _) = book_identity(text, torrent_name, group, audio);
            books
                .iter()
                .any(|book| rules::judge_edition(&identity, &edition_of_book(book)).plausible())
        });
        if !is_book {
            return None;
        }
        // An album by the same name in the library leaves it as open as it was.
        let database = self.database().await.ok()?;
        let albums = Album::query(database.pool().pool())
            .filter(AlbumWhereInput {
                user_id: Some(StringFilter {
                    eq: Some(owner.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
            .unwrap_or_default();
        let is_album = groups.iter().any(|group| {
            albums.iter().any(|album| {
                release_match::title_score(
                    &group.label,
                    &BookFacts {
                        title: album.name.clone(),
                        ..Default::default()
                    },
                ) >= POSSIBLE_TITLE
            })
        });
        (!is_album).then_some(ReleaseKind::Audiobook)
    }

    async fn identify_download(
        self: &Arc<Self>,
        info_hash: &str,
        trigger: IdentifyTrigger,
    ) -> Result<()> {
        let Some(row) = self.torrent_row(info_hash).await? else {
            return Ok(());
        };
        let Some(trigger) = settled_trigger(
            trigger,
            row.identification_status.as_deref() == Some("done"),
            row.post_process_status.is_some(),
        ) else {
            return Ok(());
        };
        let torrents = self
            .manager
            .get_torrent()
            .await
            .ok_or_else(|| anyhow::anyhow!("The torrent client isn't running"))?;
        let files = settled_files(&row, torrents.release_files(info_hash).await?);
        // A magnet with no file list yet has nothing to look at, and it'll be seen again.
        if files.is_empty() {
            return Ok(());
        }

        let owner = row.user_id.clone();
        self.forget_old_identification_runs(info_hash).await;
        self.set_identification_status(info_hash, "running", None)
            .await?;
        let mut run = RunContext {
            feed: Feed {
                scan: self,
                owner: owner.clone(),
                hash: info_hash.to_string(),
                run: format!("{:020}-{}", Utc::now().timestamp_millis(), Uuid::new_v4()),
                trigger: trigger.label(),
                sequence: AtomicI32::new(0),
            },
            owner,
            hash: info_hash.to_string(),
            name: row.name.clone(),
            files,
            hint: RequestHint::default(),
            text: TextFacts::default(),
            libraries: self.owned_libraries(&row.user_id).await.unwrap_or_default(),
            copied: self.copied_files(info_hash).await.unwrap_or_default(),
            jev: Jev::configured(&self.manager).await,
            listening: self.available_listener().await,
            may_import: trigger != IdentifyTrigger::Observed,
        };

        self.note_request(&row, &mut run).await;
        let (audio, video, text_files, unread) = self.read_files(&mut run, &row.name).await;
        // Files that can't be read say nothing about what they are. Whatever was worked out when they
        // could be read stands, rather than being replaced by a guess from their names.
        if unread > 0
            && self
                .identified_items(info_hash)
                .await
                .is_ok_and(|items| !items.is_empty())
        {
            let summary = format!(
                "Kept what was found before: {} can't be read now",
                plural(unread, "file", "files")
            );
            run.feed.write("warning", &summary, None, None).await;
            return self
                .set_identification_status(info_hash, "done", row.identification_summary.as_deref())
                .await;
        }
        let chosen = row
            .identification_kind
            .as_deref()
            .and_then(ReleaseKind::parse);
        let kind = self
            .decide_kind(&run, &row.name, &audio, &video, chosen)
            .await;
        let drafts = match kind {
            ReleaseKind::Audiobook => self.identify_books(&run, &audio).await,
            ReleaseKind::Music => self.identify_music(&run, &audio).await,
            ReleaseKind::Tv => self.identify_tv(&run, &video).await,
            ReleaseKind::Movie => self.identify_movies(&run, &video).await,
            ReleaseKind::Ebook | ReleaseKind::Unknown => {
                vec![self.unsupported_item(&run, kind, &row.name, &text_files)]
            }
        };
        let summary = self.settle_results(&run, drafts).await;
        run.feed.write("found", &summary, None, None).await;
        self.set_identification_status(info_hash, "done", Some(&summary))
            .await
    }

    /// Drop all but the latest runs. What's left over from a deleted run is harmless.
    async fn forget_old_identification_runs(&self, info_hash: &str) {
        let Ok(database) = self.database().await else {
            return;
        };
        let events = IdentificationEvent::query(database.pool().pool())
            .filter(IdentificationEventWhereInput {
                info_hash: Some(StringFilter {
                    eq: Some(info_hash.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
            .unwrap_or_default();
        let mut runs: Vec<String> = events.into_iter().map(|event| event.run_id).collect();
        runs.sort();
        runs.dedup();
        // Leave room for the one about to start.
        let keep_from = runs.len().saturating_sub(RUNS_KEPT - 1);
        let old: Vec<String> = runs.drain(..keep_from).collect();
        if old.is_empty() {
            return;
        }
        if let Err(error) = self
            .execute_mutation(
                &AuthUser::system_admin(),
                r#"mutation ForgetIdentificationRuns($where: IdentificationEventWhereInput) {
                    Deleted: deleteIdentificationEvents(where: $where) { Success: success }
                }"#,
                serde_json::json!({ "where": {
                    "infoHash": { "eq": info_hash },
                    "runId": { "inList": old },
                }}),
            )
            .await
        {
            warn!(info_hash, error = %error, "Could not clear old identification runs");
        }
    }

    /// Everything recorded about a download that has gone.
    pub async fn forget_identification(&self, info_hash: &str) {
        for mutation in [
            r#"mutation ForgetEvents($where: IdentificationEventWhereInput) {
                Deleted: deleteIdentificationEvents(where: $where) { Success: success }
            }"#,
            r#"mutation ForgetItems($where: IdentifiedItemWhereInput) {
                Deleted: deleteIdentifiedItems(where: $where) { Success: success }
            }"#,
        ] {
            if let Err(error) = self
                .execute_mutation(
                    &AuthUser::system_admin(),
                    mutation,
                    serde_json::json!({ "where": { "infoHash": { "eq": info_hash } } }),
                )
                .await
            {
                warn!(info_hash, error = %error, "Could not forget a removed download's identification");
            }
        }
    }

    async fn owned_libraries(&self, owner: &str) -> Result<Vec<Library>> {
        let database = self.database().await?;
        Ok(Library::query(database.pool().pool())
            .filter(LibraryWhereInput {
                user_id: Some(StringFilter {
                    eq: Some(owner.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?)
    }

    async fn copied_files(&self, info_hash: &str) -> Result<HashSet<String>> {
        let database = self.database().await?;
        Ok(PendingFileMatch::query(database.pool().pool())
            .filter(PendingFileMatchWhereInput {
                source_id: Some(StringFilter {
                    eq: Some(info_hash.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?
            .into_iter()
            .filter(|row| row.copied_at.is_some())
            .map(|row| row.source_path)
            .collect())
    }

    /// Whoever can listen, if listening is on and installed.
    pub(super) async fn available_listener(&self) -> Option<Whisper> {
        let database = self.database().await.ok()?;
        let settings = MetadataSettings::load(database.pool()).await.ok()?;
        if !settings.listening_enabled {
            return None;
        }
        let whisper = Whisper::from_env();
        whisper.check().await.ok().map(|()| whisper)
    }

    /// What the grab was for, which everything after starts from.
    async fn note_request(&self, row: &Torrent, run: &mut RunContext<'_>) {
        let link = TorrentImportLink::from_torrent(row);
        let Ok(database) = self.database().await else {
            return;
        };
        let pool = database.pool().pool();
        let mut hint = RequestHint::default();
        if let Some(id) = link.audiobook_id.as_ref() {
            hint.kind = Some(ReleaseKind::Audiobook);
            hint.item_id = Some(id.clone());
            hint.title = Audiobook::get(pool, id)
                .await
                .ok()
                .flatten()
                .map(|book| book.title);
        } else if let Some(id) = link.movie_id.as_ref() {
            hint.kind = Some(ReleaseKind::Movie);
            hint.item_id = Some(id.clone());
            hint.title = Movie::get(pool, id)
                .await
                .ok()
                .flatten()
                .map(|movie| movie.title);
        } else if let Some(id) = link.show_id.as_ref() {
            hint.kind = Some(ReleaseKind::Tv);
            hint.item_id = Some(id.clone());
            hint.title = Show::get(pool, id)
                .await
                .ok()
                .flatten()
                .map(|show| show.name);
        } else if link.episode_id.is_some() {
            hint.kind = Some(ReleaseKind::Tv);
        } else if let Some(id) = link.album_id.as_ref() {
            hint.kind = Some(ReleaseKind::Music);
            hint.item_id = Some(id.clone());
            hint.title = Album::get(pool, id)
                .await
                .ok()
                .flatten()
                .map(|album| album.name);
        } else if link.track_id.is_some() {
            hint.kind = Some(ReleaseKind::Music);
        } else if link.chapter_id.is_some() {
            hint.kind = Some(ReleaseKind::Audiobook);
        } else if let Some(library_id) = row.library_id.as_ref()
            && let Some(library) = run
                .libraries
                .iter()
                .find(|library| &library.id == library_id)
        {
            hint.kind = ReleaseKind::from_library_type(&library.library_type);
        }
        let message = match (&hint.title, hint.kind) {
            (Some(title), _) => Some(format!("Requested for \u{201c}{title}\u{201d}")),
            (None, Some(_)) => row
                .library_id
                .as_ref()
                .and_then(|id| run.libraries.iter().find(|library| &library.id == id))
                .map(|library| format!("Added for the {} library", library.name)),
            (None, None) => None,
        };
        if let Some(message) = message {
            run.feed.write("done", &message, None, None).await;
        }
        run.hint = hint;
    }

    /// Read what the finished files say about themselves.
    async fn read_files(
        &self,
        run: &mut RunContext<'_>,
        name: &str,
    ) -> (Vec<ReleaseFile>, Vec<ReleaseFile>, usize, usize) {
        let step = run.feed.start("Reading the files").await;
        let audio_files: Vec<&SourceFileImport> = run
            .files
            .iter()
            .filter(|file| Self::is_audio_import_path(&file.source_path))
            .collect();
        let import_files: Vec<ImportFile> = run
            .files
            .iter()
            .filter(|file| Self::is_video_import_path(&file.source_path))
            .cloned()
            .map(|file| ImportFile { file })
            .collect();
        // Samples and extras aren't what it is. Only worth removing once it's clear what's there.
        let (usable_video, skipped) = Self::filter_import_candidates(import_files);
        let video_refs: Vec<&SourceFileImport> =
            usable_video.iter().map(|item| &item.file).collect();
        let mut audio = self.release_facts_for(&audio_files).await;
        let mut video = self.release_facts_for(&video_refs).await;
        // A file being written or a busy disk can fail once. What was read is remembered.
        let missing = |facts: &[ReleaseFile]| {
            facts
                .iter()
                .any(|file| file.duration.is_none() && file.size > 0)
        };
        if missing(&audio) || missing(&video) {
            tokio::time::sleep(Duration::from_secs(2)).await;
            audio = self.release_facts_for(&audio_files).await;
            video = self.release_facts_for(&video_refs).await;
        }

        let finished = run
            .files
            .iter()
            .filter(|file| file.file_size > 0 && file.downloaded_bytes >= file.file_size)
            .count();
        let text_files: Vec<&SourceFileImport> = run
            .files
            .iter()
            .filter(|file| {
                Path::new(&file.source_path)
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        matches!(
                            extension.to_ascii_lowercase().as_str(),
                            "txt" | "nfo" | "cue" | "md"
                        )
                    })
            })
            .collect();
        let tagged = audio
            .iter()
            .filter(|file| file.tags.album.is_some())
            .count();
        let markers = audio
            .iter()
            .filter(|file| !file.embedded.is_empty())
            .count();
        let unread = audio
            .iter()
            .chain(&video)
            .filter(|file| {
                file.duration.is_none()
                    && file.size > 0
                    && run
                        .files
                        .iter()
                        .any(|f| f.source_path == file.key && f.downloaded_bytes >= f.file_size)
            })
            .count();
        let mut lines = vec![format!("{finished} of {} files finished", run.files.len())];
        if !audio.is_empty() {
            lines.push(format!(
                "Album tags on {tagged} of {} audio files",
                audio.len()
            ));
        }
        if markers > 0 {
            lines.push(format!("Chapter markers inside {markers} files"));
        }
        if !skipped.is_empty() {
            lines.push(format!(
                "Left out {}",
                plural(skipped.len(), "sample or extra", "samples and extras")
            ));
        }
        let mut parts = Vec::new();
        if !audio.is_empty() {
            parts.push(plural(audio.len(), "audio file", "audio files"));
        }
        if !video.is_empty() {
            parts.push(plural(video.len(), "video file", "video files"));
        }
        if !text_files.is_empty() {
            parts.push(plural(text_files.len(), "text file", "text files"));
        }
        let message = if parts.is_empty() {
            format!("\u{201c}{name}\u{201d} holds no audio, video or text files")
        } else {
            parts.join(", ")
        };
        run.feed
            .finish(
                step,
                if unread > 0 { "warning" } else { "done" },
                &if unread > 0 {
                    format!(
                        "{message}, {} couldn't be read",
                        plural(unread, "file", "files")
                    )
                } else {
                    message
                },
                Some(lines.join("\n")),
            )
            .await;

        let text_count = text_files.len();
        let text_paths: Vec<(String, bool)> = text_files
            .iter()
            .map(|file| {
                (
                    file.source_path.clone(),
                    file.downloaded_bytes >= file.file_size,
                )
            })
            .collect();
        run.text = self.read_text_files(run, &text_paths).await;
        (audio, video, text_count, unread)
    }

    /// Text files can name every book in a pack. A file too big or unreadable is skipped.
    async fn read_text_files(&self, run: &RunContext<'_>, paths: &[(String, bool)]) -> TextFacts {
        let mut facts = TextFacts::default();
        let mut read = Vec::new();
        for (path, finished) in paths {
            if !*finished {
                continue;
            }
            let small = tokio::fs::metadata(path)
                .await
                .is_ok_and(|meta| meta.len() <= MAX_TEXT_BYTES);
            if !small {
                continue;
            }
            let Ok(bytes) = tokio::fs::read(path).await else {
                continue;
            };
            let name = Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("a text file")
                .to_string();
            let found = rules::read_text_file(&name, &String::from_utf8_lossy(&bytes));
            if !found.blocks.is_empty() || !found.warnings.is_empty() {
                read.push((name, found.blocks.len()));
                facts.blocks.extend(found.blocks);
                facts.warnings.extend(found.warnings);
            }
        }
        if !read.is_empty() {
            let titles: Vec<String> = facts
                .blocks
                .iter()
                .filter_map(|block| block.title.clone())
                .collect();
            let message = match read.as_slice() {
                [(name, count)] if *count > 0 => {
                    format!("{name} describes {}", plural(*count, "title", "titles"))
                }
                [(name, _)] => format!("{name} has notes about the files"),
                _ => format!(
                    "{} describe what's here",
                    plural(read.len(), "text file", "text files")
                ),
            };
            let mut detail = titles;
            detail.extend(facts.warnings.iter().cloned());
            run.feed
                .write(
                    "found",
                    &message,
                    (!detail.is_empty()).then(|| detail.join("\n")),
                    None,
                )
                .await;
        }
        facts
    }

    async fn decide_kind(
        &self,
        run: &RunContext<'_>,
        name: &str,
        audio: &[ReleaseFile],
        video: &[ReleaseFile],
        chosen: Option<ReleaseKind>,
    ) -> ReleaseKind {
        let extension = |path: &str| {
            Path::new(path)
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .unwrap_or_default()
        };
        let albums: HashSet<String> = audio
            .iter()
            .filter_map(|file| file.tags.album.as_deref().map(normalize_album))
            .collect();
        let facts = rules::KindFacts {
            name: name.to_string(),
            audio_files: audio.len(),
            video_files: video.len(),
            ebook_files: run
                .files
                .iter()
                .filter(|file| {
                    release_match::EBOOK_EXTENSIONS.contains(&extension(&file.source_path).as_str())
                })
                .count(),
            audiobook_format_files: audio
                .iter()
                .filter(|file| extension(&file.path) == "m4b")
                .count(),
            episode_files: video
                .iter()
                .filter(|file| {
                    let hint = Self::parse_episode_hint(&file.path);
                    hint.season.is_some() && hint.episode.is_some()
                })
                .count(),
            audio_seconds: audio.iter().filter_map(|file| file.duration).collect(),
            album_tags: albums.len(),
            audiobook_genre_files: audio
                .iter()
                .filter(|file| {
                    file.tags
                        .genre
                        .as_deref()
                        .is_some_and(rules::is_audiobook_genre)
                })
                .count(),
            text_says_audiobook: rules::text_says_audiobook(&run.text),
            hint: run.hint.kind,
            chosen,
            in_library: self
                .library_kind_of_audio(&run.owner, name, audio, &run.text.blocks)
                .await,
        };
        let (kind, reason) = rules::classify_kind(&facts);
        run.feed
            .write(
                "found",
                &format!("This looks like {}", kind.label()),
                Some(reason),
                None,
            )
            .await;
        kind
    }

    fn unsupported_item(
        &self,
        run: &RunContext<'_>,
        kind: ReleaseKind,
        name: &str,
        _text_files: &usize,
    ) -> ItemDraft {
        ItemDraft {
            key: "release".to_string(),
            kind: kind.as_str(),
            status: "unknown",
            label: name.to_string(),
            paths: run
                .files
                .iter()
                .map(|file| file.source_path.clone())
                .collect(),
            finished: run
                .files
                .iter()
                .filter(|file| file.downloaded_bytes >= file.file_size)
                .count(),
            summary: Some(if kind == ReleaseKind::Ebook {
                "An ebook. Libraries here hold audiobooks, so there's nothing to import."
                    .to_string()
            } else {
                "Nothing here can go into a library.".to_string()
            }),
            ..Default::default()
        }
    }

    // -----------------------------------------------------------------------------------------
    // Audiobooks
    // -----------------------------------------------------------------------------------------

    async fn identify_books(&self, run: &RunContext<'_>, audio: &[ReleaseFile]) -> Vec<ItemDraft> {
        let groups = release_match::group_files(audio);
        run.feed
            .write(
                "found",
                &plural(groups.len(), "book", "books"),
                Some(
                    groups
                        .iter()
                        .map(|group| {
                            format!(
                                "{}: {}",
                                group.label,
                                plural(group.files.len(), "file", "files")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                ),
                None,
            )
            .await;
        let books = match self.owned_books(&run.owner).await {
            Ok(books) => books,
            Err(error) => {
                run.feed
                    .write(
                        "warning",
                        "Couldn't read the library's books",
                        Some(error.to_string()),
                        None,
                    )
                    .await;
                Vec::new()
            }
        };
        let mut items = Vec::with_capacity(groups.len());
        for group in &groups {
            items.push(
                self.identify_book_group(run, group, audio, &books, groups.len() == 1)
                    .await,
            );
        }
        items
    }

    async fn owned_books(&self, owner: &str) -> Result<Vec<Audiobook>> {
        let database = self.database().await?;
        Ok(Audiobook::query(database.pool().pool())
            .filter(AudiobookWhereInput {
                user_id: Some(StringFilter {
                    eq: Some(owner.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?)
    }

    #[allow(clippy::too_many_lines)]
    async fn identify_book_group(
        &self,
        run: &RunContext<'_>,
        group: &BookGroup,
        audio: &[ReleaseFile],
        books: &[Audiobook],
        only_group: bool,
    ) -> ItemDraft {
        let step = run
            .feed
            .start(&format!("Identifying {}", group.label))
            .await;
        let members: Vec<&ReleaseFile> = group.files.iter().map(|index| &audio[*index]).collect();
        let (mut identity, block) = book_identity(&run.text.blocks, &run.name, group, audio);
        let mut draft = ItemDraft {
            key: group.key.clone(),
            kind: "audiobook",
            status: "unknown",
            label: group.label.clone(),
            paths: members.iter().map(|file| file.key.clone()).collect(),
            finished: members
                .iter()
                .filter(|file| file.duration.is_some())
                .count(),
            seconds: identity.seconds,
            library_id: run.library_for(&["audiobooks"]),
            group_key: Some(group.key.clone()),
            ..Default::default()
        };
        draft.note(
            Source::Files,
            format!(
                "Files grouped by their {}",
                match group.grouped_by {
                    "tags" => "album tags",
                    "folders" => "folders",
                    _ => "names",
                }
            ),
        );
        if let Some(block) = &block {
            draft.note(
                Source::Files,
                format!(
                    "A text file describes it{}",
                    block
                        .narrator
                        .as_deref()
                        .map(|narrator| format!(", read by {narrator}"))
                        .unwrap_or_default()
                ),
            );
        }
        if let Some(title) = identity.titles.first() {
            for warning in &run.text.warnings {
                if rules::opening_mentions(warning, title) {
                    draft.warn(
                        Source::Files,
                        format!("Note in the download: {warning}"),
                        None,
                    );
                }
            }
        }
        let flagged = members
            .iter()
            .filter(|file| {
                let path = file.path.to_ascii_lowercase();
                path.contains("bad file") || path.contains("wrong file")
            })
            .count();
        if flagged > 0 {
            draft.warn(
                Source::Files,
                format!(
                    "{} named as damaged or wrong, so a chapter may have a fault",
                    plural(flagged, "file is", "files are")
                ),
                Some(format!("{} damaged", plural(flagged, "file", "files"))),
            );
        }

        if let Some(matched) = self.book_in_library(run, &identity, books, only_group) {
            self.fill_from_library_book(run, group, audio, matched, &mut draft)
                .await;
        } else {
            self.fill_from_book_listings(
                run,
                group,
                audio,
                &members,
                &mut identity,
                books,
                &mut draft,
            )
            .await;
        }
        draft.identity = identity;

        let detail = draft
            .evidence
            .iter()
            .map(|finding| finding.text.clone())
            .chain(draft.summary.clone())
            .collect::<Vec<_>>()
            .join("\n");
        let message = match (&draft.title, draft.status) {
            (Some(title), "imported") => format!("{}: imported as {title}", group.label),
            (Some(title), "wanted") => format!("{}: wanted, and it's {title}", group.label),
            (Some(title), "in_library") => format!("{}: {title}, in your library", group.label),
            (Some(title), "identified") => format!(
                "{}: {title}{}",
                group.label,
                draft
                    .creator
                    .as_deref()
                    .map(|creator| format!(" by {creator}"))
                    .unwrap_or_default()
            ),
            (_, "needs_choice") => format!("{}: more than one thing it could be", group.label),
            _ => format!("{}: not identified", group.label),
        };
        let status = if matches!(draft.status, "needs_choice" | "unknown") {
            "warning"
        } else {
            "found"
        };
        run.feed.finish(step, status, &message, Some(detail)).await;
        draft
    }

    /// The book in the library this could be: what it was requested for, then the best that fits. The
    /// title alone isn't enough, which is how another author's "The Last Colony" got in.
    fn book_in_library<'b>(
        &self,
        run: &RunContext<'_>,
        identity: &GroupIdentity,
        books: &'b [Audiobook],
        only_group: bool,
    ) -> Option<(&'b Audiobook, EditionVerdict)> {
        let mut plausible: Vec<(&Audiobook, EditionVerdict)> = Vec::new();
        for book in books {
            let verdict = rules::judge_edition(identity, &edition_of_book(book));
            let requested = run.hint.item_id.as_deref() == Some(book.id.as_str());
            // A lone group of files in a download grabbed for this book needn't name it.
            if verdict.plausible()
                || (requested && only_group && verdict.fit != EditionFit::Different)
            {
                plausible.push((book, verdict));
            }
        }
        plausible.sort_by(|a, b| {
            let requested =
                |book: &Audiobook| run.hint.item_id.as_deref() == Some(book.id.as_str());
            requested(b.0)
                .cmp(&requested(a.0))
                .then_with(|| b.1.score.total_cmp(&a.1.score))
        });
        plausible.into_iter().next()
    }

    async fn fill_from_library_book(
        &self,
        run: &RunContext<'_>,
        group: &BookGroup,
        audio: &[ReleaseFile],
        (book, verdict): (&Audiobook, EditionVerdict),
        draft: &mut ItemDraft,
    ) {
        draft.title = Some(book.title.clone());
        draft.creator = book.author_name.clone();
        draft.cover_url = book.cover_url.clone();
        draft.library_item_type = Some("Audiobook");
        draft.library_item_id = Some(book.id.clone());
        draft.library_id = Some(book.library_id.clone());
        draft.confidence = verdict.score;
        draft.note_reasons(Source::Library, verdict.reasons.iter().cloned());
        draft.candidates = vec![candidate_from_book(book, &verdict)];
        let requested = run.hint.item_id.as_deref() == Some(book.id.as_str());
        if requested {
            draft.note(Source::Library, "It was requested for this book");
        }
        if draft.paths.iter().all(|path| run.copied.contains(path)) {
            draft.status = "imported";
            draft.summary = Some("In your library.".to_string());
            return;
        }
        let Ok((_, chapters, facts, reference)) = self.book_reference(&book.id).await else {
            draft.status = "in_library";
            return;
        };
        // Grabbed for this book, but the files' tags name another: asking was a hint, and tags are
        // what the files say they are. With no tags, a matching length is as good as it gets.
        if requested && verdict.title_score < POSSIBLE_TITLE {
            let tagged = group.grouped_by == "tags";
            if tagged || verdict.fit != EditionFit::Recording {
                draft.status = "needs_choice";
                draft.can_accept = true;
                draft.summary = Some(if tagged {
                    format!(
                        "Grabbed for \u{201c}{}\u{201d}, but its tags say \u{201c}{}\u{201d}.",
                        release_match::short_title(&book.title),
                        group.label
                    )
                } else {
                    format!(
                        "Grabbed for \u{201c}{}\u{201d}, but neither its name nor its length match.",
                        release_match::short_title(&book.title)
                    )
                });
                return;
            }
            draft.note(
                Source::Library,
                "Its name doesn't say which book it is, but its length matches",
            );
        }
        let wanted = requested || chapters.iter().any(|chapter| chapter.wanted);
        let has_files = chapters
            .iter()
            .any(|chapter| chapter.media_file_id.is_some());
        match release_match::layout_book(audio, group, &facts, &reference) {
            BookOutcome::Ready(layout) => {
                draft.note_reasons(Source::Listing, layout.evidence.iter().cloned());
                draft.summary = Some(layout_line(&layout));
                draft.status = if has_files {
                    "in_library"
                } else if wanted {
                    "wanted"
                } else {
                    "in_library"
                };
                if has_files {
                    draft.summary =
                        Some("This book already has files in your library.".to_string());
                }
            }
            BookOutcome::Review(review) => {
                draft.status = "needs_choice";
                draft.can_accept = review.fallback.is_some();
                draft.summary = Some(review.reason);
            }
        }
    }

    /// Which listing this is, trying every source that can tell them apart.
    #[allow(clippy::too_many_arguments)]
    async fn fill_from_book_listings(
        &self,
        run: &RunContext<'_>,
        group: &BookGroup,
        audio: &[ReleaseFile],
        members: &[&ReleaseFile],
        identity: &mut GroupIdentity,
        books: &[Audiobook],
        draft: &mut ItemDraft,
    ) {
        let searches = rules::edition_searches(&identity.titles, &identity.authors);
        let mut found = self.book_listings(identity, &searches).await;
        let complete = draft.complete();

        // A release that names its Audible id has said exactly what it is, if the length agrees.
        let named_id = members
            .iter()
            .find_map(|file| rules::audible_id_in(&file.key))
            .or_else(|| rules::audible_id_in(&group.label))
            .or_else(|| rules::audible_id_in(&run.name));
        let mut named_index = None;
        if let Some(id) = named_id {
            named_index = found
                .results
                .iter()
                .position(|result| result.provider_id == id);
            if named_index.is_none()
                && let Ok(metadata) = self.metadata_service().await
                && let Ok(details) = metadata.get_audiobook(&id).await
                // An id from another country's store comes back as an empty listing.
                && details.author_name.is_some()
            {
                found.push(listing_from_details(&details), identity);
                named_index = Some(found.results.len() - 1);
            }
            named_index = named_index.filter(|index| found.verdicts[*index].plausible());
            if named_index.is_some() {
                draft.note(Source::Files, format!("Its name gives the Audible id {id}"));
            } else {
                draft.warn(
                    Source::Files,
                    format!(
                        "Its name gives the Audible id {id}, which doesn't fit or isn't listed here"
                    ),
                    None,
                );
            }
        }
        let mut listened: Option<String> = None;

        // Nothing to go on, or the tags are wrong: what the recording says about itself.
        if named_index.is_none()
            && rules::clear_edition(&found.editions, &found.verdicts).is_none()
            && complete
            && let Some(whisper) = &run.listening
            && let Some(heard) = self.hear_opening(whisper, members).await
        {
            draft.note(
                Source::Heard,
                format!(
                    "\u{201c}{}\u{201d}",
                    heard.chars().take(140).collect::<String>().trim()
                ),
            );
            if let Some((title, author)) = rules::title_from_opening(&heard) {
                identity.titles.push(title.clone());
                identity.authors.push(author.clone());
                let more = self
                    .book_listings(identity, &[format!("{title} {author}"), title])
                    .await;
                found.merge(more, identity);
            }
            listened = Some(heard);
        }

        let mut chosen =
            named_index.or_else(|| rules::clear_edition(&found.editions, &found.verdicts));
        if chosen.is_none()
            && let Some(heard) = &listened
            && let Some(index) = heard_edition(heard, &found)
        {
            draft.note(
                Source::Heard,
                "Its opening names this listing's title and people",
            );
            chosen = Some(index);
        }
        if chosen.is_none()
            && complete
            && let Some(jev) = &run.jev
            && let Some((index, sure)) = jev_edition(jev, identity, &found).await
        {
            draft.note(
                Source::Jev,
                format!("Jev is {:.0}% sure of this listing", sure * 100.0),
            );
            chosen = Some(index);
        }

        draft.candidates = found.candidates(books, draft.library_id.as_deref());
        if let Some(error) = &found.error {
            draft.warn(Source::Other, format!("A search failed: {error}"), None);
        }
        let Some(index) = chosen else {
            let any = found.verdicts.iter().any(EditionVerdict::plausible);
            draft.status = if any || !draft.candidates.is_empty() {
                "needs_choice"
            } else {
                "unknown"
            };
            draft.title = identity.titles.first().cloned();
            draft.creator = identity.authors.first().cloned();
            draft.summary = Some(if any {
                "More than one listing could be this. Choose one.".to_string()
            } else if draft.candidates.is_empty() {
                "No listing found. Search for it by hand.".to_string()
            } else {
                "None of the listings found fit these files. Choose one or search again."
                    .to_string()
            });
            return;
        };

        let edition = &found.editions[index];
        let verdict = &found.verdicts[index];
        let result = &found.results[index];
        draft.title = Some(edition.title.clone());
        draft.creator = edition.author.clone();
        draft.year = result.year;
        draft.cover_url = result.cover_url.clone();
        draft.provider_id = Some(result.provider_id.clone());
        draft.confidence = verdict.score;
        draft.status = "identified";
        draft.note(
            Source::Listing,
            format!("Found on {}", provider_name(&result.provider_id)),
        );
        draft.note_reasons(Source::Listing, verdict.reasons.iter().cloned());

        // What adding it would do, from the listing's own chapters.
        let Ok(metadata) = self.metadata_service().await else {
            return;
        };
        if let Ok(details) = metadata.get_audiobook(&result.provider_id).await {
            let reference: Vec<release_match::ReferenceChapter> = details
                .chapters
                .iter()
                .map(|chapter| release_match::ReferenceChapter {
                    title: Some(chapter.title.clone()),
                    start: chapter.start_secs,
                    seconds: Some(chapter.length_secs),
                })
                .collect();
            let facts = BookFacts {
                title: details.title.clone(),
                author: details.author_name.clone(),
                narrator: details.narrators.first().cloned(),
                asin: Some(details.provider_id.clone()),
                seconds: details.total_duration_secs.map(|seconds| seconds as f64),
                series: details.series_name.clone(),
            };
            match release_match::layout_book(audio, group, &facts, &reference) {
                BookOutcome::Ready(layout) => draft.summary = Some(layout_line(&layout)),
                BookOutcome::Review(review) => {
                    draft.status = "needs_choice";
                    draft.can_accept = false;
                    draft.summary = Some(review.reason);
                }
            }
        }
    }

    /// Search for listings until one clearly fits, and compare them all with the files.
    async fn book_listings(&self, identity: &GroupIdentity, searches: &[String]) -> BookListings {
        self.search_book_listings(identity, searches, true).await
    }

    /// Every listing the searches turn up, for a person looking for something other than the obvious.
    async fn all_book_listings(
        &self,
        identity: &GroupIdentity,
        searches: &[String],
    ) -> BookListings {
        self.search_book_listings(identity, searches, false).await
    }

    async fn search_book_listings(
        &self,
        identity: &GroupIdentity,
        searches: &[String],
        stop_when_clear: bool,
    ) -> BookListings {
        let mut found = BookListings::default();
        let Ok(metadata) = self.metadata_service().await else {
            found.error = Some("Metadata isn't available".to_string());
            return found;
        };
        let mut seen = HashSet::new();
        for query in searches.iter().take(MAX_SEARCHES_PER_ITEM) {
            match metadata.search_audiobooks(query).await {
                Ok(results) => {
                    for result in results {
                        if seen.insert(result.provider_id.clone()) {
                            found.push(result, identity);
                        }
                    }
                }
                Err(error) => found.error = Some(error.to_string()),
            }
            if stop_when_clear && rules::clear_edition(&found.editions, &found.verdicts).is_some() {
                break;
            }
        }
        // Chapter counts cost a request each, so only for the best few. They also give exact lengths.
        let top: Vec<usize> = rules::rank_editions(&found.editions, &found.verdicts)
            .into_iter()
            .filter(|index| found.verdicts[*index].plausible())
            .take(CHAPTER_LOOKUPS)
            .collect();
        for index in top {
            let id = found.results[index].provider_id.clone();
            if let Ok(details) = metadata.get_audiobook(&id).await {
                found.editions[index].chapters =
                    (!details.chapters.is_empty()).then_some(details.chapters.len());
                if let Some(seconds) = details.total_duration_secs {
                    found.editions[index].seconds = Some(seconds as f64);
                }
                found.verdicts[index] = rules::judge_edition(identity, &found.editions[index]);
            }
        }
        found
    }

    /// The first minute of a book, which usually names it.
    async fn hear_opening(&self, whisper: &Whisper, members: &[&ReleaseFile]) -> Option<String> {
        let files: Vec<(String, f64)> = members
            .iter()
            .take(3)
            .map(|file| (file.key.clone(), file.duration.unwrap_or(0.0)))
            .collect();
        let clip = std::env::temp_dir().join(format!("librarian-listen-{}.wav", Uuid::new_v4()));
        let heard = async {
            whisper::cut_clip(&files, 0.0, OPENING_SECS, &clip).await?;
            whisper.words(&clip).await
        }
        .await;
        let _ = tokio::fs::remove_file(&clip).await;
        match heard {
            Ok(words) if !words.is_empty() => Some(
                words
                    .iter()
                    .map(|word| word.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            Ok(_) => None,
            Err(error) => {
                warn!(error = %error, "Could not listen to a recording's opening");
                None
            }
        }
    }

    // -----------------------------------------------------------------------------------------
    // Music
    // -----------------------------------------------------------------------------------------

    async fn identify_music(&self, run: &RunContext<'_>, audio: &[ReleaseFile]) -> Vec<ItemDraft> {
        let groups = release_match::group_files(audio);
        run.feed
            .write(
                "found",
                &plural(groups.len(), "album or single", "albums or singles"),
                None,
                None,
            )
            .await;
        let database = self.database().await.ok();
        let albums = match &database {
            Some(database) => Album::query(database.pool().pool())
                .filter(AlbumWhereInput {
                    user_id: Some(StringFilter {
                        eq: Some(run.owner.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .fetch_all()
                .await
                .unwrap_or_default(),
            None => Vec::new(),
        };
        let mut artists: HashMap<String, Option<String>> = HashMap::new();
        if let Some(database) = &database {
            for album in &albums {
                if !artists.contains_key(&album.artist_id) {
                    let name = Artist::get(database.pool().pool(), &album.artist_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|artist| artist.name);
                    artists.insert(album.artist_id.clone(), name);
                }
            }
        }
        let mut items = Vec::with_capacity(groups.len());
        for (index, group) in groups.iter().enumerate() {
            items.push(
                self.identify_album_group(
                    run,
                    group,
                    audio,
                    &albums,
                    &artists,
                    index < MAX_PROVIDER_ITEMS,
                )
                .await,
            );
        }
        items
    }

    #[allow(clippy::too_many_arguments)]
    async fn identify_album_group(
        &self,
        run: &RunContext<'_>,
        group: &BookGroup,
        audio: &[ReleaseFile],
        albums: &[Album],
        artists: &HashMap<String, Option<String>>,
        search_providers: bool,
    ) -> ItemDraft {
        let step = run
            .feed
            .start(&format!("Identifying {}", group.label))
            .await;
        let members: Vec<ReleaseFile> = group
            .files
            .iter()
            .map(|index| audio[*index].clone())
            .collect();
        let tags = members.first().map(|file| &file.tags);
        let album_title = tags
            .and_then(|tags| tags.album.as_deref().map(rules::tidy_title))
            .unwrap_or_else(|| rules::title_from_label(&group.label));
        let artist = tags
            .and_then(|tags| tags.album_artist.clone().or_else(|| tags.artist.clone()))
            .map(|name| rules::tidy_person_name(&name));
        let identity = GroupIdentity {
            titles: vec![album_title.clone()],
            authors: artist.clone().into_iter().collect(),
            seconds: members
                .iter()
                .map(|file| file.duration)
                .sum::<Option<f64>>(),
            file_count: members.len(),
            ..Default::default()
        };
        let mut draft = ItemDraft {
            key: group.key.clone(),
            kind: "music",
            status: "unknown",
            label: group.label.clone(),
            title: Some(album_title.clone()),
            creator: artist.clone(),
            paths: members.iter().map(|file| file.key.clone()).collect(),
            finished: members
                .iter()
                .filter(|file| file.duration.is_some())
                .count(),
            seconds: identity.seconds,
            library_id: run.library_for(&["music"]),
            identity,
            ..Default::default()
        };
        if members.len() == 1 {
            draft.note(Source::Files, "One track on its own, so probably a single");
        }

        let names: Vec<Option<String>> = members
            .iter()
            .map(|file| {
                file.tags.title.clone().or_else(|| {
                    Path::new(&file.path)
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .map(str::to_string)
                })
            })
            .collect();
        let line_up = |tracks: &[TrackFacts]| -> f64 {
            if tracks.is_empty() {
                return 0.0;
            }
            let matched = release_match::match_tracks(&members, &names, tracks)
                .iter()
                .filter(|chosen| chosen.is_some())
                .count();
            matched as f64 / members.len().max(1) as f64
        };

        // In the library first: an album it's already known about needs no searching.
        let mut best_library: Option<(&Album, f64, f64, Vec<Track>)> = None;
        for album in albums {
            let facts = BookFacts {
                title: album.name.clone(),
                author: artists.get(&album.artist_id).cloned().flatten(),
                ..Default::default()
            };
            let title = release_match::title_score(&album_title, &facts);
            let artist_differs = match (&artist, &facts.author) {
                (Some(a), Some(b)) => !rules::same_person(a, b),
                _ => false,
            };
            let requested = run.hint.item_id.as_deref() == Some(album.id.as_str());
            if (title < POSSIBLE_TITLE && !requested) || artist_differs {
                continue;
            }
            let tracks = self.album_tracks(&album.id).await.unwrap_or_default();
            let facts: Vec<TrackFacts> = tracks.iter().map(track_facts).collect();
            let lined_up = line_up(&facts);
            if lined_up < ENOUGH_TRACKS && !requested {
                continue;
            }
            if best_library
                .as_ref()
                .is_none_or(|(_, score, ..)| title > *score)
            {
                best_library = Some((album, title, lined_up, tracks));
            }
        }

        if let Some((album, title, lined_up, tracks)) = best_library {
            draft.title = Some(album.name.clone());
            draft.creator = artists.get(&album.artist_id).cloned().flatten();
            draft.year = album.year;
            draft.cover_url = album.cover_url.clone();
            draft.library_item_type = Some("Album");
            draft.library_item_id = Some(album.id.clone());
            draft.library_id = Some(album.library_id.clone());
            draft.confidence = title * 0.4 + lined_up * 0.6;
            draft.note(
                Source::Library,
                format!(
                    "{:.0}% of the files line up with this album's tracks",
                    lined_up * 100.0
                ),
            );
            let wanted = tracks
                .iter()
                .any(|track| track.wanted && track.media_file_id.is_none());
            let requested = run.hint.item_id.as_deref() == Some(album.id.as_str());
            draft.status = if draft.paths.iter().all(|path| run.copied.contains(path)) {
                "imported"
            } else if wanted || requested {
                "wanted"
            } else {
                "in_library"
            };
        } else if search_providers {
            draft.candidates = self
                .album_candidates(&album_title, artist.as_deref(), Some(&line_up))
                .await;
            match draft.candidates.first().cloned() {
                Some(best)
                    if best.score >= CLEAR_ALBUM
                        && best.reasons.iter().any(|reason| reason == "Artist matches") =>
                {
                    draft.title = Some(best.title.clone());
                    draft.creator = best.creator.clone();
                    draft.year = best.year;
                    draft.cover_url = best.cover_url.clone();
                    draft.provider_id = best.provider_id.clone();
                    draft.confidence = best.score;
                    draft.status = "identified";
                    draft.note(Source::Listing, "Found on MusicBrainz");
                    draft.note_reasons(Source::Listing, best.reasons);
                }
                Some(_) => {
                    draft.status = "needs_choice";
                    draft.summary =
                        Some("More than one album could be this. Choose one.".to_string());
                }
                None => {
                    draft.summary = Some("No album found. Search for it by hand.".to_string());
                }
            }
        } else {
            draft.status = "needs_choice";
            draft.summary = Some(
                "Too many albums here to look each one up. Search for this one by hand."
                    .to_string(),
            );
        }

        let (status, message) = match draft.status {
            "unknown" | "needs_choice" => ("warning", format!("{}: not settled", group.label)),
            "imported" => ("found", format!("{}: imported", group.label)),
            _ => (
                "found",
                format!(
                    "{}: {}{}",
                    group.label,
                    draft.title.clone().unwrap_or_default(),
                    draft
                        .creator
                        .as_deref()
                        .map(|creator| format!(" by {creator}"))
                        .unwrap_or_default()
                ),
            ),
        };
        run.feed
            .finish(step, status, &message, Some(draft.evidence_text()))
            .await;
        draft
    }

    async fn album_tracks(&self, album_id: &str) -> Result<Vec<Track>> {
        let database = self.database().await?;
        Ok(Track::query(database.pool().pool())
            .filter(TrackWhereInput {
                album_id: Some(StringFilter {
                    eq: Some(album_id.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?)
    }

    async fn album_candidates(
        &self,
        title: &str,
        artist: Option<&str>,
        line_up: Option<&LineUp<'_>>,
    ) -> Vec<IdentifyCandidate> {
        let Ok(metadata) = self.metadata_service().await else {
            return Vec::new();
        };
        let query = match artist {
            Some(artist) => format!("{title} {artist}"),
            None => title.to_string(),
        };
        let results = metadata
            .search_albums(&query, true, true, true, true, true)
            .await
            .unwrap_or_default();
        let mut candidates: Vec<IdentifyCandidate> = Vec::new();
        for (position, result) in results.into_iter().take(CANDIDATES_KEPT).enumerate() {
            let facts = BookFacts {
                title: result.title.clone(),
                author: result.artist_name.clone(),
                ..Default::default()
            };
            let title_part = release_match::title_score(title, &facts);
            let artist_same = match (artist, result.artist_name.as_deref()) {
                (Some(a), Some(b)) => Some(rules::same_person(a, b)),
                _ => None,
            };
            let mut reasons = Vec::new();
            match artist_same {
                Some(true) => reasons.push("Artist matches".to_string()),
                Some(false) => reasons.push(format!(
                    "A different artist: {}",
                    result.artist_name.as_deref().unwrap_or_default()
                )),
                None => {}
            }
            // Track lengths are the real test, but each listing is a request to a slow service.
            let mut lined_up = None;
            if position < 3
                && artist_same != Some(false)
                && let Some(line_up) = line_up
                && let Ok(id) = Uuid::parse_str(&result.provider_id)
                && let Ok(client) = metadata.get_musicbrainz_client().await
                && let Ok(tracks) = client.get_release_group_tracks(id).await
            {
                let facts: Vec<TrackFacts> = tracks
                    .iter()
                    .map(|track| TrackFacts {
                        title: track.title.clone(),
                        number: Some(track.position),
                        disc: Some(track.disc_number),
                        seconds: track.duration_secs.map(f64::from),
                    })
                    .collect();
                let share = line_up(&facts);
                reasons.push(format!(
                    "{:.0}% of the files line up with its tracks",
                    share * 100.0
                ));
                lined_up = Some(share);
            }
            let score = title_part * 0.5
                + match artist_same {
                    Some(true) => 0.25,
                    None => 0.1,
                    Some(false) => 0.0,
                }
                // Deluxe editions have tracks the listing doesn't, so few lining up doesn't count against.
                + lined_up.map_or(0.1, |share| (share * 0.25).max(0.1));
            candidates.push(IdentifyCandidate {
                source: "provider".to_string(),
                provider: Some("MusicBrainz".to_string()),
                provider_id: Some(result.provider_id),
                title: result.title,
                creator: result.artist_name,
                year: result.year,
                cover_url: result.cover_url,
                score,
                reasons,
                ..Default::default()
            });
        }
        candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
        candidates
    }

    // -----------------------------------------------------------------------------------------
    // TV
    // -----------------------------------------------------------------------------------------

    async fn identify_tv(&self, run: &RunContext<'_>, video: &[ReleaseFile]) -> Vec<ItemDraft> {
        let mut seasons: BTreeMap<(String, i32), (String, Vec<&ReleaseFile>)> = BTreeMap::new();
        for file in video {
            let hint = Self::parse_episode_hint(&file.path);
            let show = hint
                .show_name
                .clone()
                .or_else(|| run.hint.title.clone())
                .unwrap_or_else(|| "Unknown show".to_string());
            let season = hint.season.unwrap_or(1);
            seasons
                .entry((release_match::normalize_words(&show), season))
                .or_insert_with(|| (show, Vec::new()))
                .1
                .push(file);
        }
        run.feed
            .write(
                "found",
                &plural(seasons.len(), "season", "seasons"),
                None,
                None,
            )
            .await;
        let shows = match self.database().await {
            Ok(database) => Show::query(database.pool().pool())
                .filter(ShowWhereInput {
                    user_id: Some(StringFilter {
                        eq: Some(run.owner.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .fetch_all()
                .await
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        let mut items = Vec::with_capacity(seasons.len());
        for ((key, season), (show_name, files)) in seasons {
            let step = run
                .feed
                .start(&format!("Identifying {show_name}, season {season}"))
                .await;
            let label = format!("{show_name}, season {season}");
            let mut draft = ItemDraft {
                key: format!("{key}|{season}"),
                kind: "tv",
                status: "unknown",
                label: label.clone(),
                title: Some(show_name.clone()),
                season: Some(season),
                paths: files.iter().map(|file| file.key.clone()).collect(),
                finished: files.iter().filter(|file| file.duration.is_some()).count(),
                seconds: files.iter().map(|file| file.duration).sum::<Option<f64>>(),
                library_id: run.library_for(&["tv"]),
                identity: GroupIdentity {
                    titles: vec![show_name.clone()],
                    file_count: files.len(),
                    ..Default::default()
                },
                ..Default::default()
            };
            let numbers: HashSet<i32> = files
                .iter()
                .filter_map(|file| Self::parse_episode_hint(&file.path).episode)
                .collect();
            let library_show = shows
                .iter()
                .map(|show| {
                    let requested = run.hint.item_id.as_deref() == Some(show.id.as_str());
                    let score = release_match::title_score(
                        &show_name,
                        &BookFacts {
                            title: show.name.clone(),
                            ..Default::default()
                        },
                    );
                    (show, if requested { 1.0 } else { score })
                })
                .filter(|(_, score)| *score >= POSSIBLE_TITLE)
                .max_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((show, score)) = library_show {
                draft.title = Some(show.name.clone());
                draft.year = show.year;
                draft.cover_url = show.poster_url.clone();
                draft.library_item_type = Some("Show");
                draft.library_item_id = Some(show.id.clone());
                draft.library_id = Some(show.library_id.clone());
                draft.confidence = score;
                let episodes = self.show_season(&show.id, season).await.unwrap_or_default();
                let listed = episodes
                    .iter()
                    .filter(|episode| numbers.contains(&episode.episode))
                    .count();
                let wanted = episodes
                    .iter()
                    .filter(|episode| {
                        numbers.contains(&episode.episode)
                            && episode.wanted
                            && episode.media_file_id.is_none()
                    })
                    .count();
                draft.note(
                    Source::Library,
                    format!(
                        "{listed} of its {} files are episodes this show lists",
                        files.len()
                    ),
                );
                draft.status = if draft.paths.iter().all(|path| run.copied.contains(path)) {
                    "imported"
                } else if wanted > 0 {
                    draft.summary =
                        Some(format!("{} wanted", plural(wanted, "episode", "episodes")));
                    "wanted"
                } else {
                    "in_library"
                };
            } else {
                draft.candidates = self.show_candidates(&show_name).await;
                let contest = draft.candidates.get(1).map_or(0.0, |other| other.score);
                match draft.candidates.first().cloned() {
                    Some(best) if best.score >= CLEAR_TITLE && best.score - contest >= 0.05 => {
                        draft.title = Some(best.title);
                        draft.year = best.year;
                        draft.cover_url = best.cover_url;
                        draft.provider_id = best.provider_id;
                        draft.confidence = best.score;
                        draft.status = "identified";
                        draft.note(Source::Listing, "Found on TVmaze");
                    }
                    Some(_) => {
                        draft.status = "needs_choice";
                        draft.summary =
                            Some("More than one show could be this. Choose one.".to_string());
                    }
                    None => {
                        draft.summary = Some("No show found. Search for it by hand.".to_string())
                    }
                }
            }
            let settled = !matches!(draft.status, "unknown" | "needs_choice");
            run.feed
                .finish(
                    step,
                    if settled { "found" } else { "warning" },
                    &format!(
                        "{label}: {}",
                        if settled {
                            plural(files.len(), "episode", "episodes")
                        } else {
                            "not settled".to_string()
                        }
                    ),
                    Some(draft.evidence_text()),
                )
                .await;
            items.push(draft);
        }
        items
    }

    async fn show_season(&self, show_id: &str, season: i32) -> Result<Vec<Episode>> {
        let database = self.database().await?;
        Ok(Episode::query(database.pool().pool())
            .filter(EpisodeWhereInput {
                show_id: Some(StringFilter {
                    eq: Some(show_id.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?
            .into_iter()
            .filter(|episode| episode.season == season)
            .collect())
    }

    async fn show_candidates(&self, name: &str) -> Vec<IdentifyCandidate> {
        let Ok(metadata) = self.metadata_service().await else {
            return Vec::new();
        };
        let results = metadata.search_tv_shows(name).await.unwrap_or_default();
        let mut candidates: Vec<IdentifyCandidate> = results
            .into_iter()
            .take(CANDIDATES_KEPT)
            .map(|result| IdentifyCandidate {
                source: "provider".to_string(),
                provider: Some("TVmaze".to_string()),
                provider_id: Some(result.provider_id.to_string()),
                score: release_match::title_score(
                    name,
                    &BookFacts {
                        title: result.name.clone(),
                        ..Default::default()
                    },
                ),
                title: result.name,
                creator: result.network,
                year: result.year,
                cover_url: result.poster_url,
                ..Default::default()
            })
            .collect();
        candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
        candidates
    }

    // -----------------------------------------------------------------------------------------
    // Movies
    // -----------------------------------------------------------------------------------------

    async fn identify_movies(&self, run: &RunContext<'_>, video: &[ReleaseFile]) -> Vec<ItemDraft> {
        if video.len() > 1 {
            run.feed
                .write("found", &plural(video.len(), "movie", "movies"), None, None)
                .await;
        }
        let movies = match self.database().await {
            Ok(database) => Movie::query(database.pool().pool())
                .filter(MovieWhereInput {
                    user_id: Some(StringFilter {
                        eq: Some(run.owner.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .fetch_all()
                .await
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        let mut items = Vec::with_capacity(video.len());
        for (index, file) in video.iter().enumerate() {
            let parsed = Self::parse_movie_hint(&file.path);
            let title = parsed
                .title
                .clone()
                .or_else(|| run.hint.title.clone())
                .unwrap_or_else(|| file.path.clone());
            let step = run.feed.start(&format!("Identifying {title}")).await;
            let mut draft = ItemDraft {
                key: release_match::normalize_words(&file.path),
                kind: "movie",
                status: "unknown",
                label: Path::new(&file.path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(&file.path)
                    .to_string(),
                title: Some(title.clone()),
                year: parsed.year,
                paths: vec![file.key.clone()],
                finished: usize::from(file.duration.is_some()),
                seconds: file.duration,
                library_id: run.library_for(&["movies"]),
                identity: GroupIdentity {
                    titles: vec![title.clone()],
                    seconds: file.duration,
                    file_count: 1,
                    ..Default::default()
                },
                ..Default::default()
            };
            let requested = |movie: &Movie| run.hint.item_id.as_deref() == Some(movie.id.as_str());
            let library_movie = movies
                .iter()
                .filter_map(|movie| {
                    let score = release_match::title_score(
                        &title,
                        &BookFacts {
                            title: movie.title.clone(),
                            ..Default::default()
                        },
                    );
                    let year_differs =
                        matches!((parsed.year, movie.year), (Some(a), Some(b)) if a != b);
                    let score = if requested(movie) { 1.0 } else { score };
                    ((score >= CLEAR_TITLE && !year_differs) || requested(movie))
                        .then_some((movie, score))
                })
                .max_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((movie, score)) = library_movie {
                draft.title = Some(movie.title.clone());
                draft.year = movie.year;
                draft.cover_url = movie.poster_url.clone();
                draft.library_item_type = Some("Movie");
                draft.library_item_id = Some(movie.id.clone());
                draft.library_id = Some(movie.library_id.clone());
                draft.confidence = score;
                draft.status = if run.copied.contains(&file.key) {
                    "imported"
                } else if (movie.wanted || requested(movie)) && movie.media_file_id.is_none() {
                    "wanted"
                } else {
                    "in_library"
                };
            } else if index < MAX_PROVIDER_ITEMS {
                draft.candidates = self
                    .movie_candidates(&title, parsed.year, file.duration)
                    .await;
                let contest = draft.candidates.get(1).map_or(0.0, |other| other.score);
                match draft.candidates.first().cloned() {
                    Some(best) if best.score >= CLEAR_TITLE && best.score - contest >= 0.05 => {
                        draft.title = Some(best.title);
                        draft.year = best.year;
                        draft.cover_url = best.cover_url;
                        draft.provider_id = best.provider_id;
                        draft.confidence = best.score;
                        draft.status = "identified";
                        draft.note(Source::Listing, "Found on TMDB");
                        draft.note_reasons(Source::Listing, best.reasons);
                    }
                    Some(_) => {
                        draft.status = "needs_choice";
                        draft.summary =
                            Some("More than one movie could be this. Choose one.".to_string());
                    }
                    None => {
                        draft.summary = Some("No movie found. Search for it by hand.".to_string())
                    }
                }
            } else {
                draft.status = "needs_choice";
                draft.summary = Some(
                    "Too many movies here to look each one up. Search for this one by hand."
                        .to_string(),
                );
            }
            let settled = !matches!(draft.status, "unknown" | "needs_choice");
            run.feed
                .finish(
                    step,
                    if settled { "found" } else { "warning" },
                    &format!(
                        "{}: {}",
                        draft.label,
                        if settled {
                            draft.title.clone().unwrap_or_default()
                        } else {
                            "not settled".to_string()
                        }
                    ),
                    Some(draft.evidence_text()),
                )
                .await;
            items.push(draft);
        }
        items
    }

    async fn movie_candidates(
        &self,
        title: &str,
        year: Option<i32>,
        seconds: Option<f64>,
    ) -> Vec<IdentifyCandidate> {
        let Ok(metadata) = self.metadata_service().await else {
            return Vec::new();
        };
        let results = metadata
            .search_movies(title, year)
            .await
            .unwrap_or_default();
        let mut candidates = Vec::new();
        for (position, result) in results.into_iter().take(CANDIDATES_KEPT).enumerate() {
            let title_part = release_match::title_score(
                title,
                &BookFacts {
                    title: result.title.clone(),
                    ..Default::default()
                },
            );
            let mut reasons = Vec::new();
            let year_part = match (year, result.year) {
                (Some(a), Some(b)) if a == b => {
                    reasons.push("Year matches".to_string());
                    1.0
                }
                (Some(_), Some(b)) => {
                    reasons.push(format!("A different year: {b}"));
                    0.0
                }
                _ => 0.5,
            };
            // Runtime separates remakes and same-named films, but it's a request each.
            let mut runtime_part = 0.5;
            if position < 3
                && let Some(seconds) = seconds
                && let Ok(details) = metadata.get_movie(result.provider_id).await
                && let Some(runtime) = details.runtime.filter(|runtime| *runtime > 0)
            {
                let ratio = seconds / (f64::from(runtime) * 60.0);
                runtime_part = if (ratio - 1.0).abs() <= RUNTIME_TOLERANCE {
                    reasons.push(format!("Runtime fits: {runtime} min"));
                    1.0
                } else {
                    reasons.push(format!("Runtime doesn't fit: {runtime} min"));
                    0.0
                };
            }
            candidates.push(IdentifyCandidate {
                source: "provider".to_string(),
                provider: Some("TMDB".to_string()),
                provider_id: Some(result.provider_id.to_string()),
                title: result.title,
                year: result.year,
                cover_url: result.poster_url,
                score: title_part * 0.6 + year_part * 0.25 + runtime_part * 0.15,
                reasons,
                ..Default::default()
            });
        }
        candidates.sort_by(|a, b| b.score.total_cmp(&a.score));
        candidates
    }

    // -----------------------------------------------------------------------------------------
    // Results
    // -----------------------------------------------------------------------------------------

    /// Save what was found, import what was wanted, and say how it went.
    async fn settle_results(
        self: &Arc<Self>,
        run: &RunContext<'_>,
        mut drafts: Vec<ItemDraft>,
    ) -> String {
        let existing = self.identified_items(&run.hash).await.unwrap_or_default();
        // What's imported stays imported, even if a later look at the files would say otherwise.
        for draft in &mut drafts {
            if let Some(previous) = existing.iter().find(|item| item.item_key == draft.key)
                && previous.status == "imported"
            {
                draft.status = "imported";
                draft.title = previous.title.clone().or(draft.title.take());
                draft.creator = previous.creator.clone().or(draft.creator.take());
                draft.library_item_type = previous
                    .library_item_type
                    .as_deref()
                    .and_then(static_item_type);
                draft.library_item_id = previous.library_item_id.clone();
                draft.library_id = previous.library_id.clone();
                draft.summary = previous.summary.clone();
            }
        }
        let keys: HashSet<&str> = drafts.iter().map(|draft| draft.key.as_str()).collect();
        for item in &existing {
            if !keys.contains(item.item_key.as_str()) && item.status != "imported" {
                self.delete_identified_item(&item.id).await;
            }
        }
        for draft in &drafts {
            self.save_identified_item(run, draft, &existing).await;
        }
        let mut changed = false;
        for draft in &mut drafts {
            if !run.may_import || draft.status != "wanted" || !draft.complete() {
                continue;
            }
            if self.import_wanted_item(run, draft).await {
                changed = true;
            }
        }
        if changed {
            let existing = self.identified_items(&run.hash).await.unwrap_or_default();
            for draft in &drafts {
                self.save_identified_item(run, draft, &existing).await;
            }
        }
        summarize_items(&drafts)
    }

    async fn identified_items(&self, info_hash: &str) -> Result<Vec<IdentifiedItem>> {
        let database = self.database().await?;
        Ok(IdentifiedItem::query(database.pool().pool())
            .filter(IdentifiedItemWhereInput {
                info_hash: Some(StringFilter {
                    eq: Some(info_hash.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?)
    }

    async fn delete_identified_item(&self, id: &str) {
        if let Err(error) = self
            .execute_mutation(
                &AuthUser::system_admin(),
                r#"mutation ForgetIdentifiedItem($id: String!) {
                    Deleted: deleteIdentifiedItem(id: $id) { Success: success }
                }"#,
                serde_json::json!({ "id": id }),
            )
            .await
        {
            warn!(id, error = %error, "Could not remove an old identification result");
        }
    }

    async fn save_identified_item(
        &self,
        run: &RunContext<'_>,
        draft: &ItemDraft,
        existing: &[IdentifiedItem],
    ) {
        let json = |value: &dyn erased_json::Json| value.to_json();
        let input = serde_json::json!({
            "kind": draft.kind,
            "status": draft.status,
            "label": draft.label,
            "title": draft.title,
            "creator": draft.creator,
            "year": draft.year,
            "coverUrl": draft.cover_url,
            "fileCount": draft.paths.len() as i32,
            "finishedFiles": draft.finished as i32,
            "seconds": draft.seconds,
            "confidence": draft.confidence,
            "summary": draft.summary,
            "evidence": json(&draft.evidence),
            "candidates": json(&draft.candidates),
            "filePaths": json(&draft.paths),
            "identity": json(&draft.identity),
            "providerId": draft.provider_id,
            "libraryItemType": draft.library_item_type,
            "libraryItemId": draft.library_item_id,
            "libraryId": draft.library_id,
            "season": draft.season,
            "canAccept": draft.can_accept,
        });
        let result = match existing.iter().find(|item| item.item_key == draft.key) {
            Some(item) => {
                self.execute_mutation(
                    &AuthUser::system_admin(),
                    r#"mutation UpdateFound($id: String!, $input: UpdateIdentifiedItemInput!) {
                        Saved: updateIdentifiedItem(id: $id, input: $input) { Success: success Error: error }
                    }"#,
                    serde_json::json!({ "id": item.id, "input": input }),
                )
                .await
            }
            None => {
                let mut create = input;
                create["userId"] = serde_json::json!(run.owner);
                create["infoHash"] = serde_json::json!(run.hash);
                create["itemKey"] = serde_json::json!(draft.key);
                self.execute_mutation(
                    &AuthUser::system_admin(),
                    r#"mutation CreateFound($input: CreateIdentifiedItemInput!) {
                        Saved: createIdentifiedItem(input: $input) { Success: success Error: error }
                    }"#,
                    serde_json::json!({ "input": create }),
                )
                .await
            }
        };
        match result {
            Ok(data)
                if data
                    .pointer("/Saved/Success")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true) => {}
            Ok(data) => {
                warn!(key = %draft.key, data = %data, "Could not save an identification result")
            }
            Err(error) => {
                warn!(key = %draft.key, error = %error, "Could not save an identification result")
            }
        }
    }

    /// Something a person already asked for, now finished: bring it in without being asked.
    async fn import_wanted_item(
        self: &Arc<Self>,
        run: &RunContext<'_>,
        draft: &mut ItemDraft,
    ) -> bool {
        let Some(item_id) = draft.library_item_id.clone() else {
            return false;
        };
        let title = draft.title.clone().unwrap_or_else(|| draft.label.clone());
        let step = run.feed.start(&format!("Importing {title}")).await;
        let files = run
            .files
            .iter()
            .filter(|file| draft.paths.contains(&file.source_path))
            .cloned()
            .collect();
        let summary = self
            .import_item_files(
                &AuthUser::system_admin(),
                &run.hash,
                draft.kind,
                &item_id,
                draft.group_key.as_deref(),
                draft.season,
                files,
                false,
            )
            .await;
        if !summary.success || summary.files_processed == 0 {
            let reason = summary
                .error
                .unwrap_or_else(|| "Nothing was imported".to_string());
            run.feed
                .finish(
                    step,
                    "failed",
                    &format!("Couldn't import {title}"),
                    Some(reason.clone()),
                )
                .await;
            draft.status = "needs_choice";
            draft.summary = Some(reason);
            return true;
        }
        run.feed
            .finish(
                step,
                "found",
                &format!(
                    "Imported {title}: {}",
                    plural(summary.files_processed as usize, "file", "files")
                ),
                None,
            )
            .await;
        draft.status = "imported";
        draft.summary = Some("Imported and ready to play.".to_string());
        if draft.kind == "audiobook"
            && let Some(whisper) = &run.listening
        {
            let step = run
                .feed
                .start(&format!("Listening to {title}'s chapters"))
                .await;
            match self.check_book_chapters(&item_id, whisper).await {
                Ok(report) => run.feed.finish(step, "found", &report, None).await,
                Err(error) => {
                    run.feed
                        .finish(
                            step,
                            "warning",
                            "Couldn't check the chapters by listening",
                            Some(error.to_string()),
                        )
                        .await;
                }
            }
        }
        true
    }

    /// Whether an unlinked download is audiobooks. Those wait for identification, because the generic
    /// matcher looks at one file at a time, and a pack's parts look like chapters of the wrong book.
    pub(super) async fn looks_like_audiobooks(
        &self,
        owner: &str,
        name: &str,
        files: &[ImportFile],
    ) -> bool {
        let audio: Vec<&SourceFileImport> = files
            .iter()
            .map(|item| &item.file)
            .filter(|file| Self::is_audio_import_path(&file.source_path))
            .collect();
        let video = files
            .iter()
            .filter(|item| Self::is_video_import_path(&item.file.source_path))
            .count();
        if audio.is_empty() || video > audio.len() {
            return false;
        }
        let facts = self.release_facts_for(&audio).await;
        let albums: HashSet<String> = facts
            .iter()
            .filter_map(|file| file.tags.album.as_deref().map(normalize_album))
            .collect();
        let (kind, _) = rules::classify_kind(&rules::KindFacts {
            name: name.to_string(),
            audio_files: facts.len(),
            video_files: video,
            audiobook_format_files: facts
                .iter()
                .filter(|file| file.path.to_ascii_lowercase().ends_with(".m4b"))
                .count(),
            audio_seconds: facts.iter().filter_map(|file| file.duration).collect(),
            album_tags: albums.len(),
            audiobook_genre_files: facts
                .iter()
                .filter(|file| {
                    file.tags
                        .genre
                        .as_deref()
                        .is_some_and(rules::is_audiobook_genre)
                })
                .count(),
            in_library: self.library_kind_of_audio(owner, name, &facts, &[]).await,
            ..Default::default()
        });
        kind == ReleaseKind::Audiobook
    }

    /// Other listings of a book, compared with the files it has.
    pub async fn book_editions(
        &self,
        audiobook_id: &str,
        query: Option<String>,
    ) -> Result<Vec<IdentifyCandidate>> {
        let (book, chapters, _, _) = self.book_reference(audiobook_id).await?;
        let files = self.book_files_in_order(&chapters).await?;
        let seconds = files
            .iter()
            .map(|file| file.duration.map(f64::from))
            .sum::<Option<f64>>()
            .filter(|_| !files.is_empty())
            .or(book.total_duration_secs.map(f64::from));
        let query = query
            .map(|query| query.trim().to_string())
            .filter(|query| !query.is_empty());
        let main_title = book
            .title
            .split(':')
            .next()
            .unwrap_or(&book.title)
            .trim()
            .to_string();
        let identity = GroupIdentity {
            titles: dedupe(
                query
                    .clone()
                    .into_iter()
                    .chain([main_title.clone(), book.title.clone()]),
            ),
            authors: book.author_name.clone().into_iter().collect(),
            seconds,
            file_count: files.len(),
            ..Default::default()
        };
        let searches = match query {
            Some(query) => vec![query],
            None => rules::edition_searches(&[main_title], &identity.authors),
        };
        let found = self.all_book_listings(&identity, &searches).await;
        if let Some(error) = &found.error
            && found.results.is_empty()
        {
            anyhow::bail!("{error}");
        }
        Ok(found.candidates(&[], Some(&book.library_id)))
    }

    /// Make a book a different listing: its details and chapters change, and its files stay, placed
    /// on the new chapters.
    pub async fn change_book_edition(&self, audiobook_id: &str, provider_id: &str) -> Result<()> {
        let owner = AuthUser::system_admin();
        let (book, chapters, _, _) = self.book_reference(audiobook_id).await?;
        if self
            .owned_books(&book.user_id)
            .await?
            .iter()
            .any(|other| other.id != book.id && other.audible_id.as_deref() == Some(provider_id))
        {
            anyhow::bail!("That listing is already another book in your library");
        }
        let metadata = self.metadata_service().await?;
        let details = metadata.get_audiobook(provider_id).await?;
        let files = self.book_files_in_order(&chapters).await?;
        let durations: Vec<f64> = files
            .iter()
            .map(|file| file.duration.map_or(0.0, f64::from))
            .collect();
        let reference: Vec<release_match::ReferenceChapter> = details
            .chapters
            .iter()
            .map(|chapter| release_match::ReferenceChapter {
                title: Some(chapter.title.clone()),
                start: chapter.start_secs,
                seconds: Some(chapter.length_secs),
            })
            .collect();
        self.update_audiobook_fields(
            &owner,
            &book.id,
            serde_json::json!({
                "title": details.title,
                "sortTitle": details.sort_title,
                "authorName": details.author_name,
                "description": details.description,
                "publisher": details.publisher,
                "language": details.language,
                "isbn": details.isbn,
                "publishedDate": details.published_date,
                "coverUrl": details.cover_url,
                "audibleId": details.provider_id,
                "asin": (details.provider == MetadataProvider::Audible).then_some(&details.provider_id),
                "narratorName": details.narrators.first(),
                "narrators": details.narrators,
                "totalDurationSecs": details.total_duration_secs,
                "seriesName": details.series_name,
                "seriesPosition": details.series_position,
                "providerChapters": serde_json::Value::Null,
                "chapterCheckState": serde_json::Value::Null,
                "chapterCheck": serde_json::Value::Null,
            }),
        )
        .await?;
        // A listing without chapters says nothing about them, so the ones it has stay.
        if reference.is_empty() {
            return Ok(());
        }
        let mut changed = book.clone();
        changed.provider_chapters = None;
        changed.chapter_source = None;
        let layout = release_match::provider_layout(
            &durations,
            &reference,
            details.total_duration_secs.map(|seconds| seconds as f64),
        );
        let ids = self
            .rewrite_book_chapters(&owner, &changed, &chapters, &[], &layout, true)
            .await?;
        self.relink_book_files(&files, &layout, &ids).await?;
        let placements = layout
            .chapters
            .iter()
            .enumerate()
            .filter_map(|(index, chapter)| {
                Some((
                    ids.get(index)?.clone(),
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
                source: layout.source.as_str(),
                shape: layout.shape.as_str(),
            },
            &imported,
        )
        .await?;
        self.update_audiobook_fields(
            &owner,
            &book.id,
            serde_json::json!({ "chapterCount": reference.len() as i32 }),
        )
        .await
    }

    /// Whether listening can run, for the settings page.
    pub async fn listening_status(&self) -> (bool, String) {
        match Whisper::from_env().check().await {
            Ok(()) => (true, "Installed and ready.".to_string()),
            Err(reason) => (false, reason),
        }
    }

    /// Bring in one result's files as a library item, by the same path a linked download takes.
    #[allow(clippy::too_many_arguments)]
    async fn import_item_files(
        &self,
        actor: &AuthUser,
        info_hash: &str,
        kind: &str,
        item_id: &str,
        group_key: Option<&str>,
        season: Option<i32>,
        files: Vec<SourceFileImport>,
        accept_review: bool,
    ) -> SourceProcessSummary {
        if kind == "audiobook" {
            return self
                .import_torrent_as_book(
                    actor,
                    info_hash,
                    Some(item_id),
                    files,
                    BookImportChoice {
                        group_key: group_key.map(str::to_string),
                        accept_review,
                    },
                )
                .await;
        }
        let patch = match kind {
            "movie" => ImportLinkPatch {
                movie_id: Some(item_id.to_string()),
                ..Default::default()
            },
            "tv" => ImportLinkPatch {
                show_id: Some(item_id.to_string()),
                season,
                ..Default::default()
            },
            "music" => ImportLinkPatch {
                album_id: Some(item_id.to_string()),
                ..Default::default()
            },
            other => {
                return Self::failed_source_summary(anyhow::anyhow!(
                    "A {other} result can't be imported"
                ));
            }
        };
        self.import_torrent_with_link(actor, info_hash, files, patch)
            .await
    }

    /// What a result could be, for a person to choose from: their own search, or the original one.
    pub async fn identification_candidates(
        &self,
        actor: &AuthUser,
        item_id: &str,
        query: Option<String>,
    ) -> Result<Vec<IdentifyCandidate>> {
        let database = self.database().await?;
        let item = IdentifiedItem::get(database.pool().pool(), &item_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("That result no longer exists"))?;
        let import =
            Self::authorize_torrent_import(database.pool(), actor, &item.info_hash, None).await?;
        let mut identity: GroupIdentity = serde_json::from_str(&item.identity).unwrap_or_default();
        let query = query
            .map(|query| query.trim().to_string())
            .filter(|query| !query.is_empty());
        if let Some(query) = &query {
            // What a person types is the title to compare with, as well as what to search for.
            identity.titles.push(query.clone());
        }
        let title = query
            .clone()
            .or_else(|| identity.titles.first().cloned())
            .unwrap_or_else(|| item.label.clone());
        Ok(match item.kind.as_str() {
            "audiobook" => {
                let searches = match &query {
                    Some(query) => vec![query.clone()],
                    None => rules::edition_searches(&identity.titles, &identity.authors),
                };
                let books = self.owned_books(&import.owner_id).await?;
                let found = self.all_book_listings(&identity, &searches).await;
                let mut candidates: Vec<IdentifyCandidate> = books
                    .iter()
                    .filter_map(|book| {
                        let verdict = rules::judge_edition(&identity, &edition_of_book(book));
                        (verdict.title_score >= 0.5).then(|| candidate_from_book(book, &verdict))
                    })
                    .collect();
                // Something in the library is listed already, whichever way it was found.
                let listed: HashSet<String> = candidates
                    .iter()
                    .filter_map(|candidate| candidate.library_item_id.clone())
                    .collect();
                candidates.extend(
                    found
                        .candidates(&books, item.library_id.as_deref())
                        .into_iter()
                        .filter(|candidate| {
                            candidate
                                .library_item_id
                                .as_ref()
                                .is_none_or(|id| !listed.contains(id))
                        }),
                );
                candidates
            }
            "music" => {
                self.album_candidates(&title, identity.authors.first().map(String::as_str), None)
                    .await
            }
            "tv" => self.show_candidates(&title).await,
            "movie" => {
                self.movie_candidates(&title, item.year, identity.seconds)
                    .await
            }
            _ => Vec::new(),
        })
    }

    /// Do what a person chose: add from a provider if it isn't in the library yet, then import.
    pub async fn import_identified_item(
        self: &Arc<Self>,
        actor: &AuthUser,
        choice: IdentifiedChoice,
    ) -> SourceProcessSummary {
        let prepared = async {
            let database = self.database().await?;
            let item = IdentifiedItem::get(database.pool().pool(), &choice.item_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("That result no longer exists"))?;
            let library_id = choice
                .library_id
                .clone()
                .or_else(|| item.library_id.clone());
            let import = Self::authorize_torrent_import(
                database.pool(),
                actor,
                &item.info_hash,
                library_id.as_deref(),
            )
            .await?;
            let target = match (&choice.library_item_id, &choice.provider_id) {
                (Some(id), _) => id.clone(),
                (None, Some(provider_id)) => {
                    self.add_from_provider(
                        &item,
                        provider_id,
                        library_id.as_deref(),
                        &import.owner_id,
                    )
                    .await?
                }
                (None, None) => match (&item.library_item_id, &item.provider_id) {
                    (Some(id), _) => id.clone(),
                    (None, Some(provider_id)) => {
                        self.add_from_provider(
                            &item,
                            provider_id,
                            library_id.as_deref(),
                            &import.owner_id,
                        )
                        .await?
                    }
                    (None, None) => anyhow::bail!("Choose what this is first"),
                },
            };
            anyhow::Ok((item, target))
        }
        .await;
        let (item, target) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => return Self::failed_source_summary(error),
        };
        let torrents = match self.manager.get_torrent().await {
            Some(torrents) => torrents,
            None => {
                return Self::failed_source_summary(anyhow::anyhow!(
                    "The torrent client isn't running"
                ));
            }
        };
        let paths: Vec<String> = serde_json::from_str(&item.file_paths).unwrap_or_default();
        let row = match self.torrent_row(&item.info_hash).await {
            Ok(Some(row)) => row,
            Ok(None) => {
                return Self::failed_source_summary(anyhow::anyhow!(
                    "That download no longer exists"
                ));
            }
            Err(error) => return Self::failed_source_summary(error),
        };
        let files = match torrents
            .release_files(&item.info_hash)
            .await
            .map(|files| settled_files(&row, files))
        {
            Ok(files) if item.kind == "audiobook" => files,
            Ok(files) => files
                .into_iter()
                .filter(|file| paths.contains(&file.source_path))
                .collect(),
            Err(error) => return Self::failed_source_summary(error),
        };
        let summary = self
            .import_item_files(
                actor,
                &item.info_hash,
                &item.kind,
                &target,
                (item.kind == "audiobook").then_some(item.item_key.as_str()),
                item.season,
                files,
                choice.accept_review,
            )
            .await;
        if summary.success {
            if let Err(error) = self.mark_item_imported(&item, &target).await {
                warn!(error = %error, "Could not record an imported result");
            }
            if item.kind == "audiobook" && self.available_listener().await.is_some() {
                self.start_chapter_check(&target);
            }
        }
        self.request_identification(&item.info_hash, IdentifyTrigger::Asked);
        summary
    }

    async fn mark_item_imported(&self, item: &IdentifiedItem, target: &str) -> Result<()> {
        let database = self.database().await?;
        let pool = database.pool().pool();
        let (item_type, title, creator, cover, library): ItemFacts = match item.kind.as_str() {
            "audiobook" => {
                let book = Audiobook::get(pool, &target.to_string()).await?;
                (
                    "Audiobook",
                    book.as_ref().map(|book| book.title.clone()),
                    book.as_ref().and_then(|book| book.author_name.clone()),
                    book.as_ref().and_then(|book| book.cover_url.clone()),
                    book.map(|book| book.library_id),
                )
            }
            "movie" => {
                let movie = Movie::get(pool, &target.to_string()).await?;
                (
                    "Movie",
                    movie.as_ref().map(|movie| movie.title.clone()),
                    None,
                    movie.as_ref().and_then(|movie| movie.poster_url.clone()),
                    movie.map(|movie| movie.library_id),
                )
            }
            "tv" => {
                let show = Show::get(pool, &target.to_string()).await?;
                (
                    "Show",
                    show.as_ref().map(|show| show.name.clone()),
                    None,
                    show.as_ref().and_then(|show| show.poster_url.clone()),
                    show.map(|show| show.library_id),
                )
            }
            _ => {
                let album = Album::get(pool, &target.to_string()).await?;
                (
                    "Album",
                    album.as_ref().map(|album| album.name.clone()),
                    item.creator.clone(),
                    album.as_ref().and_then(|album| album.cover_url.clone()),
                    album.map(|album| album.library_id),
                )
            }
        };
        self.execute_mutation(
            &AuthUser::system_admin(),
            r#"mutation MarkFoundImported($id: String!, $input: UpdateIdentifiedItemInput!) {
                Saved: updateIdentifiedItem(id: $id, input: $input) { Success: success }
            }"#,
            serde_json::json!({ "id": item.id, "input": {
                "status": "imported",
                "title": title,
                "creator": creator,
                "coverUrl": cover,
                "libraryItemType": item_type,
                "libraryItemId": target,
                "libraryId": library,
                "summary": "Imported and ready to play.",
            }}),
        )
        .await?;
        Ok(())
    }

    async fn add_from_provider(
        &self,
        item: &IdentifiedItem,
        provider_id: &str,
        library_id: Option<&str>,
        owner: &str,
    ) -> Result<String> {
        let library =
            Uuid::parse_str(library_id.ok_or_else(|| {
                anyhow::anyhow!("There's no library for this kind of media yet")
            })?)?;
        let owner = Uuid::parse_str(owner)?;
        let metadata = self.metadata_service().await?;
        Ok(match item.kind.as_str() {
            "audiobook" => metadata
                .add_audiobook_from_provider(AddAudiobookOptions {
                    provider: crate::services::metadata::providers::audiobook_provider_for_id(
                        provider_id,
                    ),
                    provider_id: provider_id.to_string(),
                    library_id: library,
                    user_id: owner,
                    monitor_type: AutoDownloadMode::None,
                })
                .await?
                .id
                .to_string(),
            "movie" => metadata
                .add_movie_from_provider(crate::services::metadata::providers::AddMovieOptions {
                    provider: MetadataProvider::Tmdb,
                    provider_id: provider_id.parse()?,
                    library_id: library,
                    user_id: owner,
                    monitored: false,
                })
                .await?
                .id
                .to_string(),
            "tv" => metadata
                .add_tv_show_from_provider(crate::services::metadata::providers::AddTvShowOptions {
                    provider: MetadataProvider::Tvmaze,
                    provider_id: provider_id.parse()?,
                    library_id: library,
                    user_id: owner,
                    monitor_type: AutoDownloadMode::None,
                    path: None,
                })
                .await?
                .id
                .to_string(),
            "music" => metadata
                .add_album_from_provider(crate::services::metadata::providers::AddAlbumOptions {
                    provider: MetadataProvider::Musicbrainz,
                    provider_id: provider_id.to_string(),
                    library_id: library,
                    user_id: owner,
                    monitor_type: AutoDownloadMode::None,
                })
                .await?
                .id
                .to_string(),
            other => anyhow::bail!("A {other} result can't be added"),
        })
    }
}

fn book_identity(
    text: &[TextBlock],
    torrent_name: &str,
    group: &BookGroup,
    audio: &[ReleaseFile],
) -> (GroupIdentity, Option<TextBlock>) {
    let members: Vec<&ReleaseFile> = group.files.iter().map(|index| &audio[*index]).collect();
    let first = members.first();
    let tags = first.map(|file| &file.tags);
    let folder = first.map(|file| folder_label(file, torrent_name));
    let album = tags.and_then(|tags| tags.album.clone());
    let lookup = album
        .as_deref()
        .map_or(group.label.clone(), rules::tidy_title);
    let block = rules::block_for_group(text, &lookup, members.len()).cloned();
    let titles = dedupe(
        [
            album.as_deref().map(rules::tidy_title),
            block
                .as_ref()
                .and_then(|block| block.title.as_deref().map(rules::tidy_title)),
            folder.as_deref().map(rules::title_from_label),
            Some(rules::title_from_label(&group.label)),
        ]
        .into_iter()
        .flatten(),
    );
    let authors = dedupe(
        [
            folder.as_deref().and_then(rules::author_from_label),
            tags.and_then(|tags| tags.album_artist.as_deref().map(rules::tidy_person_name)),
            tags.and_then(|tags| tags.artist.as_deref().map(rules::tidy_person_name)),
            block
                .as_ref()
                .and_then(|block| block.author.as_deref().map(rules::tidy_person_name)),
        ]
        .into_iter()
        .flatten(),
    );
    let narrators = dedupe(
        [
            tags.and_then(|tags| tags.narrator.clone()),
            block.as_ref().and_then(|block| block.narrator.clone()),
        ]
        .into_iter()
        .flatten(),
    );
    let identity = GroupIdentity {
        titles,
        authors,
        narrators,
        seconds: members
            .iter()
            .map(|file| file.duration)
            .sum::<Option<f64>>(),
        file_count: members.len(),
    };
    (identity, block)
}

/// How much of a set of files lines up with an album's tracks.
type LineUp<'a> = dyn Fn(&[TrackFacts]) -> f64 + Sync + 'a;

/// What a library item says about itself, for a result that became it.
type ItemFacts = (
    &'static str,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// Provider listings for a book, compared with its files.
#[derive(Default)]
struct BookListings {
    results: Vec<crate::services::metadata::providers::AudiobookSearchResult>,
    editions: Vec<EditionFacts>,
    verdicts: Vec<EditionVerdict>,
    error: Option<String>,
}

impl BookListings {
    fn push(
        &mut self,
        result: crate::services::metadata::providers::AudiobookSearchResult,
        identity: &GroupIdentity,
    ) {
        let edition = EditionFacts {
            title: result.title.clone(),
            author: result.author_name.clone(),
            narrator: result.narrator.clone(),
            seconds: result.duration_secs.map(|seconds| seconds as f64),
            chapters: None,
        };
        self.verdicts.push(rules::judge_edition(identity, &edition));
        self.editions.push(edition);
        self.results.push(result);
    }

    fn merge(&mut self, other: BookListings, identity: &GroupIdentity) {
        let known: HashSet<String> = self
            .results
            .iter()
            .map(|result| result.provider_id.clone())
            .collect();
        for (result, edition) in other.results.into_iter().zip(other.editions) {
            if !known.contains(&result.provider_id) {
                self.verdicts.push(rules::judge_edition(identity, &edition));
                self.editions.push(edition);
                self.results.push(result);
            }
        }
        // What was heard may have added a name, which changes earlier comparisons too.
        self.verdicts = self
            .editions
            .iter()
            .map(|edition| rules::judge_edition(identity, edition))
            .collect();
    }

    /// The best of them to show. Same-titled listings by other people aren't worth a person's time
    /// unless nothing better turned up, so wrong authors go last.
    fn candidates(&self, books: &[Audiobook], library_id: Option<&str>) -> Vec<IdentifyCandidate> {
        // Search results run on to anything sharing a word. Those are noise beside a real title match,
        // though if nothing matches, as when searching by author, they're all there is to show.
        const RELATED_TITLE: f64 = 0.5;
        let related = self
            .verdicts
            .iter()
            .any(|verdict| verdict.title_score >= RELATED_TITLE);
        rules::rank_editions(&self.editions, &self.verdicts)
            .into_iter()
            .filter(|index| !related || self.verdicts[*index].title_score >= RELATED_TITLE)
            .take(CANDIDATES_KEPT)
            .map(|index| {
                let edition = &self.editions[index];
                let verdict = &self.verdicts[index];
                let result = &self.results[index];
                let in_library = books
                    .iter()
                    .find(|book| book.audible_id.as_deref() == Some(result.provider_id.as_str()));
                IdentifyCandidate {
                    source: if in_library.is_some() {
                        "library"
                    } else {
                        "provider"
                    }
                    .to_string(),
                    provider: Some(provider_name(&result.provider_id).to_string()),
                    provider_id: Some(result.provider_id.clone()),
                    library_item_type: in_library.map(|_| "Audiobook".to_string()),
                    library_item_id: in_library.map(|book| book.id.clone()),
                    library_id: in_library
                        .map(|book| book.library_id.clone())
                        .or_else(|| library_id.map(str::to_string)),
                    title: edition.title.clone(),
                    creator: edition.author.clone(),
                    narrator: edition.narrator.clone(),
                    year: result.year,
                    cover_url: result.cover_url.clone(),
                    seconds: edition.seconds,
                    chapters: edition.chapters.map(|count| count as i32),
                    length_percent: verdict.length_ratio.map(|ratio| 100.0 / ratio),
                    fit: Some(verdict.fit.as_str().to_string()),
                    score: verdict.score,
                    reasons: verdict.reasons.clone(),
                }
            })
            .collect()
    }
}

fn listing_from_details(
    details: &crate::services::metadata::providers::AudiobookDetails,
) -> crate::services::metadata::providers::AudiobookSearchResult {
    crate::services::metadata::providers::AudiobookSearchResult {
        provider: details.provider,
        provider_id: details.provider_id.clone(),
        title: details.title.clone(),
        author_name: details.author_name.clone(),
        year: details
            .published_date
            .as_deref()
            .and_then(|date| date.get(..4))
            .and_then(|year| year.parse().ok()),
        cover_url: details.cover_url.clone(),
        isbn: details.isbn.clone(),
        description: details.description.clone(),
        duration_secs: details.total_duration_secs,
        narrator: details.narrators.first().cloned(),
    }
}

fn provider_name(id: &str) -> &'static str {
    match crate::services::metadata::providers::audiobook_provider_for_id(id) {
        MetadataProvider::Audible => "Audible",
        _ => "Open Library",
    }
}

fn edition_of_book(book: &Audiobook) -> EditionFacts {
    EditionFacts {
        title: book.title.clone(),
        author: book.author_name.clone(),
        narrator: book.narrator_name.clone(),
        seconds: book.total_duration_secs.map(f64::from),
        chapters: book.chapter_count.map(|count| count as usize),
    }
}

fn candidate_from_book(book: &Audiobook, verdict: &EditionVerdict) -> IdentifyCandidate {
    IdentifyCandidate {
        source: "library".to_string(),
        library_item_type: Some("Audiobook".to_string()),
        library_item_id: Some(book.id.clone()),
        library_id: Some(book.library_id.clone()),
        title: book.title.clone(),
        creator: book.author_name.clone(),
        narrator: book.narrator_name.clone(),
        cover_url: book.cover_url.clone(),
        seconds: book.total_duration_secs.map(f64::from),
        chapters: book.chapter_count,
        length_percent: verdict.length_ratio.map(|ratio| 100.0 / ratio),
        fit: Some(verdict.fit.as_str().to_string()),
        score: verdict.score,
        reasons: verdict.reasons.clone(),
        ..Default::default()
    }
}

fn track_facts(track: &Track) -> TrackFacts {
    TrackFacts {
        title: track.title.clone(),
        number: Some(track.track_number),
        disc: track.disc_number,
        seconds: track.duration_secs.map(f64::from),
    }
}

fn static_item_type(item_type: &str) -> Option<&'static str> {
    match item_type {
        "Audiobook" => Some("Audiobook"),
        "Album" => Some("Album"),
        "Show" => Some("Show"),
        "Movie" => Some("Movie"),
        _ => None,
    }
}

/// A person's words for what a layout would do.
fn layout_line(layout: &release_match::BookLayout) -> String {
    let chapters = plural(layout.chapters.len(), "chapter", "chapters");
    match layout.source {
        release_match::ChapterSource::Embedded => {
            format!("{chapters} from markers inside the files.")
        }
        release_match::ChapterSource::Files => format!("{chapters} from the files' own names."),
        release_match::ChapterSource::Provider => match layout.shape {
            release_match::FileShape::WholeBook => {
                format!("{chapters} from the listing, in one file.")
            }
            release_match::FileShape::PerChapter => {
                format!("{chapters} from the listing, one file each.")
            }
            release_match::FileShape::Parts => {
                format!("{chapters} from the listing, placed by time.")
            }
        },
        release_match::ChapterSource::Parts => {
            "No chapter information, so numbered parts.".to_string()
        }
        release_match::ChapterSource::Listened => {
            format!("{chapters}, placed where they were heard.")
        }
    }
}

/// The one listing whose title and people are named in what was heard.
fn heard_edition(heard: &str, found: &BookListings) -> Option<usize> {
    let mut best: Vec<(usize, usize)> = found
        .editions
        .iter()
        .enumerate()
        .filter(|(index, _)| found.verdicts[*index].fit != EditionFit::Different)
        .map(|(index, edition)| {
            let main = edition.title.split(':').next().unwrap_or(&edition.title);
            let mut named = usize::from(rules::opening_mentions(heard, main));
            named += usize::from(
                edition
                    .author
                    .as_deref()
                    .is_some_and(|author| rules::opening_mentions(heard, author)),
            );
            (index, named)
        })
        .filter(|(_, named)| *named >= 2)
        .collect();
    best.sort_by_key(|(_, named)| std::cmp::Reverse(*named));
    match best.as_slice() {
        [(only, _)] => Some(*only),
        // Two listings of one reading are the same book either way. Two readings aren't.
        [(first, _), rest @ ..] => {
            let reader = |index: usize| {
                found.editions[index]
                    .narrator
                    .as_deref()
                    .map(release_match::normalize_words)
            };
            rest.iter()
                .all(|(index, _)| reader(*index) == reader(*first))
                .then_some(*first)
        }
        [] => None,
    }
}

/// Ask Jev to choose among listings the rules can't separate. Lengths have already ruled out the rest.
async fn jev_edition(
    jev: &Jev,
    identity: &GroupIdentity,
    found: &BookListings,
) -> Option<(usize, f64)> {
    let options: Vec<usize> = rules::rank_editions(&found.editions, &found.verdicts)
        .into_iter()
        .filter(|index| found.verdicts[*index].plausible())
        .take(CHAPTER_LOOKUPS)
        .collect();
    if options.len() < 2 {
        return None;
    }
    let state = serde_json::json!({
        "files": {
            "titles": identity.titles,
            "authors": identity.authors,
            "narrators": identity.narrators,
            "seconds": identity.seconds,
            "file_count": identity.file_count,
        },
        "listings": options.iter().map(|index| {
            let edition = &found.editions[*index];
            serde_json::json!({
                "title": edition.title,
                "author": edition.author,
                "narrator": edition.narrator,
                "seconds": edition.seconds,
                "chapters": edition.chapters,
            })
        }).collect::<Vec<_>>(),
    });
    let questions: std::collections::BTreeMap<String, String> = (0..options.len())
        .map(|position| {
            (
                format!("listing_{position}"),
                format!("Is the listing at position {position} in listings the same recording as these files?"),
            )
        })
        .collect();
    let answers = match jev.yes_no(state, &questions).await {
        Ok(answers) => answers,
        Err(error) => {
            warn!(error = %error, "Jev could not help choose a listing");
            return None;
        }
    };
    let mut ranked: Vec<(usize, f64)> = answers
        .into_iter()
        .filter_map(|(name, sure)| {
            let position: usize = name.strip_prefix("listing_")?.parse().ok()?;
            Some((*options.get(position)?, sure))
        })
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    let best = *ranked.first()?;
    (best.1 >= JEV_ENOUGH && ranked.get(1).is_none_or(|next| best.1 - next.1 >= JEV_LEAD))
        .then_some(best)
}

fn summarize_items(items: &[ItemDraft]) -> String {
    let count = |status: &str| items.iter().filter(|item| item.status == status).count();
    let mut parts = Vec::new();
    for (status, word) in [
        ("imported", "imported"),
        ("wanted", "wanted"),
        ("in_library", "in your library"),
        ("identified", "ready to add"),
        ("needs_choice", "need a choice"),
        ("unknown", "not identified"),
    ] {
        let found = count(status);
        if found > 0 {
            parts.push(format!("{found} {word}"));
        }
    }
    if parts.is_empty() {
        "Nothing found".to_string()
    } else {
        parts.join(", ")
    }
}

fn normalize_album(album: &str) -> String {
    release_match::normalize_words(album)
}

/// JSON text for the columns that store lists.
mod erased_json {
    pub trait Json {
        fn to_json(&self) -> String;
    }
    impl<T: serde::Serialize> Json for T {
        fn to_json(&self) -> String {
            serde_json::to_string(self).unwrap_or_else(|_| "[]".to_string())
        }
    }
}

#[cfg(test)]
mod trigger_tests {
    use super::*;

    #[test]
    fn evidence_says_where_it_came_from() {
        let mut draft = ItemDraft::default();
        draft.note(Source::Listing, "Author matches");
        draft.warn(
            Source::Files,
            "2 files are named as damaged",
            Some("2 files damaged".to_string()),
        );
        assert_eq!(
            serde_json::to_value(&draft.evidence).unwrap(),
            serde_json::json!([
                { "source": "listing", "text": "Author matches", "warning": false },
                {
                    "source": "files",
                    "text": "2 files are named as damaged",
                    "warning": true,
                    "short": "2 files damaged"
                },
            ])
        );
        draft.note_reasons(
            Source::Listing,
            ["A different author: Mike Robb".to_string()],
        );
        assert!(draft.evidence[2].warning);
    }

    #[test]
    fn a_first_look_is_only_needed_once() {
        let first = settled_trigger(IdentifyTrigger::Observed, false, true);
        assert_eq!(first, Some(IdentifyTrigger::Observed));
        assert_eq!(settled_trigger(IdentifyTrigger::Observed, true, true), None);
    }

    #[test]
    fn a_restored_torrents_files_seeming_to_finish_is_not_news() {
        assert_eq!(
            settled_trigger(IdentifyTrigger::FileFinished, true, true),
            None
        );
    }

    #[test]
    fn a_first_look_replaced_while_queued_still_happens_and_cannot_import() {
        let trigger = settled_trigger(IdentifyTrigger::FileFinished, false, true);
        assert_eq!(trigger, Some(IdentifyTrigger::Observed));
    }

    #[test]
    fn a_file_finishing_in_a_running_download_is_looked_at() {
        assert_eq!(
            settled_trigger(IdentifyTrigger::FileFinished, true, false),
            Some(IdentifyTrigger::FileFinished)
        );
    }

    #[test]
    fn being_asked_or_finishing_always_runs() {
        for trigger in [IdentifyTrigger::Finished, IdentifyTrigger::Asked] {
            assert_eq!(settled_trigger(trigger, true, true), Some(trigger));
        }
    }
}
