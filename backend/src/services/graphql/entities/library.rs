use async_graphql::{Result, SimpleObject};
use librarian_macros::{GraphQLEntity, GraphQLOperations};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::services::graphql::entities::{ShowOrderByInput, ShowWhereInput};
use crate::services::graphql::orm::{EntityQuery, StringFilter};

use super::album::Album;
use super::artist::Artist;
use super::audiobook::Audiobook;
use super::media_file::MediaFile;
use super::movie::Movie;
use super::show::Show;

#[derive(GraphQLEntity, GraphQLOperations, SimpleObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(name = "Library")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "libraries", plural = "Libraries", default_sort = "name")]
pub struct Library {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    #[sortable]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "Path")]
    #[filterable(type = "string")]
    pub path: String,

    #[graphql(name = "LibraryType")]
    #[filterable(type = "string")]
    #[sortable]
    pub library_type: String,

    #[graphql(name = "Icon")]
    pub icon: Option<String>,

    #[graphql(name = "Color")]
    pub color: Option<String>,

    #[graphql(name = "AutoScan")]
    #[filterable(type = "boolean")]
    pub auto_scan: bool,

    #[graphql(name = "ScanIntervalMinutes")]
    #[filterable(type = "number")]
    pub scan_interval_minutes: i32,

    #[graphql(name = "WatchForChanges")]
    #[filterable(type = "boolean")]
    pub watch_for_changes: bool,

    #[graphql(name = "AutoAddDiscovered")]
    #[filterable(type = "boolean")]
    pub auto_add_discovered: bool,

    #[graphql(name = "AutoDownload")]
    #[filterable(type = "boolean")]
    pub auto_download: bool,

    #[graphql(name = "AutoHunt")]
    #[filterable(type = "boolean")]
    pub auto_hunt: bool,

    #[graphql(name = "Scanning")]
    #[filterable(type = "boolean")]
    pub scanning: bool,

    #[graphql(name = "LastScannedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_scanned_at: Option<String>,

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
    pub movies: Vec<Movie>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    pub shows: Vec<Show>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    pub artists: Vec<Artist>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    pub albums: Vec<Album>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    pub audiobooks: Vec<Audiobook>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    pub media_files: Vec<MediaFile>,
}

#[async_graphql::ComplexObject]
impl Library {
    #[graphql(name = "Shows")]
    async fn shows(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<Vec<Show>> {
        let where_input = ShowWhereInput {
            library_id: Some(StringFilter::eq(&self.id)),
            ..Default::default()
        };
        let order_by = ShowOrderByInput::default();
        let shows = EntityQuery::<Show>::new()
            .filter(&where_input)
            .order_by(&order_by)
            .fetch_all(ctx.data_unchecked::<Database>())
            .await?;

        Ok(shows)
    }
}

#[derive(Default)]
pub struct LibraryCustomOperations;

// // ============================================================================
// // ComplexObject Resolvers (relations + computed fields)
// // ============================================================================

// #[async_graphql::ComplexObject]
// impl LibraryEntity {
//     /// Total item count in this library
//     #[graphql(name = "ItemCount")]
//     async fn item_count(&self, ctx: &async_graphql::Context<'_>) -> i64 {
//         let db = ctx.data_unchecked::<Database>();

//         if let Ok(id) = uuid::Uuid::parse_str(&self.id) {
//             if let Ok(stats) = crate::db::operations::get_library_stats(db.pool(), id).await {
//                 return stats.movie_count
//                     + stats.tv_show_count
//                     + stats.artist_count
//                     + stats.album_count
//                     + stats.audiobook_count;
//             }
//         }
//         0
//     }

//     /// Total size of media files in bytes
//     #[graphql(name = "TotalSizeBytes")]
//     async fn total_size_bytes(&self, ctx: &async_graphql::Context<'_>) -> i64 {
//         let db = ctx.data_unchecked::<Database>();

//         if let Ok(id) = uuid::Uuid::parse_str(&self.id) {
//             if let Ok(stats) = crate::db::operations::get_library_stats(db.pool(), id).await {
//                 return stats.total_size_bytes;
//             }
//         }
//         0
//     }

//     /// Movies in this library
//     #[graphql(name = "Movies")]
//     async fn movies_resolver(
//         &self,
//         ctx: &async_graphql::Context<'_>,
//         #[graphql(name = "Where")] where_input: Option<super::movie::MovieEntityWhereInput>,
//         #[graphql(name = "OrderBy")] order_by: Option<Vec<super::movie::MovieEntityOrderByInput>>,
//         #[graphql(name = "Page")] page: Option<crate::graphql::orm::PageInput>,
//     ) -> async_graphql::Result<super::movie::MovieEntityConnection> {
//         let db = ctx.data_unchecked::<Database>();
//         let pool = db.pool();

//         let mut query = EntityQuery::<MovieEntity>::new()
//             .where_clause("library_id = ?", SqlValue::String(self.id.clone()));

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
//         Ok(super::movie::MovieEntityConnection::from_generic(conn))
//     }

//     /// TV Shows in this library
//     #[graphql(name = "TvShows")]
//     async fn tv_shows_resolver(
//         &self,
//         ctx: &async_graphql::Context<'_>,
//         #[graphql(name = "Where")] where_input: Option<super::tv_show::TvShowEntityWhereInput>,
//         #[graphql(name = "OrderBy")] order_by: Option<Vec<super::tv_show::TvShowEntityOrderByInput>>,
//         #[graphql(name = "Page")] page: Option<crate::graphql::orm::PageInput>,
//     ) -> async_graphql::Result<super::tv_show::TvShowEntityConnection> {
//         let db = ctx.data_unchecked::<Database>();
//         let pool = db.pool();

//         let mut query = EntityQuery::<TvShowEntity>::new()
//             .where_clause("library_id = ?", SqlValue::String(self.id.clone()));

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
//         Ok(super::tv_show::TvShowEntityConnection::from_generic(conn))
//     }

//     /// Media files in this library
//     #[graphql(name = "MediaFiles")]
//     async fn media_files_resolver(
//         &self,
//         ctx: &async_graphql::Context<'_>,
//         #[graphql(name = "Where")] where_input: Option<super::media_file::MediaFileEntityWhereInput>,
//         #[graphql(name = "OrderBy")]
//         order_by: Option<Vec<super::media_file::MediaFileEntityOrderByInput>>,
//         #[graphql(name = "Page")] page: Option<crate::graphql::orm::PageInput>,
//     ) -> async_graphql::Result<super::media_file::MediaFileEntityConnection> {
//         let db = ctx.data_unchecked::<Database>();
//         let pool = db.pool();

//         let mut query = EntityQuery::<MediaFileEntity>::new()
//             .where_clause("library_id = ?", SqlValue::String(self.id.clone()));

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
//         Ok(super::media_file::MediaFileEntityConnection::from_generic(
//             conn,
//         ))
//     }
// }

// // ============================================================================
// // Custom Operations (non-CRUD - service calls)
// // ============================================================================

// /// Result of scan operation
// #[derive(Debug, SimpleObject)]
// pub struct ScanResult {
//     pub library_id: String,
//     pub status: String,
//     pub message: Option<String>,
// }

// /// Result of consolidation
// #[derive(Debug, SimpleObject)]
// pub struct ConsolidateResult {
//     pub success: bool,
//     pub folders_removed: i32,
//     pub files_moved: i32,
//     pub messages: Vec<String>,
// }

// /// Custom library operations that require external services
// ///
// /// These operations CAN'T be replaced by generated CRUD:
// /// - ScanLibrary: Triggers scanner service
// /// - ConsolidateLibrary: File reorganization
// #[derive(Default)]
// pub struct LibraryCustomOperations;

// #[Object]
// impl LibraryCustomOperations {
//     /// Trigger a library scan
//     #[graphql(name = "ScanLibrary")]
//     async fn scan_library(&self, ctx: &Context<'_>, id: String) -> Result<ScanResult> {
//         let _user = ctx.auth_user()?;
//         let scanner = ctx.data_unchecked::<Arc<ScannerService>>();
//         let db = ctx.data_unchecked::<Database>().clone();

//         let library_id = Uuid::parse_str(&id)
//             .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

//         tracing::info!(library_id = %id, "Scan requested for library");

//         let id_clone = id.clone();
//         let scanner = scanner.clone();
//         tokio::spawn(async move {
//             match scanner.scan_library(library_id).await {
//                 Ok(progress) => {
//                     tracing::info!(
//                         library_id = %library_id,
//                         total_files = progress.total_files,
//                         new_files = progress.new_files,
//                         "Library scan completed"
//                     );
//                 }
//                 Err(e) => {
//                     tracing::error!(library_id = %library_id, error = %e, "Library scan failed");
//                     // Update scanning status using raw SQL
//                     let _ = sqlx::query(
//                         "UPDATE libraries SET is_scanning = false, updated_at = datetime('now') WHERE id = ?",
//                     )
//                     .bind(&id_clone)
//                     .execute(db.pool())
//                     .await;
//                 }
//             }
//         });

//         Ok(ScanResult {
//             library_id: id,
//             status: "started".to_string(),
//             message: Some("Scan has been started".to_string()),
//         })
//     }

//     /// Consolidate library folders
//     #[graphql(name = "ConsolidateLibrary")]
//     async fn consolidate_library(&self, ctx: &Context<'_>, id: String) -> Result<ConsolidateResult> {
//         let _user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>();

//         let library_id = Uuid::parse_str(&id)
//             .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

//         let organizer = crate::services::OrganizerService::new(db.clone());

//         match organizer.consolidate_library(library_id).await {
//             Ok(result) => Ok(ConsolidateResult {
//                 success: result.success,
//                 folders_removed: result.folders_removed,
//                 files_moved: result.files_moved,
//                 messages: result.messages,
//             }),
//             Err(e) => Ok(ConsolidateResult {
//                 success: false,
//                 folders_removed: 0,
//                 files_moved: 0,
//                 messages: vec![format!("Consolidation failed: {}", e)],
//             }),
//         }
//     }
// }
