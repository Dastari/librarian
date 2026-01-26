use async_graphql::{Result, SimpleObject};
use librarian_macros::{GraphQLEntity, GraphQLOperations};
use serde::{Deserialize, Serialize};

use crate::{
    db::Database,
    graphql::{
        entities::{EpisodeOrderByInput, EpisodeWhereInput},
        orm::{EntityQuery, StringFilter},
    },
};

use super::episode::Episode;

#[derive(GraphQLEntity, GraphQLOperations, SimpleObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(name = "Show", complex)]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "shows",
    plural = "Shows",
    default_sort = "name",
    notify = "libraries"
)]
pub struct Show {
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

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "SortName")]
    #[sortable]
    pub sort_name: Option<String>,

    #[graphql(name = "Year")]
    #[filterable(type = "number")]
    #[sortable]
    pub year: Option<i32>,

    #[graphql(name = "Status")]
    #[filterable(type = "string")]
    pub status: Option<String>,

    #[graphql(name = "TvmazeId")]
    #[filterable(type = "number")]
    pub tvmaze_id: Option<i32>,

    #[graphql(name = "TmdbId")]
    #[filterable(type = "number")]
    pub tmdb_id: Option<i32>,

    #[graphql(name = "TvdbId")]
    #[filterable(type = "number")]
    pub tvdb_id: Option<i32>,

    #[graphql(name = "ImdbId")]
    #[filterable(type = "string")]
    pub imdb_id: Option<String>,

    #[graphql(name = "Overview")]
    pub overview: Option<String>,

    #[graphql(name = "Network")]
    #[filterable(type = "string")]
    pub network: Option<String>,

    #[graphql(name = "Runtime")]
    #[filterable(type = "number")]
    pub runtime: Option<i32>,

    #[graphql(name = "Genres")]
    #[json_field]
    pub genres: Vec<String>,

    #[graphql(name = "PosterUrl")]
    pub poster_url: Option<String>,

    #[graphql(name = "BackdropUrl")]
    pub backdrop_url: Option<String>,

    #[graphql(name = "ContentRating")]
    #[filterable(type = "string")]
    pub content_rating: Option<String>,

    #[graphql(name = "Monitored")]
    #[filterable(type = "boolean")]
    pub monitored: bool,

    #[graphql(name = "MonitorType")]
    #[filterable(type = "string")]
    pub monitor_type: String,

    #[graphql(name = "Path")]
    pub path: Option<String>,

    #[graphql(name = "EpisodeCount")]
    #[filterable(type = "number")]
    #[sortable]
    pub episode_count: Option<i32>,

    #[graphql(name = "EpisodeFileCount")]
    #[filterable(type = "number")]
    pub episode_file_count: Option<i32>,

    #[graphql(name = "SizeBytes")]
    #[filterable(type = "number")]
    #[sortable]
    pub size_bytes: Option<i64>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    pub episodes: Vec<Episode>,
}

#[async_graphql::ComplexObject]
impl Show {
    /// Episodes in this show
    #[graphql(name = "Episodes")]
    async fn episodes(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Vec<Episode>> {
        let where_input = EpisodeWhereInput {
            show_id: Some(StringFilter::eq(&self.id)),
            ..Default::default()
        };
        let order_by = EpisodeOrderByInput::default();
        let episodes = EntityQuery::<Episode>::new()
            .filter(&where_input)
            .order_by(&order_by)
            .fetch_all(ctx.data_unchecked::<Database>())
            .await?;

        Ok(episodes)
    }
}
#[derive(Default)]
pub struct ShowCustomOperations;

// // ============================================================================
// // ComplexObject Resolvers
// // ============================================================================

// #[async_graphql::ComplexObject]
// impl TvShowEntity {
//     /// Episodes in this TV show
//     #[graphql(name = "Episodes")]
//     async fn episodes_resolver(
//         &self,
//         ctx: &async_graphql::Context<'_>,
//         #[graphql(name = "Where")] where_input: Option<super::episode::EpisodeEntityWhereInput>,
//         #[graphql(name = "OrderBy")] order_by: Option<Vec<super::episode::EpisodeEntityOrderByInput>>,
//         #[graphql(name = "Page")] page: Option<crate::graphql::orm::PageInput>,
//     ) -> async_graphql::Result<super::episode::EpisodeEntityConnection> {
//         let db = ctx.data_unchecked::<Database>();
//         let pool = db.pool();

//         let mut query = EntityQuery::<EpisodeEntity>::new()
//             .where_clause("tv_show_id = ?", SqlValue::String(self.id.clone()));

//         if let Some(ref filter) = where_input {
//             query = query.filter(filter);
//         }
//         if let Some(ref orders) = order_by {
//             for order in orders {
//                 query = query.order_by(order);
//             }
//         }
//         if query.order_clauses.is_empty() {
//             query = query.default_order();
//         }
//         if let Some(ref p) = page {
//             query = query.paginate(p);
//         }

//         let conn = query
//             .fetch_connection(pool)
//             .await
//             .map_err(|e| async_graphql::Error::new(e.to_string()))?;
//         Ok(super::episode::EpisodeEntityConnection::from_generic(conn))
//     }
// }

// // ============================================================================
// // Custom Operations (non-CRUD - external API calls)
// // ============================================================================

// /// Input for adding a TV show from external provider
// #[derive(Debug, InputObject)]
// pub struct AddTvShowFromProviderInput {
//     /// Provider name (tvmaze, tmdb, tvdb)
//     pub provider: String,
//     /// Provider ID
//     pub provider_id: i32,
//     /// Monitor type (all, future, none)
//     pub monitor_type: Option<MonitorType>,
//     /// Custom path
//     pub path: Option<String>,
// }

// /// Result of TV show operations
// #[derive(Debug, SimpleObject)]
// #[graphql(name = "TvShowOperationResult")]
// pub struct TvShowOperationResult {
//     #[graphql(name = "Success")]
//     pub success: bool,
//     #[graphql(name = "TvShow")]
//     pub tv_show: Option<TvShowEntity>,
//     #[graphql(name = "Error")]
//     pub error: Option<String>,
// }

// /// Custom TV show operations that require external services
// ///
// /// These operations CAN'T be replaced by generated CRUD:
// /// - SearchTvShows: Searches external metadata APIs
// /// - AddTvShowFromProvider: Fetches metadata from external API
// /// - RefreshTvShowMetadata: Re-fetches from external API
// #[derive(Default)]
// pub struct TvShowCustomOperations;

// #[Object]
// impl TvShowCustomOperations {
//     /// Search for TV shows from metadata providers
//     #[graphql(name = "SearchTvShows")]
//     async fn search_tv_shows(&self, ctx: &Context<'_>, query: String) -> Result<Vec<TvShowSearchResult>> {
//         let _user = ctx.auth_user()?;
//         let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

//         let results = metadata
//             .search_shows(&query)
//             .await
//             .map_err(|e| async_graphql::Error::new(e.to_string()))?;

//         Ok(results
//             .into_iter()
//             .map(|r| TvShowSearchResult {
//                 provider: format!("{:?}", r.provider).to_lowercase(),
//                 provider_id: r.provider_id as i32,
//                 name: r.name,
//                 year: r.year,
//                 status: r.status,
//                 network: r.network,
//                 overview: r.overview,
//                 poster_url: r.poster_url,
//                 tvdb_id: r.tvdb_id.map(|id| id as i32),
//                 imdb_id: r.imdb_id,
//                 score: r.score,
//             })
//             .collect())
//     }

//     /// Add a TV show from external provider
//     #[graphql(name = "AddTvShowFromProvider")]
//     async fn add_tv_show_from_provider(
//         &self,
//         ctx: &Context<'_>,
//         library_id: String,
//         input: AddTvShowFromProviderInput,
//     ) -> Result<TvShowOperationResult> {
//         let user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();
//         let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

//         let lib_id = Uuid::parse_str(&library_id)
//             .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
//         let user_id = Uuid::parse_str(&user.user_id)
//             .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

//         let provider = match input.provider.as_str() {
//             "tvmaze" => crate::services::MetadataProvider::TvMaze,
//             "tmdb" => crate::services::MetadataProvider::Tmdb,
//             "tvdb" => crate::services::MetadataProvider::TvDb,
//             _ => return Err(async_graphql::Error::new("Invalid provider")),
//         };

//         let monitor_type = input
//             .monitor_type
//             .map(|mt| match mt {
//                 MonitorType::All => "all",
//                 MonitorType::Future => "future",
//                 MonitorType::None => "none",
//             })
//             .unwrap_or("all")
//             .to_string();

//         match metadata
//             .add_tv_show_from_provider(crate::services::AddTvShowOptions {
//                 provider,
//                 provider_id: input.provider_id as u32,
//                 library_id: lib_id,
//                 user_id,
//                 monitored: true,
//                 monitor_type,
//                 path: input.path,
//             })
//             .await
//         {
//             Ok(record) => {
//                 // Fetch the created show as an entity
//                 let show_id = record.id.to_string();
//                 let show_entity = TvShowEntity::get(db.pool(), &show_id)
//                     .await
//                     .map_err(|e| async_graphql::Error::new(format!("Failed to fetch show: {}", e)))?
//                     .ok_or_else(|| async_graphql::Error::new("Show not found after creation"))?;

//                 tracing::info!(
//                     show_name = %show_entity.name,
//                     show_id = %show_entity.id,
//                     "User added TV show from provider"
//                 );
//                 Ok(TvShowOperationResult {
//                     success: true,
//                     tv_show: Some(show_entity),
//                     error: None,
//                 })
//             }
//             Err(e) => Ok(TvShowOperationResult {
//                 success: false,
//                 tv_show: None,
//                 error: Some(e.to_string()),
//             }),
//         }
//     }

//     /// Refresh TV show metadata from external provider
//     #[graphql(name = "RefreshTvShowMetadata")]
//     async fn refresh_tv_show_metadata(
//         &self,
//         ctx: &Context<'_>,
//         id: String,
//     ) -> Result<TvShowOperationResult> {
//         let _user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();
//         let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

//         // Fetch show using entity query
//         let show = TvShowEntity::get(db.pool(), &id)
//             .await
//             .map_err(|e| async_graphql::Error::new(e.to_string()))?
//             .ok_or_else(|| async_graphql::Error::new("Show not found"))?;

//         let (provider, provider_id) = if let Some(tvmaze_id) = show.tvmaze_id {
//             (crate::services::MetadataProvider::TvMaze, tvmaze_id as u32)
//         } else if let Some(tmdb_id) = show.tmdb_id {
//             (crate::services::MetadataProvider::Tmdb, tmdb_id as u32)
//         } else if let Some(tvdb_id) = show.tvdb_id {
//             (crate::services::MetadataProvider::TvDb, tvdb_id as u32)
//         } else {
//             return Ok(TvShowOperationResult {
//                 success: false,
//                 tv_show: None,
//                 error: Some("No provider ID found".to_string()),
//             });
//         };

//         let show_details = metadata
//             .get_show(provider, provider_id)
//             .await
//             .map_err(|e| async_graphql::Error::new(e.to_string()))?;

//         // Cache artwork
//         let (cached_poster_url, cached_backdrop_url) =
//             if let Some(artwork_service) = metadata.artwork_service() {
//                 let entity_id = format!("{}_{}", provider_id, show.library_id);
//                 let poster = artwork_service
//                     .cache_image_optional(
//                         show_details.poster_url.as_deref(),
//                         crate::services::artwork::ArtworkType::Poster,
//                         "show",
//                         &entity_id,
//                     )
//                     .await;
//                 let backdrop = artwork_service
//                     .cache_image_optional(
//                         show_details.backdrop_url.as_deref(),
//                         crate::services::artwork::ArtworkType::Backdrop,
//                         "show",
//                         &entity_id,
//                     )
//                     .await;
//                 (poster, backdrop)
//             } else {
//                 (show_details.poster_url.clone(), show_details.backdrop_url.clone())
//             };

//         // Update TV show metadata using raw SQL
//         let genres_json = serde_json::to_string(&show_details.genres).unwrap_or_else(|_| "[]".to_string());
//         sqlx::query(
//             r#"
//             UPDATE tv_shows SET
//                 name = ?1, overview = ?2, status = ?3, network = ?4,
//                 runtime = ?5, genres = ?6, poster_url = ?7, backdrop_url = ?8,
//                 updated_at = datetime('now')
//             WHERE id = ?9
//             "#,
//         )
//         .bind(&show_details.name)
//         .bind(&show_details.overview)
//         .bind(show_details.status.as_deref().unwrap_or("unknown"))
//         .bind(&show_details.network)
//         .bind(show_details.runtime)
//         .bind(&genres_json)
//         .bind(&cached_poster_url)
//         .bind(&cached_backdrop_url)
//         .bind(&id)
//         .execute(db.pool())
//         .await
//         .ok();

//         // Refresh episodes using raw SQL
//         let episodes = metadata
//             .get_episodes(provider, provider_id)
//             .await
//             .map_err(|e| async_graphql::Error::new(e.to_string()))?;

//         for ep in episodes {
//             let ep_id = uuid::Uuid::new_v4().to_string();
//             let air_date_str = ep.air_date.as_deref();
//             let tvmaze_id = if provider == crate::services::MetadataProvider::TvMaze {
//                 Some(ep.provider_id as i32)
//             } else {
//                 None
//             };

//             // Insert or update episode
//             sqlx::query(
//                 r#"
//                 INSERT INTO episodes (id, tv_show_id, season, episode, absolute_number, title, overview, air_date, runtime, tvmaze_id, created_at, updated_at)
//                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, datetime('now'), datetime('now'))
//                 ON CONFLICT(tv_show_id, season, episode) DO UPDATE SET
//                     title = ?6, overview = ?7, air_date = ?8, runtime = ?9, updated_at = datetime('now')
//                 "#,
//             )
//             .bind(&ep_id)
//             .bind(&id)
//             .bind(ep.season)
//             .bind(ep.episode)
//             .bind(ep.absolute_number)
//             .bind(&ep.title)
//             .bind(&ep.overview)
//             .bind(air_date_str)
//             .bind(ep.runtime)
//             .bind(tvmaze_id)
//             .execute(db.pool())
//             .await
//             .ok();
//         }

//         // Update episode count stats
//         sqlx::query(
//             r#"
//             UPDATE tv_shows SET
//                 episode_count = (SELECT COUNT(*) FROM episodes WHERE tv_show_id = ?1),
//                 episode_file_count = (SELECT COUNT(*) FROM episodes WHERE tv_show_id = ?1 AND media_file_id IS NOT NULL),
//                 size_bytes = (SELECT COALESCE(SUM(mf.size_bytes), 0) FROM episodes e JOIN media_files mf ON e.media_file_id = mf.id WHERE e.tv_show_id = ?1),
//                 updated_at = datetime('now')
//             WHERE id = ?1
//             "#,
//         )
//         .bind(&id)
//         .execute(db.pool())
//         .await
//         .ok();

//         // Fetch updated entity and convert to view model
//         let updated = TvShowEntity::get(db.pool(), &id).await.ok().flatten();

//         Ok(TvShowOperationResult {
//             success: true,
//             tv_show: updated,
//             error: None,
//         })
//     }
// }
