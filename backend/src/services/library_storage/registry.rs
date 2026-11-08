//! Finds the storage a path lives on.
//!
//! The local backend is always there. A remote backend (an SMB share) is registered under its
//! share's credential key once its sign-in is known; until then a path on that share fails with
//! [`StorageErrorKind::Unavailable`] instead of being treated as a missing file.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock, RwLock};

use tokio_util::sync::CancellationToken;

use super::{
    DirEntry, EntryMeta, IntoStoragePath, LocalStorage, OpenedRead, PlaceMode, PlaceOutcome,
    Storage, StorageError, StorageErrorKind, StorageKind, StoragePath, StorageResult, ToolInput,
    WalkOptions, WalkStream,
};

pub struct LibraryStorage {
    local: Arc<dyn Storage>,
    remotes: RwLock<HashMap<String, Arc<dyn Storage>>>,
}

impl Default for LibraryStorage {
    fn default() -> Self {
        Self {
            local: Arc::new(LocalStorage),
            remotes: RwLock::new(HashMap::new()),
        }
    }
}

/// The process-wide registry.
pub fn library_storage() -> &'static LibraryStorage {
    static REGISTRY: OnceLock<LibraryStorage> = OnceLock::new();
    REGISTRY.get_or_init(LibraryStorage::default)
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

    /// The storage `path` lives on.
    pub fn for_path(&self, path: &StoragePath) -> StorageResult<Arc<dyn Storage>> {
        match path.kind() {
            StorageKind::Local => Ok(self.local.clone()),
            StorageKind::Smb => {
                let key = path
                    .as_smb()
                    .map(|location| location.share_root().to_uri())
                    .ok_or_else(|| {
                        StorageError::new(
                            StorageErrorKind::Other,
                            format!("'{path}' is not a valid smb:// address"),
                        )
                    })?;
                self.remotes
                    .read()
                    .ok()
                    .and_then(|remotes| remotes.get(&key).cloned())
                    .ok_or_else(|| {
                        StorageError::new(
                            StorageErrorKind::Unavailable,
                            format!("The share '{key}' is not connected"),
                        )
                    })
            }
        }
    }

    pub async fn stat(&self, path: impl IntoStoragePath) -> StorageResult<EntryMeta> {
        let path = path.into_storage_path();
        self.for_path(&path)?.stat(&path).await
    }

    pub async fn lstat(&self, path: impl IntoStoragePath) -> StorageResult<EntryMeta> {
        let path = path.into_storage_path();
        self.for_path(&path)?.lstat(&path).await
    }

    pub async fn exists(&self, path: impl IntoStoragePath) -> StorageResult<bool> {
        let path = path.into_storage_path();
        self.for_path(&path)?.exists(&path).await
    }

    pub async fn list_dir(&self, dir: impl IntoStoragePath) -> StorageResult<Vec<DirEntry>> {
        let dir = dir.into_storage_path();
        self.for_path(&dir)?.list_dir(&dir).await
    }

    pub async fn is_dir_empty(&self, dir: impl IntoStoragePath) -> StorageResult<bool> {
        let dir = dir.into_storage_path();
        self.for_path(&dir)?.is_dir_empty(&dir).await
    }

    pub fn walk(
        &self,
        root: impl IntoStoragePath,
        options: WalkOptions,
        cancel: CancellationToken,
    ) -> StorageResult<WalkStream> {
        let root = root.into_storage_path();
        Ok(self.for_path(&root)?.walk(&root, options, cancel))
    }

    pub async fn open_read(&self, path: impl IntoStoragePath) -> StorageResult<OpenedRead> {
        let path = path.into_storage_path();
        self.for_path(&path)?.open_read(&path).await
    }

    pub async fn read_all(&self, path: impl IntoStoragePath) -> StorageResult<Vec<u8>> {
        let path = path.into_storage_path();
        self.for_path(&path)?.read_all(&path).await
    }

    pub async fn put_from_local(
        &self,
        src: &Path,
        dst: impl IntoStoragePath,
        mode: PlaceMode,
    ) -> StorageResult<PlaceOutcome> {
        let dst = dst.into_storage_path();
        self.for_path(&dst)?.put_from_local(src, &dst, mode).await
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
        self.for_path(&from)?.rename(&from, &to, mode).await
    }

    pub async fn remove_file(&self, path: impl IntoStoragePath) -> StorageResult<()> {
        let path = path.into_storage_path();
        self.for_path(&path)?.remove_file(&path).await
    }

    pub async fn create_dir_all(&self, dir: impl IntoStoragePath) -> StorageResult<()> {
        let dir = dir.into_storage_path();
        self.for_path(&dir)?.create_dir_all(&dir).await
    }

    pub async fn remove_dir(&self, dir: impl IntoStoragePath) -> StorageResult<()> {
        let dir = dir.into_storage_path();
        self.for_path(&dir)?.remove_dir(&dir).await
    }

    pub async fn tool_input(&self, path: impl IntoStoragePath) -> StorageResult<ToolInput> {
        let path = path.into_storage_path();
        self.for_path(&path)?.tool_input(&path).await
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
                .is_ok()
        );
        assert!(
            registry
                .for_path(&StoragePath::new("smb://nas/other/a.mkv"))
                .is_err()
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
