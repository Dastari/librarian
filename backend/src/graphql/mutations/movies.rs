use super::prelude::*;
use tokio::sync::broadcast;

#[derive(Default)]
pub struct MovieMutations;

/// Helper to broadcast library change events
async fn broadcast_library_changed(ctx: &Context<'_>, library_id: Uuid) {
    if let Ok(tx) = ctx.data::<broadcast::Sender<LibraryChangedEvent>>() {
        let db = ctx.data_unchecked::<Database>();
        if let Ok(Some(lib)) = db.libraries().get_by_id(library_id).await {
            let _ = tx.send(LibraryChangedEvent {
                change_type: LibraryChangeType::Updated,
                library_id: library_id.to_string(),
                library_name: Some(lib.name.clone()),
                library: Some(Library::from_db(lib)),
            });
        }
    }
}

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

        if !metadata.has_tmdb().await {
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

                // Broadcast library change event for UI reactivity
                broadcast_library_changed(ctx, lib_id).await;

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

        // Get the library_id before deleting so we can broadcast the change
        let library_id = db.movies().get_by_id(movie_id).await.ok().flatten().map(|m| m.library_id);

        match db.movies().delete(movie_id).await {
            Ok(true) => {
                // Broadcast library change event for UI reactivity
                if let Some(lib_id) = library_id {
                    broadcast_library_changed(ctx, lib_id).await;
                }
                Ok(MutationResult {
                    success: true,
                    error: None,
                })
            }
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

    /// Refresh metadata for a movie (re-fetches from TMDB and caches artwork)
    async fn refresh_movie(&self, ctx: &Context<'_>, id: String) -> Result<MovieResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let movie_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid movie ID: {}", e)))?;

        let movie = db
            .movies()
            .get_by_id(movie_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Movie not found"))?;

        // Get TMDB ID
        let tmdb_id = match movie.tmdb_id {
            Some(id) => id as u32,
            None => {
                return Ok(MovieResult {
                    success: false,
                    movie: None,
                    error: Some("No TMDB ID found for movie".to_string()),
                });
            }
        };

        // Fetch fresh movie details from TMDB
        let movie_details = metadata
            .get_movie(tmdb_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Cache artwork if artwork service is available
        let (cached_poster_url, cached_backdrop_url) =
            if let Some(artwork_service) = metadata.artwork_service() {
                let entity_id = format!("{}_{}", tmdb_id, movie.library_id);

                let poster_url = artwork_service
                    .cache_image_optional(
                        movie_details.poster_url.as_deref(),
                        crate::services::artwork::ArtworkType::Poster,
                        "movie",
                        &entity_id,
                    )
                    .await;

                let backdrop_url = artwork_service
                    .cache_image_optional(
                        movie_details.backdrop_url.as_deref(),
                        crate::services::artwork::ArtworkType::Backdrop,
                        "movie",
                        &entity_id,
                    )
                    .await;

                tracing::info!(
                    movie_id = %movie_id,
                    movie_title = %movie.title,
                    poster_cached = poster_url.is_some(),
                    backdrop_cached = backdrop_url.is_some(),
                    "Refreshed movie artwork caching"
                );

                (poster_url, backdrop_url)
            } else {
                (
                    movie_details.poster_url.clone(),
                    movie_details.backdrop_url.clone(),
                )
            };

        // Update movie metadata including artwork
        let update = crate::db::UpdateMovie {
            title: Some(movie_details.title),
            original_title: movie_details.original_title,
            overview: movie_details.overview,
            tagline: movie_details.tagline,
            runtime: movie_details.runtime,
            genres: Some(movie_details.genres),
            director: movie_details.director,
            cast_names: Some(movie_details.cast_names),
            poster_url: cached_poster_url,
            backdrop_url: cached_backdrop_url,
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
                error: Some("Movie not found after update".to_string()),
            }),
            Err(e) => Ok(MovieResult {
                success: false,
                movie: None,
                error: Some(e.to_string()),
            }),
        }
    }
}
