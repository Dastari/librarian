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
#[graphql(name = "SourcePriorityRule")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "source_priority_rules",
    plural = "SourcePriorityRules",
    default_sort = "created_at"
)]
pub struct SourcePriorityRule {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "LibraryType")]
    #[filterable(type = "string")]
    pub library_type: Option<String>,

    #[graphql(name = "LibraryId")]
    #[filterable(type = "string")]
    pub library_id: Option<String>,

    #[graphql(name = "PriorityOrder")]
    #[json_field]
    pub priority_order: Vec<String>,

    #[graphql(name = "SearchAllSources")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub search_all_sources: bool,

    #[graphql(name = "Enabled")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub enabled: bool,

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
pub struct SourcePriorityRuleCustomOperations;
