use async_graphql::{Context, SimpleObject};
use librarian_macros::{GraphQLEntity, GraphQLOperations};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::services::graphql::AuthUser;

use super::common::{calculate_content_status, ContentStatus, ContentType};
use super::media_file::MediaFile;
#[derive(
    GraphQLEntity,
    GraphQLOperations,
    SimpleObject,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[graphql(name = "Chapter", complex)]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "chapters",
    plural = "Chapters",
    default_sort = "chapter_number"
)]
pub struct Chapter {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "AudiobookId")]
    #[filterable(type = "string")]
    pub audiobook_id: String,

    #[graphql(name = "ChapterNumber")]
    #[filterable(type = "number")]
    #[sortable]
    pub chapter_number: i32,

    #[graphql(name = "Title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: Option<String>,

    #[graphql(name = "StartTimeSecs")]
    #[filterable(type = "number")]
    pub start_time_secs: f64,

    #[graphql(name = "EndTimeSecs")]
    #[filterable(type = "number")]
    pub end_time_secs: Option<f64>,

    #[graphql(name = "DurationSecs")]
    #[filterable(type = "number")]
    pub duration_secs: Option<i32>,

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
pub struct ChapterCustomOperations;

// ============================================================================
// ComplexObject Resolvers (computed fields)
// ============================================================================

#[async_graphql::ComplexObject]
impl Chapter {
    /// Computed status based on playback, file availability, and download state
    ///
    /// Returns one of: PLAYING, PAUSED, AVAILABLE, DOWNLOADING, WANTED, MISSING
    #[graphql(name = "Status")]
    async fn status(&self, ctx: &Context<'_>) -> ContentStatus {
        let db = match ctx.data::<Database>() {
            Ok(db) => db,
            Err(_) => return ContentStatus::Missing,
        };

        let user_id = match ctx.data::<AuthUser>() {
            Ok(user) => user.user_id.clone(),
            Err(_) => return if self.media_file_id.is_some() {
                ContentStatus::Available
            } else if self.wanted {
                ContentStatus::Wanted
            } else {
                ContentStatus::Missing
            },
        };

        calculate_content_status(
            db,
            ContentType::Chapter,
            &self.id,
            &user_id,
            self.media_file_id.as_deref(),
            self.wanted,
        )
        .await
    }
}
