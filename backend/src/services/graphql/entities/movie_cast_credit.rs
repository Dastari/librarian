use async_graphql::SimpleObject;
use macros::{GraphQLEntity, GraphQLOperations};
use serde::{Deserialize, Serialize};

use super::movie::Movie;
use super::person::Person;

#[derive(GraphQLEntity, GraphQLOperations, SimpleObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(name = "MovieCastCredit")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "movie_cast_credits",
    plural = "MovieCastCredits",
    default_sort = "cast_order"
)]
pub struct MovieCastCredit {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "MovieId")]
    #[filterable(type = "string")]
    pub movie_id: String,

    #[graphql(name = "PersonId")]
    #[filterable(type = "string")]
    pub person_id: String,

    #[graphql(name = "CharacterName")]
    #[filterable(type = "string")]
    pub character_name: Option<String>,

    #[graphql(name = "CastOrder")]
    #[filterable(type = "number")]
    #[sortable]
    pub cast_order: Option<i32>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,

    #[graphql(name = "Movie")]
    #[relation(target = "Movie", from = "movie_id", to = "id")]
    pub movie: Option<Movie>,

    #[graphql(name = "Person")]
    #[relation(target = "Person", from = "person_id", to = "id")]
    pub person: Option<Person>,
}

#[derive(Default)]
pub struct MovieCastCreditCustomOperations;
