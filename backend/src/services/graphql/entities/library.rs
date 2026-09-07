//! Library Entity
//!
//! This module contains the Library entity with macro-generated relations.
//! Relations use DataLoader batching to avoid N+1 queries.

use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use super::album::Album;
use super::artist::Artist;
use super::audiobook::Audiobook;
use super::collection::Collection;
use super::media_file::MediaFile;
use super::movie::Movie;
use super::show::Show;

/// Library entity representing a media library.
///
/// Relations (Shows, Movies, Artists, etc.) are automatically generated
/// by the GraphQLRelations macro and use DataLoader for N+1 prevention.
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
#[graphql_entity(table = "libraries", plural = "Libraries", default_sort = "name")]
pub struct Library {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    #[sortable]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,

    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "path")]
    #[filterable(type = "string")]
    pub path: String,

    #[graphql(name = "libraryType")]
    #[filterable(type = "string")]
    #[sortable]
    pub library_type: String,

    #[graphql(name = "icon")]
    pub icon: Option<String>,

    #[graphql(name = "color")]
    pub color: Option<String>,

    #[graphql(name = "autoScan")]
    #[filterable(type = "boolean")]
    pub auto_scan: bool,

    #[graphql(name = "autoOrganize")]
    #[filterable(type = "boolean")]
    pub auto_organize: bool,

    #[graphql(name = "namingPattern")]
    #[filterable(type = "string")]
    pub naming_pattern: String,

    #[graphql(name = "scanIntervalMinutes")]
    #[filterable(type = "number")]
    pub scan_interval_minutes: i32,

    #[graphql(name = "watchForChanges")]
    #[filterable(type = "boolean")]
    pub watch_for_changes: bool,

    #[graphql(name = "scanning")]
    #[filterable(type = "boolean")]
    pub scanning: bool,

    #[graphql(name = "lastScannedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_scanned_at: Option<String>,

    /// Primary quality profile for this library (docs/tier1-features-plan.md
    /// §2). Resolution precedence: per-entity override (Show/Movie/Album/
    /// Audiobook) > this field > seeded default profile.
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

    /// Collections in this library
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(target = "Collection", to = "library_id", multiple)]
    pub collections: Vec<Collection>,

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
