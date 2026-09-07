// GraphQLRelations expands a compatibility let-else pattern that cannot be
// rewritten at this entity call site.
#![allow(clippy::question_mark)]

use crate::graphql::entities::*;
use async_graphql::Context;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::services::graphql::AuthUser;

use super::audiobook::Audiobook;
use super::media_file::MediaFile;
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
    table = "chapters",
    plural = "Chapters",
    default_sort = "chapter_number",
    index = "audiobook_id",
    index = "media_file_id"
)]
pub struct Chapter {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "audiobookId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub audiobook_id: String,

    #[graphql(name = "chapterNumber")]
    #[filterable(type = "number")]
    #[sortable]
    pub chapter_number: i32,

    #[graphql(name = "title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: Option<String>,

    #[graphql(name = "startTimeSecs")]
    #[filterable(type = "number")]
    pub start_time_secs: f64,

    #[graphql(name = "endTimeSecs")]
    #[filterable(type = "number")]
    pub end_time_secs: Option<f64>,

    #[graphql(name = "durationSecs")]
    #[filterable(type = "number")]
    pub duration_secs: Option<i32>,

    #[graphql(name = "wanted")]
    #[filterable(type = "boolean")]
    pub wanted: bool,

    /// Explicit user opt-out. `None` is the legacy/default false value.
    #[graphql(name = "ignored")]
    #[filterable(type = "boolean")]
    pub ignored: Option<bool>,

    #[graphql(name = "mediaFileId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
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
    #[relation(
        target = "Audiobook",
        from = "audiobook_id",
        to = "id",
        on_delete = "cascade"
    )]
    pub audiobook: Option<Audiobook>,

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
pub struct ChapterCustomOperations;
