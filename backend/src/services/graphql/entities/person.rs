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
    table = "people",
    plural = "People",
    default_sort = "name",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct Person {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "tmdbPersonId")]
    #[filterable(type = "number")]
    #[unique]
    pub tmdb_person_id: i32,

    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "profileUrl")]
    pub profile_url: Option<String>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,

    /// Movie credits for this person. Deleting the person deletes its credits.
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "MovieCastCredit",
        from = "id",
        to = "person_id",
        multiple,
        emit_fk = false
    )]
    pub cast_credits: Vec<MovieCastCredit>,
}

#[derive(Default)]
pub struct PersonCustomOperations;
