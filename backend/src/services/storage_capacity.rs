//! Free space on the volumes Librarian writes to: the rules, without any I/O (`docs/design.md`
//! Q146). The job that measures and stores readings is `jobs::storage_capacity`.
//!
//! - **Levels.** A volume is `warning` below 10 % or 50 GB free, whichever is larger, and
//!   `critical` below 5 % or 20 GB (Settings › General › Storage changes them).
//! - **Hysteresis.** A level is left only once the space is back above its threshold by a margin
//!   (a tenth of the threshold, at least 1 GB), so a volume sitting on the line does not flap
//!   between alert and recovery.
//! - **Quiet writes.** A reading is stored only when it moves by more than 0.5 % of the volume,
//!   changes level or status, or the stored one is a day old.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

/// The setting keys (category `storage`). Fractions are 0–1; sizes are bytes.
pub const WARNING_FRACTION_KEY: &str = "storage.warning_free_fraction";
pub const WARNING_BYTES_KEY: &str = "storage.warning_free_bytes";
pub const CRITICAL_FRACTION_KEY: &str = "storage.critical_free_fraction";
pub const CRITICAL_BYTES_KEY: &str = "storage.critical_free_bytes";
pub const SETTINGS_CATEGORY: &str = "storage";

const GB: u64 = 1_000_000_000;
/// The smallest recovery margin, so a tiny threshold still has some.
const MIN_MARGIN: u64 = GB;
/// A change of available space smaller than this share of the volume is not written.
const MEANINGFUL_FRACTION: f64 = 0.005;
/// A stored reading older than this is refreshed even if nothing moved.
const STALE_AFTER_HOURS: i64 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Normal,
    Warning,
    Critical,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "warning" => Self::Warning,
            "critical" => Self::Critical,
            _ => Self::Normal,
        }
    }
}

/// When a volume is low and when it is critically low.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    pub warning_fraction: f64,
    pub warning_bytes: u64,
    pub critical_fraction: f64,
    pub critical_bytes: u64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            warning_fraction: 0.10,
            warning_bytes: 50 * GB,
            critical_fraction: 0.05,
            critical_bytes: 20 * GB,
        }
    }
}

impl Thresholds {
    /// From the stored settings, falling back to the defaults for anything missing or out of
    /// range. Critical is never looser than warning.
    pub fn from_settings(get: impl Fn(&str) -> Option<String>) -> Self {
        let defaults = Self::default();
        let fraction = |key: &str, fallback: f64| {
            get(key)
                .and_then(|value| value.trim().trim_matches('"').parse::<f64>().ok())
                .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
                .unwrap_or(fallback)
        };
        let bytes = |key: &str, fallback: u64| {
            get(key)
                .and_then(|value| value.trim().trim_matches('"').parse::<f64>().ok())
                .filter(|value| value.is_finite() && *value >= 0.0)
                .map(|value| value as u64)
                .unwrap_or(fallback)
        };
        let warning_fraction = fraction(WARNING_FRACTION_KEY, defaults.warning_fraction);
        let warning_bytes = bytes(WARNING_BYTES_KEY, defaults.warning_bytes);
        Self {
            warning_fraction,
            warning_bytes,
            critical_fraction: fraction(CRITICAL_FRACTION_KEY, defaults.critical_fraction)
                .min(warning_fraction),
            critical_bytes: bytes(CRITICAL_BYTES_KEY, defaults.critical_bytes).min(warning_bytes),
        }
    }

    /// The free bytes below which `level` applies on a volume of `total` bytes: the larger of
    /// the share and the size. `Normal` has no floor.
    pub fn floor(&self, level: Level, total: u64) -> u64 {
        let (fraction, bytes) = match level {
            Level::Normal => return 0,
            Level::Warning => (self.warning_fraction, self.warning_bytes),
            Level::Critical => (self.critical_fraction, self.critical_bytes),
        };
        ((total as f64 * fraction) as u64).max(bytes)
    }
}

/// How far above a threshold the space must climb before its level is left.
pub fn recovery_margin(floor: u64) -> u64 {
    (floor / 10).max(MIN_MARGIN)
}

/// The level for `available` free bytes, given the level the volume was at.
pub fn level_for(previous: Level, available: u64, total: u64, thresholds: &Thresholds) -> Level {
    let warning = thresholds.floor(Level::Warning, total);
    let critical = thresholds.floor(Level::Critical, total);
    let raw = if available < critical {
        Level::Critical
    } else if available < warning {
        Level::Warning
    } else {
        Level::Normal
    };
    match previous {
        Level::Critical if available < critical.saturating_add(recovery_margin(critical)) => {
            Level::Critical
        }
        Level::Critical | Level::Warning
            if raw == Level::Normal
                && available < warning.saturating_add(recovery_margin(warning)) =>
        {
            Level::Warning
        }
        _ => raw,
    }
}

/// What is stored for a volume, for deciding whether a new reading is worth a write.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredReading {
    pub available: Option<u64>,
    pub total: Option<u64>,
    pub level: Level,
    pub reachable: bool,
    pub checked_at: Option<DateTime<Utc>>,
}

/// Whether `new` differs enough from `stored` to write it.
pub fn worth_writing(stored: &StoredReading, new: &StoredReading, now: DateTime<Utc>) -> bool {
    if stored.level != new.level || stored.reachable != new.reachable {
        return true;
    }
    if !new.reachable {
        // An unreachable volume keeps its last reading; there is nothing new to write.
        return false;
    }
    let (Some(old_available), Some(old_total), Some(available), Some(total)) =
        (stored.available, stored.total, new.available, new.total)
    else {
        return stored.available != new.available || stored.total != new.total;
    };
    let step = (total as f64 * MEANINGFUL_FRACTION) as u64;
    if old_total.abs_diff(total) > step || old_available.abs_diff(available) > step {
        return true;
    }
    stored
        .checked_at
        .is_none_or(|at| now - at >= Duration::hours(STALE_AFTER_HOURS))
}

// ===========================================================================
// What lives on a volume
// ===========================================================================

/// Something Librarian keeps on a volume.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Resident {
    Library {
        id: String,
        name: String,
        owner_id: String,
    },
    Downloads,
    AppData,
}

/// A place to measure: what it is and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub resident: Resident,
    pub path: String,
}

/// Locations gathered by volume key, each volume's residents in a stable order (libraries by
/// name, then the download folder, then Librarian's data). One measurement per key.
pub fn group_by_volume(located: Vec<(String, Location)>) -> BTreeMap<String, Vec<Location>> {
    let mut groups: BTreeMap<String, Vec<Location>> = BTreeMap::new();
    for (key, location) in located {
        let members = groups.entry(key).or_default();
        if !members
            .iter()
            .any(|member| member.resident == location.resident)
        {
            members.push(location);
        }
    }
    for members in groups.values_mut() {
        members.sort_by(|a, b| match (&a.resident, &b.resident) {
            (Resident::Library { name: x, .. }, Resident::Library { name: y, .. }) => {
                x.to_lowercase().cmp(&y.to_lowercase())
            }
            (x, y) => rank(x).cmp(&rank(y)),
        });
    }
    groups
}

fn rank(resident: &Resident) -> u8 {
    match resident {
        Resident::Library { .. } => 0,
        Resident::Downloads => 1,
        Resident::AppData => 2,
    }
}

/// What a person calls the volume: "media on 10.0.1.190" for a share, else the folder measured.
pub fn volume_label(key: &str, members: &[Location]) -> String {
    if let Ok(location) = crate::services::library_storage::SmbLocation::parse(key) {
        return format!("{} on {}", location.share, location.host);
    }
    members
        .first()
        .map(|member| member.path.clone())
        .unwrap_or_else(|| key.to_string())
}

// ===========================================================================
// Words
// ===========================================================================

/// Bytes the way disks are sold: "1.2 TB", "48 GB", "900 MB".
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else if value < 10.0 {
        let text = format!("{value:.1}");
        format!("{} {}", text.trim_end_matches(".0"), UNITS[unit])
    } else {
        format!("{value:.0} {}", UNITS[unit])
    }
}

/// "the Movies and TV libraries, the download folder and Librarian's own data".
pub fn residents_text(members: &[Location]) -> String {
    let libraries: Vec<&str> = members
        .iter()
        .filter_map(|member| match &member.resident {
            Resident::Library { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let mut parts = Vec::new();
    match libraries.as_slice() {
        [] => {}
        [one] => parts.push(format!("the {one} library")),
        many => parts.push(format!("the {} libraries", join_and(many))),
    }
    if members
        .iter()
        .any(|member| member.resident == Resident::Downloads)
    {
        parts.push("the download folder".to_string());
    }
    if members
        .iter()
        .any(|member| member.resident == Resident::AppData)
    {
        parts.push("Librarian's own data".to_string());
    }
    let parts: Vec<&str> = parts.iter().map(String::as_str).collect();
    join_and(&parts)
}

fn join_and(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The notification for a low volume: title and message.
pub fn alert_text(
    level: Level,
    label: &str,
    available: u64,
    total: u64,
    members: &[Location],
) -> (String, String) {
    let title = match level {
        Level::Critical => format!("Disk space is critically low on {label}"),
        _ => format!("Disk space is low on {label}"),
    };
    let percent = if total == 0 {
        0.0
    } else {
        available as f64 * 100.0 / total as f64
    };
    let on_it = residents_text(members);
    let mut message = format!(
        "{} free of {} ({percent:.1}%).",
        human_bytes(available),
        human_bytes(total)
    );
    if !on_it.is_empty() {
        message.push_str(&format!(" It holds {on_it}."));
    }
    message.push_str(match level {
        Level::Critical if members.iter().any(|m| m.resident == Resident::Downloads) => {
            " Downloads that would not fit are refused until space is freed."
        }
        Level::Critical => " Free some space soon: imports and moves may fail.",
        _ => " Free some space or move a library to a bigger disk.",
    });
    (title, message)
}

/// Why a release of `size` bytes may not be grabbed onto a download volume at `level` with
/// `available` bytes, or `None` when it may. Only a critical volume refuses, and only what would
/// not fit; a release of unknown size is let through.
pub fn grab_refusal(level: Level, available: u64, size: Option<i64>) -> Option<String> {
    let size = u64::try_from(size?).ok()?;
    (level == Level::Critical && size >= available).then(|| {
        format!(
            "Not enough disk space: the download folder has {} free and this release is {}. Free some space, then grab it again.",
            human_bytes(available),
            human_bytes(size)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TB: u64 = 1_000 * GB;

    fn library(name: &str) -> Location {
        Location {
            resident: Resident::Library {
                id: format!("id-{name}"),
                name: name.to_string(),
                owner_id: "owner".to_string(),
            },
            path: format!("smb://10.0.1.190/media/{name}"),
        }
    }

    #[test]
    fn the_larger_of_the_share_and_the_size_is_the_threshold() {
        let thresholds = Thresholds::default();
        // 8 TB: 10 % is 800 GB, more than 50 GB.
        assert_eq!(thresholds.floor(Level::Warning, 8 * TB), 800 * GB);
        assert_eq!(thresholds.floor(Level::Critical, 8 * TB), 400 * GB);
        // 200 GB: 10 % is 20 GB, less than 50 GB.
        assert_eq!(thresholds.floor(Level::Warning, 200 * GB), 50 * GB);
        assert_eq!(thresholds.floor(Level::Critical, 200 * GB), 20 * GB);
        assert_eq!(thresholds.floor(Level::Normal, 8 * TB), 0);
    }

    #[test]
    fn levels_follow_the_thresholds() {
        let t = Thresholds::default();
        let total = TB; // warning below 100 GB, critical below 50 GB
        assert_eq!(level_for(Level::Normal, 500 * GB, total, &t), Level::Normal);
        assert_eq!(level_for(Level::Normal, 99 * GB, total, &t), Level::Warning);
        assert_eq!(
            level_for(Level::Normal, 49 * GB, total, &t),
            Level::Critical
        );
        assert_eq!(
            level_for(Level::Warning, 10 * GB, total, &t),
            Level::Critical
        );
    }

    #[test]
    fn a_level_is_left_only_clearly_above_its_threshold() {
        let t = Thresholds::default();
        let total = TB; // warning 100 GB (+10 GB margin), critical 50 GB (+5 GB margin)
        // Just back above critical: still critical.
        assert_eq!(
            level_for(Level::Critical, 52 * GB, total, &t),
            Level::Critical
        );
        assert_eq!(
            level_for(Level::Critical, 56 * GB, total, &t),
            Level::Warning
        );
        // Just back above warning: still warning, from either level.
        assert_eq!(
            level_for(Level::Warning, 105 * GB, total, &t),
            Level::Warning
        );
        assert_eq!(
            level_for(Level::Critical, 105 * GB, total, &t),
            Level::Warning
        );
        assert_eq!(
            level_for(Level::Warning, 111 * GB, total, &t),
            Level::Normal
        );
        // Without a level to leave, the threshold itself decides.
        assert_eq!(level_for(Level::Normal, 101 * GB, total, &t), Level::Normal);
    }

    #[test]
    fn a_volume_on_the_line_does_not_flap() {
        let t = Thresholds::default();
        let total = TB;
        let mut level = Level::Normal;
        let mut changes = 0;
        for available in [99, 101, 99, 102, 100, 104, 98, 103] {
            let next = level_for(level, available * GB, total, &t);
            if next != level {
                changes += 1;
            }
            level = next;
        }
        assert_eq!(changes, 1, "only the first crossing changes the level");
        assert_eq!(level, Level::Warning);
    }

    #[test]
    fn settings_override_the_defaults_and_critical_is_never_looser() {
        let stored = |key: &str| match key {
            WARNING_FRACTION_KEY => Some("0.2".to_string()),
            WARNING_BYTES_KEY => Some("\"100000000000\"".to_string()),
            CRITICAL_FRACTION_KEY => Some("0.5".to_string()),
            CRITICAL_BYTES_KEY => Some("nonsense".to_string()),
            _ => None,
        };
        let t = Thresholds::from_settings(stored);
        assert_eq!(t.warning_fraction, 0.2);
        assert_eq!(t.warning_bytes, 100 * GB);
        assert_eq!(t.critical_fraction, 0.2, "capped at the warning share");
        assert_eq!(t.critical_bytes, 20 * GB, "the default replaces nonsense");
        assert_eq!(Thresholds::from_settings(|_| None), Thresholds::default());
    }

    #[test]
    fn small_moves_are_not_written_but_level_changes_and_old_readings_are() {
        let now = Utc::now();
        let stored = StoredReading {
            available: Some(500 * GB),
            total: Some(TB),
            level: Level::Normal,
            reachable: true,
            checked_at: Some(now - Duration::minutes(15)),
        };
        let moved = |available: u64| StoredReading {
            available: Some(available),
            checked_at: Some(now),
            ..stored.clone()
        };
        // 0.5 % of 1 TB is 5 GB.
        assert!(!worth_writing(&stored, &moved(504 * GB), now));
        assert!(worth_writing(&stored, &moved(506 * GB), now));
        let leveled = StoredReading {
            level: Level::Warning,
            ..moved(500 * GB)
        };
        assert!(worth_writing(&stored, &leveled, now));
        let old = StoredReading {
            checked_at: Some(now - Duration::hours(25)),
            ..stored.clone()
        };
        assert!(worth_writing(&old, &moved(500 * GB), now));
        // Unreachable: written once, then left alone.
        let gone = StoredReading {
            reachable: false,
            ..stored.clone()
        };
        assert!(worth_writing(&stored, &gone, now));
        assert!(!worth_writing(&gone, &gone, now));
    }

    #[test]
    fn libraries_on_one_share_are_measured_once() {
        let share = "smb://10.0.1.190/media".to_string();
        let groups = group_by_volume(vec![
            (share.clone(), library("TV")),
            (share.clone(), library("Movies")),
            (
                "local:2049".to_string(),
                Location {
                    resident: Resident::AppData,
                    path: "/var/lib/librarian".to_string(),
                },
            ),
            (
                share.clone(),
                Location {
                    resident: Resident::Downloads,
                    path: "smb://10.0.1.190/media/downloads".to_string(),
                },
            ),
            (share.clone(), library("Music")),
            (share.clone(), library("Movies")),
        ]);
        assert_eq!(groups.len(), 2);
        let on_share: Vec<String> = groups[&share]
            .iter()
            .map(|member| match &member.resident {
                Resident::Library { name, .. } => name.clone(),
                other => format!("{other:?}"),
            })
            .collect();
        assert_eq!(on_share, ["Movies", "Music", "TV", "Downloads"]);
        assert_eq!(volume_label(&share, &groups[&share]), "media on 10.0.1.190");
        assert_eq!(
            volume_label("local:2049", &groups["local:2049"]),
            "/var/lib/librarian"
        );
    }

    #[test]
    fn the_alert_names_what_lives_on_the_volume() {
        let members = vec![
            library("Movies"),
            library("TV"),
            Location {
                resident: Resident::Downloads,
                path: String::new(),
            },
        ];
        let (title, message) = alert_text(
            Level::Warning,
            "media on 10.0.1.190",
            48 * GB,
            8 * TB,
            &members,
        );
        assert_eq!(title, "Disk space is low on media on 10.0.1.190");
        assert!(
            message.starts_with("48 GB free of 8 TB (0.6%)."),
            "{message}"
        );
        assert!(
            message.contains("the Movies and TV libraries and the download folder"),
            "{message}"
        );
        let (title, message) = alert_text(Level::Critical, "x", GB, 8 * TB, &members);
        assert!(title.contains("critically low"));
        assert!(message.contains("would not fit are refused"));
        assert_eq!(
            residents_text(&[library("Music")]),
            "the Music library".to_string()
        );
    }

    #[test]
    fn sizes_read_as_disks_are_sold() {
        assert_eq!(human_bytes(1_200 * GB), "1.2 TB");
        assert_eq!(human_bytes(8 * TB), "8 TB");
        assert_eq!(human_bytes(48 * GB), "48 GB");
        assert_eq!(human_bytes(512), "512 B");
    }

    #[test]
    fn a_critical_download_volume_refuses_only_what_would_not_fit() {
        let size = Some((30 * GB) as i64);
        assert!(grab_refusal(Level::Critical, 10 * GB, size).is_some());
        assert!(grab_refusal(Level::Critical, 40 * GB, size).is_none());
        assert!(grab_refusal(Level::Warning, 10 * GB, size).is_none());
        assert!(grab_refusal(Level::Critical, 10 * GB, None).is_none());
        let reason = grab_refusal(Level::Critical, 10 * GB, size).unwrap();
        assert!(reason.contains("10 GB free"), "{reason}");
        assert!(reason.contains("30 GB"), "{reason}");
    }
}
