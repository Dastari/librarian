//! Metadata settings loaded from the database (app_settings).

use anyhow::Result;

use crate::db::Database;
use crate::services::torrent::database::{get_setting, get_setting_string};

#[derive(Debug, Clone)]
pub struct MetadataSettings {
    pub tmdb_api_key: Option<String>,
    pub tvdb_api_key: Option<String>,
    pub preferred_language: Option<String>,
    pub auto_fetch: bool,
    pub musicbrainz_user_agent: Option<String>,
    pub tmdb_cache_days: i32,
    pub tvmaze_cache_days: i32,
    pub musicbrainz_cache_days: i32,
    pub openlibrary_cache_days: i32,
}

impl MetadataSettings {
    const DEFAULT_PROVIDER_CACHE_DAYS: i32 = 7;

    async fn load_cache_days(db: &Database, key: &str) -> i32 {
        match get_setting::<i32>(db, key).await {
            Ok(Some(days)) => days.clamp(0, 365),
            Ok(None) => Self::DEFAULT_PROVIDER_CACHE_DAYS,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    setting_key = key,
                    default_days = Self::DEFAULT_PROVIDER_CACHE_DAYS,
                    "Failed to parse metadata cache staleness setting; using default"
                );
                Self::DEFAULT_PROVIDER_CACHE_DAYS
            }
        }
    }

    pub async fn load(db: &Database) -> Result<Self> {
        let tmdb_api_key = get_setting_string(db, "metadata.tmdb_api_key").await?;
        let tvdb_api_key = get_setting_string(db, "metadata.tvdb_api_key").await?;
        // Use get_setting_string instead of get_setting::<String> to avoid JSON parse
        // failures on plain string values (e.g. "en" without quotes)
        let preferred_language = get_setting_string(db, "metadata.preferred_language").await?;
        let auto_fetch = match get_setting::<bool>(db, "metadata.auto_fetch").await {
            Ok(v) => v.unwrap_or(true),
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to parse app_settings key 'metadata.auto_fetch'; defaulting auto_fetch=true. error={}",
                    e
                );
                true
            }
        };
        let musicbrainz_user_agent =
            get_setting_string(db, "metadata.musicbrainz_user_agent").await?;
        let tmdb_cache_days = Self::load_cache_days(db, "metadata.cache_days.tmdb").await;
        let tvmaze_cache_days = Self::load_cache_days(db, "metadata.cache_days.tvmaze").await;
        let musicbrainz_cache_days =
            Self::load_cache_days(db, "metadata.cache_days.musicbrainz").await;
        let openlibrary_cache_days =
            Self::load_cache_days(db, "metadata.cache_days.openlibrary").await;

        Ok(Self {
            tmdb_api_key,
            tvdb_api_key,
            preferred_language,
            auto_fetch,
            musicbrainz_user_agent,
            tmdb_cache_days,
            tvmaze_cache_days,
            musicbrainz_cache_days,
            openlibrary_cache_days,
        })
    }
}
