use async_graphql::{Context, Object, Result};
use tracing::info;

use crate::db::Database;
use crate::graphql::entities::Movie;
use crate::services::ArtworkService;

#[derive(Default)]
pub struct ArtworkMutations;

#[Object(name = "ArtworkMutation")]
impl ArtworkMutations {
    /// Recache artwork for a specific movie
    #[graphql(name = "RecacheMovieArtwork")]
    async fn recache_movie_artwork(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "MovieId")] movie_id: String,
    ) -> Result<bool> {
        let db = ctx.data_unchecked::<Database>();

        let movie = Movie::get(db.pool(), &movie_id).await?;

        if let Some(movie) = movie {
            let artwork_service = ArtworkService::new(db.clone());

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
    #[graphql(name = "RecacheAllMovieArtwork")]
    async fn recache_all_movie_artwork(&self, ctx: &Context<'_>) -> Result<i64> {
        let db = ctx.data_unchecked::<Database>();

        let movies = Movie::query(db.pool())
            .fetch_all()
            .await?
            .into_iter()
            .filter(|movie| movie.poster_url.is_some() || movie.backdrop_url.is_some())
            .collect::<Vec<_>>();

        let count = movies.len() as i64;
        let db_clone = db.clone();

        info!(
            count = count,
            "Recaching artwork for all movies requested: movie_count={}", count
        );

        // Process in background
        tokio::spawn(async move {
            let artwork_service = ArtworkService::new(db_clone);

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
