use async_graphql::{Result, SimpleObject};
use librarian_macros::{GraphQLEntity, GraphQLOperations};
use serde::{Deserialize, Serialize};

use super::media_file::MediaFile;

#[derive(GraphQLEntity, GraphQLOperations, SimpleObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(name = "Movie")]
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

    #[graphql(name = "PosterUrl")]
    pub poster_url: Option<String>,

    #[graphql(name = "BackdropUrl")]
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

    #[graphql(name = "Status")]
    #[filterable(type = "string")]
    pub status: Option<String>,

    #[graphql(name = "Monitored")]
    #[filterable(type = "boolean")]
    pub monitored: bool,

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

#[derive(Default)]
pub struct MovieCustomOperations;

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
