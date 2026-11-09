//! The storage on an SMB share, read and written by Librarian itself (the `smb2` crate), with no
//! operating-system mount.
//!
//! One `SmbStorage` is one share. It keeps a small pool of connections (each a signed-in session
//! with the share's tree connected) so a scan and a playback do not queue behind each other;
//! every connection pipelines its own requests.
//!
//! **Reconnecting.** A connection that is already known to be dead is replaced *before* an
//! operation is sent, which is safe for anything. An operation that fails *after* it was sent
//! because the connection dropped is different: a read, listing or stat is repeated once on a
//! fresh connection (repeating cannot change what was asked), while a write, rename or delete is
//! never repeated, because it may already have happened. That one fails as `Unavailable`, says so,
//! and the next call starts on a fresh connection.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use smb2::{ClientConfig, ErrorKind as SmbErrorKind, RenameOptions, SmbClient, Tree};
use tokio::io::AsyncReadExt;
use tokio::sync::{Mutex, MutexGuard, mpsc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::smb_reader::SmbReader;
use super::{
    DirEntry, EntryKind, EntryMeta, OpenedRead, PlaceMode, PlaceOutcome, SmbLocation, Storage,
    StorageCapabilities, StorageError, StorageErrorKind, StorageKind, StoragePath, StorageResult,
    ToolInput, WalkCandidate, WalkEntry, WalkOptions, WalkStream,
};

/// Connections kept per share.
const POOL_SIZE: usize = 3;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
/// Bytes sent per write request when copying a file in.
const WRITE_CHUNK: usize = 1024 * 1024;

/// A sign-in for a share. An empty user name is a guest session.
#[derive(Clone)]
pub struct SmbCredentials {
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for SmbCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmbCredentials")
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .finish()
    }
}

struct Conn {
    client: SmbClient,
    tree: Tree,
}

struct Share {
    root: SmbLocation,
    credentials: SmbCredentials,
    pool: Vec<Mutex<Option<Conn>>>,
    next: AtomicUsize,
}

/// One share, reached over SMB.
#[derive(Clone)]
pub struct SmbStorage(Arc<Share>);

/// How a failure of one call is to be treated.
enum Failure {
    /// The connection dropped under the call.
    ConnectionLost(smb2::Error),
    Other(StorageError),
}

fn classify(error: smb2::Error, what: &str) -> Failure {
    let kind = match error.kind() {
        SmbErrorKind::NotFound => StorageErrorKind::NotFound,
        SmbErrorKind::AlreadyExists => StorageErrorKind::AlreadyExists,
        SmbErrorKind::AccessDenied | SmbErrorKind::AuthRequired => {
            StorageErrorKind::PermissionDenied
        }
        SmbErrorKind::ConnectionLost | SmbErrorKind::SessionExpired | SmbErrorKind::TimedOut => {
            return Failure::ConnectionLost(error);
        }
        SmbErrorKind::Unsupported => StorageErrorKind::Unsupported,
        _ => match error.status() {
            Some(status) if status == smb2::types::status::NtStatus::DIRECTORY_NOT_EMPTY => {
                StorageErrorKind::NotEmpty
            }
            _ => StorageErrorKind::Other,
        },
    };
    Failure::Other(StorageError::new(kind, format!("{what}: {error}")))
}

fn lost(what: &str, error: &smb2::Error, repeated: bool) -> StorageError {
    let tail = if repeated {
        "the connection dropped again"
    } else {
        "the connection dropped, so the change was not repeated and may or may not have been made"
    };
    StorageError::new(
        StorageErrorKind::Unavailable,
        format!("{what}: {tail} ({error})"),
    )
}

fn file_kind(is_directory: bool, is_link: bool) -> EntryKind {
    if is_link {
        EntryKind::Symlink
    } else if is_directory {
        EntryKind::Directory
    } else {
        EntryKind::File
    }
}

/// Resolve `host` (saying so plainly when a name does not resolve from here) and sign in.
async fn connect_client(
    host: &str,
    port: Option<u16>,
    credentials: &SmbCredentials,
) -> StorageResult<SmbClient> {
    let port = port.unwrap_or(445);
    let address = tokio::net::lookup_host((host, port))
        .await
        .ok()
        .and_then(|mut addresses| addresses.next())
        .ok_or_else(|| {
            StorageError::new(
                StorageErrorKind::Unavailable,
                format!(
                    "The server name '{host}' does not resolve from the machine Librarian runs on. Try its IP address instead"
                ),
            )
        })?;
    let config = ClientConfig {
        addr: address.to_string(),
        timeout: CONNECT_TIMEOUT,
        username: credentials.username.clone(),
        password: credentials.password.clone(),
        // Dead connections are replaced by this module, so that a write is never replayed.
        auto_reconnect: false,
        ..Default::default()
    };
    SmbClient::connect(config).await.map_err(|error| {
        let shown = match error.kind() {
            SmbErrorKind::AuthRequired | SmbErrorKind::AccessDenied => {
                format!(
                    "The server refused the sign-in for '{}' ({error})",
                    credentials.username
                )
            }
            SmbErrorKind::ConnectionLost | SmbErrorKind::TimedOut => {
                format!("Could not reach {host}:{port} ({error})")
            }
            _ => format!("Could not sign in to {host}:{port} ({error})"),
        };
        let kind = match error.kind() {
            SmbErrorKind::AuthRequired | SmbErrorKind::AccessDenied => {
                StorageErrorKind::PermissionDenied
            }
            _ => StorageErrorKind::Unavailable,
        };
        StorageError::new(kind, shown)
    })
}

/// One share a server offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShareListing {
    pub name: String,
    pub comment: String,
}

/// Sign in to a server and ask it which disk shares it offers (srvsvc `NetShareEnumAll`, the
/// list Windows shows for a server). Administrative shares ending in `$` are left out. The outer
/// error is a failed sign-in; the inner one a server that signed the user in but would not list
/// its shares (some refuse it to users who may still open a share by name).
pub async fn list_server_shares(
    host: &str,
    port: Option<u16>,
    credentials: &SmbCredentials,
) -> StorageResult<StorageResult<Vec<ShareListing>>> {
    let mut client = connect_client(host, port, credentials).await?;
    let shares = match client.list_shares().await {
        Ok(shares) => shares,
        Err(error) => {
            let kind = match error.kind() {
                SmbErrorKind::AccessDenied | SmbErrorKind::AuthRequired => {
                    StorageErrorKind::PermissionDenied
                }
                SmbErrorKind::ConnectionLost | SmbErrorKind::TimedOut => {
                    StorageErrorKind::Unavailable
                }
                _ => StorageErrorKind::Other,
            };
            return Ok(Err(StorageError::new(
                kind,
                format!("{host} signed you in but would not list its shares ({error})"),
            )));
        }
    };
    let mut listed: Vec<ShareListing> = shares
        .into_iter()
        .filter(|share| !share.name.ends_with('$'))
        .map(|share| ShareListing {
            name: share.name,
            comment: share.comment,
        })
        .collect();
    listed.sort_by_key(|share| share.name.to_lowercase());
    Ok(Ok(listed))
}

impl SmbStorage {
    pub fn new(root: SmbLocation, credentials: SmbCredentials) -> Self {
        let root = root.share_root();
        Self(Arc::new(Share {
            root,
            credentials,
            pool: (0..POOL_SIZE).map(|_| Mutex::new(None)).collect(),
            next: AtomicUsize::new(0),
        }))
    }

    /// Sign in and connect the share, reporting why not in words a person can act on.
    pub async fn check(&self) -> StorageResult<()> {
        let guard = self.0.acquire().await?;
        drop(guard);
        Ok(())
    }

    pub fn share_root(&self) -> &SmbLocation {
        &self.0.root
    }
}

impl Share {
    /// Forget pooled connections that have dropped, so the next call connects afresh.
    async fn drop_dead(&self) {
        for slot in &self.pool {
            let mut guard = slot.lock().await;
            if guard
                .as_ref()
                .is_some_and(|conn| conn.client.is_disconnected())
            {
                *guard = None;
            }
        }
    }
}

impl Share {
    /// Resolve the host (a clear error when a name does not resolve here) and sign in.
    async fn connect(&self) -> StorageResult<Conn> {
        let host = &self.root.host;
        let mut client = connect_client(host, self.root.port, &self.credentials).await?;
        let tree = client
            .connect_share(&self.root.share)
            .await
            .map_err(|error| {
                let kind = if error.kind() == SmbErrorKind::NotFound {
                    StorageErrorKind::NotFound
                } else {
                    StorageErrorKind::Unavailable
                };
                StorageError::new(
                    kind,
                    format!(
                        "Signed in to {host}, but the share '{}' could not be opened ({error})",
                        self.root.share
                    ),
                )
            })?;
        Ok(Conn { client, tree })
    }

    /// A live connection: a free one if any, else wait for the next in turn. A connection known
    /// to be dead is replaced here, before anything is sent on it.
    async fn acquire(&self) -> StorageResult<MutexGuard<'_, Option<Conn>>> {
        let start = self.next.fetch_add(1, Ordering::Relaxed);
        let mut guard = None;
        for offset in 0..self.pool.len() {
            if let Ok(free) = self.pool[(start + offset) % self.pool.len()].try_lock() {
                guard = Some(free);
                break;
            }
        }
        let mut guard = match guard {
            Some(guard) => guard,
            None => self.pool[start % self.pool.len()].lock().await,
        };
        if guard
            .as_ref()
            .is_some_and(|conn| conn.client.is_disconnected())
        {
            *guard = None;
        }
        if guard.is_none() {
            *guard = Some(self.connect().await?);
        }
        Ok(guard)
    }

    fn relative(&self, path: &StoragePath) -> StorageResult<String> {
        let location = path.as_smb().ok_or_else(|| {
            StorageError::new(
                StorageErrorKind::Unsupported,
                format!("'{path}' is not an smb:// path"),
            )
        })?;
        if location.relative_to(&self.root).is_none() {
            return Err(StorageError::new(
                StorageErrorKind::Unsupported,
                format!("'{path}' is not on the share {}", self.root.to_uri()),
            ));
        }
        Ok(location.path)
    }
}

/// Run `$body` on a connection, repeating it once on a fresh one if the connection dropped under
/// it. Only for calls that can be repeated without changing what was asked.
macro_rules! repeatable {
    ($share:expr, $what:expr, |$client:ident, $tree:ident| $body:expr) => {{
        let mut attempt = 0;
        loop {
            let mut guard = $share.acquire().await?;
            let conn = guard.as_mut().expect("acquire connects");
            let ($client, $tree) = (&mut conn.client, &mut conn.tree);
            let outcome = $body.await;
            match outcome {
                Ok(value) => break Ok(value),
                Err(error) => match classify(error, $what) {
                    Failure::Other(error) => break Err(error),
                    Failure::ConnectionLost(error) => {
                        *guard = None;
                        attempt += 1;
                        if attempt >= 2 {
                            break Err(lost($what, &error, true));
                        }
                    }
                },
            }
        }
    }};
}

/// Run `$body` once. A connection that drops under it is discarded and the failure reported.
macro_rules! once {
    ($share:expr, $what:expr, |$client:ident, $tree:ident| $body:expr) => {{
        let mut guard = $share.acquire().await?;
        let conn = guard.as_mut().expect("acquire connects");
        let ($client, $tree) = (&mut conn.client, &mut conn.tree);
        let outcome = $body.await;
        match outcome {
            Ok(value) => Ok(value),
            Err(error) => match classify(error, $what) {
                Failure::Other(error) => Err(error),
                Failure::ConnectionLost(error) => {
                    *guard = None;
                    Err(lost($what, &error, false))
                }
            },
        }
    }};
}

impl Share {
    async fn stat(&self, path: &StoragePath) -> StorageResult<EntryMeta> {
        let rel = self.relative(path)?;
        if rel.is_empty() {
            // The share root: listing it proves it is there.
            repeatable!(self, "reading the share root", |client, tree| client
                .list_directory(tree, ""))?;
            return Ok(EntryMeta {
                kind: EntryKind::Directory,
                size: 0,
                modified: None,
                identity: None,
            });
        }
        let what = format!("reading '{path}'");
        let info = repeatable!(self, &what, |client, tree| client.stat(tree, &rel))?;
        Ok(EntryMeta {
            kind: file_kind(info.is_directory, info.is_link()),
            size: info.size,
            modified: info.modified.to_system_time(),
            identity: None,
        })
    }

    async fn list(&self, dir: &StoragePath) -> StorageResult<Vec<DirEntry>> {
        let rel = self.relative(dir)?;
        let what = format!("listing '{dir}'");
        let entries = repeatable!(self, &what, |client, tree| client
            .list_directory(tree, &rel))?;
        Ok(entries
            .into_iter()
            .filter(|entry| entry.name != "." && entry.name != "..")
            .map(|entry| {
                let kind = file_kind(entry.is_directory, entry.is_link());
                DirEntry {
                    meta: Some(EntryMeta {
                        kind,
                        size: entry.size,
                        modified: entry.modified.to_system_time(),
                        identity: None,
                    }),
                    name: entry.name,
                    kind,
                }
            })
            .collect())
    }
}

/// A name beside `rel` that no scan takes for media, for bytes on their way to `rel`.
fn temp_name(rel: &str) -> String {
    let (dir, name) = rel.rsplit_once('/').unwrap_or(("", rel));
    join_rel(dir, &format!("{name}.librarian-tmp-{}", Uuid::new_v4()))
}

impl Share {
    /// Rename a finished temporary copy to its real name (without replacing, for `NoClobber`),
    /// and remove it if that fails.
    async fn place_temp(
        &self,
        temp: &str,
        rel: &str,
        mode: PlaceMode,
        what: &str,
    ) -> StorageResult<PlaceOutcome> {
        let options = RenameOptions::new().replace_if_exists(mode == PlaceMode::Replace);
        let renamed = once!(self, what, |client, tree| client
            .rename_with(tree, temp, rel, options));
        match renamed {
            Ok(()) => Ok(PlaceOutcome::Placed),
            Err(error) => {
                let _ = once!(self, "removing a partial copy", |client, tree| client
                    .delete_file(tree, temp));
                if error.kind() == StorageErrorKind::AlreadyExists {
                    Ok(PlaceOutcome::TargetExists)
                } else {
                    Err(error)
                }
            }
        }
    }
}

fn join_rel(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

#[async_trait]
impl Storage for SmbStorage {
    fn kind(&self) -> StorageKind {
        StorageKind::Smb
    }

    fn as_smb(&self) -> Option<&SmbStorage> {
        Some(self)
    }

    fn capabilities(&self) -> StorageCapabilities {
        StorageCapabilities {
            atomic_rename: true,
            // A rename without replace and an exclusive create are each one request, decided
            // by the server.
            atomic_no_clobber: true,
            real_directories: true,
            hard_links: false,
            symlinks: false,
            change_notifications: true,
            file_identity: false,
            tools_need_url: true,
        }
    }

    async fn stat(&self, path: &StoragePath) -> StorageResult<EntryMeta> {
        self.0.stat(path).await
    }

    async fn lstat(&self, path: &StoragePath) -> StorageResult<EntryMeta> {
        self.0.stat(path).await
    }

    async fn list_dir(&self, dir: &StoragePath) -> StorageResult<Vec<DirEntry>> {
        self.0.list(dir).await
    }

    fn walk(
        &self,
        root: &StoragePath,
        options: WalkOptions,
        cancel: CancellationToken,
    ) -> WalkStream {
        let (tx, entries) = mpsc::channel(128);
        let share = self.0.clone();
        let root = root.clone();
        let task = tokio::spawn(async move {
            let name = root.file_name().unwrap_or_default();
            let top = WalkCandidate {
                depth: 0,
                name: &name,
                kind: EntryKind::Directory,
            };
            if options.prune.as_ref().is_some_and(|prune| prune(&top)) {
                return;
            }
            let report = |candidate: &WalkCandidate<'_>| {
                options
                    .report
                    .as_ref()
                    .is_none_or(|report| report(candidate))
            };
            if report(&top) {
                let entry = WalkEntry {
                    path: root.clone(),
                    depth: 0,
                    kind: EntryKind::Directory,
                    meta: None,
                };
                if tx.send(Ok(entry)).await.is_err() {
                    return;
                }
            }
            let mut stack = vec![(root.clone(), 0usize)];
            while let Some((dir, depth)) = stack.pop() {
                if cancel.is_cancelled() {
                    return;
                }
                let listing = match share.list(&dir).await {
                    Ok(listing) => listing,
                    Err(error) => {
                        let unreachable = error.kind() == StorageErrorKind::Unavailable;
                        if tx.send(Err(error)).await.is_err() || unreachable {
                            // A share that has gone away will not answer the rest either.
                            return;
                        }
                        continue;
                    }
                };
                for entry in listing {
                    let candidate = WalkCandidate {
                        depth: depth + 1,
                        name: &entry.name,
                        kind: entry.kind,
                    };
                    if options
                        .prune
                        .as_ref()
                        .is_some_and(|prune| prune(&candidate))
                    {
                        continue;
                    }
                    let path = dir.join(&entry.name);
                    if report(&candidate) {
                        let reported = WalkEntry {
                            path: path.clone(),
                            depth: depth + 1,
                            kind: entry.kind,
                            meta: entry.meta.clone(),
                        };
                        if tx.send(Ok(reported)).await.is_err() {
                            return;
                        }
                    }
                    if entry.kind == EntryKind::Directory {
                        stack.push((path, depth + 1));
                    }
                }
            }
        });
        WalkStream { entries, task }
    }

    async fn open_read(&self, path: &StoragePath) -> StorageResult<OpenedRead> {
        let rel = self.0.relative(path)?;
        let what = format!("opening '{path}'");
        let reader = repeatable!(self.0, &what, |client, tree| client
            .open_file_reader(tree, &rel))?;
        let size = reader.size();
        let modified = reader.info().modified.to_system_time();
        Ok(OpenedRead {
            meta: EntryMeta {
                kind: EntryKind::File,
                size,
                modified,
                identity: None,
            },
            reader: Box::new(SmbReader::new(reader)),
        })
    }

    async fn put_from_local(
        &self,
        src: &Path,
        dst: &StoragePath,
        mode: PlaceMode,
    ) -> StorageResult<PlaceOutcome> {
        if mode == PlaceMode::NoClobber && self.0.stat(dst).await.is_ok() {
            return Ok(PlaceOutcome::TargetExists);
        }
        let source = tokio::fs::File::open(src).await?;
        self.put_from_reader(Box::new(source), dst, mode).await
    }

    async fn put_from_reader(
        &self,
        mut source: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
        dst: &StoragePath,
        mode: PlaceMode,
    ) -> StorageResult<PlaceOutcome> {
        let rel = self.0.relative(dst)?;
        if mode == PlaceMode::NoClobber && self.0.stat(dst).await.is_ok() {
            return Ok(PlaceOutcome::TargetExists);
        }
        // The bytes go to a name no scan will take for media, then the finished file is renamed
        // into place: a half-copied file never has its real name, and the rename without replace
        // is what makes "never overwrite" a decision of the server.
        let temp = temp_name(&rel);
        let what = format!("copying to '{dst}'");
        let copied: StorageResult<()> = async {
            let mut guard = self.0.acquire().await?;
            let conn = guard.as_mut().expect("acquire connects");
            let result: Result<(), smb2::Error> = async {
                let mut writer = conn
                    .client
                    .create_file_writer_exclusive(&conn.tree, &temp)
                    .await?;
                let mut buffer = vec![0u8; WRITE_CHUNK];
                loop {
                    let read = source.read(&mut buffer).await.map_err(smb2::Error::Io)?;
                    if read == 0 {
                        break;
                    }
                    writer.write_chunk(&buffer[..read]).await?;
                }
                writer.finish().await?;
                Ok(())
            }
            .await;
            match result {
                Ok(()) => Ok(()),
                Err(error) => match classify(error, &what) {
                    Failure::Other(error) => Err(error),
                    Failure::ConnectionLost(error) => {
                        *guard = None;
                        Err(lost(&what, &error, false))
                    }
                },
            }
        }
        .await;
        if let Err(error) = copied {
            // Best effort: the temporary name is never a library file.
            let _ = once!(self.0, "removing a partial copy", |client, tree| client
                .delete_file(tree, &temp));
            return Err(error);
        }
        self.0.place_temp(&temp, &rel, mode, &what).await
    }

    async fn copy_within(
        &self,
        from: &StoragePath,
        to: &StoragePath,
        mode: PlaceMode,
    ) -> StorageResult<PlaceOutcome> {
        let (from_rel, to_rel) = (self.0.relative(from)?, self.0.relative(to)?);
        if mode == PlaceMode::NoClobber && self.0.stat(to).await.is_ok() {
            return Ok(PlaceOutcome::TargetExists);
        }
        // FSCTL_SRV_COPYCHUNK: the server copies the bytes itself, so none cross the network.
        let temp = temp_name(&to_rel);
        let what = format!("copying '{from}' to '{to}' on the server");
        let copied = once!(self.0, &what, |client, tree| client
            .server_side_copy_file(tree, &from_rel, &temp));
        match copied {
            Ok(_) => self.0.place_temp(&temp, &to_rel, mode, &what).await,
            Err(error) => {
                let _ = once!(self.0, "removing a partial copy", |client, tree| client
                    .delete_file(tree, &temp));
                if error.kind() == StorageErrorKind::Unsupported
                    || error.kind() == StorageErrorKind::Other
                {
                    // A server without server-side copy: stream it through Librarian instead.
                    tracing::debug!(
                        from = %from,
                        to = %to,
                        error = %error,
                        "Server-side copy refused; copying through Librarian"
                    );
                    let opened = self.open_read(from).await?;
                    self.put_from_reader(Box::new(opened.reader), to, mode)
                        .await
                } else {
                    Err(error)
                }
            }
        }
    }

    async fn rename(
        &self,
        from: &StoragePath,
        to: &StoragePath,
        mode: PlaceMode,
    ) -> StorageResult<PlaceOutcome> {
        let (from_rel, to_rel) = (self.0.relative(from)?, self.0.relative(to)?);
        let what = format!("renaming '{from}' to '{to}'");
        let options = RenameOptions::new().replace_if_exists(mode == PlaceMode::Replace);
        match once!(self.0, &what, |client, tree| client
            .rename_with(tree, &from_rel, &to_rel, options))
        {
            Ok(()) => Ok(PlaceOutcome::Placed),
            Err(error) if error.kind() == StorageErrorKind::AlreadyExists => {
                Ok(PlaceOutcome::TargetExists)
            }
            Err(error) => Err(error),
        }
    }

    async fn remove_file(&self, path: &StoragePath) -> StorageResult<()> {
        let rel = self.0.relative(path)?;
        let what = format!("deleting '{path}'");
        once!(self.0, &what, |client, tree| client.delete_file(tree, &rel))
    }

    async fn create_dir_all(&self, dir: &StoragePath) -> StorageResult<()> {
        let rel = self.0.relative(dir)?;
        let what = format!("creating the folder '{dir}'");
        let mut current = String::new();
        for component in rel.split('/').filter(|part| !part.is_empty()) {
            current = join_rel(&current, component);
            match once!(self.0, &what, |client, tree| client
                .create_directory(tree, &current))
            {
                Ok(()) => {}
                Err(error) if error.kind() == StorageErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    async fn remove_dir(&self, dir: &StoragePath) -> StorageResult<()> {
        let rel = self.0.relative(dir)?;
        let what = format!("removing the folder '{dir}'");
        once!(self.0, &what, |client, tree| client
            .delete_directory(tree, &rel))
    }

    async fn tool_input(&self, path: &StoragePath) -> StorageResult<ToolInput> {
        // Issued by the tool URL service; this storage only says that one is needed.
        let _ = self.0.relative(path)?;
        Err(StorageError::new(
            StorageErrorKind::Unsupported,
            "A file on a share is given to tools through the tool URL service",
        ))
    }
}

impl SmbStorage {
    /// Open a file for positioned reads and writes (creating it when `create`), with its size.
    /// The handle lives on its own clone of a pooled connection; see [`SmbFile`].
    pub async fn open_file(
        &self,
        path: &StoragePath,
        create: bool,
    ) -> StorageResult<(super::SmbFile, u64)> {
        let rel = self.0.relative(path)?;
        let mut attempt = 0;
        loop {
            let (conn, tree) = {
                let mut guard = self.0.acquire().await?;
                let held = guard.as_mut().expect("acquire connects");
                match held.client.connection_for(&held.tree) {
                    Ok(conn) => (conn, held.tree.clone()),
                    Err(error) => {
                        *guard = None;
                        return Err(StorageError::new(
                            StorageErrorKind::Unavailable,
                            format!("The connection to the share is not usable ({error})"),
                        ));
                    }
                }
            };
            match super::SmbFile::open(conn, tree, &rel, create).await {
                // Opening is safe to repeat once: it either finds or makes the same file.
                Err(error) if error.is_unavailable() && attempt == 0 => {
                    attempt += 1;
                    self.0.drop_dead().await;
                }
                other => return other,
            }
        }
    }

    /// The share's free and total bytes, where the server says.
    pub async fn space(&self) -> StorageResult<(u64, u64)> {
        let info = repeatable!(self.0, "reading the share's free space", |client, tree| {
            client.fs_info(tree)
        })?;
        Ok((info.free_bytes, info.total_bytes))
    }
}
