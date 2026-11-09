//! Where library files live, behind one interface.
//!
//! Everything that reads or writes a library file (scans, streaming, imports, organizing, quality
//! upgrades, duplicate trash) goes through [`Storage`] instead of `std::fs`, so a library folder
//! can sit on this machine's disk, on an SMB share reached without an OS mount, or later in a
//! cloud bucket. Only the local backend exists today; `docs/design.md` Q85 records the model and
//! the plan for the rest.
//!
//! The interface assumes as little as a bucket gives: no real directories, no hard links, no
//! atomic rename, no way to hand a path to another program. What a backend can really do is
//! reported by [`Storage::capabilities`], and callers pick a strategy from it rather than
//! assuming a filesystem.
//!
//! Not to be confused with [`crate::services::storage`], the object store for blobs (artwork,
//! backups) that do not belong to a library.

mod local;
mod location;
mod path;
mod registry;
mod smb;
mod smb_reader;
mod types;

pub use local::{LocalStorage, place_no_clobber_blocking};
pub use location::{CredentialRef, LocationError, SmbLocation, StorageLocation};
pub use path::{IntoStoragePath, StorageKind, StoragePath, is_smb_uri};
pub use registry::{LibraryStorage, ShareProvider, ToolUrlIssuer, library_storage};
pub use smb::{SmbCredentials, SmbStorage};
pub use types::{
    DirEntry, EntryKind, EntryMeta, FileIdentity, PlaceMode, PlaceOutcome, StorageCapabilities,
    StorageError, StorageErrorKind, StorageResult, ToolInput, WalkCandidate, WalkEntry, WalkFilter,
    WalkOptions, WalkStream,
};

use std::path::Path;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek};
use tokio_util::sync::CancellationToken;

/// An open file that can be read from any position: what a video player scrubbing needs.
pub trait StorageRead: AsyncRead + AsyncSeek + Send + Unpin {}
impl<T: AsyncRead + AsyncSeek + Send + Unpin> StorageRead for T {}

/// A file opened for reading, with its size and times as of the open.
pub struct OpenedRead {
    pub reader: Box<dyn StorageRead>,
    pub meta: EntryMeta,
}

/// One place library files live. Paths are the strings the database keeps; a backend only ever
/// receives paths of its own kind.
#[async_trait]
pub trait Storage: Send + Sync {
    fn kind(&self) -> StorageKind;

    fn capabilities(&self) -> StorageCapabilities;

    /// Size, kind and times, following a symlink to what it points at.
    async fn stat(&self, path: &StoragePath) -> StorageResult<EntryMeta>;

    /// Like [`stat`](Self::stat), but a symlink is reported as a symlink.
    async fn lstat(&self, path: &StoragePath) -> StorageResult<EntryMeta>;

    /// Whether the path exists. Only "not found" is `false`: any other failure is an error, so a
    /// share that is down is never mistaken for a file that is gone.
    async fn exists(&self, path: &StoragePath) -> StorageResult<bool> {
        match self.stat(path).await {
            Ok(_) => Ok(true),
            Err(error) if error.is_not_found() => Ok(false),
            Err(error) => Err(error),
        }
    }

    /// The entries of one folder, without `.` and `..`, in no particular order.
    async fn list_dir(&self, dir: &StoragePath) -> StorageResult<Vec<DirEntry>>;

    /// Whether a folder has nothing in it.
    async fn is_dir_empty(&self, dir: &StoragePath) -> StorageResult<bool> {
        Ok(self.list_dir(dir).await?.is_empty())
    }

    /// Walk the tree under `root`, root first, without following symlinks. Stops when `cancel` is
    /// cancelled or the receiver is dropped.
    fn walk(
        &self,
        root: &StoragePath,
        options: WalkOptions,
        cancel: CancellationToken,
    ) -> WalkStream;

    /// Open a file for reading from any position.
    async fn open_read(&self, path: &StoragePath) -> StorageResult<OpenedRead>;

    /// The whole file. For small files only (sidecar text, subtitles).
    async fn read_all(&self, path: &StoragePath) -> StorageResult<Vec<u8>> {
        let mut opened = self.open_read(path).await?;
        let mut bytes = Vec::new();
        opened.reader.read_to_end(&mut bytes).await?;
        Ok(bytes)
    }

    /// Copy a file from this machine's disk to `dst`, which must be in this storage. The source
    /// is never changed. With [`PlaceMode::NoClobber`] an existing `dst` is left alone.
    async fn put_from_local(
        &self,
        src: &Path,
        dst: &StoragePath,
        mode: PlaceMode,
    ) -> StorageResult<PlaceOutcome>;

    /// Move a file within this storage. With [`PlaceMode::NoClobber`] an existing `to` is left
    /// alone. Where [`StorageCapabilities::atomic_rename`] is `false` this is a copy then a
    /// delete and a failure can leave both.
    async fn rename(
        &self,
        from: &StoragePath,
        to: &StoragePath,
        mode: PlaceMode,
    ) -> StorageResult<PlaceOutcome>;

    /// Delete one file. The parent folder stays.
    async fn remove_file(&self, path: &StoragePath) -> StorageResult<()>;

    /// Make a folder and its missing parents. Making one that exists is not an error. A no-op
    /// where folders are only prefixes.
    async fn create_dir_all(&self, dir: &StoragePath) -> StorageResult<()>;

    /// Remove a folder only if it is empty (`NotEmpty` otherwise), so a file that lands in it a
    /// moment before is never lost.
    async fn remove_dir(&self, dir: &StoragePath) -> StorageResult<()>;

    /// How an external program should read this file. See `docs/design.md` Q85.
    async fn tool_input(&self, path: &StoragePath) -> StorageResult<ToolInput>;
}

/// Path prefix of the loopback URLs that serve a file on a share to a tool.
pub const TOOL_URL_MARKER: &str = "/internal/tool/";

/// Whether a tool argument is a loopback tool URL rather than a path.
pub fn is_tool_url(argument: &str) -> bool {
    argument.starts_with("http://") && argument.contains(TOOL_URL_MARKER)
}

/// What ffmpeg and ffprobe must be allowed to open to read a tool URL (their default for a
/// `-f`-forced or playlist-like input is file and pipe only).
pub const TOOL_PROTOCOL_WHITELIST: &str = "file,pipe,http,tcp";

/// What to put where a tool takes its input: the file's own path on this machine, or the loopback
/// URL for a file on a share. The tool is told nothing else about the storage.
pub async fn tool_argument(path: impl IntoStoragePath) -> StorageResult<String> {
    Ok(match library_storage().tool_input(path).await? {
        ToolInput::Path(path) => path.to_string_lossy().into_owned(),
        ToolInput::Url(url) => url,
    })
}

/// [`tool_argument`] as a `PathBuf`, for tool wrappers that take a path (a URL passed through
/// `Command::arg` is just text, so a `PathBuf` carries it unchanged).
pub async fn tool_input_path(path: impl IntoStoragePath) -> StorageResult<std::path::PathBuf> {
    Ok(std::path::PathBuf::from(tool_argument(path).await?))
}
