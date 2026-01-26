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
#[graphql(name = "ScheduleCache")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "schedule_cache",
    plural = "ScheduleCaches",
    default_sort = "air_date"
)]
pub struct ScheduleCache {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "TvmazeEpisodeId")]
    #[filterable(type = "number")]
    pub tvmaze_episode_id: i32,

    #[graphql(name = "EpisodeName")]
    #[filterable(type = "string")]
    #[sortable]
    pub episode_name: String,

    #[graphql(name = "Season")]
    #[filterable(type = "number")]
    #[sortable]
    pub season: i32,

    #[graphql(name = "EpisodeNumber")]
    #[filterable(type = "number")]
    #[sortable]
    pub episode_number: i32,

    #[graphql(name = "EpisodeType")]
    #[filterable(type = "string")]
    pub episode_type: Option<String>,

    #[graphql(name = "AirDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub air_date: String,

    #[graphql(name = "AirTime")]
    pub air_time: Option<String>,

    #[graphql(name = "AirStamp")]
    #[filterable(type = "date")]
    pub air_stamp: Option<String>,

    #[graphql(name = "Runtime")]
    #[filterable(type = "number")]
    pub runtime: Option<i32>,

    #[graphql(name = "EpisodeImageUrl")]
    pub episode_image_url: Option<String>,

    #[graphql(name = "Summary")]
    pub summary: Option<String>,

    #[graphql(name = "TvmazeShowId")]
    #[filterable(type = "number")]
    pub tvmaze_show_id: i32,

    #[graphql(name = "ShowName")]
    #[filterable(type = "string")]
    #[sortable]
    pub show_name: String,

    #[graphql(name = "ShowNetwork")]
    #[filterable(type = "string")]
    pub show_network: Option<String>,

    #[graphql(name = "ShowPosterUrl")]
    pub show_poster_url: Option<String>,

    #[graphql(name = "ShowGenres")]
    #[json_field]
    pub show_genres: Vec<String>,

    #[graphql(name = "CountryCode")]
    #[filterable(type = "string")]
    pub country_code: String,

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
pub struct ScheduleCacheCustomOperations;
