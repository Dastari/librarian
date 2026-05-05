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
    default_sort = "id"
)]
pub struct TorznabCategory {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    #[sortable]
    pub id: String,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "ParentId")]
    #[filterable(type = "string")]
    pub parent_id: Option<String>,

    #[graphql(name = "Description")]
    pub description: Option<String>,
}

#[derive(Default)]
pub struct TorznabCategoryCustomOperations;
