use async_graphql::SimpleObject;
use librarian_macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use super::media_file::MediaFile;

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
