use graphql_orm::{GraphQLEntity, GraphQLOperations};
use serde::{Deserialize, Serialize};

#[derive(GraphQLEntity, GraphQLOperations, Clone, Debug, Serialize, Deserialize)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "artwork_cache",
    plural = "ArtworkCaches",
    default_sort = "created_at"
)]
pub struct ArtworkCache {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "entityType")]
    #[filterable(type = "string")]
    #[sortable]
    pub entity_type: String,

    #[graphql(name = "entityId")]
    #[filterable(type = "string")]
    pub entity_id: String,

    #[graphql(name = "artworkType")]
    #[filterable(type = "string")]
    #[sortable]
    pub artwork_type: String,

    #[graphql(name = "storageObjectId")]
    #[graphql_orm(default = "''")]
    #[filterable(type = "string")]
    pub storage_object_id: String,

    #[graphql(name = "sourceUrl")]
    pub source_url: Option<String>,

    #[graphql(name = "width")]
    #[filterable(type = "number")]
    pub width: Option<i32>,

    #[graphql(name = "height")]
    #[filterable(type = "number")]
    pub height: Option<i32>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

#[derive(Default)]
pub struct ArtworkCacheCustomOperations;
