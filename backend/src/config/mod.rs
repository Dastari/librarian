//! Application configuration management

use std::env;

use anyhow::{Context, Result};
use ipnet::IpNet;

use crate::app_mode::RunMode;

/// Application configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    /// Server host (for generating URLs)
    pub host: Option<String>,

    /// Server port
    pub port: u16,

    /// Database URL or path (SQLite)
    /// For SQLite: use DATABASE_PATH or DATABASE_URL with sqlite:// prefix
    pub database_url: String,

    /// JWT secret (legacy: not used for auth; auth loads from database auth_secrets table).
    pub jwt_secret: String,

    /// TheTVDB API key
    pub tvdb_api_key: Option<String>,

    /// TMDB API key
    pub tmdb_api_key: Option<String>,

    /// Media library root path
    pub media_path: String,

    /// Downloads directory path
    pub downloads_path: String,

    /// Transcode cache directory path
    pub cache_path: String,

    /// Path/name of the `ffmpeg` binary used for on-demand HLS
    /// remux/transcode. Defaults to the bare `"ffmpeg"` on `$PATH` when unset
    /// (mirrors `cache_path`'s always-defaulted shape); set `FFMPEG_PATH` to
    /// point at a bundled binary (e.g. Windows distribution).
    pub ffmpeg_path: Option<String>,

    /// Session/state directory path (for DHT, resume data)
    pub session_path: String,

    /// Object storage backend identifier.
    pub storage_backend: String,

    /// Local object storage path.
    pub storage_path: String,

    /// Local backup repository path.
    pub backup_path: String,

    /// Enable DHT for torrent discovery
    pub torrent_enable_dht: bool,

    /// Listen port for incoming torrent connections (0 = random)
    pub torrent_listen_port: u16,

    /// Maximum concurrent torrent downloads
    pub torrent_max_concurrent: usize,

    /// Automatically scan for cast devices in the background
    pub cast_auto_discovery: bool,

    /// Seconds between background cast discovery scans
    pub cast_discovery_interval_secs: u64,

    /// Per-scan cast discovery timeout in milliseconds
    pub cast_discovery_timeout_ms: u64,

    /// Days to retain stale automatically discovered, non-favorite devices.
    pub cast_discovery_retention_days: u64,

    /// Explicit receiver-reachable base URL used in Cast media URLs.
    pub advertised_media_url: Option<String>,

    /// Run mode (server/tray/service)
    pub run_mode: RunMode,

    /// Auto-start tray on login (Windows)
    pub tray_autostart: bool,

    /// Allowed CORS origins (from comma-separated `LIBRARIAN_CORS_ORIGINS`).
    /// Only relevant to cross-origin callers such as the Vite dev server —
    /// the production frontend is served same-origin from the embedded
    /// binary and doesn't need CORS at all. Defaults to the dev frontend
    /// origins when unset.
    pub cors_origins: Vec<String>,

    /// Mark server-managed auth cookies `Secure`. Enable this when the public
    /// application URL is HTTPS, including deployments terminated by a reverse proxy.
    /// Forwarded headers are intentionally not trusted to derive this value.
    pub secure_cookies: bool,

    /// Direct peer networks that are allowed to supply `Forwarded` or
    /// `X-Forwarded-Proto`. Empty by default: forwarded headers are ignored.
    pub trusted_proxies: Vec<IpNet>,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // For SQLite, prefer DATABASE_PATH, fall back to DATABASE_URL
        let mut database_url = env::var("DATABASE_PATH")
            .or_else(|_| env::var("DATABASE_URL"))
            .unwrap_or_else(|_| "./data/librarian.db".to_string());
        if !database_url.starts_with("sqlite:") {
            database_url = format!("sqlite://{}", database_url);
        }

        // JWT secret is loaded from database at runtime; env value is legacy/unused for auth.
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_default();

        let advertised_media_url = env::var("LIBRARIAN_ADVERTISED_MEDIA_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|value| validate_advertised_media_url(&value))
            .transpose()?;

        Ok(Self {
            host: env::var("HOST").ok(),

            port: env::var("PORT")
                .unwrap_or_else(|_| "3001".to_string())
                .parse()
                .context("Invalid PORT")?,

            database_url,

            jwt_secret,

            tvdb_api_key: env::var("TVDB_API_KEY").ok(),

            tmdb_api_key: env::var("TMDB_API_KEY").ok(),

            media_path: env::var("MEDIA_PATH").unwrap_or_else(|_| "./data/media".to_string()),

            downloads_path: env::var("DOWNLOADS_PATH")
                .unwrap_or_else(|_| "./data/downloads".to_string()),

            cache_path: env::var("CACHE_PATH").unwrap_or_else(|_| "./data/cache".to_string()),

            ffmpeg_path: env::var("FFMPEG_PATH").ok(),

            session_path: env::var("SESSION_PATH").unwrap_or_else(|_| "./data/session".to_string()),

            storage_backend: env::var("STORAGE_BACKEND").unwrap_or_else(|_| "local".to_string()),

            storage_path: env::var("STORAGE_PATH").unwrap_or_else(|_| "./data/storage".to_string()),

            backup_path: env::var("BACKUP_PATH").unwrap_or_else(|_| "./data/backups".to_string()),

            torrent_enable_dht: env::var("TORRENT_ENABLE_DHT")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),

            torrent_listen_port: env::var("TORRENT_LISTEN_PORT")
                .unwrap_or_else(|_| "0".to_string())
                .parse()
                .unwrap_or(0),

            torrent_max_concurrent: env::var("TORRENT_MAX_CONCURRENT")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .unwrap_or(5),

            cast_auto_discovery: env_bool("CAST_AUTO_DISCOVERY", true),

            cast_discovery_interval_secs: env::var("CAST_DISCOVERY_INTERVAL_SECONDS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),

            cast_discovery_timeout_ms: env::var("CAST_DISCOVERY_TIMEOUT_MS")
                .unwrap_or_else(|_| "1500".to_string())
                .parse()
                .unwrap_or(1500),

            cast_discovery_retention_days: env::var("CAST_DISCOVERY_RETENTION_DAYS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .context("Invalid CAST_DISCOVERY_RETENTION_DAYS")?,

            advertised_media_url,

            run_mode: RunMode::from_env(),

            tray_autostart: env::var("TRAY_AUTOSTART")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),

            cors_origins: env::var("LIBRARIAN_CORS_ORIGINS")
                .ok()
                .map(|value| {
                    value
                        .split(',')
                        .map(|origin| origin.trim().to_string())
                        .filter(|origin| !origin.is_empty())
                        .collect::<Vec<_>>()
                })
                .filter(|origins| !origins.is_empty())
                .unwrap_or_else(default_dev_cors_origins),

            secure_cookies: env_bool("LIBRARIAN_SECURE_COOKIES", false),

            trusted_proxies: parse_trusted_proxies(
                env::var("LIBRARIAN_TRUSTED_PROXIES").ok().as_deref(),
            )?,
        })
    }
}

fn validate_advertised_media_url(value: &str) -> Result<String> {
    let mut url = url::Url::parse(value.trim())
        .context("LIBRARIAN_ADVERTISED_MEDIA_URL must be an absolute URL")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        anyhow::bail!(
            "LIBRARIAN_ADVERTISED_MEDIA_URL must be an http(s) origin without credentials, query, or fragment"
        );
    }
    let normalized_path = url.path().trim_end_matches('/').to_owned();
    url.set_path(&normalized_path);
    Ok(url.to_string().trim_end_matches('/').to_string())
}

fn parse_trusted_proxies(value: Option<&str>) -> Result<Vec<IpNet>> {
    value
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            entry
                .parse::<IpNet>()
                .or_else(|_| entry.parse::<std::net::IpAddr>().map(IpNet::from))
                .with_context(|| format!("Invalid IP/CIDR in LIBRARIAN_TRUSTED_PROXIES: {entry}"))
        })
        .collect()
}

/// Default CORS allowlist used when `LIBRARIAN_CORS_ORIGINS` is unset: the
/// Vite dev servers on port 3000 (`frontend/`) and 3002 (`web/`).
fn default_dev_cors_origins() -> Vec<String> {
    vec![
        "http://localhost:3000".to_string(),
        "http://127.0.0.1:3000".to_string(),
        "http://localhost:3002".to_string(),
        "http://127.0.0.1:3002".to_string(),
    ]
}

fn env_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "true" | "1" | "yes" | "on"
            )
        })
        .unwrap_or(default)
}

/// Whether the server is running in development mode: either a debug build
/// (`cargo build` without `--release`) or `LIBRARIAN_DEV=1` is set (useful to
/// opt a release build into dev-only behavior, e.g. local testing of a
/// release binary). Used to gate GraphQL introspection/GraphiQL.
pub fn dev_mode() -> bool {
    cfg!(debug_assertions) || env_bool("LIBRARIAN_DEV", false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusted_proxy_entries_require_valid_ip_or_cidr() {
        let entries = parse_trusted_proxies(Some("127.0.0.1, 10.0.0.0/8")).unwrap();
        assert_eq!(entries.len(), 2);
        let address: std::net::IpAddr = "10.5.4.3".parse().unwrap();
        assert!(entries[1].contains(&address));
        assert!(parse_trusted_proxies(Some("not-a-network")).is_err());
    }
}
