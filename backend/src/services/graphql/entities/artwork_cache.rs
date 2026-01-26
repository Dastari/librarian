use async_graphql::SimpleObject;
use librarian_macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

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
#[graphql(name = "ArtworkCache")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "artwork_cache",
    plural = "ArtworkCaches",
    default_sort = "created_at"
)]
pub struct ArtworkCache {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "EntityType")]
    #[filterable(type = "string")]
    #[sortable]
    pub entity_type: String,

    #[graphql(name = "EntityId")]
    #[filterable(type = "string")]
    pub entity_id: String,

    #[graphql(name = "ArtworkType")]
    #[filterable(type = "string")]
    #[sortable]
    pub artwork_type: String,

    #[graphql(name = "ContentHash")]
    #[filterable(type = "string")]
    pub content_hash: String,

    #[graphql(name = "MimeType")]
    #[filterable(type = "string")]
    pub mime_type: String,

    #[graphql(skip)]
    #[serde(skip)]
    pub data: Vec<u8>,

    #[graphql(name = "SizeBytes")]
    #[filterable(type = "number")]
    #[sortable]
    pub size_bytes: i64,

    #[graphql(name = "SourceUrl")]
    pub source_url: Option<String>,

    #[graphql(name = "Width")]
    #[filterable(type = "number")]
    pub width: Option<i32>,

    #[graphql(name = "Height")]
    #[filterable(type = "number")]
    pub height: Option<i32>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

#[derive(Default)]
pub struct ArtworkCacheCustomOperations;
