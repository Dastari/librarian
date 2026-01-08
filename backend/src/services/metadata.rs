//! Unified metadata service
//!
//! Provides a unified interface for fetching TV show metadata from multiple providers.
//! Priority: TVMaze (free, no key) → TMDB (if configured) → TheTVDB (if configured)

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::artwork::{ArtworkService, ArtworkType};
use super::filename_parser::{parse_episode, ParsedEpisode};
use super::tvmaze::{TvMazeClient, TvMazeEpisode, TvMazeShow};
use crate::db::{CreateEpisode, CreateTvShow, Database, TvShowRecord};

/// Metadata provider enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetadataProvider {
    TvMaze,
    Tmdb,
    TvDb,
}

/// Unified show search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowSearchResult {
    pub provider: MetadataProvider,
    pub provider_id: u32,
    pub name: String,
    pub year: Option<i32>,
    pub status: Option<String>,
    pub network: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub tvdb_id: Option<u32>,
    pub imdb_id: Option<String>,
    pub score: f64,
}

/// Unified show details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowDetails {
    pub provider: MetadataProvider,
    pub provider_id: u32,
    pub name: String,
    pub year: Option<i32>,
    pub status: Option<String>,
    pub network: Option<String>,
    pub overview: Option<String>,
    pub genres: Vec<String>,
    pub runtime: Option<i32>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub tvdb_id: Option<u32>,
    pub tmdb_id: Option<u32>,
    pub imdb_id: Option<String>,
}

/// Unified episode details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeDetails {
    pub provider: MetadataProvider,
    pub provider_id: u32,
    pub season: i32,
    pub episode: i32,
    pub absolute_number: Option<i32>,
    pub title: Option<String>,
    pub overview: Option<String>,
    pub air_date: Option<String>,
    pub runtime: Option<i32>,
}

/// Result of parsing and identifying media
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseAndIdentifyResult {
    pub parsed: ParsedEpisode,
    pub matches: Vec<ShowSearchResult>,
}

/// Metadata service configuration
#[derive(Debug, Clone, Default)]
pub struct MetadataServiceConfig {
    pub tmdb_api_key: Option<String>,
    pub tvdb_api_key: Option<String>,
    pub openai_api_key: Option<String>,
}

/// Options for adding a TV show from a metadata provider
#[derive(Debug, Clone)]
pub struct AddTvShowOptions {
    /// Metadata provider to use
    pub provider: MetadataProvider,
    /// Provider-specific ID (e.g., TVMaze ID)
    pub provider_id: u32,
    /// Library to add the show to
    pub library_id: Uuid,
    /// User who owns the library
    pub user_id: Uuid,
    /// Whether to monitor for new episodes
    pub monitored: bool,
    /// Monitor type: "all", "future", or "none"
    pub monitor_type: String,
    /// Quality profile to use (optional)
    pub quality_profile_id: Option<Uuid>,
    /// Custom path within the library (optional)
    pub path: Option<String>,
}

/// Unified metadata service
pub struct MetadataService {
    tvmaze: TvMazeClient,
    #[allow(dead_code)]
    config: MetadataServiceConfig,
    db: Database,
    artwork_service: Option<Arc<ArtworkService>>,
}

impl MetadataService {
    pub fn new(db: Database, config: MetadataServiceConfig) -> Self {
        Self {
            tvmaze: TvMazeClient::new(),
            config,
            db,
            artwork_service: None,
        }
    }

    /// Create with default config (no API keys)
    pub fn new_default(db: Database) -> Self {
        Self::new(db, MetadataServiceConfig::default())
    }

    /// Create with artwork service for caching images to Supabase storage
    pub fn new_with_artwork(
        db: Database,
        config: MetadataServiceConfig,
        artwork_service: Arc<ArtworkService>,
    ) -> Self {
        Self {
            tvmaze: TvMazeClient::new(),
            config,
            db,
            artwork_service: Some(artwork_service),
        }
    }

    /// Search for TV shows across providers
    pub async fn search_shows(&self, query: &str) -> Result<Vec<ShowSearchResult>> {
        info!(query = %query, "Searching for shows");

        let mut results = Vec::new();

        // Try TVMaze first (free, no API key)
        match self.tvmaze.search_shows(query).await {
            Ok(tvmaze_results) => {
                for r in tvmaze_results {
                    results.push(ShowSearchResult {
                        provider: MetadataProvider::TvMaze,
                        provider_id: r.show.id,
                        name: r.show.name.clone(),
                        year: r.show.premiere_year(),
                        status: normalize_show_status(r.show.status.as_deref()),
                        network: r.show.network_name().map(String::from),
                        overview: r.show.clean_summary(),
                        poster_url: r.show.poster_url().map(String::from),
                        tvdb_id: r.show.tvdb_id(),
                        imdb_id: r.show.imdb_id().map(String::from),
                        score: r.score,
                    });
                }
            }
            Err(e) => {
                warn!(error = %e, "TVMaze search failed");
            }
        }

        // TODO: Add TMDB search if configured
        // TODO: Add TheTVDB search if configured

        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        debug!(count = results.len(), "Found shows");
        Ok(results)
    }

    /// Get show details from a provider
    pub async fn get_show(&self, provider: MetadataProvider, provider_id: u32) -> Result<ShowDetails> {
        info!(provider = ?provider, id = provider_id, "Fetching show details");

        match provider {
            MetadataProvider::TvMaze => {
                let show = self.tvmaze.get_show(provider_id).await?;
                Ok(self.tvmaze_show_to_details(&show))
            }
            MetadataProvider::Tmdb => {
                // TODO: Implement TMDB
                anyhow::bail!("TMDB not yet implemented")
            }
            MetadataProvider::TvDb => {
                // TODO: Implement TheTVDB
                anyhow::bail!("TheTVDB not yet implemented")
            }
        }
    }

    /// Get episodes for a show from a provider
    pub async fn get_episodes(&self, provider: MetadataProvider, provider_id: u32) -> Result<Vec<EpisodeDetails>> {
        info!(provider = ?provider, id = provider_id, "Fetching episodes");

        match provider {
            MetadataProvider::TvMaze => {
                let episodes = self.tvmaze.get_episodes(provider_id).await?;
                Ok(episodes.iter().map(|e| self.tvmaze_episode_to_details(e)).collect())
            }
            MetadataProvider::Tmdb => {
                anyhow::bail!("TMDB not yet implemented")
            }
            MetadataProvider::TvDb => {
                anyhow::bail!("TheTVDB not yet implemented")
            }
        }
    }

    /// Parse a filename and try to identify the show
    pub async fn parse_and_identify(&self, title: &str) -> Result<ParseAndIdentifyResult> {
        info!(title = %title, "Parsing and identifying media");

        // Parse the filename
        let parsed = parse_episode(title);

        // Search for matches if we have a show name
        let matches = if let Some(ref show_name) = parsed.show_name {
            // Build a search query
            let mut query = show_name.clone();
            if let Some(year) = parsed.year {
                query = format!("{} {}", query, year);
            }

            self.search_shows(&query).await.unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(ParseAndIdentifyResult { parsed, matches })
    }

    /// Add a TV show from a metadata provider to a library.
    /// 
    /// This is the single code path for creating TV shows, used by both:
    /// - The addTvShow GraphQL mutation (manual add)
    /// - The scanner service (automatic discovery)
    /// 
    /// It handles:
    /// 1. Fetching show details from the provider
    /// 2. Caching artwork to Supabase storage
    /// 3. Creating the TV show record with normalized status
    /// 4. Fetching and creating all episode records
    /// 5. Updating show statistics
    pub async fn add_tv_show_from_provider(
        &self,
        options: AddTvShowOptions,
    ) -> Result<TvShowRecord> {
        info!(
            provider = ?options.provider,
            provider_id = options.provider_id,
            library_id = %options.library_id,
            "Adding TV show from provider"
        );

        // Get show details from provider
        let show_details = self.get_show(options.provider, options.provider_id).await?;

        // Cache artwork to Supabase storage if artwork service is available
        let (cached_poster_url, cached_backdrop_url) = if let Some(ref artwork_service) = self.artwork_service {
            let entity_id = format!("{}_{}", options.provider_id, options.library_id);
            
            let poster_url = artwork_service
                .cache_image_optional(
                    show_details.poster_url.as_deref(),
                    ArtworkType::Poster,
                    "show",
                    &entity_id,
                )
                .await;
            
            let backdrop_url = artwork_service
                .cache_image_optional(
                    show_details.backdrop_url.as_deref(),
                    ArtworkType::Backdrop,
                    "show",
                    &entity_id,
                )
                .await;

            info!(
                poster_cached = poster_url.is_some(),
                backdrop_cached = backdrop_url.is_some(),
                "Artwork caching completed"
            );

            (poster_url, backdrop_url)
        } else {
            // No artwork service, use original URLs
            (show_details.poster_url.clone(), show_details.backdrop_url.clone())
        };

        // Create the TV show in the database
        let tv_shows_repo = self.db.tv_shows();
        let tv_show = tv_shows_repo
            .create(CreateTvShow {
                library_id: options.library_id,
                user_id: options.user_id,
                name: show_details.name.clone(),
                sort_name: None,
                year: show_details.year,
                status: show_details.status, // Already normalized by get_show
                tvmaze_id: if options.provider == MetadataProvider::TvMaze {
                    Some(options.provider_id as i32)
                } else {
                    None
                },
                tmdb_id: show_details.tmdb_id.map(|id| id as i32),
                tvdb_id: show_details.tvdb_id.map(|id| id as i32),
                imdb_id: show_details.imdb_id,
                overview: show_details.overview,
                network: show_details.network,
                runtime: show_details.runtime,
                genres: show_details.genres,
                poster_url: cached_poster_url,
                backdrop_url: cached_backdrop_url,
                monitored: options.monitored,
                monitor_type: options.monitor_type.clone(),
                quality_profile_id: options.quality_profile_id,
                path: options.path.clone(),
            })
            .await?;

        info!(
            show_id = %tv_show.id,
            show_name = %tv_show.name,
            poster_url = ?tv_show.poster_url,
            "Created TV show record with cached artwork"
        );

        // Fetch and create episodes
        match self.get_episodes(options.provider, options.provider_id).await {
            Ok(episodes) => {
                let episodes_repo = self.db.episodes();
                let mut created_count = 0;

                for ep in episodes {
                    match episodes_repo
                        .create(CreateEpisode {
                            tv_show_id: tv_show.id,
                            season: ep.season,
                            episode: ep.episode,
                            absolute_number: ep.absolute_number,
                            title: ep.title,
                            overview: ep.overview,
                            air_date: ep.air_date.and_then(|d| {
                                chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()
                            }),
                            runtime: ep.runtime,
                            tvmaze_id: if options.provider == MetadataProvider::TvMaze {
                                Some(ep.provider_id as i32)
                            } else {
                                None
                            },
                            tmdb_id: None,
                            tvdb_id: None,
                            status: None, // Will use database default
                        })
                        .await
                    {
                        Ok(_) => created_count += 1,
                        Err(e) => {
                            warn!(
                                show_id = %tv_show.id,
                                season = ep.season,
                                episode = ep.episode,
                                error = %e,
                                "Failed to create episode"
                            );
                        }
                    }
                }

                info!(
                    show_id = %tv_show.id,
                    episode_count = created_count,
                    "Created episode records"
                );
            }
            Err(e) => {
                warn!(
                    show_id = %tv_show.id,
                    error = %e,
                    "Failed to fetch episodes, show created without episodes"
                );
            }
        }

        // Update show statistics
        if let Err(e) = tv_shows_repo.update_stats(tv_show.id).await {
            warn!(show_id = %tv_show.id, error = %e, "Failed to update show stats");
        }

        Ok(tv_show)
    }

    /// Convert TVMaze show to unified ShowDetails
    fn tvmaze_show_to_details(&self, show: &TvMazeShow) -> ShowDetails {
        ShowDetails {
            provider: MetadataProvider::TvMaze,
            provider_id: show.id,
            name: show.name.clone(),
            year: show.premiere_year(),
            status: normalize_show_status(show.status.as_deref()),
            network: show.network_name().map(String::from),
            overview: show.clean_summary(),
            genres: show.genres.clone(),
            runtime: show.runtime.or(show.average_runtime).map(|r| r as i32),
            poster_url: show.poster_url_original().map(String::from),
            backdrop_url: None, // TVMaze doesn't have backdrops
            tvdb_id: show.tvdb_id(),
            tmdb_id: None,
            imdb_id: show.imdb_id().map(String::from),
        }
    }

    /// Convert TVMaze episode to unified EpisodeDetails
    fn tvmaze_episode_to_details(&self, episode: &TvMazeEpisode) -> EpisodeDetails {
        EpisodeDetails {
            provider: MetadataProvider::TvMaze,
            provider_id: episode.id,
            season: episode.season as i32,
            episode: episode.number as i32,
            absolute_number: None,
            title: Some(episode.name.clone()),
            overview: episode.clean_summary(),
            air_date: episode.airdate.clone(),
            runtime: episode.runtime.map(|r| r as i32),
        }
    }
}

/// Create a sharable metadata service
pub fn create_metadata_service(db: Database, config: MetadataServiceConfig) -> Arc<MetadataService> {
    Arc::new(MetadataService::new(db, config))
}

/// Create a sharable metadata service with artwork caching
pub fn create_metadata_service_with_artwork(
    db: Database,
    config: MetadataServiceConfig,
    artwork_service: Arc<ArtworkService>,
) -> Arc<MetadataService> {
    Arc::new(MetadataService::new_with_artwork(db, config, artwork_service))
}

/// Normalize show status from metadata providers to database-compatible values.
/// 
/// Database constraint allows: 'continuing', 'ended', 'upcoming', 'cancelled', 'unknown'
/// 
/// This function maps various provider-specific status strings to these values:
/// - "Running" (TVMaze) → "continuing"
/// - "Ended" → "ended"  
/// - "Cancelled"/"Canceled" → "cancelled"
/// - "To Be Determined"/"TBD"/"In Development" → "upcoming"
/// - Anything else → "unknown"
pub fn normalize_show_status(status: Option<&str>) -> Option<String> {
    status.map(|s| {
        match s.to_lowercase().as_str() {
            "running" | "continuing" => "continuing",
            "ended" => "ended",
            "cancelled" | "canceled" => "cancelled",
            "to be determined" | "tbd" | "in development" | "upcoming" => "upcoming",
            _ => "unknown",
        }.to_string()
    })
}
