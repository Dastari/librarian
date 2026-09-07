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
    table = "source_priority_rules",
    plural = "SourcePriorityRules",
    default_sort = "created_at",
    read_policy = "admin.read",
    write_policy = "admin.write"
)]
pub struct SourcePriorityRule {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "libraryType")]
    #[filterable(type = "string")]
    pub library_type: Option<String>,

    #[graphql(name = "libraryId")]
    #[filterable(type = "string")]
    pub library_id: Option<String>,

    #[graphql(name = "priorityOrder")]
    #[json_field]
    pub priority_order: Vec<String>,

    #[graphql(name = "searchAllSources")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub search_all_sources: bool,

    #[graphql(name = "enabled")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub enabled: bool,

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
pub struct SourcePriorityRuleCustomOperations;
