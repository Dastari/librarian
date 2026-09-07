use crate::graphql::entities::*;
use async_graphql::{Context, InputObject, Object, Result};
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::common::AutoDownloadMode;
use super::track::Track;
use crate::{
    db::Database,
    graphql::auth::AuthExt,
    graphql::entities::{Library, TrackOrderByInput, TrackWhereInput},
    services::metadata::providers::{AddAlbumOptions, MetadataProvider, MetadataService},
};

/// Album Entity
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
    table = "albums",
    plural = "Albums",
    default_sort = "name",
    index = "library_id"
)]
pub struct Album {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "artistId")]
    #[filterable(type = "string")]
    pub artist_id: String,

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

    #[graphql(name = "year")]
    #[filterable(type = "number")]
    #[sortable]
    pub year: Option<i32>,

    #[graphql(name = "musicbrainzId")]
    #[filterable(type = "string")]
    pub musicbrainz_id: Option<String>,

    #[graphql(name = "albumType")]
    #[filterable(type = "string")]
    pub album_type: Option<String>,

    #[graphql(name = "genres")]
    #[json_field]
    pub genres: Vec<String>,

    #[graphql(name = "label")]
    #[filterable(type = "string")]
    pub label: Option<String>,

    #[graphql(name = "country")]
    #[filterable(type = "string")]
    pub country: Option<String>,

    #[graphql(name = "releaseDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub release_date: Option<String>,

    #[graphql(name = "coverUrl")]
    pub cover_url: Option<String>,

    #[graphql(name = "trackCount")]
    #[filterable(type = "number")]
    pub track_count: Option<i32>,

    #[graphql(name = "discCount")]
    #[filterable(type = "number")]
    pub disc_count: Option<i32>,

    #[graphql(name = "totalDurationSecs")]
    #[filterable(type = "number")]
    pub total_duration_secs: Option<i32>,

    #[graphql(name = "autoDownload")]
    #[filterable(type = "boolean")]
    pub auto_download: bool,

    #[graphql(name = "autoDownloadMode")]
    pub auto_download_mode: AutoDownloadMode,

    /// Optional quality profile override; falls back to
    /// `Library.qualityProfileId` (then the seeded default) when unset.
    #[graphql(name = "qualityProfileId")]
    #[filterable(type = "string")]
    pub quality_profile_id: Option<String>,

    #[graphql(name = "hasFiles")]
    #[filterable(type = "boolean")]
    pub has_files: bool,

    #[graphql(name = "sizeBytes")]
    #[filterable(type = "number")]
    #[sortable]
    pub size_bytes: Option<i64>,

    #[graphql(name = "path")]
    pub path: Option<String>,

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
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(name = "AlbumSearchResult")]
pub struct AlbumSearchResultGql {
    #[graphql(name = "provider")]
    pub provider: String,
    #[graphql(name = "providerId")]
    pub provider_id: String,
    #[graphql(name = "title")]
    pub title: String,
    #[graphql(name = "artistName")]
    pub artist_name: Option<String>,
    #[graphql(name = "year")]
    pub year: Option<i32>,
    #[graphql(name = "albumType")]
    pub album_type: Option<String>,
    #[graphql(name = "coverUrl")]
    pub cover_url: Option<String>,
    #[graphql(name = "score")]
    pub score: Option<f64>,
}

#[Object]
impl AlbumCustomOperations {
    /// Search albums on MusicBrainz.
    #[graphql(name = "searchAlbums")]
    #[allow(clippy::too_many_arguments)]
    async fn search_albums(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "query")] query: String,
        #[graphql(name = "includeEps", default = false)] include_eps: bool,
        #[graphql(name = "includeSingles", default = false)] include_singles: bool,
        #[graphql(name = "includeCompilations", default = false)] include_compilations: bool,
        #[graphql(name = "includeLive", default = false)] include_live: bool,
        #[graphql(name = "includeSoundtracks", default = false)] include_soundtracks: bool,
    ) -> Result<Vec<AlbumSearchResultGql>> {
        let _user = ctx.librarian_auth_user()?;
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
    #[graphql(name = "libraryId")]
    pub library_id: String,
    #[graphql(name = "musicbrainzId")]
    pub musicbrainz_id: String,
    /// Enable auto-download for the album. Ignored when `autoDownloadMode` is
    /// given explicitly. Defaults to false (mode `NONE`).
    #[graphql(name = "autoDownload")]
    pub auto_download: Option<bool>,
    /// Auto-download mode for the album's tracks. Defaults to `NONE`.
    #[graphql(name = "autoDownloadMode")]
    pub auto_download_mode: Option<AutoDownloadMode>,
}

#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(name = "AlbumOperationResult")]
pub struct AlbumOperationResult {
    #[graphql(name = "success")]
    pub success: bool,
    #[graphql(name = "album")]
    pub album: Option<Album>,
    #[graphql(name = "error")]
    pub error: Option<String>,
}

#[derive(Default)]
pub struct AlbumMetadataMutations;

#[Object]
impl AlbumMetadataMutations {
    /// Add an album to a library by fetching metadata from MusicBrainz.
    #[graphql(name = "addAlbum")]
    async fn add_album(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: AddAlbumInput,
    ) -> Result<AlbumOperationResult> {
        let user = ctx.librarian_auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let library_id = uuid::Uuid::parse_str(&input.library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = uuid::Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let monitor_type = input.auto_download_mode.unwrap_or({
            if input.auto_download.unwrap_or(false) {
                AutoDownloadMode::All
            } else {
                AutoDownloadMode::None
            }
        });

        match metadata
            .add_album_from_provider(AddAlbumOptions {
                provider: MetadataProvider::Musicbrainz,
                provider_id: input.musicbrainz_id,
                library_id,
                user_id,
                monitor_type,
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
