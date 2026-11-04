use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

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

    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "TorznabCategory",
        from = "parent_id",
        to = "id",
        on_delete = "set_null",
        emit_fk = false
    )]
    pub parent: Option<Box<TorznabCategory>>,
}

#[derive(Default)]
pub struct TorznabCategoryCustomOperations;
