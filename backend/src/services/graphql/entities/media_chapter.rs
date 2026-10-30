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
    table = "media_chapters",
    plural = "MediaChapters",
    default_sort = "chapter_index",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct MediaChapter {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub media_file_id: String,

    #[graphql(name = "chapterIndex")]
    #[filterable(type = "number")]
    #[sortable]
    pub chapter_index: i32,

    #[graphql(name = "startSecs")]
    #[filterable(type = "number")]
    pub start_secs: f64,

    #[graphql(name = "endSecs")]
    #[filterable(type = "number")]
    pub end_secs: f64,

    #[graphql(name = "title")]
    #[filterable(type = "string")]
    pub title: Option<String>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    // Query relations do not add physical FKs or change existing delete behavior.
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(
        target = "MediaFile",
        from = "media_file_id",
        to = "id",
        on_delete = "cascade",
        propagate_change = "up",
        emit_fk = false
    )]
    pub media_file: Option<Box<MediaFile>>,
}

#[derive(Default)]
pub struct MediaChapterCustomOperations;
