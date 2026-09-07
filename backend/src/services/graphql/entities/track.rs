// GraphQLRelations expands a compatibility let-else pattern that cannot be
// rewritten at this entity call site.
#![allow(clippy::question_mark)]

use crate::graphql::entities::*;
use async_graphql::Context;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::services::graphql::AuthUser;

use super::album::Album;
use super::library::Library;
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
    table = "tracks",
    plural = "Tracks",
    default_sort = "track_number",
    index = "album_id",
    index = "artist_id",
    index = "media_file_id"
)]
pub struct Track {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "albumId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub album_id: String,

    #[graphql(name = "libraryId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub library_id: String,

    #[graphql(name = "title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: String,

    #[graphql(name = "trackNumber")]
    #[filterable(type = "number")]
    #[sortable]
    pub track_number: i32,

    #[graphql(name = "discNumber")]
    #[filterable(type = "number")]
    #[sortable]
    pub disc_number: Option<i32>,

    #[graphql(name = "musicbrainzId")]
    #[filterable(type = "string")]
    pub musicbrainz_id: Option<String>,

    #[graphql(name = "isrc")]
    #[filterable(type = "string")]
    pub isrc: Option<String>,

    #[graphql(name = "durationSecs")]
    #[filterable(type = "number")]
    #[sortable]
    pub duration_secs: Option<i32>,

    #[graphql(name = "explicit")]
    #[filterable(type = "boolean")]
    pub explicit: bool,

    #[graphql(name = "artistName")]
    #[filterable(type = "string")]
    pub artist_name: Option<String>,

    #[graphql(name = "artistId")]
    #[filterable(type = "string")]
    pub artist_id: Option<String>,

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
    #[relation(target = "Album", from = "album_id", to = "id", on_delete = "cascade")]
    pub album: Option<Album>,
    #[graphql(skip)]
    #[serde(skip)]
    #[relation(
        target = "Library",
        from = "library_id",
        to = "id",
        on_delete = "cascade"
    )]
    pub library: Option<Library>,

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
pub struct TrackCustomOperations;
