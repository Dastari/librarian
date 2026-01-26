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
#[graphql(name = "IndexerSearchCache")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "indexer_search_cache",
    plural = "IndexerSearchCaches",
    default_sort = "created_at"
)]
pub struct IndexerSearchCache {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "IndexerConfigId")]
    #[filterable(type = "string")]
    pub indexer_config_id: String,

    #[graphql(name = "QueryHash")]
    #[filterable(type = "string")]
    pub query_hash: String,

    #[graphql(name = "QueryType")]
    #[filterable(type = "string")]
    pub query_type: String,

    #[graphql(name = "Results")]
    pub results: String,

    #[graphql(name = "ResultCount")]
    #[filterable(type = "number")]
    pub result_count: i32,

    #[graphql(name = "ExpiresAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub expires_at: String,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,
}

#[derive(Default)]
pub struct IndexerSearchCacheCustomOperations;
