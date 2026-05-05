//! Source Entity and Custom Operations
//!
//! The Source entity represents a configured content acquisition source
//! (torrent indexer, usenet indexer, RSS feed). Credentials are stored
//! as an encrypted JSON blob and are never exposed through GraphQL.
//!
//! Standard CRUD (CreateSource, UpdateSource, DeleteSource) is auto-generated
//! by the #[graphql_entity] macro. Custom operations handle encryption-sensitive
//! tasks and source-specific queries.

use std::sync::Arc;

use crate::graphql::entities::*;
use async_graphql::{Context, InputObject, Object, Result};
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use super::super::auth::AuthExt;
use crate::db::Database;
use crate::services::ServicesManager;
use crate::services::sources::definitions::{
    SettingType, get_available_definitions, get_definition_info,
};

// =============================================================================
// Source Entity (auto-generated CRUD via #[graphql_entity])
// =============================================================================

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "sources", plural = "Sources", default_sort = "priority")]
pub struct Source {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "Name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "SourceType")]
    #[filterable(type = "string")]
    #[sortable]
    pub source_type: String,

    #[graphql(name = "DefinitionId")]
    #[filterable(type = "string")]
    pub definition_id: String,

    #[graphql(name = "Enabled")]
    #[filterable(type = "boolean")]
    pub enabled: bool,

    #[graphql(name = "Priority")]
    #[filterable(type = "number")]
    #[sortable]
    pub priority: i32,

    #[graphql(name = "MediaTypes")]
    #[filterable(type = "string")]
    pub media_types: String,

    #[graphql(name = "SiteUrl")]
    pub site_url: Option<String>,

    #[graphql(name = "SupportsSearch")]
    #[filterable(type = "boolean")]
    pub supports_search: bool,

    #[graphql(name = "SupportsTvSearch")]
    #[filterable(type = "boolean")]
    pub supports_tv_search: bool,

    #[graphql(name = "SupportsMovieSearch")]
    #[filterable(type = "boolean")]
    pub supports_movie_search: bool,

    #[graphql(name = "SupportsMusicSearch")]
    #[filterable(type = "boolean")]
    pub supports_music_search: bool,

    #[graphql(name = "SupportsBookSearch")]
    #[filterable(type = "boolean")]
    pub supports_book_search: bool,

    // Credentials stored as encrypted "nonce:ciphertext" (base64).
    // #[graphql(skip)] hides from query output, #[input_only] keeps in Create/Update inputs,
    // #[transform(write = "...")] encrypts plaintext JSON before writing to DB.
    #[graphql(skip)]
    #[input_only]
    pub credentials: String,

    #[graphql(name = "Settings")]
    pub settings: Option<String>,

    #[graphql(name = "LastError")]
    pub last_error: Option<String>,

    #[graphql(name = "ErrorCount")]
    #[filterable(type = "number")]
    pub error_count: i32,

    #[graphql(name = "LastSuccessAt")]
    #[filterable(type = "date")]
    pub last_success_at: Option<String>,

    #[graphql(name = "LastErrorAt")]
    #[filterable(type = "date")]
    pub last_error_at: Option<String>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

/// Entity-level custom operations (empty - we use extra_query/mutation types)
#[derive(Default)]
pub struct SourceCustomOperations;

// =============================================================================
// GraphQL Types for Source Operations (PascalCase)
// =============================================================================

/// Information about an available source definition (e.g., IPTorrents)
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceDefinitionInfo")]
pub struct SourceDefinitionInfoGql {
    #[graphql(name = "Id")]
    pub id: String,
    #[graphql(name = "Name")]
    pub name: String,
    #[graphql(name = "Description")]
    pub description: String,
    #[graphql(name = "SourceType")]
    pub source_type: String,
    #[graphql(name = "TrackerType")]
    pub tracker_type: String,
    #[graphql(name = "Language")]
    pub language: String,
    #[graphql(name = "SiteLink")]
    pub site_link: String,
    #[graphql(name = "RequiredCredentials")]
    pub required_credentials: Vec<String>,
}

/// Definition of a configurable setting for a source
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceSettingDefinition")]
pub struct SourceSettingDefinitionGql {
    #[graphql(name = "Key")]
    pub key: String,
    #[graphql(name = "Label")]
    pub label: String,
    #[graphql(name = "SettingType")]
    pub setting_type: String,
    #[graphql(name = "DefaultValue")]
    pub default_value: Option<String>,
    #[graphql(name = "Options")]
    pub options: Option<Vec<SourceSettingOptionGql>>,
}

/// Option for a select-type setting
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceSettingOption")]
pub struct SourceSettingOptionGql {
    #[graphql(name = "Value")]
    pub value: String,
    #[graphql(name = "Label")]
    pub label: String,
}

/// A single release from a source search
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceReleaseInfo")]
pub struct SourceReleaseInfoGql {
    #[graphql(name = "Title")]
    pub title: String,
    #[graphql(name = "Guid")]
    pub guid: String,
    #[graphql(name = "Link")]
    pub link: Option<String>,
    #[graphql(name = "MagnetUri")]
    pub magnet_uri: Option<String>,
    #[graphql(name = "InfoHash")]
    pub info_hash: Option<String>,
    #[graphql(name = "Details")]
    pub details: Option<String>,
    #[graphql(name = "PublishDate")]
    pub publish_date: String,
    #[graphql(name = "Categories")]
    pub categories: Vec<i32>,
    #[graphql(name = "Size")]
    pub size: Option<i64>,
    #[graphql(name = "SizeFormatted")]
    pub size_formatted: Option<String>,
    #[graphql(name = "Seeders")]
    pub seeders: Option<i32>,
    #[graphql(name = "Leechers")]
    pub leechers: Option<i32>,
    #[graphql(name = "Peers")]
    pub peers: Option<i32>,
    #[graphql(name = "Grabs")]
    pub grabs: Option<i32>,
    #[graphql(name = "IsFreeleech")]
    pub is_freeleech: bool,
    #[graphql(name = "ImdbId")]
    pub imdb_id: Option<String>,
    #[graphql(name = "Poster")]
    pub poster: Option<String>,
    #[graphql(name = "Description")]
    pub description: Option<String>,
    #[graphql(name = "SourceId")]
    pub source_id: Option<String>,
    #[graphql(name = "SourceName")]
    pub source_name: Option<String>,
}

/// Results from a single source
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceSearchResultItem")]
pub struct SourceSearchResultItemGql {
    #[graphql(name = "SourceId")]
    pub source_id: String,
    #[graphql(name = "SourceName")]
    pub source_name: String,
    #[graphql(name = "Releases")]
    pub releases: Vec<SourceReleaseInfoGql>,
    #[graphql(name = "ElapsedMs")]
    pub elapsed_ms: i64,
    #[graphql(name = "FromCache")]
    pub from_cache: bool,
    #[graphql(name = "Error")]
    pub error: Option<String>,
}

/// Aggregated search results from all sources
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceSearchResultSet")]
pub struct SourceSearchResultSetGql {
    #[graphql(name = "Sources")]
    pub sources: Vec<SourceSearchResultItemGql>,
    #[graphql(name = "TotalReleases")]
    pub total_releases: i32,
    #[graphql(name = "TotalElapsedMs")]
    pub total_elapsed_ms: i64,
    #[graphql(name = "SourcesSearched")]
    pub sources_searched: i32,
}

/// Generic success/error result for source mutations
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceMutationResult")]
pub struct SourceMutationResultGql {
    #[graphql(name = "Success")]
    pub success: bool,
    #[graphql(name = "Error")]
    pub error: Option<String>,
}

/// Result of testing a source connection
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceTestConnectionResult")]
pub struct SourceTestConnectionResultGql {
    #[graphql(name = "Success")]
    pub success: bool,
    #[graphql(name = "Error")]
    pub error: Option<String>,
    #[graphql(name = "ReleasesFound")]
    pub releases_found: Option<i32>,
    #[graphql(name = "ElapsedMs")]
    pub elapsed_ms: Option<i64>,
}

// =============================================================================
// Input Types (PascalCase)
// =============================================================================

/// Input for searching sources
#[derive(InputObject, Clone, Debug)]
#[graphql(name = "SearchSourcesInput")]
pub struct SearchSourcesInput {
    #[graphql(name = "Query")]
    pub query: String,
    #[graphql(name = "SourceIds")]
    pub source_ids: Option<Vec<String>>,
    #[graphql(name = "Categories")]
    pub categories: Option<Vec<i32>>,
    #[graphql(name = "Season")]
    pub season: Option<i32>,
    #[graphql(name = "Episode")]
    pub episode: Option<String>,
    #[graphql(name = "ImdbId")]
    pub imdb_id: Option<String>,
    #[graphql(name = "Limit")]
    pub limit: Option<i32>,
}

/// Input for updating source priorities
#[derive(InputObject, Clone, Debug)]
#[graphql(name = "UpdateSourcePrioritiesInput")]
pub struct UpdateSourcePrioritiesInput {
    /// Source IDs in the desired priority order (first = highest priority)
    #[graphql(name = "SourceIds")]
    pub source_ids: Vec<String>,
}

// =============================================================================
// Custom Query Operations
// =============================================================================

/// Custom query operations for sources
#[derive(Default)]
pub struct SourceCustomQueries;

#[Object]
impl SourceCustomQueries {
    /// Get available source definitions (e.g., IPTorrents, Newznab, etc.)
    #[graphql(name = "AvailableSourceDefinitions")]
    async fn available_source_definitions(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<SourceDefinitionInfoGql>> {
        let _user = ctx.librarian_auth_user()?;

        let definitions = get_available_definitions()
            .iter()
            .map(|info| SourceDefinitionInfoGql {
                id: info.id.to_string(),
                name: info.name.to_string(),
                description: info.description.to_string(),
                source_type: info.source_type.to_string(),
                tracker_type: info.tracker_type.to_string(),
                language: info.language.to_string(),
                site_link: info.site_link.to_string(),
                required_credentials: info
                    .required_credentials
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            })
            .collect();

        Ok(definitions)
    }

    /// Get setting definitions for a source definition
    #[graphql(name = "SourceSettingDefinitions")]
    async fn source_setting_definitions(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "DefinitionId")] definition_id: String,
    ) -> Result<Vec<SourceSettingDefinitionGql>> {
        let _user = ctx.librarian_auth_user()?;

        let info = get_definition_info(&definition_id).ok_or_else(|| {
            async_graphql::Error::new(format!("Unknown source definition: {}", definition_id))
        })?;

        let settings = info
            .optional_settings
            .iter()
            .map(|s| SourceSettingDefinitionGql {
                key: s.key.to_string(),
                label: s.label.to_string(),
                setting_type: match s.setting_type {
                    SettingType::Text => "Text".to_string(),
                    SettingType::Password => "Password".to_string(),
                    SettingType::Checkbox => "Checkbox".to_string(),
                    SettingType::Select => "Select".to_string(),
                },
                default_value: s.default_value.map(|v| v.to_string()),
                options: s.options.map(|opts| {
                    opts.iter()
                        .map(|(value, label)| SourceSettingOptionGql {
                            value: value.to_string(),
                            label: label.to_string(),
                        })
                        .collect()
                }),
            })
            .collect();

        Ok(settings)
    }

    /// Search across all enabled sources
    #[graphql(name = "SearchSources")]
    async fn search_sources(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: SearchSourcesInput,
    ) -> Result<SourceSearchResultSetGql> {
        let _user = ctx.librarian_auth_user()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;

        let sources_svc = manager
            .get_sources()
            .await
            .ok_or_else(|| async_graphql::Error::new("Sources service not available"))?;

        let sources_manager = sources_svc
            .get_manager()
            .await
            .ok_or_else(|| async_graphql::Error::new("Sources manager not initialized"))?;

        use crate::services::sources::{QueryType, SourceQuery};

        let query = SourceQuery {
            query_type: if input.season.is_some() || input.episode.is_some() {
                QueryType::TvSearch
            } else if input.imdb_id.is_some() {
                QueryType::MovieSearch
            } else {
                QueryType::Search
            },
            search_term: Some(input.query.clone()),
            categories: input.categories.unwrap_or_default(),
            season: input.season,
            episode: input.episode,
            imdb_id: input.imdb_id,
            limit: input.limit,
            cache: true,
            ..Default::default()
        };

        let start = std::time::Instant::now();

        let results = if let Some(ref source_ids) = input.source_ids {
            sources_manager.search_sources(source_ids, &query).await
        } else {
            sources_manager.search_all(&query).await
        };

        let mut total_releases = 0;
        let gql_results: Vec<SourceSearchResultItemGql> = results
            .into_iter()
            .map(|r| {
                let releases: Vec<SourceReleaseInfoGql> = r
                    .releases
                    .iter()
                    .map(|rel| {
                        total_releases += 1;
                        SourceReleaseInfoGql {
                            title: rel.title.clone(),
                            guid: rel.guid.clone(),
                            link: rel.link.clone(),
                            magnet_uri: rel.magnet_uri.clone(),
                            info_hash: rel.info_hash.clone(),
                            details: rel.details.clone(),
                            publish_date: rel.publish_date.to_rfc3339(),
                            categories: rel.categories.clone(),
                            size: rel.size,
                            size_formatted: rel.size.map(format_bytes),
                            seeders: rel.seeders,
                            leechers: rel.leechers(),
                            peers: rel.peers,
                            grabs: rel.grabs,
                            is_freeleech: rel.is_freeleech(),
                            imdb_id: rel.imdb.map(|id| format!("tt{:07}", id)),
                            poster: rel.poster.clone(),
                            description: rel.description.clone(),
                            source_id: rel.source_id.clone(),
                            source_name: rel.source_name.clone(),
                        }
                    })
                    .collect();

                SourceSearchResultItemGql {
                    source_id: r.source_id,
                    source_name: r.source_name,
                    releases,
                    elapsed_ms: r.elapsed_ms as i64,
                    from_cache: r.from_cache,
                    error: r.error,
                }
            })
            .collect();

        let sources_searched = gql_results.len() as i32;

        Ok(SourceSearchResultSetGql {
            sources: gql_results,
            total_releases,
            total_elapsed_ms: start.elapsed().as_millis() as i64,
            sources_searched,
        })
    }
}

// =============================================================================
// Custom Mutation Operations
// =============================================================================

/// Custom mutation operations for sources.
/// Standard CRUD (CreateSource, UpdateSource, DeleteSource) is handled by the macro.
/// Credentials are encrypted via the #[transform(write = "...")] hook in CreateSource/UpdateSource.
/// These mutations handle source-specific operations like testing and priority reordering.
#[derive(Default)]
pub struct SourceCustomMutations;

#[Object]
impl SourceCustomMutations {
    /// Test a source connection
    #[graphql(name = "TestSource")]
    async fn test_source(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Id")] id: String,
    ) -> Result<SourceTestConnectionResultGql> {
        let _user = ctx.librarian_auth_user()?;
        let db = ctx.data::<Database>()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;

        let Some(source) = Source::get(db.pool(), &id).await? else {
            return Ok(SourceTestConnectionResultGql {
                success: false,
                error: Some("Source not found".to_string()),
                releases_found: None,
                elapsed_ms: None,
            });
        };

        let sources_svc = manager
            .get_sources()
            .await
            .ok_or_else(|| async_graphql::Error::new("Sources service not available"))?;

        let sources_manager = sources_svc
            .get_manager()
            .await
            .ok_or_else(|| async_graphql::Error::new("Sources manager not initialized"))?;

        let start = std::time::Instant::now();

        match sources_manager.test_source(&id).await {
            Ok(true) => {
                // Test search to verify full functionality
                use crate::services::sources::SourceQuery;
                let query = SourceQuery::search("");

                match sources_manager
                    .search_sources(&[id.clone()], &query)
                    .await
                    .first()
                {
                    Some(result) => {
                        let releases_found = result.releases.len() as i32;

                        let _ = update_source_status(
                            db,
                            &id,
                            Some(0),
                            Some(None),
                            Some(Some(now_iso_string())),
                            None,
                        )
                        .await;

                        Ok(SourceTestConnectionResultGql {
                            success: true,
                            error: None,
                            releases_found: Some(releases_found),
                            elapsed_ms: Some(start.elapsed().as_millis() as i64),
                        })
                    }
                    None => Ok(SourceTestConnectionResultGql {
                        success: true,
                        error: None,
                        releases_found: Some(0),
                        elapsed_ms: Some(start.elapsed().as_millis() as i64),
                    }),
                }
            }
            Ok(false) => {
                let error = "Connection test failed - check your credentials".to_string();

                let _ = update_source_status(
                    db,
                    &id,
                    Some(source.error_count + 1),
                    Some(Some(error.clone())),
                    None,
                    Some(Some(now_iso_string())),
                )
                .await;

                Ok(SourceTestConnectionResultGql {
                    success: false,
                    error: Some(error),
                    releases_found: None,
                    elapsed_ms: Some(start.elapsed().as_millis() as i64),
                })
            }
            Err(e) => {
                let error = e.to_string();

                let _ = update_source_status(
                    db,
                    &id,
                    Some(source.error_count + 1),
                    Some(Some(error.clone())),
                    None,
                    Some(Some(now_iso_string())),
                )
                .await;

                Ok(SourceTestConnectionResultGql {
                    success: false,
                    error: Some(format!("Connection error: {}", error)),
                    releases_found: None,
                    elapsed_ms: Some(start.elapsed().as_millis() as i64),
                })
            }
        }
    }

    /// Update source priorities (reorder)
    #[graphql(name = "UpdateSourcePriorities")]
    async fn update_source_priorities(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "Input")] input: UpdateSourcePrioritiesInput,
    ) -> Result<SourceMutationResultGql> {
        let _user = ctx.librarian_auth_user()?;
        let db = ctx.data::<Database>()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;

        for (index, source_id) in input.source_ids.iter().enumerate() {
            let priority = (index + 1) as i32;
            Source::update_by_id(
                db,
                source_id,
                UpdateSourceInput {
                    name: None,
                    source_type: None,
                    definition_id: None,
                    enabled: None,
                    priority: Some(priority),
                    media_types: None,
                    site_url: None,
                    supports_search: None,
                    supports_tv_search: None,
                    supports_movie_search: None,
                    supports_music_search: None,
                    supports_book_search: None,
                    credentials: None,
                    settings: None,
                    last_error: None,
                    error_count: None,
                    last_success_at: None,
                    last_error_at: None,
                },
            )
            .await?;
        }

        // Reload sources
        if let Some(sources_svc) = manager.get_sources().await {
            if let Err(e) = sources_svc.reload().await {
                tracing::warn!(
                    error = %e,
                    source_count = input.source_ids.len(),
                    "Failed to reload sources after priority update: source_count={}, error={}",
                    input.source_ids.len(),
                    e
                );
            }
        }

        Ok(SourceMutationResultGql {
            success: true,
            error: None,
        })
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn format_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;
    const TB: i64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn now_iso_string() -> String {
    chrono::Utc::now().to_rfc3339()
}

async fn update_source_status(
    db: &Database,
    id: &str,
    error_count: Option<i32>,
    last_error: Option<Option<String>>,
    last_success_at: Option<Option<String>>,
    last_error_at: Option<Option<String>>,
) -> Result<()> {
    Source::update_by_id(
        db,
        &id.to_string(),
        UpdateSourceInput {
            name: None,
            source_type: None,
            definition_id: None,
            enabled: None,
            priority: None,
            media_types: None,
            site_url: None,
            supports_search: None,
            supports_tv_search: None,
            supports_movie_search: None,
            supports_music_search: None,
            supports_book_search: None,
            credentials: None,
            settings: None,
            last_error,
            error_count,
            last_success_at,
            last_error_at,
        },
    )
    .await?;
    Ok(())
}
