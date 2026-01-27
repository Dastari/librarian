//! Library Entity
//!
//! This module contains the Library entity with macro-generated relations.
//! Relations use DataLoader batching to avoid N+1 queries.

use async_graphql::SimpleObject;
use librarian_macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use super::album::Album;
use super::artist::Artist;
use super::audiobook::Audiobook;
use super::media_file::MediaFile;
use super::movie::Movie;
use super::show::Show;

/// Library entity representing a media library.
///
/// Relations (Shows, Movies, Artists, etc.) are automatically generated
/// by the GraphQLRelations macro and use DataLoader for N+1 prevention.
#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, SimpleObject, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(name = "Library", complex)]
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

    // ========================================================================
    // Relations - These generate ComplexObject resolvers via GraphQLRelations
    // ========================================================================
    //
    // Each relation:
    // - Exposes a GraphQL field with Where/OrderBy/Page args
    // - Uses DataLoader for batching when no args provided (N+1 free)
    // - Falls back to direct SQL query when args provided (full filter support)

    /// Shows in this library
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(target = "Show", to = "library_id", multiple)]
    pub shows: Vec<Show>,

    /// Movies in this library
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(target = "Movie", to = "library_id", multiple)]
    pub movies: Vec<Movie>,

    /// Albums in this library
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(target = "Album", to = "library_id", multiple)]
    pub albums: Vec<Album>,

    /// Audiobooks in this library
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(target = "Audiobook", to = "library_id", multiple)]
    pub audiobooks: Vec<Audiobook>,

    /// Media files in this library
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(target = "MediaFile", to = "library_id", multiple)]
    pub media_files: Vec<MediaFile>,
}

#[derive(Default)]
pub struct LibraryCustomOperations;

// Custom operations (ScanLibrary, ConsolidateLibrary, etc.) can be added here
// as an #[Object] impl on LibraryCustomOperations when needed.
