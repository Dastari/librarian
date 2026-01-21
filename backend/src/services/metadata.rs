//! Unified metadata service
//!
//! Provides a unified interface for fetching TV show metadata from multiple providers.
//! Priority: TVMaze (free, no key) → TMDB (if configured) → TheTVDB (if configured)

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::artwork::{ArtworkService, ArtworkType};
use super::audible::AudiobookMetadataClient;
use super::cache::{SharedCache, create_cache};
use super::filename_parser::{ParsedEpisode, parse_episode};
use super::musicbrainz::{MusicBrainzClient, MusicBrainzReleaseGroup};
use super::tmdb::{TmdbClient, normalize_movie_status};
use super::tvmaze::{TvMazeClient, TvMazeEpisode, TvMazeScheduleEntry, TvMazeShow};
use crate::db::{
    AlbumRecord, AudiobookRecord, CreateEpisode, CreateMovie, CreateTvShow, Database, MovieRecord,
    TvShowRecord,
};

/// Metadata provider enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetadataProvider {
    TvMaze,
    Tmdb,
    TvDb,
    MusicBrainz,
    OpenLibrary,
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

/// Unified movie search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieSearchResult {
    pub provider: MetadataProvider,
    pub provider_id: u32,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub imdb_id: Option<String>,
    pub vote_average: Option<f64>,
    pub popularity: Option<f64>,
}

/// Unified movie details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieDetails {
    pub provider: MetadataProvider,
    pub provider_id: u32,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub status: Option<String>,
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub genres: Vec<String>,
    pub runtime: Option<i32>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub imdb_id: Option<String>,
    pub director: Option<String>,
    pub cast_names: Vec<String>,
    pub production_countries: Vec<String>,
    pub spoken_languages: Vec<String>,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i32>,
    pub certification: Option<String>,
    pub collection_id: Option<i32>,
    pub collection_name: Option<String>,
    pub collection_poster_url: Option<String>,
    pub release_date: Option<String>,
}

/// Options for adding a movie from a metadata provider
#[derive(Debug, Clone)]
pub struct AddMovieOptions {
    /// Metadata provider to use (should be Tmdb for movies)
    pub provider: MetadataProvider,
    /// Provider-specific ID (e.g., TMDB ID)
    pub provider_id: u32,
    /// Library to add the movie to
    pub library_id: Uuid,
    /// User who owns the library
    pub user_id: Uuid,
    /// Whether to monitor for releases
    pub monitored: bool,
    /// Custom path within the library (optional)
    pub path: Option<String>,
}

/// Album search result from MusicBrainz
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumSearchResult {
    pub provider: MetadataProvider,
    /// MusicBrainz release group ID (UUID)
    pub provider_id: Uuid,
    pub title: String,
    pub artist_name: Option<String>,
    pub year: Option<i32>,
    pub album_type: Option<String>,
    pub cover_url: Option<String>,
    pub score: Option<i32>,
}

/// Artist search result from MusicBrainz
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistSearchResult {
    pub provider: MetadataProvider,
    /// MusicBrainz artist ID (UUID)
    pub provider_id: Uuid,
    pub name: String,
    pub sort_name: Option<String>,
    pub country: Option<String>,
    pub artist_type: Option<String>,
    pub disambiguation: Option<String>,
    pub score: Option<i32>,
}

/// Options for adding an album from MusicBrainz
#[derive(Debug, Clone)]
pub struct AddAlbumOptions {
    /// MusicBrainz release group ID (UUID)
    pub musicbrainz_id: Uuid,
    /// Library to add the album to
    pub library_id: Uuid,
    /// User who owns the library
    pub user_id: Uuid,
    /// Whether to monitor for releases
    pub monitored: bool,
}

/// Audiobook search result from OpenLibrary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudiobookSearchResult {
    pub provider: MetadataProvider,
    /// OpenLibrary work ID
    pub provider_id: String,
    pub title: String,
    pub author_name: Option<String>,
    pub year: Option<i32>,
    pub cover_url: Option<String>,
    pub isbn: Option<String>,
    pub description: Option<String>,
}

/// Options for adding an audiobook from OpenLibrary
#[derive(Debug, Clone)]
pub struct AddAudiobookOptions {
    /// OpenLibrary work ID
    pub openlibrary_id: String,
    /// Library to add the audiobook to
    pub library_id: Uuid,
    /// User who owns the library
    pub user_id: Uuid,
    /// Whether to monitor for releases
    pub monitored: bool,
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
    /// Custom path within the library (optional)
    pub path: Option<String>,
}

/// Unified metadata service
pub struct MetadataService {
    tvmaze: TvMazeClient,
    tmdb: Option<TmdbClient>,
    musicbrainz: MusicBrainzClient,
    openlibrary: AudiobookMetadataClient,
    config: MetadataServiceConfig,
    db: Database,
    artwork_service: Option<Arc<ArtworkService>>,
    /// Cache for TVMaze schedule data (TTL: 30 minutes)
    schedule_cache: SharedCache<Vec<TvMazeScheduleEntry>>,
}

/// Default cache TTL for TVMaze schedule: 30 minutes
const SCHEDULE_CACHE_TTL: Duration = Duration::from_secs(30 * 60);

impl MetadataService {
    pub fn new(db: Database, config: MetadataServiceConfig) -> Self {
        let tmdb = config
            .tmdb_api_key
            .as_ref()
            .filter(|k| !k.is_empty())
            .map(|k| TmdbClient::new(k.clone()));

        Self {
            tvmaze: TvMazeClient::new(),
            tmdb,
            musicbrainz: MusicBrainzClient::new_default(),
            openlibrary: AudiobookMetadataClient::new(),
            config,
            db,
            artwork_service: None,
            schedule_cache: create_cache(SCHEDULE_CACHE_TTL),
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
        let tmdb = config
            .tmdb_api_key
            .as_ref()
            .filter(|k| !k.is_empty())
            .map(|k| TmdbClient::new(k.clone()));

        Self {
            tvmaze: TvMazeClient::new(),
            tmdb,
            musicbrainz: MusicBrainzClient::new_default(),
            openlibrary: AudiobookMetadataClient::new(),
            config,
            db,
            artwork_service: Some(artwork_service),
            schedule_cache: create_cache(SCHEDULE_CACHE_TTL),
        }
    }

    /// Check if TMDB is configured
    pub fn has_tmdb(&self) -> bool {
        self.tmdb.is_some()
    }

    /// Get reference to artwork service if available
    pub fn artwork_service(&self) -> Option<&Arc<ArtworkService>> {
        self.artwork_service.as_ref()
    }

    /// Search for TV shows across providers
    pub async fn search_shows(&self, query: &str) -> Result<Vec<ShowSearchResult>> {
        info!("Searching for TV show '{}'", query);

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
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        debug!(count = results.len(), "Found shows");
        Ok(results)
    }

    /// Get show details from a provider
    pub async fn get_show(
        &self,
        provider: MetadataProvider,
        provider_id: u32,
    ) -> Result<ShowDetails> {
        debug!(
            "Fetching show details from {:?} (ID: {})",
            provider, provider_id
        );

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
            MetadataProvider::MusicBrainz => {
                anyhow::bail!("MusicBrainz is not for TV shows")
            }
            MetadataProvider::OpenLibrary => {
                anyhow::bail!("OpenLibrary is not for TV shows")
            }
        }
    }

    /// Get episodes for a show from a provider
    pub async fn get_episodes(
        &self,
        provider: MetadataProvider,
        provider_id: u32,
    ) -> Result<Vec<EpisodeDetails>> {
        debug!(
            "Fetching episodes from {:?} for show {}",
            provider, provider_id
        );

        match provider {
            MetadataProvider::TvMaze => {
                let episodes = self.tvmaze.get_episodes(provider_id).await?;
                Ok(episodes
                    .iter()
                    .map(|e| self.tvmaze_episode_to_details(e))
                    .collect())
            }
            MetadataProvider::Tmdb => {
                anyhow::bail!("TMDB not yet implemented")
            }
            MetadataProvider::TvDb => {
                anyhow::bail!("TheTVDB not yet implemented")
            }
            MetadataProvider::MusicBrainz => {
                anyhow::bail!("MusicBrainz is not for TV shows")
            }
            MetadataProvider::OpenLibrary => {
                anyhow::bail!("OpenLibrary is not for TV shows")
            }
        }
    }

    /// Parse a filename and try to identify the show
    pub async fn parse_and_identify(&self, title: &str) -> Result<ParseAndIdentifyResult> {
        info!("Parsing and identifying '{}'", title);

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

    // =========================================================================
    // Movie Methods
    // =========================================================================

    /// Search for movies using TMDB
    pub async fn search_movies(
        &self,
        query: &str,
        year: Option<i32>,
    ) -> Result<Vec<MovieSearchResult>> {
        info!(
            "Searching for movie '{}'{}",
            query,
            year.map(|y| format!(" ({})", y)).unwrap_or_default()
        );

        let tmdb = self.tmdb.as_ref().ok_or_else(|| {
            anyhow::anyhow!("TMDB API key not configured. Add tmdb_api_key to settings.")
        })?;

        let movies = tmdb.search_movies(query, year).await?;

        let results: Vec<MovieSearchResult> = movies
            .into_iter()
            .map(|m| {
                // Compute year before moving fields
                let year = m.year();
                let poster_url = tmdb.poster_url(m.poster_path.as_deref());
                let backdrop_url = tmdb.backdrop_url(m.backdrop_path.as_deref());

                MovieSearchResult {
                    provider: MetadataProvider::Tmdb,
                    provider_id: m.id as u32,
                    title: m.title,
                    original_title: m.original_title,
                    year,
                    overview: m.overview,
                    poster_url,
                    backdrop_url,
                    imdb_id: m.imdb_id,
                    vote_average: m.vote_average,
                    popularity: m.popularity,
                }
            })
            .collect();

        debug!(count = results.len(), "Found movies");
        Ok(results)
    }

    /// Get movie details from TMDB
    pub async fn get_movie(&self, tmdb_id: u32) -> Result<MovieDetails> {
        debug!("Fetching movie details from TMDB (ID: {})", tmdb_id);

        let tmdb = self
            .tmdb
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("TMDB API key not configured"))?;

        // Fetch movie details
        let movie = tmdb.get_movie(tmdb_id as i32).await?;

        // Fetch credits for director and cast
        let credits = tmdb.get_credits(tmdb_id as i32).await.ok();
        let director = credits.as_ref().and_then(|c| c.director());
        let cast_names = credits.as_ref().map(|c| c.top_cast(10)).unwrap_or_default();

        // Fetch release dates for certification
        let release_dates = tmdb.get_release_dates(tmdb_id as i32).await.ok();
        let certification = release_dates.as_ref().and_then(|r| r.us_certification());

        // Build collection info if available
        let (collection_id, collection_name, collection_poster_url) =
            if let Some(ref collection) = movie.belongs_to_collection {
                (
                    Some(collection.id),
                    Some(collection.name.clone()),
                    tmdb.poster_url(collection.poster_path.as_deref()),
                )
            } else {
                (None, None, None)
            };

        Ok(MovieDetails {
            provider: MetadataProvider::Tmdb,
            provider_id: tmdb_id,
            title: movie.title.clone(),
            original_title: movie.original_title.clone(),
            year: movie.year(),
            status: normalize_movie_status(movie.status.as_deref()),
            overview: movie.overview.clone(),
            tagline: movie.tagline.clone(),
            genres: movie.genre_names(),
            runtime: movie.runtime,
            poster_url: tmdb.original_url(movie.poster_path.as_deref()),
            backdrop_url: tmdb.original_url(movie.backdrop_path.as_deref()),
            imdb_id: movie.imdb_id.clone(),
            director,
            cast_names,
            production_countries: movie.country_codes(),
            spoken_languages: movie.language_codes(),
            vote_average: movie.vote_average,
            vote_count: movie.vote_count,
            certification,
            collection_id,
            collection_name,
            collection_poster_url,
            release_date: movie.release_date.clone(),
        })
    }

    /// Add a movie from TMDB to a library
    pub async fn add_movie_from_provider(&self, options: AddMovieOptions) -> Result<MovieRecord> {
        debug!(
            "Adding movie from {:?} (ID: {}) to library",
            options.provider, options.provider_id
        );

        // Only TMDB is supported for movies
        if options.provider != MetadataProvider::Tmdb {
            anyhow::bail!("Only TMDB is supported for movie metadata");
        }

        // Check if movie already exists in this library
        let movies_repo = self.db.movies();
        if let Some(existing) = movies_repo
            .get_by_tmdb_id(options.library_id, options.provider_id as i32)
            .await?
        {
            debug!("Movie '{}' already exists in library", existing.title);
            return Ok(existing);
        }

        // Get movie details from TMDB
        let movie_details = self.get_movie(options.provider_id).await?;

        // Cache artwork to Supabase storage if artwork service is available
        let (cached_poster_url, cached_backdrop_url) =
            if let Some(ref artwork_service) = self.artwork_service {
                let entity_id = format!("movie_{}_{}", options.provider_id, options.library_id);

                let poster_url = artwork_service
                    .cache_image_optional(
                        movie_details.poster_url.as_deref(),
                        ArtworkType::Poster,
                        "movie",
                        &entity_id,
                    )
                    .await;

                let backdrop_url = artwork_service
                    .cache_image_optional(
                        movie_details.backdrop_url.as_deref(),
                        ArtworkType::Backdrop,
                        "movie",
                        &entity_id,
                    )
                    .await;

                debug!(
                    "Cached artwork for movie (poster: {}, backdrop: {})",
                    poster_url.is_some(),
                    backdrop_url.is_some()
                );

                (poster_url, backdrop_url)
            } else {
                (
                    movie_details.poster_url.clone(),
                    movie_details.backdrop_url.clone(),
                )
            };

        // Parse release date
        let release_date = movie_details
            .release_date
            .as_ref()
            .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());

        // Convert vote_average to Decimal
        let tmdb_rating = movie_details
            .vote_average
            .map(|v| rust_decimal::Decimal::from_f64_retain(v))
            .flatten();

        // Create the movie in the database
        let movies_repo = self.db.movies();
        let movie = movies_repo
            .create(CreateMovie {
                library_id: options.library_id,
                user_id: options.user_id,
                title: movie_details.title.clone(),
                sort_title: None, // Could generate this later
                original_title: movie_details.original_title,
                year: movie_details.year,
                tmdb_id: Some(options.provider_id as i32),
                imdb_id: movie_details.imdb_id,
                overview: movie_details.overview,
                tagline: movie_details.tagline,
                runtime: movie_details.runtime,
                genres: movie_details.genres,
                production_countries: movie_details.production_countries,
                spoken_languages: movie_details.spoken_languages,
                director: movie_details.director,
                cast_names: movie_details.cast_names,
                tmdb_rating,
                tmdb_vote_count: movie_details.vote_count,
                poster_url: cached_poster_url,
                backdrop_url: cached_backdrop_url,
                collection_id: movie_details.collection_id,
                collection_name: movie_details.collection_name,
                collection_poster_url: movie_details.collection_poster_url,
                release_date,
                certification: movie_details.certification,
                status: movie_details.status,
                monitored: options.monitored,
                path: options.path,
            })
            .await?;

        info!("Added movie '{}' to library", movie.title);

        Ok(movie)
    }

    // =========================================================================
    // Music Methods
    // =========================================================================

    /// Search for artists using MusicBrainz
    pub async fn search_artists(&self, query: &str) -> Result<Vec<ArtistSearchResult>> {
        info!("Searching MusicBrainz for artist '{}'", query);

        let artists = self.musicbrainz.search_artists(query).await?;

        let results: Vec<ArtistSearchResult> = artists
            .into_iter()
            .map(|a| ArtistSearchResult {
                provider: MetadataProvider::MusicBrainz,
                provider_id: a.id,
                name: a.name,
                sort_name: a.sort_name,
                country: a.country,
                artist_type: a.artist_type,
                disambiguation: a.disambiguation,
                score: a.score,
            })
            .collect();

        debug!(count = results.len(), "Found artists");
        Ok(results)
    }

    /// Search for albums using MusicBrainz
    pub async fn search_albums(&self, query: &str) -> Result<Vec<AlbumSearchResult>> {
        info!("Searching MusicBrainz for album '{}'", query);

        let albums = self.musicbrainz.search_albums(query).await?;

        // Use Cover Art Archive's direct URL pattern for thumbnails
        // This avoids making any HTTP requests - the URL will redirect to the actual image
        // If no cover exists, the frontend will handle the 404 gracefully
        let results: Vec<AlbumSearchResult> = albums
            .into_iter()
            .map(|album| {
                // Cover Art Archive provides a redirect URL for release groups
                // front-250 gives a 250px thumbnail, perfect for search results
                let cover_url = Some(format!(
                    "https://coverartarchive.org/release-group/{}/front-250",
                    album.id
                ));

                AlbumSearchResult {
                    provider: MetadataProvider::MusicBrainz,
                    provider_id: album.id,
                    title: album.title.clone(),
                    artist_name: album.artist_names(),
                    year: album.year(),
                    album_type: Some(album.normalized_type()),
                    cover_url,
                    score: album.score,
                }
            })
            .collect();

        debug!(count = results.len(), "Found albums");
        Ok(results)
    }

    /// Search for albums using MusicBrainz with type filtering
    ///
    /// Types can include: "Album", "EP", "Single", "Compilation", "Live", "Soundtrack"
    pub async fn search_albums_with_types(
        &self,
        query: &str,
        types: &[String],
    ) -> Result<Vec<AlbumSearchResult>> {
        info!(
            "Searching MusicBrainz for album '{}' with types {:?}",
            query, types
        );

        let albums = self
            .musicbrainz
            .search_albums_with_types(query, types)
            .await?;

        // Use Cover Art Archive's direct URL pattern for thumbnails
        let results: Vec<AlbumSearchResult> = albums
            .into_iter()
            .map(|album| {
                let cover_url = Some(format!(
                    "https://coverartarchive.org/release-group/{}/front-250",
                    album.id
                ));

                AlbumSearchResult {
                    provider: MetadataProvider::MusicBrainz,
                    provider_id: album.id,
                    title: album.title.clone(),
                    artist_name: album.artist_names(),
                    year: album.year(),
                    album_type: Some(album.normalized_type()),
                    cover_url,
                    score: album.score,
                }
            })
            .collect();

        debug!(count = results.len(), "Found albums");
        Ok(results)
    }

    /// Get album details from MusicBrainz
    pub async fn get_album(&self, musicbrainz_id: Uuid) -> Result<MusicBrainzReleaseGroup> {
        debug!(
            "Fetching album details from MusicBrainz (ID: {})",
            musicbrainz_id
        );
        self.musicbrainz.get_release_group(musicbrainz_id).await
    }

    /// Add an album from MusicBrainz to a library
    ///
    /// This fetches album metadata, cover art, and track listings from MusicBrainz.
    pub async fn add_album_from_provider(&self, options: AddAlbumOptions) -> Result<AlbumRecord> {
        debug!(
            "Adding album {} from MusicBrainz to library",
            options.musicbrainz_id
        );

        let albums_repo = self.db.albums();
        let tracks_repo = self.db.tracks();

        // Check if album already exists in this library
        if let Some(existing) = albums_repo
            .get_by_musicbrainz_id(options.library_id, options.musicbrainz_id)
            .await?
        {
            debug!("Album '{}' already exists in library", existing.name);
            return Ok(existing);
        }

        // Get album details from MusicBrainz
        let album_details = self
            .musicbrainz
            .get_release_group(options.musicbrainz_id)
            .await?;

        // Get artist info if available
        let (artist_id, artist_name) = if let Some(ref credits) = album_details.artist_credit {
            if let Some(first_credit) = credits.first() {
                // Find or create the artist
                let artist = albums_repo
                    .find_or_create_artist(
                        options.library_id,
                        options.user_id,
                        &first_credit.artist.name,
                        first_credit.artist.sort_name.as_deref(),
                        Some(first_credit.artist.id),
                    )
                    .await?;
                (artist.id, first_credit.artist.name.clone())
            } else {
                anyhow::bail!("Album has no artist credits");
            }
        } else {
            anyhow::bail!("Album has no artist information");
        };

        // Get cover art
        info!(
            musicbrainz_id = %options.musicbrainz_id,
            album_name = %album_details.title,
            "Fetching cover art for album"
        );
        let cover_url = match self.musicbrainz.get_cover_art(options.musicbrainz_id).await {
            Ok(url) => {
                info!(
                    musicbrainz_id = %options.musicbrainz_id,
                    cover_url = ?url,
                    "Cover art fetch result"
                );
                url
            }
            Err(e) => {
                warn!(
                    musicbrainz_id = %options.musicbrainz_id,
                    error = %e,
                    "Failed to fetch cover art from Cover Art Archive"
                );
                None
            }
        };

        // Cache artwork if available
        let cached_cover_url = if let Some(ref artwork_service) = self.artwork_service {
            if let Some(ref url) = cover_url {
                let entity_id = format!("album_{}", options.musicbrainz_id);
                info!(
                    musicbrainz_id = %options.musicbrainz_id,
                    original_url = %url,
                    entity_id = %entity_id,
                    "Caching album cover art to storage"
                );
                match artwork_service
                    .cache_image(url, ArtworkType::Poster, "album", &entity_id)
                    .await
                {
                    Ok(cached_url) => {
                        info!(
                            musicbrainz_id = %options.musicbrainz_id,
                            original_url = %url,
                            cached_url = %cached_url,
                            "Successfully cached album cover art"
                        );
                        Some(cached_url)
                    }
                    Err(e) => {
                        warn!(
                            musicbrainz_id = %options.musicbrainz_id,
                            error = %e,
                            original_url = %url,
                            "Failed to cache album cover art"
                        );
                        cover_url
                    }
                }
            } else {
                info!(
                    musicbrainz_id = %options.musicbrainz_id,
                    "No cover art URL available to cache (CAA returned none)"
                );
                None
            }
        } else {
            warn!(
                musicbrainz_id = %options.musicbrainz_id,
                "No artwork service available - cover art will not be cached"
            );
            cover_url
        };

        // Get track listing from MusicBrainz
        let track_list = self
            .musicbrainz
            .get_tracks_for_release_group(options.musicbrainz_id)
            .await
            .unwrap_or_else(|e| {
                warn!(error = %e, "Failed to fetch track listing, continuing without tracks");
                vec![]
            });

        // Calculate disc count and track count
        let disc_count = track_list.iter().map(|t| t.disc_number).max();
        let track_count = Some(track_list.len() as i32);

        // Create album record
        let album = albums_repo
            .create(crate::db::albums::CreateAlbum {
                artist_id,
                library_id: options.library_id,
                user_id: options.user_id,
                name: album_details.title.clone(),
                sort_name: None,
                year: album_details.year(),
                musicbrainz_id: Some(options.musicbrainz_id),
                album_type: Some(album_details.normalized_type()),
                genres: vec![],
                label: None,
                country: None,
                release_date: None,
                cover_url: cached_cover_url,
                track_count,
                disc_count,
            })
            .await?;

        info!(
            "Added album '{}' by {} ({} tracks) to library",
            album.name,
            artist_name,
            track_list.len()
        );

        // Create track records
        if !track_list.is_empty() {
            // Check library settings to determine initial track status
            // If auto_hunt or auto_download is enabled, tracks should be "wanted" so they get hunted
            let initial_status = {
                let library = self.db.libraries().get_by_id(options.library_id).await?;
                if let Some(lib) = library {
                    if lib.auto_hunt || lib.auto_download {
                        Some("wanted".to_string())
                    } else {
                        Some("missing".to_string())
                    }
                } else {
                    Some("missing".to_string())
                }
            };

            let tracks_to_create: Vec<crate::db::CreateTrack> = track_list
                .into_iter()
                .map(|t| crate::db::CreateTrack {
                    album_id: album.id,
                    library_id: options.library_id,
                    title: t.title,
                    track_number: t.track_number,
                    disc_number: t.disc_number,
                    musicbrainz_id: Some(t.musicbrainz_id),
                    isrc: t.isrc,
                    duration_secs: t.duration_secs,
                    explicit: false,
                    artist_name: t.artist_name,
                    artist_id: None, // Could look up artist if different from album artist
                    status: initial_status.clone(),
                })
                .collect();

            match tracks_repo.create_many(tracks_to_create).await {
                Ok(created_tracks) => {
                    debug!(
                        "Created {} track records for album '{}' with status '{}'",
                        created_tracks.len(),
                        album.name,
                        initial_status.as_deref().unwrap_or("missing")
                    );
                }
                Err(e) => {
                    warn!("Failed to create track records for '{}': {}", album.name, e);
                }
            }
        }

        Ok(album)
    }

    // =========================================================================
    // Audiobook Methods
    // =========================================================================

    /// Search for audiobooks using OpenLibrary
    pub async fn search_audiobooks(&self, query: &str) -> Result<Vec<AudiobookSearchResult>> {
        info!("Searching OpenLibrary for audiobook '{}'", query);
        let results = self.openlibrary.search(query).await?;
        let audiobooks: Vec<AudiobookSearchResult> = results
            .into_iter()
            .map(|r| AudiobookSearchResult {
                provider: MetadataProvider::OpenLibrary,
                provider_id: r.openlibrary_id.unwrap_or_default(),
                title: r.title,
                author_name: r.author_name,
                year: r.year,
                cover_url: r.cover_url,
                isbn: r.isbn,
                description: r.description,
            })
            .collect();
        debug!(count = audiobooks.len(), "Found audiobooks");
        Ok(audiobooks)
    }

    /// Add an audiobook from OpenLibrary to a library
    pub async fn add_audiobook_from_provider(
        &self,
        options: AddAudiobookOptions,
    ) -> Result<AudiobookRecord> {
        debug!("Adding audiobook {} to library", options.openlibrary_id);
        let audiobooks_repo = self.db.audiobooks();
        if let Some(existing) = audiobooks_repo
            .get_by_openlibrary_id(options.library_id, &options.openlibrary_id)
            .await?
        {
            return Ok(existing);
        }
        let details = self.openlibrary.get_work(&options.openlibrary_id).await?;
        let author = audiobooks_repo
            .find_or_create_author(
                options.library_id,
                options.user_id,
                &details.author_name,
                None,
            )
            .await?;
        let cover_url = details.cover_url.clone();
        let audiobook = audiobooks_repo
            .create(crate::db::CreateAudiobook {
                author_id: Some(author.id),
                library_id: options.library_id,
                user_id: options.user_id,
                title: details.title.clone(),
                sort_title: None,
                subtitle: details.subtitle,
                openlibrary_id: Some(options.openlibrary_id),
                isbn: details.isbn,
                description: details.description,
                publisher: details.publishers.first().cloned(),
                language: details.language,
                cover_url,
            })
            .await?;
        Ok(audiobook)
    }

    /// Get upcoming TV schedule (database cached)
    ///
    /// Fetches episodes airing in the next N days from the database cache.
    /// The cache is populated by a background job that syncs from TVMaze every 6 hours.
    /// Falls back to direct TVMaze API if cache is empty.
    /// Optionally filter by country code (e.g., "US", "GB").
    pub async fn get_upcoming_schedule(
        &self,
        days: u32,
        country: Option<&str>,
    ) -> Result<Vec<TvMazeScheduleEntry>> {
        let country_code = country.unwrap_or("US");
        let repo = self.db.schedule();

        // Try to get from database cache first
        let cached_entries = repo.get_upcoming(days as i32, Some(country_code)).await?;

        if !cached_entries.is_empty() {
            debug!(
                country = country_code,
                count = cached_entries.len(),
                "Returning schedule from database cache"
            );

            // Convert database records to TvMazeScheduleEntry format
            let schedule: Vec<TvMazeScheduleEntry> = cached_entries
                .into_iter()
                .map(|record| {
                    use super::tvmaze::{TvMazeImage, TvMazeNetwork};

                    TvMazeScheduleEntry {
                        id: record.tvmaze_episode_id as u32,
                        name: record.episode_name,
                        season: Some(record.season as u32),
                        number: Some(record.episode_number as u32),
                        episode_type: record.episode_type,
                        airdate: Some(record.air_date.to_string()),
                        airtime: record.air_time,
                        air_stamp: record.air_stamp.map(|ts| ts.to_string()),
                        runtime: record.runtime.map(|r| r as u32),
                        image: record.episode_image_url.map(|url| TvMazeImage {
                            medium: Some(url),
                            original: None,
                        }),
                        summary: record.summary,
                        rating: None,
                        show: super::tvmaze::TvMazeShow {
                            id: record.tvmaze_show_id as u32,
                            name: record.show_name,
                            show_type: None,
                            language: None,
                            genres: record.show_genres,
                            status: None,
                            runtime: None,
                            average_runtime: None,
                            premiered: None,
                            ended: None,
                            official_site: None,
                            network: record.show_network.clone().map(|name| TvMazeNetwork {
                                id: 0,
                                name,
                                country: None,
                            }),
                            web_channel: None,
                            image: record.show_poster_url.map(|url| TvMazeImage {
                                medium: Some(url),
                                original: None,
                            }),
                            summary: None,
                            rating: None,
                            externals: None,
                        },
                    }
                })
                .collect();

            return Ok(schedule);
        }

        // Cache is empty - fall back to direct API call
        // This should only happen on first run before the sync job runs
        debug!(
            "Schedule cache empty, fetching {} days from TVMaze for {}",
            days, country_code
        );

        // Check in-memory cache as a secondary fallback
        let cache_key = format!("schedule:{}:{}", days, country_code);
        if let Some(cached) = self.schedule_cache.get(&cache_key) {
            debug!(cache_key = %cache_key, "Returning in-memory cached schedule");
            return Ok(cached);
        }

        // Fetch from TVMaze API
        let schedule = self
            .tvmaze
            .get_upcoming_schedule(days, Some(country_code))
            .await?;

        // Cache in memory for immediate subsequent requests
        self.schedule_cache.set(cache_key, schedule.clone());

        Ok(schedule)
    }

    /// Force refresh the schedule cache for a country
    ///
    /// This bypasses the database cache and fetches fresh data from TVMaze,
    /// then updates both the database and in-memory caches.
    pub async fn refresh_schedule_cache(&self, days: u32, country: Option<&str>) -> Result<usize> {
        use crate::jobs::schedule_sync;

        let country_code = country.unwrap_or("US");
        info!(
            "Force refreshing {} day TV schedule cache for {}",
            days, country_code
        );

        // Use the sync job to update the database
        let count =
            schedule_sync::sync_country_on_demand(self.db.pool().clone(), country_code, days)
                .await?;

        // Clear in-memory cache for this country
        let cache_key = format!("schedule:{}:{}", days, country_code);
        self.schedule_cache.remove(&cache_key);

        Ok(count)
    }

    /// Add a TV show from a metadata provider to a library.
    ///
    /// This is the single code path for creating TV shows, used by both:
    /// - The addTvShow GraphQL mutation (manual add)
    /// - The scanner service (automatic discovery)
    ///
    /// It handles:
    /// 1. Checking if show already exists (returns existing if so)
    /// 2. Fetching show details from the provider
    /// 3. Caching artwork to Supabase storage
    /// 4. Creating the TV show record with normalized status
    /// 5. Fetching and creating all episode records
    /// 6. Updating show statistics
    pub async fn add_tv_show_from_provider(
        &self,
        options: AddTvShowOptions,
    ) -> Result<TvShowRecord> {
        debug!(
            "Adding TV show from {:?} (ID: {}) to library",
            options.provider, options.provider_id
        );

        // Check if show already exists in this library
        let tv_shows_repo = self.db.tv_shows();
        if options.provider == MetadataProvider::TvMaze {
            if let Some(existing) = tv_shows_repo
                .get_by_tvmaze_id(options.library_id, options.provider_id as i32)
                .await?
            {
                debug!("TV show '{}' already exists in library", existing.name);
                return Ok(existing);
            }
        }

        // Get show details from provider
        let show_details = self.get_show(options.provider, options.provider_id).await?;

        // Cache artwork to Supabase storage if artwork service is available
        let (cached_poster_url, cached_backdrop_url) =
            if let Some(ref artwork_service) = self.artwork_service {
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

                debug!(
                    "Cached artwork for show (poster: {}, backdrop: {})",
                    poster_url.is_some(),
                    backdrop_url.is_some()
                );

                (poster_url, backdrop_url)
            } else {
                // No artwork service, use original URLs
                (
                    show_details.poster_url.clone(),
                    show_details.backdrop_url.clone(),
                )
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
                path: options.path.clone(),
                auto_download_override: None,  // Inherit from library
                backfill_existing: true,       // Default to true for new shows
                organize_files_override: None, // Inherit from library
                rename_style_override: None,   // Inherit from library
                auto_hunt_override: None,      // Inherit from library
                // Quality override settings - inherit from library by default
                allowed_resolutions_override: None,
                allowed_video_codecs_override: None,
                allowed_audio_formats_override: None,
                require_hdr_override: None,
                allowed_hdr_types_override: None,
                allowed_sources_override: None,
                release_group_blacklist_override: None,
                release_group_whitelist_override: None,
            })
            .await?;

        info!("Added TV show '{}' to library", tv_show.name);

        // Fetch and create episodes
        match self
            .get_episodes(options.provider, options.provider_id)
            .await
        {
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

                info!("Created {} episodes for '{}'", created_count, tv_show.name);
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

        // Backfill from RSS cache - check if we have any cached torrent links for this show
        match self.backfill_episodes_from_rss_cache(&tv_show).await {
            Ok(matched_count) => {
                if matched_count > 0 {
                    info!(
                        "Backfilled {} episodes for '{}' from RSS cache",
                        matched_count, tv_show.name
                    );
                }
            }
            Err(e) => {
                warn!(
                    show_id = %tv_show.id,
                    error = %e,
                    "Failed to backfill from RSS cache"
                );
            }
        }

        Ok(tv_show)
    }

    /// Backfill episode availability from cached RSS feed items
    ///
    /// When a show is added to the library, check if we have any cached RSS items
    /// that match this show's episodes and mark them as "available".
    async fn backfill_episodes_from_rss_cache(
        &self,
        tv_show: &crate::db::TvShowRecord,
    ) -> Result<i32> {
        use crate::db::episodes::EpisodeRecord;

        // Normalize the show name for matching
        let normalized_name = tv_show
            .name
            .to_lowercase()
            .replace(['.', '-', '_'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        debug!("Searching RSS cache for episodes of '{}'", tv_show.name);

        // Find RSS items that match this show name
        let matching_items: Vec<(uuid::Uuid, Option<String>, Option<i32>, Option<i32>, String)> = sqlx::query_as(
            r#"
            SELECT id, parsed_show_name, parsed_season, parsed_episode, link
            FROM rss_feed_items
            WHERE parsed_show_name IS NOT NULL
              AND parsed_season IS NOT NULL
              AND parsed_episode IS NOT NULL
              AND (
                  LOWER(REPLACE(REPLACE(REPLACE(parsed_show_name, '.', ' '), '-', ' '), '_', ' '))
                  LIKE '%' || $1 || '%'
                  OR $1 LIKE '%' || LOWER(REPLACE(REPLACE(REPLACE(parsed_show_name, '.', ' '), '-', ' '), '_', ' ')) || '%'
              )
            ORDER BY pub_date DESC
            "#,
        )
        .bind(&normalized_name)
        .fetch_all(self.db.pool())
        .await?;

        if matching_items.is_empty() {
            debug!(
                show_name = %tv_show.name,
                "No cached RSS items found for this show"
            );
            return Ok(0);
        }

        debug!(
            "Found {} cached RSS items for '{}'",
            matching_items.len(),
            tv_show.name
        );

        // Get all seasons for this show to handle year-based season mapping
        let seasons: Vec<(i32,)> = sqlx::query_as(
            "SELECT DISTINCT season FROM episodes WHERE tv_show_id = $1 ORDER BY season",
        )
        .bind(tv_show.id)
        .fetch_all(self.db.pool())
        .await?;

        let season_numbers: Vec<i32> = seasons.into_iter().map(|(s,)| s).collect();
        let uses_year_seasons = season_numbers.iter().any(|&s| s > 2000);

        let mut matched_count = 0;

        for (rss_item_id, _show_name, season, episode, torrent_link) in matching_items {
            let scene_season = match season {
                Some(s) => s,
                None => continue,
            };
            let scene_episode = match episode {
                Some(e) => e,
                None => continue,
            };

            // Try to find matching episode using various strategies
            let matched_episode: Option<EpisodeRecord> = if uses_year_seasons {
                // Strategy: Map scene season to year-based season
                if scene_season > 0 && scene_season <= season_numbers.len() as i32 {
                    let target_year_season = season_numbers[(scene_season - 1) as usize];

                    // Get episodes for that season and pick the nth one
                    let eps: Vec<EpisodeRecord> = sqlx::query_as(
                        r#"
                        SELECT * FROM episodes
                        WHERE tv_show_id = $1 AND season = $2
                        ORDER BY episode
                        "#,
                    )
                    .bind(tv_show.id)
                    .bind(target_year_season)
                    .fetch_all(self.db.pool())
                    .await?;

                    eps.get((scene_episode - 1) as usize).cloned()
                } else {
                    None
                }
            } else {
                // Standard matching: exact season/episode
                self.db
                    .episodes()
                    .get_by_show_season_episode(tv_show.id, scene_season, scene_episode)
                    .await?
            };

            if let Some(ep) = matched_episode {
                // Only update if episode is wanted/missing
                if ep.status == "missing" || ep.status == "wanted" {
                    if let Err(e) = self
                        .db
                        .episodes()
                        .mark_available(ep.id, &torrent_link, Some(rss_item_id))
                        .await
                    {
                        warn!(
                            episode_id = %ep.id,
                            error = %e,
                            "Failed to mark episode as available from RSS cache"
                        );
                    } else {
                        debug!(
                            "Backfilled {} S{:02}E{:02} from RSS cache",
                            tv_show.name, ep.season, ep.episode
                        );
                        matched_count += 1;
                    }
                }
            }
        }

        Ok(matched_count)
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

/// Create a sharable metadata service - use create_metadata_service_with_artwork instead
#[allow(dead_code)]
pub fn create_metadata_service(
    db: Database,
    config: MetadataServiceConfig,
) -> Arc<MetadataService> {
    Arc::new(MetadataService::new(db, config))
}

/// Create a sharable metadata service with artwork caching
pub fn create_metadata_service_with_artwork(
    db: Database,
    config: MetadataServiceConfig,
    artwork_service: Arc<ArtworkService>,
) -> Arc<MetadataService> {
    Arc::new(MetadataService::new_with_artwork(
        db,
        config,
        artwork_service,
    ))
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
        }
        .to_string()
    })
}
