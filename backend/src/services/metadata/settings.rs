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
}

impl MetadataSettings {
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

        Ok(Self {
            tmdb_api_key,
            tvdb_api_key,
            preferred_language,
            auto_fetch,
            musicbrainz_user_agent,
        })
    }
}
