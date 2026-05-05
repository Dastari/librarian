use super::common::{ContentStatus, ContentType, calculate_content_status};
use super::media_file::MediaFile;
use crate::graphql::entities::*;
use async_graphql::Context;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::services::graphql::AuthUser;

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
#[graphql_entity(table = "episodes", plural = "Episodes", default_sort = "season")]
pub struct Episode {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "ShowId")]
    #[filterable(type = "string")]
    pub show_id: String,

    #[graphql(name = "Season")]
    #[filterable(type = "number")]
    #[sortable]
    pub season: i32,

    #[graphql(name = "Episode")]
    #[filterable(type = "number")]
    #[sortable]
    pub episode: i32,

    #[graphql(name = "AbsoluteNumber")]
    #[filterable(type = "number")]
    pub absolute_number: Option<i32>,

    #[graphql(name = "Title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: Option<String>,

    #[graphql(name = "Overview")]
    pub overview: Option<String>,

    #[graphql(name = "AirDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub air_date: Option<String>,

    #[graphql(name = "Runtime")]
    #[filterable(type = "number")]
    pub runtime: Option<i32>,

    #[graphql(name = "TvmazeId")]
    #[filterable(type = "number")]
    pub tvmaze_id: Option<i32>,

    #[graphql(name = "TmdbId")]
    #[filterable(type = "number")]
    pub tmdb_id: Option<i32>,

    #[graphql(name = "TvdbId")]
    #[filterable(type = "number")]
    pub tvdb_id: Option<i32>,

    #[graphql(name = "Wanted")]
    #[filterable(type = "boolean")]
    pub wanted: bool,

    #[graphql(name = "MediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: Option<String>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
    #[graphql(skip)]
    #[serde(skip)]
    #[relation(target = "MediaFile", from = "media_file_id", to = "id")]
    pub media_file: Option<MediaFile>,
}

#[derive(Default)]
pub struct EpisodeCustomOperations;
