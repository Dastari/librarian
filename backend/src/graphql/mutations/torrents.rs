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

    /// Organize a completed torrent's files into the library structure (LEGACY)
    ///
    /// Note: Use processSource mutation for new source-agnostic processing.
    /// This legacy mutation:
    /// 1. Parse filenames to identify show/episode
    /// 2. Match to existing shows in the library
    /// 3. Copy files based on library settings (post_download_action is stored but ignored)
    /// 4. Create folder structure (Show Name/Season XX/)
    /// 5. Update episode status to downloaded
    ///
    /// If library_id is provided, the torrent will be linked to that library first.
    /// If album_id is provided for music, files will be matched to that album's tracks.
    /// Process pending file matches for a torrent (copy files to library)
    async fn organize_torrent(
        &self,
        ctx: &Context<'_>,
        id: i32,
        #[graphql(desc = "Optional library ID to limit matching scope")]
        library_id: Option<String>,
        #[graphql(desc = "Optional album ID for music torrents")] album_id: Option<String>,
    ) -> Result<OrganizeTorrentResult> {
        use crate::services::file_processor::FileProcessor;
        use tracing::info;
        
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();
        let analysis_queue = ctx.data_opt::<Arc<crate::services::queues::MediaAnalysisQueue>>();

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

        // Get torrent DB record
        let torrent_record = match db.torrents().get_by_info_hash(&torrent_info.info_hash).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                return Ok(OrganizeTorrentResult {
                    success: false,
                    organized_count: 0,
                    failed_count: 0,
                    messages: vec!["Torrent not found in database".to_string()],
                });
            }
            Err(e) => {
                return Ok(OrganizeTorrentResult {
                    success: false,
                    organized_count: 0,
                    failed_count: 0,
                    messages: vec![format!("Database error: {}", e)],
                });
            }
        };

        info!(
            torrent_id = id,
            torrent_name = %torrent_info.name,
            file_count = torrent_info.files.len(),
            library_id = ?library_id,
            album_id = ?album_id,
            "Processing torrent"
        );

        // If album_id is provided, create explicit matches for the album's tracks
        if let Some(ref album_id_str) = album_id {
            if let Ok(album_uuid) = Uuid::parse_str(album_id_str) {
                info!(
                    album_id = %album_id_str,
                    torrent_name = %torrent_info.name,
                    "Creating explicit file matches for album"
                );
                
                // Delete any existing matches for this torrent and rematch
                db.pending_file_matches()
                    .delete_by_source("torrent", torrent_record.id)
                    .await
                    .ok();

                // Create matches for album tracks
                create_file_matches_for_target(db, &torrent_info, Some(album_uuid), None, None).await;
            }
        }

        // Use FileProcessor to copy files to library
        let processor = match analysis_queue {
            Some(queue) => FileProcessor::with_analysis_queue(db.clone(), queue.clone()),
            None => FileProcessor::new(db.clone()),
        };

        let result = processor.process_source("torrent", torrent_record.id).await;

        match result {
            Ok(proc_result) => {
                info!(
                    torrent_name = %torrent_info.name,
                    files_processed = proc_result.files_processed,
                    files_failed = proc_result.files_failed,
                    "Torrent processing complete"
                );
                Ok(OrganizeTorrentResult {
                    success: proc_result.files_failed == 0,
                    organized_count: proc_result.files_processed as i32,
                    failed_count: proc_result.files_failed as i32,
                    messages: proc_result.messages,
                })
            }
            Err(e) => Ok(OrganizeTorrentResult {
                success: false,
                organized_count: 0,
                failed_count: 0,
                messages: vec![format!("Processing failed: {}", e)],
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

    // =========================================================================
    // Source-Agnostic File Matching/Processing Mutations
    // =========================================================================

    /// Re-match all files from a source (torrent, usenet, etc.)
    ///
    /// Deletes existing matches and re-runs matching against libraries.
    ///
    /// The source_id can be either:
    /// - A UUID (database ID)
    /// - An info_hash (for torrents) - will be looked up to get the database ID
    async fn rematch_source(
        &self,
        ctx: &Context<'_>,
        source_type: String,
        source_id: String,
        #[graphql(desc = "Optional library ID to limit matching scope")]
        library_id: Option<String>,
    ) -> Result<RematchSourceResult> {
        use crate::services::file_matcher::{FileInfo, FileMatcher};

        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();

        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|_| async_graphql::Error::new("Invalid user ID"))?;
        
        // Try to parse as UUID first, otherwise look up by info_hash for torrents
        let source_uuid = match Uuid::parse_str(&source_id) {
            Ok(uuid) => uuid,
            Err(_) if source_type == "torrent" => {
                // Try to look up by info_hash
                db.torrents()
                    .get_by_info_hash(&source_id)
                    .await
                    .map_err(|e| async_graphql::Error::new(format!("Database error: {}", e)))?
                    .ok_or_else(|| async_graphql::Error::new("Torrent not found by info_hash"))?
                    .id
            }
            Err(_) => {
                return Err(async_graphql::Error::new("Invalid source ID"));
            }
        };
        
        let target_library_id = library_id
            .as_ref()
            .and_then(|s| Uuid::parse_str(s).ok());

        // Delete existing matches for this source
        db.pending_file_matches()
            .delete_by_source(&source_type, source_uuid)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Get files based on source type
        let files: Vec<FileInfo> = if source_type == "torrent" {
            // Get torrent record by ID
            let torrent = db.torrents()
                .get_by_id(source_uuid)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?
                .ok_or_else(|| async_graphql::Error::new("Torrent not found"))?;

            // Get files from torrent service
            let torrent_files = torrent_service.get_files_for_torrent(&torrent.info_hash).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;

            torrent_files
                .iter()
                .enumerate()
                .map(|(idx, f)| FileInfo {
                    path: f.path.clone(),
                    size: f.size as i64,
                    file_index: Some(idx as i32),
                    source_name: Some(torrent.name.clone()),
                })
                .collect()
        } else {
            return Err(async_graphql::Error::new(format!(
                "Unsupported source type: {}",
                source_type
            )));
        };

        // Create matcher and match files
        let matcher = FileMatcher::new(db.clone());
        let matches = matcher.match_files(user_id, files, target_library_id).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Save matches
        let saved = matcher.save_matches(user_id, &source_type, Some(source_uuid), &matches).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let summary = FileMatcher::summarize_matches(&matches);

        Ok(RematchSourceResult {
            success: true,
            match_count: saved.len() as i32,
            error: None,
        })
    }

    /// Process all pending matches for a source (copy files to library)
    ///
    /// The source_id can be either:
    /// - A UUID (database ID)
    /// - An info_hash (for torrents) - will be looked up to get the database ID
    async fn process_source(
        &self,
        ctx: &Context<'_>,
        source_type: String,
        source_id: String,
    ) -> Result<ProcessSourceResult> {
        use crate::services::file_processor::FileProcessor;

        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let analysis_queue = ctx.data_opt::<Arc<crate::services::queues::MediaAnalysisQueue>>();

        // Try to parse as UUID first, otherwise look up by info_hash for torrents
        let source_uuid = match Uuid::parse_str(&source_id) {
            Ok(uuid) => uuid,
            Err(_) if source_type == "torrent" => {
                // Try to look up by info_hash
                db.torrents()
                    .get_by_info_hash(&source_id)
                    .await
                    .map_err(|e| async_graphql::Error::new(format!("Database error: {}", e)))?
                    .ok_or_else(|| async_graphql::Error::new("Torrent not found by info_hash"))?
                    .id
            }
            Err(_) => {
                return Err(async_graphql::Error::new("Invalid source ID"));
            }
        };

        // Create processor
        let processor = match analysis_queue {
            Some(queue) => FileProcessor::with_analysis_queue(db.clone(), queue.clone()),
            None => FileProcessor::new(db.clone()),
        };

        // Process the source
        let result = processor.process_source(&source_type, source_uuid).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(ProcessSourceResult {
            success: result.files_failed == 0,
            files_processed: result.files_processed as i32,
            files_failed: result.files_failed as i32,
            messages: result.messages,
            error: None,
        })
    }

/// Import a completed torrent's files into a library
    ///
    /// This is for the "torrent-first" workflow where you download something
    /// before adding it to your library. It will:
    /// 1. Copy files from the torrent download location to the library root
    /// 2. Trigger a library scan to auto-create the library item
    ///
    /// Use this when you have a completed torrent and want to add its content
    /// as a new item in your library (vs linking to an existing item).
    async fn import_to_library(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Torrent ID (numeric)")] torrent_id: i32,
        #[graphql(desc = "Target library ID")] library_id: String,
    ) -> Result<ImportToLibraryResult> {
        use crate::services::ScannerService;
        use std::path::PathBuf;
        use tracing::{info, warn};

        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();
        let scanner = ctx.data_unchecked::<Arc<ScannerService>>();

        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|_| async_graphql::Error::new("Invalid user ID"))?;
        let library_uuid = Uuid::parse_str(&library_id)
            .map_err(|_| async_graphql::Error::new("Invalid library ID"))?;

        // Get torrent info
        let torrent_info = match torrent_service.get_torrent_info(torrent_id as usize).await {
            Ok(info) => info,
            Err(e) => {
                return Ok(ImportToLibraryResult {
                    success: false,
                    files_copied: 0,
                    error: Some(format!("Failed to get torrent info: {}", e)),
                });
            }
        };

        // Check if torrent is complete
        if torrent_info.progress < 1.0 {
            return Ok(ImportToLibraryResult {
                success: false,
                files_copied: 0,
                error: Some("Torrent is not complete. Wait for download to finish.".to_string()),
            });
        }

        // Get library info
        let library = match db.libraries().get_by_id(library_uuid).await {
            Ok(Some(lib)) => lib,
            Ok(None) => {
                return Ok(ImportToLibraryResult {
                    success: false,
                    files_copied: 0,
                    error: Some("Library not found".to_string()),
                });
            }
            Err(e) => {
                return Ok(ImportToLibraryResult {
                    success: false,
                    files_copied: 0,
                    error: Some(format!("Database error: {}", e)),
                });
            }
        };

        info!(
            torrent_id = torrent_id,
            torrent_name = %torrent_info.name,
            library_id = %library_id,
            library_name = %library.name,
            file_count = torrent_info.files.len(),
            "Importing torrent to library"
        );

        // Determine source path (torrent download location)
        // The torrent's save_path contains the base path
        let source_base = PathBuf::from(&torrent_info.save_path);
        
        // If torrent has a single root folder, use that as the source
        // Otherwise, create a folder with the torrent name
        let source_path = if torrent_info.files.len() == 1 {
            // Single file - copy directly
            source_base.join(&torrent_info.files[0].path)
        } else {
            // Multi-file - the torrent name is usually the folder name
            source_base.join(&torrent_info.name)
        };

        // Destination is library root
        let dest_base = PathBuf::from(&library.path);
        
        // If multi-file torrent, preserve the folder structure
        let dest_path = if torrent_info.files.len() == 1 {
            dest_base.join(
                PathBuf::from(&torrent_info.files[0].path)
                    .file_name()
                    .unwrap_or_default(),
            )
        } else {
            dest_base.join(&torrent_info.name)
        };

        // Copy files
        let mut files_copied = 0;
        
        if source_path.is_file() {
            // Single file copy
            if let Err(e) = tokio::fs::copy(&source_path, &dest_path).await {
                warn!(
                    source = %source_path.display(),
                    dest = %dest_path.display(),
                    error = %e,
                    "Failed to copy file"
                );
                return Ok(ImportToLibraryResult {
                    success: false,
                    files_copied: 0,
                    error: Some(format!("Failed to copy file: {}", e)),
                });
            }
            files_copied = 1;
        } else if source_path.is_dir() {
            // Directory copy - use copy_dir_all
            match copy_dir_recursive(&source_path, &dest_path).await {
                Ok(count) => files_copied = count,
                Err(e) => {
                    warn!(
                        source = %source_path.display(),
                        dest = %dest_path.display(),
                        error = %e,
                        "Failed to copy directory"
                    );
                    return Ok(ImportToLibraryResult {
                        success: false,
                        files_copied: 0,
                        error: Some(format!("Failed to copy directory: {}", e)),
                    });
                }
            }
        } else {
            return Ok(ImportToLibraryResult {
                success: false,
                files_copied: 0,
                error: Some(format!("Source path does not exist: {}", source_path.display())),
            });
        }

        info!(
            torrent_name = %torrent_info.name,
            files_copied = files_copied,
            dest = %dest_path.display(),
            "Files copied to library, triggering scan"
        );

        // Trigger library scan
        tokio::spawn({
            let scanner = scanner.clone();
            async move {
                if let Err(e) = scanner.scan_library(library_uuid).await {
                    tracing::error!(
                        library_id = %library_uuid,
                        error = %e,
                        "Failed to scan library after import"
                    );
                }
            }
        });

        Ok(ImportToLibraryResult {
            success: true,
            files_copied: files_copied as i32,
            error: None,
        })
    }

    /// Manually set a match target for a pending file match
    async fn set_match(
        &self,
        ctx: &Context<'_>,
        match_id: String,
        target_type: String,
        target_id: String,
    ) -> Result<SetMatchResult> {
        use crate::db::MatchTarget;

        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let match_uuid = Uuid::parse_str(&match_id)
            .map_err(|_| async_graphql::Error::new("Invalid match ID"))?;
        let target_uuid = Uuid::parse_str(&target_id)
            .map_err(|_| async_graphql::Error::new("Invalid target ID"))?;

        let target = match target_type.to_lowercase().as_str() {
            "episode" => MatchTarget::Episode(target_uuid),
            "movie" => MatchTarget::Movie(target_uuid),
            "track" => MatchTarget::Track(target_uuid),
            "chapter" => MatchTarget::Chapter(target_uuid),
            _ => return Err(async_graphql::Error::new("Invalid target type")),
        };

        db.pending_file_matches()
            .update_target(match_uuid, target)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(SetMatchResult {
            success: true,
            error: None,
        })
    }

    /// Remove a pending file match
    async fn remove_match(
        &self,
        ctx: &Context<'_>,
        match_id: String,
    ) -> Result<SetMatchResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let match_uuid = Uuid::parse_str(&match_id)
            .map_err(|_| async_graphql::Error::new("Invalid match ID"))?;

        db.pending_file_matches()
            .delete(match_uuid)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(SetMatchResult {
            success: true,
            error: None,
        })
    }
}

/// Result of rematch operation
#[derive(async_graphql::SimpleObject)]
pub struct RematchSourceResult {
    pub success: bool,
    pub match_count: i32,
    pub error: Option<String>,
}

/// Result of process operation
#[derive(async_graphql::SimpleObject)]
pub struct ProcessSourceResult {
    pub success: bool,
    pub files_processed: i32,
    pub files_failed: i32,
    pub messages: Vec<String>,
    pub error: Option<String>,
}

/// Result of set/remove match operations
#[derive(async_graphql::SimpleObject)]
pub struct SetMatchResult {
    pub success: bool,
    pub error: Option<String>,
}

/// Result of import to library operation
#[derive(async_graphql::SimpleObject)]
pub struct ImportToLibraryResult {
    pub success: bool,
    pub files_copied: i32,
    pub error: Option<String>,
}

/// Recursively copy a directory
async fn copy_dir_recursive(
    source: &std::path::Path,
    dest: &std::path::Path,
) -> anyhow::Result<usize> {
    use tokio::fs;

    let mut count = 0;

    // Create destination directory
    fs::create_dir_all(dest).await?;

    let mut entries = fs::read_dir(source).await?;
    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            count += Box::pin(copy_dir_recursive(&src_path, &dest_path)).await?;
        } else {
            fs::copy(&src_path, &dest_path).await?;
            count += 1;
        }
    }

    Ok(count)
}

/// Create file-level matches for a torrent if a target item (album, movie, or episode) is specified
/// Create file-level matches for explicit target selection (album, movie, episode)
///
/// This is called when adding a torrent from the Hunt page or other locations where
/// the user has explicitly selected what the torrent is for. It creates entries in
/// `pending_file_matches` so the file processor knows how to copy the files.
async fn create_file_matches_for_target(
    db: &Database,
    torrent_info: &crate::services::torrent::TorrentInfo,
    album_id: Option<Uuid>,
    movie_id: Option<Uuid>,
    episode_id: Option<Uuid>,
) {
    use crate::db::{CreatePendingFileMatch, MatchTarget};
    use crate::services::file_utils::{is_audio_file, is_video_file};
    use crate::services::filename_parser;

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

    let user_id = torrent_record.user_id;

    // Create file matches based on what type of target was specified
    if let Some(album_id) = album_id {
        // Get album info and tracks for matching
        let album_info = db.albums().get_by_id(album_id).await.ok().flatten();
        let album_name = album_info.as_ref().map(|a| a.name.as_str()).unwrap_or("Unknown");
        let tracks = db.tracks().list_by_album(album_id).await.unwrap_or_default();

        let mut matches_created = 0;

        // Get audio files from torrent
        let audio_files: Vec<_> = torrent_info
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| is_audio_file(&f.path))
            .collect();

        // Try to match each audio file to a track
        for (idx, file) in &audio_files {
            let file_name = std::path::Path::new(&file.path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file.path);

            // Find best matching track (only match to tracks without files)
            let mut best_match: Option<(Uuid, f64)> = None;
            for track in &tracks {
                // Skip tracks that already have a linked media file
                if track.media_file_id.is_some() {
                    continue;
                }
                let similarity = filename_parser::show_name_similarity(file_name, &track.title);
                if similarity >= 0.5 {
                    if best_match.is_none() || similarity > best_match.unwrap().1 {
                        best_match = Some((track.id, similarity));
                    }
                }
            }

            if let Some((track_id, _confidence)) = best_match {
                let quality = filename_parser::parse_quality(&file.path);

                let input = CreatePendingFileMatch {
                    user_id,
                    source_path: file.path.clone(),
                    source_type: "torrent".to_string(),
                    source_id: Some(torrent_record.id),
                    source_file_index: Some(*idx as i32),
                    file_size: file.size as i64,
                    target: Some(MatchTarget::Track(track_id)),
                    unmatched_reason: None,
                    match_type: "manual".to_string(),
                    match_confidence: Some(rust_decimal::Decimal::from(1)),
                    match_attempts: 1,
                    parsed_resolution: quality.resolution,
                    parsed_codec: quality.codec,
                    parsed_source: quality.source,
                    parsed_audio: quality.audio,
                };

                if db.pending_file_matches().create(input).await.is_ok() {
                    matches_created += 1;
                }
            }
        }

        tracing::info!(
            torrent_name = %torrent_info.name,
            album_id = %album_id,
            album_name = %album_name,
            matches_created = matches_created,
            "Created file matches for album"
        );
    }

    if let Some(movie_id) = movie_id {
        // Find largest video file
        let video_files: Vec<_> = torrent_info
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| is_video_file(&f.path))
            .collect();

        if let Some((idx, file)) = video_files.iter().max_by_key(|(_, f)| f.size) {
            let quality = filename_parser::parse_quality(&file.path);

            let input = CreatePendingFileMatch {
                user_id,
                source_path: file.path.clone(),
                source_type: "torrent".to_string(),
                source_id: Some(torrent_record.id),
                source_file_index: Some(*idx as i32),
                file_size: file.size as i64,
                target: Some(MatchTarget::Movie(movie_id)),
                unmatched_reason: None,
                match_type: "manual".to_string(),
                match_confidence: Some(rust_decimal::Decimal::from(1)),
                match_attempts: 1,
                parsed_resolution: quality.resolution,
                parsed_codec: quality.codec,
                parsed_source: quality.source,
                parsed_audio: quality.audio,
            };

            if db.pending_file_matches().create(input).await.is_ok() {
                // Status is derived from pending_file_matches - no direct update needed
                tracing::info!(
                    torrent_id = %torrent_record.id,
                    movie_id = %movie_id,
                    "Created file match for movie"
                );
            }
        }
    }

    if let Some(episode_id) = episode_id {
        // Find largest video file
        let video_files: Vec<_> = torrent_info
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| is_video_file(&f.path))
            .collect();

        if let Some((idx, file)) = video_files.iter().max_by_key(|(_, f)| f.size) {
            let quality = filename_parser::parse_quality(&file.path);

            let input = CreatePendingFileMatch {
                user_id,
                source_path: file.path.clone(),
                source_type: "torrent".to_string(),
                source_id: Some(torrent_record.id),
                source_file_index: Some(*idx as i32),
                file_size: file.size as i64,
                target: Some(MatchTarget::Episode(episode_id)),
                unmatched_reason: None,
                match_type: "manual".to_string(),
                match_confidence: Some(rust_decimal::Decimal::from(1)),
                match_attempts: 1,
                parsed_resolution: quality.resolution,
                parsed_codec: quality.codec,
                parsed_source: quality.source,
                parsed_audio: quality.audio,
            };

            if db.pending_file_matches().create(input).await.is_ok() {
                // Episode status is now derived from media_file_id presence
                // "downloading" status is determined by having pending_file_matches

                tracing::info!(
                    torrent_id = %torrent_record.id,
                    episode_id = %episode_id,
                    "Created file match for episode"
                );
            }
        }
    }
}
