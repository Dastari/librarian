use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

/// The optional service asked to settle release matches that names, tags and lengths can't. There is
/// one row at most.
#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "matching_assistants",
    plural = "MatchingAssistants",
    default_sort = "created_at",
    read_policy = "admin.read",
    write_policy = "admin.write"
)]
pub struct MatchingAssistant {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "enabled")]
    #[boolean_field]
    #[filterable(type = "boolean")]
    pub enabled: bool,

    #[graphql(name = "model")]
    pub model: String,

    // Encrypted by the database write transform, as `Source.credentials` is. It can be written but
    // never read back through GraphQL, and is marked sensitive so nothing exports it.
    #[graphql(skip)]
    #[input_only]
    #[graphql_orm(sensitive)]
    pub api_key: String,

    /// Whether a key is stored, since the key itself can't be read. Set by the write transform.
    #[graphql(name = "hasApiKey")]
    #[boolean_field]
    pub has_api_key: bool,

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
pub struct MatchingAssistantCustomOperations;
