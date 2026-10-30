// GraphQLRelations expands a compatibility let-else pattern that cannot be
// rewritten at this entity call site.
#![allow(clippy::question_mark)]

//! Season entity: provider metadata for one season of a show (name, dates, episode count)
//! and the source of its poster. The poster itself is cached into object storage by the
//! artwork service and served from `/api/artwork/season/{id}/poster`.

use super::show::Show;
use crate::graphql::entities::*;
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
    table = "seasons",
    plural = "Seasons",
    default_sort = "season",
    read_policy = "member.read",
    write_policy = "admin.write",
    unique_composite = "show_id,season",
    unique_index = "show_id,season",
    upsert = "show_id,season"
)]
pub struct Season {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "showId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub show_id: String,

    /// Season number; 0 holds specials.
    #[graphql(name = "season")]
    #[filterable(type = "number")]
    #[sortable]
    pub season: i32,

    #[graphql(name = "tvmazeId")]
    #[filterable(type = "number")]
    pub tvmaze_id: Option<i32>,

    /// Provider title, when the season has one ("The Long Night"); often empty.
    #[graphql(name = "name")]
    pub name: Option<String>,

    #[graphql(name = "overview")]
    pub overview: Option<String>,

    #[graphql(name = "premiereDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub premiere_date: Option<String>,

    #[graphql(name = "endDate")]
    #[filterable(type = "date")]
    pub end_date: Option<String>,

    /// Announced number of episodes, which can exceed the episodes listed so far.
    #[graphql(name = "episodeOrder")]
    pub episode_order: Option<i32>,

    /// Provider poster URL: the source the artwork cache downloads from. Clients use the
    /// cached `/api/artwork/season/{id}/poster`, never this remote address.
    #[graphql(name = "posterUrl")]
    pub poster_url: Option<String>,

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
    #[relation(
        target = "Show",
        from = "show_id",
        to = "id",
        on_delete = "cascade",
        propagate_change = "up"
    )]
    pub show: Option<Show>,
}

#[derive(Default)]
pub struct SeasonCustomOperations;
