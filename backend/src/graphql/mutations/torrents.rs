use super::prelude::*;

#[derive(Default)]
pub struct TorrentMutations;

#[Object]
impl TorrentMutations {
    /// Add a new torrent from magnet link or URL to a .torrent file
    async fn add_torrent(
        &self,
        ctx: &Context<'_>,
        input: AddTorrentInput,
    ) -> Result<AddTorrentResult> {
        let user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();
        let db = ctx.data_unchecked::<Database>();

        // Parse user_id for database persistence
        let user_id = Uuid::parse_str(&user.user_id).ok();

        let result = if let Some(magnet) = input.magnet {
            // Magnet links go through add_magnet
            service.add_magnet(&magnet, user_id).await
        } else if let Some(url) = input.url {
            // If indexer_id is provided, download the .torrent file with authentication
            if let Some(ref indexer_id_str) = input.indexer_id {
                if let Ok(indexer_id) = Uuid::parse_str(indexer_id_str) {
                    // Get indexer config and credentials
                    if let Ok(Some(config)) = db.indexers().get(indexer_id).await {
                        if let Ok(credentials) = db.indexers().get_credentials(indexer_id).await {
                            // Get encryption key
                            if let Ok(encryption_key) =
                                db.settings().get_or_create_indexer_encryption_key().await
                            {
                                if let Ok(encryption) = crate::indexer::encryption::CredentialEncryption::from_base64_key(&encryption_key) {
                                    // Decrypt credentials
                                    let mut decrypted_creds: std::collections::HashMap<String, String> =
                                        std::collections::HashMap::new();
                                    for cred in credentials {
                                        if let Ok(value) = encryption.decrypt(&cred.encrypted_value, &cred.nonce) {
                                            decrypted_creds.insert(cred.credential_type, value);
                                        }
                                    }

                                    // Download .torrent file with authentication
                                    let torrent_bytes = download_torrent_file_authenticated(
                                        &url,
                                        &config.indexer_type,
                                        &decrypted_creds,
                                    ).await;

                                    match torrent_bytes {
                                        Ok(bytes) => {
                                            // Add torrent from bytes
                                            return match service.add_torrent_bytes(&bytes, user_id).await {
                                                Ok(info) => {
                                                    tracing::info!(
                                                        user_id = %user.user_id,
                                                        torrent_id = info.id,
                                                        torrent_name = %info.name,
                                                        "User added torrent from authenticated download"
                                                    );

                                                    // Note: File-level matching happens automatically when torrent is processed
                                                    // via torrent_file_matches table

                                                    Ok(AddTorrentResult {
                                                        success: true,
                                                        torrent: Some(info.into()),
                                                        error: None,
                                                    })
                                                }
                                                Err(e) => Ok(AddTorrentResult {
                                                    success: false,
                                                    torrent: None,
                                                    error: Some(e.to_string()),
                                                }),
                                            };
                                        }
                                        Err(e) => {
                                            tracing::warn!(error = %e, "Failed to download .torrent file with auth, falling back to unauthenticated");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Fall back to unauthenticated download
            service.add_torrent_url(&url, user_id).await
        } else {
            return Ok(AddTorrentResult {
                success: false,
                torrent: None,
                error: Some("Either magnet or url must be provided".to_string()),
            });
        };

        match result {
            Ok(info) => {
                tracing::info!(
                    user_id = %user.user_id,
                    torrent_id = info.id,
                    torrent_name = %info.name,
                    info_hash = %info.info_hash,
                    "User added torrent: {}",
                    info.name
                );

                // Note: File-level matching happens automatically when torrent is processed
                // via torrent_file_matches table in TorrentFileMatcher
                let _ = &info.info_hash; // Used for tracing/debugging

                // Mark episode/movie as downloading if explicitly specified
                if let Some(ref episode_id_str) = input.episode_id {
                    if let Ok(ep_id) = Uuid::parse_str(episode_id_str) {
                        let _ = db.episodes().mark_downloading(ep_id).await;
                    }
                }
                // Note: Movie status is derived from media_files, no need to mark downloading

                Ok(AddTorrentResult {
                    success: true,
                    torrent: Some(info.into()),
                    error: None,
                })
            }
            Err(e) => {
                tracing::warn!(
                    user_id = %user.user_id,
                    error = %e,
                    "User failed to add torrent"
                );
                Ok(AddTorrentResult {
                    success: false,
                    torrent: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Pause a torrent
    async fn pause_torrent(&self, ctx: &Context<'_>, id: i32) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();

        match service.pause(id as usize).await {
            Ok(_) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Resume a paused torrent
    async fn resume_torrent(&self, ctx: &Context<'_>, id: i32) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();

        match service.resume(id as usize).await {
            Ok(_) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Remove a torrent
    async fn remove_torrent(
        &self,
        ctx: &Context<'_>,
        id: i32,
        #[graphql(default = false)] delete_files: bool,
    ) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();

        match service.remove(id as usize, delete_files).await {
            Ok(_) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Organize a completed torrent's files into the library structure
    ///
    /// This uses the unified TorrentProcessor to:
    /// 1. Parse filenames to identify show/episode
    /// 2. Match to existing shows in the library
    /// 3. Copy/move/hardlink files based on library settings
    /// 4. Create folder structure (Show Name/Season XX/)
    /// 5. Update episode status to downloaded
    ///
    /// If library_id is provided, the torrent will be linked to that library first.
    async fn organize_torrent(
        &self,
        ctx: &Context<'_>,
        id: i32,
        #[graphql(desc = "Optional library ID to organize into (links torrent to library first)")]
        library_id: Option<String>,
        #[graphql(desc = "Optional album ID for music torrents")] album_id: Option<String>,
    ) -> Result<OrganizeTorrentResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();

        // Get torrent info to get info_hash
        let torrent_info = match torrent_service.get_torrent_info(id as usize).await {
            Ok(info) => info,
            Err(e) => {
                return Ok(OrganizeTorrentResult {
                    success: false,
                    organized_count: 0,
                    failed_count: 0,
                    messages: vec![format!("Failed to get torrent info: {}", e)],
                });
            }
        };

        // Note: library_id and album_id hints are no longer used for torrent-level linking
        // File-level matching will happen automatically in process_torrent
        let _ = (&library_id, &album_id); // Suppress unused warnings

        // Use the unified TorrentProcessor with force=true to reprocess
        // Include metadata service for auto-adding movies from TMDB
        let metadata_service = ctx.data_unchecked::<Arc<crate::services::MetadataService>>();
        let processor = crate::services::TorrentProcessor::with_services(
            db.clone(),
            ctx.data_unchecked::<Arc<crate::services::MediaAnalysisQueue>>()
                .clone(),
            metadata_service.clone(),
        );

        match processor
            .process_torrent(torrent_service, &torrent_info.info_hash, true)
            .await
        {
            Ok(result) => Ok(OrganizeTorrentResult {
                success: result.success,
                organized_count: result.files_processed,
                failed_count: result.files_failed,
                messages: result.messages,
            }),
            Err(e) => Ok(OrganizeTorrentResult {
                success: false,
                organized_count: 0,
                failed_count: 0,
                messages: vec![format!("Organization failed: {}", e)],
            }),
        }
    }

    /// Update torrent client settings
    async fn update_torrent_settings(
        &self,
        ctx: &Context<'_>,
        input: UpdateTorrentSettingsInput,
    ) -> Result<SettingsResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        // Update each setting if provided
        if let Some(v) = input.download_dir {
            settings
                .set_with_category("torrent.download_dir", v, "torrent", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.session_dir {
            settings
                .set_with_category("torrent.session_dir", v, "torrent", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.enable_dht {
            settings
                .set_with_category("torrent.enable_dht", v, "torrent", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.listen_port {
            settings
                .set_with_category("torrent.listen_port", v, "torrent", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.max_concurrent {
            settings
                .set_with_category("torrent.max_concurrent", v, "torrent", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.upload_limit {
            settings
                .set_with_category("torrent.upload_limit", v, "torrent", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.download_limit {
            settings
                .set_with_category("torrent.download_limit", v, "torrent", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }

        Ok(SettingsResult {
            success: true,
            error: None,
        })
    }
}
