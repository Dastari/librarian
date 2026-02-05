//! Unified metadata service for TMDB movie search
//!
//! Provides a simplified interface for searching movies on TMDB.
//! API key is loaded from app_settings database table.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use async_graphql::{Request, Variables};
use super::musicbrainz::MusicBrainzClient;
use super::settings::MetadataSettings;
use super::tmdb::TmdbClient;
use super::tvmaze::TvMazeClient;
use crate::db::Database;
use crate::services::graphql::{AuthUser, LibrarianSchema};
use crate::services::manager::ServicesManager;
use crate::services::graphql::entities::{Movie, Show};
use crate::services::graphql::entities::common::AutoDownloadMode;

/// Metadata provider enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetadataProvider {
    Tmdb,
    Tvmaze,
    Musicbrainz,
    OpenLibrary,
}

/// Unified movie search result (from TMDB search)
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

/// Unified movie details (from TMDB get_movie)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieDetails {
    pub provider: MetadataProvider,
    pub provider_id: u32,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i32>,
    pub tmdb_status: Option<String>,
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

/// Unified TV show search result (from TVMaze search)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvShowSearchResult {
    pub provider: MetadataProvider,
    pub provider_id: u32,
    pub name: String,
    pub year: Option<i32>,
    pub status: Option<String>,
    pub network: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub tvdb_id: Option<i32>,
    pub imdb_id: Option<String>,
    pub score: Option<f64>,
}

/// Unified TV show details (from TVMaze get_show)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvShowDetails {
    pub provider: MetadataProvider,
    pub provider_id: u32,
    pub name: String,
    pub sort_name: Option<String>,
    pub year: Option<i32>,
    pub status: Option<String>,
    pub overview: Option<String>,
    pub network: Option<String>,
    pub runtime: Option<i32>,
    pub genres: Vec<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub tvdb_id: Option<i32>,
    pub imdb_id: Option<String>,
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
    /// Whether to monitor for releases (sets wanted = monitored)
    pub monitored: bool,
}

/// Options for adding a TV show from a metadata provider
#[derive(Debug, Clone)]
pub struct AddTvShowOptions {
    pub provider: MetadataProvider,
    pub provider_id: u32,
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub monitor_type: AutoDownloadMode,
    pub path: Option<String>,
}

/// Metadata service configuration
#[derive(Debug, Clone, Default)]
pub struct MetadataServiceConfig {
    pub tmdb_api_key: Option<String>,
}

/// Unified metadata service
pub struct MetadataService {
    /// TMDB client - wrapped in RwLock to allow dynamic reloading when settings change
    tmdb: RwLock<Option<TmdbClient>>,
    /// TVMaze client - no API key, but cached for reuse
    tvmaze: RwLock<Option<Arc<TvMazeClient>>>,
    /// MusicBrainz client - user agent may change via settings
    musicbrainz: RwLock<Option<Arc<MusicBrainzClient>>>,
    musicbrainz_user_agent: RwLock<Option<String>>,
    db: Database,
    services: Arc<ServicesManager>,
}

impl MetadataService {
    pub fn new(db: Database, services: Arc<ServicesManager>, config: MetadataServiceConfig) -> Self {
        let tmdb = config
            .tmdb_api_key
            .as_ref()
            .filter(|k| !k.is_empty())
            .map(|k| TmdbClient::new(k.clone()));

        Self {
            tmdb: RwLock::new(tmdb),
            tvmaze: RwLock::new(None),
            musicbrainz: RwLock::new(None),
            musicbrainz_user_agent: RwLock::new(None),
            db,
            services,
        }
    }

    async fn graphql_schema(&self) -> Result<LibrarianSchema> {
        let graphql = self
            .services
            .get_graphql()
            .await
            .ok_or_else(|| anyhow::anyhow!("GraphQL service not available"))?;
        graphql
            .schema()
            .await
            .ok_or_else(|| anyhow::anyhow!("GraphQL schema not available"))
    }

    async fn execute_query(
        &self,
        auth_user: &AuthUser,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let schema = self.graphql_schema().await?;
        let request = Request::new(query)
            .variables(Variables::from_json(variables))
            .data(auth_user.clone());
        let response = schema.execute(request).await;
        if !response.errors.is_empty() {
            let msg = response
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(anyhow::anyhow!(msg));
        }
        Ok(serde_json::to_value(&response.data)?)
    }

    async fn execute_mutation(
        &self,
        auth_user: &AuthUser,
        mutation: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.execute_query(auth_user, mutation, variables).await
    }

    fn now_iso_string() -> String {
        chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string()
    }

    /// Create with default config (no API keys)
    pub fn new_default(db: Database, services: Arc<ServicesManager>) -> Self {
        Self::new(db, services, MetadataServiceConfig::default())
    }

    /// Check if TMDB is configured by checking the database for an API key
    pub async fn has_tmdb(&self) -> bool {
        // First check cached client
        if self.tmdb.read().await.is_some() {
            return true;
        }

        // Check database for API key
        match MetadataSettings::load(&self.db).await {
            Ok(settings) => settings.tmdb_api_key.is_some(),
            Err(_) => false,
        }
    }

    /// Get TMDB client, always checking the database for the latest API key
    ///
    /// This ensures that if the user updates the API key via settings,
    /// the next request will use the new key without requiring a server restart.
    async fn get_tmdb_client(&self) -> Result<TmdbClient> {
        let settings = MetadataSettings::load(&self.db).await?;
        let Some(key) = settings.tmdb_api_key else {
            anyhow::bail!("TMDB API key not configured. Add tmdb_api_key to settings.")
        };

        // Check if we have a cached client with the same key
        {
            let guard = self.tmdb.read().await;
            if let Some(ref client) = *guard {
                if client.api_key() == key {
                    return Ok(client.clone());
                }
                // Key changed, will create new client below
                debug!("TMDB API key changed, creating new client");
            }
        }

        // Create new client with current key
        info!("Creating TMDB client with API key from database");
        let client = TmdbClient::new(key);

        // Cache the client
        let mut guard = self.tmdb.write().await;
        *guard = Some(client.clone());

        Ok(client)
    }

    /// Get TVMaze client (no API key required)
    pub async fn get_tvmaze_client(&self) -> Result<Arc<TvMazeClient>> {
        if let Some(client) = self.tvmaze.read().await.as_ref() {
            return Ok(client.clone());
        }

        let client = Arc::new(TvMazeClient::new());
        let mut guard = self.tvmaze.write().await;
        *guard = Some(client.clone());
        Ok(client)
    }

    /// Get MusicBrainz client, honoring the user-agent setting
    pub async fn get_musicbrainz_client(&self) -> Result<Arc<MusicBrainzClient>> {
        let settings = MetadataSettings::load(&self.db).await?;
        let default_user_agent = MusicBrainzClient::default_user_agent();
        let desired_user_agent = settings
            .musicbrainz_user_agent
            .filter(|ua| !ua.trim().is_empty())
            .unwrap_or(default_user_agent);

        {
            let client_guard = self.musicbrainz.read().await;
            let ua_guard = self.musicbrainz_user_agent.read().await;
            if let (Some(client), Some(current_ua)) = (&*client_guard, &*ua_guard) {
                if current_ua == &desired_user_agent {
                    return Ok(client.clone());
                }
            }
        }

        let client = Arc::new(MusicBrainzClient::new_with_user_agent(
            desired_user_agent.clone(),
        ));
        let mut client_guard = self.musicbrainz.write().await;
        let mut ua_guard = self.musicbrainz_user_agent.write().await;
        *client_guard = Some(client.clone());
        *ua_guard = Some(desired_user_agent);
        Ok(client)
    }

    /// Search for movies on TMDB
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

        let tmdb = self.get_tmdb_client().await?;

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

    /// Search for TV shows on TVMaze
    pub async fn search_tv_shows(&self, query: &str) -> Result<Vec<TvShowSearchResult>> {
        info!("Searching for TV shows '{}'", query);

        let tvmaze = self.get_tvmaze_client().await?;
        let results = tvmaze.search_shows(query).await?;

        let shows: Vec<TvShowSearchResult> = results
            .into_iter()
            .map(|r| {
                let show = r.show;
                let network = show
                    .network
                    .as_ref()
                    .map(|n| n.name.clone())
                    .or_else(|| show.web_channel.as_ref().map(|c| c.name.clone()));
                let overview = show.clean_summary();
                TvShowSearchResult {
                    provider: MetadataProvider::Tvmaze,
                    provider_id: show.id,
                    name: show.name.clone(),
                    year: show
                        .premiered
                        .as_ref()
                        .and_then(|d| d.get(0..4))
                        .and_then(|y| y.parse().ok()),
                    status: show.status.clone(),
                    network,
                    overview,
                    poster_url: show.image.as_ref().and_then(|i| i.medium.clone()),
                    tvdb_id: show.externals.as_ref().and_then(|e| e.thetvdb).map(|v| v as i32),
                    imdb_id: show.externals.as_ref().and_then(|e| e.imdb.clone()),
                    score: Some(r.score),
                }
            })
            .collect();

        debug!(count = shows.len(), "Found TV shows");
        Ok(shows)
    }

    /// Get movie details from TMDB
    pub async fn get_movie(&self, tmdb_id: u32) -> Result<MovieDetails> {
        debug!("Fetching movie details from TMDB (ID: {})", tmdb_id);

        let tmdb = self.get_tmdb_client().await?;

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
            tmdb_status: super::tmdb::normalize_movie_status(movie.status.as_deref()),
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

    /// Get TV show details from TVMaze
    pub async fn get_tv_show(&self, tvmaze_id: u32) -> Result<TvShowDetails> {
        debug!("Fetching TV show details from TVMaze (ID: {})", tvmaze_id);

        let tvmaze = self.get_tvmaze_client().await?;
        let show = tvmaze.get_show(tvmaze_id).await?;

        let network = show
            .network
            .as_ref()
            .map(|n| n.name.clone())
            .or_else(|| show.web_channel.as_ref().map(|c| c.name.clone()));

        Ok(TvShowDetails {
            provider: MetadataProvider::Tvmaze,
            provider_id: tvmaze_id,
            name: show.name.clone(),
            sort_name: Some(show.name.clone()),
            year: show.premiered.as_ref().and_then(|d| d.get(0..4)).and_then(|y| y.parse().ok()),
            status: show.status.clone(),
            overview: show.clean_summary(),
            network,
            runtime: show.average_runtime.or(show.runtime).map(|v| v as i32),
            genres: show.genres.clone(),
            poster_url: show.image.as_ref().and_then(|i| i.original.clone()),
            backdrop_url: None,
            tvdb_id: show.externals.as_ref().and_then(|e| e.thetvdb).map(|v| v as i32),
            imdb_id: show.externals.as_ref().and_then(|e| e.imdb.clone()),
        })
    }

    /// Add a movie from TMDB to a library
    ///
    /// Fetches movie metadata from TMDB and creates a movie record using the
    /// entity's create_from_metadata method (single source of truth pattern).
    pub async fn add_movie_from_provider(&self, options: AddMovieOptions) -> Result<Movie> {
        debug!(
            "Adding movie from {:?} (ID: {}) to library",
            options.provider, options.provider_id
        );

        // Only TMDB is supported for movies
        if options.provider != MetadataProvider::Tmdb {
            anyhow::bail!("Only TMDB is supported for movie metadata");
        }

        // Check if movie already exists in this library
        let auth_user = AuthUser {
            user_id: options.user_id.to_string(),
            email: None,
            role: None,
        };
        let data = self
            .execute_query(
                &auth_user,
                r#"query MovieByTmdb($Where: MovieWhereInput, $Page: PageInput) {
                    Movies(Where: $Where, Page: $Page) {
                        Edges { Node { Id } }
                    }
                }"#,
                serde_json::json!({
                    "Where": {
                        "LibraryId": { "Eq": options.library_id.to_string() },
                        "TmdbId": { "Eq": options.provider_id as i32 }
                    },
                    "Page": { "Limit": 1, "Offset": 0 }
                }),
            )
            .await?;
        let existing_id = data
            .get("Movies")
            .and_then(|m| m.get("Edges"))
            .and_then(|e| e.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| node.get("Id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string());

        if let Some(movie_id_str) = existing_id {
            let existing_movie = Movie::get(&self.db, &movie_id_str)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Movie not found after query"))?;
            debug!("Movie '{}' already exists in library", existing_movie.title);
            return Ok(existing_movie);
        }

        // Get movie details from TMDB
        let movie_details = self.get_movie(options.provider_id).await?;

        let movie_id = Uuid::new_v4().to_string();
        let ts = Self::now_iso_string();
        let data = self
            .execute_mutation(
                &auth_user,
                r#"mutation CreateMovie($Input: CreateMovieInput!) {
                    CreateMovie(Input: $Input) { Success Error }
                }"#,
                serde_json::json!({
                    "Input": {
                        "Id": movie_id,
                        "LibraryId": options.library_id.to_string(),
                        "UserId": options.user_id.to_string(),
                        "Title": movie_details.title,
                        "SortTitle": movie_details.original_title.clone().unwrap_or_else(|| movie_details.title.clone()),
                        "OriginalTitle": movie_details.original_title,
                        "Year": movie_details.year,
                        "TmdbId": movie_details.provider_id as i32,
                        "ImdbId": movie_details.imdb_id,
                        "Overview": movie_details.overview,
                        "Tagline": movie_details.tagline,
                        "Runtime": movie_details.runtime,
                        "Genres": movie_details.genres,
                        "Director": movie_details.director,
                        "CastNames": movie_details.cast_names,
                        "ProductionCountries": movie_details.production_countries,
                        "SpokenLanguages": movie_details.spoken_languages,
                        "TmdbRating": movie_details.vote_average.map(|v| v.to_string()),
                        "TmdbVoteCount": movie_details.vote_count,
                        "PosterUrl": movie_details.poster_url,
                        "BackdropUrl": movie_details.backdrop_url,
                        "CollectionId": movie_details.collection_id,
                        "CollectionName": movie_details.collection_name,
                        "CollectionPosterUrl": movie_details.collection_poster_url,
                        "ReleaseDate": movie_details.release_date,
                        "Certification": movie_details.certification,
                        "TmdbStatus": movie_details.tmdb_status,
                        "Monitored": options.monitored,
                        "Wanted": options.monitored,
                        "HasFile": false,
                        "CreatedAt": ts,
                        "UpdatedAt": ts
                    }
                }),
            )
            .await?;
        let created = data
            .get("CreateMovie")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !created {
            let err = data
                .get("CreateMovie")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to create movie");
            anyhow::bail!(err.to_string());
        }
        let movie = Movie::get(&self.db, &movie_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Movie not found after creation"))?;

        // Cache artwork in background (don't block the response)
        let artwork_service = crate::services::ArtworkService::new(self.db.clone());
        let movie_id = movie.id.clone();
        let poster_url = movie_details.poster_url.clone();
        let backdrop_url = movie_details.backdrop_url.clone();

        tokio::spawn(async move {
            artwork_service
                .cache_movie_artwork(&movie_id, poster_url.as_deref(), backdrop_url.as_deref())
                .await;
        });

        Ok(movie)
    }

    /// Add a TV show from TVMaze to a library
    pub async fn add_tv_show_from_provider(&self, options: AddTvShowOptions) -> Result<Show> {
        debug!(
            "Adding TV show from {:?} (ID: {}) to library",
            options.provider, options.provider_id
        );

        if options.provider != MetadataProvider::Tvmaze {
            anyhow::bail!("Only TVMaze is supported for TV show metadata");
        }

        let auth_user = AuthUser {
            user_id: options.user_id.to_string(),
            email: None,
            role: None,
        };

        let data = self
            .execute_query(
                &auth_user,
                r#"query ShowByTvmaze($Where: ShowWhereInput, $Page: PageInput) {
                    Shows(Where: $Where, Page: $Page) {
                        Edges { Node { Id } }
                    }
                }"#,
                serde_json::json!({
                    "Where": {
                        "LibraryId": { "Eq": options.library_id.to_string() },
                        "TvmazeId": { "Eq": options.provider_id as i32 }
                    },
                    "Page": { "Limit": 1, "Offset": 0 }
                }),
            )
            .await?;
        let existing_id = data
            .get("Shows")
            .and_then(|m| m.get("Edges"))
            .and_then(|e| e.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| node.get("Id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string());

        if let Some(show_id) = existing_id {
            let existing_show = Show::get(&self.db, &show_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Show not found after query"))?;
            debug!("Show '{}' already exists in library", existing_show.name);
            return Ok(existing_show);
        }

        let details = self.get_tv_show(options.provider_id).await?;

        let ts = Self::now_iso_string();
        let auto_download = options.monitor_type != AutoDownloadMode::None;
        let auto_download_mode = match options.monitor_type {
            AutoDownloadMode::None => "NONE",
            AutoDownloadMode::All => "ALL",
            AutoDownloadMode::Wanted => "WANTED",
        };
        let data = self
            .execute_mutation(
                &auth_user,
                r#"mutation CreateShow($Input: CreateShowInput!) {
                    CreateShow(Input: $Input) { Success Error Show { Id } }
                }"#,
                serde_json::json!({
                    "Input": {
                        "LibraryId": options.library_id.to_string(),
                        "UserId": options.user_id.to_string(),
                        "Name": details.name,
                        "SortName": details.sort_name,
                        "Year": details.year,
                        "TvmazeId": details.provider_id as i32,
                        "TvdbId": details.tvdb_id,
                        "ImdbId": details.imdb_id,
                        "Overview": details.overview,
                        "Network": details.network,
                        "Runtime": details.runtime,
                        "Genres": details.genres,
                        "PosterUrl": details.poster_url,
                        "BackdropUrl": details.backdrop_url,
                        "ContentRating": details.status,
                        "AutoDownload": auto_download,
                        "AutoDownloadMode": auto_download_mode,
                        "Path": options.path,
                        "CreatedAt": ts,
                        "UpdatedAt": ts
                    }
                }),
            )
            .await?;
        let created = data
            .get("CreateShow")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !created {
            let err = data
                .get("CreateShow")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to create show");
            anyhow::bail!(err.to_string());
        }

        let show_id = data
            .get("CreateShow")
            .and_then(|v| v.get("Show"))
            .and_then(|v| v.get("Id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Show ID missing after creation"))?
            .to_string();

        let show = Show::get(&self.db, &show_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Show not found after creation"))?;

        // Cache artwork in background
        let artwork_service = crate::services::ArtworkService::new(self.db.clone());
        let poster_url = details.poster_url.clone();
        let backdrop_url = details.backdrop_url.clone();
        tokio::spawn(async move {
            artwork_service
                .cache_show_artwork(&show_id, poster_url.as_deref(), backdrop_url.as_deref())
                .await;
        });

        Ok(show)
    }
}

/// Options for creating a movie from metadata
#[derive(Debug, Clone)]
pub struct CreateMovieFromMetadataOptions {
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub monitored: bool,
}

/// Create a sharable metadata service
pub fn create_metadata_service(
    db: Database,
    services: Arc<ServicesManager>,
    config: MetadataServiceConfig,
) -> Arc<MetadataService> {
    Arc::new(MetadataService::new(db, services, config))
}
