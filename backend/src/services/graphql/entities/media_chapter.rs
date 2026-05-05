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
    table = "media_chapters",
    plural = "MediaChapters",
    default_sort = "chapter_index"
)]
pub struct MediaChapter {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "MediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: String,

    #[graphql(name = "ChapterIndex")]
    #[filterable(type = "number")]
    #[sortable]
    pub chapter_index: i32,

    #[graphql(name = "StartSecs")]
    #[filterable(type = "number")]
    pub start_secs: f64,

    #[graphql(name = "EndSecs")]
    #[filterable(type = "number")]
    pub end_secs: f64,

    #[graphql(name = "Title")]
    #[filterable(type = "string")]
    pub title: Option<String>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,
}

#[derive(Default)]
pub struct MediaChapterCustomOperations;
