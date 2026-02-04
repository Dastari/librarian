use async_graphql::{Context, Object, Result};
use tracing::info;

use crate::db::Database;
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

        // Fetch movie details
        let movie: Option<(String, String)> =
            sqlx::query_as("SELECT poster_url, backdrop_url FROM movies WHERE id = ?1")
                .bind(&movie_id)
                .fetch_optional(db)
                .await?;

        if let Some((poster_url, backdrop_url)) = movie {
            let artwork_service = ArtworkService::new(db.clone());

            info!(movie_id = %movie_id, "Recaching movie artwork");

            // Cache in background
            tokio::spawn(async move {
                artwork_service
                    .cache_movie_artwork(
                        &movie_id,
                        Some(poster_url.as_str()).filter(|s| !s.is_empty()),
                        Some(backdrop_url.as_str()).filter(|s| !s.is_empty()),
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

        // Fetch all movies with artwork URLs
        let movies: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT id, poster_url, backdrop_url FROM movies WHERE poster_url IS NOT NULL OR backdrop_url IS NOT NULL"
        )
        .fetch_all(db)
        .await?;

        let count = movies.len() as i64;
        let db_clone = db.clone();

        info!(count = count, "Recaching artwork for all movies");

        // Process in background
        tokio::spawn(async move {
            let artwork_service = ArtworkService::new(db_clone);

            for (movie_id, poster_url, backdrop_url) in movies {
                artwork_service
                    .cache_movie_artwork(
                        &movie_id,
                        Some(poster_url.as_str()).filter(|s| !s.is_empty()),
                        Some(backdrop_url.as_str()).filter(|s| !s.is_empty()),
                    )
                    .await;

                // Small delay to avoid overwhelming the server
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            info!("Finished recaching artwork for all movies");
        });

        Ok(count)
    }
}
