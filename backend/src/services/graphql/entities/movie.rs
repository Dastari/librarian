use async_graphql::{Context, InputObject, Object, SimpleObject};
use std::sync::Arc;

use macros::{GraphQLEntity, GraphQLOperations};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::services::graphql::AuthUser;

use super::common::{ContentStatus, ContentType, calculate_content_status};
use super::media_file::MediaFile;

// Re-export types used for movie creation from metadata
pub use crate::services::metadata::providers::{
    CreateMovieFromMetadataOptions, MovieDetails as MovieMetadataDetails,
};

#[derive(GraphQLEntity, GraphQLOperations, SimpleObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(name = "Movie", complex)]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "movies",
    plural = "Movies",
    default_sort = "title",
    notify = "libraries"
)]
pub struct Movie {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "LibraryId")]
    #[filterable(type = "string")]
    pub library_id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "Title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: String,

    #[graphql(name = "SortTitle")]
    #[sortable]
    pub sort_title: Option<String>,

    #[graphql(name = "OriginalTitle")]
    pub original_title: Option<String>,

    #[graphql(name = "Year")]
    #[filterable(type = "number")]
    #[sortable]
    pub year: Option<i32>,

    #[graphql(name = "TmdbId")]
    #[filterable(type = "number")]
    pub tmdb_id: Option<i32>,

    #[graphql(name = "ImdbId")]
    #[filterable(type = "string")]
    pub imdb_id: Option<String>,

    #[graphql(name = "Overview")]
    pub overview: Option<String>,

    #[graphql(name = "Tagline")]
    pub tagline: Option<String>,

    #[graphql(name = "Runtime")]
    #[filterable(type = "number")]
    #[sortable]
    pub runtime: Option<i32>,

    #[graphql(name = "Genres")]
    #[json_field]
    pub genres: Vec<String>,

    #[graphql(name = "Director")]
    #[filterable(type = "string")]
    pub director: Option<String>,

    #[graphql(name = "CastNames")]
    #[json_field]
    pub cast_names: Vec<String>,

    #[graphql(name = "ProductionCountries")]
    #[json_field]
    pub production_countries: Vec<String>,

    #[graphql(name = "SpokenLanguages")]
    #[json_field]
    pub spoken_languages: Vec<String>,

    #[graphql(name = "TmdbRating")]
    pub tmdb_rating: Option<String>,

    #[graphql(name = "TmdbVoteCount")]
    #[filterable(type = "number")]
    pub tmdb_vote_count: Option<i32>,

    #[graphql(skip)]
    pub poster_url: Option<String>,

    #[graphql(skip)]
    pub backdrop_url: Option<String>,

    #[graphql(name = "CollectionId")]
    #[filterable(type = "number")]
    pub collection_id: Option<i32>,

    #[graphql(name = "CollectionName")]
    #[filterable(type = "string")]
    pub collection_name: Option<String>,

    #[graphql(name = "CollectionPosterUrl")]
    pub collection_poster_url: Option<String>,

    #[graphql(name = "ReleaseDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub release_date: Option<String>,

    #[graphql(name = "Certification")]
    #[filterable(type = "string")]
    pub certification: Option<String>,

    #[graphql(name = "Monitored")]
    #[filterable(type = "boolean")]
    pub monitored: bool,

    #[graphql(name = "TmdbStatus")]
    #[filterable(type = "string")]
    pub tmdb_status: Option<String>,

    #[graphql(name = "Wanted")]
    #[filterable(type = "boolean")]
    pub wanted: bool,

    #[graphql(name = "DownloadStatus")]
    #[filterable(type = "string")]
    pub download_status: Option<String>,

    #[graphql(name = "HasFile")]
    #[filterable(type = "boolean")]
    pub has_file: bool,

    #[graphql(name = "MediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: Option<String>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,

    #[graphql(name = "MediaFile")]
    #[relation(target = "MediaFile", from = "media_file_id", to = "id")]
    pub media_file: Option<MediaFile>,
}

// ============================================================================
// Movie ComplexObject Resolvers (computed fields)
// ============================================================================

#[async_graphql::ComplexObject]
impl Movie {
    /// Get poster URL, preferring cached version if available
    #[graphql(name = "PosterUrl")]
    async fn poster_url_resolver(&self, ctx: &async_graphql::Context<'_>) -> Option<String> {
        let db = ctx.data_unchecked::<crate::db::Database>();
        let artwork_service = crate::services::ArtworkService::new(db.clone());

        artwork_service
            .get_artwork_url("movie", &self.id, "poster", self.poster_url.as_deref())
            .await
    }

    /// Get backdrop URL, preferring cached version if available
    #[graphql(name = "BackdropUrl")]
    async fn backdrop_url_resolver(&self, ctx: &async_graphql::Context<'_>) -> Option<String> {
        let db = ctx.data_unchecked::<crate::db::Database>();
        let artwork_service = crate::services::ArtworkService::new(db.clone());

        artwork_service
            .get_artwork_url("movie", &self.id, "backdrop", self.backdrop_url.as_deref())
            .await
    }

    /// Computed status based on playback, file availability, and download state
    ///
    /// Returns one of: PLAYING, PAUSED, AVAILABLE, DOWNLOADING, WANTED, MISSING
    #[graphql(name = "Status")]
    async fn status(&self, ctx: &Context<'_>) -> ContentStatus {
        let db = match ctx.data::<Database>() {
            Ok(db) => db,
            Err(_) => return ContentStatus::Missing,
        };

        let user_id = match ctx.data::<AuthUser>() {
            Ok(user) => user.user_id.clone(),
            Err(_) => {
                return if self.media_file_id.is_some() {
                    ContentStatus::Available
                } else if self.wanted {
                    ContentStatus::Wanted
                } else {
                    ContentStatus::Missing
                };
            }
        };

        calculate_content_status(
            db,
            ContentType::Movie,
            &self.id,
            &user_id,
            self.media_file_id.as_deref(),
            self.wanted,
        )
        .await
    }
}

// ============================================================================
// Movie Internal Create Methods
// ============================================================================

impl Movie {
    /// Create a movie from metadata details (e.g., from TMDB)
    ///
    /// This is the single source of truth for creating movies from external metadata.
    /// Both GraphQL resolvers and internal services should use this method.
    ///
    /// This method:
    /// 1. Generates a new UUID
    /// 2. Inserts the movie into the database
    /// 3. Returns the created movie
    ///
    /// Note: Subscription broadcasting should be handled by the caller if needed,
    /// since internal creates may not have access to the broadcast channels.
    pub async fn create_from_metadata(
        db: &crate::db::Database,
        details: &MovieMetadataDetails,
        options: CreateMovieFromMetadataOptions,
    ) -> anyhow::Result<Self> {
        let movie_id = uuid::Uuid::new_v4();

        // Serialize JSON fields
        let genres_json =
            serde_json::to_string(&details.genres).unwrap_or_else(|_| "[]".to_string());
        let cast_json =
            serde_json::to_string(&details.cast_names).unwrap_or_else(|_| "[]".to_string());
        let countries_json = serde_json::to_string(&details.production_countries)
            .unwrap_or_else(|_| "[]".to_string());
        let languages_json =
            serde_json::to_string(&details.spoken_languages).unwrap_or_else(|_| "[]".to_string());

        // Parse release date
        let release_date = details
            .release_date
            .as_ref()
            .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

        // Convert vote_average to Decimal string
        let tmdb_rating = details
            .vote_average
            .and_then(|v| rust_decimal::Decimal::from_f64_retain(v))
            .map(|d| d.to_string());

        // Insert into database
        // wanted = monitored (if user wants to auto-download, mark as wanted)
        sqlx::query(
            r#"
            INSERT INTO movies (
                id, library_id, user_id, title, original_title, year, tmdb_id, imdb_id,
                overview, tagline, runtime, genres, production_countries, spoken_languages,
                director, cast_names, tmdb_rating, tmdb_vote_count, poster_url, backdrop_url,
                collection_id, collection_name, collection_poster_url, release_date,
                certification, tmdb_status, monitored, wanted, has_file, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, 0, datetime('now'), datetime('now')
            )
            "#,
        )
        .bind(movie_id.to_string())
        .bind(options.library_id.to_string())
        .bind(options.user_id.to_string())
        .bind(&details.title)
        .bind(&details.original_title)
        .bind(details.year)
        .bind(details.provider_id as i32) // tmdb_id
        .bind(&details.imdb_id)
        .bind(&details.overview)
        .bind(&details.tagline)
        .bind(details.runtime)
        .bind(&genres_json)
        .bind(&countries_json)
        .bind(&languages_json)
        .bind(&details.director)
        .bind(&cast_json)
        .bind(&tmdb_rating)
        .bind(details.vote_count)
        .bind(&details.poster_url)
        .bind(&details.backdrop_url)
        .bind(details.collection_id)
        .bind(&details.collection_name)
        .bind(&details.collection_poster_url)
        .bind(release_date.map(|d| d.format("%Y-%m-%d").to_string()))
        .bind(&details.certification)
        .bind(&details.tmdb_status)
        .bind(options.monitored)
        .bind(options.monitored) // wanted = monitored
        .execute(db)
        .await?;

        // Fetch the created movie
        let movie = Self::get(db, &movie_id.to_string())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Movie not found after creation"))?;

        tracing::info!(
            movie_id = %movie.id,
            movie_title = %movie.title,
            library_id = %options.library_id,
            "Created movie from metadata"
        );

        Ok(movie)
    }
}

#[derive(Default)]
pub struct MovieCustomOperations;

#[async_graphql::Object]
impl MovieCustomOperations {
    /// Search for movies on TMDB
    #[graphql(name = "SearchMovies")]
    async fn search_movies(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Query")] query: String,
        #[graphql(name = "Year")] year: Option<i32>,
    ) -> async_graphql::Result<Vec<MovieSearchResultGql>> {
        use crate::graphql::auth::AuthExt;

        let _user = ctx.auth_user()?;
        let metadata =
            ctx.data_unchecked::<Arc<crate::services::metadata::providers::MetadataService>>();

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
            .map(|m| MovieSearchResultGql {
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

// ============================================================================
// Movie Metadata Mutations (TMDB integration)
// ============================================================================

/// Input for searching movies
#[derive(Debug, InputObject)]
#[graphql(name = "SearchMoviesInput")]
pub struct SearchMoviesInput {
    #[graphql(name = "Query")]
    pub query: String,
    #[graphql(name = "Year")]
    pub year: Option<i32>,
}

/// Input for adding a movie from TMDB
#[derive(Debug, InputObject)]
#[graphql(name = "AddMovieInput")]
pub struct AddMovieInput {
    /// TMDB movie ID
    #[graphql(name = "TmdbId")]
    pub tmdb_id: i32,
    /// Whether to monitor for releases (enables auto-download)
    #[graphql(name = "Monitored")]
    pub monitored: Option<bool>,
}

/// Movie search result from TMDB
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(name = "MovieSearchResult")]
pub struct MovieSearchResultGql {
    #[graphql(name = "Provider")]
    pub provider: String,
    #[graphql(name = "ProviderId")]
    pub provider_id: i32,
    #[graphql(name = "Title")]
    pub title: String,
    #[graphql(name = "OriginalTitle")]
    pub original_title: Option<String>,
    #[graphql(name = "Year")]
    pub year: Option<i32>,
    #[graphql(name = "Overview")]
    pub overview: Option<String>,
    #[graphql(name = "PosterUrl")]
    pub poster_url: Option<String>,
    #[graphql(name = "BackdropUrl")]
    pub backdrop_url: Option<String>,
    #[graphql(name = "ImdbId")]
    pub imdb_id: Option<String>,
    #[graphql(name = "VoteAverage")]
    pub vote_average: Option<f64>,
    #[graphql(name = "Popularity")]
    pub popularity: Option<f64>,
}

/// Result of movie operations
#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(name = "MovieOperationResult")]
pub struct MovieOperationResult {
    #[graphql(name = "Success")]
    pub success: bool,
    #[graphql(name = "Movie")]
    pub movie: Option<Movie>,
    #[graphql(name = "Error")]
    pub error: Option<String>,
}

/// Movie metadata mutations (TMDB integration)
#[derive(Default)]
pub struct MovieMetadataMutations;

#[Object]
impl MovieMetadataMutations {
    /// Add a movie to a library by fetching metadata from TMDB
    #[graphql(name = "AddMovie")]
    async fn add_movie(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "LibraryId")] library_id: String,
        #[graphql(name = "Input")] input: AddMovieInput,
    ) -> async_graphql::Result<MovieOperationResult> {
        use crate::graphql::auth::AuthExt;
        use crate::services::metadata::providers::{
            AddMovieOptions, MetadataProvider, MetadataService,
        };

        let user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let lib_id = uuid::Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = uuid::Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        if !metadata.has_tmdb().await {
            return Ok(MovieOperationResult {
                success: false,
                movie: None,
                error: Some("TMDB API key not configured".to_string()),
            });
        }

        let is_monitored = input.monitored.unwrap_or(true);

        match metadata
            .add_movie_from_provider(AddMovieOptions {
                provider: MetadataProvider::Tmdb,
                provider_id: input.tmdb_id as u32,
                library_id: lib_id,
                user_id,
                monitored: is_monitored,
            })
            .await
        {
            Ok(movie) => {
                tracing::info!(
                    user_id = %user.user_id,
                    movie_title = %movie.title,
                    movie_id = %movie.id,
                    library_id = %lib_id,
                    "User added movie from TMDB: {}",
                    movie.title
                );

                Ok(MovieOperationResult {
                    success: true,
                    movie: Some(movie),
                    error: None,
                })
            }
            Err(e) => Ok(MovieOperationResult {
                success: false,
                movie: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Refresh a movie's metadata and artwork from TMDB.
    #[graphql(name = "RefreshMovie")]
    async fn refresh_movie(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Id")] id: String,
    ) -> async_graphql::Result<MovieOperationResult> {
        use crate::graphql::auth::AuthExt;
        use crate::services::metadata::providers::MetadataService;

        let user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let user_id = uuid::Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match metadata.refresh_movie_from_provider(&id, user_id).await {
            Ok(movie) => Ok(MovieOperationResult {
                success: true,
                movie: Some(movie),
                error: None,
            }),
            Err(e) => Ok(MovieOperationResult {
                success: false,
                movie: None,
                error: Some(e.to_string()),
            }),
        }
    }
}

// // ============================================================================
// // ComplexObject Resolvers (relations + computed fields)
// // ============================================================================

// #[async_graphql::ComplexObject]
// impl MovieEntity {
//     /// Get the associated media file
//     #[graphql(name = "MediaFile")]
//     async fn media_file_resolver(
//         &self,
//         ctx: &async_graphql::Context<'_>,
//     ) -> async_graphql::Result<Option<MediaFileEntity>> {
//         let Some(ref media_file_id) = self.media_file_id else {
//             return Ok(None);
//         };

//         let db = ctx.data_unchecked::<Database>();

//         let entity = EntityQuery::<MediaFileEntity>::new()
//             .where_clause(
//                 &format!("{} = ?", MediaFileEntity::PRIMARY_KEY),
//                 SqlValue::String(media_file_id.clone()),
//             )
//             .fetch_one(db.pool())
//             .await
//             .map_err(|e| async_graphql::Error::new(e.to_string()))?;

//         Ok(entity)
//     }

//     /// Download progress (0.0 - 1.0) for movies that are currently downloading
//     ///
//     /// Returns None if the movie already has a file or is not being downloaded.
//     #[graphql(name = "DownloadProgress")]
//     async fn download_progress(&self, ctx: &async_graphql::Context<'_>) -> Option<f32> {
//         if self.media_file_id.is_some() {
//             return None;
//         }

//         let db = ctx.data_unchecked::<Database>();

//         // Find pending file matches for this movie
//         let matches = EntityQuery::<PendingFileMatchEntity>::new()
//             .filter(&PendingFileMatchEntityWhereInput {
//                 movie_id: Some(StringFilter::eq(&self.id)),
//                 ..Default::default()
//             })
//             .fetch_all(db.pool())
//             .await
//             .ok()?;

//         if matches.is_empty() {
//             return None;
//         }

//         // Get the torrent progress for these matches
//         let mut total_progress = 0.0f32;
//         let mut count = 0;

//         for m in &matches {
//             if m.source_type == "torrent" {
//                 if let Some(ref source_id) = m.source_id {
//                     // Get torrent files for this source
//                     let files = EntityQuery::<TorrentFileEntity>::new()
//                         .filter(&TorrentFileEntityWhereInput {
//                             torrent_id: Some(StringFilter::eq(source_id)),
//                             ..Default::default()
//                         })
//                         .fetch_all(db.pool())
//                         .await
//                         .ok()
//                         .unwrap_or_default();

//                     if let Some(file_index) = m.source_file_index {
//                         if let Some(file) = files.iter().find(|f| f.file_index == file_index) {
//                             total_progress += file.progress as f32;
//                             count += 1;
//                         }
//                     }
//                 }
//             }
//         }

//         if count > 0 {
//             Some(total_progress / count as f32)
//         } else {
//             None
//         }
//     }
// }

// // ============================================================================
// // Custom Operations (non-CRUD - external API calls)
// // ============================================================================

// /// Input for adding a movie from TMDB
// #[derive(Debug, InputObject)]
// pub struct AddMovieFromTmdbInput {
//     /// TMDB movie ID
//     pub tmdb_id: i32,
//     /// Whether to monitor for releases
//     pub monitored: Option<bool>,
// }

// /// Result of movie operations
// #[derive(Debug, SimpleObject)]
// #[graphql(name = "MovieOperationResult")]
// pub struct MovieOperationResult {
//     #[graphql(name = "Success")]
//     pub success: bool,
//     #[graphql(name = "Movie")]
//     pub movie: Option<MovieEntity>,
//     #[graphql(name = "Error")]
//     pub error: Option<String>,
// }

// /// Custom movie operations that require external services (TMDB API)
// ///
// /// These operations CAN'T be replaced by generated CRUD:
// /// - SearchMovies: Searches external TMDB API
// /// - AddMovieFromTmdb: Fetches metadata from TMDB then creates movie
// /// - RefreshMovieMetadata: Re-fetches metadata from TMDB
// #[derive(Default)]
// pub struct MovieCustomOperations;

// #[Object]
// impl MovieCustomOperations {
//     /// Search for movies on TMDB (external API)
//     #[graphql(name = "SearchMovies")]
//     async fn search_movies(
//         &self,
//         ctx: &Context<'_>,
//         query: String,
//         year: Option<i32>,
//     ) -> Result<Vec<MovieSearchResult>> {
//         let _user = ctx.auth_user()?;
//         let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

//         if !metadata.has_tmdb().await {
//             return Err(async_graphql::Error::new(
//                 "TMDB API key not configured. Add tmdb_api_key to settings.",
//             ));
//         }

//         let results = metadata
//             .search_movies(&query, year)
//             .await
//             .map_err(|e| async_graphql::Error::new(e.to_string()))?;

//         Ok(results
//             .into_iter()
//             .map(|m| MovieSearchResult {
//                 provider: "tmdb".to_string(),
//                 provider_id: m.provider_id as i32,
//                 title: m.title,
//                 original_title: m.original_title,
//                 year: m.year,
//                 overview: m.overview,
//                 poster_url: m.poster_url,
//                 backdrop_url: m.backdrop_url,
//                 imdb_id: m.imdb_id,
//                 vote_average: m.vote_average,
//                 popularity: m.popularity,
//             })
//             .collect())
//     }

//     /// Add a movie to a library by fetching metadata from TMDB
//     #[graphql(name = "AddMovieFromTmdb")]
//     async fn add_movie_from_tmdb(
//         &self,
//         ctx: &Context<'_>,
//         library_id: String,
//         input: AddMovieFromTmdbInput,
//     ) -> Result<MovieOperationResult> {
//         let user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>().clone();
//         let metadata = ctx.data_unchecked::<Arc<MetadataService>>();
//         let torrent_service = ctx
//             .data_unchecked::<Arc<crate::services::TorrentService>>()
//             .clone();

//         let lib_id = Uuid::parse_str(&library_id)
//             .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
//         let user_id = Uuid::parse_str(&user.user_id)
//             .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

//         if !metadata.has_tmdb().await {
//             return Ok(MovieOperationResult {
//                 success: false,
//                 movie: None,
//                 error: Some("TMDB API key not configured".to_string()),
//             });
//         }

//         let is_monitored = input.monitored.unwrap_or(true);

//         match metadata
//             .add_movie_from_provider(crate::services::AddMovieOptions {
//                 provider: crate::services::MetadataProvider::Tmdb,
//                 provider_id: input.tmdb_id as u32,
//                 library_id: lib_id,
//                 user_id,
//                 monitored: is_monitored,
//             })
//             .await
//         {
//             Ok(record) => {
//                 // Fetch the created movie as an entity (avoid using the Record type directly)
//                 let movie_id = record.id.to_string();
//                 let movie_entity = MovieEntity::get(db.pool(), &movie_id)
//                     .await
//                     .map_err(|e| async_graphql::Error::new(format!("Failed to fetch movie: {}", e)))?
//                     .ok_or_else(|| async_graphql::Error::new("Movie not found after creation"))?;

//                 tracing::info!(
//                     user_id = %user.user_id,
//                     movie_title = %movie_entity.title,
//                     movie_id = %movie_entity.id,
//                     library_id = %lib_id,
//                     "User added movie from TMDB: {}",
//                     movie_entity.title
//                 );

//                 // Broadcast library change event
//                 broadcast_library_changed(ctx, lib_id).await;

//                 // Trigger auto-hunt if enabled
//                 if is_monitored {
//                     let lib_id_str = lib_id.to_string();
//                     // Get library entity for auto-hunt
//                     let library = LibraryEntity::get(db.pool(), &lib_id_str)
//                         .await
//                         .map_err(|e| async_graphql::Error::new(format!("Failed to get library: {}", e)))?
//                         .ok_or_else(|| async_graphql::Error::new("Library not found"))?;

//                     spawn_auto_hunt(db, library, movie_entity.clone(), user.id, torrent_service);
//                 }

//                 Ok(MovieOperationResult {
//                     success: true,
//                     movie: Some(movie_entity),
//                     error: None,
//                 })
//             }
//             Err(e) => Ok(MovieOperationResult {
//                 success: false,
//                 movie: None,
//                 error: Some(e.to_string()),
//             }),
//         }
//     }

//     /// Refresh movie metadata from TMDB
//     #[graphql(name = "RefreshMovieMetadata")]
//     async fn refresh_movie_metadata(
//         &self,
//         ctx: &Context<'_>,
//         id: String,
//     ) -> Result<MovieOperationResult> {
//         let _user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();
//         let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

//         let movie = MovieEntity::get(db.pool(), &id)
//             .await
//             .map_err(|e| async_graphql::Error::new(e.to_string()))?
//             .ok_or_else(|| async_graphql::Error::new("Movie not found"))?;

//         let tmdb_id = match movie.tmdb_id {
//             Some(id) => id as u32,
//             None => {
//                 return Ok(MovieOperationResult {
//                     success: false,
//                     movie: None,
//                     error: Some("No TMDB ID found for movie".to_string()),
//                 });
//             }
//         };

//         // Fetch fresh movie details from TMDB
//         let movie_details = metadata
//             .get_movie(tmdb_id)
//             .await
//             .map_err(|e| async_graphql::Error::new(e.to_string()))?;

//         // Cache artwork if artwork service is available
//         let (cached_poster_url, cached_backdrop_url) =
//             if let Some(artwork_service) = metadata.artwork_service() {
//                 let entity_id = format!("{}_{}", tmdb_id, movie.library_id);

//                 let poster_url = artwork_service
//                     .cache_image_optional(
//                         movie_details.poster_url.as_deref(),
//                         crate::services::artwork::ArtworkType::Poster,
//                         "movie",
//                         &entity_id,
//                     )
//                     .await;

//                 let backdrop_url = artwork_service
//                     .cache_image_optional(
//                         movie_details.backdrop_url.as_deref(),
//                         crate::services::artwork::ArtworkType::Backdrop,
//                         "movie",
//                         &entity_id,
//                     )
//                     .await;

//                 (poster_url, backdrop_url)
//             } else {
//                 (
//                     movie_details.poster_url.clone(),
//                     movie_details.backdrop_url.clone(),
//                 )
//             };

//         // Update movie metadata using raw SQL
//         let genres_json = serde_json::to_string(&movie_details.genres).unwrap_or_default();
//         let cast_json = serde_json::to_string(&movie_details.cast_names).unwrap_or_default();

//         sqlx::query(
//             r#"
//             UPDATE movies SET
//                 title = ?1, original_title = ?2, overview = ?3, tagline = ?4,
//                 runtime = ?5, genres = ?6, director = ?7, cast_names = ?8,
//                 poster_url = ?9, backdrop_url = ?10, updated_at = datetime('now')
//             WHERE id = ?11
//             "#,
//         )
//         .bind(&movie_details.title)
//         .bind(&movie_details.original_title)
//         .bind(&movie_details.overview)
//         .bind(&movie_details.tagline)
//         .bind(movie_details.runtime)
//         .bind(&genres_json)
//         .bind(&movie_details.director)
//         .bind(&cast_json)
//         .bind(&cached_poster_url)
//         .bind(&cached_backdrop_url)
//         .bind(&id)
//         .execute(db.pool())
//         .await?;

//         // Fetch updated record
//         let updated = MovieEntity::get(db.pool(), &id)
//             .await
//             .map_err(|e| async_graphql::Error::new(e.to_string()))?
//             .ok_or_else(|| async_graphql::Error::new("Movie not found after update"))?;

//         Ok(MovieOperationResult {
//             success: true,
//             movie: Some(updated),
//             error: None,
//         })
//     }
// }

// // ============================================================================
// // Helper Functions
// // ============================================================================

// async fn broadcast_library_changed(ctx: &Context<'_>, library_id: Uuid) {
//     if let Ok(tx) = ctx.data::<broadcast::Sender<LibraryChangedEvent>>() {
//         let db = ctx.data_unchecked::<Database>();
//         let lib_id_str = library_id.to_string();
//         if let Ok(Some(lib)) = LibraryEntity::get(db.pool(), &lib_id_str).await {
//             let _ = tx.send(LibraryChangedEvent {
//                 change_type: LibraryChangeType::Updated,
//                 library_id: lib_id_str,
//                 library_name: Some(lib.name.clone()),
//                 library: Some(library_entity_to_graphql(lib)),
//             });
//         }
//     }
// }

// fn spawn_auto_hunt(
//     db: Database,
//     library: LibraryEntity,
//     movie: MovieEntity,
//     user_id: Uuid,
//     torrent_service: Arc<crate::services::TorrentService>,
// ) {
//     tokio::spawn(async move {
//         // Get encryption key
//         let encryption_key = EntityQuery::<AppSettingEntity>::new()
//             .filter(&AppSettingEntityWhereInput {
//                 key: Some(StringFilter::eq("indexer_encryption_key")),
//                 ..Default::default()
//             })
//             .fetch_one(db.pool())
//             .await
//             .ok()
//             .flatten()
//             .map(|s| s.value);

//         let encryption_key = match encryption_key {
//             Some(key) => key,
//             None => return,
//         };

//         let indexer_manager =
//             match crate::indexer::manager::IndexerManager::new(db.clone(), &encryption_key).await {
//                 Ok(mgr) => std::sync::Arc::new(mgr),
//                 Err(_) => return,
//             };

//         if indexer_manager.load_user_indexers(user_id).await.is_err() {
//             return;
//         }

//         // Use new entity-based hunt function
//         let _ = crate::jobs::auto_hunt::hunt_single_movie_entity(
//             &db,
//             &movie,
//             &library,
//             &torrent_service,
//             &indexer_manager,
//         )
//         .await;
//     });
// }
