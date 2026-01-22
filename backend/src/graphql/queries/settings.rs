use super::prelude::*;

#[derive(Default)]
pub struct SettingsQueries;

#[Object]
impl SettingsQueries {
    /// Get torrent client settings
    async fn torrent_settings(&self, ctx: &Context<'_>) -> Result<TorrentSettings> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        Ok(TorrentSettings {
            download_dir: settings
                .get_or_default("torrent.download_dir", "/data/downloads".to_string())
                .await
                .unwrap_or_default(),
            session_dir: settings
                .get_or_default("torrent.session_dir", "/data/session".to_string())
                .await
                .unwrap_or_default(),
            enable_dht: settings
                .get_or_default("torrent.enable_dht", true)
                .await
                .unwrap_or(true),
            listen_port: settings
                .get_or_default("torrent.listen_port", 6881)
                .await
                .unwrap_or(6881),
            max_concurrent: settings
                .get_or_default("torrent.max_concurrent", 5)
                .await
                .unwrap_or(5),
            upload_limit: settings
                .get_or_default("torrent.upload_limit", 0i64)
                .await
                .unwrap_or(0),
            download_limit: settings
                .get_or_default("torrent.download_limit", 0i64)
                .await
                .unwrap_or(0),
        })
    }

    /// Get all settings in a category
    async fn settings_by_category(
        &self,
        ctx: &Context<'_>,
        category: String,
    ) -> Result<Vec<AppSetting>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        let records = settings
            .list_by_category(&category)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| AppSetting {
                key: r.key,
                value: r.value,
                description: r.description,
                category: r.category,
            })
            .collect())
    }

    /// Get LLM parser settings
    async fn llm_parser_settings(&self, ctx: &Context<'_>) -> Result<LlmParserSettings> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        // Helper to get optional string (returns None if value is JSON null, "null", or empty)
        async fn get_optional_string(
            settings: &crate::db::SettingsRepository,
            key: &str,
        ) -> Result<Option<String>, async_graphql::Error> {
            // Get the raw setting record to check if value is JSON null
            let record = settings
                .get(key)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;

            match record {
                Some(r) => {
                    // Check if the JSON value is null
                    if r.value.is_null() {
                        return Ok(None);
                    }
                    // Try to get as string
                    match r.value.as_str() {
                        Some(s) if s != "null" && !s.is_empty() => Ok(Some(s.to_string())),
                        _ => Ok(None),
                    }
                }
                None => Ok(None),
            }
        }

        Ok(LlmParserSettings {
            enabled: settings
                .get_or_default("llm.enabled", false)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            ollama_url: settings
                .get_or_default("llm.ollama_url", "http://localhost:11434".to_string())
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            ollama_model: settings
                .get_or_default("llm.ollama_model", "qwen2.5-coder:7b".to_string())
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            timeout_seconds: settings
                .get_or_default("llm.timeout_seconds", 30)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            temperature: settings
                .get_or_default("llm.temperature", 0.1)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            max_tokens: settings
                .get_or_default("llm.max_tokens", 256)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            prompt_template: settings
                .get_or_default(
                    "llm.prompt_template",
                    r#"Parse this media filename. Fill ALL fields. Use null if not found.
Clean the title (remove dots/underscores). Release group is after final hyphen.
Set type to "movie" or "tv" based on whether season/episode are present.

Filename: {filename}

{"type":null,"title":null,"year":null,"season":null,"episode":null,"resolution":null,"source":null,"video_codec":null,"audio":null,"hdr":null,"release_group":null,"edition":null}"#
                        .to_string(),
                )
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            confidence_threshold: settings
                .get_or_default("llm.confidence_threshold", 0.7)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            // Library-type-specific models
            model_movies: get_optional_string(&settings, "llm.model.movies").await?,
            model_tv: get_optional_string(&settings, "llm.model.tv").await?,
            model_music: get_optional_string(&settings, "llm.model.music").await?,
            model_audiobooks: get_optional_string(&settings, "llm.model.audiobooks").await?,
            // Library-type-specific prompts
            prompt_movies: get_optional_string(&settings, "llm.prompt.movies").await?,
            prompt_tv: get_optional_string(&settings, "llm.prompt.tv").await?,
            prompt_music: get_optional_string(&settings, "llm.prompt.music").await?,
            prompt_audiobooks: get_optional_string(&settings, "llm.prompt.audiobooks").await?,
        })
    }

    /// Get all naming pattern presets
    async fn naming_patterns(&self, ctx: &Context<'_>) -> Result<Vec<NamingPattern>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let records = db
            .naming_patterns()
            .list_all()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(NamingPattern::from_record)
            .collect())
    }

    /// Get a specific naming pattern by ID
    async fn naming_pattern(&self, ctx: &Context<'_>, id: String) -> Result<Option<NamingPattern>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let pattern_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid pattern ID: {}", e)))?;

        let record = db
            .naming_patterns()
            .get_by_id(pattern_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(record.map(NamingPattern::from_record))
    }
}
