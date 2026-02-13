use async_graphql::{Context, InputObject, Object, Result, SimpleObject};
use macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::common::AutoDownloadMode;
use super::track::Track;
use crate::{
    db::Database,
    graphql::auth::AuthExt,
    graphql::{
        entities::{Library, TrackOrderByInput, TrackWhereInput},
        orm::{EntityQuery, StringFilter},
    },
    services::metadata::providers::{AddAlbumOptions, MetadataProvider, MetadataService},
};

/// Album Entity
#[derive(GraphQLEntity, GraphQLOperations, SimpleObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(name = "Album")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "albums",
    plural = "Albums",
    default_sort = "name",
    notify = "libraries"
)]

pub struct Album {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "ArtistId")]
    #[filterable(type = "string")]
    pub artist_id: String,

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

    #[graphql(name = "Year")]
    #[filterable(type = "number")]
    #[sortable]
    pub year: Option<i32>,

    #[graphql(name = "MusicbrainzId")]
    #[filterable(type = "string")]
    pub musicbrainz_id: Option<String>,

    #[graphql(name = "AlbumType")]
    #[filterable(type = "string")]
    pub album_type: Option<String>,

    #[graphql(name = "Genres")]
    #[json_field]
    pub genres: Vec<String>,

    #[graphql(name = "Label")]
    #[filterable(type = "string")]
    pub label: Option<String>,

    #[graphql(name = "Country")]
    #[filterable(type = "string")]
    pub country: Option<String>,

    #[graphql(name = "ReleaseDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub release_date: Option<String>,

    #[graphql(name = "CoverUrl")]
    pub cover_url: Option<String>,

    #[graphql(name = "TrackCount")]
    #[filterable(type = "number")]
    pub track_count: Option<i32>,

    #[graphql(name = "DiscCount")]
    #[filterable(type = "number")]
    pub disc_count: Option<i32>,

    #[graphql(name = "TotalDurationSecs")]
    #[filterable(type = "number")]
    pub total_duration_secs: Option<i32>,

    #[graphql(name = "AutoDownload")]
    #[filterable(type = "boolean")]
    pub auto_download: bool,

    #[graphql(name = "AutoDownloadMode")]
    pub auto_download_mode: AutoDownloadMode,

    #[graphql(name = "HasFiles")]
    #[filterable(type = "boolean")]
    pub has_files: bool,

    #[graphql(name = "SizeBytes")]
    #[filterable(type = "number")]
    #[sortable]
    pub size_bytes: Option<i64>,

    #[graphql(name = "Path")]
    pub path: Option<String>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,

    #[graphql(name = "Library")]
    #[relation(target = "Library", from = "library_id", to = "id")]
    pub library: Option<Library>,

    /// Tracks in this album
    #[graphql(skip)]
    #[serde(skip)]
    #[skip_db]
    #[relation(target = "Track", to = "album_id", multiple)]
    pub tracks: Vec<Track>,
}
#[derive(Default)]
pub struct AlbumCustomOperations;

/// Search result for MusicBrainz album search.
#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "AlbumSearchResult")]
pub struct AlbumSearchResultGql {
    #[graphql(name = "Provider")]
    pub provider: String,
    #[graphql(name = "ProviderId")]
    pub provider_id: String,
    #[graphql(name = "Title")]
    pub title: String,
    #[graphql(name = "ArtistName")]
    pub artist_name: Option<String>,
    #[graphql(name = "Year")]
    pub year: Option<i32>,
    #[graphql(name = "AlbumType")]
    pub album_type: Option<String>,
    #[graphql(name = "CoverUrl")]
    pub cover_url: Option<String>,
    #[graphql(name = "Score")]
    pub score: Option<f64>,
}

#[Object]
impl AlbumCustomOperations {
    /// Search albums on MusicBrainz.
    #[graphql(name = "SearchAlbums")]
    async fn search_albums(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Query")] query: String,
        #[graphql(name = "IncludeEps", default = false)] include_eps: bool,
        #[graphql(name = "IncludeSingles", default = false)] include_singles: bool,
        #[graphql(name = "IncludeCompilations", default = false)] include_compilations: bool,
        #[graphql(name = "IncludeLive", default = false)] include_live: bool,
        #[graphql(name = "IncludeSoundtracks", default = false)] include_soundtracks: bool,
    ) -> Result<Vec<AlbumSearchResultGql>> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let results = metadata
            .search_albums(
                &query,
                include_eps,
                include_singles,
                include_compilations,
                include_live,
                include_soundtracks,
            )
            .await
            // Preserve the full anyhow chain so frontend/logs show the actual
            // network failure cause (TLS/DNS/timeout/etc), not just top context.
            .map_err(|e| async_graphql::Error::new(format!("{:#}", e)))?;

        Ok(results
            .into_iter()
            .map(|a| AlbumSearchResultGql {
                provider: match a.provider {
                    MetadataProvider::Musicbrainz => "musicbrainz".to_string(),
                    MetadataProvider::Tmdb => "tmdb".to_string(),
                    MetadataProvider::Tvmaze => "tvmaze".to_string(),
                    MetadataProvider::OpenLibrary => "openlibrary".to_string(),
                },
                provider_id: a.provider_id,
                title: a.title,
                artist_name: a.artist_name,
                year: a.year,
                album_type: a.album_type,
                cover_url: a.cover_url,
                score: a.score,
            })
            .collect())
    }
}

#[derive(Debug, InputObject)]
#[graphql(name = "AddAlbumInput")]
pub struct AddAlbumInput {
    #[graphql(name = "LibraryId")]
    pub library_id: String,
    #[graphql(name = "MusicbrainzId")]
    pub musicbrainz_id: String,
}

#[derive(Debug, SimpleObject)]
#[graphql(name = "AlbumOperationResult")]
pub struct AlbumOperationResult {
    #[graphql(name = "Success")]
    pub success: bool,
    #[graphql(name = "Album")]
    pub album: Option<Album>,
    #[graphql(name = "Error")]
    pub error: Option<String>,
}

#[derive(Default)]
pub struct AlbumMetadataMutations;

#[Object]
impl AlbumMetadataMutations {
    /// Add an album to a library by fetching metadata from MusicBrainz.
    #[graphql(name = "AddAlbum")]
    async fn add_album(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: AddAlbumInput,
    ) -> Result<AlbumOperationResult> {
        let user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let library_id = uuid::Uuid::parse_str(&input.library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = uuid::Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match metadata
            .add_album_from_provider(AddAlbumOptions {
                provider: MetadataProvider::Musicbrainz,
                provider_id: input.musicbrainz_id,
                library_id,
                user_id,
            })
            .await
        {
            Ok(album) => Ok(AlbumOperationResult {
                success: true,
                album: Some(album),
                error: None,
            }),
            Err(e) => Ok(AlbumOperationResult {
                success: false,
                album: None,
                error: Some(e.to_string()),
            }),
        }
    }
}
