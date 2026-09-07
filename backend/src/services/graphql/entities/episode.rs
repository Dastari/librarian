// GraphQLRelations expands a compatibility let-else pattern that cannot be
// rewritten at this entity call site.
#![allow(clippy::question_mark)]

use super::media_file::MediaFile;
use super::show::Show;
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
#[graphql_entity(
    table = "episodes",
    plural = "Episodes",
    default_sort = "season",
    unique_composite = "show_id,tvmaze_id",
    unique_composite = "show_id,tmdb_id",
    unique_composite = "show_id,tvdb_id",
    unique_composite = "show_id,season,episode",
    unique_index = "show_id,tvmaze_id",
    unique_index = "show_id,tmdb_id",
    unique_index = "show_id,tvdb_id",
    unique_index = "show_id,season,episode",
    index = "media_file_id",
    upsert = "show_id,season,episode"
)]
pub struct Episode {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "showId")]
    #[filterable(type = "string")]
    pub show_id: String,

    #[graphql(name = "season")]
    #[filterable(type = "number")]
    #[sortable]
    pub season: i32,

    #[graphql(name = "episode")]
    #[filterable(type = "number")]
    #[sortable]
    pub episode: i32,

    #[graphql(name = "absoluteNumber")]
    #[filterable(type = "number")]
    pub absolute_number: Option<i32>,

    #[graphql(name = "title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: Option<String>,

    #[graphql(name = "overview")]
    pub overview: Option<String>,

    #[graphql(name = "airDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub air_date: Option<String>,

    /// Provider-supplied instant, including timezone; absent when not announced.
    #[graphql(name = "airStamp")]
    #[filterable(type = "date")]
    #[sortable]
    pub air_stamp: Option<String>,

    #[graphql(name = "runtime")]
    #[filterable(type = "number")]
    pub runtime: Option<i32>,

    #[graphql(name = "tvmazeId")]
    #[filterable(type = "number")]
    pub tvmaze_id: Option<i32>,

    #[graphql(name = "tmdbId")]
    #[filterable(type = "number")]
    pub tmdb_id: Option<i32>,

    #[graphql(name = "tvdbId")]
    #[filterable(type = "number")]
    pub tvdb_id: Option<i32>,

    #[graphql(name = "wanted")]
    #[filterable(type = "boolean")]
    pub wanted: bool,

    /// Explicit user opt-out. `None` is the legacy/default false value.
    #[graphql(name = "ignored")]
    #[filterable(type = "boolean")]
    pub ignored: Option<bool>,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: Option<String>,

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
    #[relation(target = "Show", from = "show_id", to = "id", on_delete = "cascade")]
    pub show: Option<Show>,
    #[graphql(skip)]
    #[serde(skip)]
    #[relation(
        target = "MediaFile",
        from = "media_file_id",
        to = "id",
        on_delete = "set_null"
    )]
    pub media_file: Option<MediaFile>,
}

#[derive(Default)]
pub struct EpisodeCustomOperations;
