use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use super::library::Library;
use super::movie::Movie;

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
#[graphql_entity(table = "collections", plural = "Collections", default_sort = "name")]
pub struct Collection {
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

    #[graphql(name = "tmdbCollectionId")]
    #[filterable(type = "number")]
    pub tmdb_collection_id: i32,

    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "overview")]
    pub overview: Option<String>,

    #[graphql(name = "posterUrl")]
    pub poster_url: Option<String>,

    #[graphql(name = "backdropUrl")]
    pub backdrop_url: Option<String>,

    #[graphql(name = "movieCount")]
    #[filterable(type = "number")]
    #[sortable]
    pub movie_count: i32,

    #[graphql(name = "lastSyncedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub last_synced_at: Option<String>,

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
    #[skip_db]
    #[relation(
        target = "Library",
        from = "library_id",
        to = "id",
        on_delete = "cascade"
    )]
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
