//! The storage on this machine's own filesystem (a mounted network share counts as local).
//!
//! This is the behaviour the pipeline already had, moved behind [`Storage`]: it does not follow
//! symlinks while walking, places files without clobbering through a hard link, and reports file
//! identity from the device and inode.

use std::path::{Path, PathBuf};

use anyhow::Context;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use walkdir::WalkDir;

use super::{
    DirEntry, EntryKind, EntryMeta, FileIdentity, OpenedRead, PlaceMode, PlaceOutcome, Storage,
    StorageCapabilities, StorageError, StorageErrorKind, StorageKind, StoragePath, StorageResult,
    ToolInput, WalkCandidate, WalkEntry, WalkOptions, WalkStream,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalStorage;

fn local_path(path: &StoragePath) -> StorageResult<&Path> {
    path.as_local().ok_or_else(|| {
        StorageError::new(
            StorageErrorKind::Unsupported,
            format!("'{path}' is not a path on this machine"),
        )
    })
}

fn kind_of(file_type: std::fs::FileType) -> EntryKind {
    if file_type.is_symlink() {
        EntryKind::Symlink
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else if file_type.is_file() {
        EntryKind::File
    } else {
        EntryKind::Other
    }
}

fn meta_of(metadata: &std::fs::Metadata) -> EntryMeta {
    #[cfg(unix)]
    let identity = {
        use std::os::unix::fs::MetadataExt;
        Some(FileIdentity {
            volume: metadata.dev(),
            index: metadata.ino(),
        })
    };
    #[cfg(not(unix))]
    let identity = None;
    EntryMeta {
        kind: kind_of(metadata.file_type()),
        size: metadata.len(),
        modified: metadata.modified().ok(),
        identity,
    }
}

#[async_trait]
impl Storage for LocalStorage {
    fn kind(&self) -> StorageKind {
        StorageKind::Local
    }

    fn capabilities(&self) -> StorageCapabilities {
        StorageCapabilities {
            atomic_rename: true,
            atomic_no_clobber: true,
            real_directories: true,
            hard_links: true,
            symlinks: true,
            change_notifications: true,
            file_identity: cfg!(unix),
            tools_need_url: false,
        }
    }

    async fn stat(&self, path: &StoragePath) -> StorageResult<EntryMeta> {
        Ok(meta_of(&tokio::fs::metadata(local_path(path)?).await?))
    }

    async fn lstat(&self, path: &StoragePath) -> StorageResult<EntryMeta> {
        Ok(meta_of(
            &tokio::fs::symlink_metadata(local_path(path)?).await?,
        ))
    }

    async fn exists(&self, path: &StoragePath) -> StorageResult<bool> {
        Ok(tokio::fs::try_exists(local_path(path)?).await?)
    }

    async fn list_dir(&self, dir: &StoragePath) -> StorageResult<Vec<DirEntry>> {
        let mut reader = tokio::fs::read_dir(local_path(dir)?).await?;
        let mut entries = Vec::new();
        while let Some(entry) = reader.next_entry().await? {
            let kind = entry
                .file_type()
                .await
                .map(kind_of)
                .unwrap_or(EntryKind::Other);
            entries.push(DirEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                kind,
                meta: None,
            });
        }
        Ok(entries)
    }

    async fn is_dir_empty(&self, dir: &StoragePath) -> StorageResult<bool> {
        let mut reader = tokio::fs::read_dir(local_path(dir)?).await?;
        Ok(reader.next_entry().await?.is_none())
    }

    fn walk(
        &self,
        root: &StoragePath,
        options: WalkOptions,
        cancel: CancellationToken,
    ) -> WalkStream {
        let (tx, entries) = mpsc::channel(128);
        let root = PathBuf::from(root.as_str());
        let task = tokio::task::spawn_blocking(move || {
            let candidate_of = |entry: &walkdir::DirEntry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                (entry.depth(), name, kind_of(entry.file_type()))
            };
            let prune = options.prune.clone();
            let walk = WalkDir::new(&root).into_iter().filter_entry(|entry| {
                let Some(prune) = &prune else {
                    return true;
                };
                let (depth, name, kind) = candidate_of(entry);
                !prune(&WalkCandidate {
                    depth,
                    name: &name,
                    kind,
                })
            });
            for result in walk {
                if cancel.is_cancelled() {
                    break;
                }
                let entry = match result {
                    Ok(entry) => entry,
                    Err(error) => {
                        // walkdir's own text names the path that failed.
                        let kind = error
                            .io_error()
                            .map(|io| StorageError::from(std::io::Error::from(io.kind())).kind())
                            .unwrap_or(StorageErrorKind::Other);
                        let error = StorageError::new(kind, error.to_string());
                        if tx.blocking_send(Err(error)).is_err() {
                            break;
                        }
                        continue;
                    }
                };
                let (depth, name, kind) = candidate_of(&entry);
                if let Some(report) = &options.report
                    && !report(&WalkCandidate {
                        depth,
                        name: &name,
                        kind,
                    })
                {
                    continue;
                }
                let meta = if options.with_metadata {
                    entry.metadata().ok().map(|metadata| meta_of(&metadata))
                } else {
                    None
                };
                let reported = WalkEntry {
                    path: StoragePath::new(entry.path().to_string_lossy().into_owned()),
                    depth,
                    kind,
                    meta,
                };
                if tx.blocking_send(Ok(reported)).is_err() {
                    break;
                }
            }
        });
        WalkStream { entries, task }
    }

    async fn open_read(&self, path: &StoragePath) -> StorageResult<OpenedRead> {
        let file = tokio::fs::File::open(local_path(path)?).await?;
        let meta = meta_of(&file.metadata().await?);
        Ok(OpenedRead {
            reader: Box::new(file),
            meta,
        })
    }

    async fn put_from_local(
        &self,
        src: &Path,
        dst: &StoragePath,
        mode: PlaceMode,
    ) -> StorageResult<PlaceOutcome> {
        let dst = local_path(dst)?.to_path_buf();
        let src = src.to_path_buf();
        match mode {
            PlaceMode::Replace => {
                tokio::fs::copy(&src, &dst).await?;
                Ok(PlaceOutcome::Placed)
            }
            PlaceMode::NoClobber => {
                tokio::task::spawn_blocking(move || place_no_clobber_blocking(&src, &dst, false))
                    .await
                    .map_err(|error| StorageError::new(StorageErrorKind::Other, error.to_string()))?
                    .map_err(placement_error)
            }
        }
    }

    async fn rename(
        &self,
        from: &StoragePath,
        to: &StoragePath,
        mode: PlaceMode,
    ) -> StorageResult<PlaceOutcome> {
        let (from, to) = (
            local_path(from)?.to_path_buf(),
            local_path(to)?.to_path_buf(),
        );
        match mode {
            PlaceMode::Replace => {
                tokio::fs::rename(&from, &to).await?;
                Ok(PlaceOutcome::Placed)
            }
            PlaceMode::NoClobber => {
                tokio::task::spawn_blocking(move || place_no_clobber_blocking(&from, &to, true))
                    .await
                    .map_err(|error| StorageError::new(StorageErrorKind::Other, error.to_string()))?
                    .map_err(placement_error)
            }
        }
    }

    async fn remove_file(&self, path: &StoragePath) -> StorageResult<()> {
        Ok(tokio::fs::remove_file(local_path(path)?).await?)
    }

    async fn create_dir_all(&self, dir: &StoragePath) -> StorageResult<()> {
        Ok(tokio::fs::create_dir_all(local_path(dir)?).await?)
    }

    async fn remove_dir(&self, dir: &StoragePath) -> StorageResult<()> {
        Ok(tokio::fs::remove_dir(local_path(dir)?).await?)
    }

    async fn tool_input(&self, path: &StoragePath) -> StorageResult<ToolInput> {
        Ok(ToolInput::Path(local_path(path)?.to_path_buf()))
    }
}

/// Keep the failure's kind when the cause is an `io::Error` (a missing parent is `NotFound`).
fn placement_error(error: anyhow::Error) -> StorageError {
    let kind = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<std::io::Error>())
        .map(|io| StorageError::from(std::io::Error::from(io.kind())).kind())
        .unwrap_or(StorageErrorKind::Other);
    StorageError::new(kind, format!("{error:#}"))
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
/// `dst`'s parent directory must already exist. Synchronous syscalls: call from a
/// blocking thread.
pub fn place_no_clobber_blocking(
    src: &Path,
    dst: &Path,
    remove_source_on_success: bool,
) -> anyhow::Result<PlaceOutcome> {
    match std::fs::hard_link(src, dst) {
        Ok(()) => {
            if remove_source_on_success {
                std::fs::remove_file(src).with_context(|| {
                    format!("failed to remove source file after move: {}", src.display())
                })?;
            }
            Ok(PlaceOutcome::Placed)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(PlaceOutcome::TargetExists),
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
) -> anyhow::Result<PlaceOutcome> {
    let tmp_path = cross_device_temp_path(dst);
    let result = (|| -> anyhow::Result<PlaceOutcome> {
        std::fs::copy(src, &tmp_path).with_context(|| {
            format!(
                "cross-device copy-to-temp failed: {} -> {}",
                src.display(),
                tmp_path.display()
            )
        })?;
        match std::fs::hard_link(&tmp_path, dst) {
            Ok(()) => Ok(PlaceOutcome::Placed),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                Ok(PlaceOutcome::TargetExists)
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
    if outcome == PlaceOutcome::Placed && remove_source_on_success {
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
    use super::*;
    use crate::services::library_storage::WalkFilter;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    fn path_in(dir: &Path, name: &str) -> StoragePath {
        StoragePath::new(dir.join(name).to_string_lossy().into_owned())
    }

    async fn collect(mut stream: WalkStream) -> Vec<WalkEntry> {
        let mut out = Vec::new();
        while let Some(entry) = stream.entries.recv().await {
            out.push(entry.expect("walk entry"));
        }
        stream.task.await.expect("walk task");
        out
    }

    #[tokio::test]
    async fn stat_reports_kind_size_and_a_missing_file_as_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.mkv"), b"12345").unwrap();
        let storage = LocalStorage;

        let meta = storage.stat(&path_in(dir.path(), "a.mkv")).await.unwrap();
        assert!(meta.is_file());
        assert_eq!(meta.size, 5);
        assert!(meta.modified.is_some());
        assert!(
            storage
                .stat(&StoragePath::new(dir.path().to_string_lossy().into_owned()))
                .await
                .unwrap()
                .is_dir()
        );

        let missing = path_in(dir.path(), "missing.mkv");
        assert!(storage.stat(&missing).await.unwrap_err().is_not_found());
        assert!(!storage.exists(&missing).await.unwrap());
        assert!(storage.exists(&path_in(dir.path(), "a.mkv")).await.unwrap());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn hard_links_share_an_identity_and_lstat_does_not_follow_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("a.mkv");
        std::fs::write(&original, b"x").unwrap();
        std::fs::hard_link(&original, dir.path().join("b.mkv")).unwrap();
        std::os::unix::fs::symlink(&original, dir.path().join("link.mkv")).unwrap();
        std::fs::write(dir.path().join("c.mkv"), b"x").unwrap();
        let storage = LocalStorage;

        let a = storage.stat(&path_in(dir.path(), "a.mkv")).await.unwrap();
        let b = storage.stat(&path_in(dir.path(), "b.mkv")).await.unwrap();
        let c = storage.stat(&path_in(dir.path(), "c.mkv")).await.unwrap();
        assert!(a.identity.is_some());
        assert_eq!(a.identity, b.identity);
        assert_ne!(a.identity, c.identity);

        let link = path_in(dir.path(), "link.mkv");
        assert_eq!(storage.lstat(&link).await.unwrap().kind, EntryKind::Symlink);
        assert_eq!(storage.stat(&link).await.unwrap().kind, EntryKind::File);
    }

    #[tokio::test]
    async fn open_read_seeks_to_any_position_and_reports_the_size() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.bin"), b"0123456789").unwrap();
        let mut opened = LocalStorage
            .open_read(&path_in(dir.path(), "a.bin"))
            .await
            .unwrap();
        assert_eq!(opened.meta.size, 10);
        opened
            .reader
            .seek(std::io::SeekFrom::Start(4))
            .await
            .unwrap();
        let mut rest = Vec::new();
        opened.reader.read_to_end(&mut rest).await.unwrap();
        assert_eq!(rest, b"456789");
        assert_eq!(
            LocalStorage
                .read_all(&path_in(dir.path(), "a.bin"))
                .await
                .unwrap(),
            b"0123456789"
        );
    }

    #[tokio::test]
    async fn walk_prunes_reports_and_carries_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("Show/Season 1")).unwrap();
        std::fs::create_dir_all(root.join(".trash")).unwrap();
        std::fs::write(root.join("Show/Season 1/e1.mkv"), b"abc").unwrap();
        std::fs::write(root.join("Show/notes.txt"), b"n").unwrap();
        std::fs::write(root.join(".trash/gone.mkv"), b"abc").unwrap();

        let prune: WalkFilter = Arc::new(|c| c.depth == 1 && c.name == ".trash");
        let report: WalkFilter =
            Arc::new(|c| c.kind == EntryKind::File && c.name.ends_with(".mkv"));
        let entries = collect(LocalStorage.walk(
            &StoragePath::new(root.to_string_lossy().into_owned()),
            WalkOptions {
                prune: Some(prune),
                report: Some(report),
                with_metadata: true,
            },
            CancellationToken::new(),
        ))
        .await;

        assert_eq!(entries.len(), 1);
        assert!(entries[0].path.as_str().ends_with("Show/Season 1/e1.mkv"));
        assert_eq!(entries[0].depth, 3);
        assert_eq!(entries[0].meta.as_ref().map(|m| m.size), Some(3));
    }

    #[tokio::test]
    async fn walk_reports_root_first_and_does_not_follow_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("elsewhere.mkv"), b"x").unwrap();
        std::fs::create_dir_all(root.join("inside")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), root.join("inside/link")).unwrap();

        let entries = collect(LocalStorage.walk(
            &StoragePath::new(root.to_string_lossy().into_owned()),
            WalkOptions::default(),
            CancellationToken::new(),
        ))
        .await;
        assert_eq!(entries[0].depth, 0);
        assert!(
            entries
                .iter()
                .all(|entry| !entry.path.as_str().contains("elsewhere.mkv"))
        );
        #[cfg(unix)]
        assert!(
            entries
                .iter()
                .any(|e| e.kind == EntryKind::Symlink && e.path.as_str().ends_with("inside/link"))
        );
    }

    #[tokio::test]
    async fn a_cancelled_walk_stops() {
        let dir = tempfile::tempdir().unwrap();
        for index in 0..50 {
            std::fs::write(dir.path().join(format!("{index}.mkv")), b"x").unwrap();
        }
        let cancel = CancellationToken::new();
        cancel.cancel();
        let entries = collect(LocalStorage.walk(
            &StoragePath::new(dir.path().to_string_lossy().into_owned()),
            WalkOptions::default(),
            cancel,
        ))
        .await;
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn remove_dir_refuses_a_folder_with_something_in_it() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Film")).unwrap();
        std::fs::write(dir.path().join("Film/subs.srt"), b"s").unwrap();
        let storage = LocalStorage;
        let film = path_in(dir.path(), "Film");

        assert!(!storage.is_dir_empty(&film).await.unwrap());
        let error = storage.remove_dir(&film).await.unwrap_err();
        assert_eq!(error.kind(), StorageErrorKind::NotEmpty);

        storage
            .remove_file(&path_in(dir.path(), "Film/subs.srt"))
            .await
            .unwrap();
        assert!(storage.is_dir_empty(&film).await.unwrap());
        storage.remove_dir(&film).await.unwrap();
        assert!(!storage.exists(&film).await.unwrap());
    }

    #[tokio::test]
    async fn create_dir_all_is_idempotent_and_list_dir_names_the_children() {
        let dir = tempfile::tempdir().unwrap();
        let storage = LocalStorage;
        let nested = path_in(dir.path(), "a/b/c");
        storage.create_dir_all(&nested).await.unwrap();
        storage.create_dir_all(&nested).await.unwrap();
        std::fs::write(dir.path().join("a/file.mkv"), b"x").unwrap();

        let mut names: Vec<_> = storage
            .list_dir(&path_in(dir.path(), "a"))
            .await
            .unwrap()
            .into_iter()
            .map(|entry| (entry.name, entry.kind))
            .collect();
        names.sort_by(|left, right| left.0.cmp(&right.0));
        assert_eq!(
            names,
            vec![
                ("b".to_string(), EntryKind::Directory),
                ("file.mkv".to_string(), EntryKind::File)
            ]
        );
    }

    #[tokio::test]
    async fn no_clobber_put_and_rename_never_replace() {
        let dir = tempfile::tempdir().unwrap();
        let storage = LocalStorage;
        std::fs::write(dir.path().join("src.mkv"), b"new").unwrap();
        std::fs::write(dir.path().join("taken.mkv"), b"existing").unwrap();

        let outcome = storage
            .put_from_local(
                &dir.path().join("src.mkv"),
                &path_in(dir.path(), "taken.mkv"),
                PlaceMode::NoClobber,
            )
            .await
            .unwrap();
        assert_eq!(outcome, PlaceOutcome::TargetExists);
        assert_eq!(
            std::fs::read(dir.path().join("taken.mkv")).unwrap(),
            b"existing"
        );

        let outcome = storage
            .put_from_local(
                &dir.path().join("src.mkv"),
                &path_in(dir.path(), "copy.mkv"),
                PlaceMode::NoClobber,
            )
            .await
            .unwrap();
        assert_eq!(outcome, PlaceOutcome::Placed);
        assert!(dir.path().join("src.mkv").exists(), "a put is a copy");

        let outcome = storage
            .rename(
                &path_in(dir.path(), "src.mkv"),
                &path_in(dir.path(), "taken.mkv"),
                PlaceMode::NoClobber,
            )
            .await
            .unwrap();
        assert_eq!(outcome, PlaceOutcome::TargetExists);
        assert!(dir.path().join("src.mkv").exists());

        let outcome = storage
            .rename(
                &path_in(dir.path(), "src.mkv"),
                &path_in(dir.path(), "moved.mkv"),
                PlaceMode::NoClobber,
            )
            .await
            .unwrap();
        assert_eq!(outcome, PlaceOutcome::Placed);
        assert!(!dir.path().join("src.mkv").exists());
        assert_eq!(std::fs::read(dir.path().join("moved.mkv")).unwrap(), b"new");

        storage
            .rename(
                &path_in(dir.path(), "moved.mkv"),
                &path_in(dir.path(), "taken.mkv"),
                PlaceMode::Replace,
            )
            .await
            .unwrap();
        assert_eq!(std::fs::read(dir.path().join("taken.mkv")).unwrap(), b"new");
    }

    #[tokio::test]
    async fn a_missing_parent_is_reported_as_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("src.mkv"), b"x").unwrap();
        let error = LocalStorage
            .put_from_local(
                &dir.path().join("src.mkv"),
                &path_in(dir.path(), "no/such/dir/x.mkv"),
                PlaceMode::NoClobber,
            )
            .await
            .unwrap_err();
        assert!(error.is_not_found(), "{error}");
    }

    #[tokio::test]
    async fn a_remote_path_is_refused_by_the_local_backend() {
        let error = LocalStorage
            .stat(&StoragePath::new("smb://host/share/a.mkv"))
            .await
            .unwrap_err();
        assert_eq!(error.kind(), StorageErrorKind::Unsupported);
    }

    #[tokio::test]
    async fn tools_read_a_local_file_by_its_path() {
        let input = LocalStorage
            .tool_input(&StoragePath::new("/data/media/a.mkv"))
            .await
            .unwrap();
        assert_eq!(input, ToolInput::Path(PathBuf::from("/data/media/a.mkv")));
    }
}
