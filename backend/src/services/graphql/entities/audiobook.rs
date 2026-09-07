use crate::graphql::entities::*;
use async_graphql::{Context, InputObject, Object, Result};
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::chapter::Chapter;
use super::common::AutoDownloadMode;
use super::library::Library;
use crate::graphql::auth::AuthExt;
use crate::services::metadata::providers::{
    AddAudiobookOptions, MetadataProvider, MetadataService,
};

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
    table = "audiobooks",
    plural = "Audiobooks",
    default_sort = "title",
    index = "library_id"
)]
pub struct Audiobook {
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

    #[graphql(name = "title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: String,

    #[graphql(name = "sortTitle")]
    #[sortable]
    pub sort_title: Option<String>,

    #[graphql(name = "authorName")]
    #[filterable(type = "string")]
    #[sortable]
    pub author_name: Option<String>,

    #[graphql(name = "narratorName")]
    #[filterable(type = "string")]
    pub narrator_name: Option<String>,

    #[graphql(name = "narrators")]
    #[json_field]
    pub narrators: Vec<String>,

    #[graphql(name = "description")]
    pub description: Option<String>,

    #[graphql(name = "publisher")]
    #[filterable(type = "string")]
    pub publisher: Option<String>,

    #[graphql(name = "publishedDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub published_date: Option<String>,

    #[graphql(name = "language")]
    #[filterable(type = "string")]
    pub language: Option<String>,

    #[graphql(name = "isbn")]
    #[filterable(type = "string")]
    pub isbn: Option<String>,

    #[graphql(name = "asin")]
    #[filterable(type = "string")]
    pub asin: Option<String>,

    #[graphql(name = "audibleId")]
    #[filterable(type = "string")]
    pub audible_id: Option<String>,

    #[graphql(name = "goodreadsId")]
    #[filterable(type = "string")]
    pub goodreads_id: Option<String>,

    #[graphql(name = "totalDurationSecs")]
    #[filterable(type = "number")]
    #[sortable]
    pub total_duration_secs: Option<i32>,

    #[graphql(name = "chapterCount")]
    #[filterable(type = "number")]
    pub chapter_count: Option<i32>,

    #[graphql(name = "coverUrl")]
    pub cover_url: Option<String>,

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

    #[graphql(skip)]
    #[serde(skip)]
    #[relation(target = "Chapter", from = "id", to = "audiobook_id", multiple)]
    pub chapters: Vec<Chapter>,
}

#[derive(Default)]
pub struct AudiobookCustomOperations;

/// Search result for OpenLibrary audiobook search.
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(name = "AudiobookSearchResult")]
pub struct AudiobookSearchResultGql {
    #[graphql(name = "provider")]
    pub provider: String,
    #[graphql(name = "providerId")]
    pub provider_id: String,
    #[graphql(name = "title")]
    pub title: String,
    #[graphql(name = "authorName")]
    pub author_name: Option<String>,
    #[graphql(name = "year")]
    pub year: Option<i32>,
    #[graphql(name = "coverUrl")]
    pub cover_url: Option<String>,
    #[graphql(name = "isbn")]
    pub isbn: Option<String>,
    #[graphql(name = "description")]
    pub description: Option<String>,
}

#[Object]
impl AudiobookCustomOperations {
    /// Search audiobooks on OpenLibrary.
    #[graphql(name = "searchAudiobooks")]
    async fn search_audiobooks(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "query")] query: String,
    ) -> Result<Vec<AudiobookSearchResultGql>> {
        let _user = ctx.librarian_auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let results = metadata
            .search_audiobooks(&query)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|a| AudiobookSearchResultGql {
                provider: match a.provider {
                    MetadataProvider::OpenLibrary => "openlibrary".to_string(),
                    MetadataProvider::Musicbrainz => "musicbrainz".to_string(),
                    MetadataProvider::Tmdb => "tmdb".to_string(),
                    MetadataProvider::Tvmaze => "tvmaze".to_string(),
                },
                provider_id: a.provider_id,
                title: a.title,
                author_name: a.author_name,
                year: a.year,
                cover_url: a.cover_url,
                isbn: a.isbn,
                description: a.description,
            })
            .collect())
    }
}

#[derive(Debug, InputObject)]
#[graphql(name = "AddAudiobookInput")]
pub struct AddAudiobookInput {
    #[graphql(name = "libraryId")]
    pub library_id: String,
    #[graphql(name = "openlibraryId")]
    pub openlibrary_id: String,
    /// Enable auto-download for the audiobook. Ignored when
    /// `autoDownloadMode` is given explicitly. Defaults to false (`NONE`).
    #[graphql(name = "autoDownload")]
    pub auto_download: Option<bool>,
    /// Auto-download mode for the audiobook. Defaults to `NONE`.
    #[graphql(name = "autoDownloadMode")]
    pub auto_download_mode: Option<AutoDownloadMode>,
}

#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(name = "AudiobookOperationResult")]
pub struct AudiobookOperationResult {
    #[graphql(name = "success")]
    pub success: bool,
    #[graphql(name = "audiobook")]
    pub audiobook: Option<Audiobook>,
    #[graphql(name = "error")]
    pub error: Option<String>,
}

#[derive(Default)]
pub struct AudiobookMetadataMutations;

#[Object]
impl AudiobookMetadataMutations {
    /// Add an audiobook to a library by fetching metadata from OpenLibrary.
    #[graphql(name = "addAudiobook")]
    async fn add_audiobook(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: AddAudiobookInput,
    ) -> Result<AudiobookOperationResult> {
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
            .add_audiobook_from_provider(AddAudiobookOptions {
                provider: MetadataProvider::OpenLibrary,
                provider_id: input.openlibrary_id,
                library_id,
                user_id,
                monitor_type,
            })
            .await
        {
            Ok(audiobook) => Ok(AudiobookOperationResult {
                success: true,
                audiobook: Some(audiobook),
                error: None,
            }),
            Err(e) => Ok(AudiobookOperationResult {
                success: false,
                audiobook: None,
                error: Some(e.to_string()),
            }),
        }
    }
}
