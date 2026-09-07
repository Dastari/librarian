use async_graphql::{Context, Object, Result};
use tracing::info;

use crate::db::Database;
use crate::graphql::entities::Movie;
use crate::services::graphql::auth::AuthExt;
use crate::services::{ArtworkService, ServicesManager};

#[derive(Default)]
pub struct ArtworkMutations;

#[Object(name = "ArtworkMutation")]
impl ArtworkMutations {
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
