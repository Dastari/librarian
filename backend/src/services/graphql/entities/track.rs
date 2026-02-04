use async_graphql::{Context, SimpleObject};
use macros::{GraphQLEntity, GraphQLOperations};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::services::graphql::AuthUser;

use super::common::{ContentStatus, ContentType, calculate_content_status};
use super::media_file::MediaFile;

#[derive(GraphQLEntity, GraphQLOperations, SimpleObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(name = "Track", complex)]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "tracks", plural = "Tracks", default_sort = "track_number")]
pub struct Track {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "AlbumId")]
    #[filterable(type = "string")]
    pub album_id: String,

    #[graphql(name = "LibraryId")]
    #[filterable(type = "string")]
    pub library_id: String,

    #[graphql(name = "Title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: String,

    #[graphql(name = "TrackNumber")]
    #[filterable(type = "number")]
    #[sortable]
    pub track_number: i32,

    #[graphql(name = "DiscNumber")]
    #[filterable(type = "number")]
    #[sortable]
    pub disc_number: Option<i32>,

    #[graphql(name = "MusicbrainzId")]
    #[filterable(type = "string")]
    pub musicbrainz_id: Option<String>,

    #[graphql(name = "Isrc")]
    #[filterable(type = "string")]
    pub isrc: Option<String>,

    #[graphql(name = "DurationSecs")]
    #[filterable(type = "number")]
    #[sortable]
    pub duration_secs: Option<i32>,

    #[graphql(name = "Explicit")]
    #[filterable(type = "boolean")]
    pub explicit: bool,

    #[graphql(name = "ArtistName")]
    #[filterable(type = "string")]
    pub artist_name: Option<String>,

    #[graphql(name = "ArtistId")]
    #[filterable(type = "string")]
    pub artist_id: Option<String>,

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
pub struct TrackCustomOperations;

// ============================================================================
// ComplexObject Resolvers (computed fields)
// ============================================================================

#[async_graphql::ComplexObject]
impl Track {
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
            Err(_) => {
                return if self.media_file_id.is_some() {
                    ContentStatus::Available
                } else if self.wanted {
                    ContentStatus::Wanted
                } else {
                    ContentStatus::Missing
                };
            }
        };

        calculate_content_status(
            db,
            ContentType::Track,
            &self.id,
            &user_id,
            self.media_file_id.as_deref(),
            self.wanted,
        )
        .await
    }
}
