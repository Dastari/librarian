use async_graphql::{Result, SimpleObject};
use librarian_macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use super::chapter::Chapter;

#[derive(
    GraphQLEntity,
    GraphQLRelations,
    GraphQLOperations,
    SimpleObject,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[graphql(name = "Audiobook", complex)]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "audiobooks",
    plural = "Audiobooks",
    default_sort = "title",
    notify = "libraries"
)]
pub struct Audiobook {
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

    #[graphql(name = "AuthorName")]
    #[filterable(type = "string")]
    #[sortable]
    pub author_name: Option<String>,

    #[graphql(name = "NarratorName")]
    #[filterable(type = "string")]
    pub narrator_name: Option<String>,

    #[graphql(name = "Narrators")]
    #[json_field]
    pub narrators: Vec<String>,

    #[graphql(name = "Description")]
    pub description: Option<String>,

    #[graphql(name = "Publisher")]
    #[filterable(type = "string")]
    pub publisher: Option<String>,

    #[graphql(name = "PublishedDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub published_date: Option<String>,

    #[graphql(name = "Language")]
    #[filterable(type = "string")]
    pub language: Option<String>,

    #[graphql(name = "Isbn")]
    #[filterable(type = "string")]
    pub isbn: Option<String>,

    #[graphql(name = "Asin")]
    #[filterable(type = "string")]
    pub asin: Option<String>,

    #[graphql(name = "AudibleId")]
    #[filterable(type = "string")]
    pub audible_id: Option<String>,

    #[graphql(name = "GoodreadsId")]
    #[filterable(type = "string")]
    pub goodreads_id: Option<String>,

    #[graphql(name = "TotalDurationSecs")]
    #[filterable(type = "number")]
    #[sortable]
    pub total_duration_secs: Option<i32>,

    #[graphql(name = "ChapterCount")]
    #[filterable(type = "number")]
    pub chapter_count: Option<i32>,

    #[graphql(name = "CoverUrl")]
    pub cover_url: Option<String>,

    #[graphql(name = "HasFiles")]
    #[filterable(type = "boolean")]
    pub has_files: bool,

    #[graphql(name = "SizeBytes")]
    #[filterable(type = "number")]
    #[sortable]
    pub size_bytes: Option<i64>,

    #[graphql(name = "Path")]
    pub path: Option<String>,

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
    #[relation(target = "Chapter", from = "id", to = "audiobook_id", multiple)]
    pub chapters: Vec<Chapter>,
}

#[derive(Default)]
pub struct AudiobookCustomOperations;

// // ============================================================================
// // Custom Operations (non-CRUD - external API calls)
// // ============================================================================

// /// Input for adding an audiobook from OpenLibrary
// #[derive(Debug, InputObject)]
// pub struct AddAudiobookFromOpenLibraryInput {
//     /// OpenLibrary work ID
//     #[graphql(name = "OpenlibraryId")]
//     pub openlibrary_id: String,
// }

// /// Result of audiobook operations
// #[derive(Debug, SimpleObject)]
// #[graphql(name = "AudiobookOperationResult")]
// pub struct AudiobookOperationResult {
//     #[graphql(name = "Success")]
//     pub success: bool,
//     #[graphql(name = "Audiobook")]
//     pub audiobook: Option<AudiobookEntity>,
//     #[graphql(name = "Error")]
//     pub error: Option<String>,
// }

// /// Custom audiobook operations that require external services
// ///
// /// These operations CAN'T be replaced by generated CRUD:
// /// - SearchAudiobooks: Searches external OpenLibrary API
// /// - AddAudiobookFromOpenLibrary: Fetches metadata from OpenLibrary
// #[derive(Default)]
// pub struct AudiobookCustomOperations;

// #[Object]
// impl AudiobookCustomOperations {
//     /// Search for audiobooks on OpenLibrary
//     #[graphql(name = "SearchAudiobooks")]
//     async fn search_audiobooks(
//         &self,
//         ctx: &Context<'_>,
//         query: String,
//     ) -> Result<Vec<AudiobookSearchResult>> {
//         let _user = ctx.auth_user()?;
//         let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

//         let results = metadata
//             .search_audiobooks(&query)
//             .await
//             .map_err(|e| async_graphql::Error::new(e.to_string()))?;

//         Ok(results
//             .into_iter()
//             .map(|a| AudiobookSearchResult {
//                 provider: "openlibrary".to_string(),
//                 provider_id: a.provider_id,
//                 title: a.title,
//                 author_name: a.author_name,
//                 year: a.year,
//                 cover_url: a.cover_url,
//                 isbn: a.isbn,
//                 description: a.description,
//             })
//             .collect())
//     }

//     /// Add an audiobook to a library from OpenLibrary
//     #[graphql(name = "AddAudiobookFromOpenLibrary")]
//     async fn add_audiobook_from_openlibrary(
//         &self,
//         ctx: &Context<'_>,
//         library_id: String,
//         input: AddAudiobookFromOpenLibraryInput,
//     ) -> Result<AudiobookOperationResult> {
//         use super::{LibraryEntity, AppSettingEntity, AppSettingEntityWhereInput};
//         use crate::graphql::filters::StringFilter;
//         use crate::graphql::orm::EntityQuery;
//         use crate::graphql::helpers::library_entity_to_graphql;

//         let user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>().clone();
//         let metadata = ctx.data_unchecked::<Arc<MetadataService>>();
//         let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>().clone();

//         let lib_id = Uuid::parse_str(&library_id)
//             .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
//         let user_id = Uuid::parse_str(&user.user_id)
//             .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

//         // Verify library exists using entity query
//         let library = LibraryEntity::get(db.pool(), &library_id)
//             .await
//             .map_err(|e| async_graphql::Error::new(e.to_string()))?
//             .ok_or_else(|| async_graphql::Error::new("Library not found"))?;

//         use crate::services::metadata::AddAudiobookOptions;
//         match metadata
//             .add_audiobook_from_provider(AddAudiobookOptions {
//                 openlibrary_id: input.openlibrary_id.clone(),
//                 library_id: lib_id,
//                 user_id,
//                 monitored: true,
//             })
//             .await
//         {
//             Ok(record) => {
//                 // Fetch created audiobook as entity
//                 let audiobook_id = record.id.to_string();
//                 let audiobook_entity = AudiobookEntity::get(db.pool(), &audiobook_id)
//                     .await
//                     .map_err(|e| async_graphql::Error::new(format!("Failed to fetch audiobook: {}", e)))?
//                     .ok_or_else(|| async_graphql::Error::new("Audiobook not found after creation"))?;

//                 tracing::info!(
//                     user_id = %user.user_id,
//                     audiobook_title = %audiobook_entity.title,
//                     audiobook_id = %audiobook_entity.id,
//                     library_id = %lib_id,
//                     "User added audiobook from OpenLibrary: {}",
//                     audiobook_entity.title
//                 );

//                 // Broadcast library change event
//                 if let Ok(tx) = ctx.data::<broadcast::Sender<LibraryChangedEvent>>() {
//                     let _ = tx.send(LibraryChangedEvent {
//                         change_type: LibraryChangeType::Updated,
//                         library_id: library_id.clone(),
//                         library_name: Some(library.name.clone()),
//                         library: Some(library_entity_to_graphql(library.clone())),
//                     });
//                 }

//                 // Trigger auto-hunt if enabled
//                 if library.auto_hunt {
//                     spawn_audiobook_auto_hunt(db, library, audiobook_entity.clone(), user_id, torrent_service);
//                 }

//                 Ok(AudiobookOperationResult {
//                     success: true,
//                     audiobook: Some(audiobook_entity),
//                     error: None,
//                 })
//             }
//             Err(e) => Ok(AudiobookOperationResult {
//                 success: false,
//                 audiobook: None,
//                 error: Some(e.to_string()),
//             }),
//         }
//     }
// }

// // ============================================================================
// // Helper Functions
// // ============================================================================

// fn spawn_audiobook_auto_hunt(
//     db: Database,
//     library: super::LibraryEntity,
//     audiobook: AudiobookEntity,
//     user_id: Uuid,
//     torrent_service: Arc<TorrentService>,
// ) {
//     use super::{AppSettingEntity, AppSettingEntityWhereInput};
//     use crate::graphql::filters::StringFilter;
//     use crate::graphql::orm::EntityQuery;

//     tokio::spawn(async move {
//         // Get encryption key using entity query
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

//         // Use entity-based hunt function
//         let _ = crate::jobs::auto_hunt::hunt_single_audiobook_entity(
//             &db,
//             &audiobook,
//             &library,
//             &torrent_service,
//             &indexer_manager,
//         )
//         .await;
//     });
// }
