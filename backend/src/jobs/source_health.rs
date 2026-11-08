//! Source health: testing every enabled source at least hourly, recording how it went, and
//! telling people when one starts failing.
//!
//! - **The check.** [`run_due_checks`] tests each enabled source that is due, one at a time with a
//!   random pause between (never a burst). A real search that succeeded since the last check counts
//!   as the check and the source is not tested again; a real search that failed counts as a failed
//!   check without another request. A feed's poll is its check (`rss_poll` records it the same way).
//! - **The record.** `Source.healthStatus` (`ok`, `failing`, `auth_expired`), `healthKind`,
//!   `lastCheckedAt`, `nextCheckAt` and the existing `lastError`, `errorCount`, `lastSuccessAt` and
//!   `lastErrorAt`. A source that keeps failing is tested less and less often (hourly, then 2h, 4h …
//!   up to a day, `sources::health::check_interval`); the "Test now" action ignores that.
//! - **The alert.** One notification (category `source_health`) per failing source or feed, for the
//!   admins and, for a feed, its owner. A sign-in problem alerts at once, since it will not mend
//!   itself; other failures after two checks in a row, so a hiccup stays quiet. The notification is
//!   rewritten if the reason changes and resolved when the source recovers. Nothing is created twice.
//!
//! Messages never contain credentials, cookies or feed addresses (`services::redact`).

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use graphql_orm::graphql::filters::{DateFilter, StringFilter};
use rand::RngExt;
use tracing::{debug, info, warn};

use crate::db::Database;
use crate::graphql::entities::{
    CreateNotificationInput, Notification, NotificationWhereInput, RssFeed, Source,
    UpdateNotificationInput, UpdateSourceInput, User, UserWhereInput,
};
use crate::services::ServicesManager;
use crate::services::redact::redact_text;
use crate::services::sources::SessionExpired;
use crate::services::sources::health::{
    FailureKind, STATUS_OK, check_due, check_interval, classify_error_text,
};
use crate::services::sources::manager::{SourceUse, SourcesManager};

pub const CATEGORY: &str = "source_health";
/// `Notification.actionType` of these notifications; `actionData` holds the source or feed id.
pub const ACTION_TYPE: &str = "source_health";

/// Most checks one pass makes, so a long list is spread over passes.
const MAX_CHECKS_PER_PASS: usize = 20;
/// The pause between two sources' checks is random, within these bounds (seconds).
const JITTER_SECONDS: (u64, u64) = (5, 45);
/// A test that has not answered by now has failed.
const TEST_TIMEOUT: Duration = Duration::from_secs(60);
/// Checks in a row that must fail before a failure other than a sign-in problem alerts.
const ALERT_AFTER_FAILURES: i32 = 2;

fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

// ===========================================================================
// What a check found
// ===========================================================================

/// How one check went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckResult {
    Ok,
    Failed { kind: FailureKind, message: String },
}

impl CheckResult {
    pub fn failed(kind: FailureKind, message: impl AsRef<str>) -> Self {
        Self::Failed {
            kind,
            message: redact_text(message.as_ref()),
        }
    }

    /// From an error: a lost session is an authentication failure whatever its words.
    pub fn from_error(error: &anyhow::Error) -> Self {
        let text = format!("{error:#}");
        let kind = if error.downcast_ref::<SessionExpired>().is_some() {
            FailureKind::Auth
        } else {
            classify_error_text(&text)
        };
        Self::failed(kind, text)
    }
}

/// The check a real search stands for, if one happened since `checked_at` and within the hour.
pub fn check_from_use(
    usage: Option<&SourceUse>,
    checked_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Option<CheckResult> {
    let usage = usage?;
    if now - usage.at >= chrono::Duration::hours(1) || checked_at.is_some_and(|at| usage.at <= at) {
        return None;
    }
    Some(match (&usage.ok, &usage.error) {
        (true, _) => CheckResult::Ok,
        (false, error) => {
            let text = error
                .clone()
                .unwrap_or_else(|| "the last search failed".to_string());
            CheckResult::failed(classify_error_text(&text), text)
        }
    })
}

// ===========================================================================
// Alerts
// ===========================================================================

/// The notification for a failing source: its title and message. The title names what is wrong,
/// the message what to do.
pub fn alert_text(name: &str, kind: FailureKind, detail: &str) -> (String, String) {
    let detail = detail.trim();
    match kind {
        FailureKind::Auth => (
            format!("{name} can't sign in"),
            if detail.is_empty() {
                format!(
                    "{name} can't sign in: its login has expired or was refused. Update its credentials in Settings › Sources."
                )
            } else {
                format!(
                    "{name} can't sign in: {detail} Update its credentials in Settings › Sources."
                )
            },
        ),
        FailureKind::Unreachable => (
            format!("{name} is unreachable"),
            format!(
                "{name} cannot be reached right now{}. Librarian keeps checking, less often the longer it fails.",
                if detail.is_empty() {
                    String::new()
                } else {
                    format!(" ({detail})")
                }
            ),
        ),
        FailureKind::RateLimited => (
            format!("{name} is rate limiting requests"),
            format!(
                "{name} is asking Librarian to slow down{}. Searches and checks back off by themselves.",
                if detail.is_empty() {
                    String::new()
                } else {
                    format!(" ({detail})")
                }
            ),
        ),
        FailureKind::Other => (
            format!("{name} is failing"),
            format!(
                "{name} is failing{}. Open Settings › Sources to test it.",
                if detail.is_empty() {
                    String::new()
                } else {
                    format!(": {detail}")
                }
            ),
        ),
    }
}

/// Whether a failure has earned an alert after `consecutive_failures` checks in a row.
pub fn should_alert(kind: FailureKind, consecutive_failures: i32) -> bool {
    kind == FailureKind::Auth || consecutive_failures >= ALERT_AFTER_FAILURES
}

fn level(kind: FailureKind) -> &'static str {
    match kind {
        FailureKind::Auth => "ERROR",
        _ => "WARNING",
    }
}

/// Raises, rewrites and resolves the notifications about failing sources and feeds.
pub struct Alerts {
    db: Database,
}

impl Alerts {
    pub fn new(db: &Database) -> Self {
        Self { db: db.clone() }
    }

    /// Everyone who should hear about a source or feed: the admins and, for a feed, its owner.
    async fn recipients(&self, owner: Option<&str>) -> Vec<String> {
        let mut ids: Vec<String> = match User::query(self.db.pool())
            .filter(UserWhereInput {
                role: Some(string_eq("admin")),
                ..Default::default()
            })
            .fetch_all()
            .await
        {
            Ok(admins) => admins.into_iter().map(|admin| admin.id).collect(),
            Err(error) => {
                warn!(error = %error, "Source health: could not list the admins to tell");
                Vec::new()
            }
        };
        if let Some(owner) = owner
            && !ids.iter().any(|id| id == owner)
        {
            ids.push(owner.to_string());
        }
        ids
    }

    async fn open_for(&self, user_id: &str, subject_id: &str) -> Option<Notification> {
        let rows = Notification::query(self.db.pool())
            .filter(NotificationWhereInput {
                user_id: Some(string_eq(user_id)),
                category: Some(string_eq(CATEGORY)),
                resolved_at: Some(DateFilter {
                    is_null: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
            .map_err(
                |error| warn!(error = %error, "Source health: could not look for an open alert"),
            )
            .ok()?;
        rows.into_iter()
            .find(|row| row.action_data.as_deref() == Some(subject_id))
    }

    /// The source or feed is failing: open its alert, or rewrite the open one if the reason changed.
    pub async fn failing(
        &self,
        subject_id: &str,
        name: &str,
        owner: Option<&str>,
        kind: FailureKind,
        detail: &str,
    ) {
        let (title, message) = alert_text(name, kind, detail);
        for user_id in self.recipients(owner).await {
            match self.open_for(&user_id, subject_id).await {
                Some(open) if open.title == title && open.message == message => {}
                Some(open) => {
                    if let Err(error) = Notification::update_by_id(
                        &self.db,
                        &open.id,
                        UpdateNotificationInput {
                            notification_type: Some(level(kind).to_string()),
                            title: Some(title.clone()),
                            message: Some(message.clone()),
                            ..Default::default()
                        },
                    )
                    .await
                    {
                        warn!(source = %name, error = %error, "Source health: could not update an alert");
                    }
                }
                None => {
                    if let Err(error) = Notification::insert(
                        &self.db,
                        CreateNotificationInput {
                            user_id: user_id.clone(),
                            notification_type: level(kind).to_string(),
                            category: CATEGORY.to_string(),
                            title: title.clone(),
                            message: message.clone(),
                            library_id: None,
                            torrent_id: None,
                            media_file_id: None,
                            pending_match_id: None,
                            action_type: Some(ACTION_TYPE.to_string()),
                            action_data: Some(subject_id.to_string()),
                            read_at: None,
                            resolved_at: None,
                            resolution: None,
                        },
                    )
                    .await
                    {
                        warn!(source = %name, error = %error, "Source health: could not raise an alert");
                    }
                }
            }
        }
    }

    /// It works again: close its alert for everyone.
    pub async fn recovered(&self, subject_id: &str, owner: Option<&str>) {
        for user_id in self.recipients(owner).await {
            let Some(open) = self.open_for(&user_id, subject_id).await else {
                continue;
            };
            if let Err(error) = Notification::update_by_id(
                &self.db,
                &open.id,
                UpdateNotificationInput {
                    resolved_at: Some(Some(Utc::now().to_rfc3339())),
                    resolution: Some(Some("recovered".to_string())),
                    ..Default::default()
                },
            )
            .await
            {
                warn!(error = %error, "Source health: could not resolve an alert");
            }
        }
    }
}

/// Record the outcome of a feed poll for its alert. Called by `rss_poll` after it has written the
/// feed's own health fields.
pub async fn feed_outcome(
    db: &Database,
    feed: &RssFeed,
    failure: Option<(FailureKind, &str, i32)>,
) {
    let alerts = Alerts::new(db);
    match failure {
        None => alerts.recovered(&feed.id, Some(&feed.user_id)).await,
        Some((kind, detail, consecutive)) if should_alert(kind, consecutive) => {
            alerts
                .failing(&feed.id, &feed.name, Some(&feed.user_id), kind, detail)
                .await;
        }
        Some(_) => {}
    }
}

// ===========================================================================
// Recording a source's check
// ===========================================================================

/// Write the outcome of a check to the source's health fields and raise or close its alert.
pub async fn record_source_check(
    db: &Database,
    source: &Source,
    result: &CheckResult,
    now: DateTime<Utc>,
) {
    let stamp = now.to_rfc3339();
    let update = match result {
        CheckResult::Ok => UpdateSourceInput {
            error_count: Some(0),
            last_error: Some(None),
            last_success_at: Some(Some(stamp.clone())),
            health_status: Some(Some(STATUS_OK.to_string())),
            health_kind: Some(None),
            last_checked_at: Some(Some(stamp.clone())),
            next_check_at: Some(Some((now + check_interval(0)).to_rfc3339())),
            ..Default::default()
        },
        CheckResult::Failed { kind, message } => {
            let failures = source.error_count.max(0) + 1;
            UpdateSourceInput {
                error_count: Some(failures),
                last_error: Some(Some(message.clone())),
                last_error_at: Some(Some(stamp.clone())),
                health_status: Some(Some(kind.status().to_string())),
                health_kind: Some(Some(kind.as_str().to_string())),
                last_checked_at: Some(Some(stamp.clone())),
                // After one failure the retry is an hour away; each further one doubles it.
                next_check_at: Some(Some((now + check_interval(failures - 1)).to_rfc3339())),
                ..Default::default()
            }
        }
    };
    if let Err(error) = Source::update_by_id(db, &source.id, update).await {
        warn!(source = %source.name, error = %redact_text(&error.to_string()), "Source health: could not record a check");
    }
    let alerts = Alerts::new(db);
    match result {
        CheckResult::Ok => alerts.recovered(&source.id, None).await,
        CheckResult::Failed { kind, message } => {
            if should_alert(*kind, source.error_count.max(0) + 1) {
                alerts
                    .failing(&source.id, &source.name, None, *kind, message)
                    .await;
            }
        }
    }
}

// ===========================================================================
// The checks
// ===========================================================================

/// Test one source over the network.
async fn test(sources: &SourcesManager, source: &Source) -> CheckResult {
    match tokio::time::timeout(TEST_TIMEOUT, sources.test_source(&source.id)).await {
        Err(_) => CheckResult::failed(
            FailureKind::Unreachable,
            format!(
                "the connection test timed out after {}s",
                TEST_TIMEOUT.as_secs()
            ),
        ),
        Ok(Ok(true)) => CheckResult::Ok,
        Ok(Ok(false)) => CheckResult::failed(
            FailureKind::Auth,
            "the connection test failed; check the credentials",
        ),
        Ok(Err(error)) => {
            let text = format!("{error:#}");
            if text.contains("Source not loaded") {
                CheckResult::failed(
                    FailureKind::Other,
                    "the source is not loaded; its credentials may not decrypt with the current key",
                )
            } else {
                CheckResult::from_error(&error)
            }
        }
    }
}

/// Check one source now: test it and record the result, ignoring the schedule and any recent use.
/// This is "Test now".
pub async fn check_source_now(
    manager: &Arc<ServicesManager>,
    source_id: &str,
) -> Result<CheckResult> {
    let db = manager
        .get_database()
        .await
        .context("database service not available")?
        .pool()
        .clone();
    let sources = manager
        .get_sources()
        .await
        .context("sources service not available")?
        .get_manager()
        .await
        .context("sources manager not initialized")?;
    let source = Source::get(db.pool(), &source_id.to_string())
        .await?
        .context("source not found")?;
    let result = test(&sources, &source).await;
    record_source_check(&db, &source, &result, Utc::now()).await;
    Ok(result)
}

/// What one pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HealthSummary {
    pub due: usize,
    pub tested: usize,
    pub from_use: usize,
    pub failing: usize,
}

/// Check every enabled source that is due: at most [`MAX_CHECKS_PER_PASS`], most overdue first, with
/// a random pause before each test. Cheap when nothing is due.
pub async fn run_due_checks(manager: &Arc<ServicesManager>) -> Result<HealthSummary> {
    let db = manager
        .get_database()
        .await
        .context("database service not available")?
        .pool()
        .clone();
    let Some(sources_service) = manager.get_sources().await else {
        return Ok(HealthSummary::default());
    };
    let Some(sources) = sources_service.get_manager().await else {
        return Ok(HealthSummary::default());
    };
    let now = Utc::now();
    let mut due: Vec<Source> = Source::query(db.pool())
        .fetch_all()
        .await
        .context("failed to list sources for the health check")?
        .into_iter()
        .filter(|source| source.enabled)
        .filter(|source| {
            check_due(
                source.last_checked_at.as_deref(),
                source.next_check_at.as_deref(),
                now,
            )
        })
        .collect();
    due.sort_by(|a, b| {
        a.next_check_at
            .as_deref()
            .or(a.last_checked_at.as_deref())
            .cmp(&b.next_check_at.as_deref().or(b.last_checked_at.as_deref()))
    });
    let mut summary = HealthSummary {
        due: due.len(),
        ..Default::default()
    };
    due.truncate(MAX_CHECKS_PER_PASS);

    let mut tested_any = false;
    for source in due {
        let checked_at = source
            .last_checked_at
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|time| time.with_timezone(&Utc));
        let result = match check_from_use(
            sources.latest_use(&source.id).as_ref(),
            checked_at,
            Utc::now(),
        ) {
            Some(result) => {
                summary.from_use += 1;
                debug!(source = %source.name, "Source health: a recent search counts as the check for '{}'", source.name);
                result
            }
            None => {
                if tested_any {
                    let seconds = rand::rng().random_range(JITTER_SECONDS.0..=JITTER_SECONDS.1);
                    tokio::time::sleep(Duration::from_secs(seconds)).await;
                }
                tested_any = true;
                summary.tested += 1;
                test(&sources, &source).await
            }
        };
        if let CheckResult::Failed { kind, message } = &result {
            summary.failing += 1;
            info!(
                source = %source.name,
                kind = kind.as_str(),
                "Source health: '{}' is failing ({}): {message}",
                source.name,
                kind.as_str()
            );
        }
        record_source_check(&db, &source, &result, Utc::now()).await;
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn a_sign_in_problem_alerts_at_once_and_the_rest_after_two_failures() {
        assert!(should_alert(FailureKind::Auth, 1));
        assert!(!should_alert(FailureKind::Unreachable, 1));
        assert!(should_alert(FailureKind::Unreachable, 2));
        assert!(!should_alert(FailureKind::RateLimited, 1));
    }

    #[test]
    fn the_alert_says_what_is_wrong_and_what_to_do() {
        let (title, message) =
            alert_text("IPTorrents", FailureKind::Auth, "its login has expired.");
        assert_eq!(title, "IPTorrents can't sign in");
        assert!(message.starts_with("IPTorrents can't sign in: its login has expired."));
        assert!(message.contains("Update its credentials"));

        let (title, message) = alert_text("Torznab", FailureKind::Unreachable, "timed out");
        assert_eq!(title, "Torznab is unreachable");
        assert!(message.contains("timed out"));
        assert!(
            alert_text("X", FailureKind::RateLimited, "")
                .0
                .contains("rate limiting")
        );
    }

    #[test]
    fn a_real_search_since_the_last_check_stands_for_the_check() {
        let now = at("2026-11-08T12:00:00Z");
        let good = SourceUse {
            at: at("2026-11-08T11:30:00Z"),
            ok: true,
            error: None,
        };
        assert_eq!(
            check_from_use(Some(&good), None, now),
            Some(CheckResult::Ok)
        );
        // A use that is older than the last check was already counted.
        assert_eq!(
            check_from_use(Some(&good), Some(at("2026-11-08T11:45:00Z")), now),
            None
        );
        // A use more than an hour old is no check.
        let stale = SourceUse {
            at: at("2026-11-08T10:30:00Z"),
            ok: true,
            error: None,
        };
        assert_eq!(check_from_use(Some(&stale), None, now), None);
        assert_eq!(check_from_use(None, None, now), None);
    }

    #[test]
    fn a_failed_search_is_a_failed_check_with_its_kind_and_without_addresses() {
        let now = at("2026-11-08T12:00:00Z");
        let bad = SourceUse {
            at: at("2026-11-08T11:50:00Z"),
            ok: false,
            error: Some("HTTP 403 for https://tracker.example/api?apikey=SECRET123".to_string()),
        };
        match check_from_use(Some(&bad), None, now) {
            Some(CheckResult::Failed { kind, message }) => {
                assert_eq!(kind, FailureKind::Auth);
                assert!(!message.contains("SECRET123"), "{message}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_ended_session_is_an_authentication_failure_whatever_it_says() {
        let error = anyhow::Error::new(SessionExpired("signing in again is paused".to_string()));
        assert!(matches!(
            CheckResult::from_error(&error),
            CheckResult::Failed {
                kind: FailureKind::Auth,
                ..
            }
        ));
        let other = anyhow::anyhow!("error sending request: dns error");
        assert!(matches!(
            CheckResult::from_error(&other),
            CheckResult::Failed {
                kind: FailureKind::Unreachable,
                ..
            }
        ));
    }
}
