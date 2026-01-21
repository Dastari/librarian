use super::prelude::*;

#[derive(Default)]
pub struct SettingsMutations;

#[Object]
impl SettingsMutations {
    /// Create a custom naming pattern
    async fn create_naming_pattern(
        &self,
        ctx: &Context<'_>,
        input: CreateNamingPatternInput,
    ) -> Result<NamingPatternResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let record = db
            .naming_patterns()
            .create(crate::db::CreateNamingPattern {
                name: input.name,
                pattern: input.pattern,
                description: input.description,
                library_type: input.library_type.unwrap_or_else(|| "tv".to_string()),
            })
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(NamingPatternResult {
            success: true,
            naming_pattern: Some(NamingPattern::from_record(record)),
            error: None,
        })
    }

    /// Delete a custom naming pattern (system patterns cannot be deleted)
    async fn delete_naming_pattern(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let pattern_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid pattern ID: {}", e)))?;

        let deleted = db
            .naming_patterns()
            .delete(pattern_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: deleted,
            error: if deleted {
                None
            } else {
                Some("Pattern not found or is a system pattern".to_string())
            },
        })
    }

    /// Set a naming pattern as the default
    async fn set_default_naming_pattern(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let pattern_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid pattern ID: {}", e)))?;

        let updated = db
            .naming_patterns()
            .set_default(pattern_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: updated,
            error: if updated {
                None
            } else {
                Some("Pattern not found".to_string())
            },
        })
    }

    /// Update a custom naming pattern (system patterns cannot be edited)
    async fn update_naming_pattern(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateNamingPatternInput,
    ) -> Result<NamingPatternResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let pattern_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid pattern ID: {}", e)))?;

        let record = db
            .naming_patterns()
            .update(
                pattern_id,
                crate::db::UpdateNamingPattern {
                    name: input.name,
                    pattern: input.pattern,
                    description: input.description,
                },
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        match record {
            Some(r) => Ok(NamingPatternResult {
                success: true,
                naming_pattern: Some(NamingPattern::from_record(r)),
                error: None,
            }),
            None => Ok(NamingPatternResult {
                success: false,
                naming_pattern: None,
                error: Some("Pattern not found or is a system pattern".to_string()),
            }),
        }
    }

    /// Update LLM parser settings
    async fn update_llm_parser_settings(
        &self,
        ctx: &Context<'_>,
        input: UpdateLlmParserSettingsInput,
    ) -> Result<SettingsResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        // Update each setting if provided
        if let Some(v) = input.enabled {
            settings
                .set_with_category("llm.enabled", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.ollama_url {
            settings
                .set_with_category("llm.ollama_url", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.ollama_model {
            settings
                .set_with_category("llm.ollama_model", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.timeout_seconds {
            settings
                .set_with_category("llm.timeout_seconds", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.temperature {
            settings
                .set_with_category("llm.temperature", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.max_tokens {
            settings
                .set_with_category("llm.max_tokens", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.prompt_template {
            settings
                .set_with_category("llm.prompt_template", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.confidence_threshold {
            settings
                .set_with_category("llm.confidence_threshold", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        // Library-type-specific models
        if let Some(v) = input.model_movies {
            settings
                .set_with_category("llm.model.movies", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.model_tv {
            settings
                .set_with_category("llm.model.tv", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.model_music {
            settings
                .set_with_category("llm.model.music", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.model_audiobooks {
            settings
                .set_with_category("llm.model.audiobooks", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        // Library-type-specific prompts
        if let Some(v) = input.prompt_movies {
            settings
                .set_with_category("llm.prompt.movies", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.prompt_tv {
            settings
                .set_with_category("llm.prompt.tv", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.prompt_music {
            settings
                .set_with_category("llm.prompt.music", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.prompt_audiobooks {
            settings
                .set_with_category("llm.prompt.audiobooks", v, "llm", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }

        Ok(SettingsResult {
            success: true,
            error: None,
        })
    }

    /// Test connection to Ollama server and list available models
    async fn test_ollama_connection(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Ollama server URL (defaults to configured URL)")] url: Option<String>,
    ) -> Result<OllamaConnectionResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        // Get URL from input or settings
        let ollama_url = match url {
            Some(u) => u,
            None => settings
                .get_or_default("llm.ollama_url", "http://localhost:11434".to_string())
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
        };

        let config = crate::services::OllamaConfig {
            url: ollama_url,
            ..Default::default()
        };
        let ollama = crate::services::OllamaService::new(config);

        match ollama.test_connection().await {
            Ok(models) => Ok(OllamaConnectionResult {
                success: true,
                available_models: models,
                error: None,
            }),
            Err(e) => Ok(OllamaConnectionResult {
                success: false,
                available_models: vec![],
                error: Some(e.to_string()),
            }),
        }
    }

    /// Test filename parsing with both regex and LLM parsers
    async fn test_filename_parser(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Filename to parse")] filename: String,
    ) -> Result<TestFilenameParserResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        // Run regex parser with timing
        let regex_start = std::time::Instant::now();
        let parsed_ep = crate::services::filename_parser::parse_episode(&filename);
        let parsed_quality = crate::services::filename_parser::parse_quality(&filename);
        let regex_time_ms = regex_start.elapsed().as_secs_f64() * 1000.0;

        // Determine media type based on whether season/episode info was found
        let media_type = if parsed_ep.season.is_some()
            || parsed_ep.episode.is_some()
            || parsed_ep.date.is_some()
        {
            Some("tv".to_string())
        } else {
            Some("movie".to_string())
        };

        let regex_result = FilenameParseResult {
            media_type,
            title: parsed_ep.show_name.clone(),
            year: parsed_ep.year.map(|y| y as i32),
            season: parsed_ep.season.map(|s| s as i32),
            episode: parsed_ep.episode.map(|e| e as i32),
            episode_end: None, // Not supported by current parser
            resolution: parsed_quality.resolution.or(parsed_ep.resolution),
            source: parsed_quality.source.or(parsed_ep.source),
            video_codec: parsed_quality.codec.or(parsed_ep.codec),
            audio: parsed_quality.audio.or(parsed_ep.audio),
            hdr: parsed_quality.hdr.or(parsed_ep.hdr),
            release_group: parsed_ep.release_group,
            edition: None,          // Not supported by current parser
            complete_series: false, // Not supported by current parser
            confidence: 0.8,        // Default confidence for regex parser
        };

        // Check if LLM parsing is enabled
        let llm_enabled: bool = settings
            .get_or_default("llm.enabled", false)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let (llm_result, llm_time_ms, llm_error) = if llm_enabled {
            // Build LLM config from settings
            let config = crate::services::OllamaConfig {
                url: settings
                    .get_or_default("llm.ollama_url", "http://localhost:11434".to_string())
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?,
                model: settings
                    .get_or_default("llm.ollama_model", "qwen2.5-coder:7b".to_string())
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?,
                timeout_seconds: settings
                    .get_or_default::<i32>("llm.timeout_seconds", 30)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    as u64,
                temperature: settings
                    .get_or_default::<f64>("llm.temperature", 0.1)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    as f32,
                max_tokens: settings
                    .get_or_default::<i32>("llm.max_tokens", 256)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    as u32,
                prompt_template: settings
                    .get_or_default(
                        "llm.prompt_template",
                        "Parse this media filename: {filename}".to_string(),
                    )
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            };

            let ollama = crate::services::OllamaService::new(config);
            let llm_start = std::time::Instant::now();

            match ollama.parse_filename(&filename).await {
                Ok(parsed) => {
                    let time_ms = llm_start.elapsed().as_secs_f64() * 1000.0;
                    let result = FilenameParseResult {
                        media_type: parsed.media_type,
                        title: parsed.title,
                        year: parsed.year,
                        season: parsed.season,
                        episode: parsed.episode,
                        episode_end: parsed.episode_end,
                        resolution: parsed.resolution,
                        source: parsed.source,
                        video_codec: parsed.video_codec,
                        audio: parsed.audio,
                        hdr: parsed.hdr,
                        release_group: parsed.release_group,
                        edition: parsed.edition,
                        complete_series: parsed.complete_series,
                        confidence: parsed.confidence,
                    };
                    (Some(result), Some(time_ms), None)
                }
                Err(e) => {
                    let time_ms = llm_start.elapsed().as_secs_f64() * 1000.0;
                    (None, Some(time_ms), Some(e.to_string()))
                }
            }
        } else {
            (None, None, Some("LLM parsing is not enabled".to_string()))
        };

        Ok(TestFilenameParserResult {
            regex_result,
            regex_time_ms,
            llm_result,
            llm_time_ms,
            llm_error,
        })
    }

    /// Set a generic app setting by key
    ///
    /// This allows setting arbitrary key-value pairs in the app_settings table.
    /// The category is extracted from the key (e.g., "metadata.tmdb_api_key" → "metadata").
    async fn set_setting(
        &self,
        ctx: &Context<'_>,
        key: String,
        value: String,
    ) -> Result<SettingsResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        // Extract category from key (use first part before dot, or "general")
        let category = key.split('.').next().unwrap_or("general").to_string();

        settings
            .set_with_category(&key, &value, &category, None)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        tracing::debug!(key = %key, "Updated app setting");

        Ok(SettingsResult {
            success: true,
            error: None,
        })
    }

    /// Refresh the TV schedule cache
    ///
    /// Forces a refresh of the TV schedule cache from TVMaze.
    /// This is normally done automatically every 6 hours.
    async fn refresh_schedule_cache(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 14, desc = "Number of days to fetch")] days: i32,
        #[graphql(desc = "Country code (e.g., 'US', 'GB')")] country: Option<String>,
    ) -> Result<RefreshScheduleResult> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        match metadata
            .refresh_schedule_cache(days as u32, country.as_deref())
            .await
        {
            Ok(count) => Ok(RefreshScheduleResult {
                success: true,
                entries_updated: count as i32,
                error: None,
            }),
            Err(e) => Ok(RefreshScheduleResult {
                success: false,
                entries_updated: 0,
                error: Some(e.to_string()),
            }),
        }
    }
}
