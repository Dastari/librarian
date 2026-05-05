use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "metadata_cache",
    plural = "MetadataCaches",
    default_sort = "updated_at",
    unique_composite = "provider,operation,cache_key"
)]
pub struct MetadataCache {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "Provider")]
    #[filterable(type = "string")]
    #[sortable]
    pub provider: String,

    #[graphql(name = "Operation")]
    #[filterable(type = "string")]
    #[sortable]
    pub operation: String,

    #[graphql(name = "CacheKey")]
    #[filterable(type = "string")]
    pub cache_key: String,

    #[graphql(name = "Payload")]
    pub payload: String,

    #[graphql(name = "PayloadVersion")]
    #[filterable(type = "number")]
    pub payload_version: i32,

    #[graphql(name = "FetchedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub fetched_at: String,

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
pub struct MetadataCacheCustomOperations;
