use async_graphql::SimpleObject;
use macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use super::library::Library;
use super::movie::Movie;

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
#[graphql(name = "Collection", complex)]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "collections", plural = "Collections", default_sort = "name")]
pub struct Collection {
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

    #[graphql(name = "TmdbCollectionId")]
    #[filterable(type = "number")]
    pub tmdb_collection_id: i32,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "Overview")]
    pub overview: Option<String>,

    #[graphql(name = "PosterUrl")]
    pub poster_url: Option<String>,

    #[graphql(name = "BackdropUrl")]
    pub backdrop_url: Option<String>,

    #[graphql(name = "MovieCount")]
    #[filterable(type = "number")]
    #[sortable]
    pub movie_count: i32,

    #[graphql(name = "LastSyncedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_synced_at: Option<String>,

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
    #[relation(target = "Library", from = "library_id", to = "id")]
    pub library: Option<Library>,

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "Movie",
        from = "tmdb_collection_id",
        to = "collection_id",
        multiple
    )]
    pub movies: Vec<Movie>,
}

#[derive(Default)]
pub struct CollectionCustomOperations;
