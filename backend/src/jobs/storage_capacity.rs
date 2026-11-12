//! Watching free space on every volume Librarian writes to (`docs/design.md` Q146).
//!
//! - **What is measured.** Each library folder, the torrent download folder and Librarian's own
//!   data and cache folders. They are grouped by volume first (one share, or one device on this
//!   machine), and each volume is read once: `services::library_storage` asks the share with SMB2
//!   QUERY_INFO or this machine with `statvfs`, quotas applied.
//! - **The record.** One `StorageVolume` row per volume, written only when the reading moves
//!   meaningfully (`services::storage_capacity::worth_writing`), and `Library.storageVolumeId`
//!   pointing at it. A share that cannot be read is marked `unavailable` and keeps its last
//!   numbers. Nothing here ever fails a scan or an import: a failed reading is just recorded.
//! - **When.** Every 12 minutes, and half a minute after something nudges it ([`nudge`]): an
//!   import, a move of the download folder, a library added or moved.
//! - **The alert.** One notification (category `storage_space`) per volume and level for the
//!   admins and the owners of the libraries on it, naming what lives there. It is resolved when
//!   the level is left (with hysteresis), so a volume on the line does not raise it again and
//!   again, and an escalation to critical resolves the warning and raises the critical one.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use graphql_orm::graphql::filters::{BoolFilter, DateFilter, StringFilter};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::db::Database;
use crate::graphql::entities::{
    AppSetting, AppSettingWhereInput, CreateNotificationInput, CreateStorageVolumeInput, Library,
    Notification, NotificationWhereInput, StorageVolume, StorageVolumeWhereInput,
    UpdateLibraryInput, UpdateNotificationInput, UpdateStorageVolumeInput, User, UserWhereInput,
};
use crate::services::ServicesManager;
use crate::services::library_storage::{VolumeCapacity, library_storage};
use crate::services::redact::redact_text;
use crate::services::storage_capacity::{
    Level, Location, Resident, SETTINGS_CATEGORY, StoredReading, Thresholds, alert_text,
    grab_refusal, group_by_volume, level_for, volume_label, worth_writing,
};

pub const CATEGORY: &str = "storage_space";
/// `Notification.actionType` of these alerts; `actionData` is `<volume id>:<level>`.
pub const ACTION_TYPE: &str = "storage_volume";

/// How often every volume is read.
const PASS_INTERVAL: Duration = Duration::from_secs(12 * 60);
/// After a nudge, how long to wait for the burst it belongs to (an import of many files) to end.
const SETTLE: Duration = Duration::from_secs(30);
/// A reading that has not answered by now has failed; the share is treated as unavailable.
const READ_TIMEOUT: Duration = Duration::from_secs(20);

const STATUS_OK: &str = "ok";
const STATUS_UNAVAILABLE: &str = "unavailable";

static NUDGE: LazyLock<Notify> = LazyLock::new(Notify::new);

/// Ask for a reading soon: something was just written or moved. Cheap; bursts collapse into one.
pub fn nudge() {
    NUDGE.notify_one();
}

fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

// ===========================================================================
// Where to measure
// ===========================================================================

/// Librarian's own folders: the database's and the cache's, as the config reads them.
fn app_data_paths() -> Vec<String> {
    let database = std::env::var("DATABASE_PATH")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "./data/librarian.db".to_string());
    let database = database
        .trim_start_matches("sqlite://")
        .trim_start_matches("sqlite:")
        .to_string();
    let cache = std::env::var("CACHE_PATH").unwrap_or_else(|_| "./data/cache".to_string());
    let mut paths = Vec::new();
    for raw in [database, cache] {
        if raw.is_empty() || raw.contains(":memory:") {
            continue;
        }
        let path = std::path::Path::new(&raw);
        let folder = if raw.ends_with(".db") {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        let absolute = std::path::absolute(folder).unwrap_or_else(|_| folder.to_path_buf());
        paths.push(absolute.to_string_lossy().into_owned());
    }
    paths
}

async fn setting(db: &Database, key: &str) -> Option<String> {
    crate::services::torrent::get_setting_string(db, key)
        .await
        .ok()
        .flatten()
}

async fn thresholds(db: &Database) -> Thresholds {
    let rows = AppSetting::query(db.pool())
        .filter(AppSettingWhereInput {
            category: Some(string_eq(SETTINGS_CATEGORY)),
            ..Default::default()
        })
        .fetch_all()
        .await
        .unwrap_or_default();
    let values: HashMap<String, String> =
        rows.into_iter().map(|row| (row.key, row.value)).collect();
    Thresholds::from_settings(|key| values.get(key).cloned())
}

/// Every place to measure, with the libraries as read.
async fn locations(db: &Database, app_data: &[String]) -> Result<(Vec<Location>, Vec<Library>)> {
    let libraries = Library::query(db.pool())
        .fetch_all()
        .await
        .context("failed to list the libraries to measure")?;
    let mut out: Vec<Location> = libraries
        .iter()
        .map(|library| Location {
            resident: Resident::Library {
                id: library.id.clone(),
                name: library.name.clone(),
                owner_id: library.user_id.clone(),
            },
            path: library.path.clone(),
        })
        .collect();
    if let Some(dir) = setting(db, "torrent.download_dir").await {
        out.push(Location {
            resident: Resident::Downloads,
            path: dir,
        });
    }
    out.extend(app_data.iter().map(|path| Location {
        resident: Resident::AppData,
        path: path.clone(),
    }));
    Ok((out, libraries))
}

// ===========================================================================
// One pass
// ===========================================================================

/// What one pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PassSummary {
    pub volumes: usize,
    pub written: usize,
    pub unavailable: usize,
    pub low: usize,
}

/// How a volume's reading went.
type Reading = std::result::Result<VolumeCapacity, String>;

/// Read every volume once and store what moved. `app_data` is Librarian's own folders.
pub async fn run_pass_with(db: &Database, app_data: &[String]) -> Result<PassSummary> {
    let (locations, libraries) = locations(db, app_data).await?;
    let mut located = Vec::new();
    for location in locations {
        match library_storage().volume_key(location.path.as_str()).await {
            Ok(key) => located.push((key, location)),
            Err(error) => debug!(
                path = %location.path,
                error = %redact_text(&error.to_string()),
                "Storage capacity: '{}' cannot be placed on a volume; it is left out",
                location.path
            ),
        }
    }
    let groups = group_by_volume(located);
    let mut readings = BTreeMap::new();
    for (key, members) in &groups {
        readings.insert(key.clone(), read(key, members).await);
    }
    store(db, &groups, readings, &libraries, Utc::now()).await
}

/// Read one volume through the first of its folders, never waiting longer than [`READ_TIMEOUT`].
async fn read(key: &str, members: &[Location]) -> Reading {
    let Some(first) = members.first() else {
        return Err("nothing to measure".to_string());
    };
    match tokio::time::timeout(
        READ_TIMEOUT,
        library_storage().capacity(first.path.as_str()),
    )
    .await
    {
        Ok(Ok(capacity)) => Ok(capacity),
        Ok(Err(error)) => {
            let message = redact_text(&error.to_string());
            debug!(volume = %key, error = %message, "Storage capacity: could not read the free space of {key}");
            Err(message)
        }
        Err(_) => Err(format!(
            "reading the free space timed out after {}s",
            READ_TIMEOUT.as_secs()
        )),
    }
}

fn parse_time(value: Option<&str>) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value?)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}

fn library_names(members: &[Location]) -> Option<String> {
    let names: Vec<&str> = members
        .iter()
        .filter_map(|member| match &member.resident {
            Resident::Library { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    (!names.is_empty()).then(|| serde_json::to_string(&names).unwrap_or_default())
}

fn bytes(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// Write the readings, link the libraries, and raise or resolve the alerts. Separate from the
/// reading so it can be tested with readings made up.
async fn store(
    db: &Database,
    groups: &BTreeMap<String, Vec<Location>>,
    mut readings: BTreeMap<String, Reading>,
    libraries: &[Library],
    now: DateTime<Utc>,
) -> Result<PassSummary> {
    let thresholds = thresholds(db).await;
    let existing: HashMap<String, StorageVolume> = StorageVolume::query(db.pool())
        .fetch_all()
        .await
        .context("failed to list the stored volumes")?
        .into_iter()
        .map(|row| (row.volume_key.clone(), row))
        .collect();
    let alerts = Alerts::new(db);
    let mut summary = PassSummary {
        volumes: groups.len(),
        ..Default::default()
    };

    for (key, members) in groups {
        let reading = readings
            .remove(key)
            .unwrap_or_else(|| Err("not read".to_string()));
        let stored = existing.get(key);
        let previous = stored.map_or(Level::Normal, |row| Level::parse(&row.level));
        let level = match &reading {
            Ok(capacity) => level_for(
                previous,
                capacity.available_bytes,
                capacity.total_bytes,
                &thresholds,
            ),
            Err(_) => previous,
        };
        let label = volume_label(key, members);
        let holds_downloads = members.iter().any(|m| m.resident == Resident::Downloads);
        let holds_app_data = members.iter().any(|m| m.resident == Resident::AppData);
        let names = library_names(members);
        let kind = if key.starts_with("smb://") {
            "smb"
        } else {
            "local"
        };

        let row = match stored {
            None => {
                let input = CreateStorageVolumeInput {
                    volume_key: key.clone(),
                    kind: kind.to_string(),
                    label: label.clone(),
                    status: if reading.is_ok() {
                        STATUS_OK
                    } else {
                        STATUS_UNAVAILABLE
                    }
                    .to_string(),
                    level: level.as_str().to_string(),
                    total_bytes: reading.as_ref().ok().map(|c| bytes(c.total_bytes)),
                    available_bytes: reading.as_ref().ok().map(|c| bytes(c.available_bytes)),
                    free_bytes: reading.as_ref().ok().map(|c| bytes(c.free_bytes)),
                    detail: reading.as_ref().err().cloned(),
                    holds_downloads,
                    holds_app_data,
                    library_names: names.clone(),
                    checked_at: reading.is_ok().then(|| now.to_rfc3339()),
                };
                summary.written += 1;
                match StorageVolume::insert(db, input).await {
                    Ok(row) => row,
                    Err(error) => {
                        warn!(volume = %label, error = %error, "Storage capacity: could not record the volume {label}");
                        continue;
                    }
                }
            }
            Some(row) => {
                let before = StoredReading {
                    available: row.available_bytes.map(|v| v.max(0) as u64),
                    total: row.total_bytes.map(|v| v.max(0) as u64),
                    level: previous,
                    reachable: row.status == STATUS_OK,
                    checked_at: parse_time(row.checked_at.as_deref()),
                };
                let after = StoredReading {
                    available: reading.as_ref().ok().map(|c| c.available_bytes),
                    total: reading.as_ref().ok().map(|c| c.total_bytes),
                    level,
                    reachable: reading.is_ok(),
                    checked_at: Some(now),
                };
                let membership_changed = row.label != label
                    || row.holds_downloads != holds_downloads
                    || row.holds_app_data != holds_app_data
                    || row.library_names != names;
                if worth_writing(&before, &after, now) || membership_changed {
                    let mut update = UpdateStorageVolumeInput {
                        label: Some(label.clone()),
                        level: Some(level.as_str().to_string()),
                        holds_downloads: Some(holds_downloads),
                        holds_app_data: Some(holds_app_data),
                        library_names: Some(names.clone()),
                        ..Default::default()
                    };
                    match &reading {
                        Ok(capacity) => {
                            update.status = Some(STATUS_OK.to_string());
                            update.total_bytes = Some(Some(bytes(capacity.total_bytes)));
                            update.available_bytes = Some(Some(bytes(capacity.available_bytes)));
                            update.free_bytes = Some(Some(bytes(capacity.free_bytes)));
                            update.detail = Some(None);
                            update.checked_at = Some(Some(now.to_rfc3339()));
                        }
                        Err(message) => {
                            update.status = Some(STATUS_UNAVAILABLE.to_string());
                            update.detail = Some(Some(message.clone()));
                        }
                    }
                    summary.written += 1;
                    match StorageVolume::update_by_id(db, &row.id, update).await {
                        Ok(Some(updated)) => updated,
                        Ok(None) => row.clone(),
                        Err(error) => {
                            warn!(volume = %label, error = %error, "Storage capacity: could not update the volume {label}");
                            row.clone()
                        }
                    }
                } else {
                    row.clone()
                }
            }
        };

        if reading.is_err() {
            summary.unavailable += 1;
        }
        if level != Level::Normal {
            summary.low += 1;
        }
        if previous != level {
            info!(
                volume = %label,
                from = previous.as_str(),
                to = level.as_str(),
                "Storage capacity: {label} went from {} to {}",
                previous.as_str(),
                level.as_str()
            );
        }

        // Point each library on it here.
        for member in members {
            let Resident::Library { id, .. } = &member.resident else {
                continue;
            };
            let current = libraries
                .iter()
                .find(|library| &library.id == id)
                .and_then(|library| library.storage_volume_id.as_deref());
            if current != Some(row.id.as_str())
                && let Err(error) = Library::update_by_id(
                    db,
                    id,
                    UpdateLibraryInput {
                        storage_volume_id: Some(Some(row.id.clone())),
                        ..Default::default()
                    },
                )
                .await
            {
                warn!(library_id = %id, error = %error, "Storage capacity: could not link library {id} to the volume {label}");
            }
        }

        // An unreadable volume keeps whatever alert it had: nothing new is known.
        if let Ok(capacity) = &reading {
            alerts.sync(&row, level, capacity, members).await;
        }
    }

    // Volumes nothing lives on any more.
    for (key, row) in &existing {
        if groups.contains_key(key) {
            continue;
        }
        alerts.sync_level(&row.id, None, Vec::new(), None).await;
        if let Err(error) = StorageVolume::delete_by_id(db, &row.id).await {
            warn!(volume = %row.label, error = %error, "Storage capacity: could not remove the unused volume {}", row.label);
        }
    }
    Ok(summary)
}

/// Run a pass with Librarian's own folders found from the environment.
pub async fn run_pass(db: &Database) -> Result<PassSummary> {
    run_pass_with(db, &app_data_paths()).await
}

/// The background job: a pass every [`PASS_INTERVAL`], and soon after a [`nudge`].
pub async fn run_loop(
    manager: Arc<ServicesManager>,
    cancel: CancellationToken,
    startup_delay: Duration,
) {
    tokio::select! {
        _ = cancel.cancelled() => return,
        _ = tokio::time::sleep(startup_delay) => {}
    }
    loop {
        match manager.get_database().await {
            Some(service) => match run_pass(service.pool()).await {
                Ok(summary) => debug!(
                    volumes = summary.volumes,
                    written = summary.written,
                    unavailable = summary.unavailable,
                    low = summary.low,
                    "Storage capacity: read {} volume(s), {} changed, {} unavailable, {} low on space",
                    summary.volumes,
                    summary.written,
                    summary.unavailable,
                    summary.low
                ),
                Err(error) => warn!(error = %error, "Storage capacity: the pass failed"),
            },
            None => debug!("Storage capacity: no database yet; skipping a pass"),
        }
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(PASS_INTERVAL) => {}
            _ = NUDGE.notified() => {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = tokio::time::sleep(SETTLE) => {}
                }
            }
        }
    }
}

// ===========================================================================
// Grabs onto a full disk
// ===========================================================================

/// Why a release of `size` bytes may not be grabbed now, or `None`: the download folder's volume
/// is critically low and the release would not fit (Q146). Reads the stored reading only.
pub async fn download_refusal(db: &Database, size: Option<i64>) -> Option<String> {
    size?;
    let volume = StorageVolume::query(db.pool())
        .filter(StorageVolumeWhereInput {
            holds_downloads: Some(BoolFilter {
                eq: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await
        .ok()?
        .into_iter()
        .find(|row| row.status == STATUS_OK)?;
    grab_refusal(
        Level::parse(&volume.level),
        volume.available_bytes?.max(0) as u64,
        size,
    )
}

// ===========================================================================
// Alerts
// ===========================================================================

fn notification_type(level: Level) -> &'static str {
    match level {
        Level::Critical => "ERROR",
        _ => "WARNING",
    }
}

fn action_data(volume_id: &str, level: Level) -> String {
    format!("{volume_id}:{}", level.as_str())
}

/// Raises and resolves the notifications about low volumes.
pub struct Alerts {
    db: Database,
}

impl Alerts {
    pub fn new(db: &Database) -> Self {
        Self { db: db.clone() }
    }

    /// The admins and the owners of the libraries on the volume.
    async fn recipients(&self, members: &[Location]) -> Vec<String> {
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
                warn!(error = %error, "Storage capacity: could not list the admins to tell");
                Vec::new()
            }
        };
        for member in members {
            if let Resident::Library { owner_id, .. } = &member.resident
                && !ids.contains(owner_id)
            {
                ids.push(owner_id.clone());
            }
        }
        ids
    }

    /// Every open alert about the volume, for anyone.
    async fn open(&self, volume_id: &str) -> Vec<Notification> {
        let prefix = format!("{volume_id}:");
        Notification::query(self.db.pool())
            .filter(NotificationWhereInput {
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
                |error| warn!(error = %error, "Storage capacity: could not look for open alerts"),
            )
            .unwrap_or_default()
            .into_iter()
            .filter(|row| {
                row.action_data
                    .as_deref()
                    .is_some_and(|data| data.starts_with(&prefix))
            })
            .collect()
    }

    /// Bring the alerts for `volume` in line with `level`.
    pub async fn sync(
        &self,
        volume: &StorageVolume,
        level: Level,
        capacity: &VolumeCapacity,
        members: &[Location],
    ) {
        let wanted = (level != Level::Normal).then(|| {
            alert_text(
                level,
                &volume.label,
                capacity.available_bytes,
                capacity.total_bytes,
                members,
            )
        });
        let recipients = if wanted.is_some() {
            self.recipients(members).await
        } else {
            Vec::new()
        };
        self.sync_level(
            &volume.id,
            (level != Level::Normal).then_some(level),
            recipients,
            wanted,
        )
        .await;
    }

    /// Resolve every open alert for the volume at another level than `level`, and open one at
    /// `level` for each recipient who has none.
    async fn sync_level(
        &self,
        volume_id: &str,
        level: Option<Level>,
        recipients: Vec<String>,
        text: Option<(String, String)>,
    ) {
        let open = self.open(volume_id).await;
        let wanted = level.map(|level| action_data(volume_id, level));
        let mut covered = HashSet::new();
        for row in &open {
            if wanted.as_deref() == row.action_data.as_deref() {
                covered.insert(row.user_id.clone());
                continue;
            }
            let was = row
                .action_data
                .as_deref()
                .and_then(|data| data.rsplit(':').next())
                .map(Level::parse)
                .unwrap_or(Level::Warning);
            let resolution = match level {
                None => "recovered",
                Some(now) if now > was => "escalated",
                Some(_) => "eased",
            };
            if let Err(error) = Notification::update_by_id(
                &self.db,
                &row.id,
                UpdateNotificationInput {
                    resolved_at: Some(Some(Utc::now().to_rfc3339())),
                    resolution: Some(Some(resolution.to_string())),
                    ..Default::default()
                },
            )
            .await
            {
                warn!(error = %error, "Storage capacity: could not resolve an alert");
            }
        }
        let (Some(level), Some(data), Some((title, message))) = (level, wanted, text) else {
            return;
        };
        for user_id in recipients {
            if covered.contains(&user_id) {
                continue;
            }
            if let Err(error) = Notification::insert(
                &self.db,
                CreateNotificationInput {
                    user_id: user_id.clone(),
                    notification_type: notification_type(level).to_string(),
                    category: CATEGORY.to_string(),
                    title: title.clone(),
                    message: message.clone(),
                    library_id: None,
                    torrent_id: None,
                    media_file_id: None,
                    pending_match_id: None,
                    action_type: Some(ACTION_TYPE.to_string()),
                    action_data: Some(data.clone()),
                    read_at: None,
                    resolved_at: None,
                    resolution: None,
                },
            )
            .await
            {
                warn!(error = %error, "Storage capacity: could not raise an alert: {title}");
            }
        }
    }
}

#[cfg(test)]
#[path = "storage_capacity_tests.rs"]
mod tests;
