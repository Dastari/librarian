use std::sync::Arc;

use async_graphql::{Context, Object, Result, SimpleObject};

use crate::services::graphql::auth::RoleGuard;
use crate::services::graphql::queries::backup::BackupSnapshotSummaryGql;
use crate::services::manager::ServicesManager;

#[derive(Default)]
pub struct BackupMutations;

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "CreateFullBackupResult")]
pub struct CreateFullBackupResultGql {
    #[graphql(name = "success")]
    pub success: bool,
    #[graphql(name = "snapshot")]
    pub snapshot: Option<BackupSnapshotSummaryGql>,
    #[graphql(name = "error")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "VerifyBackupSnapshotResult")]
pub struct VerifyBackupSnapshotResultGql {
    #[graphql(name = "success")]
    pub success: bool,
    #[graphql(name = "snapshotId")]
    pub snapshot_id: String,
    #[graphql(name = "error")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "RestoreFullBackupResult")]
pub struct RestoreFullBackupResultGql {
    #[graphql(name = "success")]
    pub success: bool,
    #[graphql(name = "snapshotId")]
    pub snapshot_id: String,
    #[graphql(name = "error")]
    pub error: Option<String>,
}

#[Object(name = "BackupMutation")]
impl BackupMutations {
    #[graphql(name = "createFullBackup", guard = "RoleGuard::new(\"Admin\")")]
    async fn create_full_backup(&self, ctx: &Context<'_>) -> Result<CreateFullBackupResultGql> {
        let backup = backup_service(ctx).await?;
        match backup.create_full_backup().await {
            Ok(snapshot) => Ok(CreateFullBackupResultGql {
                success: true,
                snapshot: snapshot.map(Into::into),
                error: None,
            }),
            Err(err) => Ok(CreateFullBackupResultGql {
                success: false,
                snapshot: None,
                error: Some(err.to_string()),
            }),
        }
    }

    #[graphql(name = "verifyBackupSnapshot", guard = "RoleGuard::new(\"Admin\")")]
    async fn verify_backup_snapshot(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "snapshotId")] snapshot_id: String,
    ) -> Result<VerifyBackupSnapshotResultGql> {
        let backup = backup_service(ctx).await?;
        match backup.verify_snapshot(&snapshot_id).await {
            Ok(()) => Ok(VerifyBackupSnapshotResultGql {
                success: true,
                snapshot_id,
                error: None,
            }),
            Err(err) => Ok(VerifyBackupSnapshotResultGql {
                success: false,
                snapshot_id,
                error: Some(err.to_string()),
            }),
        }
    }

    /// Restores a full database backup into an **empty** database. This does not touch a
    /// database that already has data (the underlying runtime rejects non-empty targets), so
    /// it is intended for disaster recovery onto a fresh instance, not for restoring over a
    /// live, populated database.
    #[graphql(name = "restoreFullBackup", guard = "RoleGuard::new(\"Admin\")")]
    async fn restore_full_backup(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "snapshotId")] snapshot_id: String,
    ) -> Result<RestoreFullBackupResultGql> {
        let backup = backup_service(ctx).await?;
        match backup.restore_full_backup(&snapshot_id).await {
            Ok(()) => Ok(RestoreFullBackupResultGql {
                success: true,
                snapshot_id,
                error: None,
            }),
            Err(err) => Ok(RestoreFullBackupResultGql {
                success: false,
                snapshot_id,
                error: Some(err.to_string()),
            }),
        }
    }
}

async fn backup_service(ctx: &Context<'_>) -> Result<Arc<crate::services::backup::BackupService>> {
    let services = ctx.data_unchecked::<Arc<ServicesManager>>();
    services
        .get_backup()
        .await
        .ok_or_else(|| async_graphql::Error::new("Backup service unavailable"))
}
