use async_graphql::SimpleObject;
use librarian_macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use super::album::Album;

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
#[graphql(name = "Artist", complex)]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "artists", plural = "Artists", default_sort = "name")]
pub struct Artist {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "LibraryId")]
    #[filterable(type = "string")]
    pub library_id: String,

    #[graphql(name = "UserId")]
    #[filterable(type = "string")]
    pub user_id: String,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "SortName")]
    #[sortable]
    pub sort_name: Option<String>,

    #[graphql(name = "MusicbrainzId")]
    #[filterable(type = "string")]
    pub musicbrainz_id: Option<String>,

    #[graphql(name = "Bio")]
    pub bio: Option<String>,

    #[graphql(name = "Disambiguation")]
    pub disambiguation: Option<String>,

    #[graphql(name = "ImageUrl")]
    pub image_url: Option<String>,

    #[graphql(name = "AlbumCount")]
    #[filterable(type = "number")]
    #[sortable]
    pub album_count: Option<i32>,

    #[graphql(name = "TrackCount")]
    #[filterable(type = "number")]
    pub track_count: Option<i32>,

    #[graphql(name = "TotalDurationSecs")]
    #[filterable(type = "number")]
    pub total_duration_secs: Option<i32>,

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
    #[relation(target = "Album", from = "id", to = "artist_id", multiple)]
    pub albums: Vec<Album>,
}

#[derive(Default)]
pub struct ArtistCustomOperations;
