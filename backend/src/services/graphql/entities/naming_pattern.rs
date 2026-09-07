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
    table = "naming_patterns",
    plural = "NamingPatterns",
    default_sort = "name",
    unique_composite = "library_type,name",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct NamingPattern {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,

    #[graphql(name = "libraryType")]
    #[filterable(type = "string")]
    #[sortable]
    pub library_type: String,

    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "pattern")]
    pub pattern: String,

    #[graphql(name = "description")]
    pub description: Option<String>,

    #[graphql(name = "isDefault")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_default: bool,

    #[graphql(name = "isSystem")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_system: bool,

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
pub struct NamingPatternCustomOperations;
