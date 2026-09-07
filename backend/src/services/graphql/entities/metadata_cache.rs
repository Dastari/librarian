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
    unique_composite = "provider,operation,cache_key",
    read_policy = "admin.read",
    write_policy = "admin.write"
)]
pub struct MetadataCache {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "provider")]
    #[filterable(type = "string")]
    #[sortable]
    pub provider: String,

    #[graphql(name = "operation")]
    #[filterable(type = "string")]
    #[sortable]
    pub operation: String,

    #[graphql(name = "cacheKey")]
    #[filterable(type = "string")]
    pub cache_key: String,

    #[graphql(name = "payload")]
    pub payload: String,

    #[graphql(name = "payloadVersion")]
    #[filterable(type = "number")]
    pub payload_version: i32,

    #[graphql(name = "fetchedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub fetched_at: String,

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
pub struct MetadataCacheCustomOperations;
