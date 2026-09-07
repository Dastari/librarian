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
    table = "torznab_categories",
    plural = "TorznabCategories",
    default_sort = "id",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct TorznabCategory {
    #[graphql(name = "id")]
    #[primary_key]
    #[graphql_orm(auto_generated = false)]
    #[filterable(type = "string")]
    #[sortable]
    pub id: String,

    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "parentId")]
    #[filterable(type = "string")]
    pub parent_id: Option<String>,

    #[graphql(name = "description")]
    pub description: Option<String>,
}

#[derive(Default)]
pub struct TorznabCategoryCustomOperations;
