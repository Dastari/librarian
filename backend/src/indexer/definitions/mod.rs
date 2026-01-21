//! Indexer definitions and implementations
//!
//! This module contains native Rust implementations of torrent indexers
//! as well as the Cardigann YAML-based definition engine.
//!
//! # Adding a new indexer
//!
//! 1. Create a new file in this directory (e.g., `myindexer.rs`)
//! 2. Implement the `Indexer` trait for your indexer
//! 3. Add it to the `AVAILABLE_INDEXERS` list
//! 4. Register it in the `IndexerManager`

pub mod cardigann;
pub mod iptorrents;
pub mod newznab;

use once_cell::sync::Lazy;

/// Information about an available indexer type
#[derive(Debug, Clone)]
pub struct IndexerTypeInfo {
    /// Unique identifier for this indexer type (e.g., "iptorrents")
    pub id: &'static str,
    /// Display name
    pub name: &'static str,
    /// Description
    pub description: &'static str,
    /// Type: "private", "public", "semi-private"
    pub tracker_type: &'static str,
    /// Language code
    pub language: &'static str,
    /// Primary site URL
    pub site_link: &'static str,
    /// Required credential types
    pub required_credentials: &'static [&'static str],
    /// Optional settings
    pub optional_settings: &'static [SettingDefinition],
    /// Whether this is a native implementation
    pub is_native: bool,
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

/// List of all available native indexer types
pub static AVAILABLE_INDEXERS: Lazy<Vec<IndexerTypeInfo>> = Lazy::new(|| {
    vec![
        IndexerTypeInfo {
            id: "iptorrents",
            name: "IPTorrents",
            description: "IPTorrents is a Private site. Always a step ahead.",
            tracker_type: "private",
            language: "en-US",
            site_link: "https://iptorrents.com/",
            required_credentials: &["cookie", "user_agent"],
            optional_settings: &[
                SettingDefinition {
                    key: "freeleech",
                    label: "Search freeleech only",
                    setting_type: SettingType::Checkbox,
                    default_value: Some("false"),
                    options: None,
                },
                SettingDefinition {
                    key: "sort",
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
            is_native: true,
        },
        IndexerTypeInfo {
            id: "newznab",
            name: "Newznab",
            description: "Generic Newznab-compatible Usenet indexer (NZBGeek, DrunkenSlug, etc.)",
            tracker_type: "private",
            language: "en-US",
            site_link: "",  // User must provide API URL
            required_credentials: &["api_key"],
            optional_settings: &[
                SettingDefinition {
                    key: "vip_expiry_check",
                    label: "Check VIP status expiry",
                    setting_type: SettingType::Checkbox,
                    default_value: Some("false"),
                    options: None,
                },
            ],
            is_native: true,
        },
    ]
});

/// Get information about all available indexer types
pub fn get_available_indexers() -> &'static [IndexerTypeInfo] {
    &AVAILABLE_INDEXERS
}

/// Get information about a specific indexer type
pub fn get_indexer_info(id: &str) -> Option<&'static IndexerTypeInfo> {
    AVAILABLE_INDEXERS.iter().find(|i| i.id == id)
}

/// Credential types used by indexers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialType {
    /// Session cookie
    Cookie,
    /// Browser user agent
    UserAgent,
    /// API key
    ApiKey,
    /// Username
    Username,
    /// Password
    Password,
    /// Passkey (for RSS feeds)
    Passkey,
    /// 2FA token
    TwoFactorToken,
}

impl std::fmt::Display for CredentialType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialType::Cookie => write!(f, "cookie"),
            CredentialType::UserAgent => write!(f, "user_agent"),
            CredentialType::ApiKey => write!(f, "api_key"),
            CredentialType::Username => write!(f, "username"),
            CredentialType::Password => write!(f, "password"),
            CredentialType::Passkey => write!(f, "passkey"),
            CredentialType::TwoFactorToken => write!(f, "2fa_token"),
        }
    }
}

impl std::str::FromStr for CredentialType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cookie" => Ok(CredentialType::Cookie),
            "user_agent" | "useragent" => Ok(CredentialType::UserAgent),
            "api_key" | "apikey" => Ok(CredentialType::ApiKey),
            "username" => Ok(CredentialType::Username),
            "password" => Ok(CredentialType::Password),
            "passkey" => Ok(CredentialType::Passkey),
            "2fa_token" | "2fa" | "twofa" => Ok(CredentialType::TwoFactorToken),
            _ => Err(anyhow::anyhow!("Unknown credential type: {}", s)),
        }
    }
}
