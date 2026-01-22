//! Application configuration management

use std::env;

use anyhow::{Context, Result};

/// Application configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    /// Server host (for generating URLs)
    pub host: Option<String>,

    /// Server port
    pub port: u16,

    /// Database URL (PostgreSQL) or path (SQLite)
    /// For SQLite: use DATABASE_PATH or DATABASE_URL with sqlite:// prefix
    pub database_url: String,

    /// Supabase API URL (only required for postgres feature)
    pub supabase_url: Option<String>,

    /// Supabase anonymous key (only required for postgres feature)
    pub supabase_anon_key: Option<String>,

    /// Supabase service role key (only required for postgres feature)
    pub supabase_service_key: Option<String>,

    /// JWT secret for token verification
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
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // For SQLite, prefer DATABASE_PATH, fall back to DATABASE_URL
        #[cfg(feature = "sqlite")]
        let database_url = env::var("DATABASE_PATH")
            .or_else(|_| env::var("DATABASE_URL"))
            .unwrap_or_else(|_| "./data/librarian.db".to_string());

        #[cfg(feature = "postgres")]
        let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;

        // Supabase config is only required for postgres feature
        #[cfg(feature = "postgres")]
        let (supabase_url, supabase_anon_key, supabase_service_key) = (
            Some(env::var("SUPABASE_URL").context("SUPABASE_URL is required")?),
            Some(env::var("SUPABASE_ANON_KEY").context("SUPABASE_ANON_KEY is required")?),
            Some(env::var("SUPABASE_SERVICE_KEY").context("SUPABASE_SERVICE_KEY is required")?),
        );

        #[cfg(not(feature = "postgres"))]
        let (supabase_url, supabase_anon_key, supabase_service_key) = (
            env::var("SUPABASE_URL").ok(),
            env::var("SUPABASE_ANON_KEY").ok(),
            env::var("SUPABASE_SERVICE_KEY").ok(),
        );

        // JWT_SECRET is always required - generate a random one if not provided in dev
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
            // In production, this should be set explicitly
            // For development, generate a random secret
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            std::time::SystemTime::now().hash(&mut hasher);
            format!("dev-secret-{}", hasher.finish())
        });

        Ok(Self {
            host: env::var("HOST").ok(),

            port: env::var("PORT")
                .unwrap_or_else(|_| "3001".to_string())
                .parse()
                .context("Invalid PORT")?,

            database_url,

            supabase_url,
            supabase_anon_key,
            supabase_service_key,

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
        })
    }
}
