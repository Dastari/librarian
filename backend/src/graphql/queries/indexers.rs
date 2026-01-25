use super::prelude::*;

#[derive(Default)]
pub struct IndexerQueries;

#[Object]
impl IndexerQueries {
    /// Get all configured indexers for the current user
    async fn indexers(&self, ctx: &Context<'_>) -> Result<Vec<IndexerConfig>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .indexers()
            .list_by_user(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| IndexerConfig {
                id: r.id.to_string(),
                indexer_type: r.indexer_type,
                name: r.name,
                enabled: r.enabled,
                priority: r.priority,
                site_url: r.site_url,
                is_healthy: r.error_count == 0,
                last_error: r.last_error,
                error_count: r.error_count,
                last_success_at: r.last_success_at.map(|dt| dt.to_rfc3339()),
                created_at: r.created_at.to_rfc3339(),
                updated_at: r.updated_at.to_rfc3339(),
                capabilities: IndexerCapabilities {
                    supports_search: r.supports_search.unwrap_or(true),
                    supports_tv_search: r.supports_tv_search.unwrap_or(true),
                    supports_movie_search: r.supports_movie_search.unwrap_or(true),
                    supports_music_search: r.supports_music_search.unwrap_or(false),
                    supports_book_search: r.supports_book_search.unwrap_or(false),
                    supports_imdb_search: r.supports_imdb_search.unwrap_or(false),
                    supports_tvdb_search: r.supports_tvdb_search.unwrap_or(false),
                },
            })
            .collect())
    }

    /// Get a specific indexer by ID
    async fn indexer(&self, ctx: &Context<'_>, id: String) -> Result<Option<IndexerConfig>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let config_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid indexer ID: {}", e)))?;

        let record = db
            .indexers()
            .get(config_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Verify ownership
        if let Some(ref r) = record {
            let user_id = Uuid::parse_str(&user.user_id)?;
            if r.user_id != user_id {
                return Ok(None);
            }
        }

        Ok(record.map(|r| IndexerConfig {
            id: r.id.to_string(),
            indexer_type: r.indexer_type,
            name: r.name,
            enabled: r.enabled,
            priority: r.priority,
            site_url: r.site_url,
            is_healthy: r.error_count == 0,
            last_error: r.last_error,
            error_count: r.error_count,
            last_success_at: r.last_success_at.map(|dt| dt.to_rfc3339()),
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
            capabilities: IndexerCapabilities {
                supports_search: r.supports_search.unwrap_or(true),
                supports_tv_search: r.supports_tv_search.unwrap_or(true),
                supports_movie_search: r.supports_movie_search.unwrap_or(true),
                supports_music_search: r.supports_music_search.unwrap_or(false),
                supports_book_search: r.supports_book_search.unwrap_or(false),
                supports_imdb_search: r.supports_imdb_search.unwrap_or(false),
                supports_tvdb_search: r.supports_tvdb_search.unwrap_or(false),
            },
        }))
    }

    /// Get available indexer types (for creating new indexers)
    async fn available_indexer_types(&self, ctx: &Context<'_>) -> Result<Vec<IndexerTypeInfo>> {
        let _user = ctx.auth_user()?;

        use crate::indexer::definitions::get_available_indexers;

        let types = get_available_indexers()
            .iter()
            .map(|info| IndexerTypeInfo {
                id: info.id.to_string(),
                name: info.name.to_string(),
                description: info.description.to_string(),
                tracker_type: info.tracker_type.to_string(),
                language: info.language.to_string(),
                site_link: info.site_link.to_string(),
                required_credentials: info
                    .required_credentials
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                is_native: info.is_native,
            })
            .collect();

        Ok(types)
    }

    /// Get setting definitions for an indexer type
    async fn indexer_setting_definitions(
        &self,
        ctx: &Context<'_>,
        indexer_type: String,
    ) -> Result<Vec<IndexerSettingDefinition>> {
        let _user = ctx.auth_user()?;

        use crate::indexer::definitions::{SettingType, get_indexer_info};

        let info = get_indexer_info(&indexer_type).ok_or_else(|| {
            async_graphql::Error::new(format!("Unknown indexer type: {}", indexer_type))
        })?;

        let settings = info
            .optional_settings
            .iter()
            .map(|s| IndexerSettingDefinition {
                key: s.key.to_string(),
                label: s.label.to_string(),
                setting_type: match s.setting_type {
                    SettingType::Text => "text".to_string(),
                    SettingType::Password => "password".to_string(),
                    SettingType::Checkbox => "checkbox".to_string(),
                    SettingType::Select => "select".to_string(),
                },
                default_value: s.default_value.map(|s| s.to_string()),
                options: s.options.map(|opts| {
                    opts.iter()
                        .map(|(value, label)| IndexerSettingOption {
                            value: value.to_string(),
                            label: label.to_string(),
                        })
                        .collect()
                }),
            })
            .collect();

        Ok(settings)
    }

    /// Search indexers for torrents
    async fn search_indexers(
        &self,
        ctx: &Context<'_>,
        input: IndexerSearchInput,
    ) -> Result<IndexerSearchResultSet> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let start = std::time::Instant::now();

        // Get indexers to search
        let indexer_ids: Option<Vec<Uuid>> = input.indexer_ids.as_ref().map(|ids| {
            ids.iter()
                .filter_map(|id| Uuid::parse_str(id).ok())
                .collect()
        });

        let configs = if let Some(ref ids) = indexer_ids {
            let all_configs = db
                .indexers()
                .list_enabled_by_user(user_id)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            all_configs
                .into_iter()
                .filter(|c| ids.contains(&c.id))
                .collect()
        } else {
            db.indexers()
                .list_enabled_by_user(user_id)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?
        };

        // Get encryption key from database
        let encryption_key = db
            .settings()
            .get_or_create_indexer_encryption_key()
            .await
            .map_err(|e| {
                async_graphql::Error::new(format!("Failed to get encryption key: {}", e))
            })?;
        let encryption =
            crate::indexer::encryption::CredentialEncryption::from_base64_key(&encryption_key)
                .map_err(|e| async_graphql::Error::new(format!("Encryption error: {}", e)))?;

        // Build query
        use crate::indexer::{QueryType, TorznabQuery};
        let query = TorznabQuery {
            query_type: if input.season.is_some() || input.episode.is_some() {
                QueryType::TvSearch
            } else if input.imdb_id.is_some() {
                QueryType::MovieSearch
            } else {
                QueryType::Search
            },
            search_term: Some(input.query.clone()),
            categories: input.categories.unwrap_or_default(),
            season: input.season,
            episode: input.episode,
            imdb_id: input.imdb_id,
            limit: input.limit,
            cache: true,
            ..Default::default()
        };

        let mut results: Vec<IndexerSearchResultItem> = Vec::new();
        let mut total_releases = 0;

        // Search each indexer
        for config in configs {
            let indexer_start = std::time::Instant::now();

            // Get and decrypt credentials
            let credentials = match db.indexers().get_credentials(config.id).await {
                Ok(c) => c,
                Err(e) => {
                    results.push(IndexerSearchResultItem {
                        indexer_id: config.id.to_string(),
                        indexer_name: config.name.clone(),
                        releases: vec![],
                        elapsed_ms: indexer_start.elapsed().as_millis() as i64,
                        from_cache: false,
                        error: Some(format!("Failed to get credentials: {}", e)),
                    });
                    continue;
                }
            };

            let settings = db
                .indexers()
                .get_settings(config.id)
                .await
                .unwrap_or_default();

            let mut decrypted_creds: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for cred in credentials {
                if let Ok(value) = encryption.decrypt(&cred.encrypted_value, &cred.nonce) {
                    decrypted_creds.insert(cred.credential_type, value);
                }
            }

            let settings_map: std::collections::HashMap<String, String> = settings
                .into_iter()
                .map(|s| (s.setting_key, s.setting_value))
                .collect();

            // Create and search indexer
            match config.indexer_type.as_str() {
                "iptorrents" => {
                    use crate::indexer::Indexer;
                    use crate::indexer::definitions::iptorrents::IPTorrentsIndexer;

                    let cookie = decrypted_creds.get("cookie").cloned().unwrap_or_default();
                    let user_agent = decrypted_creds
                        .get("user_agent")
                        .cloned()
                        .unwrap_or_default();

                    let indexer = match IPTorrentsIndexer::new(
                        config.id.to_string(),
                        config.name.clone(),
                        config.site_url.clone(),
                        &cookie,
                        &user_agent,
                        settings_map,
                    ) {
                        Ok(idx) => idx,
                        Err(e) => {
                            results.push(IndexerSearchResultItem {
                                indexer_id: config.id.to_string(),
                                indexer_name: config.name,
                                releases: vec![],
                                elapsed_ms: indexer_start.elapsed().as_millis() as i64,
                                from_cache: false,
                                error: Some(format!("Failed to create indexer: {}", e)),
                            });
                            continue;
                        }
                    };

                    match indexer.search(&query).await {
                        Ok(releases) => {
                            let _ = db.indexers().record_success(config.id).await;

                            let torrent_releases: Vec<TorrentRelease> = releases
                                .iter()
                                .map(|r| TorrentRelease {
                                    title: r.title.clone(),
                                    guid: r.guid.clone(),
                                    link: r.link.clone(),
                                    magnet_uri: r.magnet_uri.clone(),
                                    info_hash: r.info_hash.clone(),
                                    details: r.details.clone(),
                                    publish_date: r.publish_date.to_rfc3339(),
                                    categories: r.categories.clone(),
                                    size: r.size,
                                    size_formatted: r.size.map(|s| format_bytes(s as u64)),
                                    seeders: r.seeders,
                                    leechers: r.leechers(),
                                    peers: r.peers,
                                    grabs: r.grabs,
                                    is_freeleech: r.is_freeleech(),
                                    imdb_id: r.imdb.map(|id| format!("tt{:07}", id)),
                                    poster: r.poster.clone(),
                                    description: r.description.clone(),
                                    indexer_id: Some(config.id.to_string()),
                                    indexer_name: Some(config.name.clone()),
                                })
                                .collect();

                            total_releases += torrent_releases.len() as i32;

                            results.push(IndexerSearchResultItem {
                                indexer_id: config.id.to_string(),
                                indexer_name: config.name,
                                releases: torrent_releases,
                                elapsed_ms: indexer_start.elapsed().as_millis() as i64,
                                from_cache: false,
                                error: None,
                            });
                        }
                        Err(e) => {
                            let _ = db.indexers().record_error(config.id, &e.to_string()).await;

                            results.push(IndexerSearchResultItem {
                                indexer_id: config.id.to_string(),
                                indexer_name: config.name,
                                releases: vec![],
                                elapsed_ms: indexer_start.elapsed().as_millis() as i64,
                                from_cache: false,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                }
                _ => {
                    results.push(IndexerSearchResultItem {
                        indexer_id: config.id.to_string(),
                        indexer_name: config.name,
                        releases: vec![],
                        elapsed_ms: indexer_start.elapsed().as_millis() as i64,
                        from_cache: false,
                        error: Some(format!("Unsupported indexer type: {}", config.indexer_type)),
                    });
                }
            }
        }

        let sources_searched = results.len() as i32;
        let priority_rule_used = if input.library_type.is_some() || input.library_id.is_some() {
            Some("default (priority search not yet active)".to_string())
        } else {
            None
        };

        Ok(IndexerSearchResultSet {
            indexers: results,
            total_releases,
            total_elapsed_ms: start.elapsed().as_millis() as i64,
            stopped_early: false,
            sources_searched,
            priority_rule_used,
        })
    }
}
