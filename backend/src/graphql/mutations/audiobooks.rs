use super::prelude::*;

#[derive(Default)]
pub struct AudiobookMutations;

#[Object]
impl AudiobookMutations {
    /// Add an audiobook to a library from OpenLibrary
    async fn add_audiobook(
        &self,
        ctx: &Context<'_>,
        input: AddAudiobookInput,
    ) -> Result<AudiobookResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>().clone();
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>().clone();

        let library_id = Uuid::parse_str(&input.library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        use crate::services::metadata::AddAudiobookOptions;
        match metadata
            .add_audiobook_from_provider(AddAudiobookOptions {
                openlibrary_id: input.openlibrary_id.clone(),
                library_id,
                user_id,
                monitored: true,
            })
            .await
        {
            Ok(record) => {
                tracing::info!(
                    user_id = %user.user_id,
                    audiobook_title = %record.title,
                    audiobook_id = %record.id,
                    library_id = %library_id,
                    "User added audiobook: {}",
                    record.title
                );

                // Trigger immediate auto-hunt if the library has auto_hunt enabled
                {
                    let db_clone = db.clone();
                    let audiobook_record = record.clone();
                    let torrent_svc = torrent_service.clone();
                    let lib_id = library_id;

                    tokio::spawn(async move {
                        // Check if library has auto_hunt enabled
                        let library = match db_clone.libraries().get_by_id(lib_id).await {
                            Ok(Some(lib)) => lib,
                            Ok(None) => {
                                tracing::warn!(library_id = %lib_id, "Library not found for auto-hunt");
                                return;
                            }
                            Err(e) => {
                                tracing::warn!(library_id = %lib_id, error = %e, "Failed to get library for auto-hunt");
                                return;
                            }
                        };

                        if !library.auto_hunt {
                            tracing::debug!(
                                library_id = %lib_id,
                                audiobook_title = %audiobook_record.title,
                                "Library does not have auto_hunt enabled, skipping immediate hunt"
                            );
                            return;
                        }

                        tracing::info!(
                            audiobook_id = %audiobook_record.id,
                            audiobook_title = %audiobook_record.title,
                            "Triggering immediate auto-hunt for newly added audiobook"
                        );

                        // Get encryption key and create IndexerManager
                        let encryption_key = match db_clone
                            .settings()
                            .get_or_create_indexer_encryption_key()
                            .await
                        {
                            Ok(key) => key,
                            Err(e) => {
                                tracing::warn!(error = %e, "Failed to get encryption key for auto-hunt");
                                return;
                            }
                        };

                        let indexer_manager = match crate::indexer::manager::IndexerManager::new(
                            db_clone.clone(),
                            &encryption_key,
                        )
                        .await
                        {
                            Ok(mgr) => std::sync::Arc::new(mgr),
                            Err(e) => {
                                tracing::warn!(error = %e, "Failed to create IndexerManager for auto-hunt");
                                return;
                            }
                        };

                        // Load user's indexers
                        if let Err(e) = indexer_manager.load_user_indexers(user_id).await {
                            tracing::warn!(user_id = %user_id, error = %e, "Failed to load indexers for auto-hunt");
                            return;
                        }

                        // Run hunt for this specific audiobook
                        match crate::jobs::auto_hunt::hunt_single_audiobook(
                            &db_clone,
                            &audiobook_record,
                            &library,
                            &torrent_svc,
                            &indexer_manager,
                        )
                        .await
                        {
                            Ok(result) => {
                                if result.downloaded > 0 {
                                    tracing::info!(
                                        audiobook_title = %audiobook_record.title,
                                        "Immediate auto-hunt successful, audiobook download started"
                                    );
                                } else if result.matched > 0 {
                                    tracing::info!(
                                        audiobook_title = %audiobook_record.title,
                                        "Found matching releases but download failed"
                                    );
                                } else {
                                    tracing::info!(
                                        audiobook_title = %audiobook_record.title,
                                        "No matching releases found for immediate auto-hunt"
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    audiobook_title = %audiobook_record.title,
                                    error = %e,
                                    "Immediate auto-hunt failed"
                                );
                            }
                        }
                    });
                }

                Ok(AudiobookResult {
                    success: true,
                    audiobook: Some(Audiobook::from(record)),
                    error: None,
                })
            }
            Err(e) => Ok(AudiobookResult {
                success: false,
                audiobook: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete an audiobook from a library
    async fn delete_audiobook(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let audiobook_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid audiobook ID: {}", e)))?;

        // Verify audiobook exists
        let audiobook =
            db.audiobooks().get_by_id(audiobook_id).await.map_err(|e| {
                async_graphql::Error::new(format!("Failed to get audiobook: {}", e))
            })?;

        if audiobook.is_none() {
            return Ok(MutationResult {
                success: false,
                error: Some("Audiobook not found".to_string()),
            });
        }

        // Delete the audiobook and all associated data
        match db.audiobooks().delete(audiobook_id).await {
            Ok(deleted) => {
                if deleted {
                    tracing::info!(audiobook_id = %audiobook_id, "Deleted audiobook");
                    Ok(MutationResult {
                        success: true,
                        error: None,
                    })
                } else {
                    Ok(MutationResult {
                        success: false,
                        error: Some("Audiobook not found".to_string()),
                    })
                }
            }
            Err(e) => {
                tracing::error!(audiobook_id = %audiobook_id, error = %e, "Failed to delete audiobook");
                Ok(MutationResult {
                    success: false,
                    error: Some(format!("Failed to delete audiobook: {}", e)),
                })
            }
        }
    }
}
