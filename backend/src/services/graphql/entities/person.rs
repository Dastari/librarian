use async_graphql::SimpleObject;
use macros::{GraphQLEntity, GraphQLOperations};
use serde::{Deserialize, Serialize};

#[derive(GraphQLEntity, GraphQLOperations, SimpleObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(name = "Person")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "people", plural = "People", default_sort = "name")]
pub struct Person {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "TmdbPersonId")]
    #[filterable(type = "number")]
    #[unique]
    pub tmdb_person_id: i32,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "ProfileUrl")]
    pub profile_url: Option<String>,

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
pub struct PersonCustomOperations;
