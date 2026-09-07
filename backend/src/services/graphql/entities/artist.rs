use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use super::album::Album;
use super::library::Library;

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
#[graphql_entity(table = "artists", plural = "Artists", default_sort = "name")]
pub struct Artist {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "libraryId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owned.link")]
    pub library_id: String,

    #[graphql(name = "userId")]
    #[filterable(type = "string")]
    #[graphql_orm(write_policy = "owner.id")]
    pub user_id: String,

    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "sortName")]
    #[sortable]
    pub sort_name: Option<String>,

    #[graphql(name = "musicbrainzId")]
    #[filterable(type = "string")]
    pub musicbrainz_id: Option<String>,

    #[graphql(name = "bio")]
    pub bio: Option<String>,

    #[graphql(name = "disambiguation")]
    pub disambiguation: Option<String>,

    #[graphql(name = "imageUrl")]
    pub image_url: Option<String>,

    #[graphql(name = "albumCount")]
    #[filterable(type = "number")]
    #[sortable]
    pub album_count: Option<i32>,

    #[graphql(name = "trackCount")]
    #[filterable(type = "number")]
    pub track_count: Option<i32>,

    #[graphql(name = "totalDurationSecs")]
    #[filterable(type = "number")]
    pub total_duration_secs: Option<i32>,

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
        target = "Library",
        from = "library_id",
        to = "id",
        on_delete = "cascade"
    )]
    pub library: Option<Library>,

    #[graphql(skip)]
    #[serde(skip)]
    #[relation(target = "Album", from = "id", to = "artist_id", multiple)]
    pub albums: Vec<Album>,
}

#[derive(Default)]
pub struct ArtistCustomOperations;
