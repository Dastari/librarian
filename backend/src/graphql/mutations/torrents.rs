use super::prelude::*;
use crate::services::torrent_file_matcher::TorrentFileMatcher;

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

        // Parse optional album_id for music torrents
        let album_id = input
            .album_id
            .as_ref()
            .and_then(|id| Uuid::parse_str(id).ok());

        // Parse optional movie_id for movie torrents
        let movie_id = input
            .movie_id
            .as_ref()
            .and_then(|id| Uuid::parse_str(id).ok());

        // Parse optional episode_id for TV torrents
        let episode_id = input
            .episode_id
            .as_ref()
            .and_then(|id| Uuid::parse_str(id).ok());

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
                                    tracing::debug!(
                                        url = %url,
                                        indexer_type = %config.indexer_type,
                                        "Attempting authenticated torrent download"
                                    );

                                    let torrent_bytes = download_torrent_file_authenticated(
                                        &url,
                                        &config.indexer_type,
                                        &decrypted_creds,
                                    ).await;

                                    match torrent_bytes {
                                        Ok(bytes) => {
                                            tracing::debug!(
                                                size = bytes.len(),
                                                "Downloaded torrent file, adding to client"
                                            );

                                            // Add torrent from bytes
                                            match service.add_torrent_bytes(&bytes, user_id).await {
                                                Ok(info) => {
                                                    tracing::info!(
                                                        user_id = %user.user_id,
                                                        torrent_id = info.id,
                                                        torrent_name = %info.name,
                                                        "User added torrent from authenticated download"
                                                    );

                                                    // Create file-level matches if a target item is specified
                                                    create_file_matches_for_target(
                                                        db,
                                                        &info,
                                                        album_id,
                                                        movie_id,
                                                        episode_id,
                                                    )
                                                    .await;

                                                    return Ok(AddTorrentResult {
                                                        success: true,
                                                        torrent: Some(info.into()),
                                                        error: None,
                                                    });
                                                }
                                                Err(e) => {
                                                    tracing::error!(
                                                        error = %e,
                                                        url = %url,
                                                        bytes_len = bytes.len(),
                                                        "Failed to add torrent from downloaded bytes"
                                                    );
                                                    return Ok(AddTorrentResult {
                                                        success: false,
                                                        torrent: None,
                                                        error: Some(e.to_string()),
                                                    });
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                error = %e,
                                                url = %url,
                                                indexer_type = %config.indexer_type,
                                                "Failed to download .torrent file with auth, falling back to unauthenticated"
                                            );
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

                // Create file-level matches if a target item is specified
                create_file_matches_for_target(db, &info, album_id, movie_id, episode_id).await;

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
    /// This uses the unified MediaProcessor to:
    /// 1. Parse filenames to identify show/episode
    /// 2. Match to existing shows in the library
    /// 3. Copy/move/hardlink files based on library settings
    /// 4. Create folder structure (Show Name/Season XX/)
    /// 5. Update episode status to downloaded
    ///
    /// If library_id is provided, the torrent will be linked to that library first.
    /// If album_id is provided for music, files will be matched to that album's tracks.
    async fn organize_torrent(
        &self,
        ctx: &Context<'_>,
        id: i32,
        #[graphql(desc = "Optional library ID to organize into (links torrent to library first)")]
        library_id: Option<String>,
        #[graphql(desc = "Optional album ID for music torrents")] album_id: Option<String>,
    ) -> Result<OrganizeTorrentResult> {
        use tracing::info;
        
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();

        // Get torrent info to get info_hash and files
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

        info!(
            torrent_id = id,
            torrent_name = %torrent_info.name,
            file_count = torrent_info.files.len(),
            library_id = ?library_id,
            album_id = ?album_id,
            "Organizing torrent"
        );
        
        // Log library_id if provided (torrent-library linking is handled by file matches)
        if let Some(ref lib_id_str) = library_id {
            info!(
                library_id = %lib_id_str,
                "Library specified for organization"
            );
        }

        // If explicit targets are provided (album_id, movie_id, episode_id), create file matches
        // This ensures that when users explicitly select an album in the UI, files get matched
        let album_uuid = album_id.as_ref().and_then(|s| uuid::Uuid::parse_str(s).ok());
        
        if album_uuid.is_some() {
            info!(
                album_id = ?album_id,
                torrent_name = %torrent_info.name,
                "Creating explicit file matches for target"
            );
            
            // Use the helper function to create file matches
            create_file_matches_for_target(db, &torrent_info, album_uuid, None, None).await;
        }

        // Use the unified MediaProcessor with force=true to reprocess
        // Include metadata service for auto-adding movies from TMDB
        let metadata_service = ctx.data_unchecked::<Arc<crate::services::MetadataService>>();
        let processor = crate::services::MediaProcessor::with_services(
            db.clone(),
            ctx.data_unchecked::<Arc<crate::services::MediaAnalysisQueue>>()
                .clone(),
            metadata_service.clone(),
        );

        match processor
            .process_torrent(torrent_service, &torrent_info.info_hash, true)
            .await
        {
            Ok(result) => {
                info!(
                    torrent_name = %torrent_info.name,
                    success = result.success,
                    files_processed = result.files_processed,
                    files_failed = result.files_failed,
                    organized = result.organized,
                    "Torrent organization complete: '{}' - {} files processed, {} failed (success: {})",
                    torrent_info.name, result.files_processed, result.files_failed, result.success
                );
                Ok(OrganizeTorrentResult {
                    success: result.success,
                    organized_count: result.files_processed,
                    failed_count: result.files_failed,
                    messages: result.messages,
                })
            }
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

/// Create file-level matches for a torrent if a target item (album, movie, or episode) is specified
///
/// This is called when adding a torrent from the Hunt page or other locations where
/// the user has explicitly selected what the torrent is for. It creates entries in
/// `torrent_file_matches` so the torrent processor knows how to organize the files.
async fn create_file_matches_for_target(
    db: &Database,
    torrent_info: &crate::services::torrent::TorrentInfo,
    album_id: Option<Uuid>,
    movie_id: Option<Uuid>,
    episode_id: Option<Uuid>,
) {
    // Get the torrent database record
    let torrent_record = match db
        .torrents()
        .get_by_info_hash(&torrent_info.info_hash)
        .await
    {
        Ok(Some(record)) => record,
        Ok(None) => {
            tracing::warn!(
                info_hash = %torrent_info.info_hash,
                "Torrent record not found, cannot create file matches"
            );
            return;
        }
        Err(e) => {
            tracing::warn!(
                info_hash = %torrent_info.info_hash,
                error = %e,
                "Failed to get torrent record for file matching"
            );
            return;
        }
    };

    let matcher = TorrentFileMatcher::new(db.clone());

    // Create file matches based on what type of target was specified
    if let Some(album_id) = album_id {
        // Get album info for logging
        let album_info = db.albums().get_by_id(album_id).await.ok().flatten();
        let album_name = album_info.as_ref().map(|a| a.name.as_str()).unwrap_or("Unknown");
        
        match matcher
            .create_matches_for_album(torrent_record.id, &torrent_info.files, album_id)
            .await
        {
            Ok(matches) => {
                tracing::info!(
                    torrent_name = %torrent_info.name,
                    torrent_id = %torrent_record.id,
                    album_id = %album_id,
                    album_name = %album_name,
                    matched_files = matches.len(),
                    total_files = torrent_info.files.len(),
                    "Created {} file matches for album '{}' from torrent '{}'",
                    matches.len(), album_name, torrent_info.name
                );
            }
            Err(e) => {
                tracing::error!(
                    torrent_name = %torrent_info.name,
                    torrent_id = %torrent_record.id,
                    album_id = %album_id,
                    album_name = %album_name,
                    error = %e,
                    "Failed to create file matches for album '{}': {}",
                    album_name, e
                );
            }
        }
    }

    if let Some(movie_id) = movie_id {
        match matcher
            .create_matches_for_movie_torrent(torrent_record.id, &torrent_info.files, movie_id)
            .await
        {
            Ok(matches) => {
                tracing::info!(
                    torrent_id = %torrent_record.id,
                    movie_id = %movie_id,
                    matched_files = matches.len(),
                    "Created file matches for movie"
                );
            }
            Err(e) => {
                tracing::error!(
                    torrent_id = %torrent_record.id,
                    movie_id = %movie_id,
                    error = %e,
                    "Failed to create file matches for movie"
                );
            }
        }
    }

    if let Some(episode_id) = episode_id {
        match matcher
            .create_matches_for_episode_torrent(torrent_record.id, &torrent_info.files, episode_id)
            .await
        {
            Ok(matches) => {
                tracing::info!(
                    torrent_id = %torrent_record.id,
                    episode_id = %episode_id,
                    matched_files = matches.len(),
                    "Created file matches for episode"
                );

                // Mark episode as downloading
                if let Err(e) = db.episodes().mark_downloading(episode_id).await {
                    tracing::warn!(
                        episode_id = %episode_id,
                        error = %e,
                        "Failed to mark episode as downloading"
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    torrent_id = %torrent_record.id,
                    episode_id = %episode_id,
                    error = %e,
                    "Failed to create file matches for episode"
                );
            }
        }
    }
}
