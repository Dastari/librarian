use async_graphql::{Context, Object, Result, SimpleObject};
use tracing::{info, warn};

use crate::db::Database;
use crate::graphql::entities::Movie;
use crate::services::artwork_choice::{self, ArtworkEntityKind, ArtworkKind, ArtworkSlot};
use crate::services::graphql::auth::AuthExt;
use crate::services::metadata::providers::MetadataService;
use crate::services::{ArtworkService, ServicesManager};

/// What became of a change of artwork.
#[derive(SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct ArtworkChangeResult {
    pub success: bool,
    pub error: Option<String>,
}

impl ArtworkChangeResult {
    fn ok() -> Self {
        Self {
            success: true,
            error: None,
        }
    }

    fn failed(error: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(error.into()),
        }
    }
}

/// The slot, its entity checked to exist and be changeable by the caller, and the artwork cache.
async fn changeable(
    ctx: &Context<'_>,
    entity_type: ArtworkEntityKind,
    entity_id: &str,
    kind: ArtworkKind,
) -> std::result::Result<(ArtworkSlot, ArtworkService), String> {
    let user = ctx.require_member().map_err(|error| error.message)?;
    let slot = ArtworkSlot::new(entity_type, kind)
        .ok_or_else(|| "That kind of title has no picture of that kind".to_string())?;
    let db = ctx.data_unchecked::<Database>();
    let state = artwork_choice::slot_state(db, slot, entity_id)
        .await
        .map_err(|error| error.to_string())?
        .filter(|state| artwork_choice::may_change(user, slot, state))
        .ok_or_else(|| "Not found".to_string())?;
    drop(state);
    let services = ctx.data_unchecked::<std::sync::Arc<ServicesManager>>();
    let storage = services
        .get_storage()
        .await
        .ok_or_else(|| "Object storage service unavailable".to_string())?;
    Ok((slot, ArtworkService::new(db.clone(), storage)))
}

#[derive(Default)]
pub struct ArtworkMutations;

#[Object(name = "ArtworkMutation")]
impl ArtworkMutations {
    /// Show one of the pictures the provider lists (`artworkOptions`) and lock it, so a metadata
    /// refresh keeps it.
    #[graphql(name = "chooseArtwork")]
    async fn choose_artwork(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "entityType")] entity_type: ArtworkEntityKind,
        #[graphql(name = "entityId")] entity_id: String,
        kind: ArtworkKind,
        url: String,
    ) -> Result<ArtworkChangeResult> {
        let (slot, artwork) = match changeable(ctx, entity_type, &entity_id, kind).await {
            Ok(found) => found,
            Err(error) => return Ok(ArtworkChangeResult::failed(error)),
        };
        if !slot.has_provider() {
            return Ok(ArtworkChangeResult::failed(
                "A playlist's cover can only be uploaded",
            ));
        }
        let db = ctx.data_unchecked::<Database>();
        let metadata = ctx.data_unchecked::<std::sync::Arc<MetadataService>>();
        let providers =
            match artwork_choice::options::provider_options(metadata, db, slot, &entity_id).await {
                Ok(providers) => providers,
                Err(error) => return Ok(ArtworkChangeResult::failed(error.to_string())),
            };
        match artwork_choice::choose(db, &artwork, &providers, slot, &entity_id, &url).await {
            Ok(()) => Ok(ArtworkChangeResult::ok()),
            Err(error) => {
                warn!(
                    entity_type = slot.entity_type(),
                    entity_id = %entity_id,
                    artwork_type = slot.artwork_type(),
                    error = %error,
                    "Could not use the chosen {} {} picture",
                    slot.entity_type(),
                    slot.artwork_type()
                );
                Ok(ArtworkChangeResult::failed(format!(
                    "Couldn't use that picture: {error}"
                )))
            }
        }
    }

    /// Unlock a picture and put back what a metadata refresh would choose (or nothing, where the
    /// provider has nothing, as for a playlist or a movie's logo).
    #[graphql(name = "revertArtwork")]
    async fn revert_artwork(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "entityType")] entity_type: ArtworkEntityKind,
        #[graphql(name = "entityId")] entity_id: String,
        kind: ArtworkKind,
    ) -> Result<ArtworkChangeResult> {
        let (slot, artwork) = match changeable(ctx, entity_type, &entity_id, kind).await {
            Ok(found) => found,
            Err(error) => return Ok(ArtworkChangeResult::failed(error)),
        };
        let db = ctx.data_unchecked::<Database>();
        let providers = if slot.has_provider() {
            let metadata = ctx.data_unchecked::<std::sync::Arc<MetadataService>>();
            match artwork_choice::options::provider_options(metadata, db, slot, &entity_id).await {
                Ok(providers) => Some(providers),
                Err(error) => return Ok(ArtworkChangeResult::failed(error.to_string())),
            }
        } else {
            None
        };
        match artwork_choice::revert(db, &artwork, providers.as_ref(), slot, &entity_id).await {
            Ok(()) => Ok(ArtworkChangeResult::ok()),
            Err(error) => Ok(ArtworkChangeResult::failed(error.to_string())),
        }
    }

    /// Recache artwork for a specific movie
    #[graphql(name = "recacheMovieArtwork")]
    async fn recache_movie_artwork(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "movieId")] movie_id: String,
    ) -> Result<bool> {
        ctx.require_admin()?;
        let db = ctx.data_unchecked::<Database>();
        let services = ctx.data_unchecked::<std::sync::Arc<ServicesManager>>();

        let movie = Movie::get(db.pool(), &movie_id).await?;

        if let Some(movie) = movie {
            let Some(storage) = services.get_storage().await else {
                return Err(async_graphql::Error::new(
                    "Object storage service unavailable",
                ));
            };
            let artwork_service = ArtworkService::new(db.clone(), storage);

            info!(
                movie_id = %movie_id,
                "Recaching movie artwork requested: movie_id={}",
                movie_id
            );

            // Cache in background
            tokio::spawn(async move {
                artwork_service
                    .cache_movie_artwork(
                        &movie_id,
                        movie.poster_url.as_deref().filter(|s| !s.is_empty()),
                        movie.backdrop_url.as_deref().filter(|s| !s.is_empty()),
                    )
                    .await;
            });

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Recache artwork for all movies (runs in background)
    #[graphql(name = "recacheAllMovieArtwork")]
    async fn recache_all_movie_artwork(&self, ctx: &Context<'_>) -> Result<i64> {
        ctx.require_admin()?;
        let db = ctx.data_unchecked::<Database>();
        let services = ctx.data_unchecked::<std::sync::Arc<ServicesManager>>();

        let movies = Movie::query(db.pool())
            .fetch_all()
            .await?
            .into_iter()
            .filter(|movie| movie.poster_url.is_some() || movie.backdrop_url.is_some())
            .collect::<Vec<_>>();

        let count = movies.len() as i64;
        let db_clone = db.clone();
        let Some(storage) = services.get_storage().await else {
            return Err(async_graphql::Error::new(
                "Object storage service unavailable",
            ));
        };

        info!(
            count = count,
            "Recaching artwork for all movies requested: movie_count={}", count
        );

        // Process in background
        tokio::spawn(async move {
            let artwork_service = ArtworkService::new(db_clone, storage);

            for movie in movies {
                artwork_service
                    .cache_movie_artwork(
                        &movie.id,
                        movie.poster_url.as_deref().filter(|s| !s.is_empty()),
                        movie.backdrop_url.as_deref().filter(|s| !s.is_empty()),
                    )
                    .await;

                // Small delay to avoid overwhelming the server
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            info!("Finished recaching artwork for all movies background job");
        });

        Ok(count)
    }
}
