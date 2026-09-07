// GraphQLRelations expands a compatibility let-else pattern that cannot be
// rewritten at this entity call site.
#![allow(clippy::question_mark)]

use async_graphql::{Context, InputObject, Object};
use std::sync::Arc;

use crate::graphql::entities::*;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::services::graphql::AuthUser;

use super::library::Library;
use super::media_file::MediaFile;

// Re-export types used for movie creation from metadata
pub use crate::services::metadata::providers::{
    CreateMovieFromMetadataOptions, MovieDetails as MovieMetadataDetails,
};

#[derive(
    GraphQLEntity,
    GraphQLRelations,
    GraphQLOperations,
    async_graphql::SimpleObject,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[graphql(complex)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "movies",
    plural = "Movies",
    default_sort = "title",
    index = "library_id",
    index = "collection_id"
)]
pub struct Movie {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "libraryId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub library_id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,

    #[graphql(name = "title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: String,

    #[graphql(name = "sortTitle")]
    #[sortable]
    pub sort_title: Option<String>,

    #[graphql(name = "originalTitle")]
    pub original_title: Option<String>,

    #[graphql(name = "year")]
    #[filterable(type = "number")]
    #[sortable]
    pub year: Option<i32>,

    #[graphql(name = "tmdbId")]
    #[filterable(type = "number")]
    pub tmdb_id: Option<i32>,

    #[graphql(name = "imdbId")]
    #[filterable(type = "string")]
    pub imdb_id: Option<String>,

    #[graphql(name = "overview")]
    pub overview: Option<String>,

    #[graphql(name = "tagline")]
    pub tagline: Option<String>,

    #[graphql(name = "runtime")]
    #[filterable(type = "number")]
    #[sortable]
    pub runtime: Option<i32>,

    #[graphql(name = "genres")]
    #[json_field]
    pub genres: Vec<String>,

    #[graphql(name = "director")]
    #[filterable(type = "string")]
    pub director: Option<String>,

    #[graphql(name = "castNames")]
    #[json_field]
    pub cast_names: Vec<String>,

    #[graphql(name = "productionCountries")]
    #[json_field]
    pub production_countries: Vec<String>,

    #[graphql(name = "spokenLanguages")]
    #[json_field]
    pub spoken_languages: Vec<String>,

    #[graphql(name = "tmdbRating")]
    pub tmdb_rating: Option<String>,

    #[graphql(name = "tmdbVoteCount")]
    #[filterable(type = "number")]
    pub tmdb_vote_count: Option<i32>,

    #[graphql(skip)]
    pub poster_url: Option<String>,

    #[graphql(skip)]
    pub backdrop_url: Option<String>,

    #[graphql(name = "collectionId")]
    #[filterable(type = "number")]
    pub collection_id: Option<i32>,

    #[graphql(name = "collectionName")]
    #[filterable(type = "string")]
    pub collection_name: Option<String>,

    #[graphql(name = "collectionPosterUrl")]
    pub collection_poster_url: Option<String>,

    #[graphql(name = "releaseDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub release_date: Option<String>,

    #[graphql(name = "certification")]
    #[filterable(type = "string")]
    pub certification: Option<String>,

    #[graphql(name = "monitored")]
    #[filterable(type = "boolean")]
    pub monitored: bool,

    #[graphql(name = "tmdbStatus")]
    #[filterable(type = "string")]
    pub tmdb_status: Option<String>,

    #[graphql(name = "wanted")]
    #[filterable(type = "boolean")]
    pub wanted: bool,

    /// Explicit user opt-out. `None` is the legacy/default false value.
    #[graphql(name = "ignored")]
    #[filterable(type = "boolean")]
    pub ignored: Option<bool>,

    #[graphql(name = "downloadStatus")]
    #[filterable(type = "string")]
    pub download_status: Option<String>,

    #[graphql(name = "hasFile")]
    #[filterable(type = "boolean")]
    pub has_file: bool,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub media_file_id: Option<String>,

    /// Optional quality profile override; falls back to
    /// `Library.qualityProfileId` (then the seeded default) when unset.
    #[graphql(name = "qualityProfileId")]
    #[filterable(type = "string")]
    pub quality_profile_id: Option<String>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
    #[graphql(skip)]
    #[serde(skip)]
    #[relation(
        target = "Library",
        from = "library_id",
        to = "id",
        on_delete = "cascade"
    )]
    pub library: Option<Library>,
    #[graphql(skip)]
    #[serde(skip)]
    #[relation(
        target = "MediaFile",
        from = "media_file_id",
        to = "id",
        on_delete = "set_null"
    )]
    pub media_file: Option<MediaFile>,
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
        let tmdb_rating = details
            .vote_average
            .and_then(rust_decimal::Decimal::from_f64_retain)
            .map(|d| d.to_string());

        let movie = Self::insert(
            db,
            CreateMovieInput {
                library_id: options.library_id.to_string(),
                user_id: options.user_id.to_string(),
                title: details.title.clone(),
                sort_title: None,
                original_title: details.original_title.clone(),
                year: details.year,
                tmdb_id: Some(details.provider_id as i32),
                imdb_id: details.imdb_id.clone(),
                overview: details.overview.clone(),
                tagline: details.tagline.clone(),
                runtime: details.runtime,
                genres: details.genres.clone(),
                director: details.director.clone(),
                cast_names: details.cast_names.clone(),
                production_countries: details.production_countries.clone(),
                spoken_languages: details.spoken_languages.clone(),
                tmdb_rating,
                tmdb_vote_count: details.vote_count,
                poster_url: details.poster_url.clone(),
                backdrop_url: details.backdrop_url.clone(),
                collection_id: details.collection_id,
                collection_name: details.collection_name.clone(),
                collection_poster_url: details.collection_poster_url.clone(),
                release_date: details.release_date.clone(),
                certification: details.certification.clone(),
                monitored: options.monitored,
                tmdb_status: details.tmdb_status.clone(),
                wanted: options.monitored,
                ignored: None,
                download_status: None,
                has_file: false,
                media_file_id: None,
                quality_profile_id: None,
            },
        )
        .await?;

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
    #[graphql(name = "searchMovies")]
    async fn search_movies(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "query")] query: String,
        #[graphql(name = "year")] year: Option<i32>,
    ) -> async_graphql::Result<Vec<MovieSearchResultGql>> {
        use crate::graphql::auth::AuthExt;

        let _user = ctx.librarian_auth_user()?;
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
                release_date: m.release_date,
                overview: m.overview,
                poster_url: m.poster_url,
                backdrop_url: m.backdrop_url,
                imdb_id: m.imdb_id,
                vote_average: m.vote_average,
                popularity: m.popularity,
            })
            .collect())
    }

    /// Discover theatrical releases beyond the user's library.
    #[graphql(name = "movieReleases")]
    async fn movie_releases(
        &self,
        ctx: &Context<'_>,
        kind: crate::services::metadata::tmdb::MovieReleaseKind,
        region: Option<String>,
        #[graphql(default = 1)] page: i32,
    ) -> async_graphql::Result<Vec<MovieSearchResultGql>> {
        use crate::graphql::auth::AuthExt;
        ctx.require_member()?;
        let metadata = ctx.data::<Arc<crate::services::metadata::providers::MetadataService>>()?;
        let movies = metadata
            .movie_releases(kind, region, page)
            .await
            .map_err(|error| async_graphql::Error::new(error.to_string()))?;
        Ok(movies
            .into_iter()
            .map(|m| MovieSearchResultGql {
                provider: "tmdb".into(),
                provider_id: m.provider_id as i32,
                title: m.title,
                original_title: m.original_title,
                year: m.year,
                release_date: m.release_date,
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
    #[graphql(name = "searchMovieCollections")]
    async fn search_movie_collections(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "query")] query: String,
    ) -> async_graphql::Result<Vec<MovieCollectionSearchResultGql>> {
        use crate::graphql::auth::AuthExt;

        let _user = ctx.librarian_auth_user()?;
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
    #[graphql(name = "movieCollectionDetails")]
    async fn movie_collection_details(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "libraryId")] library_id: String,
        #[graphql(name = "collectionId")] collection_id: i32,
    ) -> async_graphql::Result<MovieCollectionDetailsGql> {
        use crate::graphql::auth::AuthExt;
        use crate::services::metadata::providers::MetadataService;

        let user = ctx.librarian_auth_user()?;
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
    #[graphql(name = "query")]
    pub query: String,
    #[graphql(name = "year")]
    pub year: Option<i32>,
}

/// Input for adding a movie from TMDB
#[derive(Debug, InputObject)]
#[graphql(name = "AddMovieInput")]
pub struct AddMovieInput {
    /// TMDB movie ID
    #[graphql(name = "tmdbId")]
    pub tmdb_id: i32,
    /// Whether to monitor for releases (enables auto-download)
    #[graphql(name = "monitored")]
    pub monitored: Option<bool>,
}

/// Input for adding/importing a movie collection from TMDB
#[derive(Debug, InputObject)]
#[graphql(name = "AddMovieCollectionInput")]
pub struct AddMovieCollectionInput {
    /// TMDB collection ID
    #[graphql(name = "collectionId")]
    pub collection_id: i32,
    /// Mark missing imported movies as wanted
    #[graphql(name = "wantedMissing")]
    pub wanted_missing: Option<bool>,
}

/// Movie search result from TMDB
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(name = "MovieSearchResult")]
pub struct MovieSearchResultGql {
    #[graphql(name = "provider")]
    pub provider: String,
    #[graphql(name = "providerId")]
    pub provider_id: i32,
    #[graphql(name = "title")]
    pub title: String,
    #[graphql(name = "originalTitle")]
    pub original_title: Option<String>,
    #[graphql(name = "year")]
    pub year: Option<i32>,
    #[graphql(name = "releaseDate")]
    pub release_date: Option<String>,
    #[graphql(name = "overview")]
    pub overview: Option<String>,
    #[graphql(name = "posterUrl")]
    pub poster_url: Option<String>,
    #[graphql(name = "backdropUrl")]
    pub backdrop_url: Option<String>,
    #[graphql(name = "imdbId")]
    pub imdb_id: Option<String>,
    #[graphql(name = "voteAverage")]
    pub vote_average: Option<f64>,
    #[graphql(name = "popularity")]
    pub popularity: Option<f64>,
}

/// Movie collection search result from TMDB
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(name = "MovieCollectionSearchResult")]
pub struct MovieCollectionSearchResultGql {
    #[graphql(name = "provider")]
    pub provider: String,
    #[graphql(name = "collectionId")]
    pub collection_id: i32,
    #[graphql(name = "name")]
    pub name: String,
    #[graphql(name = "overview")]
    pub overview: Option<String>,
    #[graphql(name = "posterUrl")]
    pub poster_url: Option<String>,
    #[graphql(name = "backdropUrl")]
    pub backdrop_url: Option<String>,
}

/// Movie row in a collection detail response
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(name = "MovieCollectionMovieDetails")]
pub struct MovieCollectionMovieDetailsGql {
    #[graphql(name = "tmdbId")]
    pub tmdb_id: i32,
    #[graphql(name = "title")]
    pub title: String,
    #[graphql(name = "year")]
    pub year: Option<i32>,
    #[graphql(name = "posterUrl")]
    pub poster_url: Option<String>,
    #[graphql(name = "libraryMovieId")]
    pub library_movie_id: Option<String>,
    #[graphql(name = "mediaFileId")]
    pub media_file_id: Option<String>,
    #[graphql(name = "fileSizeBytes")]
    pub file_size_bytes: Option<i64>,
    #[graphql(name = "resolution")]
    pub resolution: Option<String>,
    #[graphql(name = "videoCodec")]
    pub video_codec: Option<String>,
    #[graphql(name = "audioCodec")]
    pub audio_codec: Option<String>,
    #[graphql(name = "audioChannels")]
    pub audio_channels: Option<String>,
    #[graphql(name = "wanted")]
    pub wanted: bool,
}

/// Full TMDB collection details with local overlay
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(name = "MovieCollectionDetails")]
pub struct MovieCollectionDetailsGql {
    #[graphql(name = "collectionId")]
    pub collection_id: i32,
    #[graphql(name = "name")]
    pub name: String,
    #[graphql(name = "overview")]
    pub overview: Option<String>,
    #[graphql(name = "posterUrl")]
    pub poster_url: Option<String>,
    #[graphql(name = "backdropUrl")]
    pub backdrop_url: Option<String>,
    #[graphql(name = "movies")]
    pub movies: Vec<MovieCollectionMovieDetailsGql>,
}

/// Result of movie operations
#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(name = "MovieOperationResult")]
pub struct MovieOperationResult {
    #[graphql(name = "success")]
    pub success: bool,
    #[graphql(name = "movie")]
    pub movie: Option<Movie>,
    #[graphql(name = "error")]
    pub error: Option<String>,
}

/// Result of importing a movie collection
#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(name = "MovieCollectionOperationResult")]
pub struct MovieCollectionOperationResult {
    #[graphql(name = "success")]
    pub success: bool,
    #[graphql(name = "collectionId")]
    pub collection_id: Option<i32>,
    #[graphql(name = "collectionName")]
    pub collection_name: Option<String>,
    #[graphql(name = "importedCount")]
    pub imported_count: i32,
    #[graphql(name = "existingCount")]
    pub existing_count: i32,
    #[graphql(name = "wantedUpdatedCount")]
    pub wanted_updated_count: i32,
    #[graphql(name = "error")]
    pub error: Option<String>,
}

/// Movie metadata mutations (TMDB integration)
#[derive(Default)]
pub struct MovieMetadataMutations;

#[Object]
impl MovieMetadataMutations {
    /// Add a movie to a library by fetching metadata from TMDB
    #[graphql(name = "addMovie")]
    async fn add_movie(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "libraryId")] library_id: String,
        #[graphql(name = "input")] input: AddMovieInput,
    ) -> async_graphql::Result<MovieOperationResult> {
        use crate::graphql::auth::AuthExt;
        use crate::services::metadata::providers::{
            AddMovieOptions, MetadataProvider, MetadataService,
        };

        let user = ctx.librarian_auth_user()?;
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
    #[graphql(name = "addMovieCollection")]
    async fn add_movie_collection(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "libraryId")] library_id: String,
        #[graphql(name = "input")] input: AddMovieCollectionInput,
    ) -> async_graphql::Result<MovieCollectionOperationResult> {
        use crate::graphql::auth::AuthExt;
        use crate::services::metadata::providers::{
            AddMovieCollectionOptions, MetadataProvider, MetadataService,
        };

        let user = ctx.librarian_auth_user()?;
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
    #[graphql(name = "refreshMovie")]
    async fn refresh_movie(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "id")] id: String,
    ) -> async_graphql::Result<MovieOperationResult> {
        use crate::graphql::auth::AuthExt;
        use crate::services::metadata::providers::MetadataService;

        let user = ctx.librarian_auth_user()?;
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
