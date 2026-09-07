//! `AutoDownloadService`: the auto-download background loop.
//!
//! Runs `jobs::auto_download::run_once` on a timer (only when explicitly
//! enabled — see the safety note below) plus a retry/reconciliation sweep
//! that re-checks torrents grabbed by auto-download whose post-process status
//! is still null/unmatched, bounded to a 7-day retry window (design.md Q34).
//!
//! # Safety default: disabled unless explicitly enabled
//!
//! The automatic timer loop defaults to **disabled**. The import path this
//! feeds (torrent completion -> `matchMediaFile` -> auto-link) has a history
//! of confidence-floor bugs (tier1-features-plan.md §5 top risk #1); running
//! unattended search+grab automation before that's solid could mass-produce
//! wrong-show links. Enable via the `auto_download.enabled` app_setting or the
//! `LIBRARIAN_AUTO_DOWNLOAD_ENABLED` env var. The `triggerAutoDownload`
//! GraphQL mutation always runs a pass on demand regardless of this gate, so
//! admins can safely exercise the pipeline before flipping it on.

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
use crate::services::torrent::get_default_user_id;
use graphql_orm::graphql::filters::{DateFilter, StringFilter};

use super::auto_download::{self, AutoDownloadRunSummary, HuntScope};

const ENV_ENABLED: &str = "LIBRARIAN_AUTO_DOWNLOAD_ENABLED";
const SETTING_ENABLED: &str = "auto_download.enabled";
const SETTING_INTERVAL_MINUTES: &str = "auto_download.interval_minutes";
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
}

impl Default for AutoDownloadServiceConfig {
    fn default() -> Self {
        Self {
            default_interval_minutes: 60,
            retry_sweep_interval_minutes: 15,
            default_retry_after_minutes: 15,
            // Q34: retries are bounded to torrents added within the last 7 days.
            default_retry_window_days: 7,
        }
    }
}

struct AutoDownloadRuntime {
    cancel_token: CancellationToken,
    main_handle: tokio::task::JoinHandle<()>,
    retry_handle: tokio::task::JoinHandle<()>,
}

impl AutoDownloadRuntime {
    async fn shutdown(self) {
        self.cancel_token.cancel();
        let _ = self.main_handle.await;
        let _ = self.retry_handle.await;
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

    /// Run one pass immediately, regardless of the enabled gate. Used by the
    /// `triggerAutoDownload` GraphQL mutation.
    pub async fn trigger_now(&self, library_id: Option<String>) -> Result<AutoDownloadRunSummary> {
        auto_download::run_once(&self.manager, library_id.as_deref()).await
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
        let default_interval = self.config.default_interval_minutes;
        let token_main = cancel_token.clone();
        let main_handle = tokio::spawn(async move {
            main_loop(manager_main, default_interval, token_main).await;
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
        }));

        info!(
            service = "auto_download",
            "Auto-download service started (disabled by default; enable via auto_download.enabled setting or LIBRARIAN_AUTO_DOWNLOAD_ENABLED)"
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
        if runtime.main_handle.is_finished() || runtime.retry_handle.is_finished() {
            return Ok(ServiceHealth::degraded(
                "one or more auto-download workers exited unexpectedly",
            ));
        }
        Ok(ServiceHealth::healthy())
    }
}

async fn is_enabled(manager: &Arc<ServicesManager>) -> bool {
    let Some(db_service) = manager.get_database().await else {
        return false;
    };
    let db = db_service.pool().clone();
    if let Ok(Some(v)) = crate::services::torrent::get_setting::<bool>(&db, SETTING_ENABLED).await {
        return v;
    }
    std::env::var(ENV_ENABLED)
        .ok()
        .map(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

async fn load_u64_setting(db: &Database, key: &str, default: u64) -> u64 {
    crate::services::torrent::get_setting::<u64>(db, key)
        .await
        .ok()
        .flatten()
        .unwrap_or(default)
}

async fn load_i64_setting(db: &Database, key: &str, default: i64) -> i64 {
    crate::services::torrent::get_setting::<i64>(db, key)
        .await
        .ok()
        .flatten()
        .unwrap_or(default)
}

async fn main_loop(
    manager: Arc<ServicesManager>,
    default_interval_minutes: u64,
    cancel: CancellationToken,
) {
    loop {
        let interval_minutes = match manager.get_database().await {
            Some(db_service) => {
                load_u64_setting(
                    &db_service.pool().clone(),
                    SETTING_INTERVAL_MINUTES,
                    default_interval_minutes,
                )
                .await
            }
            None => default_interval_minutes,
        }
        .max(1);

        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(Duration::from_secs(interval_minutes * 60)) => {
                if !is_enabled(&manager).await {
                    tracing::debug!("Auto-download: disabled, skipping scheduled run");
                    continue;
                }
                match auto_download::run_once(&manager, None).await {
                    Ok(summary) => info!(
                        candidates = summary.candidates_considered,
                        searched = summary.searched,
                        grabbed = summary.grabbed,
                        errors = summary.errors.len(),
                        "Auto-download: scheduled run finished"
                    ),
                    Err(e) => warn!(error = %e, "Auto-download: scheduled run failed"),
                }
            }
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

fn has_wanted_linkage(torrent: &Torrent) -> bool {
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

    let Some(scope) = rehunt_scope(torrent) else {
        warn!(
            info_hash = %torrent.info_hash,
            torrent_name = %torrent.name,
            "Auto-download: blocklisted release has no re-huntable parent; skipping re-hunt"
        );
        return Ok(());
    };

    match auto_download::run_scoped(manager, &scope).await {
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

    let Some(user_id) = get_default_user_id(&db).await? else {
        return Ok(());
    };
    let auth_user = crate::services::graphql::AuthUser {
        user_id: user_id.to_string(),
        email: None,
        role: Some("admin".to_string()),
    };

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
            save_path: "/tmp".to_string(),
            download_path: None,
            source_url: None,
            source_feed_id: None,
            source_indexer_id: None,
            library_id: None,
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
    fn needs_retry_is_false_for_failed_status() {
        let mut torrent = base_torrent();
        torrent.post_process_status = Some("failed".to_string());
        torrent.added_at = (Utc::now() - chrono::Duration::minutes(30)).to_rfc3339();
        assert!(!needs_retry(&torrent, Utc::now(), 15, 7));
    }
}
