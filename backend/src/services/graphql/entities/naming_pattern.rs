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
#[graphql(name = "NamingPattern")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "naming_patterns",
    plural = "NamingPatterns",
    default_sort = "name"
)]
pub struct NamingPattern {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "LibraryType")]
    #[filterable(type = "string")]
    #[sortable]
    pub library_type: String,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "Pattern")]
    pub pattern: String,

    #[graphql(name = "Description")]
    pub description: Option<String>,

    #[graphql(name = "IsDefault")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_default: bool,

    #[graphql(name = "IsSystem")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub is_system: bool,

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
pub struct NamingPatternCustomOperations;
