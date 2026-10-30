//! Backup service: full logical database backup/restore (via `graphql-orm-backup`, adapted
//! onto `graphql-orm`'s [`GraphqlOrmBackupRuntime`]) plus stored-object backup.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use bytes::Bytes;
use futures::StreamExt;
use graphql_orm::graphql::filters::StringFilter;
use graphql_orm::graphql::orm::{
    BackupRow as OrmBackupRow, BackupValue as OrmBackupValue, BackupValueKind, DefaultBackend,
    EntityBackupDescriptor, GraphqlOrmBackupRuntime, IntrospectionBackend,
    RestoreContext as OrmRestoreContext, RestoreMode as OrmRestoreMode, SchemaModel,
};
use graphql_orm_backup::{
    BackupChangeExport, BackupError, BackupKind, BackupObjectIndex, BackupObjectRef,
    BackupRepository, BackupSnapshotManifest, BackupTableExport, FullBackupRequest,
    GraphqlOrmBackupAdapter, GraphqlOrmBackupSchema, LocalBackupRepository, RestoreContext,
    RestoreMode, RestoreObjectSink, create_full_backup as run_create_full_backup, load_manifest,
    restore_objects as run_restore_objects, restore_snapshot as run_restore_snapshot,
    snapshot_manifest_key, verify_manifest_and_objects,
};
use graphql_orm_storage::{
    BlobPutOptions, BlobStore, LocalStorageBackend, StorageByteStream, StorageNamespace,
    StoredObject, build_storage_key, validate_blob_key,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::Database;
use crate::graphql::entities::{StorageObject, StorageObjectWhereInput};
use crate::services::database::{VerifiedSchemaBackup, entity_metadata};
use crate::services::manager::{Service, ServiceHealth};
use crate::services::storage::{ObjectStorageService, ObjectStorageServiceConfig};

#[path = "backup_schema.rs"]
mod schema_identity;
use schema_identity::{SchemaIdentity, canonical_backend};

const INCREMENTAL_BACKUP_UNAVAILABLE_REASON: &str =
    "Incremental backups require the graphql-orm change-journal feature, which is not enabled";

#[derive(Debug, Clone)]
pub struct BackupServiceConfig {
    pub repository_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct BackupCapabilities {
    pub full_database_backup_available: bool,
    pub object_backup_available: bool,
    pub restore_available: bool,
    pub incremental_backup_available: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BackupSnapshotSummary {
    pub snapshot_id: String,
    pub created_at: i64,
    pub kind: String,
    pub app_id: String,
    pub app_version: String,
    pub table_count: i32,
    pub object_count: i32,
    pub total_object_bytes: i64,
    pub manifest_key: String,
}

/// Opaque proof that a Librarian full/synthetic-full snapshot and all of its
/// referenced payload checksums were verified.
pub(crate) struct VerifiedFullBackup {
    snapshot_id: String,
}

impl VerifiedFullBackup {
    pub(crate) fn snapshot_id(&self) -> &str {
        &self.snapshot_id
    }
}

pub struct BackupService {
    db: Database,
    storage: Arc<ObjectStorageService>,
    repository: LocalBackupRepository,
    app_id: String,
    app_version: String,
    offline_restore: Option<Arc<OfflineRestoreTarget>>,
    // External backup manifests have no domain entity. Coalesced revisions
    // invalidate administrator snapshots without fabricating entity events.
    changes: tokio::sync::watch::Sender<u64>,
}

impl BackupService {
    pub fn new(
        db: Database,
        storage: Arc<ObjectStorageService>,
        config: BackupServiceConfig,
    ) -> Self {
        Self {
            db,
            storage,
            repository: LocalBackupRepository::new(config.repository_path),
            app_id: "librarian".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            offline_restore: None,
            changes: tokio::sync::watch::channel(0).0,
        }
    }

    /// Constructs an unserved recovery service for an exclusively owned staging database and
    /// local object root. The caller must keep both targets offline, including all workers,
    /// until restore completes, and publish them together only after success. This service
    /// cannot be started by the services manager. On failure discard the staging targets;
    /// neither the ORM nor object storage provides cross-store rollback.
    ///
    /// `StorageService` does not expose original-key writes in the pinned storage release.
    /// Construct both storage views from the same config so reads and restore writes agree.
    pub fn new_for_offline_restore(
        db: Database,
        storage_config: ObjectStorageServiceConfig,
        config: BackupServiceConfig,
    ) -> Result<Self> {
        let storage = Arc::new(ObjectStorageService::new(storage_config.clone())?);
        let mut service = Self::new(db, storage, config);
        service.offline_restore = Some(Arc::new(OfflineRestoreTarget {
            store: Arc::new(LocalStorageBackend::new(&storage_config.local_path)),
            root: storage_config.local_path,
            attempted: AtomicBool::new(false),
        }));
        Ok(service)
    }

    pub fn capabilities(&self) -> BackupCapabilities {
        let restore_available = self
            .offline_restore
            .as_ref()
            .is_some_and(|target| !target.attempted.load(Ordering::Acquire));
        BackupCapabilities {
            full_database_backup_available: true,
            object_backup_available: true,
            restore_available,
            incremental_backup_available: false,
            reason: Some(if restore_available {
                INCREMENTAL_BACKUP_UNAVAILABLE_REASON.to_string()
            } else if self.offline_restore.is_some() {
                format!(
                    "Recovery target already used; discard it before another attempt. {INCREMENTAL_BACKUP_UNAVAILABLE_REASON}"
                )
            } else {
                format!(
                    "Restore requires an offline staging target. {INCREMENTAL_BACKUP_UNAVAILABLE_REASON}"
                )
            }),
        }
    }

    pub async fn list_snapshots(&self) -> Result<Vec<BackupSnapshotSummary>> {
        let keys = self
            .repository
            .list_blobs("snapshots")
            .await
            .context("Failed to list backup snapshots")?;
        let mut summaries = Vec::new();

        for key in keys
            .into_iter()
            .filter(|key| key.ends_with("/manifest.json"))
        {
            let bytes = self
                .repository
                .get_blob(&key)
                .await
                .with_context(|| format!("Failed to read backup manifest '{}'", key))?;
            let manifest: BackupSnapshotManifest = serde_json::from_slice(&bytes)
                .with_context(|| format!("Failed to parse backup manifest '{}'", key))?;
            summaries.push(summary_from_manifest(manifest, key)?);
        }

        summaries.sort_by_key(|summary| Reverse(summary.created_at));
        Ok(summaries)
    }

    pub async fn create_full_backup(&self) -> Result<Option<BackupSnapshotSummary>> {
        let adapter =
            LibrarianBackupDatabaseAdapter::new(self.db.clone(), self.app_version.clone());
        let objects = self.object_index();
        let repository = self.repository.clone();
        let request = FullBackupRequest {
            snapshot_id: Uuid::new_v4(),
            created_at: current_unix_timestamp(),
            app_id: self.app_id.clone(),
            app_version: self.app_version.clone(),
        };

        // Same rustc limitation documented on `verify_snapshot` below: `graphql-orm-backup`'s
        // concurrent object-upload path builds `Stream::map` closures over `&dyn
        // BackupObjectIndex` whose opaque future type cannot be proven `Send` in a
        // higher-ranked-lifetime context, so awaiting it directly (or via `tokio::spawn`)
        // fails to compile. Running it on a blocking-pool thread via `Handle::block_on`
        // sidesteps this because `block_on` has no `Send` bound on its future.
        let handle = tokio::runtime::Handle::current();
        let result = tokio::task::spawn_blocking(move || {
            handle.block_on(run_create_full_backup(
                &repository,
                &adapter,
                &objects,
                request,
            ))
        })
        .await
        .context("Backup creation task panicked")?
        .context("Failed to create full backup")?;
        let manifest_key = snapshot_manifest_key(result.manifest.snapshot_id);

        let summary = summary_from_manifest(result.manifest, manifest_key)?;
        self.changes
            .send_modify(|revision| *revision = revision.wrapping_add(1));
        Ok(Some(summary))
    }

    pub(crate) fn subscribe_changes(&self) -> tokio::sync::watch::Receiver<u64> {
        self.changes.subscribe()
    }

    /// Restores managed metadata and bytes into an offline, empty recovery target.
    /// An ordinary live service fails closed. Each recovery service permits one attempt;
    /// a failed or cancelled attempt must never be published or retried in place.
    pub async fn restore_full_backup(&self, snapshot_id: &str) -> Result<()> {
        let target = self.offline_restore.clone().context(
            "Restore requires an offline staging target; stop application access and workers and use new_for_offline_restore",
        )?;
        let snapshot_id = Uuid::parse_str(snapshot_id)
            .with_context(|| format!("Invalid backup snapshot id '{}'", snapshot_id))?;
        if target.attempted.swap(true, Ordering::AcqRel) {
            anyhow::bail!(
                "Recovery target already used; discard it and start with a fresh empty target"
            );
        }
        let adapter =
            LibrarianBackupDatabaseAdapter::new(self.db.clone(), self.app_version.clone());
        let repository = self.repository.clone();
        let storage = self.storage.clone();

        // Preserve the existing backup task boundary for the complete restore operation.
        // Dropping the request does not interrupt an active apply or release its one-shot guard.
        let handle = tokio::runtime::Handle::current();
        tokio::task::spawn_blocking(move || {
            handle.block_on(async move {
                ensure_empty_object_root(&target.root).await?;
                let page = target.store.list_blobs_page("", None, 1).await?;
                if !page.keys.is_empty() || page.next_continuation.is_some() {
                    anyhow::bail!("Object restore target is not empty");
                }
                let manifest = load_manifest(&repository, snapshot_id).await?;
                validate_full_restore_manifest(&manifest, snapshot_id)?;

                // Dry run checks every payload, chain and schema before either store changes.
                run_restore_snapshot(&repository, &adapter, snapshot_id, RestoreContext::dry_run())
                    .await?;
                if !adapter.restore_target_is_empty().await? {
                    anyhow::bail!("Restore database target is not empty");
                }
                // Persist the staging boundary before apply. A second recovery instance or
                // a restarted process must reject an unfinished target, even if no object
                // write completed. Leave this marker behind on every failure after claim.
                tokio::fs::create_dir_all(&target.root).await?;
                let marker = target.root.join(".librarian-restore-in-progress");
                let claim = tokio::fs::OpenOptions::new().write(true).create_new(true)
                    .open(&marker).await.context("Failed to claim offline object restore target")?;
                drop(claim);
                let result = run_restore_snapshot(
                    &repository, &adapter, snapshot_id, RestoreContext::empty_database(),
                ).await?;
                if result.manifest_chain_len != 1 || result.incremental_change_count != 0
                    || result.full_table_count != manifest.database.table_count
                    || result.full_row_count != manifest.database.row_count
                {
                    anyhow::bail!("Database restore result disagrees with the full snapshot manifest");
                }
                let objects = restored_storage_objects(&adapter.db, &storage, &manifest).await?;
                let sink = LibrarianRestoreObjectSink { store: target.store.clone() };
                run_restore_objects(&repository, &manifest, &sink).await?;
                // Read back through the exact abstraction used by artwork/content consumers.
                // Provider write outcomes alone are insufficient proof that bytes are usable.
                for object in &objects {
                    verify_restored_object(&storage, object).await?;
                }
                tokio::fs::remove_file(marker).await
                    .context("Failed to finalize offline object restore target")?;
                Ok::<(), anyhow::Error>(())
            })
        })
        .await
        .context("Backup restore task panicked; discard the offline staging target")?
        .map_err(|error| anyhow::anyhow!(
            "Failed to restore full backup; discard the offline staging target (rows or objects may be partially restored): {error:#}"
        ))?;
        Ok(())
    }

    pub async fn verify_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let snapshot_id = Uuid::parse_str(snapshot_id)
            .with_context(|| format!("Invalid backup snapshot id '{}'", snapshot_id))?;
        let key = snapshot_manifest_key(snapshot_id);
        let bytes = self
            .repository
            .get_blob(&key)
            .await
            .with_context(|| format!("Failed to read backup manifest '{}'", key))?;
        let manifest: BackupSnapshotManifest = serde_json::from_slice(&bytes)
            .with_context(|| format!("Failed to parse backup manifest '{}'", key))?;
        // `verify_manifest_and_objects` builds internal `Stream::map` closures over borrowed
        // data whose opaque future type cannot be proven `Send` in a higher-ranked-lifetime
        // context (a rustc limitation with async fns called from iterator/stream combinators).
        // Awaiting it directly (or via `tokio::spawn`, which also requires `Send`) fails to
        // compile. Running it to completion on a blocking-pool thread via `Handle::block_on`
        // sidesteps the issue because `block_on` has no `Send` bound on its future.
        let repository = self.repository.clone();
        let handle = tokio::runtime::Handle::current();
        tokio::task::spawn_blocking(move || {
            handle.block_on(verify_manifest_and_objects(&repository, &manifest))
        })
        .await
        .context("Backup verification task panicked")?
        .context("Backup verification failed")?;
        Ok(())
    }

    pub fn object_index(&self) -> LibrarianBackupObjectIndex {
        LibrarianBackupObjectIndex {
            db: self.db.clone(),
            storage: self.storage.clone(),
        }
    }
}

/// Verify that a full logical snapshot is intact and belongs to the exact live schema that an
/// explicitly reviewed migration will replace. The returned proof is intentionally opaque and is
/// the only value accepted by `apply_reviewed_schema_plan`.
pub(crate) async fn verify_schema_migration_backup(
    repository_path: PathBuf,
    snapshot_id: &str,
    expected_source_schema_hash: &str,
) -> Result<VerifiedSchemaBackup> {
    let (manifest, _) = verify_full_backup_manifest(repository_path, snapshot_id).await?;
    let identity = SchemaIdentity::parse(&manifest.graphql_orm_schema_hash)?;
    if identity.physical_schema_hash != expected_source_schema_hash {
        anyhow::bail!(
            "Backup physical schema hash {} does not match current source schema {}",
            identity.physical_schema_hash,
            expected_source_schema_hash
        );
    }

    tracing::info!(
        service = "backup",
        backup_snapshot_id = %snapshot_id,
        schema_hash = %expected_source_schema_hash,
        "Verified full backup for reviewed schema migration"
    );
    Ok(VerifiedSchemaBackup::new(
        snapshot_id.to_string(),
        expected_source_schema_hash.to_string(),
    ))
}

pub(crate) async fn verify_full_backup(
    repository_path: PathBuf,
    snapshot_id: &str,
) -> Result<VerifiedFullBackup> {
    let (_, proof) = verify_full_backup_manifest(repository_path, snapshot_id).await?;
    Ok(proof)
}

async fn verify_full_backup_manifest(
    repository_path: PathBuf,
    snapshot_id: &str,
) -> Result<(BackupSnapshotManifest, VerifiedFullBackup)> {
    let snapshot_uuid = Uuid::parse_str(snapshot_id).context("Invalid backup snapshot id")?;
    let repository = LocalBackupRepository::new(repository_path);
    let manifest_key = snapshot_manifest_key(snapshot_uuid);
    let bytes = repository
        .get_blob(&manifest_key)
        .await
        .context("Failed to read schema-migration backup manifest")?;
    let manifest: BackupSnapshotManifest = serde_json::from_slice(&bytes)
        .context("Failed to parse schema-migration backup manifest")?;

    if manifest.snapshot_id != snapshot_uuid {
        anyhow::bail!("Backup manifest snapshot id does not match the requested snapshot");
    }
    if manifest.app_id != "librarian" {
        anyhow::bail!("Backup snapshot was not produced by Librarian");
    }
    // Accept only the two known spellings of this same backend. Verify the
    // original manifest bytes/checksum below; never rewrite signed metadata.
    canonical_backend(&manifest.database_backend)?;
    if !matches!(
        manifest.backup_kind,
        BackupKind::Full | BackupKind::SyntheticFull
    ) {
        anyhow::bail!("Operation requires a full or synthetic-full backup snapshot");
    }

    let handle = tokio::runtime::Handle::current();
    let repository_for_verify = repository.clone();
    let manifest_for_verify = manifest.clone();
    tokio::task::spawn_blocking(move || {
        handle.block_on(verify_manifest_and_objects(
            &repository_for_verify,
            &manifest_for_verify,
        ))
    })
    .await
    .context("Full backup verification task panicked")?
    .context("Full backup verification failed")?;

    Ok((
        manifest,
        VerifiedFullBackup {
            snapshot_id: snapshot_id.to_string(),
        },
    ))
}

fn summary_from_manifest(
    manifest: BackupSnapshotManifest,
    manifest_key: String,
) -> Result<BackupSnapshotSummary> {
    let kind = match manifest.backup_kind {
        BackupKind::Full => "Full",
        BackupKind::Incremental => "Incremental",
        BackupKind::SyntheticFull => "SyntheticFull",
        _ => "Unknown",
    }
    .to_string();
    let total_object_bytes = manifest
        .objects
        .iter()
        .map(|object| object.size_bytes)
        .sum::<u64>();

    Ok(BackupSnapshotSummary {
        snapshot_id: manifest.snapshot_id.to_string(),
        created_at: manifest.created_at,
        kind,
        app_id: manifest.app_id,
        app_version: manifest.app_version,
        table_count: i32::try_from(manifest.database.table_count).unwrap_or(i32::MAX),
        object_count: i32::try_from(manifest.objects.len()).unwrap_or(i32::MAX),
        total_object_bytes: i64::try_from(total_object_bytes).unwrap_or(i64::MAX),
        manifest_key,
    })
}

struct OfflineRestoreTarget {
    store: Arc<dyn BlobStore>,
    root: PathBuf,
    attempted: AtomicBool,
}

/// The local provider follows filesystem links. Require a trusted, empty root with no
/// linked ancestors; the offline owner must prevent concurrent filesystem replacement.
async fn ensure_empty_object_root(root: &Path) -> Result<()> {
    let absolute = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()?.join(root)
    };
    let mut checked = PathBuf::new();
    for component in absolute.components() {
        if matches!(component, Component::ParentDir) {
            anyhow::bail!("Object restore root must not contain parent path components");
        }
        checked.push(component);
        match tokio::fs::symlink_metadata(&checked).await {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                anyhow::bail!(
                    "Object restore root has a linked or non-directory component: {}",
                    checked.display()
                );
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("Failed to inspect object restore root"),
        }
    }
    match tokio::fs::read_dir(&absolute).await {
        Ok(mut entries) => {
            if entries.next_entry().await?.is_some() {
                anyhow::bail!("Object restore target is not empty");
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("Failed to inspect object restore target"),
    }
}

fn validate_full_restore_manifest(
    manifest: &BackupSnapshotManifest,
    snapshot_id: Uuid,
) -> Result<()> {
    if manifest.snapshot_id != snapshot_id || manifest.app_id != "librarian" {
        anyhow::bail!("Restore manifest does not identify the requested Librarian snapshot");
    }
    if !matches!(
        manifest.backup_kind,
        BackupKind::Full | BackupKind::SyntheticFull
    ) || manifest.parent_snapshot_id.is_some()
        || !manifest.database.changes.is_empty()
        || !manifest.tombstones.is_empty()
    {
        anyhow::bail!("Restore requires a standalone full or synthetic-full snapshot");
    }
    let mut ids = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for object in &manifest.objects {
        validate_restore_object_key(object.object_id, &object.storage_key, &object.sha256_hex)?;
        if !ids.insert(object.object_id) || !keys.insert(&object.storage_key) {
            anyhow::bail!("Restore manifest contains duplicate object identities or keys");
        }
    }
    Ok(())
}

fn validate_restore_object_key(object_id: Uuid, key: &str, sha256_hex: &str) -> Result<()> {
    validate_blob_key(key)?;
    let components = key.split('/').collect::<Vec<_>>();
    if components.len() != 4 {
        anyhow::bail!("Invalid managed object restore key '{key}'");
    }
    let namespace = match components[0] {
        "originals" => StorageNamespace::Originals,
        "recycle_bin" => StorageNamespace::RecycleBin,
        "thumbnails" => StorageNamespace::Thumbnails,
        "derivatives" => StorageNamespace::Derivatives,
        "exports" => StorageNamespace::Exports,
        "temp" => StorageNamespace::Temp,
        _ => anyhow::bail!("Invalid managed object namespace in '{key}'"),
    };
    let extension = components[3]
        .split_once('.')
        .map(|(_, extension)| extension);
    if build_storage_key(namespace, &object_id, extension) != key {
        anyhow::bail!("Managed object restore key '{key}' does not match its identity");
    }
    if sha256_hex.len() != 64
        || !sha256_hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        anyhow::bail!("Invalid SHA-256 for managed object '{key}'");
    }
    Ok(())
}

async fn restored_storage_objects(
    db: &Database,
    storage: &ObjectStorageService,
    manifest: &BackupSnapshotManifest,
) -> Result<Vec<StoredObject>> {
    let rows = StorageObject::query(db.pool()).fetch_all().await?;
    let mut expected = manifest
        .objects
        .iter()
        .map(|object| (object.object_id, object))
        .collect::<BTreeMap<_, _>>();
    let mut objects = Vec::with_capacity(rows.len());
    for row in rows {
        let stored = storage.stored_object_from_entity(&row)?;
        let object = expected
            .remove(&stored.object_id)
            .context("Restored StorageObject metadata has no unique manifest object")?;
        if stored.backend != storage.storage().backend()
            || stored.namespace.as_str() != object.storage_key.split('/').next().unwrap_or_default()
            || stored.storage_key != object.storage_key
            || stored.sha256_hex != object.sha256_hex
            || stored.size_bytes != object.size_bytes
            || stored.mime_type != object.mime_type
        {
            anyhow::bail!(
                "Restored StorageObject metadata disagrees with manifest for '{}'",
                object.storage_key
            );
        }
        objects.push(stored);
    }
    if !expected.is_empty() {
        anyhow::bail!("Restore manifest contains objects without StorageObject metadata");
    }
    Ok(objects)
}

async fn verify_restored_object(
    storage: &ObjectStorageService,
    object: &StoredObject,
) -> Result<()> {
    let mut stream = storage
        .storage()
        .get_object_stream(object)
        .await?
        .body
        .into_inner();
    let mut hash = Sha256::new();
    let mut size = 0_u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        size = size
            .checked_add(u64::try_from(chunk.len())?)
            .context("Restored object size overflow")?;
        hash.update(&chunk);
    }
    if size != object.size_bytes || hex::encode(hash.finalize()) != object.sha256_hex {
        anyhow::bail!(
            "Restored object byte verification failed for '{}'",
            object.storage_key
        );
    }
    Ok(())
}

struct LibrarianRestoreObjectSink {
    store: Arc<dyn BlobStore>,
}

#[async_trait]
impl RestoreObjectSink for LibrarianRestoreObjectSink {
    async fn restore_object(
        &self,
        object: BackupObjectRef,
        bytes: Bytes,
    ) -> Result<(), BackupError> {
        self.restore_object_stream(object, StorageByteStream::from_bytes(bytes))
            .await
    }

    async fn restore_object_stream(
        &self,
        object: BackupObjectRef,
        body: StorageByteStream,
    ) -> Result<(), BackupError> {
        validate_restore_object_key(object.object_id, &object.storage_key, &object.sha256_hex)
            .map_err(|error| BackupError::UnsupportedOperation {
                operation: error.to_string(),
            })?;
        let outcome = self
            .store
            .put_blob_if_not_exists(
                &object.storage_key,
                body,
                BlobPutOptions {
                    content_type: object.mime_type.clone(),
                },
            )
            .await?
            .ok_or_else(|| BackupError::UnsupportedOperation {
                operation: format!(
                    "refusing to overwrite restore object '{}'",
                    object.storage_key
                ),
            })?;
        if outcome.sha256_hex != object.sha256_hex {
            return Err(BackupError::ChecksumMismatch {
                key: object.storage_key,
                expected: object.sha256_hex,
                actual: outcome.sha256_hex,
            });
        }
        if outcome.size_bytes != object.size_bytes {
            return Err(BackupError::UnsupportedOperation {
                operation: format!("restored object size mismatch for '{}'", object.storage_key),
            });
        }
        Ok(())
    }
}

#[async_trait]
impl Service for BackupService {
    fn name(&self) -> &str {
        "backup"
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["database".to_string(), "storage".to_string()]
    }

    async fn start(&self) -> Result<()> {
        if self.offline_restore.is_some() {
            anyhow::bail!(
                "Offline recovery services cannot be started; publish verified targets through normal application startup"
            );
        }
        tracing::info!(
            service = "backup",
            app_id = %self.app_id,
            app_version = %self.app_version,
            "Backup service started in staged mode"
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        tracing::info!(service = "backup", "Backup service stopped");
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        Ok(ServiceHealth::healthy())
    }
}

/// Returns the current time as UTC Unix seconds, for backup/restore request timestamps.
fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

/// Adapts the app's [`Database`] to `graphql-orm-backup`'s [`GraphqlOrmBackupAdapter`] contract
/// by driving `graphql-orm`'s own [`GraphqlOrmBackupRuntime`] (already implemented for
/// [`Database`]) over every backup-enabled entity in [`entity_metadata`]. This is the glue that
/// lets [`run_create_full_backup`]/[`run_restore_snapshot`] perform real full database
/// backup/restore instead of only the stored-object backup handled by
/// [`LibrarianBackupObjectIndex`].
struct LibrarianBackupDatabaseAdapter {
    db: Database,
    migration_version: String,
}

impl LibrarianBackupDatabaseAdapter {
    fn new(db: Database, migration_version: String) -> Self {
        Self {
            db,
            migration_version,
        }
    }

    fn entities(&self) -> Vec<&'static graphql_orm::graphql::orm::EntityMetadata> {
        entity_metadata()
    }

    fn backup_descriptors(&self) -> Vec<EntityBackupDescriptor> {
        let mut descriptors = self.db.list_backup_entities(&self.entities());
        descriptors.sort_by_key(|descriptor| descriptor.export_order);
        descriptors
    }
}

#[async_trait]
impl GraphqlOrmBackupAdapter for LibrarianBackupDatabaseAdapter {
    async fn schema_snapshot(&self) -> Result<GraphqlOrmBackupSchema, BackupError> {
        let entities = self.entities();
        let snapshot = self
            .db
            .schema_snapshot(self.migration_version.clone(), &entities);
        let backend = canonical_backend(&snapshot.backend).map_err(|error| {
            BackupError::UnsupportedOperation {
                operation: error.to_string(),
            }
        })?;
        // A descriptor digest is not a physical migration fingerprint. Read the
        // live schema with the same pinned primitive used by migration planning.
        let physical = DefaultBackend::introspect_schema(self.db.pool())
            .await
            .map_err(|source| map_sqlx_err("failed to inspect backup source schema", source))?;
        let target = SchemaModel::from_entities(&entities);
        let coverage = self
            .db
            .schema()
            .plan_migration(
                "backup-schema-coverage",
                "validate backup entity coverage",
                &physical,
                &target,
            )
            .map_err(|source| {
                map_sqlx_err("failed to validate backup source schema coverage", source)
            })?;
        // Validation alone treats extra columns/tables as warnings. A logical
        // backup must not attest to a physical schema whose data it cannot export
        // or whose indexes/constraints its recovery build cannot recreate.
        if !coverage.steps.is_empty() {
            return Err(BackupError::UnsupportedOperation {
                operation: "Backup source schema does not match generated entity metadata; use a qualified recovery build matching that source schema".to_string(),
            });
        }
        Ok(GraphqlOrmBackupSchema {
            backend: backend.to_string(),
            migration_version: snapshot.migration_version,
            schema_hash: SchemaIdentity::new(&physical.stable_hash(), &snapshot.schema_hash)
                .encode(),
        })
    }

    async fn restore_target_is_empty(&self) -> Result<bool, BackupError> {
        let tables = self.export_full().await?;
        Ok(tables.iter().all(|table| table.rows.is_empty()))
    }

    async fn export_full(&self) -> Result<Vec<BackupTableExport>, BackupError> {
        let descriptors = self.backup_descriptors();
        let mut snapshot = self
            .db
            .begin_consistent_snapshot()
            .await
            .map_err(|source| map_sqlx_err("failed to begin backup snapshot", source))?;

        let mut exports = Vec::with_capacity(descriptors.len());
        for descriptor in &descriptors {
            let rows = self
                .db
                .export_table_rows(&mut snapshot, descriptor)
                .await
                .map_err(|source| {
                    map_sqlx_err(
                        &format!("failed to export table '{}'", descriptor.table_name),
                        source,
                    )
                })?;
            exports.push(BackupTableExport {
                table_name: descriptor.table_name.clone(),
                rows: rows.into_iter().map(orm_row_to_backup_row).collect(),
            });
        }
        Ok(exports)
    }

    async fn export_incremental(
        &self,
        _parent_snapshot_id: Uuid,
    ) -> Result<Vec<BackupChangeExport>, BackupError> {
        Err(BackupError::UnsupportedOperation {
            operation: "incremental database backup requires graphql-orm change journal"
                .to_string(),
        })
    }

    async fn restore_full(
        &self,
        backup_schema: GraphqlOrmBackupSchema,
        export: Vec<BackupTableExport>,
        context: RestoreContext,
    ) -> Result<(), BackupError> {
        let target_schema = self.schema_snapshot().await?;
        if backup_schema.backend != target_schema.backend
            || backup_schema.schema_hash != target_schema.schema_hash
        {
            return Err(BackupError::RestoreSchemaMismatch {
                backup_backend: backup_schema.backend,
                backup_schema_hash: backup_schema.schema_hash,
                target_backend: target_schema.backend,
                target_schema_hash: target_schema.schema_hash,
            });
        }

        let descriptors = self.backup_descriptors();
        let mut ordered = descriptors;
        ordered.sort_by_key(|descriptor| descriptor.restore_order);
        let orm_context = orm_restore_context_from(&context);

        let mut rows_by_table = export
            .into_iter()
            .map(|table| (table.table_name, table.rows))
            .collect::<BTreeMap<_, _>>();

        for descriptor in &ordered {
            let rows = match rows_by_table.remove(&descriptor.table_name) {
                Some(table_rows) => table_rows
                    .into_iter()
                    .map(|row| backup_row_to_orm_row(row, descriptor))
                    .collect::<Result<Vec<_>, BackupError>>()?,
                None => Vec::new(),
            };

            self.db
                .import_table_rows(descriptor, &rows, &orm_context)
                .await
                .map_err(|source| {
                    map_sqlx_err(
                        &format!("failed to restore table '{}'", descriptor.table_name),
                        source,
                    )
                })?;
        }

        Ok(())
    }

    async fn restore_incremental(
        &self,
        _changes: Vec<BackupChangeExport>,
        _context: RestoreContext,
    ) -> Result<(), BackupError> {
        Err(BackupError::UnsupportedOperation {
            operation: "incremental database restore requires graphql-orm change journal"
                .to_string(),
        })
    }
}

fn map_sqlx_err(context: &str, source: sqlx::Error) -> BackupError {
    BackupError::UnsupportedOperation {
        operation: format!("{context}: {source}"),
    }
}

fn orm_restore_context_from(context: &RestoreContext) -> OrmRestoreContext {
    OrmRestoreContext {
        mode: match context.mode {
            RestoreMode::EmptyDatabase => OrmRestoreMode::EmptyDatabase,
            RestoreMode::DryRun => OrmRestoreMode::DryRun,
        },
        disable_policies: context.disable_policies,
        disable_change_journal: context.disable_change_journal,
    }
}

fn orm_row_to_backup_row(row: OrmBackupRow) -> graphql_orm_backup::BackupRow {
    let values = row
        .values
        .into_iter()
        .map(|(column, value)| (column, orm_backup_value_to_json(value)))
        .collect::<serde_json::Map<_, _>>();
    graphql_orm_backup::BackupRow {
        table_name: row.table_name,
        primary_key: row.primary_key,
        row_hash: row.row_hash,
        values,
    }
}

fn orm_backup_value_to_json(value: OrmBackupValue) -> serde_json::Value {
    match value {
        OrmBackupValue::Null => serde_json::Value::Null,
        OrmBackupValue::Bool(value) => serde_json::Value::Bool(value),
        OrmBackupValue::Integer(value) => serde_json::Value::Number(value.into()),
        OrmBackupValue::Float(value) => serde_json::Number::from_f64(value)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        OrmBackupValue::Decimal(value) => serde_json::json!(value),
        OrmBackupValue::String(value) => serde_json::Value::String(value),
        OrmBackupValue::Uuid(value) => serde_json::Value::String(value.to_string()),
        OrmBackupValue::Json(value) => value,
        OrmBackupValue::Bytes(value) => serde_json::Value::String(BASE64.encode(value)),
    }
}

fn backup_row_to_orm_row(
    row: graphql_orm_backup::BackupRow,
    descriptor: &EntityBackupDescriptor,
) -> Result<OrmBackupRow, BackupError> {
    let column_kinds = descriptor
        .columns
        .iter()
        .map(|column| (column.column_name.as_str(), column.logical_type))
        .collect::<BTreeMap<_, _>>();

    let mut values = BTreeMap::new();
    for (column, value) in row.values {
        let kind = column_kinds
            .get(column.as_str())
            .copied()
            .unwrap_or(BackupValueKind::Json);
        values.insert(column, json_to_orm_backup_value(value, kind)?);
    }

    Ok(OrmBackupRow {
        table_name: row.table_name,
        primary_key: row.primary_key,
        row_hash: row.row_hash,
        values,
    })
}

fn json_to_orm_backup_value(
    value: serde_json::Value,
    kind: BackupValueKind,
) -> Result<OrmBackupValue, BackupError> {
    if value.is_null() {
        return Ok(OrmBackupValue::Null);
    }

    let type_err = |expected: &str| BackupError::UnsupportedOperation {
        operation: format!("backup row value did not match expected type '{expected}'"),
    };

    Ok(match kind {
        BackupValueKind::Null => OrmBackupValue::Null,
        BackupValueKind::Bool => {
            OrmBackupValue::Bool(value.as_bool().ok_or_else(|| type_err("bool"))?)
        }
        BackupValueKind::Integer => {
            OrmBackupValue::Integer(value.as_i64().ok_or_else(|| type_err("integer"))?)
        }
        BackupValueKind::Float => {
            OrmBackupValue::Float(value.as_f64().ok_or_else(|| type_err("float"))?)
        }
        BackupValueKind::Decimal => {
            OrmBackupValue::Decimal(serde_json::from_value(value).map_err(|_| type_err("decimal"))?)
        }
        BackupValueKind::String => OrmBackupValue::String(
            value
                .as_str()
                .ok_or_else(|| type_err("string"))?
                .to_string(),
        ),
        BackupValueKind::Uuid => {
            let raw = value.as_str().ok_or_else(|| type_err("uuid"))?;
            let uuid = Uuid::parse_str(raw).map_err(|error| BackupError::UnsupportedOperation {
                operation: format!("invalid uuid in backup row: {error}"),
            })?;
            OrmBackupValue::Uuid(uuid)
        }
        BackupValueKind::Json => OrmBackupValue::Json(value),
        BackupValueKind::Bytes => {
            let raw = value.as_str().ok_or_else(|| type_err("bytes"))?;
            let bytes = BASE64
                .decode(raw)
                .map_err(|error| BackupError::UnsupportedOperation {
                    operation: format!("invalid base64 in backup row: {error}"),
                })?;
            OrmBackupValue::Bytes(bytes)
        }
    })
}

pub struct LibrarianBackupObjectIndex {
    db: Database,
    storage: Arc<ObjectStorageService>,
}

#[async_trait]
impl BackupObjectIndex for LibrarianBackupObjectIndex {
    async fn list_objects_for_full_backup(&self) -> Result<Vec<BackupObjectRef>, BackupError> {
        let rows = StorageObject::query(self.db.pool())
            .fetch_all()
            .await
            .map_err(|source| BackupError::UnsupportedOperation {
                operation: format!("failed to query storage objects: {source}"),
            })?;

        rows.into_iter()
            .map(storage_object_ref_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| BackupError::UnsupportedOperation {
                operation: source.to_string(),
            })
    }

    async fn list_objects_for_incremental_backup(
        &self,
        _since_snapshot_id: Uuid,
    ) -> Result<Vec<BackupObjectRef>, BackupError> {
        Err(BackupError::UnsupportedOperation {
            operation: "incremental object backup requires graphql-orm change journal".to_string(),
        })
    }

    async fn load_object(&self, object: &BackupObjectRef) -> Result<Bytes, BackupError> {
        let row = find_storage_object(&self.db, object).await?;
        let stored = self
            .storage
            .stored_object_from_entity(&row)
            .map_err(|source| BackupError::UnsupportedOperation {
                operation: source.to_string(),
            })?;
        let body = self
            .storage
            .storage()
            .get_object(&stored)
            .await
            .map_err(|source| BackupError::UnsupportedOperation {
                operation: source.to_string(),
            })?;
        Ok(Bytes::from(body.bytes))
    }
}

async fn find_storage_object(
    db: &Database,
    object: &BackupObjectRef,
) -> Result<StorageObject, BackupError> {
    let object_id = object.object_id.to_string();
    let rows = StorageObject::query(db.pool())
        .filter(StorageObjectWhereInput {
            object_id: Some(StringFilter {
                eq: Some(object_id.clone()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await
        .map_err(|source| BackupError::UnsupportedOperation {
            operation: format!("failed to query storage object by object_id: {source}"),
        })?;

    if let Some(row) = rows.into_iter().next() {
        return Ok(row);
    }

    StorageObject::query(db.pool())
        .filter(StorageObjectWhereInput {
            storage_key: Some(StringFilter {
                eq: Some(object.storage_key.clone()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await
        .map_err(|source| BackupError::UnsupportedOperation {
            operation: format!("failed to query storage object by storage_key: {source}"),
        })?
        .into_iter()
        .next()
        .ok_or_else(|| BackupError::MissingBlob {
            key: object.storage_key.clone(),
        })
}

fn storage_object_ref_from_row(row: StorageObject) -> Result<BackupObjectRef> {
    Ok(BackupObjectRef {
        object_id: Uuid::parse_str(&row.object_id)
            .with_context(|| format!("Invalid storage object_id '{}'", row.object_id))?,
        storage_key: row.storage_key,
        sha256_hex: row.sha256_hex,
        size_bytes: u64::try_from(row.size_bytes)
            .with_context(|| format!("Invalid storage size_bytes '{}'", row.size_bytes))?,
        mime_type: row.mime_type,
    })
}

#[cfg(test)]
mod tests {
    //! These tests only ever touch throwaway `sqlite::memory:` databases and `tempfile`
    //! repository directories — never the live backend database or torrent session state.
    use super::*;
    use crate::db::DbPool;
    use crate::graphql::entities::{
        AppSetting, ArtworkCache, CreateAppSettingInput, CreateArtworkCacheInput,
        CreateStorageObjectInput,
    };
    use crate::services::database::bootstrap_schema;
    use crate::services::manager::HealthStatus;
    use crate::services::storage::{ObjectStorageService, ObjectStorageServiceConfig};
    use graphql_orm_storage::{StoragePutRequest, sha256_hex};

    #[test]
    fn decimal_backup_preserves_precision_and_rejects_lossy_values() {
        use graphql_orm::graphql::orm::{DecimalDef, DecimalValue};

        let value = DecimalValue::new(
            "1234567890123456.78".parse().unwrap(),
            DecimalDef::new(18, 2).unwrap(),
        )
        .unwrap();
        let json = orm_backup_value_to_json(OrmBackupValue::Decimal(value));
        assert_eq!(json["value"], "1234567890123456.78");
        assert_eq!(
            json_to_orm_backup_value(json, BackupValueKind::Decimal).unwrap(),
            OrmBackupValue::Decimal(value),
        );

        for invalid in [
            serde_json::json!(123.45),
            serde_json::json!("123.45"),
            serde_json::json!({"value": "1.234", "definition": {"precision": 5, "scale": 2}}),
        ] {
            assert!(json_to_orm_backup_value(invalid, BackupValueKind::Decimal).is_err());
        }
    }

    /// An in-memory database with the full app schema applied but with no seed data and no
    /// lifecycle hooks. Real policy providers remain installed for strict generated
    /// repository operations while exercising backup/restore in isolation.
    async fn schema_only_database() -> Database {
        let pool = DbPool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite database should connect");
        let mut db = Database::new(pool);
        db.set_entity_policy(crate::services::graphql::policy::AppEntityPolicy);
        db.set_field_policy(crate::services::graphql::policy::AppFieldPolicy);
        db.set_row_policy(crate::services::graphql::row_policy::OwnershipRowPolicy);
        bootstrap_schema(&db)
            .await
            .expect("schema bootstrap should succeed");
        db
    }

    fn test_backup_service(db: Database, temp_dir: &tempfile::TempDir) -> BackupService {
        let storage = Arc::new(
            ObjectStorageService::new(ObjectStorageServiceConfig {
                backend: "local".to_string(),
                local_path: temp_dir.path().join("storage"),
            })
            .expect("object storage service should construct"),
        );
        BackupService::new(
            db,
            storage,
            BackupServiceConfig {
                repository_path: temp_dir.path().join("backups"),
            },
        )
    }

    fn test_restore_service(db: Database, temp_dir: &tempfile::TempDir) -> BackupService {
        BackupService::new_for_offline_restore(
            db,
            ObjectStorageServiceConfig {
                backend: "local".to_string(),
                local_path: temp_dir.path().join("restored-storage"),
            },
            BackupServiceConfig {
                repository_path: temp_dir.path().join("backups"),
            },
        )
        .expect("offline recovery service should construct")
    }

    async fn insert_managed_object(service: &BackupService, bytes: &[u8]) -> StorageObject {
        let stored = service
            .storage
            .storage()
            .put_object(StoragePutRequest {
                namespace: StorageNamespace::Thumbnails,
                file_name: Some("poster.png".to_string()),
                mime_type: Some("image/png".to_string()),
                bytes: bytes.to_vec(),
            })
            .await
            .expect("source object should store");
        StorageObject::insert(
            &service.db,
            CreateStorageObjectInput {
                object_id: stored.object_id.to_string(),
                namespace: stored.namespace.as_str().to_string(),
                backend: stored.backend.as_str().to_string(),
                storage_key: stored.storage_key,
                original_file_name: stored.original_file_name,
                mime_type: stored.mime_type,
                size_bytes: i64::try_from(stored.size_bytes).unwrap(),
                sha256_hex: stored.sha256_hex,
            },
        )
        .await
        .expect("object metadata should insert")
    }

    async fn full_snapshot(service: &BackupService) -> BackupSnapshotSummary {
        service.create_full_backup().await.unwrap().unwrap()
    }

    async fn read_manifest(
        service: &BackupService,
        snapshot: &BackupSnapshotSummary,
    ) -> BackupSnapshotManifest {
        serde_json::from_slice(
            &service
                .repository
                .get_blob(&snapshot.manifest_key)
                .await
                .unwrap(),
        )
        .unwrap()
    }

    async fn write_manifest(service: &BackupService, mut manifest: BackupSnapshotManifest) {
        graphql_orm_backup::set_manifest_checksum(&mut manifest).unwrap();
        service
            .repository
            .put_blob(
                &snapshot_manifest_key(manifest.snapshot_id),
                Bytes::from(serde_json::to_vec(&manifest).unwrap()),
            )
            .await
            .unwrap();
    }

    async fn assert_target_database_empty(service: &BackupService) {
        let adapter = LibrarianBackupDatabaseAdapter::new(service.db.clone(), "test".to_string());
        assert!(adapter.restore_target_is_empty().await.unwrap());
    }

    #[tokio::test]
    async fn capabilities_and_health_require_offline_restore_target() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let service = test_backup_service(schema_only_database().await, &temp_dir);

        let capabilities = service.capabilities();
        assert!(capabilities.full_database_backup_available);
        assert!(capabilities.object_backup_available);
        assert!(!capabilities.restore_available);
        let offline = test_restore_service(schema_only_database().await, &temp_dir);
        assert!(offline.capabilities().restore_available);
        assert!(offline.start().await.is_err());
        assert!(!capabilities.incremental_backup_available);

        let health = service.health().await.expect("health check should succeed");
        assert_eq!(health.status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn restore_adapter_rejects_a_mismatched_backup_schema_before_import() {
        let adapter =
            LibrarianBackupDatabaseAdapter::new(schema_only_database().await, "test".to_string());
        let target_schema = adapter
            .schema_snapshot()
            .await
            .expect("target schema snapshot should succeed");
        let backup_schema = GraphqlOrmBackupSchema {
            schema_hash: "mismatched-schema-hash".to_string(),
            ..target_schema.clone()
        };

        let err = adapter
            .restore_full(backup_schema, Vec::new(), RestoreContext::empty_database())
            .await
            .expect_err("mismatched backup schema should fail before import");

        match err {
            BackupError::RestoreSchemaMismatch {
                backup_backend,
                backup_schema_hash,
                target_backend,
                target_schema_hash,
            } => {
                assert_eq!(backup_backend, target_schema.backend);
                assert_eq!(backup_schema_hash, "mismatched-schema-hash");
                assert_eq!(target_backend, target_schema.backend);
                assert_eq!(target_schema_hash, target_schema.schema_hash);
            }
            other => panic!("expected RestoreSchemaMismatch, got {other}"),
        }
    }

    #[tokio::test]
    async fn full_backup_and_restore_round_trips_app_settings_into_an_empty_database() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");

        let source_db = schema_only_database().await;
        AppSetting::insert(
            &source_db,
            CreateAppSettingInput {
                key: "backup.roundtrip.test".to_string(),
                value: "42".to_string(),
                description: Some("full backup/restore round trip".to_string()),
                category: "test".to_string(),
            },
        )
        .await
        .expect("insert app setting should succeed");

        let source_service = test_backup_service(source_db, &temp_dir);
        let mut changes = source_service.subscribe_changes();
        assert_eq!(*changes.borrow_and_update(), 0);
        let summary = source_service
            .create_full_backup()
            .await
            .expect("create_full_backup should succeed")
            .expect("create_full_backup should return a snapshot summary");
        assert!(changes.has_changed().unwrap());
        assert_eq!(*changes.borrow_and_update(), 1);
        assert!(summary.table_count > 0);
        assert_eq!(summary.kind, "Full");

        let target_db = schema_only_database().await;
        let target_pool = target_db.pool().clone();
        let target_service = test_restore_service(target_db, &temp_dir);
        target_service
            .restore_full_backup(&summary.snapshot_id)
            .await
            .expect("restore_full_backup should succeed");

        let restored = AppSetting::query(&target_pool)
            .fetch_all()
            .await
            .expect("query app settings after restore should succeed");
        assert!(
            restored
                .iter()
                .any(|setting| setting.key == "backup.roundtrip.test" && setting.value == "42"),
            "restored database should contain the backed-up app setting, got: {restored:?}"
        );
    }

    #[tokio::test]
    async fn restore_full_backup_rejects_a_non_empty_target() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");

        let source_db = schema_only_database().await;
        let source_service = test_backup_service(source_db, &temp_dir);
        let summary = source_service
            .create_full_backup()
            .await
            .expect("create_full_backup should succeed")
            .expect("create_full_backup should return a snapshot summary");

        let target_db = schema_only_database().await;
        AppSetting::insert(
            &target_db,
            CreateAppSettingInput {
                key: "already.present".to_string(),
                value: "1".to_string(),
                description: None,
                category: "test".to_string(),
            },
        )
        .await
        .expect("insert app setting should succeed");
        let target_service = test_restore_service(target_db, &temp_dir);

        let err = target_service
            .restore_full_backup(&summary.snapshot_id)
            .await
            .expect_err("restore into a non-empty database should fail");
        assert!(err.to_string().contains("Failed to restore full backup"));
        let existing = AppSetting::query(target_service.db.pool())
            .fetch_all()
            .await
            .unwrap();
        assert_eq!(existing.len(), 1);
        assert_eq!(existing[0].key, "already.present");
        assert_eq!(existing[0].value, "1");
        assert!(!temp_dir.path().join("restored-storage").exists());
    }

    #[tokio::test]
    async fn full_restore_rehydrates_managed_and_artwork_bytes_in_an_empty_store() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = test_backup_service(schema_only_database().await, &temp_dir);
        let image = BASE64.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/lXcAAAAASUVORK5CYII=").unwrap();
        let poster = insert_managed_object(&source, &image).await;
        let empty = insert_managed_object(&source, &[]).await;
        ArtworkCache::insert(
            &source.db,
            CreateArtworkCacheInput {
                entity_type: "movie".to_string(),
                entity_id: "fixture-movie".to_string(),
                artwork_type: "poster".to_string(),
                storage_object_id: poster.id.clone(),
                source_url: None,
                width: Some(1),
                height: Some(1),
            },
        )
        .await
        .unwrap();
        let snapshot = full_snapshot(&source).await;
        assert_eq!(snapshot.object_count, 2);
        let target = test_restore_service(schema_only_database().await, &temp_dir);
        assert_target_database_empty(&target).await;
        assert!(!temp_dir.path().join("restored-storage").exists());
        target
            .restore_full_backup(&snapshot.snapshot_id)
            .await
            .unwrap();

        assert!(
            !temp_dir
                .path()
                .join("restored-storage/.librarian-restore-in-progress")
                .exists()
        );
        for (row, expected) in [(&poster, image.as_slice()), (&empty, &[][..])] {
            let restored = StorageObject::get(target.db.pool(), &row.id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(restored.object_id, row.object_id);
            assert_eq!(restored.storage_key, row.storage_key);
            assert_eq!(restored.sha256_hex, sha256_hex(expected));
            let stored = target.storage.stored_object_from_entity(&restored).unwrap();
            let actual = target.storage.storage().get_object(&stored).await.unwrap();
            assert_eq!(actual.bytes, expected);
            assert_eq!(
                tokio::fs::read(
                    temp_dir
                        .path()
                        .join("restored-storage")
                        .join(&row.storage_key)
                )
                .await
                .unwrap(),
                expected
            );
        }
        let artwork = ArtworkCache::query(target.db.pool())
            .fetch_all()
            .await
            .unwrap();
        assert_eq!(artwork.len(), 1);
        assert_eq!(artwork[0].storage_object_id, poster.id);
        let artwork_row = StorageObject::get(target.db.pool(), &artwork[0].storage_object_id)
            .await
            .unwrap()
            .unwrap();
        let stored = target
            .storage
            .stored_object_from_entity(&artwork_row)
            .unwrap();
        assert_eq!(
            target
                .storage
                .storage()
                .get_object(&stored)
                .await
                .unwrap()
                .bytes,
            image
        );
        assert!(
            target
                .restore_full_backup(&snapshot.snapshot_id)
                .await
                .unwrap_err()
                .to_string()
                .contains("already used")
        );
    }

    #[tokio::test]
    async fn restore_rejects_missing_or_corrupt_payloads_before_changing_targets() {
        for corrupt in [false, true] {
            let temp_dir = tempfile::tempdir().unwrap();
            let source = test_backup_service(schema_only_database().await, &temp_dir);
            insert_managed_object(&source, b"managed bytes").await;
            let snapshot = full_snapshot(&source).await;
            let manifest = read_manifest(&source, &snapshot).await;
            let blob = temp_dir
                .path()
                .join("backups")
                .join(&manifest.objects[0].content_key);
            if corrupt {
                tokio::fs::write(blob, b"corrupt bytes").await.unwrap();
            } else {
                tokio::fs::remove_file(blob).await.unwrap();
            }
            let target = test_restore_service(schema_only_database().await, &temp_dir);
            let error = target
                .restore_full_backup(&snapshot.snapshot_id)
                .await
                .unwrap_err();
            let details = format!("{error:#}");
            assert!(
                details.contains("discard the offline staging target"),
                "{details}"
            );
            assert_target_database_empty(&target).await;
            assert!(!temp_dir.path().join("restored-storage").exists());
        }
    }

    #[tokio::test]
    async fn restore_rejects_existing_object_bytes_without_overwriting_them() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = test_backup_service(schema_only_database().await, &temp_dir);
        let row = insert_managed_object(&source, b"backed up bytes").await;
        let snapshot = full_snapshot(&source).await;
        let target = test_restore_service(schema_only_database().await, &temp_dir);
        let existing = temp_dir
            .path()
            .join("restored-storage")
            .join(&row.storage_key);
        tokio::fs::create_dir_all(existing.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&existing, b"keep existing bytes")
            .await
            .unwrap();
        let error = target
            .restore_full_backup(&snapshot.snapshot_id)
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Object restore target is not empty")
        );
        assert_eq!(
            tokio::fs::read(existing).await.unwrap(),
            b"keep existing bytes"
        );
        assert_target_database_empty(&target).await;
    }

    #[tokio::test]
    async fn restore_rejects_unsafe_keys_and_inconsistent_object_metadata() {
        for invalid_key in [true, false] {
            let temp_dir = tempfile::tempdir().unwrap();
            let source = test_backup_service(schema_only_database().await, &temp_dir);
            insert_managed_object(&source, b"object bytes").await;
            let snapshot = full_snapshot(&source).await;
            let mut manifest = read_manifest(&source, &snapshot).await;
            if invalid_key {
                manifest.objects[0].storage_key = "../outside.png".to_string();
            } else {
                // Well-formed, checksum-valid manifest that disagrees with backed-up rows.
                manifest.objects[0].mime_type = Some("application/octet-stream".to_string());
            }
            let object_key = manifest.objects[0].storage_key.clone();
            write_manifest(&source, manifest).await;
            let target = test_restore_service(schema_only_database().await, &temp_dir);
            assert!(
                target
                    .restore_full_backup(&snapshot.snapshot_id)
                    .await
                    .is_err()
            );
            assert!(
                !temp_dir
                    .path()
                    .join("restored-storage")
                    .join(object_key)
                    .exists()
            );
            if invalid_key {
                assert_target_database_empty(&target).await;
            }
            assert!(!temp_dir.path().join("outside.png").exists());
            assert!(
                target
                    .restore_full_backup(&snapshot.snapshot_id)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("already used")
            );
        }
    }

    #[tokio::test]
    async fn live_service_refuses_restore_even_when_its_database_is_empty() {
        let temp_dir = tempfile::tempdir().unwrap();
        let service = test_backup_service(schema_only_database().await, &temp_dir);
        let snapshot = full_snapshot(&service).await;
        let error = service
            .restore_full_backup(&snapshot.snapshot_id)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("offline staging target"));
        assert_target_database_empty(&service).await;
        assert!(!temp_dir.path().join("storage").exists());
    }

    #[tokio::test]
    async fn object_sink_never_overwrites_a_key_created_after_target_preflight() {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Arc::new(LocalStorageBackend::new(temp_dir.path()));
        let object_id = Uuid::new_v4();
        let key = build_storage_key(StorageNamespace::Thumbnails, &object_id, Some("png"));
        store
            .put_blob(
                &key,
                StorageByteStream::from_bytes(b"existing".to_vec()),
                BlobPutOptions::default(),
            )
            .await
            .unwrap();
        let sink = LibrarianRestoreObjectSink {
            store: store.clone(),
        };
        let error = sink
            .restore_object(
                BackupObjectRef {
                    object_id,
                    storage_key: key.clone(),
                    sha256_hex: sha256_hex(b"replacement"),
                    size_bytes: 11,
                    mime_type: None,
                },
                Bytes::from_static(b"replacement"),
            )
            .await
            .unwrap_err();
        assert!(error.to_string().contains("refusing to overwrite"));
        assert_eq!(
            tokio::fs::read(temp_dir.path().join(key)).await.unwrap(),
            b"existing"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn restore_rejects_linked_object_roots_and_ancestors() {
        for ancestor in [false, true] {
            let temp_dir = tempfile::tempdir().unwrap();
            let outside = tempfile::tempdir().unwrap();
            let source = test_backup_service(schema_only_database().await, &temp_dir);
            insert_managed_object(&source, b"bytes").await;
            let snapshot = full_snapshot(&source).await;
            let link = temp_dir.path().join("linked-root");
            std::os::unix::fs::symlink(outside.path(), &link).unwrap();
            let root = if ancestor { link.join("child") } else { link };
            let target = BackupService::new_for_offline_restore(
                schema_only_database().await,
                ObjectStorageServiceConfig {
                    backend: "local".to_string(),
                    local_path: root,
                },
                BackupServiceConfig {
                    repository_path: temp_dir.path().join("backups"),
                },
            )
            .unwrap();
            let error = target
                .restore_full_backup(&snapshot.snapshot_id)
                .await
                .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("linked or non-directory component")
            );
            assert_target_database_empty(&target).await;
            assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
        }
    }

    /// Delegates all real I/O to the pinned local provider except for a deterministic
    /// second-object failure, so the service's partial-restore path is exercised.
    struct FailSecondObject {
        local: LocalStorageBackend,
        writes: std::sync::atomic::AtomicUsize,
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
        ) -> Result<graphql_orm_storage::BlobWriteOutcome, graphql_orm_storage::StorageError>
        {
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
                    operation: "injected second-object write failure".to_string(),
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
        ) -> Result<Option<graphql_orm_storage::BlobMetadata>, graphql_orm_storage::StorageError>
        {
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

    #[tokio::test]
    async fn partial_object_failure_is_reported_and_target_cannot_be_reused_or_started() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = test_backup_service(schema_only_database().await, &temp_dir);
        insert_managed_object(&source, b"first bytes").await;
        insert_managed_object(&source, b"second bytes").await;
        let snapshot = full_snapshot(&source).await;
        let manifest = read_manifest(&source, &snapshot).await;
        let mut target = test_restore_service(schema_only_database().await, &temp_dir);
        let root = temp_dir.path().join("restored-storage");
        target.offline_restore = Some(Arc::new(OfflineRestoreTarget {
            store: Arc::new(FailSecondObject {
                local: LocalStorageBackend::new(&root),
                writes: std::sync::atomic::AtomicUsize::new(0),
            }),
            root: root.clone(),
            attempted: AtomicBool::new(false),
        }));
        let error = target
            .restore_full_backup(&snapshot.snapshot_id)
            .await
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("injected second-object write failure"),
            "{error:#}"
        );
        assert!(error.to_string().contains("may be partially restored"));
        assert!(root.join(&manifest.objects[0].storage_key).exists());
        assert!(!root.join(&manifest.objects[1].storage_key).exists());
        assert_eq!(
            StorageObject::query(target.db.pool())
                .fetch_all()
                .await
                .unwrap()
                .len(),
            2
        );
        assert!(
            target
                .restore_full_backup(&snapshot.snapshot_id)
                .await
                .unwrap_err()
                .to_string()
                .contains("already used")
        );
        assert!(target.start().await.is_err());
        assert!(!target.capabilities().restore_available);
        assert!(root.join(".librarian-restore-in-progress").exists());
        let fresh_service = test_restore_service(schema_only_database().await, &temp_dir);
        assert!(
            fresh_service
                .restore_full_backup(&snapshot.snapshot_id)
                .await
                .unwrap_err()
                .to_string()
                .contains("Object restore target is not empty")
        );
        assert_target_database_empty(&fresh_service).await;
        let live = test_backup_service(schema_only_database().await, &temp_dir);
        assert!(!live.capabilities().restore_available);
    }

    #[tokio::test]
    async fn object_sink_propagates_hash_size_and_stream_failures() {
        for failure in ["hash", "size", "stream"] {
            let temp_dir = tempfile::tempdir().unwrap();
            let sink = LibrarianRestoreObjectSink {
                store: Arc::new(LocalStorageBackend::new(temp_dir.path())),
            };
            let object_id = Uuid::new_v4();
            let key = build_storage_key(StorageNamespace::Originals, &object_id, None);
            let object = BackupObjectRef {
                object_id,
                storage_key: key.clone(),
                sha256_hex: sha256_hex(if failure == "hash" {
                    b"wrong"
                } else {
                    b"bytes"
                }),
                size_bytes: if failure == "size" { 6 } else { 5 },
                mime_type: None,
            };
            let body = if failure == "stream" {
                StorageByteStream::new(Box::pin(futures::stream::iter(vec![
                    Ok(Bytes::from_static(b"part")),
                    Err(graphql_orm_storage::StorageError::UnsupportedOperation {
                        operation: "injected stream failure".to_string(),
                    }),
                ])))
            } else {
                StorageByteStream::from_bytes(b"bytes".to_vec())
            };
            let error = sink.restore_object_stream(object, body).await.unwrap_err();
            match failure {
                "hash" => assert!(matches!(error, BackupError::ChecksumMismatch { .. })),
                "size" => assert!(error.to_string().contains("size mismatch")),
                _ => {
                    assert!(error.to_string().contains("injected stream failure"));
                    assert!(!temp_dir.path().join(key).exists());
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "backup_schema_tests.rs"]
mod schema_tests;
