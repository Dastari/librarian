//! Settings › Updates: the server's version, the newest release on its channel, "Check now" and
//! "Upgrade" (design.md Q151–Q152). Admin only, except the TV app's newest version, which any
//! signed-in client may read. The channel and the automatic check are ordinary app settings
//! (`updates.channel`, `updates.auto_check`) written through the generated mutations.

use async_graphql::{Context, Object, Result, SimpleObject};

use crate::services::graphql::auth::{AuthExt, RoleGuard};
use crate::services::updates::{self, Progress, Stage, Status, UpgradeRecord, github::ReleaseInfo};

#[derive(Default)]
pub struct ServerUpdateQueries;

#[derive(Default)]
pub struct ServerUpdateMutations;

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "ServerRelease", rename_fields = "camelCase")]
pub struct ServerReleaseGql {
    pub version: String,
    pub tag: String,
    pub name: Option<String>,
    pub published_at: Option<String>,
    /// The release page on GitHub.
    pub url: String,
    /// Markdown as written on GitHub. Show it as text; never as HTML.
    pub notes: Option<String>,
    pub prerelease: bool,
    /// This platform's archive, when the release has one.
    pub asset_name: Option<String>,
    pub asset_size: Option<i64>,
    pub has_checksums: bool,
}

impl ServerReleaseGql {
    fn from_release(release: ReleaseInfo, os: &str, arch: &str) -> Self {
        let asset = updates::github::platform_asset(&release, os, arch).cloned();
        Self {
            has_checksums: release.asset(updates::github::CHECKSUMS_ASSET).is_some(),
            version: release.version,
            tag: release.tag,
            name: release.name,
            published_at: release.published_at,
            url: release.url,
            notes: release.notes,
            prerelease: release.prerelease,
            asset_name: asset.as_ref().map(|asset| asset.name.clone()),
            asset_size: asset.map(|asset| asset.size as i64),
        }
    }
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "ServerUpgradeProgress", rename_fields = "camelCase")]
pub struct ServerUpgradeProgressGql {
    pub target_version: String,
    /// `downloading`, `verifying`, `backing_up`, `installing`, `restarting` or `failed`.
    pub stage: String,
    pub message: String,
    pub started_at: String,
    pub updated_at: String,
    pub failed: bool,
}

impl From<Progress> for ServerUpgradeProgressGql {
    fn from(value: Progress) -> Self {
        Self {
            target_version: value.target,
            stage: value.stage.as_str().to_string(),
            message: value.message,
            started_at: value.started_at.to_rfc3339(),
            updated_at: value.updated_at.to_rfc3339(),
            failed: value.stage == Stage::Failed,
        }
    }
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "ServerUpgradeRecord", rename_fields = "camelCase")]
pub struct ServerUpgradeRecordGql {
    pub from_version: String,
    pub to_version: String,
    pub at: String,
    pub by: String,
    /// `upgraded`, `rolled_back` or `failed`.
    pub outcome: String,
    pub detail: Option<String>,
    pub backup_snapshot_id: Option<String>,
}

impl From<UpgradeRecord> for ServerUpgradeRecordGql {
    fn from(value: UpgradeRecord) -> Self {
        Self {
            from_version: value.from,
            to_version: value.to,
            at: value.at,
            by: value.by,
            outcome: value.outcome,
            detail: value.detail,
            backup_snapshot_id: value.backup_snapshot_id,
        }
    }
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "TvAppRelease", rename_fields = "camelCase")]
pub struct TvAppReleaseGql {
    pub version: String,
    pub tag: String,
    pub published_at: Option<String>,
    pub url: String,
    /// The APK's download link, when the release has one.
    pub apk_url: Option<String>,
}

impl From<ReleaseInfo> for TvAppReleaseGql {
    fn from(value: ReleaseInfo) -> Self {
        let apk_url = value
            .assets
            .iter()
            .find(|asset| asset.name.ends_with(".apk"))
            .map(|asset| asset.url.clone());
        Self {
            version: value.version,
            tag: value.tag,
            published_at: value.published_at,
            url: value.url,
            apk_url,
        }
    }
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "ServerUpdateStatus", rename_fields = "camelCase")]
pub struct ServerUpdateStatusGql {
    pub current_version: String,
    /// `release`, `debug`, `docker`, `dev` or `source`.
    pub build_kind: String,
    /// `linux-x86_64`, `windows-x86_64`, …
    pub platform: String,
    /// The GitHub repository releases come from.
    pub repository: String,
    /// `stable` or `dev`.
    pub channel: String,
    /// The channel was not chosen: it follows the running version.
    pub channel_is_default: bool,
    pub auto_check: bool,
    pub last_checked_at: Option<String>,
    pub last_error: Option<String>,
    pub next_check_at: Option<String>,
    /// The newest release on the channel.
    pub latest: Option<ServerReleaseGql>,
    pub update_available: bool,
    pub can_self_upgrade: bool,
    /// Why this install can't upgrade itself (its build, platform or folder), or why the latest
    /// release can't be installed (no checksums).
    pub self_upgrade_blocked_reason: Option<String>,
    /// What to do instead.
    pub self_upgrade_instructions: Option<String>,
    pub progress: Option<ServerUpgradeProgressGql>,
    pub history: Vec<ServerUpgradeRecordGql>,
    pub tv_app: Option<TvAppReleaseGql>,
}

impl ServerUpdateStatusGql {
    fn from_status(status: Status, os: &str, arch: &str) -> Self {
        let can_self_upgrade = status.can_self_upgrade();
        let (reason, instead) = match (&status.blocked, &status.not_installable) {
            (Some(blocked), _) => (Some(blocked.reason.clone()), Some(blocked.instead.clone())),
            (None, Some(reason)) => (Some(reason.clone()), None),
            (None, None) => (None, None),
        };
        Self {
            current_version: status.current_version,
            build_kind: status.build_kind.to_string(),
            platform: status.platform,
            repository: status.repository,
            channel: status.channel.as_str().to_string(),
            channel_is_default: status.channel_is_default,
            auto_check: status.auto_check,
            last_checked_at: status.last_checked_at,
            last_error: status.last_error,
            next_check_at: status.next_check_at,
            latest: status
                .latest
                .map(|release| ServerReleaseGql::from_release(release, os, arch)),
            update_available: status.update_available,
            can_self_upgrade,
            self_upgrade_blocked_reason: reason,
            self_upgrade_instructions: instead,
            progress: status.progress.map(Into::into),
            history: status.history.into_iter().map(Into::into).collect(),
            tv_app: status.tv_app.map(Into::into),
        }
    }
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "CheckForServerUpdateResult", rename_fields = "camelCase")]
pub struct CheckForServerUpdateResultGql {
    pub success: bool,
    pub error: Option<String>,
    pub status: Option<ServerUpdateStatusGql>,
}

#[derive(Clone, Debug, SimpleObject)]
#[graphql(name = "UpgradeServerResult", rename_fields = "camelCase")]
pub struct UpgradeServerResultGql {
    pub success: bool,
    pub error: Option<String>,
}

fn updater() -> Result<std::sync::Arc<updates::Updater>> {
    updates::updater()
        .ok_or_else(|| async_graphql::Error::new("The update service has not started"))
}

async fn status_gql() -> Result<ServerUpdateStatusGql> {
    let updater = updater()?;
    let status = updater.status().await?;
    Ok(ServerUpdateStatusGql::from_status(
        status,
        updater.facts().os,
        updater.facts().arch,
    ))
}

#[Object(name = "ServerUpdateQuery")]
impl ServerUpdateQueries {
    #[graphql(name = "serverUpdateStatus", guard = "RoleGuard::new(\"Admin\")")]
    async fn server_update_status(&self) -> Result<ServerUpdateStatusGql> {
        status_gql().await
    }

    /// The newest Android TV app release the last check found, for the TV shell to offer.
    #[graphql(name = "tvAppRelease", guard = "RoleGuard::new(\"Member\")")]
    async fn tv_app_release(&self, ctx: &Context<'_>) -> Result<Option<TvAppReleaseGql>> {
        let db = ctx.data::<crate::db::Database>()?;
        Ok(updates::load_record(db)
            .await
            .catalogue
            .newest_tv
            .map(Into::into))
    }
}

#[Object(name = "ServerUpdateMutation")]
impl ServerUpdateMutations {
    /// Ask GitHub now. Pressed again within 20 seconds, it returns the last answer.
    #[graphql(name = "checkForServerUpdate", guard = "RoleGuard::new(\"Admin\")")]
    async fn check_for_server_update(&self) -> Result<CheckForServerUpdateResultGql> {
        let error = updater()?
            .check(true)
            .await
            .err()
            .map(|error| format!("{error:#}"));
        Ok(CheckForServerUpdateResultGql {
            success: error.is_none(),
            error,
            status: Some(status_gql().await?),
        })
    }

    /// Upgrade to `version`, which must be the newest release on the channel (the confirmation
    /// names it). Returns once the download has started; `serverUpdateStatus.progress` follows it,
    /// and the server restarts when the new binary is in place.
    #[graphql(name = "upgradeServer", guard = "RoleGuard::new(\"Admin\")")]
    async fn upgrade_server(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "version")] version: String,
    ) -> Result<UpgradeServerResultGql> {
        let user = ctx.require_admin()?;
        let db = ctx.data::<crate::db::Database>()?;
        let by = crate::graphql::entities::User::get(db.pool(), &user.user_id)
            .await
            .ok()
            .flatten()
            .map(|row| row.username)
            .unwrap_or_else(|| user.user_id.clone());
        match updater()?.start_upgrade(&version, &by).await {
            Ok(()) => Ok(UpgradeServerResultGql {
                success: true,
                error: None,
            }),
            Err(error) => Ok(UpgradeServerResultGql {
                success: false,
                error: Some(format!("{error:#}")),
            }),
        }
    }
}
