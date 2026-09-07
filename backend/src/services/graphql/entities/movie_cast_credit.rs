use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use super::movie::Movie;
use super::person::Person;

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
    table = "movie_cast_credits",
    plural = "MovieCastCredits",
    default_sort = "cast_order",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct MovieCastCredit {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "movieId")]
    #[filterable(type = "string")]
    pub movie_id: String,

    #[graphql(name = "personId")]
    #[filterable(type = "string")]
    pub person_id: String,

    #[graphql(name = "characterName")]
    #[filterable(type = "string")]
    pub character_name: Option<String>,

    #[graphql(name = "castOrder")]
    #[filterable(type = "number")]
    #[sortable]
    pub cast_order: Option<i32>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
    #[graphql(skip)]
    #[serde(skip)]
    #[relation(target = "Movie", from = "movie_id", to = "id", on_delete = "cascade")]
    pub movie: Option<Movie>,
    #[graphql(skip)]
    #[serde(skip)]
    #[relation(
        target = "Person",
        from = "person_id",
        to = "id",
        on_delete = "cascade"
    )]
    pub person: Option<Person>,
}

#[derive(Default)]
pub struct MovieCastCreditCustomOperations;
