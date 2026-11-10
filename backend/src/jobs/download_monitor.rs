//! `AutoDownloadService`: the automatic-downloading scheduler.
//!
//! Three workers run for the life of the server, and every one of them does nothing while the
//! `auto_download.enabled` setting is off (the default; there is no environment fallback, so
//! Settings › Downloads always shows what the server is doing):
//!
//! - the **scheduler** ticks every minute. It sweeps the whole library every
//!   `auto_download.interval_minutes`, and between sweeps runs the cheap **air-time pass** every
//!   five minutes, which looks only at episodes that have aired within the last
//!   `auto_download.air_window_days` days (and have passed their grace period) so a new episode is
//!   searched for within minutes of becoming searchable, not at the next hourly sweep;
//! - the **RSS worker** ticks every minute and polls each enabled feed that is due
//!   (`jobs::rss_poll`);
//! - the **retry sweep** re-checks torrents grabbed for a wanted target whose import never
//!   resolved, blocklists dead grabs and, when automatic downloading is on, searches again for the
//!   target (design.md Q34).
//!
//! `triggerAutoDownload` ("Run now") and `searchMissing` run a pass on demand whether or not the
//! switch is on: pressing a button is a person's decision, not automation.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::db::Database;
use crate::graphql::entities::{
    CreateReleaseBlocklistInput, ReleaseBlocklist, ReleaseBlocklistWhereInput, Torrent,
    TorrentWhereInput,
};
use crate::services::ServicesManager;
use crate::services::manager::{Service, ServiceHealth};
use crate::services::torrent::system_auth_user;
use graphql_orm::graphql::filters::{DateFilter, StringFilter};

use super::auto_download::{self, AutoDownloadRunSummary, HuntScope};
use super::auto_settings::{AutoDownloadSettings, SETTING_STATUS};
use super::auto_trail::Trigger;
use super::recommend;
use super::rss_poll;
use super::source_health;

/// The air-time pass runs this often between sweeps.
const AIR_TIME_PASS_MINUTES: i64 = 5;
const SETTING_RETRY_SWEEP_INTERVAL_MINUTES: &str = "auto_download.retry_sweep_interval_minutes";
const SETTING_RETRY_AFTER_MINUTES: &str = "auto_download.retry_after_minutes";
const SETTING_RETRY_WINDOW_DAYS: &str = "auto_download.retry_window_days";

/// A grabbed torrent that has downloaded nothing for this long is treated as
/// dead: it is blocklisted and its wanted target is re-hunted.
const STALLED_AFTER_HOURS: i64 = 24;

/// Config defaults for [`AutoDownloadService`]; all are also overridable at
/// runtime via `app_settings` (see the `SETTING_*` keys above), per
/// design.md's settings-loading rule.
#[derive(Debug, Clone)]
pub struct AutoDownloadServiceConfig {
    pub default_interval_minutes: u64,
    pub retry_sweep_interval_minutes: u64,
    pub default_retry_after_minutes: i64,
    pub default_retry_window_days: i64,
    /// How often the scheduler and the RSS worker look at the clock.
    pub tick_seconds: u64,
    /// Let the other services finish starting before the first pass.
    pub startup_delay_seconds: u64,
}

impl Default for AutoDownloadServiceConfig {
    fn default() -> Self {
        Self {
            default_interval_minutes: 60,
            retry_sweep_interval_minutes: 15,
            default_retry_after_minutes: 15,
            // Q34: retries are bounded to torrents added within the last 7 days.
            default_retry_window_days: 7,
            tick_seconds: 60,
            startup_delay_seconds: 45,
        }
    }
}

struct AutoDownloadRuntime {
    cancel_token: CancellationToken,
    main_handle: tokio::task::JoinHandle<()>,
    retry_handle: tokio::task::JoinHandle<()>,
    rss_handle: tokio::task::JoinHandle<()>,
    health_handle: tokio::task::JoinHandle<()>,
    recommend_handle: tokio::task::JoinHandle<()>,
    capacity_handle: tokio::task::JoinHandle<()>,
}

impl AutoDownloadRuntime {
    async fn shutdown(self) {
        self.cancel_token.cancel();
        let _ = self.main_handle.await;
        let _ = self.retry_handle.await;
        let _ = self.rss_handle.await;
        let _ = self.health_handle.await;
        let _ = self.recommend_handle.await;
        let _ = self.capacity_handle.await;
    }
}

/// Auto-download background loop: candidate discovery + grab on a timer, plus
/// a retry/reconciliation sweep. Depends on database, graphql, sources, torrent.
pub struct AutoDownloadService {
    manager: Arc<ServicesManager>,
    config: AutoDownloadServiceConfig,
    inner: tokio::sync::RwLock<Option<Arc<AutoDownloadRuntime>>>,
}

impl AutoDownloadService {
    pub fn new(manager: Arc<ServicesManager>, config: AutoDownloadServiceConfig) -> Self {
        Self {
            manager,
            config,
            inner: tokio::sync::RwLock::new(None),
        }
    }

    /// Run one pass immediately, regardless of the enabled gate: a person asked for it. Used by the
    /// `triggerAutoDownload` GraphQL mutation.
    pub async fn trigger_now(&self, library_id: Option<String>) -> Result<AutoDownloadRunSummary> {
        let summary = auto_download::run_once(&self.manager, library_id.as_deref()).await?;
        record_hunt_status(&self.manager, "manual", &summary).await;
        Ok(summary)
    }

    /// Run one hunt pass over a specific scope immediately, ignoring both the
    /// enabled gate and the per-target search backoff. Used by the
    /// `searchMissing` GraphQL mutation and by the retry sweep's re-hunt.
    pub async fn search_missing(&self, scope: HuntScope) -> Result<AutoDownloadRunSummary> {
        auto_download::run_scoped(&self.manager, &scope).await
    }
}

#[async_trait]
impl Service for AutoDownloadService {
    fn name(&self) -> &str {
        "auto_download"
    }

    fn dependencies(&self) -> Vec<String> {
        vec![
            "database".to_string(),
            "graphql".to_string(),
            "sources".to_string(),
            "torrent".to_string(),
        ]
    }

    async fn start(&self) -> Result<()> {
        if self.config.default_interval_minutes == 0
            || self.config.retry_sweep_interval_minutes == 0
            || self.config.default_retry_after_minutes <= 0
            || self.config.default_retry_window_days <= 0
            || self.config.tick_seconds == 0
        {
            anyhow::bail!("Auto-download intervals and retry windows must be greater than zero");
        }
        let previous = self.inner.write().await.take();
        if let Some(arc_r) = previous {
            arc_r.cancel_token.cancel();
            if let Ok(runtime) = Arc::try_unwrap(arc_r) {
                runtime.shutdown().await;
            }
        }

        let cancel_token = CancellationToken::new();

        let manager_main = self.manager.clone();
        let token_main = cancel_token.clone();
        let rhythm = Rhythm {
            tick: Duration::from_secs(self.config.tick_seconds),
            startup_delay: Duration::from_secs(self.config.startup_delay_seconds),
        };
        let main_handle = tokio::spawn(async move {
            scheduler_loop(manager_main, token_main, rhythm).await;
        });

        let manager_rss = self.manager.clone();
        let token_rss = cancel_token.clone();
        let rss_handle = tokio::spawn(async move {
            rss_loop(manager_rss, token_rss, rhythm).await;
        });

        let manager_health = self.manager.clone();
        let token_health = cancel_token.clone();
        let health_handle = tokio::spawn(async move {
            health_loop(manager_health, token_health, rhythm).await;
        });

        let manager_recommend = self.manager.clone();
        let token_recommend = cancel_token.clone();
        let recommend_handle = tokio::spawn(async move {
            recommend_loop(manager_recommend, token_recommend, rhythm).await;
        });

        // Free space on every volume (Q146): runs whether or not automatic downloading is on.
        let manager_capacity = self.manager.clone();
        let token_capacity = cancel_token.clone();
        let capacity_handle = tokio::spawn(async move {
            super::storage_capacity::run_loop(
                manager_capacity,
                token_capacity,
                rhythm.startup_delay,
            )
            .await;
        });

        let manager_retry = self.manager.clone();
        let retry_sweep_interval = self.config.retry_sweep_interval_minutes;
        let default_retry_after = self.config.default_retry_after_minutes;
        let default_retry_window = self.config.default_retry_window_days;
        let token_retry = cancel_token.clone();
        let retry_handle = tokio::spawn(async move {
            retry_sweep_loop(
                manager_retry,
                retry_sweep_interval,
                default_retry_after,
                default_retry_window,
                token_retry,
            )
            .await;
        });

        *self.inner.write().await = Some(Arc::new(AutoDownloadRuntime {
            cancel_token,
            main_handle,
            retry_handle,
            rss_handle,
            health_handle,
            recommend_handle,
            capacity_handle,
        }));

        info!(
            service = "auto_download",
            "Auto-download service started; it does nothing until the auto_download.enabled setting is on (Settings > Downloads)"
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        let previous = self.inner.write().await.take();
        if let Some(arc_r) = previous {
            arc_r.cancel_token.cancel();
            if let Ok(runtime) = Arc::try_unwrap(arc_r) {
                runtime.shutdown().await;
            }
        }
        info!(service = "auto_download", "Auto-download service stopped");
        Ok(())
    }

    async fn health(&self) -> Result<ServiceHealth> {
        let runtime = self.inner.read().await;
        let Some(runtime) = runtime.as_ref() else {
            return Ok(ServiceHealth::degraded("auto_download service not started"));
        };
        if runtime.main_handle.is_finished()
            || runtime.retry_handle.is_finished()
            || runtime.rss_handle.is_finished()
            || runtime.health_handle.is_finished()
            || runtime.recommend_handle.is_finished()
            || runtime.capacity_handle.is_finished()
        {
            return Ok(ServiceHealth::degraded(
                "one or more auto-download workers exited unexpectedly",
            ));
        }
        Ok(ServiceHealth::healthy())
    }
}

async fn load_settings(manager: &Arc<ServicesManager>) -> Option<AutoDownloadSettings> {
    let db_service = manager.get_database().await?;
    Some(AutoDownloadSettings::load(&db_service.pool().clone()).await)
}

async fn load_i64_setting(db: &Database, key: &str, default: i64) -> i64 {
    crate::services::torrent::get_setting::<i64>(db, key)
        .await
        .ok()
        .flatten()
        .unwrap_or(default)
}

async fn load_u64_setting(db: &Database, key: &str, default: u64) -> u64 {
    crate::services::torrent::get_setting::<u64>(db, key)
        .await
        .ok()
        .flatten()
        .unwrap_or(default)
}

/// What the scheduler should do on a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassKind {
    /// Search everything wanted.
    Sweep,
    /// Search only episodes that have just aired.
    AirTime,
}

/// Decide what to run on a tick. Pure, so the schedule can be tested with a fixed clock.
///
/// Nothing runs while automatic downloading is off. The first tick after it is turned on sweeps
/// immediately (the per-target backoff keeps that from searching what was just searched); after
/// that a sweep is due every `interval_minutes` and, in between, the air-time pass every five minutes.
pub fn next_pass(
    settings: &AutoDownloadSettings,
    now: DateTime<Utc>,
    last_sweep: Option<DateTime<Utc>>,
    last_air_time: Option<DateTime<Utc>>,
) -> Option<PassKind> {
    if !settings.enabled {
        return None;
    }
    let elapsed = |last: Option<DateTime<Utc>>, minutes: i64| {
        last.is_none_or(|last| now - last >= chrono::Duration::minutes(minutes))
    };
    if elapsed(last_sweep, settings.interval_minutes as i64) {
        return Some(PassKind::Sweep);
    }
    // With a zero-day window no episode is ever fresh, so the extra pass would find nothing.
    if settings.air_window_days > 0 && elapsed(last_air_time, AIR_TIME_PASS_MINUTES) {
        return Some(PassKind::AirTime);
    }
    None
}

/// Merge one pass's result into the stored status JSON (`{"sweep": {...}, "airTime": {...}, ...}`),
/// keeping the other kinds' last results.
pub fn merge_status(
    existing: Option<&str>,
    kind: &str,
    at: DateTime<Utc>,
    result: serde_json::Value,
) -> String {
    let mut status = existing
        .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    let mut entry = result;
    if let Some(object) = entry.as_object_mut() {
        object.insert("at".to_string(), serde_json::json!(at.to_rfc3339()));
    }
    status[kind] = entry;
    status.to_string()
}

async fn write_status(manager: &Arc<ServicesManager>, kind: &str, result: serde_json::Value) {
    let Some(db_service) = manager.get_database().await else {
        return;
    };
    let db = db_service.pool().clone();
    use crate::graphql::entities::{
        AppSetting, AppSettingWhereInput, CreateAppSettingInput, UpdateAppSettingInput,
    };
    let existing = AppSetting::query(db.pool())
        .filter(AppSettingWhereInput {
            key: Some(StringFilter {
                eq: Some(SETTING_STATUS.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await
        .ok()
        .and_then(|rows| rows.into_iter().next());
    let value = merge_status(
        existing.as_ref().map(|row| row.value.as_str()),
        kind,
        Utc::now(),
        result,
    );
    let outcome = match existing {
        Some(row) => AppSetting::update_by_id(
            &db,
            &row.id,
            UpdateAppSettingInput {
                value: Some(value),
                ..Default::default()
            },
        )
        .await
        .map(|_| ()),
        None => AppSetting::insert(
            &db,
            CreateAppSettingInput {
                key: SETTING_STATUS.to_string(),
                value,
                description: Some("Automatic downloading: what each pass did last".to_string()),
                category: "auto_download".to_string(),
            },
        )
        .await
        .map(|_| ()),
    };
    if let Err(error) = outcome {
        warn!(error = %error, "Auto-download: could not record the status of the last pass");
    }
}

async fn record_hunt_status(
    manager: &Arc<ServicesManager>,
    kind: &str,
    summary: &AutoDownloadRunSummary,
) {
    write_status(
        manager,
        kind,
        serde_json::json!({
            "candidates": summary.candidates_considered,
            "searched": summary.searched,
            "grabbed": summary.grabbed,
            "backoffSkipped": summary.backoff_skipped,
            "capped": summary.capped,
            "errors": summary.errors.iter().take(3).collect::<Vec<_>>(),
        }),
    )
    .await;
}

/// How often the workers look at the clock, and how long they let the rest of the server start.
#[derive(Debug, Clone, Copy)]
struct Rhythm {
    tick: Duration,
    startup_delay: Duration,
}

async fn scheduler_loop(manager: Arc<ServicesManager>, cancel: CancellationToken, rhythm: Rhythm) {
    tokio::select! {
        _ = cancel.cancelled() => return,
        _ = tokio::time::sleep(rhythm.startup_delay) => {}
    }
    let mut last_sweep: Option<DateTime<Utc>> = None;
    let mut last_air_time: Option<DateTime<Utc>> = None;
    loop {
        let Some(settings) = load_settings(&manager).await else {
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = tokio::time::sleep(rhythm.tick) => continue,
            }
        };
        if !settings.enabled {
            // Turning it on starts with a sweep.
            last_sweep = None;
            last_air_time = None;
        }
        let now = Utc::now();
        match next_pass(&settings, now, last_sweep, last_air_time) {
            Some(PassKind::Sweep) => {
                last_sweep = Some(now);
                last_air_time = Some(now);
                run_hunt(&manager, "sweep", Trigger::Sweep, false).await;
            }
            Some(PassKind::AirTime) => {
                last_air_time = Some(now);
                run_hunt(&manager, "airTime", Trigger::AirTime, true).await;
            }
            None => tracing::debug!("Auto-download: nothing due on this tick"),
        }
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(rhythm.tick) => {}
        }
    }
}

async fn run_hunt(manager: &Arc<ServicesManager>, kind: &str, trigger: Trigger, fresh_only: bool) {
    match auto_download::run_pass(manager, &HuntScope::default(), trigger, fresh_only).await {
        Ok(summary) => {
            info!(
                pass = kind,
                candidates = summary.candidates_considered,
                searched = summary.searched,
                grabbed = summary.grabbed,
                backoff_skipped = summary.backoff_skipped,
                capped = summary.capped,
                errors = summary.errors.len(),
                "Auto-download: {kind} pass finished: {} searched, {} grabbed, {} waiting on backoff, {} waiting on the limit",
                summary.searched,
                summary.grabbed,
                summary.backoff_skipped,
                summary.capped
            );
            record_hunt_status(manager, kind, &summary).await;
        }
        Err(error) => warn!(pass = kind, error = %error, "Auto-download: {kind} pass failed"),
    }
}

/// Tests every enabled source at least hourly (`source_health`). It runs whether or not automatic
/// downloading is on: a source that has stopped working should be noticed either way.
async fn health_loop(manager: Arc<ServicesManager>, cancel: CancellationToken, rhythm: Rhythm) {
    tokio::select! {
        _ = cancel.cancelled() => return,
        // Let the rest of the server start, and the first searches go first.
        _ = tokio::time::sleep(rhythm.startup_delay * 2) => {}
    }
    loop {
        match source_health::run_due_checks(&manager).await {
            Ok(summary) if summary.tested + summary.from_use > 0 => info!(
                tested = summary.tested,
                from_use = summary.from_use,
                failing = summary.failing,
                "Source health: checked {} source(s) ({} by their recent use), {} failing",
                summary.tested + summary.from_use,
                summary.from_use,
                summary.failing
            ),
            Ok(_) => {}
            Err(error) => warn!(error = %error, "Source health: the check pass failed"),
        }
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(rhythm.tick) => {}
        }
    }
}

/// Answers the lookups people queued and keeps what is flagged wanted looked up (`recommend`), all
/// inside the hourly search budget.
async fn recommend_loop(manager: Arc<ServicesManager>, cancel: CancellationToken, rhythm: Rhythm) {
    tokio::select! {
        _ = cancel.cancelled() => return,
        _ = tokio::time::sleep(rhythm.startup_delay * 3) => {}
    }
    loop {
        if let Err(error) = recommend::work(&manager).await {
            warn!(error = %crate::services::redact::redact_text(&format!("{error:#}")), "Recommendations: the worker failed");
        }
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(rhythm.tick) => {}
        }
    }
}

async fn rss_loop(manager: Arc<ServicesManager>, cancel: CancellationToken, rhythm: Rhythm) {
    tokio::select! {
        _ = cancel.cancelled() => return,
        _ = tokio::time::sleep(rhythm.startup_delay) => {}
    }
    loop {
        // Feeds are sources: their catalogue fills whether or not automatic downloading is on.
        // Only grabbing from them waits for the switch.
        if let Some(settings) = load_settings(&manager).await {
            match rss_poll::poll_due_feeds(&manager, settings.enabled).await {
                Ok(summary) if summary.feeds_polled > 0 => {
                    info!(
                        feeds = summary.feeds_polled,
                        failed = summary.feeds_failed,
                        new_items = summary.new_items,
                        matched = summary.matched,
                        grabbed = summary.grabbed,
                        capped = summary.capped,
                        "RSS: polled {} feed(s): {} new item(s), {} matched, {} grabbed",
                        summary.feeds_polled,
                        summary.new_items,
                        summary.matched,
                        summary.grabbed
                    );
                    write_status(
                        &manager,
                        "rss",
                        serde_json::json!({
                            "feeds": summary.feeds_polled,
                            "failed": summary.feeds_failed,
                            "newItems": summary.new_items,
                            "matched": summary.matched,
                            "grabbed": summary.grabbed,
                            "capped": summary.capped,
                        }),
                    )
                    .await;
                }
                Ok(_) => {}
                Err(error) => warn!(error = %error, "RSS: poll pass failed"),
            }
        }
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(rhythm.tick) => {}
        }
    }
}

/// Whether a torrent is eligible for the retry sweep: it was grabbed by
/// auto-download (has at least one wanted-linkage id set), its post-process
/// status never resolved to a terminal success/failure (`null`/`"unmatched"`),
/// it has aged past `retry_after_minutes` since being added, and it is still
/// within the `retry_window_days` retry window (design.md Q34).
///
/// Pulled out as a pure function (no DB access) so the cutoff/window logic is
/// unit-testable without a database.
fn needs_retry(
    torrent: &Torrent,
    now: DateTime<Utc>,
    retry_after_minutes: i64,
    retry_window_days: i64,
) -> bool {
    if !has_wanted_linkage(torrent) {
        return false;
    }

    let status_needs_retry = matches!(
        torrent.post_process_status.as_deref(),
        None | Some("unmatched")
    );
    if !status_needs_retry {
        return false;
    }

    let Ok(added_at) = DateTime::parse_from_rfc3339(&torrent.added_at) else {
        // Unparseable timestamp: don't retry rather than risk an infinite loop.
        return false;
    };
    let added_at = added_at.with_timezone(&Utc);

    let age = now.signed_duration_since(added_at);
    let old_enough = age >= chrono::Duration::minutes(retry_after_minutes);
    let within_window = age <= chrono::Duration::days(retry_window_days);
    old_enough && within_window
}

/// Whether a grabbed torrent has made no progress at all for longer than
/// `stalled_after_hours`. Such a torrent will never complete (dead swarm, no
/// peers, wrong tracker), so it is blocklisted rather than retried forever.
fn is_stalled(torrent: &Torrent, now: DateTime<Utc>, stalled_after_hours: i64) -> bool {
    if torrent.completed_at.is_some() || torrent.progress > 0.0 || torrent.downloaded_bytes > 0 {
        return false;
    }
    let Ok(added_at) = DateTime::parse_from_rfc3339(&torrent.added_at) else {
        return false;
    };
    now.signed_duration_since(added_at.with_timezone(&Utc))
        >= chrono::Duration::hours(stalled_after_hours)
}

/// Why a grabbed torrent should be added to the release blocklist, if it
/// should at all. Pure so the classification is unit-testable.
fn blocklist_reason(
    torrent: &Torrent,
    now: DateTime<Utc>,
    stalled_after_hours: i64,
) -> Option<&'static str> {
    let has_linkage = has_wanted_linkage(torrent);
    if !has_linkage {
        return None;
    }
    if torrent.post_process_status.as_deref() == Some("failed") {
        return Some("import_failed");
    }
    if is_stalled(torrent, now, stalled_after_hours) {
        return Some("stalled");
    }
    None
}

pub(super) fn has_wanted_linkage(torrent: &Torrent) -> bool {
    torrent.episode_id.is_some()
        || torrent.movie_id.is_some()
        || torrent.track_id.is_some()
        || torrent.chapter_id.is_some()
        || torrent.show_id.is_some()
        || torrent.album_id.is_some()
        || torrent.audiobook_id.is_some()
}

/// The hunt scope to re-search after blocklisting a dead grab. Episode/track
/// grabs re-hunt their parent show/album, which finds the same missing child
/// again with the blocked release excluded.
fn rehunt_scope(torrent: &Torrent) -> Option<HuntScope> {
    let scope = HuntScope {
        library_id: torrent.library_id.clone(),
        show_id: torrent.show_id.clone(),
        season: torrent.season,
        movie_id: torrent.movie_id.clone(),
        album_id: torrent.album_id.clone(),
        audiobook_id: torrent.audiobook_id.clone(),
        ignore_backoff: true,
        ..Default::default()
    };
    (scope.show_id.is_some()
        || scope.movie_id.is_some()
        || scope.album_id.is_some()
        || scope.audiobook_id.is_some())
    .then_some(scope)
}

/// Whether this release is already on the blocklist (by info hash).
async fn already_blocklisted(db: &Database, info_hash: &str) -> Result<bool> {
    Ok(!ReleaseBlocklist::query(db.pool())
        .filter(ReleaseBlocklistWhereInput {
            info_hash: Some(StringFilter {
                eq: Some(info_hash.to_string()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?
        .is_empty())
}

/// Record a dead grab on the release blocklist so the hunt never picks it
/// again, then re-hunt the wanted target it was supposed to satisfy.
async fn blocklist_and_rehunt(
    manager: &Arc<ServicesManager>,
    db: &Database,
    torrent: &Torrent,
    reason: &str,
) -> Result<()> {
    if already_blocklisted(db, &torrent.info_hash).await? {
        return Ok(());
    }

    ReleaseBlocklist::insert(
        db,
        CreateReleaseBlocklistInput {
            info_hash: Some(torrent.info_hash.clone()),
            guid: torrent.source_url.clone(),
            title: torrent.name.clone(),
            source_id: torrent.source_indexer_id.clone(),
            reason: reason.to_string(),
            show_id: torrent.show_id.clone(),
            movie_id: torrent.movie_id.clone(),
            album_id: torrent.album_id.clone(),
            audiobook_id: torrent.audiobook_id.clone(),
            expires_at: None,
        },
    )
    .await?;

    info!(
        info_hash = %torrent.info_hash,
        torrent_name = %torrent.name,
        reason,
        "Auto-download: blocklisted a dead release"
    );

    let searching_again = AutoDownloadSettings::load(db).await.enabled;
    if let Some(library_id) = torrent.library_id.as_deref() {
        let torrent_id = Some(torrent.id.as_str());
        super::auto_trail::Trail::new(db)
            .blocklisted(
                library_id,
                &torrent.name,
                reason,
                searching_again,
                torrent_id,
            )
            .await;
    }
    if !searching_again {
        info!(
            info_hash = %torrent.info_hash,
            torrent_name = %torrent.name,
            "Auto-download: automatic downloading is off, so the target of the blocklisted release is not searched again"
        );
        return Ok(());
    }

    let Some(scope) = rehunt_scope(torrent) else {
        warn!(
            info_hash = %torrent.info_hash,
            torrent_name = %torrent.name,
            "Auto-download: blocklisted release has no re-huntable parent; skipping re-hunt"
        );
        return Ok(());
    };

    match auto_download::run_pass(manager, &scope, Trigger::Retry, false).await {
        Ok(summary) => info!(
            torrent_name = %torrent.name,
            searched = summary.searched,
            grabbed = summary.grabbed,
            "Auto-download: re-hunted the target of a blocklisted release"
        ),
        Err(error) => warn!(
            error = %error,
            torrent_name = %torrent.name,
            "Auto-download: re-hunt after blocklisting failed"
        ),
    }
    Ok(())
}

/// Every torrent added inside the retry window, regardless of state.
async fn find_window_torrents(db: &Database, retry_window_days: i64) -> Result<Vec<Torrent>> {
    let window_start = Utc::now() - chrono::Duration::days(retry_window_days);
    Ok(Torrent::query(db.pool())
        .filter(TorrentWhereInput {
            added_at: Some(DateFilter {
                gte: Some(window_start.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?)
}

pub async fn run_retry_sweep(
    manager: &Arc<ServicesManager>,
    retry_after_minutes: i64,
    retry_window_days: i64,
) -> Result<()> {
    let db_service = manager
        .get_database()
        .await
        .ok_or_else(|| anyhow::anyhow!("database service not available"))?;
    let db = db_service.pool().clone();

    let torrent_service = manager
        .get_torrent()
        .await
        .ok_or_else(|| anyhow::anyhow!("torrent service not available"))?;

    let auth_user = system_auth_user();

    let now = Utc::now();
    let window = find_window_torrents(&db, retry_window_days).await?;

    // Dead grabs (import failed, or zero progress for over a day) are
    // blocklisted and their target re-hunted; everything else that never
    // resolved is simply re-processed.
    let (dead, rest): (Vec<Torrent>, Vec<Torrent>) = window
        .into_iter()
        .partition(|torrent| blocklist_reason(torrent, now, STALLED_AFTER_HOURS).is_some());

    for torrent in &dead {
        let Some(reason) = blocklist_reason(torrent, now, STALLED_AFTER_HOURS) else {
            continue;
        };
        if let Err(error) = blocklist_and_rehunt(manager, &db, torrent, reason).await {
            warn!(
                error = %error,
                info_hash = %torrent.info_hash,
                torrent_name = %torrent.name,
                "Auto-download: failed to blocklist and re-hunt a dead release"
            );
        }
    }

    let candidates: Vec<Torrent> = rest
        .into_iter()
        .filter(|torrent| needs_retry(torrent, now, retry_after_minutes, retry_window_days))
        .collect();
    if candidates.is_empty() {
        return Ok(());
    }

    info!(
        count = candidates.len(),
        "Auto-download: retry sweep found unresolved grabbed torrents"
    );

    for torrent in candidates {
        match torrent_service
            .process_completed_torrent(&auth_user, &torrent.info_hash, None)
            .await
        {
            Ok(summary) => info!(
                info_hash = %torrent.info_hash,
                torrent_name = %torrent.name,
                processed = summary.files_processed,
                "Auto-download: retry sweep re-processed torrent"
            ),
            Err(e) => warn!(
                error = %e,
                info_hash = %torrent.info_hash,
                torrent_name = %torrent.name,
                "Auto-download: retry sweep failed to re-process torrent"
            ),
        }
    }

    Ok(())
}

async fn retry_sweep_loop(
    manager: Arc<ServicesManager>,
    default_sweep_interval_minutes: u64,
    default_retry_after_minutes: i64,
    default_retry_window_days: i64,
    cancel: CancellationToken,
) {
    loop {
        let (sweep_interval_minutes, retry_after_minutes, retry_window_days) =
            match manager.get_database().await {
                Some(db_service) => {
                    let db = db_service.pool().clone();
                    (
                        load_u64_setting(
                            &db,
                            SETTING_RETRY_SWEEP_INTERVAL_MINUTES,
                            default_sweep_interval_minutes,
                        )
                        .await,
                        load_i64_setting(
                            &db,
                            SETTING_RETRY_AFTER_MINUTES,
                            default_retry_after_minutes,
                        )
                        .await,
                        load_i64_setting(&db, SETTING_RETRY_WINDOW_DAYS, default_retry_window_days)
                            .await,
                    )
                }
                None => (
                    default_sweep_interval_minutes,
                    default_retry_after_minutes,
                    default_retry_window_days,
                ),
            };

        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(Duration::from_secs(sweep_interval_minutes.max(1) * 60)) => {
                if let Err(e) = run_retry_sweep(&manager, retry_after_minutes, retry_window_days).await {
                    warn!(error = %e, "Auto-download: retry sweep failed");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_torrent() -> Torrent {
        Torrent {
            id: "t1".to_string(),
            user_id: "u1".to_string(),
            info_hash: "hash".to_string(),
            magnet_uri: None,
            name: "Some.Release".to_string(),
            state: "downloading".to_string(),
            progress: 1.0,
            total_bytes: 100,
            downloaded_bytes: 100,
            uploaded_bytes: 0,
            uploaded_bytes_total: 0,
            delete_files_on_remove: false,
            save_path: "/tmp".to_string(),
            download_path: None,
            source_url: None,
            source_feed_id: None,
            source_indexer_id: None,
            library_id: None,
            identification_kind: None,
            identification_status: None,
            identification_summary: None,
            post_process_status: None,
            post_process_error: None,
            processed_at: None,
            episode_id: Some("ep1".to_string()),
            movie_id: None,
            track_id: None,
            chapter_id: None,
            show_id: Some("show1".to_string()),
            album_id: None,
            audiobook_id: None,
            minimum_ratio: None,
            minimum_seed_time_minutes: None,
            season: None,
            excluded_files: vec![],
            added_at: Utc::now().to_rfc3339(),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            completed_at: None,
            files: vec![],
            library: None,
            movie: None,
            show: None,
            episode: None,
            track: None,
            chapter: None,
            album: None,
            audiobook: None,
            source_feed: None,
            pending_matches: vec![],
        }
    }

    #[test]
    fn needs_retry_is_false_without_wanted_linkage() {
        let mut torrent = base_torrent();
        torrent.episode_id = None;
        torrent.show_id = None;
        torrent.added_at = (Utc::now() - chrono::Duration::minutes(30)).to_rfc3339();
        assert!(!needs_retry(&torrent, Utc::now(), 15, 7));
    }

    #[test]
    fn needs_retry_is_false_when_already_completed() {
        let mut torrent = base_torrent();
        torrent.post_process_status = Some("completed".to_string());
        torrent.added_at = (Utc::now() - chrono::Duration::minutes(30)).to_rfc3339();
        assert!(!needs_retry(&torrent, Utc::now(), 15, 7));
    }

    #[test]
    fn needs_retry_is_false_before_retry_after_minutes_elapsed() {
        let mut torrent = base_torrent();
        torrent.added_at = (Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();
        assert!(!needs_retry(&torrent, Utc::now(), 15, 7));
    }

    #[test]
    fn needs_retry_is_true_within_window_past_delay() {
        let mut torrent = base_torrent();
        torrent.added_at = (Utc::now() - chrono::Duration::minutes(30)).to_rfc3339();
        assert!(needs_retry(&torrent, Utc::now(), 15, 7));
    }

    #[test]
    fn needs_retry_is_true_for_unmatched_status() {
        let mut torrent = base_torrent();
        torrent.post_process_status = Some("unmatched".to_string());
        torrent.added_at = (Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        assert!(needs_retry(&torrent, Utc::now(), 15, 7));
    }

    #[test]
    fn needs_retry_is_false_past_the_7_day_retry_window() {
        let mut torrent = base_torrent();
        // Q34: retries are limited to torrents added within the last 7 days.
        torrent.added_at = (Utc::now() - chrono::Duration::days(8)).to_rfc3339();
        assert!(!needs_retry(&torrent, Utc::now(), 15, 7));
    }

    #[test]
    fn blocklist_reason_flags_failed_imports() {
        let mut torrent = base_torrent();
        torrent.post_process_status = Some("failed".to_string());
        assert_eq!(
            blocklist_reason(&torrent, Utc::now(), STALLED_AFTER_HOURS),
            Some("import_failed")
        );
    }

    #[test]
    fn blocklist_reason_flags_stalled_downloads() {
        let mut torrent = base_torrent();
        torrent.progress = 0.0;
        torrent.downloaded_bytes = 0;
        torrent.added_at = (Utc::now() - chrono::Duration::hours(30)).to_rfc3339();
        assert_eq!(
            blocklist_reason(&torrent, Utc::now(), STALLED_AFTER_HOURS),
            Some("stalled")
        );

        // Still inside the stall window.
        torrent.added_at = (Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        assert_eq!(
            blocklist_reason(&torrent, Utc::now(), STALLED_AFTER_HOURS),
            None
        );
    }

    #[test]
    fn blocklist_reason_ignores_progressing_and_unlinked_torrents() {
        let mut progressing = base_torrent();
        progressing.progress = 0.0;
        progressing.downloaded_bytes = 4096;
        progressing.added_at = (Utc::now() - chrono::Duration::hours(48)).to_rfc3339();
        assert_eq!(
            blocklist_reason(&progressing, Utc::now(), STALLED_AFTER_HOURS),
            None
        );

        let mut manual = base_torrent();
        manual.episode_id = None;
        manual.show_id = None;
        manual.post_process_status = Some("failed".to_string());
        assert_eq!(
            blocklist_reason(&manual, Utc::now(), STALLED_AFTER_HOURS),
            None
        );
    }

    #[test]
    fn rehunt_scope_targets_the_parent_and_ignores_backoff() {
        let torrent = base_torrent();
        let scope = rehunt_scope(&torrent).expect("an episode grab re-hunts its show");
        assert_eq!(scope.show_id.as_deref(), Some("show1"));
        assert!(scope.ignore_backoff);

        let mut unlinked = base_torrent();
        unlinked.episode_id = None;
        unlinked.show_id = None;
        assert!(rehunt_scope(&unlinked).is_none());
    }

    #[test]
    fn only_unfinished_unfailed_linked_downloads_count_against_the_cap() {
        let downloading = base_torrent();
        assert!(auto_download::counts_as_in_flight(&downloading));

        let mut finished = base_torrent();
        finished.completed_at = Some(Utc::now().to_rfc3339());
        assert!(
            !auto_download::counts_as_in_flight(&finished),
            "seeding is not downloading"
        );

        let mut failed = base_torrent();
        failed.post_process_status = Some("failed".to_string());
        assert!(!auto_download::counts_as_in_flight(&failed));

        let mut manual = base_torrent();
        manual.episode_id = None;
        manual.show_id = None;
        assert!(
            !auto_download::counts_as_in_flight(&manual),
            "a download nobody wanted for a title"
        );
    }

    fn settings(enabled: bool) -> AutoDownloadSettings {
        AutoDownloadSettings {
            enabled,
            ..AutoDownloadSettings::default()
        }
    }

    fn at(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn nothing_is_scheduled_while_the_setting_is_off() {
        let now = at("2026-11-08T12:00:00Z");
        assert_eq!(next_pass(&settings(false), now, None, None), None);
        assert_eq!(
            next_pass(
                &settings(false),
                now,
                Some(now - chrono::Duration::days(3)),
                None
            ),
            None
        );
    }

    #[test]
    fn the_first_tick_after_turning_it_on_sweeps() {
        let now = at("2026-11-08T12:00:00Z");
        assert_eq!(
            next_pass(&settings(true), now, None, None),
            Some(PassKind::Sweep)
        );
    }

    #[test]
    fn a_sweep_is_due_every_interval_and_the_air_time_pass_fills_the_gaps() {
        let now = at("2026-11-08T12:00:00Z");
        let settings = settings(true); // sweep every 60 minutes, air-time pass every 5
        let swept = Some(now - chrono::Duration::minutes(10));

        // 10 minutes after a sweep, 10 after the last air-time pass: the air-time pass is due.
        assert_eq!(
            next_pass(
                &settings,
                now,
                swept,
                Some(now - chrono::Duration::minutes(10))
            ),
            Some(PassKind::AirTime)
        );
        // Air-time pass ran 2 minutes ago: nothing is due.
        assert_eq!(
            next_pass(
                &settings,
                now,
                swept,
                Some(now - chrono::Duration::minutes(2))
            ),
            None
        );
        // An hour after the sweep: sweep again, whatever the air-time pass did.
        assert_eq!(
            next_pass(
                &settings,
                now,
                Some(now - chrono::Duration::minutes(60)),
                Some(now - chrono::Duration::minutes(1))
            ),
            Some(PassKind::Sweep)
        );
    }

    #[test]
    fn with_no_post_air_window_there_is_no_air_time_pass() {
        let now = at("2026-11-08T12:00:00Z");
        let settings = AutoDownloadSettings {
            air_window_days: 0,
            ..settings(true)
        };
        assert_eq!(
            next_pass(
                &settings,
                now,
                Some(now - chrono::Duration::minutes(10)),
                None
            ),
            None
        );
    }

    #[test]
    fn status_keeps_each_kind_of_pass_and_survives_damage() {
        let at = at("2026-11-08T12:00:00Z");
        let first = merge_status(None, "sweep", at, serde_json::json!({"searched": 3}));
        let second = merge_status(Some(&first), "rss", at, serde_json::json!({"feeds": 2}));
        let status: serde_json::Value = serde_json::from_str(&second).unwrap();
        assert_eq!(status["sweep"]["searched"], 3);
        assert_eq!(status["rss"]["feeds"], 2);
        assert_eq!(status["rss"]["at"], at.to_rfc3339());

        let repaired = merge_status(
            Some("{not json"),
            "sweep",
            at,
            serde_json::json!({"searched": 1}),
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&repaired).unwrap()["sweep"]["searched"],
            1
        );
    }

    #[test]
    fn needs_retry_is_false_for_failed_status() {
        let mut torrent = base_torrent();
        torrent.post_process_status = Some("failed".to_string());
        torrent.added_at = (Utc::now() - chrono::Duration::minutes(30)).to_rfc3339();
        assert!(!needs_retry(&torrent, Utc::now(), 15, 7));
    }
}
