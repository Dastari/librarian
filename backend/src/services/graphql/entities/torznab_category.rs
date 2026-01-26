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
#[graphql(name = "TorznabCategory")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "torznab_categories",
    plural = "TorznabCategories",
    default_sort = "id"
)]
pub struct TorznabCategory {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "number")]
    #[sortable]
    pub id: i32,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "ParentId")]
    #[filterable(type = "number")]
    pub parent_id: Option<i32>,

    #[graphql(name = "Description")]
    pub description: Option<String>,
}

#[derive(Default)]
pub struct TorznabCategoryCustomOperations;
