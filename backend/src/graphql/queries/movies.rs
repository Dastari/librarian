use super::prelude::*;

/// Helper to populate download_progress for movies with status "downloading"
async fn populate_movie_download_progress(db: &Database, movies: &mut [Movie]) {
    let torrent_files_repo = db.torrent_files();
    for movie in movies.iter_mut() {
        if movie.download_status == DownloadStatus::Downloading {
            if let Ok(movie_id) = Uuid::parse_str(&movie.id) {
                if let Ok(Some(progress)) = torrent_files_repo
                    .get_download_progress_for_movie(movie_id)
                    .await
                {
                    movie.download_progress = Some(progress);
                }
            }
        }
    }
}

#[derive(Default)]
pub struct MovieQueries;

#[Object]
impl MovieQueries {
    /// Get all movies for the current user (across all libraries)
    async fn all_movies(&self, ctx: &Context<'_>) -> Result<Vec<Movie>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .movies()
            .list_by_user(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let mut movies: Vec<Movie> = records.into_iter().map(movie_record_to_graphql).collect();
        populate_movie_download_progress(db, &mut movies).await;
        Ok(movies)
    }

    /// Get all movies in a library
    async fn movies(&self, ctx: &Context<'_>, library_id: String) -> Result<Vec<Movie>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let records = db
            .movies()
            .list_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let mut movies: Vec<Movie> = records.into_iter().map(movie_record_to_graphql).collect();
        populate_movie_download_progress(db, &mut movies).await;
        Ok(movies)
    }

    /// Get movies in a library with cursor-based pagination and filtering
    async fn movies_connection(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<MovieWhereInput>,
        order_by: Option<MovieOrderByInput>,
    ) -> Result<MovieConnection> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        // Parse pagination args
        let (offset, limit) =
            parse_pagination_args(first, after).map_err(|e| async_graphql::Error::new(e))?;

        // Build filter conditions
        let title_filter = r#where
            .as_ref()
            .and_then(|w| w.title.as_ref().and_then(|f| f.contains.clone()));
        let year_filter = r#where
            .as_ref()
            .and_then(|w| w.year.as_ref().and_then(|f| f.eq));
        let monitored_filter = r#where
            .as_ref()
            .and_then(|w| w.monitored.as_ref().and_then(|f| f.eq));
        let has_file_filter = r#where
            .as_ref()
            .and_then(|w| w.has_file.as_ref().and_then(|f| f.eq));
        let download_status_filter = r#where
            .as_ref()
            .and_then(|w| w.download_status.as_ref().and_then(|f| f.eq.clone()));

        // Determine sort field and direction
        let sort_field = order_by
            .as_ref()
            .and_then(|o| o.field)
            .unwrap_or(MovieSortField::SortTitle);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(OrderDirection::Asc);

        // Get paginated movies from database
        // Note: download_status is now derived from media_file_id, not stored
        let (records, total) = db
            .movies()
            .list_by_library_paginated(
                lib_id,
                offset,
                limit,
                title_filter.as_deref(),
                year_filter,
                monitored_filter,
                has_file_filter,
                &sort_field_to_column(sort_field),
                sort_dir == OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let mut movies: Vec<Movie> = records.into_iter().map(movie_record_to_graphql).collect();
        populate_movie_download_progress(db, &mut movies).await;
        let connection = Connection::from_items(movies, offset, limit, total);

        Ok(MovieConnection::from_connection(connection))
    }

    /// Get a specific movie by ID
    async fn movie(&self, ctx: &Context<'_>, id: String) -> Result<Option<Movie>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let movie_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid movie ID: {}", e)))?;

        let record = db
            .movies()
            .get_by_id(movie_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        match record {
            Some(r) => {
                let mut movie = movie_record_to_graphql(r);
                if movie.download_status == DownloadStatus::Downloading {
                    if let Ok(Some(progress)) = db
                        .torrent_files()
                        .get_download_progress_for_movie(movie_id)
                        .await
                    {
                        movie.download_progress = Some(progress);
                    }
                }
                Ok(Some(movie))
            }
            None => Ok(None),
        }
    }

    /// Search for movies on TMDB
    async fn search_movies(
        &self,
        ctx: &Context<'_>,
        query: String,
        year: Option<i32>,
    ) -> Result<Vec<MovieSearchResult>> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        if !metadata.has_tmdb().await {
            return Err(async_graphql::Error::new(
                "TMDB API key not configured. Add tmdb_api_key to settings.",
            ));
        }

        let results = metadata
            .search_movies(&query, year)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|m| MovieSearchResult {
                provider: "tmdb".to_string(),
                provider_id: m.provider_id as i32,
                title: m.title,
                original_title: m.original_title,
                year: m.year,
                overview: m.overview,
                poster_url: m.poster_url,
                backdrop_url: m.backdrop_url,
                imdb_id: m.imdb_id,
                vote_average: m.vote_average,
                popularity: m.popularity,
            })
            .collect())
    }
}
