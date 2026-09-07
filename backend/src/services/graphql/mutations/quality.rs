//! Quality profile custom mutations (`docs/tier1-features-plan.md` §2):
//! - `evaluateMediaFileQuality`: resolve the effective profile for a media
//!   file and (re)compute/persist its `quality_status`.
//! - `recomputeLibraryQualityStatus`: bulk re-evaluate every media file in a
//!   library, e.g. after changing which profile is assigned.
//! - `approveQualityUpgrade`: Q38 — the user-facing "approve" action for a
//!   quality-upgrade notification (see
//!   `LibraryScanService::maybe_notify_quality_upgrade`, wired from Q44's
//!   duplicate-torrent handling). Replaces the on-disk file with the
//!   candidate that triggered the notification, queues re-analysis, and
//!   resolves the notification. Dismissing an upgrade notification needs no
//!   custom mutation — the frontend already resolves it via the generic
//!   `updateNotification` mutation (`resolution: DISMISSED`).

use std::path::Path;
use std::sync::Arc;

use async_graphql::{Context, Object, Result, SimpleObject};
use graphql_orm::graphql::filters::StringFilter;

use crate::graphql::entities::{
    MediaFile, MediaFileWhereInput, Notification, NotificationWhereInput, UpdateNotificationInput,
};
use crate::services::ServicesManager;
use crate::services::graphql::auth::AuthExt;
use crate::services::quality::profile::{self, QualityEvaluation};

/// Notification `actionType` used for Q38/Q44 upgrade-available notifications.
pub const QUALITY_UPGRADE_ACTION_TYPE: &str = "quality_upgrade";

fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "QualityEvaluationResult")]
#[graphql(rename_fields = "camelCase")]
pub struct QualityEvaluationResultGql {
    pub success: bool,
    pub quality_status: Option<String>,
    pub reasons: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "RecomputeQualityResult")]
#[graphql(rename_fields = "camelCase")]
pub struct RecomputeQualityResultGql {
    pub success: bool,
    pub evaluated: i32,
    pub optimal: i32,
    pub suboptimal: i32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "ApproveQualityUpgradeResult")]
#[graphql(rename_fields = "camelCase")]
pub struct ApproveQualityUpgradeResultGql {
    pub success: bool,
    pub error: Option<String>,
}

async fn evaluate_and_persist(
    manager: &Arc<ServicesManager>,
    media_file_id: &str,
) -> anyhow::Result<QualityEvaluation> {
    let db_service = manager
        .get_database()
        .await
        .ok_or_else(|| anyhow::anyhow!("database service not available"))?;
    let db = db_service.pool().clone();
    profile::recompute_and_persist(&db, media_file_id).await
}

#[derive(Default)]
pub struct QualityMutations;

#[Object]
impl QualityMutations {
    /// Resolve the effective quality profile for a media file, evaluate it,
    /// and persist `quality_status`.
    #[graphql(name = "evaluateMediaFileQuality")]
    async fn evaluate_media_file_quality(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "mediaFileId")] media_file_id: String,
    ) -> Result<QualityEvaluationResultGql> {
        ctx.require_member()?;
        let manager = ctx.data_unchecked::<Arc<ServicesManager>>();

        match evaluate_and_persist(manager, &media_file_id).await {
            Ok(evaluation) => Ok(QualityEvaluationResultGql {
                success: true,
                quality_status: Some(evaluation.status_str().to_string()),
                reasons: evaluation.reasons().to_vec(),
                error: None,
            }),
            Err(e) => Ok(QualityEvaluationResultGql {
                success: false,
                quality_status: None,
                reasons: vec![],
                error: Some(e.to_string()),
            }),
        }
    }

    /// Re-evaluate every media file in a library against its resolved
    /// profile. Use after (re)assigning a `qualityProfileId`.
    #[graphql(name = "recomputeLibraryQualityStatus")]
    async fn recompute_library_quality_status(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "libraryId")] library_id: String,
    ) -> Result<RecomputeQualityResultGql> {
        ctx.require_admin()?;
        let manager = ctx.data_unchecked::<Arc<ServicesManager>>();

        let Some(db_service) = manager.get_database().await else {
            return Ok(RecomputeQualityResultGql {
                success: false,
                evaluated: 0,
                optimal: 0,
                suboptimal: 0,
                error: Some("database service not available".to_string()),
            });
        };
        let db = db_service.pool().clone();

        let files = match MediaFile::query(db.pool())
            .filter(MediaFileWhereInput {
                library_id: Some(string_eq(&library_id)),
                ..Default::default()
            })
            .fetch_all()
            .await
        {
            Ok(files) => files,
            Err(e) => {
                return Ok(RecomputeQualityResultGql {
                    success: false,
                    evaluated: 0,
                    optimal: 0,
                    suboptimal: 0,
                    error: Some(e.to_string()),
                });
            }
        };

        let mut optimal = 0;
        let mut suboptimal = 0;
        for media_file in &files {
            match evaluate_and_persist(manager, &media_file.id).await {
                Ok(evaluation) => {
                    if evaluation.is_optimal() {
                        optimal += 1;
                    } else {
                        suboptimal += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        media_file_id = %media_file.id,
                        error = %e,
                        "recomputeLibraryQualityStatus: failed to evaluate media file"
                    );
                }
            }
        }

        Ok(RecomputeQualityResultGql {
            success: true,
            evaluated: files.len() as i32,
            optimal,
            suboptimal,
            error: None,
        })
    }

    /// Q38: approve a pending quality-upgrade notification. Replaces the
    /// existing library file with the candidate that triggered the
    /// notification (never done automatically — see
    /// `LibraryScanService::maybe_notify_quality_upgrade`), queues
    /// re-analysis, and marks the notification resolved/`ACCEPTED`.
    #[graphql(name = "approveQualityUpgrade")]
    async fn approve_quality_upgrade(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "notificationId")] notification_id: String,
    ) -> Result<ApproveQualityUpgradeResultGql> {
        ctx.require_admin()?;
        let manager = ctx.data_unchecked::<Arc<ServicesManager>>();

        match approve_quality_upgrade_inner(manager, &notification_id).await {
            Ok(()) => Ok(ApproveQualityUpgradeResultGql {
                success: true,
                error: None,
            }),
            Err(e) => Ok(ApproveQualityUpgradeResultGql {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }
}

async fn approve_quality_upgrade_inner(
    manager: &Arc<ServicesManager>,
    notification_id: &str,
) -> anyhow::Result<()> {
    let db_service = manager
        .get_database()
        .await
        .ok_or_else(|| anyhow::anyhow!("database service not available"))?;
    let db = db_service.pool().clone();

    let notifications = Notification::query(db.pool())
        .filter(NotificationWhereInput {
            id: Some(string_eq(notification_id)),
            ..Default::default()
        })
        .fetch_all()
        .await?;
    let notification = notifications
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("notification not found"))?;

    if notification.action_type.as_deref() != Some(QUALITY_UPGRADE_ACTION_TYPE) {
        anyhow::bail!("notification is not a quality-upgrade notification");
    }
    if notification.resolved_at.is_some() {
        anyhow::bail!("notification already resolved");
    }

    let action_data: serde_json::Value = notification
        .action_data
        .as_deref()
        .map(serde_json::from_str)
        .transpose()?
        .ok_or_else(|| anyhow::anyhow!("notification has no actionData"))?;

    let source_path = action_data
        .get("sourcePath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("actionData missing sourcePath"))?;
    let target_path = action_data
        .get("targetPath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("actionData missing targetPath"))?;
    let media_file_id = action_data
        .get("mediaFileId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("actionData missing mediaFileId"))?;

    replace_file_on_disk(Path::new(source_path), Path::new(target_path)).await?;

    // Re-probe the replaced file and recompute its quality status. This is
    // fire-and-forget (queued), matching how `analyzeMediaFile` already works
    // elsewhere in the pipeline (see `LibraryScanService::queue_analyze_job`).
    if let Some(scan_service) = manager.get_library_scan().await {
        let _ = scan_service
            .queue_analyze_job(media_file_id, target_path)
            .await;
    }

    let now = chrono::Utc::now().to_rfc3339();
    Notification::update_by_id(
        &db,
        &notification.id,
        UpdateNotificationInput {
            resolved_at: Some(Some(now.clone())),
            resolution: Some(Some("ACCEPTED".to_string())),
            read_at: Some(Some(now)),
            ..Default::default()
        },
    )
    .await?;

    Ok(())
}

/// Replace `target` with `source`'s contents, clobbering the existing file.
/// Only called after explicit user approval (Q38 — never automatic).
/// Hardlinks when possible (same filesystem, matches the rest of the
/// pipeline's import behavior), falling back to a copy.
async fn replace_file_on_disk(source: &Path, target: &Path) -> anyhow::Result<()> {
    if !source.exists() {
        anyhow::bail!("source file no longer exists: {}", source.display());
    }
    if target.exists() {
        tokio::fs::remove_file(target).await?;
    }
    if tokio::fs::hard_link(source, target).await.is_err() {
        tokio::fs::copy(source, target).await?;
    }
    Ok(())
}
