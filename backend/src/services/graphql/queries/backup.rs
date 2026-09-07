use std::sync::Arc;

use async_graphql::{Context, Object, Result, SimpleObject};

use crate::services::backup::{BackupCapabilities, BackupSnapshotSummary};
use crate::services::graphql::auth::RoleGuard;
use crate::services::manager::ServicesManager;

#[derive(Default)]
pub struct BackupQueries;

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "BackupCapabilities")]
pub struct BackupCapabilitiesGql {
    #[graphql(name = "fullDatabaseBackupAvailable")]
    pub full_database_backup_available: bool,
    #[graphql(name = "objectBackupAvailable")]
    pub object_backup_available: bool,
    #[graphql(name = "restoreAvailable")]
    pub restore_available: bool,
    #[graphql(name = "incrementalBackupAvailable")]
    pub incremental_backup_available: bool,
    #[graphql(name = "reason")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "BackupSnapshotSummary")]
pub struct BackupSnapshotSummaryGql {
    #[graphql(name = "snapshotId")]
    pub snapshot_id: String,
    #[graphql(name = "createdAt")]
    pub created_at: i64,
    #[graphql(name = "kind")]
    pub kind: String,
    #[graphql(name = "appId")]
    pub app_id: String,
    #[graphql(name = "appVersion")]
    pub app_version: String,
    #[graphql(name = "tableCount")]
    pub table_count: i32,
    #[graphql(name = "objectCount")]
    pub object_count: i32,
    #[graphql(name = "totalObjectBytes")]
    pub total_object_bytes: i64,
    #[graphql(name = "manifestKey")]
    pub manifest_key: String,
}

#[Object(name = "BackupQuery")]
impl BackupQueries {
    #[graphql(name = "backupCapabilities", guard = "RoleGuard::new(\"Admin\")")]
    async fn backup_capabilities(&self, ctx: &Context<'_>) -> Result<BackupCapabilitiesGql> {
        let backup = backup_service(ctx).await?;
        Ok(backup.capabilities().into())
    }

    #[graphql(name = "backupSnapshots", guard = "RoleGuard::new(\"Admin\")")]
    async fn backup_snapshots(&self, ctx: &Context<'_>) -> Result<Vec<BackupSnapshotSummaryGql>> {
        let backup = backup_service(ctx).await?;
        let snapshots = backup.list_snapshots().await?;
        Ok(snapshots.into_iter().map(Into::into).collect())
    }
}

async fn backup_service(ctx: &Context<'_>) -> Result<Arc<crate::services::backup::BackupService>> {
    let services = ctx.data_unchecked::<Arc<ServicesManager>>();
    services
        .get_backup()
        .await
        .ok_or_else(|| async_graphql::Error::new("Backup service unavailable"))
}

impl From<BackupCapabilities> for BackupCapabilitiesGql {
    fn from(value: BackupCapabilities) -> Self {
        Self {
            full_database_backup_available: value.full_database_backup_available,
            object_backup_available: value.object_backup_available,
            restore_available: value.restore_available,
            incremental_backup_available: value.incremental_backup_available,
            reason: value.reason,
        }
    }
}

impl From<BackupSnapshotSummary> for BackupSnapshotSummaryGql {
    fn from(value: BackupSnapshotSummary) -> Self {
        Self {
            snapshot_id: value.snapshot_id,
            created_at: value.created_at,
            kind: value.kind,
            app_id: value.app_id,
            app_version: value.app_version,
            table_count: value.table_count,
            object_count: value.object_count,
            total_object_bytes: value.total_object_bytes,
            manifest_key: value.manifest_key,
        }
    }
}
