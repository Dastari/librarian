//! The `auto_download.*` app settings, read once per pass.
//!
//! Every value lives in `app_settings` as JSON text and is re-read on each tick, so a change in
//! Settings › Downloads takes effect without a restart. `auto_download.enabled` is the only
//! master switch: there is deliberately no environment fallback, because the Settings screen
//! could not then show what the server is really doing.

use std::collections::HashMap;

use serde_json::Value;
use tracing::warn;

use crate::db::Database;

pub const SETTING_ENABLED: &str = "auto_download.enabled";
pub const SETTING_INTERVAL_MINUTES: &str = "auto_download.interval_minutes";
pub const SETTING_SEARCH_BACKOFF_MINUTES: &str = "auto_download.search_backoff_minutes";
pub const SETTING_AIR_GRACE_MINUTES: &str = "auto_download.air_grace_minutes";
pub const SETTING_AIR_WINDOW_DAYS: &str = "auto_download.air_window_days";
pub const SETTING_AIR_RETRY_MINUTES: &str = "auto_download.air_retry_minutes";
pub const SETTING_MAX_CONCURRENT_GRABS: &str = "auto_download.max_concurrent_grabs";
/// Written by the service after each pass so Settings can say when it last ran.
pub const SETTING_STATUS: &str = "auto_download.status";

const ALL_KEYS: &[&str] = &[
    SETTING_ENABLED,
    SETTING_INTERVAL_MINUTES,
    SETTING_SEARCH_BACKOFF_MINUTES,
    SETTING_AIR_GRACE_MINUTES,
    SETTING_AIR_WINDOW_DAYS,
    SETTING_AIR_RETRY_MINUTES,
    SETTING_MAX_CONCURRENT_GRABS,
];

/// What the scheduler is told to do. Defaults are what a fresh install uses.
#[derive(Debug, Clone, PartialEq)]
pub struct AutoDownloadSettings {
    /// Master switch. Nothing automatic runs while this is false (default).
    pub enabled: bool,
    /// How often the whole library is swept for anything wanted.
    pub interval_minutes: u64,
    /// A target that was searched is not searched again for this long.
    pub search_backoff_minutes: i64,
    /// An episode is searched this long after its air time, giving releases time to appear.
    pub air_grace_minutes: i64,
    /// For this many days after an episode airs it is retried on the quicker schedule.
    pub air_window_days: i64,
    /// How often a freshly aired episode that has not turned up yet is searched again.
    pub air_retry_minutes: i64,
    /// Automatic grabs that may be downloading at once; more wait for a slot.
    pub max_concurrent_grabs: usize,
}

impl Default for AutoDownloadSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_minutes: 60,
            search_backoff_minutes: 360,
            air_grace_minutes: 60,
            air_window_days: 7,
            air_retry_minutes: 60,
            max_concurrent_grabs: 5,
        }
    }
}

fn number(values: &HashMap<String, Value>, key: &str) -> Option<i64> {
    match values.get(key)? {
        Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|value| value as i64)),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

fn flag(values: &HashMap<String, Value>, key: &str) -> Option<bool> {
    match values.get(key)? {
        Value::Bool(value) => Some(*value),
        Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

impl AutoDownloadSettings {
    /// Build the settings from raw values, keeping the default for anything missing or
    /// unusable and clamping the rest to ranges the scheduler can honour.
    pub fn from_values(values: &HashMap<String, Value>) -> Self {
        let defaults = Self::default();
        Self {
            enabled: flag(values, SETTING_ENABLED).unwrap_or(defaults.enabled),
            interval_minutes: number(values, SETTING_INTERVAL_MINUTES)
                .map_or(defaults.interval_minutes, |value| {
                    value.clamp(5, 1440) as u64
                }),
            search_backoff_minutes: number(values, SETTING_SEARCH_BACKOFF_MINUTES)
                .map_or(defaults.search_backoff_minutes, |value| {
                    value.clamp(5, 10_080)
                }),
            air_grace_minutes: number(values, SETTING_AIR_GRACE_MINUTES)
                .map_or(defaults.air_grace_minutes, |value| value.clamp(0, 1440)),
            air_window_days: number(values, SETTING_AIR_WINDOW_DAYS)
                .map_or(defaults.air_window_days, |value| value.clamp(0, 60)),
            air_retry_minutes: number(values, SETTING_AIR_RETRY_MINUTES)
                .map_or(defaults.air_retry_minutes, |value| value.clamp(5, 1440)),
            max_concurrent_grabs: number(values, SETTING_MAX_CONCURRENT_GRABS)
                .map_or(defaults.max_concurrent_grabs, |value| {
                    value.clamp(1, 100) as usize
                }),
        }
    }

    /// Read every setting. A value that cannot be read keeps its default, and the master
    /// switch then defaults to off, so a database hiccup can never turn downloading on.
    pub async fn load(db: &Database) -> Self {
        let mut values = HashMap::new();
        for key in ALL_KEYS {
            match crate::services::torrent::get_setting::<Value>(db, key).await {
                Ok(Some(value)) => {
                    values.insert((*key).to_string(), value);
                }
                Ok(None) => {}
                Err(error) => warn!(
                    setting = key,
                    error = %error,
                    "Auto-download: could not read a setting; using its default"
                ),
            }
        }
        Self::from_values(&values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn values(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect()
    }

    #[test]
    fn a_fresh_install_is_off_with_the_documented_defaults() {
        let settings = AutoDownloadSettings::from_values(&HashMap::new());
        assert!(!settings.enabled);
        assert_eq!(settings, AutoDownloadSettings::default());
        assert_eq!(settings.interval_minutes, 60);
        assert_eq!(settings.search_backoff_minutes, 360);
        assert_eq!(settings.air_grace_minutes, 60);
        assert_eq!(settings.air_window_days, 7);
    }

    #[test]
    fn only_an_explicit_true_turns_it_on() {
        let on = AutoDownloadSettings::from_values(&values(&[(SETTING_ENABLED, json!(true))]));
        assert!(on.enabled);
        let text = AutoDownloadSettings::from_values(&values(&[(SETTING_ENABLED, json!("on"))]));
        assert!(text.enabled);
        let junk =
            AutoDownloadSettings::from_values(&values(&[(SETTING_ENABLED, json!("sometimes"))]));
        assert!(!junk.enabled);
        let wrong_type =
            AutoDownloadSettings::from_values(&values(&[(SETTING_ENABLED, json!(12))]));
        assert!(!wrong_type.enabled);
    }

    #[test]
    fn values_are_clamped_to_what_the_scheduler_can_honour() {
        let settings = AutoDownloadSettings::from_values(&values(&[
            (SETTING_INTERVAL_MINUTES, json!(1)),
            (SETTING_SEARCH_BACKOFF_MINUTES, json!(99_999_999)),
            (SETTING_AIR_GRACE_MINUTES, json!(-30)),
            (SETTING_AIR_WINDOW_DAYS, json!(400)),
            (SETTING_AIR_RETRY_MINUTES, json!(0)),
            (SETTING_MAX_CONCURRENT_GRABS, json!(0)),
        ]));
        assert_eq!(settings.interval_minutes, 5);
        assert_eq!(settings.search_backoff_minutes, 10_080);
        assert_eq!(settings.air_grace_minutes, 0);
        assert_eq!(settings.air_window_days, 60);
        assert_eq!(settings.air_retry_minutes, 5);
        assert_eq!(settings.max_concurrent_grabs, 1);
    }
}
