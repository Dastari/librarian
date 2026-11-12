//! The vocabulary every storage backend speaks: errors, entry metadata, walking and placing.

use std::sync::Arc;
use std::time::SystemTime;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::path::StoragePath;

/// Why a storage call failed, in terms the pipeline decides on. Backends map their own errors
/// (an `io::Error`, an SMB status, an S3 error code) onto these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageErrorKind {
    NotFound,
    AlreadyExists,
    /// `remove_dir` on a folder with something in it.
    NotEmpty,
    PermissionDenied,
    /// The backend cannot do this at all (a bucket has no hard links).
    Unsupported,
    /// The connection to the storage is down or the storage is unreachable.
    Unavailable,
    Other,
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct StorageError {
    kind: StorageErrorKind,
    message: String,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

pub type StorageResult<T> = std::result::Result<T, StorageError>;

impl StorageError {
    pub fn new(kind: StorageErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
        }
    }

    pub fn unsupported(what: &str) -> Self {
        Self::new(
            StorageErrorKind::Unsupported,
            format!("This storage does not support {what}"),
        )
    }

    pub fn with_source(
        kind: StorageErrorKind,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn kind(&self) -> StorageErrorKind {
        self.kind
    }

    /// The same message, filed under another kind.
    pub fn with_kind(mut self, kind: StorageErrorKind) -> Self {
        self.kind = kind;
        self
    }

    /// Whether the storage could not be reached (a share that is down), as opposed to an answer
    /// about the file.
    pub fn is_unavailable(&self) -> bool {
        self.kind == StorageErrorKind::Unavailable
    }

    pub fn is_not_found(&self) -> bool {
        self.kind == StorageErrorKind::NotFound
    }
}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        use std::io::ErrorKind;
        let kind = match error.kind() {
            ErrorKind::NotFound => StorageErrorKind::NotFound,
            ErrorKind::AlreadyExists => StorageErrorKind::AlreadyExists,
            ErrorKind::PermissionDenied => StorageErrorKind::PermissionDenied,
            ErrorKind::DirectoryNotEmpty => StorageErrorKind::NotEmpty,
            // ENOTEMPTY where the kind is not mapped.
            _ if error.raw_os_error() == Some(39) => StorageErrorKind::NotEmpty,
            _ => StorageErrorKind::Other,
        };
        Self {
            kind,
            message: error.to_string(),
            source: Some(Box::new(error)),
        }
    }
}

/// What an entry is. A symlink is its own kind: a walk does not follow it, and a library never
/// organizes or deletes through one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

/// Which file an entry is, within one storage. Two paths with the same identity are the same
/// file (a hard link, or the same path spelled twice).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileIdentity {
    pub volume: u64,
    pub index: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryMeta {
    pub kind: EntryKind,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub identity: Option<FileIdentity>,
}

impl EntryMeta {
    pub fn is_file(&self) -> bool {
        self.kind == EntryKind::File
    }

    pub fn is_dir(&self) -> bool {
        self.kind == EntryKind::Directory
    }
}

/// One entry of a folder listing. `meta` is there when the backend has it for free (an SMB
/// directory query returns sizes and times with the names; a local `readdir` does not).
#[derive(Clone, Debug)]
pub struct DirEntry {
    pub name: String,
    pub kind: EntryKind,
    pub meta: Option<EntryMeta>,
}

/// What a walk is looking at, before deciding whether to descend into or report it.
#[derive(Clone, Copy, Debug)]
pub struct WalkCandidate<'a> {
    /// 0 is the walk root itself.
    pub depth: usize,
    pub name: &'a str,
    pub kind: EntryKind,
}

pub type WalkFilter = Arc<dyn Fn(&WalkCandidate<'_>) -> bool + Send + Sync>;

/// How to walk a tree.
#[derive(Clone, Default)]
pub struct WalkOptions {
    /// Return `true` to leave an entry out of the walk, and not descend into it if it is a
    /// folder. Applied to the root too.
    pub prune: Option<WalkFilter>,
    /// Return `true` to report an entry. `None` reports everything that was not pruned.
    pub report: Option<WalkFilter>,
    /// Fill in [`WalkEntry::meta`] for the entries that are reported. A local walk pays a `stat`
    /// per reported entry for it; a remote one already has the answer from the listing.
    pub with_metadata: bool,
}

#[derive(Clone, Debug)]
pub struct WalkEntry {
    pub path: StoragePath,
    pub depth: usize,
    pub kind: EntryKind,
    /// `None` when metadata was not asked for or could not be read.
    pub meta: Option<EntryMeta>,
}

/// A walk in progress. Entries arrive as they are found; dropping `entries` stops the walk.
pub struct WalkStream {
    pub entries: mpsc::Receiver<StorageResult<WalkEntry>>,
    /// Finishes when the walk has stopped.
    pub task: JoinHandle<()>,
}

/// What to do when the destination already exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceMode {
    /// Never replace: report [`PlaceOutcome::TargetExists`] and touch nothing.
    NoClobber,
    /// Replace what is there.
    Replace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceOutcome {
    /// The source (or its bytes, for a copy) is now at the destination.
    Placed,
    /// The destination already existed; nothing was touched.
    TargetExists,
}

/// What a backend can do, so callers choose a strategy rather than assume one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageCapabilities {
    /// Renaming within the storage is one atomic, cheap operation. `false` (a bucket) means a
    /// rename is copy then delete and can be interrupted half way.
    pub atomic_rename: bool,
    /// `NoClobber` placement and rename cannot race with another writer.
    pub atomic_no_clobber: bool,
    /// Folders are real objects: they can be created empty and removed when empty. `false` (a
    /// bucket) means folders exist only as prefixes of files, `create_dir_all` does nothing and an
    /// "empty folder" cannot occur.
    pub real_directories: bool,
    pub hard_links: bool,
    pub symlinks: bool,
    /// The storage can say when something under a folder changed.
    pub change_notifications: bool,
    /// `EntryMeta::identity` tells whether two paths are the same file.
    pub file_identity: bool,
    /// External tools (ffprobe, ffmpeg, fpcalc, whisper) cannot open a path on this storage and
    /// need a URL served by Librarian.
    pub tools_need_url: bool,
}

/// How an external tool reads a file: by path, or by a URL on the loopback server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolInput {
    /// A path on this machine's filesystem, passed as the tool's input argument.
    Path(std::path::PathBuf),
    /// A short-lived, single-file URL on Librarian's own loopback server. See `docs/design.md`
    /// Q85. Not issued yet: no backend returns this.
    Url(String),
}
