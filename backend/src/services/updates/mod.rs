//! Server updates: checking GitHub for new releases, telling admins, and upgrading in place
//! (design.md Q151–Q152).
//!
//! - **The check** ([`Updater::check`]) reads the release list every 6 hours (first 2 minutes after
//!   startup) and on "Check now", with an ETag, and records the newest stable, newest overall and
//!   newest TV app release in the `updates.state` app setting. GitHub's rate limits set a time
//!   before which nothing is asked again.
//! - **The channel** is the `updates.channel` app setting (`stable` or `dev`); unset, it is Dev when
//!   the running version is a pre-release and Stable otherwise. `updates.auto_check` (default on)
//!   turns the background check off; "Check now" still works.
//! - **The notification** goes to every admin once per version ([`notify`]).
//! - **The upgrade** ([`Updater::start_upgrade`]): admin only, release builds on Linux and Windows
//!   only ([`version::self_upgrade_block`]). Download the platform archive and `SHA256SUMS` from the
//!   configured repository, verify, unpack the one binary, run it with `--version`, take and verify
//!   a full backup, swap it in, restart. [`install`] has the swap, the marker and the rollback.
//! - **The record.** Every attempt ends up in `updates.history` (from, to, when, who, outcome).

pub mod github;
pub mod install;
pub mod notify;
pub mod version;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use graphql_orm::graphql::filters::StringFilter;
use semver::Version;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use self::github::{Catalogue, FetchOutcome, ReleaseInfo};
use self::install::{InstallPaths, Marker, MarkerState, Startup, SwapStyle};
use self::version::{Blocked, BuildFacts, Channel, running_version};
use crate::db::Database;
use crate::graphql::entities::{
    AppSetting, AppSettingWhereInput, CreateAppSettingInput, UpdateAppSettingInput,
};
use crate::services::manager::ServicesManager;

pub const SETTING_CATEGORY: &str = "updates";
pub const SETTING_CHANNEL: &str = "updates.channel";
pub const SETTING_AUTO_CHECK: &str = "updates.auto_check";
/// Written by the checker: what GitHub last said.
pub const SETTING_STATE: &str = "updates.state";
/// Written by upgrades: what happened, newest first.
pub const SETTING_HISTORY: &str = "updates.history";

const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(120);
/// "Check now" pressed again within this reuses the last answer.
const MANUAL_CHECK_GAP: chrono::Duration = chrono::Duration::seconds(20);
const HISTORY_LIMIT: usize = 20;

// ===========================================================================
// What is stored
// ===========================================================================

/// The checker's record (`updates.state`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CheckRecord {
    pub repository: Option<String>,
    pub last_checked_at: Option<String>,
    pub last_error: Option<String>,
    pub etag: Option<String>,
    /// GitHub asked to wait until then.
    pub wait_until: Option<String>,
    pub catalogue: Catalogue,
    /// Versions already announced to the admins.
    pub notified: Vec<String>,
}

/// One upgrade attempt (`updates.history`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeRecord {
    pub from: String,
    pub to: String,
    pub at: String,
    pub by: String,
    /// `upgraded`, `rolled_back` or `failed`.
    pub outcome: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub backup_snapshot_id: Option<String>,
}

async fn find_setting(db: &Database, key: &str) -> Option<AppSetting> {
    AppSetting::query(db.pool())
        .filter(AppSettingWhereInput {
            key: Some(StringFilter {
                eq: Some(key.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await
        .ok()
        .and_then(|rows| rows.into_iter().next())
}

async fn put_setting(db: &Database, key: &str, value: String, description: &str) -> Result<()> {
    match find_setting(db, key).await {
        Some(row) => {
            AppSetting::update_by_id(
                db,
                &row.id,
                UpdateAppSettingInput {
                    value: Some(value),
                    ..Default::default()
                },
            )
            .await?;
        }
        None => {
            AppSetting::insert(
                db,
                CreateAppSettingInput {
                    key: key.to_string(),
                    value,
                    description: Some(description.to_string()),
                    category: SETTING_CATEGORY.to_string(),
                },
            )
            .await?;
        }
    }
    Ok(())
}

async fn load_json<T: serde::de::DeserializeOwned + Default>(db: &Database, key: &str) -> T {
    find_setting(db, key)
        .await
        .and_then(|row| serde_json::from_str(&row.value).ok())
        .unwrap_or_default()
}

pub async fn load_record(db: &Database) -> CheckRecord {
    load_json(db, SETTING_STATE).await
}

async fn save_record(db: &Database, record: &CheckRecord) {
    if let Err(error) = put_setting(
        db,
        SETTING_STATE,
        serde_json::to_string(record).unwrap_or_default(),
        "Server updates: what the last check of GitHub found",
    )
    .await
    {
        warn!(error = %error, "Updates: could not save what the release check found");
    }
}

pub async fn load_history(db: &Database) -> Vec<UpgradeRecord> {
    load_json(db, SETTING_HISTORY).await
}

pub async fn record_history(db: &Database, entry: UpgradeRecord) {
    let mut history = load_history(db).await;
    history.insert(0, entry);
    history.truncate(HISTORY_LIMIT);
    if let Err(error) = put_setting(
        db,
        SETTING_HISTORY,
        serde_json::to_string(&history).unwrap_or_default(),
        "Server updates: upgrades done from Settings › Updates",
    )
    .await
    {
        warn!(error = %error, "Updates: could not record an upgrade in the history");
    }
}

/// The channel the admin chose, or the default for the running version. The flag says whether it
/// is the default.
pub async fn channel(db: &Database, running: &Version) -> (Channel, bool) {
    let chosen = crate::services::torrent::get_setting_string(db, SETTING_CHANNEL)
        .await
        .ok()
        .flatten()
        .and_then(|value| Channel::parse(&value));
    match chosen {
        Some(channel) => (channel, false),
        None => (Channel::default_for(running), true),
    }
}

pub async fn auto_check(db: &Database) -> bool {
    crate::services::torrent::get_setting::<serde_json::Value>(db, SETTING_AUTO_CHECK)
        .await
        .ok()
        .flatten()
        .map(|value| match value {
            serde_json::Value::Bool(on) => on,
            serde_json::Value::String(text) => !matches!(text.trim(), "false" | "0" | "off"),
            _ => true,
        })
        .unwrap_or(true)
}

fn parse_time(value: Option<&str>) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value?)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}

// ===========================================================================
// The upgrade's progress
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Downloading,
    Verifying,
    BackingUp,
    Installing,
    Restarting,
    Failed,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Downloading => "downloading",
            Self::Verifying => "verifying",
            Self::BackingUp => "backing_up",
            Self::Installing => "installing",
            Self::Restarting => "restarting",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Progress {
    pub target: String,
    pub stage: Stage,
    pub message: String,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ===========================================================================
// The updater
// ===========================================================================

/// What Settings › Updates shows.
#[derive(Debug, Clone)]
pub struct Status {
    pub current_version: String,
    pub build_kind: &'static str,
    pub platform: String,
    pub repository: String,
    pub channel: Channel,
    pub channel_is_default: bool,
    pub auto_check: bool,
    pub last_checked_at: Option<String>,
    pub last_error: Option<String>,
    pub next_check_at: Option<String>,
    pub latest: Option<ReleaseInfo>,
    pub update_available: bool,
    /// Why the latest release can't be installed by Librarian itself, when it can't.
    pub not_installable: Option<String>,
    pub blocked: Option<Blocked>,
    pub progress: Option<Progress>,
    pub history: Vec<UpgradeRecord>,
    pub tv_app: Option<ReleaseInfo>,
}

impl Status {
    pub fn can_self_upgrade(&self) -> bool {
        self.update_available
            && self.blocked.is_none()
            && self.not_installable.is_none()
            && self
                .progress
                .as_ref()
                .is_none_or(|progress| progress.stage == Stage::Failed)
    }
}

pub struct Updater {
    manager: Arc<ServicesManager>,
    facts: BuildFacts,
    repository: String,
    check_lock: tokio::sync::Mutex<()>,
    progress: Mutex<Option<Progress>>,
    next_check_at: Mutex<Option<DateTime<Utc>>>,
}

static UPDATER: OnceLock<Arc<Updater>> = OnceLock::new();

pub fn updater() -> Option<Arc<Updater>> {
    UPDATER.get().cloned()
}

/// Create the updater and start the background check. Call once, after the services started.
pub fn start(manager: Arc<ServicesManager>, tui: bool) {
    let updater = Arc::new(Updater {
        manager,
        facts: BuildFacts::current(tui),
        repository: github::repository(),
        check_lock: tokio::sync::Mutex::new(()),
        progress: Mutex::new(None),
        next_check_at: Mutex::new(None),
    });
    if UPDATER.set(updater.clone()).is_err() {
        return;
    }
    info!(
        version = %running_version(),
        build = updater.facts.kind().as_str(),
        repository = %updater.repository,
        "Updates: running Librarian {} ({} build); checking {} for releases",
        running_version(),
        updater.facts.kind().as_str(),
        updater.repository
    );
    tokio::spawn(async move {
        let mut wait = FIRST_CHECK_DELAY;
        loop {
            updater
                .set_next_check(Utc::now() + chrono::Duration::from_std(wait).unwrap_or_default());
            tokio::time::sleep(wait).await;
            wait = CHECK_INTERVAL;
            let Some(db) = updater.db().await else {
                continue;
            };
            if !auto_check(&db).await {
                continue;
            }
            if let Err(error) = updater.check(false).await {
                warn!(error = %error, "Updates: the release check failed: {error:#}");
            }
        }
    });
}

impl Updater {
    async fn db(&self) -> Option<Database> {
        Some(self.manager.get_database().await?.pool().clone())
    }

    fn set_next_check(&self, at: DateTime<Utc>) {
        *self.next_check_at.lock().unwrap_or_else(|e| e.into_inner()) = Some(at);
    }

    fn progress(&self) -> Option<Progress> {
        self.progress
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn set_progress(&self, target: &str, stage: Stage, message: impl Into<String>) {
        let message = message.into();
        info!(target_version = %target, stage = stage.as_str(), "Updates: upgrade to {target}: {message}");
        let mut slot = self.progress.lock().unwrap_or_else(|e| e.into_inner());
        let started_at = slot
            .as_ref()
            .filter(|progress| progress.target == target && progress.stage != Stage::Failed)
            .map(|progress| progress.started_at)
            .unwrap_or_else(Utc::now);
        *slot = Some(Progress {
            target: target.to_string(),
            stage,
            message,
            started_at,
            updated_at: Utc::now(),
        });
    }

    /// Mark an upgrade as started, unless one already is (checked and set under one lock, so two
    /// presses can't both start).
    fn claim(&self, target: &str) -> bool {
        let mut slot = self.progress.lock().unwrap_or_else(|e| e.into_inner());
        if slot
            .as_ref()
            .is_some_and(|progress| progress.stage != Stage::Failed)
        {
            return false;
        }
        *slot = Some(Progress {
            target: target.to_string(),
            stage: Stage::Downloading,
            message: "Starting".to_string(),
            started_at: Utc::now(),
            updated_at: Utc::now(),
        });
        true
    }

    pub fn facts(&self) -> &BuildFacts {
        &self.facts
    }

    /// Ask GitHub, record the answer and announce a new version. `manual` is "Check now": it
    /// reports a rate-limit wait as an error instead of skipping quietly.
    pub async fn check(&self, manual: bool) -> Result<()> {
        let _guard = self.check_lock.lock().await;
        let db = self.db().await.context("database service not available")?;
        let mut record = load_record(&db).await;
        if record.repository.as_deref() != Some(self.repository.as_str()) {
            record = CheckRecord {
                repository: Some(self.repository.clone()),
                notified: record.notified,
                ..Default::default()
            };
        }
        let now = Utc::now();
        if let Some(until) = parse_time(record.wait_until.as_deref()).filter(|until| *until > now) {
            if manual {
                bail!(
                    "GitHub asked Librarian to wait before asking again; try after {}",
                    until.format("%H:%M UTC")
                );
            }
            return Ok(());
        }
        if manual
            && parse_time(record.last_checked_at.as_deref())
                .is_some_and(|at| now - at < MANUAL_CHECK_GAP)
            && record.last_error.is_none()
        {
            return Ok(());
        }
        let etag = (record.catalogue != Catalogue::default())
            .then(|| record.etag.clone())
            .flatten();
        let outcome = github::fetch_releases(&self.repository, etag.as_deref()).await;
        record.last_checked_at = Some(now.to_rfc3339());
        let result = match outcome {
            Ok(FetchOutcome::NotModified { wait_until }) => {
                record.last_error = None;
                record.wait_until = wait_until.map(|at| at.to_rfc3339());
                Ok(())
            }
            Ok(FetchOutcome::Fresh {
                releases,
                etag,
                wait_until,
            }) => {
                record.catalogue = github::catalogue(&releases, &self.repository);
                record.etag = etag;
                record.last_error = None;
                record.wait_until = wait_until.map(|at| at.to_rfc3339());
                Ok(())
            }
            Ok(FetchOutcome::RateLimited { until }) => {
                record.wait_until = Some(until.to_rfc3339());
                let message = format!(
                    "GitHub's rate limit was reached; Librarian asks again after {}",
                    until.format("%H:%M UTC")
                );
                record.last_error = Some(message.clone());
                Err(anyhow::anyhow!(message))
            }
            Err(error) => {
                record.last_error = Some(format!("{error:#}"));
                Err(error)
            }
        };
        self.announce(&db, &mut record).await;
        save_record(&db, &record).await;
        result
    }

    async fn announce(&self, db: &Database, record: &mut CheckRecord) {
        let running = running_version();
        let (channel, _) = channel(db, &running).await;
        let latest = record.catalogue.newest(channel).cloned();
        notify::resolve_outdated(
            db,
            &running,
            latest.as_ref().and_then(ReleaseInfo::semver).as_ref(),
        )
        .await;
        let Some(due) = notify::version_to_announce(latest.as_ref(), &running, &record.notified)
        else {
            return;
        };
        let blocked = self.why_not(due);
        if notify::announce(db, due, &running, blocked.as_deref()).await {
            notify::remember(&mut record.notified, &due.version);
        }
    }

    /// Why this install can't upgrade itself to `release`, in one sentence pair, or `None`.
    fn why_not(&self, release: &ReleaseInfo) -> Option<String> {
        if let Some(blocked) = version::self_upgrade_block(&self.facts) {
            return Some(format!("{} {}", blocked.reason, blocked.instead));
        }
        github::installable(release, self.facts.os, self.facts.arch)
            .err()
            .or_else(|| self.dir_problem())
    }

    fn dir_problem(&self) -> Option<String> {
        let exe = self.facts.exe_path.as_ref()?;
        let dir = InstallPaths::new(exe).dir().to_path_buf();
        (!install::dir_writable(&dir)).then(|| {
            format!(
                "Librarian can't write to {}, where its binary is. Give the service user write access there, or replace the binary by hand.",
                dir.display()
            )
        })
    }

    pub async fn status(&self) -> Result<Status> {
        let db = self.db().await.context("database service not available")?;
        let running = running_version();
        let record = load_record(&db).await;
        let (channel, channel_is_default) = channel(&db, &running).await;
        let latest = record.catalogue.newest(channel).cloned();
        let update_available = latest
            .as_ref()
            .and_then(ReleaseInfo::semver)
            .is_some_and(|version| version > running);
        let blocked = version::self_upgrade_block(&self.facts);
        let not_installable = match (&latest, update_available, &blocked) {
            (Some(release), true, None) => {
                github::installable(release, self.facts.os, self.facts.arch)
                    .err()
                    .or_else(|| self.dir_problem())
            }
            _ => None,
        };
        let next_check_at = if auto_check(&db).await {
            let scheduled = *self.next_check_at.lock().unwrap_or_else(|e| e.into_inner());
            let wait = parse_time(record.wait_until.as_deref());
            scheduled
                .map(|at| wait.map_or(at, |wait| at.max(wait)))
                .map(|at| at.to_rfc3339())
        } else {
            None
        };
        Ok(Status {
            current_version: running.to_string(),
            build_kind: self.facts.kind().as_str(),
            platform: self.facts.platform(),
            repository: self.repository.clone(),
            channel,
            channel_is_default,
            auto_check: auto_check(&db).await,
            last_checked_at: record.last_checked_at.clone(),
            last_error: record.last_error.clone(),
            next_check_at,
            latest,
            update_available,
            not_installable,
            blocked,
            progress: self.progress(),
            history: load_history(&db).await,
            tv_app: record.catalogue.newest_tv.clone(),
        })
    }

    /// Start upgrading to `version`, which must be the newest release on the channel. Returns once
    /// the work is under way; [`Status::progress`] follows it.
    pub async fn start_upgrade(self: &Arc<Self>, version: &str, by: &str) -> Result<()> {
        let status = self.status().await?;
        if let Some(blocked) = &status.blocked {
            bail!("{} {}", blocked.reason, blocked.instead);
        }
        let release = status
            .latest
            .clone()
            .context("No release is known yet; check for updates first")?;
        if release.version != version.trim().trim_start_matches('v') {
            bail!(
                "{} is not the newest release on the {} channel ({} is); check again",
                version,
                status.channel.as_str(),
                release.version
            );
        }
        if !status.update_available {
            bail!("This server already runs {}", status.current_version);
        }
        if let Some(reason) = status.not_installable {
            bail!(reason);
        }
        if !self.claim(&release.version) {
            bail!("An upgrade is already under way");
        }
        let updater = self.clone();
        let by = by.to_string();
        tokio::spawn(async move {
            let target = release.version.clone();
            if let Err(error) = updater.upgrade(&release, &by).await {
                let detail = format!("{error:#}");
                error!(target_version = %target, error = %detail, "Updates: the upgrade to {target} failed: {detail}");
                updater.set_progress(&target, Stage::Failed, detail.clone());
                if let Some(db) = updater.db().await {
                    record_history(
                        &db,
                        UpgradeRecord {
                            from: running_version().to_string(),
                            to: target,
                            at: Utc::now().to_rfc3339(),
                            by,
                            outcome: "failed".to_string(),
                            detail: Some(detail),
                            backup_snapshot_id: None,
                        },
                    )
                    .await;
                }
            }
        });
        Ok(())
    }

    async fn upgrade(&self, release: &ReleaseInfo, by: &str) -> Result<()> {
        let target = release.version.as_str();
        let expected = release
            .semver()
            .context("the release tag is not a version")?;
        let (archive, sums) = github::installable(release, self.facts.os, self.facts.arch)
            .map_err(anyhow::Error::msg)?;
        let exe = self
            .facts
            .exe_path
            .clone()
            .context("Librarian can't tell which file it runs from")?;
        let paths = InstallPaths::new(exe);
        let staging = paths.staging();
        let _ = tokio::fs::remove_dir_all(&staging).await;
        tokio::fs::create_dir_all(&staging)
            .await
            .with_context(|| format!("Could not create {}", staging.display()))?;
        let result = self
            .download_verify_install(release, &expected, archive, sums, &paths, &staging, by)
            .await;
        let _ = tokio::fs::remove_dir_all(&staging).await;
        result?;

        info!(
            from = %running_version(),
            to = %target,
            by = %by,
            "Updates: Librarian {target} is in place at {}; restarting into it",
            paths.exe.display()
        );
        if let Some(db) = self.db().await {
            notify::resolve_outdated(&db, &expected, None).await;
        }
        self.set_progress(
            target,
            Stage::Restarting,
            format!("Restarting into {target}. This page reconnects by itself."),
        );
        request_restart(paths.exe.clone());
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn download_verify_install(
        &self,
        release: &ReleaseInfo,
        expected: &Version,
        archive: &github::AssetInfo,
        sums: &github::AssetInfo,
        paths: &InstallPaths,
        staging: &std::path::Path,
        by: &str,
    ) -> Result<()> {
        let target = release.version.as_str();
        self.set_progress(
            target,
            Stage::Downloading,
            format!(
                "Downloading {} ({} MB)",
                archive.name,
                archive.size.div_ceil(1024 * 1024)
            ),
        );
        let sums_text =
            github::download_small(&sums.url, &self.repository, github::CHECKSUMS_SIZE_CAP)
                .await
                .context("Could not download SHA256SUMS")?;
        let sums_text = String::from_utf8(sums_text).context("SHA256SUMS is not text")?;
        let archive_path = staging.join(&archive.name);
        let actual = github::download_to_file(
            &archive.url,
            &self.repository,
            archive.size,
            github::ARCHIVE_SIZE_CAP,
            &archive_path,
        )
        .await
        .with_context(|| format!("Could not download {}", archive.name))?;

        self.set_progress(target, Stage::Verifying, "Checking the SHA-256 checksum");
        github::verify_checksum(&sums_text, &archive.name, &actual)?;
        let binary_name = github::binary_name(self.facts.os);
        let unpacked = staging.join(binary_name);
        {
            let archive_path = archive_path.clone();
            let unpacked = unpacked.clone();
            tokio::task::spawn_blocking(move || {
                install::extract_binary(&archive_path, binary_name, &unpacked)
            })
            .await
            .context("unpacking panicked")??;
        }
        self.set_progress(
            target,
            Stage::Verifying,
            "Checking that the new binary runs",
        );
        install::preflight(&unpacked, expected).await?;

        self.set_progress(target, Stage::BackingUp, "Taking a full database backup");
        let backup = self
            .manager
            .get_backup()
            .await
            .context("The backup service is not available, so Librarian won't upgrade")?;
        let snapshot = backup
            .create_full_backup()
            .await
            .context("The backup failed, so Librarian won't upgrade")?
            .context("The backup returned no snapshot, so Librarian won't upgrade")?;
        backup
            .verify_snapshot(&snapshot.snapshot_id)
            .await
            .context("The backup did not verify, so Librarian won't upgrade")?;
        info!(snapshot = %snapshot.snapshot_id, "Updates: backup {} taken before upgrading to {target}", snapshot.snapshot_id);

        self.set_progress(target, Stage::Installing, "Swapping in the new binary");
        tokio::fs::rename(&unpacked, paths.incoming())
            .await
            .with_context(|| {
                format!(
                    "Could not move the new binary to {}",
                    paths.incoming().display()
                )
            })?;
        let marker = Marker {
            state: MarkerState::Pending,
            from: running_version().to_string(),
            to: target.to_string(),
            by: by.to_string(),
            at: Utc::now().to_rfc3339(),
            attempts: 0,
            reason: None,
            backup_snapshot_id: Some(snapshot.snapshot_id.clone()),
        };
        install::write_marker(paths, &marker)?;
        if let Err(error) = install::swap_in(paths, SwapStyle::for_this_platform()) {
            install::remove_marker(paths);
            let _ = std::fs::remove_file(paths.incoming());
            return Err(error);
        }
        Ok(())
    }
}

// ===========================================================================
// Restarting
// ===========================================================================

static RESTART: OnceLock<PathBuf> = OnceLock::new();
static RESTART_NOTIFY: OnceLock<tokio::sync::Notify> = OnceLock::new();

fn restart_notify() -> &'static tokio::sync::Notify {
    RESTART_NOTIFY.get_or_init(tokio::sync::Notify::new)
}

fn request_restart(exe: PathBuf) {
    let _ = RESTART.set(exe);
    restart_notify().notify_one();
}

/// Resolves when an upgrade wants the server to stop and restart. The main loop then stops the
/// services as for a shutdown and calls [`restart_if_requested`].
pub async fn restart_requested() {
    restart_notify().notified().await;
}

/// After a clean shutdown: restart into the new binary if an upgrade asked for it.
pub fn restart_if_requested() {
    if let Some(exe) = RESTART.get() {
        info!("Updates: restarting into {}", exe.display());
        install::restart_into(exe);
    }
}

// ===========================================================================
// Startup: trials and rollbacks
// ===========================================================================

/// What the start of the process found, kept until the database is up to record it.
static STARTUP: Mutex<Option<Startup>> = Mutex::new(None);
static READY: AtomicBool = AtomicBool::new(false);

/// Server invocations only: the offline commands and `--version` must not count as a start.
pub fn is_server_invocation(args: &[String]) -> bool {
    !args.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "backup" | "schema" | "credentials" | "repair" | "--version" | "-V"
        )
    })
}

/// Run first thing in a server start, before configuration is read, so even a new binary that
/// cannot read its configuration is rolled back on its next start. See [`install`] for the marker.
pub fn on_process_start() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !is_server_invocation(&args) {
        return;
    }
    let Some(exe) = version::current_exe() else {
        return;
    };
    let paths = InstallPaths::new(exe);
    let decision = install::startup_decision(install::read_marker(&paths), &running_version());
    let keep = match decision {
        Startup::Normal => return,
        Startup::Trial(mut marker) => {
            marker.attempts += 1;
            if let Err(error) = install::write_marker(&paths, &marker) {
                eprintln!(
                    "Librarian upgrade: could not count this start of {}: {error:#}",
                    marker.to
                );
            }
            eprintln!(
                "Librarian upgrade: first start of {} (from {}); it rolls back unless it is ready within {} seconds",
                marker.to,
                marker.from,
                install::READY_DEADLINE.as_secs()
            );
            spawn_watchdog(paths.clone());
            Startup::Trial(marker)
        }
        Startup::RollBack(marker) => {
            let reason = format!(
                "Librarian {} stopped before it was ready, so {} was put back.",
                marker.to, marker.from
            );
            eprintln!("Librarian upgrade: {reason}");
            match install::roll_back_and_restart(&paths, marker.clone(), &reason) {
                Ok(()) => return,
                Err(error) => {
                    eprintln!(
                        "Librarian upgrade: the rollback failed: {error:#}; carrying on with {}",
                        marker.to
                    );
                    install::remove_marker(&paths);
                    let mut marker = marker;
                    marker.reason = Some(format!(
                        "Librarian {} failed its first start and could not be rolled back: {error:#}",
                        marker.to
                    ));
                    Startup::Stale(marker)
                }
            }
        }
        Startup::RolledBack(marker) => {
            install::remove_marker(&paths);
            Startup::RolledBack(marker)
        }
        Startup::Stale(marker) => {
            install::remove_marker(&paths);
            // Only worth reporting when the binary it started from is still the one running; a
            // binary someone put in place by hand has nothing to say about it.
            if marker.from != running_version().to_string() {
                return;
            }
            Startup::Stale(marker)
        }
    };
    *STARTUP.lock().unwrap_or_else(|e| e.into_inner()) = Some(keep);
}

fn spawn_watchdog(paths: InstallPaths) {
    let spawned = std::thread::Builder::new()
        .name("librarian-upgrade-watchdog".to_string())
        .spawn(move || {
            let started = std::time::Instant::now();
            while started.elapsed() < install::READY_DEADLINE {
                if READY.load(Ordering::Acquire) {
                    return;
                }
                std::thread::sleep(Duration::from_secs(1));
            }
            if READY.load(Ordering::Acquire) {
                return;
            }
            let Some(marker) = install::read_marker(&paths) else {
                return;
            };
            let reason = format!(
                "Librarian {} was not ready within {} seconds, so {} was put back.",
                marker.to,
                install::READY_DEADLINE.as_secs(),
                marker.from
            );
            eprintln!("Librarian upgrade: {reason}");
            if let Err(error) = install::roll_back_and_restart(&paths, marker, &reason) {
                eprintln!("Librarian upgrade: the rollback failed: {error:#}");
            }
        });
    if let Err(error) = spawned {
        eprintln!("Librarian upgrade: could not start the startup watchdog: {error}");
    }
}

/// The server is listening: this start counts as "started ok". Records how the last upgrade
/// ended and tells the admins about a rollback.
pub async fn mark_ready(db: &Database) {
    READY.store(true, Ordering::Release);
    let startup = STARTUP.lock().unwrap_or_else(|e| e.into_inner()).take();
    let Some(startup) = startup else {
        return;
    };
    match startup {
        Startup::Trial(marker) => {
            if let Some(exe) = version::current_exe() {
                install::remove_marker(&InstallPaths::new(exe));
            }
            info!(from = %marker.from, to = %marker.to, by = %marker.by, "Updates: upgraded from {} to {}; this start is ready", marker.from, marker.to);
            record_history(
                db,
                UpgradeRecord {
                    from: marker.from.clone(),
                    to: marker.to.clone(),
                    at: marker.at.clone(),
                    by: marker.by.clone(),
                    outcome: "upgraded".to_string(),
                    detail: None,
                    backup_snapshot_id: marker.backup_snapshot_id.clone(),
                },
            )
            .await;
            if let Ok(running) = Version::parse(&marker.to) {
                notify::resolve_outdated(db, &running, None).await;
            }
            notify::tell_admins(
                db,
                "SUCCESS",
                &format!("Librarian was upgraded to {}", marker.to),
                &format!(
                    "{} upgraded this server from {} to {}. The previous binary is kept beside it as a .previous file.",
                    marker.by, marker.from, marker.to
                ),
            )
            .await;
        }
        Startup::RolledBack(marker) | Startup::Stale(marker) => {
            let reason = marker.reason.clone().unwrap_or_else(|| {
                format!(
                    "The upgrade to {} did not take; {} is still running.",
                    marker.to, marker.from
                )
            });
            warn!(from = %marker.from, to = %marker.to, "Updates: {reason}");
            record_history(
                db,
                UpgradeRecord {
                    from: marker.from.clone(),
                    to: marker.to.clone(),
                    at: marker.at.clone(),
                    by: marker.by.clone(),
                    outcome: if marker.state == MarkerState::RolledBack {
                        "rolled_back".to_string()
                    } else {
                        "failed".to_string()
                    },
                    detail: Some(reason.clone()),
                    backup_snapshot_id: marker.backup_snapshot_id.clone(),
                },
            )
            .await;
            notify::tell_admins(
                db,
                "ERROR",
                &if marker.state == MarkerState::RolledBack {
                    format!("The upgrade to Librarian {} was rolled back", marker.to)
                } else {
                    format!("The upgrade to Librarian {} did not complete", marker.to)
                },
                &format!(
                    "{reason} The failed binary is kept as a .failed file beside the running one; the logs from its start say why."
                ),
            )
            .await;
        }
        Startup::Normal | Startup::RollBack(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_commands_and_version_do_not_count_as_starts() {
        let args = |list: &[&str]| list.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        assert!(is_server_invocation(&args(&[])));
        assert!(is_server_invocation(&args(&["--server"])));
        assert!(!is_server_invocation(&args(&["--version"])));
        assert!(!is_server_invocation(&args(&["backup", "create"])));
        assert!(!is_server_invocation(&args(&["schema", "plan"])));
    }

    #[test]
    fn records_read_back_with_missing_fields() {
        let record: CheckRecord = serde_json::from_str(r#"{"lastCheckedAt":"x"}"#).unwrap();
        assert_eq!(record.last_checked_at.as_deref(), Some("x"));
        assert!(record.notified.is_empty());
        assert_eq!(record.catalogue, Catalogue::default());
    }
}
