//! Source definitions and implementations
//!
//! This module contains native Rust implementations of content sources
//! (torrent indexers, usenet indexers, RSS feeds).
//!
//! # Adding a new source definition
//!
//! 1. Create a new file in this directory (e.g., `mynewsource.rs`)
//! 2. Implement the `Source` trait for your source
//! 3. Add it to the `AVAILABLE_DEFINITIONS` list
//! 4. Register it in the `SourcesManager`

pub mod iptorrents;
pub mod limetorrents;
pub mod thepiratebay;
#[path = "1337x.rs"]
pub mod x1337;
pub mod yts;

use once_cell::sync::Lazy;

use super::SourceType;

/// Information about an available source definition
#[derive(Debug, Clone)]
pub struct SourceDefinitionInfo {
    /// Unique identifier for this source type (e.g., "iptorrents")
    pub id: &'static str,
    /// Display name
    pub name: &'static str,
    /// Description
    pub description: &'static str,
    /// The type of source
    pub source_type: SourceType,
    /// Type: "Private", "Public", "SemiPrivate"
    pub tracker_type: &'static str,
    /// Language code
    pub language: &'static str,
    /// Primary site URL
    pub site_link: &'static str,
    /// Required credential field names
    pub required_credentials: &'static [&'static str],
    /// Optional settings
    pub optional_settings: &'static [SettingDefinition],
}

/// Definition of a configurable setting
#[derive(Debug, Clone)]
pub struct SettingDefinition {
    pub key: &'static str,
    pub label: &'static str,
    pub setting_type: SettingType,
    pub default_value: Option<&'static str>,
    pub options: Option<&'static [(&'static str, &'static str)]>,
}

#[derive(Debug, Clone, Copy)]
pub enum SettingType {
    Text,
    Password,
    Checkbox,
    Select,
}

/// List of all available source definitions
pub static AVAILABLE_DEFINITIONS: Lazy<Vec<SourceDefinitionInfo>> = Lazy::new(|| {
    vec![
        SourceDefinitionInfo {
            id: "1337x",
            name: "1337x",
            description: "Popular public torrent indexer for general content.",
            source_type: SourceType::TorrentIndexer,
            tracker_type: "Public",
            language: "en-US",
            site_link: "https://1337x.to/",
            required_credentials: &[],
            optional_settings: &[],
        },
        SourceDefinitionInfo {
            id: "iptorrents",
            name: "IPTorrents",
            description: "IPTorrents is a Private site. Always a step ahead.",
            source_type: SourceType::TorrentIndexer,
            tracker_type: "Private",
            language: "en-US",
            site_link: "https://iptorrents.com/",
            required_credentials: &["Cookie", "UserAgent"],
            optional_settings: &[
                SettingDefinition {
                    key: "Freeleech",
                    label: "Search freeleech only",
                    setting_type: SettingType::Checkbox,
                    default_value: Some("false"),
                    options: None,
                },
                SettingDefinition {
                    key: "Sort",
                    label: "Sort requested from site",
                    setting_type: SettingType::Select,
                    default_value: Some("time"),
                    options: Some(&[
                        ("time", "Created"),
                        ("size", "Size"),
                        ("seeders", "Seeders"),
                        ("name", "Title"),
                    ]),
                },
            ],
        },
        SourceDefinitionInfo {
            id: "limetorrents",
            name: "LimeTorrents",
            description: "Public torrent indexer with broad general content.",
            source_type: SourceType::TorrentIndexer,
            tracker_type: "Public",
            language: "en-US",
            site_link: "https://www.limetorrents.lol/",
            required_credentials: &[],
            optional_settings: &[],
        },
        SourceDefinitionInfo {
            id: "thepiratebay",
            name: "The Pirate Bay",
            description: "Public BitTorrent indexer powered by The Pirate Bay.",
            source_type: SourceType::TorrentIndexer,
            tracker_type: "Public",
            language: "en-US",
            site_link: "https://thepiratebay.org/",
            required_credentials: &[],
            optional_settings: &[SettingDefinition {
                key: "Uploader",
                label: "Filter by uploader",
                setting_type: SettingType::Text,
                default_value: None,
                options: None,
            }],
        },
        SourceDefinitionInfo {
            id: "yts",
            name: "YTS",
            description: "Movie-focused public torrent indexer via YTS API.",
            source_type: SourceType::TorrentIndexer,
            tracker_type: "Public",
            language: "en-US",
            site_link: "https://yts.mx/",
            required_credentials: &[],
            optional_settings: &[],
        },
    ]
});

/// Get information about all available source definitions
pub fn get_available_definitions() -> &'static [SourceDefinitionInfo] {
    &AVAILABLE_DEFINITIONS
}

/// Get information about a specific source definition
pub fn get_definition_info(id: &str) -> Option<&'static SourceDefinitionInfo> {
    AVAILABLE_DEFINITIONS.iter().find(|d| d.id == id)
}
