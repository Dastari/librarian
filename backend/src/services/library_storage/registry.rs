//! Finds the storage a path lives on.
//!
//! The local backend is always there. A remote backend (an SMB share) is registered under its
//! share's credential key once its sign-in is known; until then a path on that share fails with
//! [`StorageErrorKind::Unavailable`] instead of being treated as a missing file.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;

use tokio_util::sync::CancellationToken;

use super::own_writes::own_write;
use super::{
    DirEntry, EntryMeta, IntoStoragePath, LocalStorage, OpenedRead, PlaceMode, PlaceOutcome,
    SmbLocation, Storage, StorageError, StorageErrorKind, StorageKind, StoragePath, StorageResult,
    ToolInput, WalkOptions, WalkStream,
};

/// Connects a share on first use, from whatever the host keeps its sign-ins in.
#[async_trait]
pub trait ShareProvider: Send + Sync {
    async fn connect(&self, share: &SmbLocation) -> StorageResult<Arc<dyn Storage>>;
}

/// Hands out the loopback URL an external tool reads a remote file from (Q85).
#[async_trait]
pub trait ToolUrlIssuer: Send + Sync {
    async fn issue(&self, path: &StoragePath) -> StorageResult<String>;
}

/// How long a share that failed to connect is not tried again, so a scan of a library on a share
/// that is down does not wait out a connection timeout for every file.
const FAILURE_MEMORY: Duration = Duration::from_secs(10);

pub struct LibraryStorage {
    local: Arc<dyn Storage>,
    remotes: RwLock<HashMap<String, Arc<dyn Storage>>>,
    provider: RwLock<Option<Arc<dyn ShareProvider>>>,
    tool_urls: RwLock<Option<Arc<dyn ToolUrlIssuer>>>,
    connecting: tokio::sync::Mutex<()>,
    failures: Mutex<HashMap<String, (Instant, String)>>,
    /// How long a path on a share waits for the host to install its share provider. At start the
    /// scheduler can ask before the server has finished setting itself up.
    provider_grace: Duration,
}

impl Default for LibraryStorage {
    fn default() -> Self {
        Self {
            local: Arc::new(LocalStorage),
            remotes: RwLock::new(HashMap::new()),
            provider: RwLock::new(None),
            tool_urls: RwLock::new(None),
            connecting: tokio::sync::Mutex::new(()),
            failures: Mutex::new(HashMap::new()),
            provider_grace: Duration::ZERO,
        }
    }
}

/// The process-wide registry.
pub fn library_storage() -> &'static LibraryStorage {
    static REGISTRY: OnceLock<LibraryStorage> = OnceLock::new();
    REGISTRY.get_or_init(|| LibraryStorage {
        // Unit tests have no host to wait for.
        provider_grace: if cfg!(test) {
            Duration::ZERO
        } else {
            Duration::from_secs(10)
        },
        ..LibraryStorage::default()
    })
}

impl LibraryStorage {
    /// Make a remote storage answer for every path on its share. `share_root` is the share's
    /// `smb://host/share` address.
    pub fn register_remote(&self, share_root: &StoragePath, storage: Arc<dyn Storage>) {
        if let Some(location) = share_root.as_smb()
            && let Ok(mut remotes) = self.remotes.write()
        {
            remotes.insert(location.share_root().to_uri(), storage);
        }
    }

    /// Where shares are connected from when a path on one is first used.
    pub fn install_share_provider(&self, provider: Arc<dyn ShareProvider>) {
        if let Ok(mut slot) = self.provider.write() {
            *slot = Some(provider);
        }
    }

    /// Whether a share provider is installed yet.
    pub fn has_share_provider(&self) -> bool {
        self.provider.read().is_ok_and(|slot| slot.is_some())
    }

    pub fn install_tool_url_issuer(&self, issuer: Arc<dyn ToolUrlIssuer>) {
        if let Ok(mut slot) = self.tool_urls.write() {
            *slot = Some(issuer);
        }
    }

    /// Forget a share's connection (its sign-in changed): the next use connects afresh.
    pub fn forget_share(&self, share: &SmbLocation) {
        let key = share.share_root().to_uri();
        if let Ok(mut remotes) = self.remotes.write() {
            remotes.remove(&key);
        }
        if let Ok(mut failures) = self.failures.lock() {
            failures.remove(&key);
        }
    }

    async fn provider_within_grace(&self) -> Option<Arc<dyn ShareProvider>> {
        let deadline = Instant::now() + self.provider_grace;
        loop {
            if let Some(provider) = self.provider.read().ok().and_then(|slot| slot.clone()) {
                return Some(provider);
            }
            if Instant::now() >= deadline {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    fn registered(&self, key: &str) -> Option<Arc<dyn Storage>> {
        self.remotes.read().ok()?.get(key).cloned()
    }

    /// The storage `path` lives on, connecting its share if this is the first use.
    pub async fn for_path(&self, path: &StoragePath) -> StorageResult<Arc<dyn Storage>> {
        match path.kind() {
            StorageKind::Local => Ok(self.local.clone()),
            StorageKind::Smb => {
                let location = path.as_smb().ok_or_else(|| {
                    StorageError::new(
                        StorageErrorKind::Other,
                        format!("'{path}' is not a valid smb:// address"),
                    )
                })?;
                let key = location.share_root().to_uri();
                if let Some(storage) = self.registered(&key) {
                    return Ok(storage);
                }
                let provider = self.provider_within_grace().await;
                let Some(provider) = provider else {
                    return Err(StorageError::new(
                        StorageErrorKind::Unavailable,
                        format!("The share '{key}' is not connected"),
                    ));
                };
                let _one_at_a_time = self.connecting.lock().await;
                if let Some(storage) = self.registered(&key) {
                    return Ok(storage);
                }
                if let Some((when, message)) = self
                    .failures
                    .lock()
                    .ok()
                    .and_then(|failures| failures.get(&key).cloned())
                    && when.elapsed() < FAILURE_MEMORY
                {
                    return Err(StorageError::new(StorageErrorKind::Unavailable, message));
                }
                match provider.connect(&location.share_root()).await {
                    Ok(storage) => {
                        if let Ok(mut remotes) = self.remotes.write() {
                            remotes.insert(key.clone(), storage.clone());
                        }
                        if let Ok(mut failures) = self.failures.lock() {
                            failures.remove(&key);
                        }
                        Ok(storage)
                    }
                    Err(error) => {
                        if let Ok(mut failures) = self.failures.lock() {
                            failures.insert(key, (Instant::now(), error.to_string()));
                        }
                        Err(error)
                    }
                }
            }
        }
    }

    /// The share `path` is on, as an SMB storage.
    pub async fn smb_storage_for(
        &self,
        path: impl IntoStoragePath,
    ) -> StorageResult<super::SmbStorage> {
        let path = path.into_storage_path();
        let storage = self.for_path(&path).await?;
        storage.as_smb().cloned().ok_or_else(|| {
            StorageError::new(
                StorageErrorKind::Unsupported,
                format!("'{path}' is not on an SMB share"),
            )
        })
    }

    /// Forget every connected share on one server (its sign-in changed or it was removed).
    pub fn forget_server(&self, host: &str, port: Option<u16>) {
        let host = host.trim().to_ascii_lowercase();
        let matches = |key: &str| {
            super::SmbLocation::parse(key)
                .is_ok_and(|location| location.host == host && location.port == port)
        };
        if let Ok(mut remotes) = self.remotes.write() {
            remotes.retain(|key, _| !matches(key));
        }
        if let Ok(mut failures) = self.failures.lock() {
            failures.retain(|key, _| !matches(key));
        }
    }

    /// Copy a file from wherever it is to wherever `dst` is, leaving the source untouched: from
    /// this machine with `put_from_local`, within one share with a server-side copy, and between
    /// two storages by streaming the bytes through Librarian.
    pub async fn copy(
        &self,
        src: impl IntoStoragePath,
        dst: impl IntoStoragePath,
        mode: PlaceMode,
    ) -> StorageResult<PlaceOutcome> {
        let (src, dst) = (src.into_storage_path(), dst.into_storage_path());
        let _own = own_write([&dst]);
        let target = self.for_path(&dst).await?;
        if src.kind() == StorageKind::Local {
            return target
                .put_from_local(Path::new(src.as_str()), &dst, mode)
                .await;
        }
        let same_share = match (src.as_smb(), dst.as_smb()) {
            (Some(a), Some(b)) => a.share_root() == b.share_root(),
            _ => false,
        };
        if same_share {
            return target.copy_within(&src, &dst, mode).await;
        }
        let opened = self.for_path(&src).await?.open_read(&src).await?;
        target
            .put_from_reader(Box::new(opened.reader), &dst, mode)
            .await
    }

    /// Copy a file from any storage to a path on this machine (which must not exist yet).
    pub async fn download_to_local(
        &self,
        src: impl IntoStoragePath,
        dst: &Path,
    ) -> StorageResult<u64> {
        let src = src.into_storage_path();
        let mut opened = self.for_path(&src).await?.open_read(&src).await?;
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dst)
            .await?;
        let copied = tokio::io::copy(&mut opened.reader, &mut file).await;
        match copied {
            Ok(bytes) => {
                file.sync_all().await?;
                Ok(bytes)
            }
            Err(error) => {
                drop(file);
                let _ = tokio::fs::remove_file(dst).await;
                Err(error.into())
            }
        }
    }

    pub async fn stat(&self, path: impl IntoStoragePath) -> StorageResult<EntryMeta> {
        let path = path.into_storage_path();
        self.for_path(&path).await?.stat(&path).await
    }

    /// Which volume `path` is on (Q146): the share's root, or `local:<device>`.
    /// A path on a share is placed without connecting, so a share that is down still has one.
    pub async fn volume_key(&self, path: impl IntoStoragePath) -> StorageResult<String> {
        let path = path.into_storage_path();
        if let Some(location) = path.as_smb() {
            return Ok(location.share_root().to_uri());
        }
        self.for_path(&path).await?.volume_key(&path).await
    }

    /// The room on the volume holding `path`, quotas applied (Q146).
    pub async fn capacity(
        &self,
        path: impl IntoStoragePath,
    ) -> StorageResult<super::VolumeCapacity> {
        let path = path.into_storage_path();
        self.for_path(&path).await?.capacity(&path).await
    }

    pub async fn lstat(&self, path: impl IntoStoragePath) -> StorageResult<EntryMeta> {
        let path = path.into_storage_path();
        self.for_path(&path).await?.lstat(&path).await
    }

    pub async fn exists(&self, path: impl IntoStoragePath) -> StorageResult<bool> {
        let path = path.into_storage_path();
        self.for_path(&path).await?.exists(&path).await
    }

    pub async fn list_dir(&self, dir: impl IntoStoragePath) -> StorageResult<Vec<DirEntry>> {
        let dir = dir.into_storage_path();
        self.for_path(&dir).await?.list_dir(&dir).await
    }

    pub async fn is_dir_empty(&self, dir: impl IntoStoragePath) -> StorageResult<bool> {
        let dir = dir.into_storage_path();
        self.for_path(&dir).await?.is_dir_empty(&dir).await
    }

    pub async fn walk(
        &self,
        root: impl IntoStoragePath,
        options: WalkOptions,
        cancel: CancellationToken,
    ) -> StorageResult<WalkStream> {
        let root = root.into_storage_path();
        Ok(self.for_path(&root).await?.walk(&root, options, cancel))
    }

    pub async fn open_read(&self, path: impl IntoStoragePath) -> StorageResult<OpenedRead> {
        let path = path.into_storage_path();
        self.for_path(&path).await?.open_read(&path).await
    }

    pub async fn read_all(&self, path: impl IntoStoragePath) -> StorageResult<Vec<u8>> {
        let path = path.into_storage_path();
        self.for_path(&path).await?.read_all(&path).await
    }

    pub async fn put_from_local(
        &self,
        src: &Path,
        dst: impl IntoStoragePath,
        mode: PlaceMode,
    ) -> StorageResult<PlaceOutcome> {
        let dst = dst.into_storage_path();
        let _own = own_write([&dst]);
        self.for_path(&dst)
            .await?
            .put_from_local(src, &dst, mode)
            .await
    }

    /// Move within one storage. Moving between two storages is a copy and a delete the caller
    /// plans (`put_from_local` then `remove_file`): refused here so it is never done by accident.
    pub async fn rename(
        &self,
        from: impl IntoStoragePath,
        to: impl IntoStoragePath,
        mode: PlaceMode,
    ) -> StorageResult<PlaceOutcome> {
        let (from, to) = (from.into_storage_path(), to.into_storage_path());
        if from.kind() != to.kind() {
            return Err(StorageError::unsupported(
                "renaming between two different storages",
            ));
        }
        let _own = own_write([&from, &to]);
        self.for_path(&from).await?.rename(&from, &to, mode).await
    }

    pub async fn remove_file(&self, path: impl IntoStoragePath) -> StorageResult<()> {
        let path = path.into_storage_path();
        let _own = own_write([&path]);
        self.for_path(&path).await?.remove_file(&path).await
    }

    pub async fn create_dir_all(&self, dir: impl IntoStoragePath) -> StorageResult<()> {
        let dir = dir.into_storage_path();
        let _own = own_write([&dir]);
        self.for_path(&dir).await?.create_dir_all(&dir).await
    }

    pub async fn remove_dir(&self, dir: impl IntoStoragePath) -> StorageResult<()> {
        let dir = dir.into_storage_path();
        let _own = own_write([&dir]);
        self.for_path(&dir).await?.remove_dir(&dir).await
    }

    /// Be told about changes anywhere under the folder `dir` (Q115).
    pub async fn watch(&self, dir: impl IntoStoragePath) -> StorageResult<super::FolderWatch> {
        let dir = dir.into_storage_path();
        self.for_path(&dir).await?.watch(&dir).await
    }

    /// How an external program reads this file: by path where it can, else through a loopback
    /// URL from the tool URL service.
    pub async fn tool_input(&self, path: impl IntoStoragePath) -> StorageResult<ToolInput> {
        let path = path.into_storage_path();
        let storage = self.for_path(&path).await?;
        if !storage.capabilities().tools_need_url {
            return storage.tool_input(&path).await;
        }
        let issuer = self.tool_urls.read().ok().and_then(|slot| slot.clone());
        match issuer {
            Some(issuer) => Ok(ToolInput::Url(issuer.issue(&path).await?)),
            None => Err(StorageError::new(
                StorageErrorKind::Unavailable,
                "The tool URL service is not running, so tools cannot read this file",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn local_paths_route_to_the_local_backend() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.mkv"), b"abc").unwrap();
        let registry = LibraryStorage::default();
        assert_eq!(
            registry.stat(dir.path().join("a.mkv")).await.unwrap().size,
            3
        );
        assert!(registry.exists(dir.path().join("a.mkv")).await.unwrap());
        assert!(!registry.exists(dir.path().join("b.mkv")).await.unwrap());
    }

    #[tokio::test]
    async fn an_unregistered_share_is_unavailable_not_missing() {
        let registry = LibraryStorage::default();
        let error = registry
            .exists("smb://truenas.localdomain/media/Movies/a.mkv")
            .await
            .unwrap_err();
        assert_eq!(error.kind(), StorageErrorKind::Unavailable);
    }

    #[tokio::test]
    async fn a_registered_share_answers_for_all_its_paths() {
        let registry = LibraryStorage::default();
        registry.register_remote(&StoragePath::new("smb://nas/media"), Arc::new(LocalStorage));
        assert!(
            registry
                .for_path(&StoragePath::new("smb://NAS/Media/Movies/a.mkv"))
                .await
                .is_ok()
        );
        assert!(
            registry
                .for_path(&StoragePath::new("smb://nas/other/a.mkv"))
                .await
                .is_err()
        );
    }

    /// A provider that counts its connections and fails or succeeds as told.
    struct CountingProvider {
        connects: std::sync::atomic::AtomicUsize,
        fail: std::sync::atomic::AtomicBool,
    }

    impl CountingProvider {
        fn new(fail: bool) -> Arc<Self> {
            Arc::new(Self {
                connects: std::sync::atomic::AtomicUsize::new(0),
                fail: std::sync::atomic::AtomicBool::new(fail),
            })
        }

        fn connects(&self) -> usize {
            self.connects.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl ShareProvider for CountingProvider {
        async fn connect(&self, _share: &SmbLocation) -> StorageResult<Arc<dyn Storage>> {
            self.connects
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(20)).await;
            if self.fail.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(StorageError::new(
                    StorageErrorKind::Unavailable,
                    "Could not reach nas:445 (test)",
                ));
            }
            Ok(Arc::new(LocalStorage))
        }
    }

    /// Hand the registry a provider that is shared with the test.
    struct Shared(Arc<CountingProvider>);

    #[async_trait]
    impl ShareProvider for Shared {
        async fn connect(&self, share: &SmbLocation) -> StorageResult<Arc<dyn Storage>> {
            self.0.connect(share).await
        }
    }

    #[tokio::test]
    async fn a_share_connects_once_however_many_ask_at_once() {
        let provider = CountingProvider::new(false);
        let registry = Arc::new(LibraryStorage::default());
        registry.install_share_provider(Arc::new(Shared(provider.clone())));
        let path = StoragePath::new("smb://nas/media/Movies/a.mkv");
        let asks = (0..8).map(|_| {
            let (registry, path) = (registry.clone(), path.clone());
            tokio::spawn(async move { registry.for_path(&path).await.is_ok() })
        });
        for ask in futures::future::join_all(asks).await {
            assert!(ask.unwrap());
        }
        assert_eq!(provider.connects(), 1);
        // And later calls reuse the connection.
        registry.for_path(&path).await.unwrap();
        assert_eq!(provider.connects(), 1);
    }

    #[tokio::test]
    async fn a_share_that_failed_is_not_tried_again_at_once_and_is_when_forgotten() {
        let provider = CountingProvider::new(true);
        let registry = LibraryStorage::default();
        registry.install_share_provider(Arc::new(Shared(provider.clone())));
        let path = StoragePath::new("smb://nas/media/Movies/a.mkv");
        for _ in 0..5 {
            let error = registry.for_path(&path).await.err().expect("down");
            assert_eq!(error.kind(), StorageErrorKind::Unavailable);
            assert!(error.to_string().contains("Could not reach"), "{error}");
        }
        assert_eq!(
            provider.connects(),
            1,
            "a scan does not wait out a timeout per file"
        );
        // The sign-in changed, or the share is back: forgetting it lets the next call try.
        provider
            .fail
            .store(false, std::sync::atomic::Ordering::SeqCst);
        registry.forget_share(&path.as_smb().unwrap());
        registry.for_path(&path).await.expect("connected now");
        assert_eq!(provider.connects(), 2);
    }

    #[tokio::test]
    async fn a_path_on_a_share_waits_briefly_for_the_host_to_install_the_provider() {
        let registry = Arc::new(LibraryStorage {
            provider_grace: Duration::from_secs(3),
            ..LibraryStorage::default()
        });
        let provider = CountingProvider::new(false);
        let installer = {
            let registry = registry.clone();
            let provider = provider.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(250)).await;
                registry.install_share_provider(Arc::new(Shared(provider)));
            })
        };
        // Asked at start-up, before the server has set itself up: it waits, then connects.
        registry
            .for_path(&StoragePath::new("smb://nas/media/a.mkv"))
            .await
            .expect("connected once the provider arrived");
        installer.await.unwrap();
        assert_eq!(provider.connects(), 1);
        // With no host at all it gives up after the grace period, as unavailable.
        let alone = LibraryStorage {
            provider_grace: Duration::from_millis(300),
            ..LibraryStorage::default()
        };
        let error = alone
            .for_path(&StoragePath::new("smb://nas/media/a.mkv"))
            .await
            .err()
            .expect("no provider");
        assert_eq!(error.kind(), StorageErrorKind::Unavailable);
    }

    #[tokio::test]
    async fn a_failure_does_not_stick_to_a_different_share() {
        let provider = CountingProvider::new(true);
        let registry = LibraryStorage::default();
        registry.install_share_provider(Arc::new(Shared(provider.clone())));
        assert!(
            registry
                .for_path(&StoragePath::new("smb://nas/media/a.mkv"))
                .await
                .is_err()
        );
        provider
            .fail
            .store(false, std::sync::atomic::Ordering::SeqCst);
        assert!(
            registry
                .for_path(&StoragePath::new("smb://nas/backups/a.mkv"))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn a_file_on_a_share_needs_a_tool_url_and_a_local_one_does_not() {
        struct FixedIssuer;
        #[async_trait]
        impl ToolUrlIssuer for FixedIssuer {
            async fn issue(&self, path: &StoragePath) -> StorageResult<String> {
                Ok(format!(
                    "http://127.0.0.1:1/internal/tool/token/{}",
                    path.file_name().unwrap()
                ))
            }
        }
        struct NeedsUrl;
        #[async_trait]
        impl Storage for NeedsUrl {
            fn kind(&self) -> StorageKind {
                StorageKind::Smb
            }
            fn capabilities(&self) -> crate::services::library_storage::StorageCapabilities {
                crate::services::library_storage::StorageCapabilities {
                    tools_need_url: true,
                    ..LocalStorage.capabilities()
                }
            }
            async fn stat(&self, _: &StoragePath) -> StorageResult<EntryMeta> {
                Err(StorageError::unsupported("stat"))
            }
            async fn lstat(&self, _: &StoragePath) -> StorageResult<EntryMeta> {
                Err(StorageError::unsupported("lstat"))
            }
            async fn list_dir(&self, _: &StoragePath) -> StorageResult<Vec<DirEntry>> {
                Err(StorageError::unsupported("list"))
            }
            fn walk(&self, _: &StoragePath, _: WalkOptions, _: CancellationToken) -> WalkStream {
                unreachable!()
            }
            async fn open_read(&self, _: &StoragePath) -> StorageResult<OpenedRead> {
                Err(StorageError::unsupported("read"))
            }
            async fn put_from_local(
                &self,
                _: &Path,
                _: &StoragePath,
                _: PlaceMode,
            ) -> StorageResult<PlaceOutcome> {
                Err(StorageError::unsupported("put"))
            }
            async fn put_from_reader(
                &self,
                _: Box<dyn tokio::io::AsyncRead + Send + Unpin>,
                _: &StoragePath,
                _: PlaceMode,
            ) -> StorageResult<PlaceOutcome> {
                Err(StorageError::unsupported("put"))
            }
            async fn rename(
                &self,
                _: &StoragePath,
                _: &StoragePath,
                _: PlaceMode,
            ) -> StorageResult<PlaceOutcome> {
                Err(StorageError::unsupported("rename"))
            }
            async fn remove_file(&self, _: &StoragePath) -> StorageResult<()> {
                Err(StorageError::unsupported("remove"))
            }
            async fn create_dir_all(&self, _: &StoragePath) -> StorageResult<()> {
                Err(StorageError::unsupported("mkdir"))
            }
            async fn remove_dir(&self, _: &StoragePath) -> StorageResult<()> {
                Err(StorageError::unsupported("rmdir"))
            }
            async fn tool_input(&self, _: &StoragePath) -> StorageResult<ToolInput> {
                Err(StorageError::unsupported("tool input"))
            }
        }
        let registry = LibraryStorage::default();
        registry.register_remote(&StoragePath::new("smb://nas/media"), Arc::new(NeedsUrl));
        // No issuer yet: tools cannot read it, and the answer says so.
        let error = registry
            .tool_input("smb://nas/media/a.mkv")
            .await
            .unwrap_err();
        assert_eq!(error.kind(), StorageErrorKind::Unavailable);
        registry.install_tool_url_issuer(Arc::new(FixedIssuer));
        assert_eq!(
            registry
                .tool_input("smb://nas/media/Film.mkv")
                .await
                .unwrap(),
            ToolInput::Url("http://127.0.0.1:1/internal/tool/token/Film.mkv".to_string())
        );
        // A local file is passed by path, issuer or not.
        assert_eq!(
            registry.tool_input("/data/media/Film.mkv").await.unwrap(),
            ToolInput::Path(std::path::PathBuf::from("/data/media/Film.mkv"))
        );
    }

    #[tokio::test]
    async fn renaming_between_storages_is_refused() {
        let registry = LibraryStorage::default();
        let error = registry
            .rename("/data/a.mkv", "smb://nas/media/a.mkv", PlaceMode::NoClobber)
            .await
            .unwrap_err();
        assert_eq!(error.kind(), StorageErrorKind::Unsupported);
    }
}
