//! Included inside the real backup module only in an isolated rehearsal source copy.
//! Real local-provider I/O with a deterministic failure on the second object write.
use super::*;
use std::sync::atomic::AtomicUsize;

pub(crate) fn inject_second_object_failure(service: &mut BackupService) -> Result<()> {
    let target = service
        .offline_restore
        .as_ref()
        .context("offline target required")?;
    let root = target.root.clone();
    service.offline_restore = Some(Arc::new(OfflineRestoreTarget {
        store: Arc::new(FailSecondObject {
            local: LocalStorageBackend::new(&root),
            writes: AtomicUsize::new(0),
        }),
        root,
        attempted: AtomicBool::new(false),
    }));
    Ok(())
}

struct FailSecondObject {
    local: LocalStorageBackend,
    writes: AtomicUsize,
}

#[async_trait]
impl BlobStore for FailSecondObject {
    fn backend(&self) -> graphql_orm_storage::StorageBackend {
        self.local.backend()
    }

    async fn put_blob(
        &self,
        key: &str,
        body: StorageByteStream,
        options: BlobPutOptions,
    ) -> Result<graphql_orm_storage::BlobWriteOutcome, graphql_orm_storage::StorageError> {
        self.local.put_blob(key, body, options).await
    }

    async fn put_blob_if_not_exists(
        &self,
        key: &str,
        body: StorageByteStream,
        options: BlobPutOptions,
    ) -> Result<Option<graphql_orm_storage::BlobWriteOutcome>, graphql_orm_storage::StorageError>
    {
        if self.writes.fetch_add(1, Ordering::AcqRel) == 1 {
            return Err(graphql_orm_storage::StorageError::UnsupportedOperation {
                operation: "rehearsal injected second-object write failure".into(),
            });
        }
        self.local.put_blob_if_not_exists(key, body, options).await
    }

    async fn get_blob(
        &self,
        key: &str,
    ) -> Result<graphql_orm_storage::BlobBody, graphql_orm_storage::StorageError> {
        self.local.get_blob(key).await
    }

    async fn blob_exists(&self, key: &str) -> Result<bool, graphql_orm_storage::StorageError> {
        self.local.blob_exists(key).await
    }

    async fn head_blob(
        &self,
        key: &str,
    ) -> Result<Option<graphql_orm_storage::BlobMetadata>, graphql_orm_storage::StorageError> {
        self.local.head_blob(key).await
    }

    async fn list_blobs_page(
        &self,
        prefix: &str,
        continuation: Option<String>,
        limit: usize,
    ) -> Result<graphql_orm_storage::BlobListPage, graphql_orm_storage::StorageError> {
        self.local
            .list_blobs_page(prefix, continuation, limit)
            .await
    }

    async fn delete_blob(&self, key: &str) -> Result<(), graphql_orm_storage::StorageError> {
        self.local.delete_blob(key).await
    }
}
