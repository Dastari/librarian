//! Application configuration management

use std::env;

use anyhow::{Context, Result};

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

    /// Session/state directory path (for DHT, resume data)
    pub session_path: String,

    /// Enable DHT for torrent discovery
    pub torrent_enable_dht: bool,

    /// Listen port for incoming torrent connections (0 = random)
    pub torrent_listen_port: u16,

    /// Maximum concurrent torrent downloads
    pub torrent_max_concurrent: usize,

    /// Run mode (server/tray/service)
    pub run_mode: RunMode,

    /// Auto-start tray on login (Windows)
    pub tray_autostart: bool,
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

            session_path: env::var("SESSION_PATH").unwrap_or_else(|_| "./data/session".to_string()),

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

            run_mode: RunMode::from_env(),

            tray_autostart: env::var("TRAY_AUTOSTART")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        })
    }
}
