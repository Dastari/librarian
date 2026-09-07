//! Source Entity and Custom Operations
//!
//! The Source entity represents a configured content acquisition source
//! (torrent indexer, usenet indexer, RSS feed). Credentials are stored
//! as an encrypted JSON blob and are never exposed through GraphQL.
//!
//! Standard CRUD (CreateSource, UpdateSource, DeleteSource) is auto-generated
//! by the #[graphql_entity] macro. The database service installs an ORM write
//! transform that encrypts credentials before generated CreateSource/UpdateSource
//! writes persist them.

use std::sync::Arc;

use crate::graphql::entities::*;
use async_graphql::{Context, InputObject, Object, Result};
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

use super::super::auth::AuthExt;
use crate::db::Database;
use crate::services::ServicesManager;
use crate::services::quality;
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
#[graphql_entity(
    table = "sources",
    plural = "Sources",
    default_sort = "priority",
    read_policy = "admin.read",
    write_policy = "admin.write"
)]
pub struct Source {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "name")]
    #[filterable(type = "string")]
    #[sortable]
    pub name: String,

    #[graphql(name = "sourceType")]
    #[filterable(type = "string")]
    #[sortable]
    pub source_type: String,

    #[graphql(name = "definitionId")]
    #[filterable(type = "string")]
    pub definition_id: String,

    #[graphql(name = "enabled")]
    #[filterable(type = "boolean")]
    pub enabled: bool,

    #[graphql(name = "priority")]
    #[filterable(type = "number")]
    #[sortable]
    pub priority: i32,

    #[graphql(name = "mediaTypes")]
    #[filterable(type = "string")]
    pub media_types: String,

    #[graphql(name = "siteUrl")]
    pub site_url: Option<String>,

    #[graphql(name = "supportsSearch")]
    #[filterable(type = "boolean")]
    pub supports_search: bool,

    #[graphql(name = "supportsTvSearch")]
    #[filterable(type = "boolean")]
    pub supports_tv_search: bool,

    #[graphql(name = "supportsMovieSearch")]
    #[filterable(type = "boolean")]
    pub supports_movie_search: bool,

    #[graphql(name = "supportsMusicSearch")]
    #[filterable(type = "boolean")]
    pub supports_music_search: bool,

    #[graphql(name = "supportsBookSearch")]
    #[filterable(type = "boolean")]
    pub supports_book_search: bool,

    // Credentials stored as encrypted "nonce:ciphertext" (base64).
    // #[graphql(skip)] hides from query output, #[input_only] keeps in Create/Update inputs.
    #[graphql(skip)]
    #[input_only]
    pub credentials: String,

    #[graphql(name = "settings")]
    pub settings: Option<String>,

    #[graphql(name = "lastError")]
    pub last_error: Option<String>,

    #[graphql(name = "errorCount")]
    #[filterable(type = "number")]
    pub error_count: i32,

    #[graphql(name = "lastSuccessAt")]
    #[filterable(type = "date")]
    pub last_success_at: Option<String>,

    #[graphql(name = "lastErrorAt")]
    #[filterable(type = "date")]
    pub last_error_at: Option<String>,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

/// Entity-level custom operations (empty - we use extra_query/mutation types)
#[derive(Default)]
pub struct SourceCustomOperations;

// =============================================================================
// GraphQL types for source operations.
// =============================================================================

/// Information about an available source definition (e.g., IPTorrents)
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceDefinitionInfo")]
pub struct SourceDefinitionInfoGql {
    #[graphql(name = "id")]
    pub id: String,
    #[graphql(name = "name")]
    pub name: String,
    #[graphql(name = "description")]
    pub description: String,
    #[graphql(name = "sourceType")]
    pub source_type: String,
    #[graphql(name = "trackerType")]
    pub tracker_type: String,
    #[graphql(name = "language")]
    pub language: String,
    #[graphql(name = "siteLink")]
    pub site_link: String,
    #[graphql(name = "requiredCredentials")]
    pub required_credentials: Vec<String>,
}

/// Definition of a configurable setting for a source
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceSettingDefinition")]
pub struct SourceSettingDefinitionGql {
    #[graphql(name = "key")]
    pub key: String,
    #[graphql(name = "label")]
    pub label: String,
    #[graphql(name = "settingType")]
    pub setting_type: String,
    #[graphql(name = "defaultValue")]
    pub default_value: Option<String>,
    #[graphql(name = "options")]
    pub options: Option<Vec<SourceSettingOptionGql>>,
}

/// Option for a select-type setting
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceSettingOption")]
pub struct SourceSettingOptionGql {
    #[graphql(name = "value")]
    pub value: String,
    #[graphql(name = "label")]
    pub label: String,
}

/// A single release from a source search
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceReleaseInfo")]
pub struct SourceReleaseInfoGql {
    #[graphql(name = "title")]
    pub title: String,
    #[graphql(name = "guid")]
    pub guid: String,
    #[graphql(name = "link")]
    pub link: Option<String>,
    #[graphql(name = "magnetUri")]
    pub magnet_uri: Option<String>,
    #[graphql(name = "infoHash")]
    pub info_hash: Option<String>,
    #[graphql(name = "details")]
    pub details: Option<String>,
    #[graphql(name = "publishDate")]
    pub publish_date: String,
    #[graphql(name = "categories")]
    pub categories: Vec<i32>,
    #[graphql(name = "size")]
    pub size: Option<i64>,
    #[graphql(name = "sizeFormatted")]
    pub size_formatted: Option<String>,
    #[graphql(name = "seeders")]
    pub seeders: Option<i32>,
    #[graphql(name = "leechers")]
    pub leechers: Option<i32>,
    #[graphql(name = "peers")]
    pub peers: Option<i32>,
    #[graphql(name = "grabs")]
    pub grabs: Option<i32>,
    #[graphql(name = "isFreeleech")]
    pub is_freeleech: bool,
    /// Private-tracker minimum share ratio declared for this release.
    #[graphql(name = "minimumRatio")]
    pub minimum_ratio: Option<f64>,
    /// Private-tracker minimum seed time in **seconds**, as Torznab reports
    /// it. Pass it straight back to `addTorrent`, which converts to minutes.
    #[graphql(name = "minimumSeedTime")]
    pub minimum_seed_time: Option<i64>,
    #[graphql(name = "imdbId")]
    pub imdb_id: Option<String>,
    #[graphql(name = "poster")]
    pub poster: Option<String>,
    #[graphql(name = "description")]
    pub description: Option<String>,
    #[graphql(name = "sourceId")]
    pub source_id: Option<String>,
    #[graphql(name = "sourceName")]
    pub source_name: Option<String>,
    /// Quality tags parsed out of the release title.
    #[graphql(name = "parsed")]
    pub parsed: ParsedReleaseGql,
    /// `"optimal"`, `"suboptimal"` or `"rejected"` when the search resolved a
    /// quality profile (via `qualityProfileId` or a target id); null otherwise.
    #[graphql(name = "profileMatch")]
    pub profile_match: Option<String>,
    /// Why the release is not `optimal` for the resolved profile. Empty when
    /// no profile was resolved or the release is optimal.
    #[graphql(name = "rejectReasons")]
    pub reject_reasons: Vec<String>,
}

/// Quality tags parsed out of a release title (see
/// `services::quality::scoring::ParsedRelease`).
#[derive(Clone, Debug, Default, async_graphql::SimpleObject)]
#[graphql(name = "ParsedRelease")]
pub struct ParsedReleaseGql {
    #[graphql(name = "resolution")]
    pub resolution: Option<String>,
    #[graphql(name = "codec")]
    pub codec: Option<String>,
    #[graphql(name = "hdrType")]
    pub hdr_type: Option<String>,
    #[graphql(name = "sourceType")]
    pub source_type: Option<String>,
    #[graphql(name = "audio")]
    pub audio: Option<String>,
    #[graphql(name = "releaseGroup")]
    pub release_group: Option<String>,
    /// ISO 639-1 codes declared by the title; empty means untagged.
    #[graphql(name = "languages")]
    pub languages: Vec<String>,
    #[graphql(name = "isSeasonPack")]
    pub is_season_pack: bool,
    #[graphql(name = "isProper")]
    pub is_proper: bool,
    #[graphql(name = "isRepack")]
    pub is_repack: bool,
    #[graphql(name = "season")]
    pub season: Option<i32>,
    #[graphql(name = "episodes")]
    pub episodes: Vec<i32>,
    #[graphql(name = "year")]
    pub year: Option<i32>,
}

impl From<&crate::services::quality::scoring::ParsedRelease> for ParsedReleaseGql {
    fn from(parsed: &crate::services::quality::scoring::ParsedRelease) -> Self {
        Self {
            resolution: parsed.resolution.clone(),
            codec: parsed.codec.clone(),
            hdr_type: parsed.hdr_type.clone(),
            source_type: parsed.source_type.clone(),
            audio: parsed.audio.clone(),
            release_group: parsed.release_group.clone(),
            languages: parsed.languages.clone(),
            is_season_pack: parsed.is_season_pack,
            is_proper: parsed.is_proper,
            is_repack: parsed.is_repack,
            season: parsed.season,
            episodes: parsed.episodes.clone(),
            year: parsed.year,
        }
    }
}

/// Results from a single source
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceSearchResultItem")]
pub struct SourceSearchResultItemGql {
    #[graphql(name = "sourceId")]
    pub source_id: String,
    #[graphql(name = "sourceName")]
    pub source_name: String,
    #[graphql(name = "releases")]
    pub releases: Vec<SourceReleaseInfoGql>,
    #[graphql(name = "elapsedMs")]
    pub elapsed_ms: i64,
    #[graphql(name = "fromCache")]
    pub from_cache: bool,
    #[graphql(name = "error")]
    pub error: Option<String>,
}

/// Aggregated search results from all sources
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceSearchResultSet")]
pub struct SourceSearchResultSetGql {
    #[graphql(name = "sources")]
    pub sources: Vec<SourceSearchResultItemGql>,
    #[graphql(name = "totalReleases")]
    pub total_releases: i32,
    #[graphql(name = "totalElapsedMs")]
    pub total_elapsed_ms: i64,
    #[graphql(name = "sourcesSearched")]
    pub sources_searched: i32,
}

/// Generic success/error result for source mutations
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceMutationResult")]
pub struct SourceMutationResultGql {
    #[graphql(name = "success")]
    pub success: bool,
    #[graphql(name = "error")]
    pub error: Option<String>,
}

/// Result of testing a source connection
#[derive(Clone, Debug, async_graphql::SimpleObject)]
#[graphql(name = "SourceTestConnectionResult")]
pub struct SourceTestConnectionResultGql {
    #[graphql(name = "success")]
    pub success: bool,
    #[graphql(name = "error")]
    pub error: Option<String>,
    #[graphql(name = "releasesFound")]
    pub releases_found: Option<i32>,
    #[graphql(name = "elapsedMs")]
    pub elapsed_ms: Option<i64>,
}

// =============================================================================
// Input types for source operations.
// =============================================================================

/// Input for searching sources.
///
/// The id/metadata fields are passed straight through to the Torznab query so
/// indexers that support id-based search return exact matches instead of
/// fuzzy text hits. The `*Id` target fields (and `qualityProfileId`) are only
/// used to resolve a `QualityProfile` for the returned releases' `parsed`,
/// `profileMatch` and `rejectReasons` fields; they never filter the search.
#[derive(InputObject, Clone, Debug)]
#[graphql(name = "SearchSourcesInput")]
pub struct SearchSourcesInput {
    #[graphql(name = "query")]
    pub query: String,
    #[graphql(name = "sourceIds")]
    pub source_ids: Option<Vec<String>>,
    #[graphql(name = "categories")]
    pub categories: Option<Vec<i32>>,
    #[graphql(name = "season")]
    pub season: Option<i32>,
    #[graphql(name = "episode")]
    pub episode: Option<String>,
    #[graphql(name = "imdbId")]
    pub imdb_id: Option<String>,
    #[graphql(name = "tvdbId")]
    pub tvdb_id: Option<i32>,
    #[graphql(name = "tmdbId")]
    pub tmdb_id: Option<i32>,
    #[graphql(name = "tvmazeId")]
    pub tvmaze_id: Option<i32>,
    #[graphql(name = "year")]
    pub year: Option<i32>,
    #[graphql(name = "artist")]
    pub artist: Option<String>,
    #[graphql(name = "album")]
    pub album: Option<String>,
    #[graphql(name = "author")]
    pub author: Option<String>,
    #[graphql(name = "title")]
    pub title: Option<String>,
    /// Evaluate results against this quality profile. Takes precedence over
    /// the profile resolved from a target below.
    #[graphql(name = "qualityProfileId")]
    pub quality_profile_id: Option<String>,
    #[graphql(name = "showId")]
    pub show_id: Option<String>,
    #[graphql(name = "movieId")]
    pub movie_id: Option<String>,
    #[graphql(name = "albumId")]
    pub album_id: Option<String>,
    #[graphql(name = "audiobookId")]
    pub audiobook_id: Option<String>,
    #[graphql(name = "limit")]
    pub limit: Option<i32>,
}

/// Input for updating source priorities
#[derive(InputObject, Clone, Debug)]
#[graphql(name = "UpdateSourcePrioritiesInput")]
pub struct UpdateSourcePrioritiesInput {
    /// Source IDs in the desired priority order (first = highest priority)
    #[graphql(name = "sourceIds")]
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
    #[graphql(name = "availableSourceDefinitions")]
    async fn available_source_definitions(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Vec<SourceDefinitionInfoGql>> {
        ctx.require_admin()?;

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
    #[graphql(name = "sourceSettingDefinitions")]
    async fn source_setting_definitions(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "definitionId")] definition_id: String,
    ) -> Result<Vec<SourceSettingDefinitionGql>> {
        ctx.require_admin()?;

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
    #[graphql(name = "searchSources")]
    async fn search_sources(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: SearchSourcesInput,
    ) -> Result<SourceSearchResultSetGql> {
        ctx.require_member()?;
        let manager = ctx.data::<Arc<ServicesManager>>()?;
        let db = ctx.data::<Database>()?;

        // Optional: the profile to grade every returned release against.
        let quality_profile = resolve_search_profile(db, &input).await;

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
            query_type: if input.season.is_some()
                || input.episode.is_some()
                || input.tvdb_id.is_some()
                || input.tvmaze_id.is_some()
            {
                QueryType::TvSearch
            } else if input.artist.is_some() || input.album.is_some() {
                QueryType::MusicSearch
            } else if input.author.is_some() {
                QueryType::BookSearch
            } else if input.imdb_id.is_some() || input.tmdb_id.is_some() {
                QueryType::MovieSearch
            } else {
                QueryType::Search
            },
            search_term: Some(input.query.clone()),
            categories: input.categories.clone().unwrap_or_default(),
            season: input.season,
            episode: input.episode.clone(),
            imdb_id: input.imdb_id.clone(),
            tvdb_id: input.tvdb_id,
            tmdb_id: input.tmdb_id,
            tvmaze_id: input.tvmaze_id,
            year: input.year,
            artist: input.artist.clone(),
            album: input.album.clone(),
            author: input.author.clone(),
            title: input.title.clone(),
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
                        let parsed = quality::scoring::parse_release(&rel.title);
                        let (profile_match, reject_reasons) = match quality_profile.as_ref() {
                            Some(profile) => {
                                // Size limits are per episode; for a season
                                // pack the episode count is unknown here, so
                                // the size check is skipped rather than
                                // wrongly rejecting a large (correct) pack.
                                let facts = quality::profile::ReleaseFacts {
                                    size_bytes: if parsed.is_season_pack {
                                        None
                                    } else {
                                        rel.size
                                    },
                                    seeders: rel.seeders,
                                    publish_date: Some(rel.publish_date),
                                    episode_count: 1,
                                    now: chrono::Utc::now(),
                                };
                                let (class, reasons) =
                                    quality::profile::match_release(&parsed, profile, &facts);
                                (Some(class.as_str().to_string()), reasons)
                            }
                            None => (None, Vec::new()),
                        };
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
                            minimum_ratio: rel.minimum_ratio,
                            minimum_seed_time: rel.minimum_seed_time,
                            imdb_id: rel.imdb.map(|id| format!("tt{:07}", id)),
                            poster: rel.poster.clone(),
                            description: rel.description.clone(),
                            source_id: rel.source_id.clone(),
                            source_name: rel.source_name.clone(),
                            parsed: ParsedReleaseGql::from(&parsed),
                            profile_match,
                            reject_reasons,
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

/// Resolve the quality profile `searchSources` should grade releases against:
/// an explicit `qualityProfileId` wins, otherwise the effective profile of the
/// named target (per-entity override > library > seeded default). Returns
/// `None` when the caller named neither, in which case releases come back
/// without `profileMatch`.
async fn resolve_search_profile(
    db: &Database,
    input: &SearchSourcesInput,
) -> Option<QualityProfile> {
    if let Some(id) = input.quality_profile_id.as_deref()
        && let Ok(Some(profile)) = QualityProfile::get(db.pool(), &id.to_string()).await
    {
        return Some(profile);
    }

    let resolved = if let Some(id) = input.show_id.as_deref() {
        match Show::get(db.pool(), &id.to_string()).await {
            Ok(Some(show)) => Some((show.quality_profile_id, show.library_id)),
            _ => None,
        }
    } else if let Some(id) = input.movie_id.as_deref() {
        match Movie::get(db.pool(), &id.to_string()).await {
            Ok(Some(movie)) => Some((movie.quality_profile_id, movie.library_id)),
            _ => None,
        }
    } else if let Some(id) = input.album_id.as_deref() {
        match Album::get(db.pool(), &id.to_string()).await {
            Ok(Some(album)) => Some((album.quality_profile_id, album.library_id)),
            _ => None,
        }
    } else if let Some(id) = input.audiobook_id.as_deref() {
        match Audiobook::get(db.pool(), &id.to_string()).await {
            Ok(Some(audiobook)) => Some((audiobook.quality_profile_id, audiobook.library_id)),
            _ => None,
        }
    } else {
        None
    };

    let (override_id, library_id) = resolved?;
    match quality::profile::resolve_profile(db, override_id.as_deref(), &library_id).await {
        Ok(profile) => Some(profile),
        Err(error) => {
            tracing::warn!(
                library_id = %library_id,
                error = %error,
                "searchSources: failed to resolve the quality profile for the target"
            );
            None
        }
    }
}

// =============================================================================
// Custom Mutation Operations
// =============================================================================

/// Custom mutation operations for sources.
/// Standard CRUD (CreateSource, UpdateSource, DeleteSource) is handled by the macro.
/// Credentials are encrypted by the database write transform in CreateSource/UpdateSource.
/// These mutations handle source-specific operations like testing and priority reordering.
#[derive(Default)]
pub struct SourceCustomMutations;

#[Object]
impl SourceCustomMutations {
    /// Test a source connection
    #[graphql(name = "testSource")]
    async fn test_source(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "id")] id: String,
    ) -> Result<SourceTestConnectionResultGql> {
        ctx.require_admin()?;
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

        let test_result = match sources_manager.test_source(&id).await {
            Err(error) if error.to_string().contains("Source not loaded") && source.enabled => {
                tracing::info!(
                    source_id = %id,
                    source_name = %source.name,
                    "Source was not loaded before test; reloading sources from database"
                );
                sources_svc.reload().await.map_err(|reload_error| {
                    async_graphql::Error::new(format!(
                        "Failed to reload sources before test: {}",
                        reload_error
                    ))
                })?;
                sources_manager.test_source(&id).await
            }
            result => result,
        };

        match test_result {
            Ok(true) => {
                // Test search to verify full functionality
                use crate::services::sources::SourceQuery;
                let query = SourceQuery::search("");

                match sources_manager
                    .search_sources(std::slice::from_ref(&id), &query)
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
    #[graphql(name = "updateSourcePriorities")]
    async fn update_source_priorities(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "input")] input: UpdateSourcePrioritiesInput,
    ) -> Result<SourceMutationResultGql> {
        ctx.require_admin()?;
        let db = ctx.data::<Database>()?;

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
