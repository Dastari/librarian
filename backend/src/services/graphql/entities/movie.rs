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

    /// Search for movie collections on TMDB
    #[graphql(name = "SearchMovieCollections")]
    async fn search_movie_collections(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Query")] query: String,
    ) -> async_graphql::Result<Vec<MovieCollectionSearchResultGql>> {
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
            .search_movie_collections(&query)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|c| MovieCollectionSearchResultGql {
                provider: "tmdb".to_string(),
                collection_id: c.collection_id,
                name: c.name,
                overview: c.overview,
                poster_url: c.poster_url,
                backdrop_url: c.backdrop_url,
            })
            .collect())
    }

    /// Get full collection details from TMDB with library state overlay.
    #[graphql(name = "MovieCollectionDetails")]
    async fn movie_collection_details(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "LibraryId")] library_id: String,
        #[graphql(name = "CollectionId")] collection_id: i32,
    ) -> async_graphql::Result<MovieCollectionDetailsGql> {
        use crate::graphql::auth::AuthExt;
        use crate::services::metadata::providers::MetadataService;

        let user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        if !metadata.has_tmdb().await {
            return Err(async_graphql::Error::new(
                "TMDB API key not configured. Add tmdb_api_key to settings.",
            ));
        }

        let lib_id = uuid::Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = uuid::Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let details = metadata
            .get_movie_collection_details(collection_id, lib_id, user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MovieCollectionDetailsGql {
            collection_id: details.collection_id,
            name: details.name,
            overview: details.overview,
            poster_url: details.poster_url,
            backdrop_url: details.backdrop_url,
            movies: details
                .movies
                .into_iter()
                .map(|m| MovieCollectionMovieDetailsGql {
                    tmdb_id: m.tmdb_id,
                    title: m.title,
                    year: m.year,
                    poster_url: m.poster_url,
                    library_movie_id: m.library_movie_id,
                    media_file_id: m.media_file_id,
                    file_size_bytes: m.file_size_bytes,
                    resolution: m.resolution,
                    video_codec: m.video_codec,
                    audio_codec: m.audio_codec,
                    audio_channels: m.audio_channels,
                    wanted: m.wanted,
                })
                .collect(),
        })
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

/// Input for adding/importing a movie collection from TMDB
#[derive(Debug, InputObject)]
#[graphql(name = "AddMovieCollectionInput")]
pub struct AddMovieCollectionInput {
    /// TMDB collection ID
    #[graphql(name = "CollectionId")]
    pub collection_id: i32,
    /// Mark missing imported movies as wanted
    #[graphql(name = "WantedMissing")]
    pub wanted_missing: Option<bool>,
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

/// Movie collection search result from TMDB
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(name = "MovieCollectionSearchResult")]
pub struct MovieCollectionSearchResultGql {
    #[graphql(name = "Provider")]
    pub provider: String,
    #[graphql(name = "CollectionId")]
    pub collection_id: i32,
    #[graphql(name = "Name")]
    pub name: String,
    #[graphql(name = "Overview")]
    pub overview: Option<String>,
    #[graphql(name = "PosterUrl")]
    pub poster_url: Option<String>,
    #[graphql(name = "BackdropUrl")]
    pub backdrop_url: Option<String>,
}

/// Movie row in a collection detail response
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(name = "MovieCollectionMovieDetails")]
pub struct MovieCollectionMovieDetailsGql {
    #[graphql(name = "TmdbId")]
    pub tmdb_id: i32,
    #[graphql(name = "Title")]
    pub title: String,
    #[graphql(name = "Year")]
    pub year: Option<i32>,
    #[graphql(name = "PosterUrl")]
    pub poster_url: Option<String>,
    #[graphql(name = "LibraryMovieId")]
    pub library_movie_id: Option<String>,
    #[graphql(name = "MediaFileId")]
    pub media_file_id: Option<String>,
    #[graphql(name = "FileSizeBytes")]
    pub file_size_bytes: Option<i64>,
    #[graphql(name = "Resolution")]
    pub resolution: Option<String>,
    #[graphql(name = "VideoCodec")]
    pub video_codec: Option<String>,
    #[graphql(name = "AudioCodec")]
    pub audio_codec: Option<String>,
    #[graphql(name = "AudioChannels")]
    pub audio_channels: Option<String>,
    #[graphql(name = "Wanted")]
    pub wanted: bool,
}

/// Full TMDB collection details with local overlay
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(name = "MovieCollectionDetails")]
pub struct MovieCollectionDetailsGql {
    #[graphql(name = "CollectionId")]
    pub collection_id: i32,
    #[graphql(name = "Name")]
    pub name: String,
    #[graphql(name = "Overview")]
    pub overview: Option<String>,
    #[graphql(name = "PosterUrl")]
    pub poster_url: Option<String>,
    #[graphql(name = "BackdropUrl")]
    pub backdrop_url: Option<String>,
    #[graphql(name = "Movies")]
    pub movies: Vec<MovieCollectionMovieDetailsGql>,
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

/// Result of importing a movie collection
#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(name = "MovieCollectionOperationResult")]
pub struct MovieCollectionOperationResult {
    #[graphql(name = "Success")]
    pub success: bool,
    #[graphql(name = "CollectionId")]
    pub collection_id: Option<i32>,
    #[graphql(name = "CollectionName")]
    pub collection_name: Option<String>,
    #[graphql(name = "ImportedCount")]
    pub imported_count: i32,
    #[graphql(name = "ExistingCount")]
    pub existing_count: i32,
    #[graphql(name = "WantedUpdatedCount")]
    pub wanted_updated_count: i32,
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

    /// Add/import all movies from a TMDB collection into a library.
    #[graphql(name = "AddMovieCollection")]
    async fn add_movie_collection(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "LibraryId")] library_id: String,
        #[graphql(name = "Input")] input: AddMovieCollectionInput,
    ) -> async_graphql::Result<MovieCollectionOperationResult> {
        use crate::graphql::auth::AuthExt;
        use crate::services::metadata::providers::{
            AddMovieCollectionOptions, MetadataProvider, MetadataService,
        };

        let user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let lib_id = uuid::Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = uuid::Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        if !metadata.has_tmdb().await {
            return Ok(MovieCollectionOperationResult {
                success: false,
                collection_id: None,
                collection_name: None,
                imported_count: 0,
                existing_count: 0,
                wanted_updated_count: 0,
                error: Some("TMDB API key not configured".to_string()),
            });
        }

        let wanted_missing = input.wanted_missing.unwrap_or(true);
        match metadata
            .add_movie_collection_from_provider(AddMovieCollectionOptions {
                provider: MetadataProvider::Tmdb,
                collection_id: input.collection_id,
                library_id: lib_id,
                user_id,
                wanted_missing,
            })
            .await
        {
            Ok(summary) => Ok(MovieCollectionOperationResult {
                success: true,
                collection_id: Some(summary.collection_id),
                collection_name: Some(summary.collection_name),
                imported_count: summary.imported_count,
                existing_count: summary.existing_count,
                wanted_updated_count: summary.wanted_updated_count,
                error: None,
            }),
            Err(e) => Ok(MovieCollectionOperationResult {
                success: false,
                collection_id: Some(input.collection_id),
                collection_name: None,
                imported_count: 0,
                existing_count: 0,
                wanted_updated_count: 0,
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
