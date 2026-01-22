use super::prelude::*;

#[derive(Default)]
pub struct MovieMutations;

#[Object]
impl MovieMutations {
    /// Add a movie to a library
    async fn add_movie(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        input: AddMovieInput,
    ) -> Result<MovieResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>().clone();
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>().clone();

        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        if !metadata.has_tmdb() {
            return Ok(MovieResult {
                success: false,
                movie: None,
                error: Some("TMDB API key not configured".to_string()),
            });
        }

        let is_monitored = input.monitored.unwrap_or(true);

        match metadata
            .add_movie_from_provider(crate::services::AddMovieOptions {
                provider: crate::services::MetadataProvider::Tmdb,
                provider_id: input.tmdb_id as u32,
                library_id: lib_id,
                user_id,
                monitored: is_monitored,
            })
            .await
        {
            Ok(record) => {
                tracing::info!(
                    user_id = %user.user_id,
                    movie_title = %record.title,
                    movie_id = %record.id,
                    library_id = %lib_id,
                    "User added movie: {}",
                    record.title
                );

                // Trigger immediate auto-hunt if the library has auto_hunt enabled and movie is monitored
                if is_monitored {
                    let db_clone = db.clone();
                    let movie_record = record.clone();
                    let torrent_svc = torrent_service.clone();

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
                                movie_title = %movie_record.title,
                                "Library does not have auto_hunt enabled, skipping immediate hunt"
                            );
                            return;
                        }

                        tracing::info!(
                            movie_id = %movie_record.id,
                            movie_title = %movie_record.title,
                            "Triggering immediate auto-hunt for newly added movie"
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

                        // Run hunt for this specific movie
                        match crate::jobs::auto_hunt::hunt_single_movie(
                            &db_clone,
                            &movie_record,
                            &library,
                            &torrent_svc,
                            &indexer_manager,
                        )
                        .await
                        {
                            Ok(result) => {
                                if result.downloaded > 0 {
                                    tracing::info!(
                                        movie_title = %movie_record.title,
                                        "Immediate auto-hunt successful, download started"
                                    );
                                } else if result.matched > 0 {
                                    tracing::info!(
                                        movie_title = %movie_record.title,
                                        "Found matching releases but download failed"
                                    );
                                } else {
                                    tracing::info!(
                                        movie_title = %movie_record.title,
                                        "No matching releases found for immediate auto-hunt"
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    movie_title = %movie_record.title,
                                    error = %e,
                                    "Immediate auto-hunt failed"
                                );
                            }
                        }
                    });
                }

                Ok(MovieResult {
                    success: true,
                    movie: Some(movie_record_to_graphql(record)),
                    error: None,
                })
            }
            Err(e) => Ok(MovieResult {
                success: false,
                movie: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update a movie
    async fn update_movie(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateMovieInput,
    ) -> Result<MovieResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let movie_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid movie ID: {}", e)))?;

        // Build update
        let update = crate::db::UpdateMovie {
            monitored: input.monitored,
            ..Default::default()
        };

        match db.movies().update(movie_id, update).await {
            Ok(Some(record)) => Ok(MovieResult {
                success: true,
                movie: Some(movie_record_to_graphql(record)),
                error: None,
            }),
            Ok(None) => Ok(MovieResult {
                success: false,
                movie: None,
                error: Some("Movie not found".to_string()),
            }),
            Err(e) => Ok(MovieResult {
                success: false,
                movie: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete a movie from a library
    async fn delete_movie(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let movie_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid movie ID: {}", e)))?;

        match db.movies().delete(movie_id).await {
            Ok(true) => Ok(MutationResult {
                success: true,
                error: None,
            }),
            Ok(false) => Ok(MutationResult {
                success: false,
                error: Some("Movie not found".to_string()),
            }),
            Err(e) => Ok(MutationResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }
}
