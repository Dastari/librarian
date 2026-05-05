use crate::graphql::entities::*;
use async_graphql::{Context, InputObject, Object, Result};
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::chapter::Chapter;
use super::common::AutoDownloadMode;
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
#[graphql_entity(table = "audiobooks", plural = "Audiobooks", default_sort = "title")]
pub struct Audiobook {
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

    #[graphql(name = "Title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: String,

    #[graphql(name = "SortTitle")]
    #[sortable]
    pub sort_title: Option<String>,

    #[graphql(name = "AuthorName")]
    #[filterable(type = "string")]
    #[sortable]
    pub author_name: Option<String>,

    #[graphql(name = "NarratorName")]
    #[filterable(type = "string")]
    pub narrator_name: Option<String>,

    #[graphql(name = "Narrators")]
    #[json_field]
    pub narrators: Vec<String>,

    #[graphql(name = "Description")]
    pub description: Option<String>,

    #[graphql(name = "Publisher")]
    #[filterable(type = "string")]
    pub publisher: Option<String>,

    #[graphql(name = "PublishedDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub published_date: Option<String>,

    #[graphql(name = "Language")]
    #[filterable(type = "string")]
    pub language: Option<String>,

    #[graphql(name = "Isbn")]
    #[filterable(type = "string")]
    pub isbn: Option<String>,

    #[graphql(name = "Asin")]
    #[filterable(type = "string")]
    pub asin: Option<String>,

    #[graphql(name = "AudibleId")]
    #[filterable(type = "string")]
    pub audible_id: Option<String>,

    #[graphql(name = "GoodreadsId")]
    #[filterable(type = "string")]
    pub goodreads_id: Option<String>,

    #[graphql(name = "TotalDurationSecs")]
    #[filterable(type = "number")]
    #[sortable]
    pub total_duration_secs: Option<i32>,

    #[graphql(name = "ChapterCount")]
    #[filterable(type = "number")]
    pub chapter_count: Option<i32>,

    #[graphql(name = "CoverUrl")]
    pub cover_url: Option<String>,

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
    #[graphql(name = "Provider")]
    pub provider: String,
    #[graphql(name = "ProviderId")]
    pub provider_id: String,
    #[graphql(name = "Title")]
    pub title: String,
    #[graphql(name = "AuthorName")]
    pub author_name: Option<String>,
    #[graphql(name = "Year")]
    pub year: Option<i32>,
    #[graphql(name = "CoverUrl")]
    pub cover_url: Option<String>,
    #[graphql(name = "Isbn")]
    pub isbn: Option<String>,
    #[graphql(name = "Description")]
    pub description: Option<String>,
}

#[Object]
impl AudiobookCustomOperations {
    /// Search audiobooks on OpenLibrary.
    #[graphql(name = "SearchAudiobooks")]
    async fn search_audiobooks(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Query")] query: String,
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
    #[graphql(name = "LibraryId")]
    pub library_id: String,
    #[graphql(name = "OpenlibraryId")]
    pub openlibrary_id: String,
}

#[derive(Debug, async_graphql::SimpleObject)]
#[graphql(name = "AudiobookOperationResult")]
pub struct AudiobookOperationResult {
    #[graphql(name = "Success")]
    pub success: bool,
    #[graphql(name = "Audiobook")]
    pub audiobook: Option<Audiobook>,
    #[graphql(name = "Error")]
    pub error: Option<String>,
}

#[derive(Default)]
pub struct AudiobookMetadataMutations;

#[Object]
impl AudiobookMetadataMutations {
    /// Add an audiobook to a library by fetching metadata from OpenLibrary.
    #[graphql(name = "AddAudiobook")]
    async fn add_audiobook(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: AddAudiobookInput,
    ) -> Result<AudiobookOperationResult> {
        let user = ctx.librarian_auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let library_id = uuid::Uuid::parse_str(&input.library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = uuid::Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match metadata
            .add_audiobook_from_provider(AddAudiobookOptions {
                provider: MetadataProvider::OpenLibrary,
                provider_id: input.openlibrary_id,
                library_id,
                user_id,
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
