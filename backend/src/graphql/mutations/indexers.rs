use super::prelude::*;

#[derive(Default)]
pub struct IndexerMutations;

#[Object]
impl IndexerMutations {
    /// Create a new indexer
    async fn create_indexer(
        &self,
        ctx: &Context<'_>,
        input: CreateIndexerInput,
    ) -> Result<IndexerResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        // Validate indexer type exists
        use crate::indexer::definitions::get_indexer_info;
        if get_indexer_info(&input.indexer_type).is_none() {
            return Ok(IndexerResult {
                success: false,
                error: Some(format!("Unknown indexer type: {}", input.indexer_type)),
                indexer: None,
            });
        }

        // Create the indexer config
        let create_data = crate::db::CreateIndexerConfig {
            user_id,
            indexer_type: input.indexer_type.clone(),
            definition_id: None,
            name: input.name.clone(),
            site_url: input.site_url.clone(),
        };

        let record = match db.indexers().create(create_data).await {
            Ok(r) => r,
            Err(e) => {
                return Ok(IndexerResult {
                    success: false,
                    error: Some(format!("Failed to create indexer: {}", e)),
                    indexer: None,
                });
            }
        };

        // Get encryption key from database (auto-generates if not set)
        let encryption_key = match db.settings().get_or_create_indexer_encryption_key().await {
            Ok(key) => key,
            Err(e) => {
                return Ok(IndexerResult {
                    success: false,
                    error: Some(format!("Failed to get encryption key: {}", e)),
                    indexer: None,
                });
            }
        };

        let encryption = match crate::indexer::encryption::CredentialEncryption::from_base64_key(
            &encryption_key,
        ) {
            Ok(e) => e,
            Err(e) => {
                return Ok(IndexerResult {
                    success: false,
                    error: Some(format!("Encryption error: {}", e)),
                    indexer: None,
                });
            }
        };

        // Store encrypted credentials
        for cred in input.credentials {
            let (encrypted_value, nonce) = match encryption.encrypt(&cred.value) {
                Ok(v) => v,
                Err(e) => {
                    return Ok(IndexerResult {
                        success: false,
                        error: Some(format!("Failed to encrypt credential: {}", e)),
                        indexer: None,
                    });
                }
            };

            let upsert = crate::db::UpsertCredential {
                credential_type: cred.credential_type,
                encrypted_value,
                nonce,
            };

            if let Err(e) = db.indexers().upsert_credential(record.id, upsert).await {
                return Ok(IndexerResult {
                    success: false,
                    error: Some(format!("Failed to save credential: {}", e)),
                    indexer: None,
                });
            }
        }

        // Store settings
        for setting in input.settings {
            if let Err(e) = db
                .indexers()
                .upsert_setting(record.id, &setting.key, &setting.value)
                .await
            {
                return Ok(IndexerResult {
                    success: false,
                    error: Some(format!("Failed to save setting: {}", e)),
                    indexer: None,
                });
            }
        }

        Ok(IndexerResult {
            success: true,
            error: None,
            indexer: Some(IndexerConfig {
                id: record.id.to_string(),
                indexer_type: record.indexer_type,
                name: record.name,
                enabled: record.enabled,
                priority: record.priority,
                site_url: record.site_url,
                is_healthy: true,
                last_error: None,
                error_count: 0,
                last_success_at: None,
                created_at: record.created_at.to_rfc3339(),
                updated_at: record.updated_at.to_rfc3339(),
                capabilities: IndexerCapabilities {
                    supports_search: true,
                    supports_tv_search: true,
                    supports_movie_search: true,
                    supports_music_search: false,
                    supports_book_search: false,
                    supports_imdb_search: false,
                    supports_tvdb_search: false,
                },
            }),
        })
    }

    /// Update an existing indexer
    async fn update_indexer(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateIndexerInput,
    ) -> Result<IndexerResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let config_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid indexer ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)?;

        // Verify ownership
        let existing = db
            .indexers()
            .get(config_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let _existing = match existing {
            Some(r) if r.user_id == user_id => r,
            Some(_) => {
                return Ok(IndexerResult {
                    success: false,
                    error: Some("Indexer not found".to_string()),
                    indexer: None,
                });
            }
            None => {
                return Ok(IndexerResult {
                    success: false,
                    error: Some("Indexer not found".to_string()),
                    indexer: None,
                });
            }
        };

        // Update config
        let update_data = crate::db::UpdateIndexerConfig {
            name: input.name,
            enabled: input.enabled,
            priority: input.priority,
            site_url: input.site_url,
            ..Default::default()
        };

        let record = match db.indexers().update(config_id, update_data).await {
            Ok(Some(r)) => r,
            Ok(None) => {
                return Ok(IndexerResult {
                    success: false,
                    error: Some("Indexer not found".to_string()),
                    indexer: None,
                });
            }
            Err(e) => {
                return Ok(IndexerResult {
                    success: false,
                    error: Some(format!("Failed to update indexer: {}", e)),
                    indexer: None,
                });
            }
        };

        // Update credentials if provided
        if let Some(credentials) = input.credentials {
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

            for cred in credentials {
                let (encrypted_value, nonce) = encryption
                    .encrypt(&cred.value)
                    .map_err(|e| async_graphql::Error::new(format!("Encryption error: {}", e)))?;

                let upsert = crate::db::UpsertCredential {
                    credential_type: cred.credential_type,
                    encrypted_value,
                    nonce,
                };

                db.indexers()
                    .upsert_credential(config_id, upsert)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            }
        }

        // Update settings if provided
        if let Some(settings) = input.settings {
            for setting in settings {
                db.indexers()
                    .upsert_setting(config_id, &setting.key, &setting.value)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            }
        }

        Ok(IndexerResult {
            success: true,
            error: None,
            indexer: Some(IndexerConfig {
                id: record.id.to_string(),
                indexer_type: record.indexer_type,
                name: record.name,
                enabled: record.enabled,
                priority: record.priority,
                site_url: record.site_url,
                is_healthy: record.error_count == 0,
                last_error: record.last_error,
                error_count: record.error_count,
                last_success_at: record.last_success_at.map(|dt| dt.to_rfc3339()),
                created_at: record.created_at.to_rfc3339(),
                updated_at: record.updated_at.to_rfc3339(),
                capabilities: IndexerCapabilities {
                    supports_search: record.supports_search.unwrap_or(true),
                    supports_tv_search: record.supports_tv_search.unwrap_or(true),
                    supports_movie_search: record.supports_movie_search.unwrap_or(true),
                    supports_music_search: record.supports_music_search.unwrap_or(false),
                    supports_book_search: record.supports_book_search.unwrap_or(false),
                    supports_imdb_search: record.supports_imdb_search.unwrap_or(false),
                    supports_tvdb_search: record.supports_tvdb_search.unwrap_or(false),
                },
            }),
        })
    }

    /// Delete an indexer
    async fn delete_indexer(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let config_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid indexer ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)?;

        // Verify ownership
        let existing = db
            .indexers()
            .get(config_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        match existing {
            Some(r) if r.user_id == user_id => {}
            _ => {
                return Ok(MutationResult {
                    success: false,
                    error: Some("Indexer not found".to_string()),
                });
            }
        }

        let deleted = db
            .indexers()
            .delete(config_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: deleted,
            error: if deleted {
                None
            } else {
                Some("Failed to delete indexer".to_string())
            },
        })
    }

    /// Test an indexer connection
    async fn test_indexer(&self, ctx: &Context<'_>, id: String) -> Result<IndexerTestResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let config_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid indexer ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)?;

        tracing::info!(
            indexer_id = %config_id,
            user_id = %user_id,
            "Testing indexer connection"
        );

        // Verify ownership
        let config = match db
            .indexers()
            .get(config_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
        {
            Some(r) if r.user_id == user_id => r,
            _ => {
                tracing::warn!(indexer_id = %config_id, "Indexer not found or not owned by user");
                return Ok(IndexerTestResult {
                    success: false,
                    error: Some("Indexer not found".to_string()),
                    releases_found: None,
                    elapsed_ms: None,
                });
            }
        };

        tracing::debug!(
            indexer_id = %config_id,
            indexer_name = %config.name,
            indexer_type = %config.indexer_type,
            "Found indexer config"
        );

        // Get credentials
        let credentials = db
            .indexers()
            .get_credentials(config_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        tracing::debug!(
            indexer_id = %config_id,
            credential_count = credentials.len(),
            credential_types = ?credentials.iter().map(|c| &c.credential_type).collect::<Vec<_>>(),
            "Retrieved credentials"
        );

        let settings = db
            .indexers()
            .get_settings(config_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        tracing::debug!(
            indexer_id = %config_id,
            settings_count = settings.len(),
            "Retrieved settings"
        );

        // Decrypt credentials using database-stored key
        let encryption_key = match db.settings().get_or_create_indexer_encryption_key().await {
            Ok(key) => key,
            Err(e) => {
                tracing::error!(error = %e, "Failed to get encryption key from database");
                return Ok(IndexerTestResult {
                    success: false,
                    error: Some(format!("Failed to get encryption key: {}", e)),
                    releases_found: None,
                    elapsed_ms: None,
                });
            }
        };
        let encryption = match crate::indexer::encryption::CredentialEncryption::from_base64_key(
            &encryption_key,
        ) {
            Ok(e) => e,
            Err(e) => {
                tracing::error!(error = %e, "Failed to initialize encryption");
                return Ok(IndexerTestResult {
                    success: false,
                    error: Some(format!("Encryption error: {}", e)),
                    releases_found: None,
                    elapsed_ms: None,
                });
            }
        };

        let mut decrypted_creds: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for cred in credentials {
            match encryption.decrypt(&cred.encrypted_value, &cred.nonce) {
                Ok(value) => {
                    tracing::debug!(
                        credential_type = %cred.credential_type,
                        value_len = value.len(),
                        "Decrypted credential"
                    );
                    decrypted_creds.insert(cred.credential_type, value);
                }
                Err(e) => {
                    tracing::error!(
                        credential_type = %cred.credential_type,
                        error = %e,
                        "Failed to decrypt credential"
                    );
                    return Ok(IndexerTestResult {
                        success: false,
                        error: Some(format!("Failed to decrypt credential: {}", e)),
                        releases_found: None,
                        elapsed_ms: None,
                    });
                }
            }
        }

        let settings_map: std::collections::HashMap<String, String> = settings
            .into_iter()
            .map(|s| (s.setting_key, s.setting_value))
            .collect();

        // Create indexer instance and test
        let start = std::time::Instant::now();

        match config.indexer_type.as_str() {
            "iptorrents" => {
                use crate::indexer::definitions::iptorrents::IPTorrentsIndexer;
                use crate::indexer::{Indexer, TorznabQuery};

                let cookie = decrypted_creds.get("cookie").cloned().unwrap_or_default();
                let user_agent = decrypted_creds
                    .get("user_agent")
                    .cloned()
                    .unwrap_or_default();

                tracing::info!(
                    indexer_id = %config_id,
                    indexer_name = %config.name,
                    cookie_len = cookie.len(),
                    has_user_agent = !user_agent.is_empty(),
                    "Creating IPTorrents indexer instance"
                );

                if cookie.is_empty() {
                    tracing::warn!(indexer_id = %config_id, "Cookie is empty - authentication will likely fail");
                }

                let indexer = match IPTorrentsIndexer::new(
                    config_id.to_string(),
                    config.name.clone(),
                    config.site_url.clone(),
                    &cookie,
                    &user_agent,
                    settings_map,
                ) {
                    Ok(idx) => idx,
                    Err(e) => {
                        tracing::error!(
                            indexer_id = %config_id,
                            error = %e,
                            "Failed to create indexer instance"
                        );
                        return Ok(IndexerTestResult {
                            success: false,
                            error: Some(format!("Failed to create indexer: {}", e)),
                            releases_found: None,
                            elapsed_ms: Some(start.elapsed().as_millis() as i64),
                        });
                    }
                };

                tracing::debug!(indexer_id = %config_id, "Testing connection...");

                // Test connection
                match indexer.test_connection().await {
                    Ok(true) => {
                        tracing::info!(indexer_id = %config_id, "Connection test passed, performing search...");

                        // Try a simple search
                        let query = TorznabQuery::search("");
                        match indexer.search(&query).await {
                            Ok(releases) => {
                                tracing::info!(
                                    indexer_id = %config_id,
                                    releases_found = releases.len(),
                                    elapsed_ms = start.elapsed().as_millis(),
                                    "Indexer test successful"
                                );

                                // Record success
                                let _ = db.indexers().record_success(config_id).await;

                                Ok(IndexerTestResult {
                                    success: true,
                                    error: None,
                                    releases_found: Some(releases.len() as i32),
                                    elapsed_ms: Some(start.elapsed().as_millis() as i64),
                                })
                            }
                            Err(e) => {
                                tracing::error!(
                                    indexer_id = %config_id,
                                    error = %e,
                                    elapsed_ms = start.elapsed().as_millis(),
                                    "Search failed"
                                );
                                let _ = db.indexers().record_error(config_id, &e.to_string()).await;

                                Ok(IndexerTestResult {
                                    success: false,
                                    error: Some(format!("Search failed: {}", e)),
                                    releases_found: None,
                                    elapsed_ms: Some(start.elapsed().as_millis() as i64),
                                })
                            }
                        }
                    }
                    Ok(false) => {
                        tracing::warn!(
                            indexer_id = %config_id,
                            elapsed_ms = start.elapsed().as_millis(),
                            "Connection test returned false - likely invalid cookie"
                        );
                        let _ = db
                            .indexers()
                            .record_error(config_id, "Connection test failed")
                            .await;

                        Ok(IndexerTestResult {
                            success: false,
                            error: Some("Connection test failed - check your cookie".to_string()),
                            releases_found: None,
                            elapsed_ms: Some(start.elapsed().as_millis() as i64),
                        })
                    }
                    Err(e) => {
                        tracing::error!(
                            indexer_id = %config_id,
                            error = %e,
                            elapsed_ms = start.elapsed().as_millis(),
                            "Connection test error"
                        );
                        let _ = db.indexers().record_error(config_id, &e.to_string()).await;

                        Ok(IndexerTestResult {
                            success: false,
                            error: Some(format!("Connection error: {}", e)),
                            releases_found: None,
                            elapsed_ms: Some(start.elapsed().as_millis() as i64),
                        })
                    }
                }
            }
            other => {
                tracing::error!(
                    indexer_id = %config_id,
                    indexer_type = other,
                    "Unsupported indexer type"
                );
                Ok(IndexerTestResult {
                    success: false,
                    error: Some(format!("Unsupported indexer type: {}", other)),
                    releases_found: None,
                    elapsed_ms: Some(start.elapsed().as_millis() as i64),
                })
            }
        }
    }

    /// Generate a new encryption key for indexer credentials
    ///
    /// WARNING: This will invalidate ALL existing indexer credentials!
    /// You will need to re-enter the credentials for all indexers.
    async fn regenerate_encryption_key(
        &self,
        ctx: &Context<'_>,
        input: GenerateEncryptionKeyInput,
    ) -> Result<SecuritySettingsResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        if !input.confirm_invalidation {
            return Ok(SecuritySettingsResult {
                success: false,
                error: Some(
                    "You must confirm that you understand this will invalidate existing credentials"
                        .to_string(),
                ),
                settings: None,
            });
        }

        // Generate a new key (32 bytes = 256 bits for AES-256)
        use rand::RngCore;
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let new_key = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key_bytes);

        // Store the new key
        if let Err(e) = db.settings().set_indexer_encryption_key(&new_key).await {
            return Ok(SecuritySettingsResult {
                success: false,
                error: Some(format!("Failed to save new encryption key: {}", e)),
                settings: None,
            });
        }

        tracing::warn!(
            "Indexer encryption key regenerated - all existing credentials are now invalid"
        );

        // Return the new settings
        let preview = format!("{}...{}", &new_key[..4], &new_key[new_key.len() - 4..]);

        Ok(SecuritySettingsResult {
            success: true,
            error: None,
            settings: Some(SecuritySettings {
                encryption_key_set: true,
                encryption_key_preview: Some(preview),
                encryption_key_last_modified: Some(chrono::Utc::now().to_rfc3339()),
            }),
        })
    }

    /// Initialize the encryption key if not already set
    ///
    /// This is safe to call multiple times - it will only create a key if one doesn't exist.
    async fn initialize_encryption_key(&self, ctx: &Context<'_>) -> Result<SecuritySettingsResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        // This will create a key only if one doesn't exist
        let key = match db.settings().get_or_create_indexer_encryption_key().await {
            Ok(k) => k,
            Err(e) => {
                return Ok(SecuritySettingsResult {
                    success: false,
                    error: Some(format!("Failed to initialize encryption key: {}", e)),
                    settings: None,
                });
            }
        };

        let preview = if key.len() >= 8 {
            format!("{}...{}", &key[..4], &key[key.len() - 4..])
        } else {
            "****".to_string()
        };

        Ok(SecuritySettingsResult {
            success: true,
            error: None,
            settings: Some(SecuritySettings {
                encryption_key_set: true,
                encryption_key_preview: Some(preview),
                encryption_key_last_modified: None,
            }),
        })
    }
}
