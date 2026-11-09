//! Torrent payloads on an SMB share (`docs/design.md` Q100).
//!
//! librqbit writes through a [`TorrentStorage`]: synchronous positioned reads and writes, called
//! from blocking threads. A torrent whose folder is an `smb://` address gets a [`ShareStorage`],
//! which reaches the share through the same `SmbStorage` connections the libraries use; every
//! other torrent keeps librqbit's own filesystem storage.
//!
//! **Which folder.** librqbit does not show a storage factory the torrent's folder, so the
//! factory works it out the way the session does: the folder the session last stored for that
//! info hash (read from its `session.json` at start, and registered on every add), else the
//! session's download folder, plus the torrent's name for a torrent of more than one file.
//!
//! **Writes are gathered.** Peers send 16 KiB blocks; one SMB request per block would be one
//! round trip per block. Blocks are kept in memory per file, joined while they are contiguous,
//! and written in large requests when a run reaches [`FLUSH_RUN`], when the torrent's buffered
//! bytes pass [`MAX_BUFFERED`], when a read touches them, and when the torrent pauses. The read
//! that checks a finished piece always flushes that piece's blocks first and is answered from
//! the bytes the server has just acknowledged, so a piece is only ever marked complete once it is
//! on the share. Unchecked blocks lost to a crash are downloaded again, as they would be anyway.
//!
//! **Outages.** A call that cannot reach the share fails with a message that says so; librqbit
//! then stops the torrent with that error, and the share watcher (`share_watch`) resumes it when
//! the share answers again. Nothing here deletes anything except when librqbit is told to delete
//! the torrent's files.

use std::any::TypeId;
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::io::IoSlice;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use anyhow::{Context, anyhow, bail};
use librqbit::storage::filesystem::FilesystemStorageFactory;
use librqbit::storage::{BoxStorageFactory, StorageFactory, StorageFactoryExt, TorrentStorage};
use librqbit::{ManagedTorrentShared, TorrentMetadata};
use parking_lot::{Mutex, RwLock};

use crate::services::library_storage::{
    SmbFile, SmbStorage, Storage, StorageErrorKind, StoragePath, is_smb_uri, library_storage,
};

/// A contiguous run of buffered blocks is written once it is this long.
const FLUSH_RUN: usize = 4 * 1024 * 1024;
/// A torrent never holds more than this in unwritten blocks.
const MAX_BUFFERED: u64 = 32 * 1024 * 1024;

/// What the factory knows about where torrents live.
#[derive(Default)]
struct Folders {
    /// The session's download folder.
    default: Option<PathBuf>,
    /// The folder last used for each info hash (40 hex characters, lower case).
    by_hash: HashMap<String, PathBuf>,
}

fn folders() -> &'static RwLock<Folders> {
    static FOLDERS: OnceLock<RwLock<Folders>> = OnceLock::new();
    FOLDERS.get_or_init(|| RwLock::new(Folders::default()))
}

/// The session's download folder, as the session was created with it.
pub fn set_default_folder(folder: &Path) {
    folders().write().default = Some(folder.to_path_buf());
}

/// Remember the folder a torrent is stored in.
pub fn register_folder(info_hash: &str, folder: &Path) {
    folders()
        .write()
        .by_hash
        .insert(info_hash.to_ascii_lowercase(), folder.to_path_buf());
}

/// Read the folders librqbit's JSON persistence stored (`<session dir>/session.json`), so a
/// torrent restored at start finds its share before librqbit opens it.
pub fn load_persisted_folders(session_dir: &Path) {
    let Ok(raw) = std::fs::read_to_string(session_dir.join("session.json")) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return;
    };
    let mut found = 0usize;
    if let Some(torrents) = value.get("torrents").and_then(|t| t.as_object()) {
        for torrent in torrents.values() {
            if let (Some(hash), Some(folder)) = (
                torrent.get("info_hash").and_then(|v| v.as_str()),
                torrent.get("output_folder").and_then(|v| v.as_str()),
            ) {
                register_folder(hash, Path::new(folder));
                found += 1;
            }
        }
    }
    tracing::debug!(
        torrents = found,
        "Read the torrent folders the session stored"
    );
}

fn hex(info_hash: &[u8; 20]) -> String {
    info_hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The folder librqbit puts this torrent in, worked out as `Session::add_torrent` does for a
/// torrent added without an explicit folder (all of Librarian's are).
fn folder_for(shared: &ManagedTorrentShared, metadata: &TorrentMetadata) -> Option<PathBuf> {
    let folders = folders().read();
    if let Some(folder) = folders.by_hash.get(&hex(&shared.info_hash.0)) {
        return Some(folder.clone());
    }
    let default = folders.default.clone()?;
    if metadata.file_infos.len() < 2 {
        return Some(default);
    }
    match metadata.info.name() {
        Some(name) if !name.is_empty() => Some(default.join(name.as_ref())),
        _ => Some(default),
    }
}

/// The session's storage factory: a share for a torrent whose folder is an `smb://` address,
/// librqbit's filesystem storage for every other.
#[derive(Clone, Copy, Default)]
pub struct LibrarianStorageFactory;

impl StorageFactory for LibrarianStorageFactory {
    type Storage = Box<dyn TorrentStorage>;

    fn create(
        &self,
        shared: &ManagedTorrentShared,
        metadata: &TorrentMetadata,
    ) -> anyhow::Result<Self::Storage> {
        match folder_for(shared, metadata) {
            Some(folder) if is_smb_uri(&folder.to_string_lossy()) => {
                Ok(Box::new(ShareStorage::new(folder, metadata)?))
            }
            _ => Ok(Box::new(
                FilesystemStorageFactory::default().create(shared, metadata)?,
            )),
        }
    }

    /// This wraps librqbit's filesystem storage and keeps its layout (one folder per torrent,
    /// the torrent's own relative paths), which is what the session's JSON persistence asks
    /// about: like librqbit's own middlewares, it answers for what it wraps.
    fn is_type_id(&self, type_id: TypeId) -> bool {
        type_id == TypeId::of::<FilesystemStorageFactory>() || type_id == TypeId::of::<Self>()
    }

    fn clone_box(&self) -> BoxStorageFactory {
        self.boxed()
    }
}

/// Run a future on the server's runtime and wait for it from a synchronous storage call.
/// librqbit calls storage from blocking threads (and, when deleting, from a worker), so the
/// future is spawned rather than driven here.
fn block_on<T: Send + 'static>(
    runtime: &tokio::runtime::Handle,
    future: impl Future<Output = T> + Send + 'static,
) -> anyhow::Result<T> {
    futures::executor::block_on(runtime.spawn(future)).map_err(|error| anyhow!("{error}"))
}

/// Blocks of one file waiting to be written: start offset to bytes, never overlapping.
#[derive(Default)]
struct Pending {
    runs: BTreeMap<u64, Vec<u8>>,
    bytes: u64,
}

impl Pending {
    /// Add a block. Returns a run that has grown long enough to write, taken out of the buffer.
    fn add(&mut self, offset: u64, data: &[u8]) -> Option<(u64, Vec<u8>)> {
        // Joined to the run that ends where it starts.
        // librqbit can write nothing at a file boundary: an empty run would replace one that
        // starts there.
        if data.is_empty() {
            return None;
        }
        let joined = self
            .runs
            .range_mut(..=offset)
            .next_back()
            .filter(|(start, run)| **start + run.len() as u64 == offset)
            .map(|(start, run)| {
                run.extend_from_slice(data);
                *start
            });
        let start = match joined {
            Some(start) => start,
            None => {
                self.runs.insert(offset, data.to_vec());
                offset
            }
        };
        self.bytes += data.len() as u64;
        // And the run that starts where it now ends.
        let end = start + self.runs[&start].len() as u64;
        if let Some(next) = self.runs.remove(&end) {
            self.runs
                .get_mut(&start)
                .expect("the run was just written")
                .extend_from_slice(&next);
        }
        if self.runs[&start].len() >= FLUSH_RUN {
            let run = self.runs.remove(&start).expect("present");
            self.bytes -= run.len() as u64;
            return Some((start, run));
        }
        None
    }

    /// Whether a block at `offset` of `len` bytes would overlap something buffered.
    fn overlaps(&self, offset: u64, len: u64) -> bool {
        let end = offset + len;
        self.runs
            .range(..end)
            .next_back()
            .is_some_and(|(start, run)| *start + run.len() as u64 > offset)
    }

    /// Take out every run that overlaps `offset..offset + len`.
    fn take_overlapping(&mut self, offset: u64, len: u64) -> Vec<(u64, Vec<u8>)> {
        let end = offset + len;
        let starts: Vec<u64> = self
            .runs
            .range(..end)
            .filter(|(start, run)| **start + run.len() as u64 > offset)
            .map(|(start, _)| *start)
            .collect();
        starts
            .into_iter()
            .map(|start| {
                let run = self.runs.remove(&start).expect("present");
                self.bytes -= run.len() as u64;
                (start, run)
            })
            .collect()
    }

    fn take_all(&mut self) -> Vec<(u64, Vec<u8>)> {
        self.bytes = 0;
        std::mem::take(&mut self.runs).into_iter().collect()
    }
}

struct FileSlot {
    path: StoragePath,
    len: u64,
    padding: bool,
    handle: tokio::sync::Mutex<Option<Arc<SmbFile>>>,
    pending: Mutex<Pending>,
}

struct Inner {
    runtime: tokio::runtime::Handle,
    folder: StoragePath,
    files: Vec<FileSlot>,
    buffered: AtomicU64,
    /// Set by `take`: this copy was replaced and must not be used.
    retired: AtomicBool,
}

impl Drop for Inner {
    /// Handles still open when the storage goes (a torrent removed or the session stopped) are
    /// closed on the server, so a later delete or rename of the file is not held up.
    fn drop(&mut self) {
        let handles: Vec<Arc<SmbFile>> = self
            .files
            .iter_mut()
            .filter_map(|slot| slot.handle.get_mut().take())
            .collect();
        if handles.is_empty() {
            return;
        }
        self.runtime.spawn(async move {
            for handle in handles {
                handle.close().await;
            }
        });
    }
}

/// One torrent's files on a share.
pub struct ShareStorage {
    inner: Arc<Inner>,
}

impl Inner {
    /// A storage failure as librqbit sees it. One that could not reach the share is reported to
    /// the share watcher, which pauses the share's torrents until it answers again.
    fn storage_error(
        &self,
        error: crate::services::library_storage::StorageError,
    ) -> anyhow::Error {
        if error.kind() == StorageErrorKind::Unavailable {
            super::share_watch::report_outage(self.folder.as_str(), &error.to_string());
            anyhow!("The download share is unavailable: {error}")
        } else {
            anyhow!("{error}")
        }
    }

    fn check_live(&self) -> anyhow::Result<()> {
        if self.retired.load(Ordering::Acquire) {
            bail!("this torrent storage was replaced");
        }
        Ok(())
    }

    async fn share(&self) -> anyhow::Result<SmbStorage> {
        let storage = library_storage()
            .smb_storage_for(&self.folder)
            .await
            .map_err(|error| self.storage_error(error))?;
        Ok(storage)
    }

    /// The open handle for a file, opening (and creating) it if there is none or the one there
    /// is dead.
    async fn handle(&self, file_id: usize) -> anyhow::Result<Arc<SmbFile>> {
        let slot = self.files.get(file_id).context("no such file")?;
        let mut held = slot.handle.lock().await;
        if let Some(handle) = held.as_ref()
            && handle.is_alive()
        {
            return Ok(handle.clone());
        }
        let share = self.share().await?;
        let (file, _) = share
            .open_file(&slot.path, true)
            .await
            .map_err(|error| self.storage_error(error))?;
        let file = Arc::new(file);
        *held = Some(file.clone());
        Ok(file)
    }

    async fn write_runs(&self, file_id: usize, runs: Vec<(u64, Vec<u8>)>) -> anyhow::Result<()> {
        if runs.is_empty() {
            return Ok(());
        }
        let handle = self.handle(file_id).await?;
        for (offset, data) in runs {
            handle
                .write_at(offset, &data)
                .await
                .map_err(|error| self.storage_error(error))?;
        }
        Ok(())
    }

    async fn close_all(&self) {
        for slot in &self.files {
            if let Some(handle) = slot.handle.lock().await.take() {
                handle.close().await;
            }
        }
    }

    async fn flush_all(&self) -> anyhow::Result<()> {
        let mut first_error = None;
        for file_id in 0..self.files.len() {
            let runs = self.files[file_id].pending.lock().take_all();
            let bytes: u64 = runs.iter().map(|(_, run)| run.len() as u64).sum();
            self.buffered.fetch_sub(bytes, Ordering::AcqRel);
            if let Err(error) = self.write_runs(file_id, runs).await
                && first_error.is_none()
            {
                first_error = Some(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

impl ShareStorage {
    fn new(folder: PathBuf, metadata: &TorrentMetadata) -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Handle::try_current()
            .context("torrent storage on a share needs the server's runtime")?;
        let folder = StoragePath::new(folder.to_string_lossy().into_owned());
        if folder.as_smb().is_none() {
            bail!("'{folder}' is not a valid smb:// folder");
        }
        let files = metadata
            .file_infos
            .iter()
            .map(|info| {
                let mut path = folder.clone();
                for part in info.relative_filename.components() {
                    path = path.join(&part.as_os_str().to_string_lossy());
                }
                FileSlot {
                    path,
                    len: info.len,
                    padding: info.attrs.padding,
                    handle: tokio::sync::Mutex::new(None),
                    pending: Mutex::new(Pending::default()),
                }
            })
            .collect();
        Ok(Self {
            inner: Arc::new(Inner {
                runtime,
                folder,
                files,
                buffered: AtomicU64::new(0),
                retired: AtomicBool::new(false),
            }),
        })
    }

    fn run<T: Send + 'static, F>(&self, work: impl FnOnce(Arc<Inner>) -> F) -> anyhow::Result<T>
    where
        F: Future<Output = anyhow::Result<T>> + Send + 'static,
    {
        self.inner.check_live()?;
        let future = work(self.inner.clone());
        block_on(&self.inner.runtime, future)?
    }
}

impl TorrentStorage for ShareStorage {
    fn init(
        &mut self,
        _shared: &ManagedTorrentShared,
        _metadata: &TorrentMetadata,
    ) -> anyhow::Result<()> {
        // Make the folders and open (creating) every file, as the filesystem storage does, so a
        // share that cannot be reached stops the torrent now with a clear error.
        self.run(|inner| async move {
            let share = inner.share().await?;
            let mut made = std::collections::HashSet::new();
            for (file_id, slot) in inner.files.iter().enumerate() {
                if slot.padding {
                    continue;
                }
                if let Some(parent) = slot.path.parent()
                    && made.insert(parent.to_string())
                {
                    share
                        .create_dir_all(&parent)
                        .await
                        .map_err(|error| inner.storage_error(error))?;
                }
                inner.handle(file_id).await?;
            }
            Ok(())
        })
    }

    fn pread_exact(&self, file_id: usize, offset: u64, buf: &mut [u8]) -> anyhow::Result<()> {
        let len = buf.len() as u64;
        let slot = self.inner.files.get(file_id).context("no such file")?;
        let runs = {
            let mut pending = slot.pending.lock();
            if pending.overlaps(offset, len) {
                pending.take_overlapping(offset, len)
            } else {
                Vec::new()
            }
        };
        if !runs.is_empty() {
            let bytes: u64 = runs.iter().map(|(_, run)| run.len() as u64).sum();
            self.inner.buffered.fetch_sub(bytes, Ordering::AcqRel);
            // Answered from what is about to be written, if one run covers the whole read.
            let covered = runs
                .iter()
                .find(|(start, run)| *start <= offset && start + run.len() as u64 >= offset + len);
            if let Some((start, run)) = covered {
                let from = (offset - start) as usize;
                buf.copy_from_slice(&run[from..from + buf.len()]);
            }
            let answered = covered.is_some();
            // Written before the read returns: a checked piece is on the share, not in memory.
            self.run(move |inner| async move { inner.write_runs(file_id, runs).await })?;
            if answered {
                return Ok(());
            }
        }
        let read = self.run(move |inner| async move {
            let handle = inner.handle(file_id).await?;
            handle
                .read_at(offset, len as usize)
                .await
                .map_err(|error| inner.storage_error(error))
        })?;
        if read.len() != buf.len() {
            bail!(
                "read {} of {} bytes at {offset} from '{}'",
                read.len(),
                buf.len(),
                slot.path
            );
        }
        buf.copy_from_slice(&read);
        Ok(())
    }

    fn pwrite_all(&self, file_id: usize, offset: u64, buf: &[u8]) -> anyhow::Result<()> {
        self.inner.check_live()?;
        if buf.is_empty() {
            return Ok(());
        }
        let slot = self.inner.files.get(file_id).context("no such file")?;
        let mut to_write = Vec::new();
        {
            let mut pending = slot.pending.lock();
            if pending.overlaps(offset, buf.len() as u64) {
                // A block written again over buffered bytes: write what is buffered there first so
                // the order of writes is kept.
                let runs = pending.take_overlapping(offset, buf.len() as u64);
                let bytes: u64 = runs.iter().map(|(_, run)| run.len() as u64).sum();
                self.inner.buffered.fetch_sub(bytes, Ordering::AcqRel);
                to_write.extend(runs);
            }
        }
        if !to_write.is_empty() {
            self.run(move |inner| async move { inner.write_runs(file_id, to_write).await })?;
        }
        let ready = slot.pending.lock().add(offset, buf);
        let mut buffered = self
            .inner
            .buffered
            .fetch_add(buf.len() as u64, Ordering::AcqRel)
            + buf.len() as u64;
        if let Some((start, run)) = ready {
            buffered = self
                .inner
                .buffered
                .fetch_sub(run.len() as u64, Ordering::AcqRel)
                .saturating_sub(run.len() as u64);
            self.run(
                move |inner| async move { inner.write_runs(file_id, vec![(start, run)]).await },
            )?;
        }
        if buffered > MAX_BUFFERED {
            self.run(|inner| async move { inner.flush_all().await })?;
        }
        Ok(())
    }

    fn pwrite_all_vectored(
        &self,
        file_id: usize,
        offset: u64,
        bufs: [IoSlice<'_>; 2],
    ) -> anyhow::Result<usize> {
        let mut joined = Vec::with_capacity(bufs[0].len() + bufs[1].len());
        joined.extend_from_slice(&bufs[0]);
        joined.extend_from_slice(&bufs[1]);
        self.pwrite_all(file_id, offset, &joined)?;
        Ok(joined.len())
    }

    fn remove_file(&self, file_id: usize, filename: &Path) -> anyhow::Result<()> {
        let path = match self.inner.files.get(file_id) {
            Some(slot) => slot.path.clone(),
            None => {
                let mut path = self.inner.folder.clone();
                for part in filename.components() {
                    path = path.join(&part.as_os_str().to_string_lossy());
                }
                path
            }
        };
        if let Some(slot) = self.inner.files.get(file_id) {
            let runs = slot.pending.lock().take_all();
            let bytes: u64 = runs.iter().map(|(_, run)| run.len() as u64).sum();
            self.inner.buffered.fetch_sub(bytes, Ordering::AcqRel);
        }
        let inner = self.inner.clone();
        block_on(&self.inner.runtime, async move {
            // A file with a handle open on it is only deleted once that handle closes.
            if let Some(slot) = inner.files.get(file_id)
                && let Some(handle) = slot.handle.lock().await.take()
            {
                handle.close().await;
            }
            match library_storage().remove_file(path.clone()).await {
                Ok(()) => Ok(()),
                Err(error) if error.is_not_found() => Ok(()),
                Err(error) => Err(anyhow!("{error}")),
            }
        })?
    }

    fn remove_directory_if_empty(&self, path: &Path) -> anyhow::Result<()> {
        let mut dir = self.inner.folder.clone();
        for part in path.components() {
            dir = dir.join(&part.as_os_str().to_string_lossy());
        }
        block_on(&self.inner.runtime, async move {
            match library_storage().remove_dir(dir.clone()).await {
                Ok(()) => Ok(()),
                Err(error)
                    if error.is_not_found() || error.kind() == StorageErrorKind::NotEmpty =>
                {
                    Ok(())
                }
                Err(error) => Err(anyhow!("{error}")),
            }
        })?
    }

    fn ensure_file_length(&self, file_id: usize, length: u64) -> anyhow::Result<()> {
        let slot = self.inner.files.get(file_id).context("no such file")?;
        if slot.padding {
            return Ok(());
        }
        debug_assert_eq!(slot.len, length);
        self.run(move |inner| async move {
            let handle = inner.handle(file_id).await?;
            handle
                .set_len(length)
                .await
                .map_err(|error| inner.storage_error(error))
        })
    }

    fn take(&self) -> anyhow::Result<Box<dyn TorrentStorage>> {
        // Pausing: write what is buffered, release this copy's handles, then hand the same files
        // to the replacement, which opens its own when it is used.
        let flushed = self.run(|inner| async move { inner.flush_all().await });
        self.inner.retired.store(true, Ordering::Release);
        let retired = self.inner.clone();
        let _ = block_on(
            &self.inner.runtime,
            async move { retired.close_all().await },
        );
        let replacement = Inner {
            runtime: self.inner.runtime.clone(),
            folder: self.inner.folder.clone(),
            files: self
                .inner
                .files
                .iter()
                .map(|slot| FileSlot {
                    path: slot.path.clone(),
                    len: slot.len,
                    padding: slot.padding,
                    handle: tokio::sync::Mutex::new(None),
                    pending: Mutex::new(Pending::default()),
                })
                .collect(),
            buffered: AtomicU64::new(0),
            retired: AtomicBool::new(false),
        };
        if let Err(error) = flushed {
            tracing::warn!(
                folder = %self.inner.folder,
                error = %error,
                "Some downloaded blocks could not be written to the share before pausing; they will be downloaded again"
            );
        }
        Ok(Box::new(ShareStorage {
            inner: Arc::new(replacement),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contiguous_blocks_join_into_one_run() {
        let mut pending = Pending::default();
        assert!(pending.add(0, &[1; 10]).is_none());
        assert!(pending.add(10, &[2; 10]).is_none());
        assert!(pending.add(30, &[4; 10]).is_none());
        assert!(pending.add(20, &[3; 10]).is_none());
        assert_eq!(pending.runs.len(), 1);
        assert_eq!(pending.bytes, 40);
        let run = &pending.runs[&0];
        assert_eq!(&run[..10], &[1; 10]);
        assert_eq!(&run[20..30], &[3; 10]);
        assert_eq!(&run[30..], &[4; 10]);
    }

    #[test]
    fn an_empty_block_changes_nothing() {
        let mut pending = Pending::default();
        pending.add(100, &[5; 10]);
        assert!(pending.add(100, &[]).is_none());
        assert!(pending.add(110, &[]).is_none());
        assert_eq!(pending.runs.len(), 1);
        assert_eq!(pending.runs[&100], vec![5; 10]);
        assert_eq!(pending.bytes, 10);
    }

    #[test]
    fn a_long_run_is_handed_back_to_be_written() {
        let mut pending = Pending::default();
        let block = vec![7u8; 16 * 1024];
        let mut handed = None;
        for index in 0..(FLUSH_RUN / block.len()) {
            handed = pending.add((index * block.len()) as u64, &block);
        }
        let (start, run) = handed.expect("the run reached the flush size");
        assert_eq!(start, 0);
        assert_eq!(run.len(), FLUSH_RUN);
        assert_eq!(pending.bytes, 0);
        assert!(pending.runs.is_empty());
    }

    #[test]
    fn overlapping_runs_are_found_and_taken() {
        let mut pending = Pending::default();
        pending.add(0, &[1; 10]);
        pending.add(100, &[2; 10]);
        assert!(pending.overlaps(5, 2));
        assert!(!pending.overlaps(10, 90));
        assert!(pending.overlaps(95, 10));
        let taken = pending.take_overlapping(0, 200);
        assert_eq!(taken.len(), 2);
        assert_eq!(pending.bytes, 0);
    }
}
