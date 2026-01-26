use async_graphql::{Result, SimpleObject};
use librarian_macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use super::track::Track;
use crate::{
    db::Database,
    graphql::{
        entities::{TrackOrderByInput, TrackWhereInput},
        orm::{EntityQuery, StringFilter},
    },
};

/// Album Entity
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
#[graphql(name = "Album", complex)]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "albums",
    plural = "Albums",
    default_sort = "name",
    notify = "libraries"
)]

pub struct Album {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "ArtistId")]
    #[filterable(type = "string")]
    pub artist_id: String,

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

    #[graphql(name = "MusicbrainzId")]
    #[filterable(type = "string")]
    pub musicbrainz_id: Option<String>,

    #[graphql(name = "AlbumType")]
    #[filterable(type = "string")]
    pub album_type: Option<String>,

    #[graphql(name = "Genres")]
    #[json_field]
    pub genres: Vec<String>,

    #[graphql(name = "Label")]
    #[filterable(type = "string")]
    pub label: Option<String>,

    #[graphql(name = "Country")]
    #[filterable(type = "string")]
    pub country: Option<String>,

    #[graphql(name = "ReleaseDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub release_date: Option<String>,

    #[graphql(name = "CoverUrl")]
    pub cover_url: Option<String>,

    #[graphql(name = "TrackCount")]
    #[filterable(type = "number")]
    pub track_count: Option<i32>,

    #[graphql(name = "DiscCount")]
    #[filterable(type = "number")]
    pub disc_count: Option<i32>,

    #[graphql(name = "TotalDurationSecs")]
    #[filterable(type = "number")]
    pub total_duration_secs: Option<i32>,

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
    #[skip_db]
    pub tracks: Vec<Track>,
}

#[async_graphql::ComplexObject]
impl Album {
    /// Tracks in this album
    #[graphql(name = "Tracks")]
    async fn tracks(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<Vec<Track>> {
        let where_input = TrackWhereInput {
            album_id: Some(StringFilter::eq(&self.id)),
            ..Default::default()
        };
        let order_by = TrackOrderByInput::default();
        let tracks = EntityQuery::<Track>::new()
            .filter(&where_input)
            .order_by(&order_by)
            .fetch_all(ctx.data_unchecked::<Database>())
            .await?;

        Ok(tracks)
    }
}

// // ============================================================================
// // Custom Operations (non-CRUD - external API calls)
// // ============================================================================

// /// Input for adding an album from MusicBrainz
// #[derive(Debug, InputObject)]
// pub struct AddAlbumFromMusicBrainzInput {
//     /// MusicBrainz release group ID
//     #[graphql(name = "MusicbrainzId")]
//     pub musicbrainz_id: String,
// }

// /// Result of album operations
// #[derive(Debug, SimpleObject)]
// #[graphql(name = "AlbumOperationResult")]
// pub struct AlbumOperationResult {
//     #[graphql(name = "Success")]
//     pub success: bool,
//     #[graphql(name = "Album")]
//     pub album: Option<Album>,
//     #[graphql(name = "Error")]
//     pub error: Option<String>,
// }

// /// Custom album operations that require external services
// ///
// /// These operations CAN'T be replaced by generated CRUD:
// /// - SearchAlbums: Searches external MusicBrainz API
// /// - AddAlbumFromMusicBrainz: Fetches metadata from MusicBrainz

#[derive(Default)]
pub struct AlbumCustomOperations;

// #[Object]
// impl AlbumCustomOperations {
//     /// Search for albums on MusicBrainz
//     ///
//     /// By default, only searches for Albums. Use the include flags to also search for:
//     /// - include_eps: Include EPs
//     /// - include_singles: Include Singles
//     /// - include_compilations: Include Compilations
//     /// - include_live: Include Live albums
//     /// - include_soundtracks: Include Soundtracks
//     #[graphql(name = "SearchAlbums")]
//     async fn search_albums(
//         &self,
//         ctx: &Context<'_>,
//         query: String,
//         #[graphql(name = "IncludeEps", default = false)] include_eps: bool,
//         #[graphql(name = "IncludeSingles", default = false)] include_singles: bool,
//         #[graphql(name = "IncludeCompilations", default = false)] include_compilations: bool,
//         #[graphql(name = "IncludeLive", default = false)] include_live: bool,
//         #[graphql(name = "IncludeSoundtracks", default = false)] include_soundtracks: bool,
//     ) -> Result<Vec<AlbumSearchResult>> {
//         let _user = ctx.auth_user()?;
//         let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

//         let mut types = vec!["Album".to_string()];
//         if include_eps {
//             types.push("EP".to_string());
//         }
//         if include_singles {
//             types.push("Single".to_string());
//         }
//         if include_compilations {
//             types.push("Compilation".to_string());
//         }
//         if include_live {
//             types.push("Live".to_string());
//         }
//         if include_soundtracks {
//             types.push("Soundtrack".to_string());
//         }

//         let results = metadata
//             .search_albums_with_types(&query, &types)
//             .await
//             .map_err(|e| async_graphql::Error::new(e.to_string()))?;

//         Ok(results
//             .into_iter()
//             .map(|a| AlbumSearchResult {
//                 provider: "musicbrainz".to_string(),
//                 provider_id: a.provider_id.to_string(),
//                 title: a.title,
//                 artist_name: a.artist_name,
//                 year: a.year,
//                 album_type: a.album_type,
//                 cover_url: a.cover_url,
//                 score: a.score,
//             })
//             .collect())
//     }

//     /// Add an album to a library from MusicBrainz
//     #[graphql(name = "AddAlbumFromMusicBrainz")]
//     async fn add_album_from_musicbrainz(
//         &self,
//         ctx: &Context<'_>,
//         library_id: String,
//         input: AddAlbumFromMusicBrainzInput,
//     ) -> Result<AlbumOperationResult> {
//         // use super::{LibraryEntity, AppSettingEntity, AppSettingEntityWhereInput};
//         // use crate::graphql::filters::StringFilter;
//         // use crate::graphql::orm::EntityQuery;
//         use crate::graphql::helpers::library_entity_to_graphql;

//         let user = ctx.auth_user()?;
//         let db = ctx.data_unchecked::<Database>().clone();
//         // let metadata = ctx.data_unchecked::<Arc<MetadataService>>();
//         // let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>().clone();

//         let lib_id = Uuid::parse_str(&library_id)
//             .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
//         let user_id = Uuid::parse_str(&user.user_id)
//             .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;
//         let mbid = Uuid::parse_str(&input.musicbrainz_id)
//             .map_err(|e| async_graphql::Error::new(format!("Invalid MusicBrainz ID: {}", e)))?;

//         use crate::services::metadata::AddAlbumOptions;
//         match metadata
//             .add_album_from_provider(AddAlbumOptions {
//                 musicbrainz_id: mbid,
//                 library_id: lib_id,
//                 user_id,
//                 monitored: true,
//             })
//             .await
//         {
//             Ok(record) => {
//                 // Fetch created album as entity
//                 let album_id = record.id.to_string();
//                 let album_entity = AlbumEntity::get(db.pool(), &album_id)
//                     .await
//                     .map_err(|e| async_graphql::Error::new(format!("Failed to fetch album: {}", e)))?
//                     .ok_or_else(|| async_graphql::Error::new("Album not found after creation"))?;

//                 tracing::info!(
//                     user_id = %user.user_id,
//                     album_name = %album_entity.name,
//                     album_id = %album_entity.id,
//                     library_id = %lib_id,
//                     "User added album from MusicBrainz: {}",
//                     album_entity.name
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
//                     spawn_album_auto_hunt(db, library, album_entity.clone(), user_id, torrent_service);
//                 }

//                 Ok(AlbumOperationResult {
//                     success: true,
//                     album: Some(album_entity),
//                     error: None,
//                 })
//             }
//             Err(e) => Ok(AlbumOperationResult {
//                 success: false,
//                 album: None,
//                 error: Some(e.to_string()),
//             }),
//         }
//     }
// }

// // ============================================================================
// // Helper Functions
// // ============================================================================

// fn spawn_album_auto_hunt(
//     db: Database,
//     library: super::LibraryEntity,
//     album: AlbumEntity,
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
//         let _ = crate::jobs::auto_hunt::hunt_single_album_entity(
//             &db,
//             &album,
//             &library,
//             &torrent_service,
//             &indexer_manager,
//         )
//         .await;
//     });
// }
