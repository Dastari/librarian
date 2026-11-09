//! The trail automatic downloading leaves in Activity.
//!
//! Every notification is category `auto_download` and belongs to the owner of the library the
//! target lives in; a job never picks a user of its own.
//!
//! - A **grab** gets one notification of its own: what was taken, from where, under which
//!   profile, and what triggered it (hourly sweep, air time, an RSS feed, a retry).
//! - A target that was **searched without result** keeps a single open "Waiting for …"
//!   notification, rewritten on every attempt with why nothing was taken (no release, below the
//!   profile, blocklisted, wrong episode), how many attempts there were and when the next one
//!   is due. It is resolved when the target is finally grabbed. Searches never pile up rows.
//! - A **failed grab** and the **cap** being reached are likewise one open notification each.
//!
//! Writing the trail must never get in the way of downloading, so every failure here is logged
//! and swallowed.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use graphql_orm::graphql::filters::{DateFilter, StringFilter};
use tracing::warn;

use crate::db::Database;
use crate::graphql::entities::{
    CreateNotificationInput, Library, Notification, NotificationWhereInput, UpdateNotificationInput,
};

pub const CATEGORY: &str = "auto_download";

/// What started a search or grab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    /// The whole-library sweep on its interval.
    Sweep,
    /// The air-time pass for episodes that have just aired.
    AirTime,
    /// A person pressed Search or Run now.
    Manual,
    /// A dead grab was blocklisted and its target searched again.
    Retry,
    /// A new item in this RSS feed.
    Feed(String),
}

impl Trigger {
    pub fn describe(&self) -> String {
        match self {
            Self::Sweep => "the scheduled sweep".to_string(),
            Self::AirTime => "the air-time check".to_string(),
            Self::Manual => "a manual search".to_string(),
            Self::Retry => "a retry after a failed grab".to_string(),
            Self::Feed(name) => format!("the RSS feed '{name}'"),
        }
    }

    /// Short form for logs.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Sweep => "sweep",
            Self::AirTime => "air-time",
            Self::Manual => "manual",
            Self::Retry => "retry",
            Self::Feed(_) => "rss",
        }
    }
}

/// A grab to record.
pub struct GrabRecord<'a> {
    pub library_id: &'a str,
    pub label: &'a str,
    pub release: &'a str,
    pub source: Option<&'a str>,
    pub profile: &'a str,
    pub trigger: &'a Trigger,
    pub torrent_id: Option<&'a str>,
}

pub struct Trail {
    db: Database,
    /// `library id -> owner`, looked up once per pass.
    owners: HashMap<String, Option<String>>,
    /// `owner:title -> first opened at` for "Waiting for …" notifications, loaded on first use.
    opened: Option<HashMap<String, String>>,
}

/// App setting remembering which "Waiting for …" notifications were opened (Q113), so one the
/// owner dismissed or deleted is not opened again by the next automatic search.
pub const OPENED_SETTING: &str = "auto_download.waiting_opened";

/// How long after it aired or came out a fresh target can go unfound before a "Waiting for …"
/// notification is opened: releases often take most of a day to appear, and a person does not need
/// telling about each hour of that.
pub const QUIET_AFTER_RELEASE: chrono::Duration = chrono::Duration::hours(24);

/// Whether a search that found nothing may open a new "Waiting for …" notification (an open one is
/// always kept up to date). A manual search always may. Automatic searches never do for single
/// tracks or back-catalogue episodes (Q84), nor for a fresh target in its first day out (Q113).
pub fn opens_waiting(
    trigger: &Trigger,
    is_track: bool,
    timing: super::auto_schedule::Timing,
    is_episode_or_pack: bool,
    released_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    use super::auto_schedule::Timing;
    if *trigger == Trigger::Manual {
        return true;
    }
    let long_tail = is_track || (timing == Timing::Back && is_episode_or_pack);
    if long_tail {
        return false;
    }
    !(timing == Timing::Fresh
        && released_at.is_some_and(|released| now - released < QUIET_AFTER_RELEASE))
}

fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

impl Trail {
    pub fn new(db: &Database) -> Self {
        Self {
            db: db.clone(),
            owners: HashMap::new(),
            opened: None,
        }
    }

    async fn owner(&mut self, library_id: &str) -> Option<String> {
        if let Some(owner) = self.owners.get(library_id) {
            return owner.clone();
        }
        let owner = match Library::get(self.db.pool(), &library_id.to_string()).await {
            Ok(library) => library.map(|library| library.user_id),
            Err(error) => {
                warn!(
                    library_id,
                    error = %error,
                    "Auto-download: could not look up the library owner for a notification"
                );
                return None;
            }
        };
        self.owners.insert(library_id.to_string(), owner.clone());
        owner
    }

    async fn open_notification(&self, owner: &str, title: &str) -> Option<Notification> {
        match Notification::query(self.db.pool())
            .filter(NotificationWhereInput {
                user_id: Some(string_eq(owner)),
                category: Some(string_eq(CATEGORY)),
                title: Some(string_eq(title)),
                resolved_at: Some(DateFilter {
                    is_null: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .fetch_all()
            .await
        {
            Ok(rows) => rows.into_iter().next(),
            Err(error) => {
                warn!(title, error = %error, "Auto-download: could not look for an open notification");
                None
            }
        }
    }

    /// Create the notification, or rewrite the open one with the same title. With `create` false
    /// only an open one is rewritten.
    async fn upsert_open(
        &mut self,
        library_id: &str,
        level: &str,
        title: &str,
        message: String,
        torrent_id: Option<&str>,
        create: bool,
    ) {
        let Some(owner) = self.owner(library_id).await else {
            return;
        };
        if let Some(existing) = self.open_notification(&owner, title).await {
            if existing.message == message && existing.notification_type == level {
                return;
            }
            if let Err(error) = Notification::update_by_id(
                &self.db,
                &existing.id,
                UpdateNotificationInput {
                    notification_type: Some(level.to_string()),
                    message: Some(message),
                    ..Default::default()
                },
            )
            .await
            {
                warn!(title, error = %error, "Auto-download: could not update a notification");
            }
            return;
        }
        if create {
            self.insert(&owner, library_id, level, title, message, torrent_id)
                .await;
        }
    }

    async fn insert(
        &self,
        owner: &str,
        library_id: &str,
        level: &str,
        title: &str,
        message: String,
        torrent_id: Option<&str>,
    ) {
        if let Err(error) = Notification::insert(
            &self.db,
            CreateNotificationInput {
                user_id: owner.to_string(),
                notification_type: level.to_string(),
                category: CATEGORY.to_string(),
                title: title.to_string(),
                message,
                library_id: Some(library_id.to_string()),
                torrent_id: torrent_id.map(ToString::to_string),
                media_file_id: None,
                pending_match_id: None,
                action_type: None,
                action_data: None,
                read_at: None,
                resolved_at: None,
                resolution: None,
            },
        )
        .await
        {
            warn!(title, error = %error, "Auto-download: could not create a notification");
        }
    }

    async fn resolve(&mut self, library_id: &str, title: &str, resolution: &str) {
        let Some(owner) = self.owner(library_id).await else {
            return;
        };
        let Some(open) = self.open_notification(&owner, title).await else {
            return;
        };
        if let Err(error) = Notification::update_by_id(
            &self.db,
            &open.id,
            UpdateNotificationInput {
                resolved_at: Some(Some(Utc::now().to_rfc3339())),
                resolution: Some(Some(resolution.to_string())),
                ..Default::default()
            },
        )
        .await
        {
            warn!(title, error = %error, "Auto-download: could not resolve a notification");
        }
    }

    /// A release was grabbed. Settles any "Waiting for …" notification for the same target.
    pub async fn grabbed(&mut self, grab: &GrabRecord<'_>) {
        let message = format!(
            "Grabbed '{}'{} under the '{}' profile, found by {}.",
            grab.release,
            grab.source
                .map(|source| format!(" from {source}"))
                .unwrap_or_default(),
            grab.profile,
            grab.trigger.describe()
        );
        let Some(owner) = self.owner(grab.library_id).await else {
            return;
        };
        self.insert(
            &owner,
            grab.library_id,
            "SUCCESS",
            &format!("Grabbed {}", grab.label),
            message,
            grab.torrent_id,
        )
        .await;
        self.resolve(grab.library_id, &waiting_title(grab.label), "grabbed")
            .await;
        // Grabbed: a later wait for the same target may be told about again.
        self.set_opened(format!("{owner}:{}", waiting_title(grab.label)), false)
            .await;
        self.resolve(grab.library_id, &failed_title(grab.label), "grabbed")
            .await;
    }

    /// Searched, nothing taken. `create` is false for the long tail of back-catalogue episodes,
    /// which would otherwise open hundreds of notifications on the first sweep: those only keep
    /// an already open one up to date (and are always in the log).
    pub async fn waiting(
        &mut self,
        library_id: &str,
        label: &str,
        why: &str,
        next_search: Option<DateTime<Utc>>,
        trigger: &Trigger,
        create: bool,
    ) {
        let next = next_search
            .map(|next| {
                format!(
                    " Next automatic search after {}.",
                    next.format("%Y-%m-%d %H:%M UTC")
                )
            })
            .unwrap_or_default();
        let message = format!(
            "Last searched {} by {}: {why}.{next}",
            Utc::now().format("%Y-%m-%d %H:%M UTC"),
            trigger.describe()
        );
        let title = waiting_title(label);
        // Opened once already and no longer open: the owner dismissed or deleted it. An automatic
        // search does not open it again; a manual one does.
        let mut create = create;
        let mut remember: Option<String> = None;
        if create && let Some(owner) = self.owner(library_id).await {
            let key = format!("{owner}:{title}");
            let seen = self.opened_map().await.contains_key(&key);
            if seen
                && *trigger != Trigger::Manual
                && self.open_notification(&owner, &title).await.is_none()
            {
                create = false;
            } else if !seen {
                remember = Some(key);
            }
        }
        self.upsert_open(library_id, "INFO", &title, message, None, create)
            .await;
        if let Some(key) = remember {
            self.set_opened(key, true).await;
        }
    }

    async fn opened_map(&mut self) -> &HashMap<String, String> {
        if self.opened.is_none() {
            self.opened = Some(super::auto_download::load_time_map(&self.db, OPENED_SETTING).await);
        }
        self.opened.get_or_insert_with(HashMap::new)
    }

    /// Remember (or forget) that a target's "Waiting for …" notification was opened.
    async fn set_opened(&mut self, key: String, opened: bool) {
        let _ = self.opened_map().await;
        let Some(map) = self.opened.as_mut() else {
            return;
        };
        let changed = if opened {
            map.insert(key, Utc::now().to_rfc3339());
            true
        } else {
            map.remove(&key).is_some()
        };
        if !changed {
            return;
        }
        let snapshot = map.clone();
        if let Err(error) = super::auto_download::save_time_map(
            &self.db,
            OPENED_SETTING,
            "Auto-download: when each \"Waiting for\" notification was first opened",
            &snapshot,
        )
        .await
        {
            warn!(error = %error, "Auto-download: could not remember an opened notification");
        }
    }

    /// A grab that never came to anything was put on the blocklist.
    pub async fn blocklisted(
        &mut self,
        library_id: &str,
        torrent_name: &str,
        reason: &str,
        searching_again: bool,
        torrent_id: Option<&str>,
    ) {
        let Some(owner) = self.owner(library_id).await else {
            return;
        };
        let reason = match reason {
            "stalled" => "it made no progress for a day",
            "import_failed" => "it could not be imported",
            other => other,
        };
        self.insert(
            &owner,
            library_id,
            "WARNING",
            &format!("Blocklisted {torrent_name}"),
            format!(
                "'{torrent_name}' was blocklisted because {reason}. {}",
                if searching_again {
                    "Searching for another release now."
                } else {
                    "Automatic downloading is off, so nothing will search again until you search by hand."
                }
            ),
            torrent_id,
        )
        .await;
    }

    /// A release was chosen but could not be added to the download client.
    pub async fn grab_failed(&mut self, library_id: &str, label: &str, release: &str, error: &str) {
        self.upsert_open(
            library_id,
            "ERROR",
            &failed_title(label),
            format!("Could not add '{release}': {error}"),
            None,
            true,
        )
        .await;
    }

    /// The limit on automatic grabs was reached, so `waiting` targets were left for later.
    pub async fn cap_reached(
        &mut self,
        library_id: &str,
        in_flight: usize,
        cap: usize,
        waiting: usize,
    ) {
        self.upsert_open(
            library_id,
            "INFO",
            CAP_TITLE,
            format!(
                "{in_flight} automatic downloads are in progress (limit {cap}), so {waiting} wanted \
                 item(s) will be searched once a slot is free."
            ),
            None,
            true,
        )
        .await;
    }

    /// Slots are free again.
    pub async fn cap_cleared(&mut self, library_id: &str) {
        self.resolve(library_id, CAP_TITLE, "cleared").await;
    }
}

const CAP_TITLE: &str = "Automatic downloads are waiting for a free slot";

pub fn waiting_title(label: &str) -> String {
    format!("Waiting for {label}")
}

pub fn failed_title(label: &str) -> String {
    format!("Could not grab {label}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_target_opens_no_notification_in_its_first_day() {
        use crate::jobs::auto_schedule::Timing;
        let now = DateTime::parse_from_rfc3339("2026-11-09T20:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let aired = |hours: i64| Some(now - chrono::Duration::hours(hours));
        // An episode that aired three hours ago and has no release yet: quiet.
        assert!(!opens_waiting(
            &Trigger::AirTime,
            false,
            Timing::Fresh,
            true,
            aired(3),
            now
        ));
        assert!(!opens_waiting(
            &Trigger::Sweep,
            false,
            Timing::Fresh,
            true,
            aired(23),
            now
        ));
        // A day on with nothing: one notification.
        assert!(opens_waiting(
            &Trigger::AirTime,
            false,
            Timing::Fresh,
            true,
            aired(25),
            now
        ));
        // A person searching is always told.
        assert!(opens_waiting(
            &Trigger::Manual,
            false,
            Timing::Fresh,
            true,
            aired(1),
            now
        ));
        // Back-catalogue episodes and single tracks never open one automatically.
        assert!(!opens_waiting(
            &Trigger::Sweep,
            false,
            Timing::Back,
            true,
            None,
            now
        ));
        assert!(!opens_waiting(
            &Trigger::Sweep,
            true,
            Timing::Back,
            false,
            None,
            now
        ));
        // A movie with no home release date keeps the old rule.
        assert!(opens_waiting(
            &Trigger::Sweep,
            false,
            Timing::Back,
            false,
            None,
            now
        ));
    }

    #[test]
    fn titles_are_stable_keys_per_target() {
        assert_eq!(waiting_title("Show S01E02"), "Waiting for Show S01E02");
        assert_eq!(failed_title("Show S01E02"), "Could not grab Show S01E02");
    }

    #[test]
    fn every_trigger_says_where_a_grab_came_from() {
        assert_eq!(Trigger::Sweep.tag(), "sweep");
        assert!(Trigger::AirTime.describe().contains("air-time"));
        assert!(
            Trigger::Feed("TV feed".into())
                .describe()
                .contains("TV feed")
        );
        assert_eq!(Trigger::Feed("x".into()).tag(), "rss");
    }
}
