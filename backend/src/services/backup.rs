//! Backup service: full logical database backup/restore (via `graphql-orm-backup`, adapted
//! onto `graphql-orm`'s [`GraphqlOrmBackupRuntime`]) plus stored-object backup.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use bytes::Bytes;
use graphql_orm::graphql::filters::StringFilter;
use graphql_orm::graphql::orm::{
    BackupRow as OrmBackupRow, BackupValue as OrmBackupValue, BackupValueKind,
    EntityBackupDescriptor, GraphqlOrmBackupRuntime, RestoreContext as OrmRestoreContext,
    RestoreMode as OrmRestoreMode,
};
use graphql_orm_backup::{
    BackupChangeExport, BackupError, BackupKind, BackupObjectIndex, BackupObjectRef,
    BackupRepository, BackupSnapshotManifest, BackupTableExport, FullBackupRequest,
    GraphqlOrmBackupAdapter, GraphqlOrmBackupSchema, LocalBackupRepository, RestoreContext,
    RestoreMode, create_full_backup as run_create_full_backup,
    restore_snapshot as run_restore_snapshot, snapshot_manifest_key, verify_manifest_and_objects,
};
use uuid::Uuid;

use crate::db::Database;
use crate::graphql::entities::{StorageObject, StorageObjectWhereInput};
use crate::services::database::{VerifiedSchemaBackup, entity_metadata};
use crate::services::manager::{Service, ServiceHealth};
use crate::services::storage::ObjectStorageService;

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
        }
    }

    pub fn capabilities(&self) -> BackupCapabilities {
        BackupCapabilities {
            full_database_backup_available: true,
            object_backup_available: true,
            restore_available: true,
            incremental_backup_available: false,
            reason: Some(INCREMENTAL_BACKUP_UNAVAILABLE_REASON.to_string()),
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

        Ok(Some(summary_from_manifest(result.manifest, manifest_key)?))
    }

    /// Restores a full database backup into an empty database, and returns the object
    /// blob keys the caller (backup/restore endpoint) is responsible for rehydrating via
    /// [`BackupService::object_index`] separately (blob restore is not yet wired here).
    pub async fn restore_full_backup(&self, snapshot_id: &str) -> Result<()> {
        let snapshot_id = Uuid::parse_str(snapshot_id)
            .with_context(|| format!("Invalid backup snapshot id '{}'", snapshot_id))?;
        let adapter =
            LibrarianBackupDatabaseAdapter::new(self.db.clone(), self.app_version.clone());
        let repository = self.repository.clone();

        // Same rustc limitation documented on `verify_snapshot`/`create_full_backup` above:
        // `graphql-orm-backup`'s manifest-chain verification builds `Stream::map` closures
        // over borrowed manifest entries whose opaque future type cannot be proven `Send` in
        // a higher-ranked-lifetime context, so awaiting it directly fails to compile. Running
        // it on a blocking-pool thread via `Handle::block_on` sidesteps this.
        let handle = tokio::runtime::Handle::current();
        tokio::task::spawn_blocking(move || {
            handle.block_on(run_restore_snapshot(
                &repository,
                &adapter,
                snapshot_id,
                RestoreContext::empty_database(),
            ))
        })
        .await
        .context("Backup restore task panicked")?
        .context("Failed to restore full backup")?;
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
    if manifest.graphql_orm_schema_hash != expected_source_schema_hash {
        anyhow::bail!(
            "Backup schema hash {} does not match current source schema {}",
            manifest.graphql_orm_schema_hash,
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
    if manifest.database_backend != "sqlite" {
        anyhow::bail!(
            "Backup snapshot backend is {}, expected sqlite",
            manifest.database_backend
        );
    }
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

#[async_trait]
impl Service for BackupService {
    fn name(&self) -> &str {
        "backup"
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["database".to_string(), "storage".to_string()]
    }

    async fn start(&self) -> Result<()> {
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
        Ok(GraphqlOrmBackupSchema {
            backend: snapshot.backend,
            migration_version: snapshot.migration_version,
            schema_hash: snapshot.schema_hash,
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
    use crate::graphql::entities::{AppSetting, CreateAppSettingInput};
    use crate::services::database::bootstrap_schema;
    use crate::services::manager::HealthStatus;
    use crate::services::storage::{ObjectStorageService, ObjectStorageServiceConfig};

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
    /// installed hooks (entity/row policy, subscriptions, etc.) — enough for exercising CRUD
    /// and backup/restore in isolation.
    async fn schema_only_database() -> Database {
        let pool = DbPool::connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite database should connect");
        let db = Database::new(pool);
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

    #[tokio::test]
    async fn capabilities_and_health_reflect_available_full_backup_and_restore() {
        let temp_dir = tempfile::tempdir().expect("tempdir should create");
        let service = test_backup_service(schema_only_database().await, &temp_dir);

        let capabilities = service.capabilities();
        assert!(capabilities.full_database_backup_available);
        assert!(capabilities.object_backup_available);
        assert!(capabilities.restore_available);
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
        let summary = source_service
            .create_full_backup()
            .await
            .expect("create_full_backup should succeed")
            .expect("create_full_backup should return a snapshot summary");
        assert!(summary.table_count > 0);
        assert_eq!(summary.kind, "Full");

        let target_db = schema_only_database().await;
        let target_pool = target_db.pool().clone();
        let target_service = test_backup_service(target_db, &temp_dir);
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
        let target_service = test_backup_service(target_db, &temp_dir);

        let err = target_service
            .restore_full_backup(&summary.snapshot_id)
            .await
            .expect_err("restore into a non-empty database should fail");
        assert!(err.to_string().contains("Failed to restore full backup"));
    }
}
