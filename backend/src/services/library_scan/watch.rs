//! Watching library folders for changes, so new, changed and removed files are picked up within
//! seconds instead of at the next scheduled scan (`docs/design.md` Q115 to Q117).
//!
//! A library with *Watch the folder* (`Library.watchForChanges`) gets a watch on its root:
//! SMB2 CHANGE_NOTIFY for a share, inotify for a folder here (`library_storage::watch`). What the
//! watch reports is filtered (hidden folders such as `.librarian-trash`, temporary names, and
//! Librarian's own writes are ignored), gathered per path, and once a path has been quiet and its
//! size and time have stopped changing, it is scanned on its own: a scan of [`ScanScope::Changes`]
//! walks only those paths and reconciles only the records under them. A share that stops
//! answering is retried, and once it is back one whole-library scan catches up on whatever changed
//! meanwhile. The scheduled scan stays as the safety net.

use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::SystemTime;

use super::*;
use crate::services::library_storage::own_writes::is_own_write;
use crate::services::library_storage::{
    FolderWatch, StorageError, StorageResult, WatchChangeKind, WatchSignal,
};

/// How long a path must go without a new notification before it is looked at.
const QUIET: Duration = Duration::from_secs(3);
/// Two looks this far apart must agree (size, time, files in a folder) before a path is scanned,
/// so a large copy is never scanned half written.
const STABLE: Duration = Duration::from_secs(4);
/// How often pending changes are looked at.
const TICK: Duration = Duration::from_secs(1);
/// More paths than this changed at once: scan the whole library instead.
const MAX_TARGETS: usize = 50;
/// Files a folder's look counts before it settles for what it has seen.
const LOOK_LIMIT: usize = 20_000;
/// The first retry of a watch that failed, doubling to [`RETRY_MAX`].
const RETRY_MIN: Duration = Duration::from_secs(5);
const RETRY_MAX: Duration = Duration::from_secs(60);
/// How often the set of watched libraries is checked again without being nudged.
const RECONCILE_EVERY: Duration = Duration::from_secs(60);
/// A watch armed for the first time catches up with one whole-library scan when the library was
/// last scanned longer ago than this (the backend was down, or watching was just turned on).
const CATCH_UP_AFTER: chrono::Duration = chrono::Duration::minutes(10);

/// Wakes the watch supervisor when a library's watch setting or folder changes.
fn nudge() -> &'static tokio::sync::Notify {
    static NUDGE: OnceLock<tokio::sync::Notify> = OnceLock::new();
    NUDGE.get_or_init(tokio::sync::Notify::new)
}

/// Look at the watched libraries again now (a library was added, removed, moved, or its watch
/// setting changed).
pub fn nudge_library_watches() {
    nudge().notify_one();
}

/// The watch state shown with a library (`Library.watchStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WatchStatus {
    Starting,
    Watching,
    /// The share cannot be reached; the watch is retried.
    Unavailable,
    /// The folder cannot be watched; the watch is retried, more slowly.
    Failed,
}

impl WatchStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Watching => "watching",
            Self::Unavailable => "unavailable",
            Self::Failed => "failed",
        }
    }
}

/// Changes nobody should scan for: anything in a hidden folder or with a hidden name (the
/// library's `.librarian-trash`, `.DS_Store`, AppleDouble `._` files, rsync's `.name.XXXXXX`
/// temporaries), Librarian's own temporary copies, and the usual names of files still being
/// downloaded or copied.
pub(crate) fn ignored_change(relative: &str) -> bool {
    if relative.is_empty() {
        return true;
    }
    if relative.split('/').any(|part| part.starts_with('.')) {
        return true;
    }
    let name = relative.rsplit('/').next().unwrap_or(relative);
    let lower = name.to_ascii_lowercase();
    lower.contains(".librarian-tmp-")
        || [".part", ".partial", ".crdownload", ".!qb", ".tmp", "~"]
            .iter()
            .any(|suffix| lower.ends_with(suffix))
}

/// What a look at a path found, to tell whether it is still changing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Signature {
    Gone,
    File {
        size: u64,
        modified: Option<SystemTime>,
    },
    Folder {
        files: usize,
        bytes: u64,
        newest: Option<SystemTime>,
    },
}

#[derive(Debug)]
struct Pending {
    kinds: BTreeSet<ChangeKindKey>,
    last_event: Instant,
    look: Option<(Signature, Instant)>,
}

/// `WatchChangeKind` in a stable order, so a set of them can be kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ChangeKindKey {
    Modified,
    Structural,
}

impl From<WatchChangeKind> for ChangeKindKey {
    fn from(kind: WatchChangeKind) -> Self {
        match kind {
            WatchChangeKind::Modified => Self::Modified,
            _ => Self::Structural,
        }
    }
}

/// What a look decided about one pending path.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Settled {
    /// Unchanged since the last look: scan it.
    Ready,
    /// Still changing, or looked at for the first time: look again later.
    Wait,
    /// Only a folder's own times changed: what changed inside it is reported on its own.
    Drop,
}

/// Changes seen and not yet scanned, by path below the library root. A path below one already
/// pending joins it (a copy into a new folder is one scan of that folder); a folder whose
/// contents changed is noted separately and dropped if only its own times changed.
#[derive(Debug, Default)]
pub(crate) struct PendingChanges {
    entries: BTreeMap<String, Pending>,
}

impl PendingChanges {
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(crate) fn note(&mut self, path: &str, kind: WatchChangeKind, now: Instant) {
        let path = path.trim_matches('/').to_string();
        let key = ChangeKindKey::from(kind);
        // Already covered by a pending path at or above it: that one just waits longer.
        let covering = self
            .entries
            .iter_mut()
            .filter(|(pending, entry)| {
                path_is_at_or_under(&path, pending)
                    && (pending.as_str() == path
                        || entry.kinds.contains(&ChangeKindKey::Structural))
            })
            .max_by_key(|(pending, _)| pending.len());
        if let Some((_, entry)) = covering {
            entry.kinds.insert(key);
            entry.last_event = now;
            entry.look = None;
            return;
        }
        let mut kinds = BTreeSet::from([key]);
        if key == ChangeKindKey::Structural {
            // Something appeared, went or was renamed here: whatever is pending below it is
            // looked at as part of it.
            let below: Vec<String> = self
                .entries
                .keys()
                .filter(|pending| path_is_at_or_under(pending, &path))
                .cloned()
                .collect();
            for pending in below {
                if let Some(entry) = self.entries.remove(&pending) {
                    kinds.extend(entry.kinds);
                }
            }
        }
        self.entries.insert(
            path,
            Pending {
                kinds,
                last_event: now,
                look: None,
            },
        );
    }

    /// Paths quiet for [`QUIET`], ready for a look.
    pub(crate) fn due(&self, now: Instant) -> Vec<String> {
        self.entries
            .iter()
            .filter(|(_, entry)| now.duration_since(entry.last_event) >= QUIET)
            .filter(|(_, entry)| {
                entry
                    .look
                    .as_ref()
                    .is_none_or(|(_, at)| now.duration_since(*at) >= STABLE)
            })
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// Record a look at `path`.
    pub(crate) fn looked(&mut self, path: &str, signature: Signature, now: Instant) -> Settled {
        let Some(entry) = self.entries.get_mut(path) else {
            return Settled::Wait;
        };
        if matches!(signature, Signature::Folder { .. })
            && !entry.kinds.contains(&ChangeKindKey::Structural)
        {
            self.entries.remove(path);
            return Settled::Drop;
        }
        let settled = match &entry.look {
            Some((previous, _)) if *previous == signature => Settled::Ready,
            _ => Settled::Wait,
        };
        entry.look = Some((signature, now));
        settled
    }

    pub(crate) fn remove(&mut self, path: &str) {
        self.entries.remove(path);
    }
}

/// The smallest set of paths covering all of `paths` (a path inside another is dropped).
pub(crate) fn covering_paths(mut paths: Vec<String>) -> Vec<String> {
    paths.sort();
    paths.dedup();
    let mut kept: Vec<String> = Vec::new();
    for path in paths {
        if !kept.iter().any(|outer| path_is_at_or_under(&path, outer)) {
            kept.push(path);
        }
    }
    kept
}

/// The changed paths of a scan, for a person: below the library root, the first few named.
pub(crate) fn describe_change_targets(root: &str, targets: &[String]) -> String {
    let root = StoragePath::new(root);
    let names: Vec<String> = targets
        .iter()
        .map(|target| {
            StoragePath::new(target.as_str())
                .relative_to(&root)
                .filter(|relative| !relative.is_empty())
                .unwrap_or_else(|| target.clone())
        })
        .collect();
    match names.len() {
        0 => "nothing".to_string(),
        1..=3 => names.join(", "),
        more => format!("{} and {} more", names[..3].join(", "), more - 3),
    }
}

/// Look at a path: gone, a file's size and time, or a folder's file count, bytes and newest time.
async fn look_at(path: &StoragePath) -> StorageResult<Signature> {
    let meta = match library_storage().stat(path).await {
        Ok(meta) => meta,
        Err(error) if error.is_not_found() => return Ok(Signature::Gone),
        Err(error) => return Err(error),
    };
    if meta.kind != EntryKind::Directory {
        return Ok(Signature::File {
            size: meta.size,
            modified: meta.modified,
        });
    }
    let cancel = CancellationToken::new();
    let mut walk = library_storage()
        .walk(
            path,
            WalkOptions {
                prune: None,
                report: Some(Arc::new(|entry| entry.kind == EntryKind::File)),
                with_metadata: true,
            },
            cancel.clone(),
        )
        .await?;
    let (mut files, mut bytes, mut newest) = (0_usize, 0_u64, None::<SystemTime>);
    while let Some(entry) = walk.entries.recv().await {
        let entry = entry?;
        files += 1;
        if let Some(meta) = entry.meta {
            bytes = bytes.saturating_add(meta.size);
            newest = newest.max(meta.modified);
        }
        if files >= LOOK_LIMIT {
            cancel.cancel();
            break;
        }
    }
    drop(walk.entries);
    let _ = walk.task.await;
    Ok(Signature::Folder {
        files,
        bytes,
        newest,
    })
}

/// What the library last said about its watch, so it is written only when it changes.
struct ShownStatus<'a> {
    library_id: &'a str,
    last: Option<(WatchStatus, Option<String>)>,
}

impl<'a> ShownStatus<'a> {
    fn new(library_id: &'a str) -> Self {
        Self {
            library_id,
            last: None,
        }
    }

    async fn show(
        &mut self,
        scan: &LibraryScanService,
        status: WatchStatus,
        detail: Option<String>,
    ) {
        let next = Some((status, detail));
        if self.last != next {
            let detail = next.as_ref().and_then(|(_, detail)| detail.as_deref());
            scan.write_watch_status(self.library_id, Some(status), detail)
                .await;
            self.last = next;
        }
    }
}

/// A library being watched, as the supervisor knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WatchedLibrary {
    id: String,
    path: String,
}

#[derive(Debug, Clone)]
struct LibraryWatchRow {
    id: String,
    path: String,
    watch: bool,
    status: Option<String>,
}

struct RunningWatch {
    path: String,
    cancel: CancellationToken,
    handle: JoinHandle<()>,
}

impl LibraryScanService {
    /// Keeps one watch running per library that asks for one, and none for the others.
    pub(super) async fn run_watch_supervisor(self: Arc<Self>, cancel: CancellationToken) {
        let mut running: HashMap<String, RunningWatch> = HashMap::new();
        loop {
            match self.load_watch_rows().await {
                Ok(rows) => {
                    let wanted: HashMap<&str, &LibraryWatchRow> = rows
                        .iter()
                        .filter(|row| row.watch)
                        .map(|row| (row.id.as_str(), row))
                        .collect();
                    let stale: Vec<String> = running
                        .iter()
                        .filter(|(id, watch)| {
                            wanted
                                .get(id.as_str())
                                .is_none_or(|row| row.path != watch.path)
                        })
                        .map(|(id, _)| id.clone())
                        .collect();
                    for id in stale {
                        if let Some(watch) = running.remove(&id) {
                            watch.cancel.cancel();
                            let _ = watch.handle.await;
                        }
                    }
                    for row in &rows {
                        if row.watch && !running.contains_key(&row.id) {
                            let watch_cancel = cancel.child_token();
                            let library = WatchedLibrary {
                                id: row.id.clone(),
                                path: row.path.clone(),
                            };
                            let this = self.clone();
                            let handle =
                                tokio::spawn(this.watch_library(library, watch_cancel.clone()));
                            running.insert(
                                row.id.clone(),
                                RunningWatch {
                                    path: row.path.clone(),
                                    cancel: watch_cancel,
                                    handle,
                                },
                            );
                        } else if !row.watch && row.status.is_some() {
                            // Watching was turned off: say nothing about a watch that is gone.
                            self.write_watch_status(&row.id, None, None).await;
                        }
                    }
                }
                Err(error) => {
                    debug!(error = %error, "Could not list the libraries to watch: {error:#}");
                }
            }
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = tokio::time::sleep(RECONCILE_EVERY) => {}
                _ = nudge().notified() => {}
            }
        }
        for (_, watch) in running.drain() {
            watch.cancel.cancel();
            let _ = watch.handle.await;
        }
    }

    async fn load_watch_rows(&self) -> Result<Vec<LibraryWatchRow>> {
        let Some(auth_user) = self.try_system_auth_user(None).await? else {
            return Ok(Vec::new());
        };
        let data = self
            .execute_graphql_paged(
                &auth_user,
                r#"query WatchedLibraries($page: PageInput) {
                    Libraries: libraries(page: $page) {
                        Edges: edges {
                            Node: node {
                                Id: id
                                Path: path
                                WatchForChanges: watchForChanges
                                WatchStatus: watchStatus
                            }
                        }
                    }
                }"#,
                serde_json::json!({}),
                "Libraries",
            )
            .await?;
        Ok(data
            .get("Libraries")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|edge| {
                let node = edge.get("Node")?;
                Some(LibraryWatchRow {
                    id: node.get("Id")?.as_str()?.to_string(),
                    path: node.get("Path")?.as_str()?.to_string(),
                    watch: node
                        .get("WatchForChanges")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    status: node
                        .get("WatchStatus")
                        .and_then(|v| v.as_str())
                        .map(str::to_string),
                })
            })
            .collect())
    }

    /// Say on the library what its watch is doing. A failure to say so is only logged: the watch
    /// itself goes on.
    async fn write_watch_status(
        &self,
        library_id: &str,
        status: Option<WatchStatus>,
        detail: Option<&str>,
    ) {
        let input = serde_json::json!({
            "watchStatus": status.map(WatchStatus::as_str),
            "watchDetail": detail,
            "watchStatusSince": status.map(|_| Utc::now().to_rfc3339()),
        });
        let outcome = async {
            let auth_user = AuthUser::system_admin();
            let data = self
                .execute_mutation(
                    &auth_user,
                    r#"mutation UpdateLibraryWatchStatus($id: String!, $input: UpdateLibraryInput!) {
                        UpdateLibrary: updateLibrary(id: $id, input: $input) { Success: success Error: error }
                    }"#,
                    serde_json::json!({ "id": library_id, "input": input }),
                )
                .await?;
            check_mutation_payloads(&data)
        }
        .await;
        if let Err(error) = outcome {
            warn!(
                library_id,
                error = %error,
                "Could not record the folder watch state of library {library_id}: {error:#}"
            );
        }
    }

    /// One library's watch, for as long as it is wanted.
    async fn watch_library(self: Arc<Self>, library: WatchedLibrary, cancel: CancellationToken) {
        let root = StoragePath::new(library.path.as_str());
        let mut shown = ShownStatus::new(&library.id);
        let mut pending = PendingChanges::default();
        // Something may have changed that no notification will tell: scan the whole library.
        let mut catch_up = self.watch_needs_catch_up(&library.id).await;
        let mut retry = RETRY_MIN;
        info!(
            library_id = %library.id,
            path = %library.path,
            "Watching library folder {} for changes",
            library.path
        );
        // "Starting" only the first time: a share that is down stays "unavailable" while it is
        // retried, rather than flickering between the two.
        shown.show(&self, WatchStatus::Starting, None).await;
        loop {
            let mut watch = match library_storage().watch(&root).await {
                Ok(watch) => watch,
                Err(error) => {
                    let unavailable = matches!(
                        error.kind(),
                        StorageErrorKind::Unavailable | StorageErrorKind::NotFound
                    );
                    warn!(
                        library_id = %library.id,
                        path = %library.path,
                        error = %error,
                        "Could not watch library folder {} (retrying in {}s): {error}",
                        library.path,
                        retry.as_secs()
                    );
                    if unavailable {
                        // When it answers again, changes made meanwhile were never told.
                        catch_up = true;
                        shown
                            .show(
                                &self,
                                WatchStatus::Unavailable,
                                Some(format!("Not reachable, retrying: {error}")),
                            )
                            .await;
                    } else {
                        shown
                            .show(&self, WatchStatus::Failed, Some(error.to_string()))
                            .await;
                    }
                    tokio::select! {
                        _ = cancel.cancelled() => return,
                        _ = tokio::time::sleep(retry) => {}
                    }
                    retry = (retry * 2).min(RETRY_MAX);
                    continue;
                }
            };
            retry = RETRY_MIN;
            shown.show(&self, WatchStatus::Watching, None).await;
            info!(
                library_id = %library.id,
                path = %library.path,
                catch_up,
                "Library folder watch armed: {}",
                library.path
            );
            let lost = self
                .follow_watch(
                    &library,
                    &root,
                    &mut watch,
                    &mut pending,
                    &mut catch_up,
                    &cancel,
                )
                .await;
            drop(watch);
            let Some(error) = lost else {
                return;
            };
            warn!(
                library_id = %library.id,
                path = %library.path,
                error = %error,
                "Library folder watch stopped, re-arming: {}: {error}",
                library.path
            );
            catch_up = true;
            shown
                .show(
                    &self,
                    WatchStatus::Unavailable,
                    Some(format!("Not reachable, retrying: {error}")),
                )
                .await;
            tokio::select! {
                _ = cancel.cancelled() => return,
                _ = tokio::time::sleep(RETRY_MIN) => {}
            }
        }
    }

    /// Whether a watch just armed should start with a whole-library scan: the library has not had
    /// one recently, so changes from before the watch may be unseen.
    async fn watch_needs_catch_up(&self, library_id: &str) -> bool {
        let Ok(database) = self.database().await else {
            return false;
        };
        let Ok(Some(library)) =
            crate::graphql::entities::Library::get(database.pool().pool(), &library_id.to_string())
                .await
        else {
            return false;
        };
        library
            .last_scanned_at
            .as_deref()
            .and_then(|at| chrono::DateTime::parse_from_rfc3339(at).ok())
            .is_none_or(|at| Utc::now().signed_duration_since(at) > CATCH_UP_AFTER)
    }

    /// Take notifications until the watch is lost (`Some(why)`) or no longer wanted (`None`).
    async fn follow_watch(
        &self,
        library: &WatchedLibrary,
        root: &StoragePath,
        watch: &mut FolderWatch,
        pending: &mut PendingChanges,
        catch_up: &mut bool,
        cancel: &CancellationToken,
    ) -> Option<StorageError> {
        let mut tick = tokio::time::interval(TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = cancel.cancelled() => return None,
                signal = watch.next() => match signal {
                    Some(WatchSignal::Changed(changes)) => {
                        let now = Instant::now();
                        for change in changes {
                            if ignored_change(&change.path)
                                || is_own_write(&root.join_relative(&change.path))
                            {
                                continue;
                            }
                            debug!(
                                library_id = %library.id,
                                path = %change.path,
                                kind = ?change.kind,
                                "Library folder change noticed: {} ({:?})",
                                change.path,
                                change.kind
                            );
                            pending.note(&change.path, change.kind, now);
                        }
                        if pending.len() > MAX_TARGETS {
                            *catch_up = true;
                        }
                    }
                    Some(WatchSignal::Overflow) => {
                        info!(
                            library_id = %library.id,
                            "Too many changes in {} to tell one by one: scanning the whole library",
                            library.path
                        );
                        *catch_up = true;
                    }
                    Some(WatchSignal::Lost(error)) => return Some(error),
                    None => {
                        return Some(StorageError::new(
                            StorageErrorKind::Unavailable,
                            "the watch ended",
                        ));
                    }
                },
                _ = tick.tick() => {
                    if *catch_up {
                        match self.queue_scan(&library.id).await {
                            Ok(QueueScanResult { queued: true, .. }) => {
                                *catch_up = false;
                                pending.clear();
                            }
                            // A scan is running; it may have walked past the change, so wait
                            // for it to end and queue another.
                            Ok(_) => {}
                            Err(error) => debug!(
                                library_id = %library.id,
                                error = %error,
                                "Could not queue a catch-up scan of {} yet: {error:#}",
                                library.path
                            ),
                        }
                    } else if !pending.is_empty() {
                        self.scan_settled_changes(library, root, pending).await;
                    }
                }
            }
        }
    }

    /// Look at the pending paths that have gone quiet, and scan those whose size and time held.
    async fn scan_settled_changes(
        &self,
        library: &WatchedLibrary,
        root: &StoragePath,
        pending: &mut PendingChanges,
    ) {
        let mut ready = Vec::new();
        for relative in pending.due(Instant::now()) {
            match look_at(&root.join_relative(&relative)).await {
                Ok(signature) => {
                    if pending.looked(&relative, signature, Instant::now()) == Settled::Ready {
                        ready.push(relative);
                    }
                }
                // The share is not answering: the watch will say so too; keep the change.
                Err(error) => debug!(
                    library_id = %library.id,
                    path = %relative,
                    error = %error,
                    "Could not look at the changed path '{relative}' in {} yet: {error}",
                    library.path
                ),
            }
        }
        if ready.is_empty() {
            return;
        }
        let targets: Vec<String> = covering_paths(ready.clone())
            .into_iter()
            .map(|relative| root.join_relative(&relative).into_string())
            .collect();
        match self.queue_change_scan(&library.id, targets).await {
            Ok(_) => {
                for relative in ready {
                    pending.remove(&relative);
                }
            }
            Err(error) => warn!(
                library_id = %library.id,
                error = %error,
                "Could not queue a scan of changed files in {}: {error:#}",
                library.path
            ),
        }
    }

    /// Queue a scan of changed paths (absolute, under the library's root). Paths that cannot hold
    /// media for this library (a changed `.nfo`, a gone file nobody catalogued) are dropped first,
    /// and if none is left nothing is queued.
    pub(crate) async fn queue_change_scan(
        &self,
        library_id: &str,
        targets: Vec<String>,
    ) -> Result<QueueScanResult> {
        let library = self.get_library(library_id).await?;
        let allowed =
            Self::extensions_for_library(&Self::normalize_library_type(&library.library_type));
        let records = self.load_media_files_by_path(library_id).await?;
        let mut relevant = Vec::new();
        for target in covering_paths(targets) {
            let catalogued = records
                .keys()
                .any(|path| path_is_at_or_under(path, &target));
            let keep = match library_storage().stat(&target).await {
                Ok(meta) if meta.kind == EntryKind::Directory => true,
                Ok(_) => catalogued || is_reported_media_name(&target, allowed),
                Err(error) if error.is_not_found() => catalogued,
                // Let the scan say what is wrong.
                Err(_) => true,
            };
            if keep {
                relevant.push(target);
            }
        }
        if relevant.is_empty() {
            return Ok(QueueScanResult {
                queued: false,
                scan_run_id: None,
            });
        }
        info!(
            library_id,
            library_name = %library.name,
            changed = relevant.len(),
            "Queueing a scan of changes in {}: {}",
            library.name,
            describe_change_targets(&library.path, &relevant)
        );
        self.queue_scan_with(library_id, ScanScope::Changes(relevant))
            .await
    }

    /// [`discover_media_files`](Self::discover_media_files) for a scan of changed paths: each
    /// target is walked if it is a folder and reported if it is a media file; one that is gone
    /// yields nothing (its records are then checked one by one). Paths stay relative to the
    /// library root.
    pub(super) fn discover_changed_media_files(
        root: PathBuf,
        targets: Vec<String>,
        allowed_extensions: &'static [&'static str],
        cancel: CancellationToken,
    ) -> (
        mpsc::Receiver<std::result::Result<DiscoveredMediaFile, DiscoveryError>>,
        JoinHandle<()>,
    ) {
        let (tx, rx) = mpsc::channel(128);
        let handle = tokio::spawn(async move {
            let root = root.into_storage_path();
            let failed = |error: StorageError| DiscoveryError {
                message: format!("Unable to read library entry: {error}"),
                unavailable: error.kind() == StorageErrorKind::Unavailable,
            };
            for target in targets {
                if cancel.is_cancelled() {
                    return;
                }
                let target = StoragePath::new(target);
                let meta = match library_storage().stat(&target).await {
                    Ok(meta) => meta,
                    Err(error) if error.is_not_found() => continue,
                    Err(error) => {
                        if tx.send(Err(failed(error))).await.is_err() {
                            return;
                        }
                        continue;
                    }
                };
                if meta.kind != EntryKind::Directory {
                    if meta.kind == EntryKind::File
                        && is_reported_media_name(target.as_str(), allowed_extensions)
                        && tx
                            .send(Ok(discovered_media_file(target, Some(meta), &root)))
                            .await
                            .is_err()
                    {
                        return;
                    }
                    continue;
                }
                let report: WalkFilter = Arc::new(move |entry| {
                    entry.kind == EntryKind::File
                        && is_reported_media_name(entry.name, allowed_extensions)
                });
                let walk = library_storage()
                    .walk(
                        &target,
                        WalkOptions {
                            prune: None,
                            report: Some(report),
                            with_metadata: true,
                        },
                        cancel.clone(),
                    )
                    .await;
                let mut walk = match walk {
                    Ok(walk) => walk,
                    Err(error) => {
                        if tx.send(Err(failed(error))).await.is_err() {
                            return;
                        }
                        continue;
                    }
                };
                while let Some(result) = walk.entries.recv().await {
                    let sent = match result {
                        Ok(entry) => {
                            tx.send(Ok(discovered_media_file(entry.path, entry.meta, &root)))
                                .await
                        }
                        Err(error) => tx.send(Err(failed(error))).await,
                    };
                    if sent.is_err() {
                        break;
                    }
                }
                drop(walk.entries);
                let _ = walk.task.await;
            }
        });
        (rx, handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(start: Instant, secs: u64) -> Instant {
        start + Duration::from_secs(secs)
    }

    #[test]
    fn hidden_temporary_and_partial_names_are_ignored() {
        for ignored in [
            ".librarian-trash/x.mkv",
            "Show/.librarian-trash/x.mkv",
            "Film (2020)/.Film.mkv.Ab12Cd",
            "Film (2020)/._Film.mkv",
            ".DS_Store",
            "Film (2020)/Film.mkv.librarian-tmp-0f9a",
            "Film (2020)/Film.mkv.part",
            "Film (2020)/Film.mkv.crdownload",
            "",
        ] {
            assert!(ignored_change(ignored), "{ignored} should be ignored");
        }
        for seen in ["Film (2020)/Film.mkv", "Show/Season 1", "Film.2020.1080p"] {
            assert!(!ignored_change(seen), "{seen} should be seen");
        }
    }

    #[test]
    fn a_copy_into_a_new_folder_is_one_target() {
        let start = Instant::now();
        let mut pending = PendingChanges::default();
        pending.note("Film (2020)", WatchChangeKind::Added, start);
        pending.note("Film (2020)/Film.mkv", WatchChangeKind::Added, start);
        pending.note(
            "Film (2020)/Film.mkv",
            WatchChangeKind::Modified,
            at(start, 1),
        );
        assert_eq!(pending.len(), 1);
        // Quiet since the last write only after `QUIET`.
        assert!(pending.due(at(start, 2)).is_empty());
        assert_eq!(pending.due(at(start, 4)), vec!["Film (2020)".to_string()]);
    }

    #[test]
    fn a_new_folder_takes_in_what_was_pending_below_it() {
        let start = Instant::now();
        let mut pending = PendingChanges::default();
        pending.note("Show/Season 1/e1.mkv", WatchChangeKind::Added, start);
        pending.note("Show/Season 1", WatchChangeKind::Modified, start);
        assert_eq!(
            pending.len(),
            2,
            "a folder's own times stay apart from its files"
        );
        pending.note("Show", WatchChangeKind::RenamedTo, start);
        assert_eq!(pending.due(at(start, 5)), vec!["Show".to_string()]);
    }

    #[test]
    fn a_path_is_ready_only_when_two_looks_agree() {
        let start = Instant::now();
        let mut pending = PendingChanges::default();
        pending.note("Film.mkv", WatchChangeKind::Added, start);
        let growing = |size| Signature::File {
            size,
            modified: None,
        };
        assert_eq!(
            pending.looked("Film.mkv", growing(10), at(start, 3)),
            Settled::Wait
        );
        // Not looked at again before `STABLE`.
        assert!(pending.due(at(start, 4)).is_empty());
        assert_eq!(pending.due(at(start, 7)), vec!["Film.mkv".to_string()]);
        assert_eq!(
            pending.looked("Film.mkv", growing(20), at(start, 7)),
            Settled::Wait
        );
        assert_eq!(
            pending.looked("Film.mkv", growing(20), at(start, 11)),
            Settled::Ready
        );
        // A notification in between starts the looks over.
        pending.note("Film.mkv", WatchChangeKind::Modified, at(start, 12));
        assert_eq!(
            pending.looked("Film.mkv", growing(20), at(start, 15)),
            Settled::Wait
        );
    }

    #[test]
    fn a_folder_whose_own_times_changed_is_dropped() {
        let start = Instant::now();
        let mut pending = PendingChanges::default();
        pending.note("Show/Season 1", WatchChangeKind::Modified, start);
        let folder = Signature::Folder {
            files: 3,
            bytes: 30,
            newest: None,
        };
        assert_eq!(
            pending.looked("Show/Season 1", folder.clone(), at(start, 3)),
            Settled::Drop
        );
        assert!(pending.is_empty());
        pending.note("Show/Season 2", WatchChangeKind::Added, start);
        assert_eq!(
            pending.looked("Show/Season 2", folder, at(start, 3)),
            Settled::Wait
        );
    }

    #[test]
    fn covering_paths_keeps_the_outermost() {
        assert_eq!(
            covering_paths(vec![
                "Show/Season 1/e1.mkv".into(),
                "Show".into(),
                "Show 2".into(),
                "Show".into(),
            ]),
            vec!["Show".to_string(), "Show 2".to_string()]
        );
    }

    #[test]
    fn scopes_cover_paths_at_or_under_their_targets() {
        let scope = ScanScope::Changes(vec!["smb://h/s/TV/Show".into()]);
        assert!(scope.covers("smb://h/s/TV/Show"));
        assert!(scope.covers("smb://h/s/TV/Show/Season 1/e.mkv"));
        assert!(!scope.covers("smb://h/s/TV/Show 2/e.mkv"));
        assert!(ScanScope::Full.covers("anything"));
        assert_eq!(
            describe_change_targets(
                "smb://h/s/TV",
                &["smb://h/s/TV/A".into(), "smb://h/s/TV/B/b.mkv".into()]
            ),
            "A, B/b.mkv"
        );
    }
}
