//! Moving a library: changing the root of a library whose files were copied somewhere else,
//! without re-matching anything.
//!
//! The person copies the folder first (to another disk, or onto a network share), keeping file
//! times. Librarian then rewrites its records from the old root to the new one, so every media
//! file keeps its id, its links to movies, episodes and tracks, its progress and its playlists, and
//! the first scan afterwards finds every file where it expects it. See `docs/design.md` Q85.
//!
//! 1. A **plan** (read only) walks the new root once and says, for every file the library holds,
//!    whether it is there with the same size, and which media files on the new root the library
//!    does not know.
//! 2. **Apply** marks the library as moving (scans, organizing and imports leave it alone while
//!    the mark is there), rewrites the records in chunks through the generated mutations, and
//!    changes the library's own path last. Every record is rewritten from the part of its path
//!    below the root, so a move that stopped is finished by running it again; the mark is also
//!    resumed when the server starts.

use std::collections::{HashMap, HashSet};

use super::*;
use crate::services::graphql::entities::{
    Album, AlbumWhereInput, AppSetting, AppSettingWhereInput, Audiobook, AudiobookWhereInput,
    CreateAppSettingInput, Show, ShowWhereInput, Subtitle, SubtitleWhereInput,
};
use crate::services::library_storage::{StorageLocation, StoragePath};

/// Prefix of the app-setting key that marks a library as being moved.
const MOVE_MARKER_PREFIX: &str = "library_move.";
/// Rows rewritten per GraphQL request.
const REWRITE_BATCH: usize = 50;
/// Missing and extra files named in a plan.
const NAMED_FILES: usize = 50;

/// What a plan found. Nothing has been changed.
#[derive(Debug, Clone, Default)]
pub struct LibraryMovePlan {
    /// The library's current root.
    pub old_path: String,
    /// The new root, normalised (`smb://host/share/folder` in lower case host and share).
    pub new_path: String,
    /// Files the library holds.
    pub file_count: usize,
    /// Present on the new root with the same size.
    pub matched: usize,
    /// Not on the new root at all.
    pub missing: usize,
    pub missing_files: Vec<String>,
    /// On the new root, but not the size the library has (a copy that did not finish).
    pub changed: usize,
    pub changed_files: Vec<String>,
    /// Media files on the new root the library does not hold. They are found by the next scan.
    pub extra: usize,
    pub extra_files: Vec<String>,
    /// Why the move cannot go ahead at all.
    pub blocked: Option<String>,
}

impl LibraryMovePlan {
    /// Whether every file is on the new root at its size.
    pub fn is_complete(&self) -> bool {
        self.blocked.is_none() && self.missing == 0 && self.changed == 0
    }
}

#[derive(Debug, Clone, Default)]
pub struct LibraryMoveOutcome {
    pub moved_files: usize,
    pub skipped_files: usize,
    pub moved_folders: usize,
    pub closed_issues: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct MoveMarker {
    new_path: String,
    old_path: String,
    started_at: String,
}

/// One file's records, ready to be rewritten.
struct FileMove {
    id: String,
    new_path: String,
    relative_path: String,
    size: Option<i64>,
    modified_at: Option<String>,
}

struct PreparedMove {
    plan: LibraryMovePlan,
    files: Vec<FileMove>,
    old_root: StoragePath,
    new_root: StoragePath,
}

/// Every row of a query, a page at a time (a page larger than the ORM's maximum silently loses
/// the rest).
macro_rules! all_rows {
    ($query:expr) => {{
        let mut rows = Vec::new();
        let mut offset = 0_i64;
        loop {
            let page = $query
                .limit(LibraryScanService::ENTITY_PAGE_SIZE)
                .offset(offset)
                .fetch_all()
                .await?;
            let count = page.len();
            rows.extend(page);
            if (count as i64) < LibraryScanService::ENTITY_PAGE_SIZE {
                break;
            }
            offset += LibraryScanService::ENTITY_PAGE_SIZE;
        }
        rows
    }};
}

/// Rows to rewrite through an update mutation: the id and the input for it.
type RewriteRows = Vec<(String, serde_json::Value)>;

fn marker_key(library_id: &str) -> String {
    format!("{MOVE_MARKER_PREFIX}{library_id}")
}

fn graphql_string(value: &str) -> serde_json::Value {
    serde_json::Value::String(value.to_string())
}

impl LibraryScanService {
    /// The move that is under way for a library, if one is.
    pub(super) async fn library_move_in_progress(
        &self,
        library_id: &str,
    ) -> Result<Option<MoveMarkerInfo>> {
        // Without a database there is no mark to find (and nothing else works either).
        let Ok(database) = self.database().await else {
            return Ok(None);
        };
        let rows = AppSetting::query(database.pool().pool())
            .filter(AppSettingWhereInput {
                key: Some(StringFilter {
                    eq: Some(marker_key(library_id)),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .limit(1)
            .fetch_all()
            .await?;
        Ok(rows
            .into_iter()
            .next()
            .and_then(|row| serde_json::from_str::<MoveMarker>(&row.value).ok())
            .map(|marker| MoveMarkerInfo {
                new_path: marker.new_path,
            }))
    }

    /// Fail while a library is being moved: nothing may place, rename or catalogue files in it.
    pub(super) async fn ensure_library_not_moving(&self, library_id: &str) -> Result<()> {
        if let Some(moving) = self.library_move_in_progress(library_id).await? {
            anyhow::bail!(
                "This library is being moved to '{}'; scanning and organizing resume when the move finishes",
                moving.new_path
            );
        }
        Ok(())
    }

    /// Compare a library's records with a folder its files were copied to. Changes nothing.
    pub async fn plan_library_move(
        &self,
        library_id: &str,
        new_path: &str,
    ) -> Result<LibraryMovePlan> {
        Ok(self.prepare_move(library_id, new_path).await?.plan)
    }

    async fn prepare_move(&self, library_id: &str, new_path: &str) -> Result<PreparedMove> {
        let library = self.get_library(library_id).await?;
        let mut plan = LibraryMovePlan {
            old_path: library.path.clone(),
            ..Default::default()
        };
        let location =
            StorageLocation::parse(new_path).map_err(|error| anyhow::anyhow!("{error}"))?;
        plan.new_path = location.to_path_string();
        let old_root = StoragePath::new(library.path.clone());
        let new_root = location.root_path();
        let prepared = |plan: LibraryMovePlan| PreparedMove {
            plan,
            files: Vec::new(),
            old_root: old_root.clone(),
            new_root: new_root.clone(),
        };
        if old_root.kind() == new_root.kind()
            && (new_root.starts_with(&old_root) || old_root.starts_with(&new_root))
        {
            plan.blocked = Some(if new_root == old_root {
                "That is already this library's folder".to_string()
            } else {
                "The new folder cannot be inside the old one, or the old one inside it".to_string()
            });
            return Ok(prepared(plan));
        }
        let storage = library_storage();
        match storage.stat(&new_root).await {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => {
                plan.blocked = Some("The new location is not a folder".to_string());
                return Ok(prepared(plan));
            }
            Err(error) => {
                plan.blocked = Some(format!("The new folder cannot be read: {error}"));
                return Ok(prepared(plan));
            }
        }

        // What the library holds, keyed by the part of each path below the old root.
        let database = self.database().await?;
        let mut held: Vec<(String, String, String, i64)> = Vec::new(); // id, path, relative, size
        let mut outside = 0_usize;
        let files = all_rows!(MediaFile::query(database.pool().pool()).filter(
            MediaFileWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library_id.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            }
        ));
        for file in files {
            let current = StoragePath::new(file.path.clone());
            // A record already under the new root was moved by an earlier, stopped attempt.
            let relative = current
                .relative_to(&old_root)
                .or_else(|| current.relative_to(&new_root));
            match relative {
                Some(relative) if !relative.is_empty() => {
                    held.push((file.id, file.path, relative, file.size));
                }
                _ => outside += 1,
            }
        }
        plan.file_count = held.len() + outside;
        if outside > 0 {
            plan.blocked = Some(format!(
                "{outside} of the library's files are not under its folder, so they cannot be moved with it"
            ));
        }

        // What is on the new root, below it.
        let allowed =
            Self::extensions_for_library(&Self::normalize_library_type(&library.library_type));
        let prune: WalkFilter =
            Arc::new(|entry| entry.depth == 1 && entry.name == LIBRARY_TRASH_DIR);
        let report: WalkFilter = Arc::new(|entry| entry.kind == EntryKind::File);
        let mut walk = storage
            .walk(
                &new_root,
                WalkOptions {
                    prune: Some(prune),
                    report: Some(report),
                    with_metadata: true,
                },
                CancellationToken::new(),
            )
            .await?;
        let mut there: HashMap<String, (u64, Option<String>)> = HashMap::new();
        let mut walk_error: Option<String> = None;
        while let Some(entry) = walk.entries.recv().await {
            match entry {
                Ok(entry) => {
                    let Some(relative) = entry.path.relative_to(&new_root) else {
                        continue;
                    };
                    let (size, modified) = entry.meta.map_or((0, None), |meta| {
                        (
                            meta.size,
                            meta.modified.map(|time| {
                                DateTime::<Utc>::from(time)
                                    .to_rfc3339_opts(SecondsFormat::Nanos, true)
                            }),
                        )
                    });
                    there.insert(relative, (size, modified));
                }
                Err(error) => walk_error = Some(error.to_string()),
            }
        }
        let _ = walk.task.await;
        if let Some(error) = walk_error {
            plan.blocked = Some(format!(
                "The new folder could not be read completely, so its files cannot be compared: {error}"
            ));
            return Ok(prepared(plan));
        }

        let mut files = Vec::with_capacity(held.len());
        let mut known: HashSet<&str> = HashSet::new();
        for (id, _, relative, size) in &held {
            known.insert(relative.as_str());
            match there.get(relative) {
                None => {
                    plan.missing += 1;
                    if plan.missing_files.len() < NAMED_FILES {
                        plan.missing_files.push(relative.clone());
                    }
                }
                Some((there_size, _)) if *there_size != *size as u64 => {
                    plan.changed += 1;
                    if plan.changed_files.len() < NAMED_FILES {
                        plan.changed_files.push(relative.clone());
                    }
                }
                Some((there_size, modified)) => {
                    plan.matched += 1;
                    files.push(FileMove {
                        id: id.clone(),
                        new_path: new_root.join_relative(relative).into_string(),
                        relative_path: relative.clone(),
                        size: Some(*there_size as i64),
                        modified_at: modified.clone(),
                    });
                }
            }
        }
        for relative in there.keys() {
            let is_media = Path::new(relative)
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .is_some_and(|extension| allowed.iter().any(|allowed| *allowed == extension));
            let hidden = relative.split('/').any(|part| part.starts_with('.'));
            if is_media && !hidden && !known.contains(relative.as_str()) {
                plan.extra += 1;
                if plan.extra_files.len() < NAMED_FILES {
                    plan.extra_files.push(relative.clone());
                }
            }
        }
        plan.missing_files.sort();
        plan.changed_files.sort();
        plan.extra_files.sort();
        Ok(PreparedMove {
            plan,
            files,
            old_root,
            new_root,
        })
    }

    /// Run the batches of `mutations` (`(field, id, input)` for each row) through the generated
    /// update mutations.
    async fn rewrite_rows(
        &self,
        auth_user: &AuthUser,
        mutation_name: &str,
        input_type: &str,
        rows: RewriteRows,
    ) -> Result<()> {
        for batch in rows.chunks(REWRITE_BATCH) {
            let mut variables = serde_json::Map::new();
            let mut declarations = Vec::new();
            let mut fields = Vec::new();
            for (index, (id, input)) in batch.iter().enumerate() {
                declarations.push(format!("$id{index}: String!, $in{index}: {input_type}!"));
                fields.push(format!(
                    "m{index}: {mutation_name}(id: $id{index}, input: $in{index}) {{ success error }}"
                ));
                variables.insert(format!("id{index}"), graphql_string(id));
                variables.insert(format!("in{index}"), input.clone());
            }
            let document = format!(
                "mutation MoveLibraryRows({}) {{ {} }}",
                declarations.join(", "),
                fields.join(" ")
            );
            self.execute_mutation(auth_user, &document, serde_json::Value::Object(variables))
                .await
                .with_context(|| format!("Failed to rewrite {} records", mutation_name))?;
        }
        Ok(())
    }

    /// Move a library's records to a folder its files were copied to.
    pub async fn apply_library_move(
        &self,
        library_id: &str,
        new_path: &str,
        allow_missing: bool,
    ) -> Result<LibraryMoveOutcome> {
        let library = self.get_library(library_id).await?;
        if library.scanning {
            anyhow::bail!("A scan of this library is running; wait for it to finish, then move it");
        }
        let location =
            StorageLocation::parse(new_path).map_err(|error| anyhow::anyhow!("{error}"))?;
        let target = location.to_path_string();
        if let Some(moving) = self.library_move_in_progress(library_id).await?
            && moving.new_path != target
        {
            anyhow::bail!(
                "This library is already being moved to '{}'; let that finish first",
                moving.new_path
            );
        }
        let prepared = self.prepare_move(library_id, &target).await?;
        if let Some(reason) = &prepared.plan.blocked {
            anyhow::bail!("{reason}");
        }
        if !prepared.plan.is_complete() && !allow_missing {
            anyhow::bail!(
                "{} files are missing from the new folder and {} have a different size; copy them first",
                prepared.plan.missing,
                prepared.plan.changed
            );
        }

        self.write_move_marker(library_id, &library.path, &target)
            .await?;
        let auth_user = self.system_auth_user(Some(&library.user_id)).await?;
        info!(
            library_id = %library_id,
            library_name = %library.name,
            old_path = %library.path,
            new_path = %target,
            files = prepared.files.len(),
            "Moving library '{}' from {} to {}: {} files",
            library.name,
            library.path,
            target,
            prepared.files.len()
        );
        let outcome = self
            .rewrite_library_records(&library, &auth_user, &prepared, &target)
            .await;
        match outcome {
            Ok(outcome) => {
                self.clear_move_marker(library_id).await?;
                info!(
                    library_id = %library_id,
                    library_name = %library.name,
                    new_path = %target,
                    moved_files = outcome.moved_files,
                    "Moved library '{}' to {}: {} files, {} folders",
                    library.name,
                    target,
                    outcome.moved_files,
                    outcome.moved_folders
                );
                Ok(outcome)
            }
            Err(error) => {
                // The mark stays: the records are partly rewritten, and only finishing the move
                // (run it again) makes them agree.
                warn!(
                    library_id = %library_id,
                    library_name = %library.name,
                    new_path = %target,
                    error = %error,
                    "Moving library '{}' stopped part way; run the move again to finish it: {}",
                    library.name,
                    error
                );
                Err(error)
            }
        }
    }

    async fn rewrite_library_records(
        &self,
        library: &LibraryRow,
        auth_user: &AuthUser,
        prepared: &PreparedMove,
        target: &str,
    ) -> Result<LibraryMoveOutcome> {
        let mut outcome = LibraryMoveOutcome::default();
        let database = self.database().await?;

        // Files.
        let rows = prepared
            .files
            .iter()
            .map(|file| {
                let mut input = serde_json::json!({
                    "path": file.new_path,
                    "relativePath": file.relative_path,
                });
                if let Some(size) = file.size {
                    input["size"] = size.into();
                }
                if let Some(modified) = &file.modified_at {
                    input["fileModifiedAt"] = modified.clone().into();
                }
                (file.id.clone(), input)
            })
            .collect::<Vec<_>>();
        outcome.moved_files = rows.len();
        self.rewrite_rows(auth_user, "updateMediaFile", "UpdateMediaFileInput", rows)
            .await?;

        // Folders the library's titles keep, when they are under the old root.
        let moved = |path: &Option<String>| -> Option<String> {
            let current = StoragePath::new(path.as_deref()?);
            let relative = current.relative_to(&prepared.old_root)?;
            Some(prepared.new_root.join_relative(&relative).into_string())
        };
        let mut folders: Vec<(&str, &str, RewriteRows)> = vec![
            ("updateShow", "UpdateShowInput", Vec::new()),
            ("updateAlbum", "UpdateAlbumInput", Vec::new()),
            ("updateAudiobook", "UpdateAudiobookInput", Vec::new()),
        ];
        for show in all_rows!(Show::query(database.pool().pool()).filter(ShowWhereInput {
            library_id: Some(StringFilter {
                eq: Some(library.id.clone()),
                ..Default::default()
            }),
            ..Default::default()
        })) {
            if let Some(path) = moved(&show.path) {
                folders[0]
                    .2
                    .push((show.id, serde_json::json!({ "path": path })));
            }
        }
        for album in all_rows!(
            Album::query(database.pool().pool()).filter(AlbumWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
        ) {
            if let Some(path) = moved(&album.path) {
                folders[1]
                    .2
                    .push((album.id, serde_json::json!({ "path": path })));
            }
        }
        for book in all_rows!(Audiobook::query(database.pool().pool()).filter(
            AudiobookWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            }
        )) {
            if let Some(path) = moved(&book.path) {
                folders[2]
                    .2
                    .push((book.id, serde_json::json!({ "path": path })));
            }
        }
        for (name, input_type, rows) in folders {
            outcome.moved_folders += rows.len();
            self.rewrite_rows(auth_user, name, input_type, rows).await?;
        }

        // Sidecar subtitles that hold an absolute path under the old root.
        let ids = prepared
            .files
            .iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        let mut subtitle_rows = Vec::new();
        for chunk in ids.chunks(500) {
            for subtitle in all_rows!(Subtitle::query(database.pool().pool()).filter(
                SubtitleWhereInput {
                    media_file_id: Some(StringFilter {
                        in_list: Some(chunk.to_vec()),
                        ..Default::default()
                    }),
                    ..Default::default()
                }
            )) {
                if let Some(path) = moved(&subtitle.file_path) {
                    subtitle_rows.push((subtitle.id, serde_json::json!({ "filePath": path })));
                }
            }
        }
        self.rewrite_rows(
            auth_user,
            "updateSubtitle",
            "UpdateSubtitleInput",
            subtitle_rows,
        )
        .await?;

        // Open scan issues name paths that no longer exist.
        for issue in all_rows!(LibraryScanIssue::query(database.pool().pool()).filter(
            LibraryScanIssueWhereInput {
                library_id: Some(StringFilter {
                    eq: Some(library.id.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            }
        )) {
            if issue.resolved_at.is_some() {
                continue;
            }
            self.update_scan_issue(
                &issue.id,
                UpdateLibraryScanIssueInput {
                    resolved_at: Some(Some(Utc::now().to_rfc3339())),
                    resolution: Some(Some(format!(
                        "The library was moved to {target}; a scan will report anything that still applies"
                    ))),
                    ..Default::default()
                },
            )
            .await?;
            outcome.closed_issues += 1;
        }

        // The library's own path last: until then, a stopped move is still resumable from it.
        self.execute_mutation(
            auth_user,
            r#"mutation MoveLibraryRoot($id: String!, $input: UpdateLibraryInput!) {
                UpdateLibrary: updateLibrary(id: $id, input: $input) { success error }
            }"#,
            serde_json::json!({ "id": library.id, "input": { "path": target } }),
        )
        .await
        .context("Failed to change the library's folder")?;
        Ok(outcome)
    }

    async fn write_move_marker(
        &self,
        library_id: &str,
        old_path: &str,
        new_path: &str,
    ) -> Result<()> {
        let database = self.database().await?;
        let value = serde_json::to_string(&MoveMarker {
            new_path: new_path.to_string(),
            old_path: old_path.to_string(),
            started_at: Utc::now().to_rfc3339(),
        })?;
        let key = marker_key(library_id);
        let existing = AppSetting::query(database.pool().pool())
            .filter(AppSettingWhereInput {
                key: Some(StringFilter {
                    eq: Some(key.clone()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .limit(1)
            .fetch_all()
            .await?;
        if let Some(row) = existing.into_iter().next() {
            AppSetting::update_by_id(
                database.pool(),
                &row.id,
                crate::services::graphql::entities::UpdateAppSettingInput {
                    key: None,
                    value: Some(value),
                    description: None,
                    category: None,
                },
            )
            .await?;
        } else {
            AppSetting::insert(
                database.pool(),
                CreateAppSettingInput {
                    key,
                    value,
                    description: Some("A library move that is under way".to_string()),
                    category: "library".to_string(),
                },
            )
            .await?;
        }
        Ok(())
    }

    async fn clear_move_marker(&self, library_id: &str) -> Result<()> {
        let database = self.database().await?;
        let rows = AppSetting::query(database.pool().pool())
            .filter(AppSettingWhereInput {
                key: Some(StringFilter {
                    eq: Some(marker_key(library_id)),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await?;
        let auth_user = self.system_auth_user(None).await?;
        for row in rows {
            self.execute_mutation(
                &auth_user,
                r#"mutation ClearLibraryMove($id: String!) {
                    DeleteAppSetting: deleteAppSetting(id: $id) { success error }
                }"#,
                serde_json::json!({ "id": row.id }),
            )
            .await?;
        }
        Ok(())
    }

    /// Finish the moves a stopped server left half done. Called once after startup.
    pub async fn resume_library_moves(&self) {
        let Ok(database) = self.database().await else {
            return;
        };
        let markers = match AppSetting::query(database.pool().pool())
            .filter(AppSettingWhereInput {
                key: Some(StringFilter {
                    starts_with: Some(MOVE_MARKER_PREFIX.to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
        {
            Ok(markers) => markers,
            Err(error) => {
                warn!(error = %error, "Could not look for library moves to resume");
                return;
            }
        };
        for row in markers {
            let Some(library_id) = row.key.strip_prefix(MOVE_MARKER_PREFIX) else {
                continue;
            };
            let Ok(marker) = serde_json::from_str::<MoveMarker>(&row.value) else {
                continue;
            };
            info!(
                library_id,
                new_path = %marker.new_path,
                "Resuming the move of library {} to {}",
                library_id,
                marker.new_path
            );
            if let Err(error) = self
                .apply_library_move(library_id, &marker.new_path, true)
                .await
            {
                warn!(
                    library_id,
                    new_path = %marker.new_path,
                    error = %error,
                    "Could not finish moving library {} to {}: {}",
                    library_id,
                    marker.new_path,
                    error
                );
            }
        }
    }
}

/// A move that is under way.
#[derive(Debug, Clone)]
pub struct MoveMarkerInfo {
    pub new_path: String,
}
