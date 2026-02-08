//! Unified metadata service for TMDB movie search
//!
//! Provides a simplified interface for searching movies on TMDB.
//! API key is loaded from app_settings database table.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::musicbrainz::MusicBrainzClient;
use super::settings::MetadataSettings;
use super::tmdb::TmdbClient;
use super::tvmaze::TvMazeClient;
use crate::db::Database;
use crate::services::graphql::entities::common::AutoDownloadMode;
use crate::services::graphql::entities::{Album, Artist, Audiobook, Movie, Show};
use crate::services::graphql::{AuthUser, LibrarianSchema};
use crate::services::manager::ServicesManager;
use async_graphql::{Request, Variables};

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

/// Unified album search result (from MusicBrainz search)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumSearchResult {
    pub provider: MetadataProvider,
    pub provider_id: String,
    pub title: String,
    pub artist_name: Option<String>,
    pub year: Option<i32>,
    pub album_type: Option<String>,
    pub cover_url: Option<String>,
    pub score: Option<f64>,
}

/// Unified audiobook search result (from OpenLibrary search)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudiobookSearchResult {
    pub provider: MetadataProvider,
    pub provider_id: String,
    pub title: String,
    pub author_name: Option<String>,
    pub year: Option<i32>,
    pub cover_url: Option<String>,
    pub isbn: Option<String>,
    pub description: Option<String>,
}

/// Unified audiobook details (from OpenLibrary work details)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudiobookDetails {
    pub provider: MetadataProvider,
    pub provider_id: String,
    pub title: String,
    pub sort_title: Option<String>,
    pub author_name: Option<String>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub language: Option<String>,
    pub isbn: Option<String>,
    pub published_date: Option<String>,
    pub cover_url: Option<String>,
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

/// Options for adding an album from a metadata provider
#[derive(Debug, Clone)]
pub struct AddAlbumOptions {
    pub provider: MetadataProvider,
    pub provider_id: String,
    pub library_id: Uuid,
    pub user_id: Uuid,
}

/// Options for adding an audiobook from a metadata provider
#[derive(Debug, Clone)]
pub struct AddAudiobookOptions {
    pub provider: MetadataProvider,
    pub provider_id: String,
    pub library_id: Uuid,
    pub user_id: Uuid,
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
    pub fn new(
        db: Database,
        services: Arc<ServicesManager>,
        config: MetadataServiceConfig,
    ) -> Self {
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

    /// Trigger a lightweight Movie update to emit subscription notifications after
    /// non-GraphQL side effects (e.g., artwork cache writes).
    async fn notify_movie_changed(&self, auth_user: &AuthUser, movie: &Movie) -> Result<()> {
        let _ = self
            .execute_mutation(
                auth_user,
                r#"mutation NotifyMovieChanged($Id: String!, $Input: UpdateMovieInput!) {
                    UpdateMovie(Id: $Id, Input: $Input) { Success Error }
                }"#,
                serde_json::json!({
                    "Id": movie.id,
                    "Input": {
                        "Title": movie.title
                    }
                }),
            )
            .await?;
        Ok(())
    }

    /// Trigger a lightweight Show update to emit subscription notifications after
    /// non-GraphQL side effects (e.g., artwork cache writes).
    async fn notify_show_changed(&self, auth_user: &AuthUser, show: &Show) -> Result<()> {
        let _ = self
            .execute_mutation(
                auth_user,
                r#"mutation NotifyShowChanged($Id: String!, $Input: UpdateShowInput!) {
                    UpdateShow(Id: $Id, Input: $Input) { Success Error }
                }"#,
                serde_json::json!({
                    "Id": show.id,
                    "Input": {
                        "Name": show.name
                    }
                }),
            )
            .await?;
        Ok(())
    }

    /// Trigger a lightweight Album update to emit subscription notifications after
    /// non-GraphQL side effects (e.g., artwork cache writes).
    async fn notify_album_changed(&self, auth_user: &AuthUser, album: &Album) -> Result<()> {
        let _ = self
            .execute_mutation(
                auth_user,
                r#"mutation NotifyAlbumChanged($Id: String!, $Input: UpdateAlbumInput!) {
                    UpdateAlbum(Id: $Id, Input: $Input) { Success Error }
                }"#,
                serde_json::json!({
                    "Id": album.id,
                    "Input": {
                        "Name": album.name
                    }
                }),
            )
            .await?;
        Ok(())
    }

    /// Trigger a lightweight Audiobook update to emit subscription notifications after
    /// non-GraphQL side effects (e.g., artwork cache writes).
    async fn notify_audiobook_changed(
        &self,
        auth_user: &AuthUser,
        audiobook: &Audiobook,
    ) -> Result<()> {
        let _ = self
            .execute_mutation(
                auth_user,
                r#"mutation NotifyAudiobookChanged($Id: String!, $Input: UpdateAudiobookInput!) {
                    UpdateAudiobook(Id: $Id, Input: $Input) { Success Error }
                }"#,
                serde_json::json!({
                    "Id": audiobook.id,
                    "Input": {
                        "Title": audiobook.title
                    }
                }),
            )
            .await?;
        Ok(())
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

        // Query only the TMDB key directly - don't load all settings
        // (MetadataSettings::load can fail if other settings have JSON parse issues,
        //  which would silently make this return false)
        match crate::services::torrent::database::get_setting_string(
            &self.db,
            "metadata.tmdb_api_key",
        )
        .await
        {
            Ok(key) => key.is_some(),
            Err(e) => {
                warn!(
                    error = %e,
                    "Failed to check TMDB API key from database key 'metadata.tmdb_api_key': error={}",
                    e
                );
                false
            }
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
        info!(
            "Creating TMDB client with API key loaded from app_settings key 'metadata.tmdb_api_key'"
        );
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
            "Metadata search requested for movie query='{}'{}",
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
        info!("Metadata search requested for TV shows query='{}'", query);

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
                    tvdb_id: show
                        .externals
                        .as_ref()
                        .and_then(|e| e.thetvdb)
                        .map(|v| v as i32),
                    imdb_id: show.externals.as_ref().and_then(|e| e.imdb.clone()),
                    score: Some(r.score),
                }
            })
            .collect();

        debug!(count = shows.len(), "Found TV shows");
        Ok(shows)
    }

    /// Search for albums on MusicBrainz
    pub async fn search_albums(
        &self,
        query: &str,
        include_eps: bool,
        include_singles: bool,
        include_compilations: bool,
        include_live: bool,
        include_soundtracks: bool,
    ) -> Result<Vec<AlbumSearchResult>> {
        let musicbrainz = self.get_musicbrainz_client().await?;

        let mut types = vec!["Album".to_string()];
        if include_eps {
            types.push("EP".to_string());
        }
        if include_singles {
            types.push("Single".to_string());
        }
        if include_compilations {
            types.push("Compilation".to_string());
        }
        if include_live {
            types.push("Live".to_string());
        }
        if include_soundtracks {
            types.push("Soundtrack".to_string());
        }

        let albums = musicbrainz.search_albums_with_types(query, &types).await?;

        Ok(albums
            .into_iter()
            .map(|album| {
                let provider_id = album.id.to_string();
                let title = album.title.clone();
                let artist_name = album.artist_names();
                let year = album.year();
                let album_type = Some(album.normalized_type());
                let score = album.score.map(|v| v as f64);
                // Use CAA thumbnail endpoint directly for search results.
                // It redirects when artwork exists and keeps search lightweight.
                let cover_url = Some(format!(
                    "https://coverartarchive.org/release-group/{}/front-250",
                    provider_id
                ));

                AlbumSearchResult {
                    provider: MetadataProvider::Musicbrainz,
                    provider_id,
                    title,
                    artist_name,
                    year,
                    album_type,
                    cover_url,
                    score,
                }
            })
            .collect())
    }

    /// Search for audiobooks on OpenLibrary
    pub async fn search_audiobooks(&self, query: &str) -> Result<Vec<AudiobookSearchResult>> {
        #[derive(Debug, Deserialize)]
        struct OpenLibraryDoc {
            key: Option<String>,
            title: Option<String>,
            author_name: Option<Vec<String>>,
            first_publish_year: Option<i32>,
            cover_i: Option<i64>,
            isbn: Option<Vec<String>>,
        }

        #[derive(Debug, Deserialize)]
        struct OpenLibrarySearchResponse {
            docs: Vec<OpenLibraryDoc>,
        }

        let client = reqwest::Client::new();
        let response = client
            .get("https://openlibrary.org/search.json")
            .query(&[("q", query)])
            .send()
            .await?
            .error_for_status()?
            .json::<OpenLibrarySearchResponse>()
            .await?;

        Ok(response
            .docs
            .into_iter()
            .filter_map(|doc| {
                let key = doc.key?;
                let provider_id = key
                    .strip_prefix("/works/")
                    .unwrap_or(key.as_str())
                    .to_string();
                let title = doc.title?;
                let cover_url = doc
                    .cover_i
                    .map(|id| format!("https://covers.openlibrary.org/b/id/{}-L.jpg", id));
                let author_name = doc.author_name.and_then(|v| v.first().cloned());
                let isbn = doc.isbn.and_then(|v| v.first().cloned());

                Some(AudiobookSearchResult {
                    provider: MetadataProvider::OpenLibrary,
                    provider_id,
                    title,
                    author_name,
                    year: doc.first_publish_year,
                    cover_url,
                    isbn,
                    description: None,
                })
            })
            .collect())
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
            year: show
                .premiered
                .as_ref()
                .and_then(|d| d.get(0..4))
                .and_then(|y| y.parse().ok()),
            status: show.status.clone(),
            overview: show.clean_summary(),
            network,
            runtime: show.average_runtime.or(show.runtime).map(|v| v as i32),
            genres: show.genres.clone(),
            poster_url: show.image.as_ref().and_then(|i| i.original.clone()),
            backdrop_url: None,
            tvdb_id: show
                .externals
                .as_ref()
                .and_then(|e| e.thetvdb)
                .map(|v| v as i32),
            imdb_id: show.externals.as_ref().and_then(|e| e.imdb.clone()),
        })
    }

    /// Get audiobook details from OpenLibrary work API
    pub async fn get_audiobook(&self, openlibrary_id: &str) -> Result<AudiobookDetails> {
        #[derive(Debug, Deserialize)]
        struct OpenLibraryWorkAuthor {
            author: OpenLibraryAuthorRef,
        }

        #[derive(Debug, Deserialize)]
        struct OpenLibraryAuthorRef {
            key: String,
        }

        #[derive(Debug, Deserialize)]
        struct OpenLibraryDescriptionObject {
            value: String,
        }

        #[derive(Debug, Deserialize)]
        #[serde(untagged)]
        enum OpenLibraryDescription {
            Text(String),
            Object(OpenLibraryDescriptionObject),
        }

        #[derive(Debug, Deserialize)]
        struct OpenLibraryWork {
            title: Option<String>,
            description: Option<OpenLibraryDescription>,
            covers: Option<Vec<i64>>,
            authors: Option<Vec<OpenLibraryWorkAuthor>>,
            first_publish_date: Option<String>,
            languages: Option<Vec<serde_json::Value>>,
        }

        #[derive(Debug, Deserialize)]
        struct OpenLibraryAuthor {
            name: Option<String>,
        }

        let client = reqwest::Client::new();
        let work_url = format!("https://openlibrary.org/works/{}.json", openlibrary_id);
        let work = client
            .get(work_url)
            .send()
            .await?
            .error_for_status()?
            .json::<OpenLibraryWork>()
            .await?;

        let description = match work.description {
            Some(OpenLibraryDescription::Text(text)) => Some(text),
            Some(OpenLibraryDescription::Object(obj)) => Some(obj.value),
            None => None,
        };

        let cover_url = work
            .covers
            .and_then(|covers| covers.first().copied())
            .map(|id| format!("https://covers.openlibrary.org/b/id/{}-L.jpg", id));

        let author_name = if let Some(authors) = work.authors {
            if let Some(author_ref) = authors.first() {
                let author_url = format!("https://openlibrary.org{}.json", author_ref.author.key);
                client
                    .get(author_url)
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<OpenLibraryAuthor>()
                    .await?
                    .name
            } else {
                None
            }
        } else {
            None
        };

        Ok(AudiobookDetails {
            provider: MetadataProvider::OpenLibrary,
            provider_id: openlibrary_id.to_string(),
            title: work
                .title
                .unwrap_or_else(|| "Unknown Audiobook".to_string()),
            sort_title: None,
            author_name,
            description,
            publisher: None,
            language: work
                .languages
                .and_then(|langs| langs.first().cloned())
                .and_then(|v| v.get("key").and_then(|k| k.as_str()).map(|s| s.to_string())),
            isbn: None,
            published_date: work.first_publish_date,
            cover_url,
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

        let data = self
            .execute_mutation(
                &auth_user,
                r#"mutation CreateMovie($Input: CreateMovieInput!) {
                    CreateMovie(Input: $Input) {
                        Success
                        Movie { Id }
                        Error
                    }
                }"#,
                serde_json::json!({
                    "Input": {
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
                        "CollectionId": movie_details.collection_id,
                        "CollectionName": movie_details.collection_name,
                        "CollectionPosterUrl": movie_details.collection_poster_url,
                        "ReleaseDate": movie_details.release_date,
                        "Certification": movie_details.certification,
                        "TmdbStatus": movie_details.tmdb_status,
                        "Monitored": options.monitored,
                        "Wanted": options.monitored,
                        "HasFile": false
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
        let created_id = data
            .get("CreateMovie")
            .and_then(|v| v.get("Movie"))
            .and_then(|m| m.get("Id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Movie not found after creation"))?;

        let movie = Movie::get(&self.db, &created_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Movie not found after creation"))?;

        // Cache artwork, then emit a lightweight entity update so library routes
        // subscribed to change events refresh when artwork becomes available.
        let artwork_service = crate::services::ArtworkService::new(self.db.clone());
        let _ = artwork_service
            .cache_movie_artwork(
                &movie.id,
                movie_details.poster_url.as_deref(),
                movie_details.backdrop_url.as_deref(),
            )
            .await;
        if let Err(e) = self.notify_movie_changed(&auth_user, &movie).await {
            warn!(movie_id = %movie.id, error = %e, "Failed to notify movie change after artwork cache");
        }

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
            let wanted_default = options.monitor_type != AutoDownloadMode::None;
            // Ensure episodes are backfilled even when show already exists.
            self.sync_show_episodes_from_tvmaze(
                &auth_user,
                &existing_show.id,
                options.provider_id,
                wanted_default,
            )
            .await?;
            debug!("Show '{}' already exists in library", existing_show.name);
            return Ok(existing_show);
        }

        let details = self.get_tv_show(options.provider_id).await?;

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
                        "Path": options.path
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

        // Populate episodes through generated mutations so subscription notifications fire.
        self.sync_show_episodes_from_tvmaze(
            &auth_user,
            &show.id,
            options.provider_id,
            auto_download,
        )
        .await?;

        // Cache artwork, then emit a lightweight entity update so library routes
        // subscribed to change events refresh when artwork becomes available.
        let artwork_service = crate::services::ArtworkService::new(self.db.clone());
        let _ = artwork_service
            .cache_show_artwork(
                &show.id,
                details.poster_url.as_deref(),
                details.backdrop_url.as_deref(),
            )
            .await;
        if let Err(e) = self.notify_show_changed(&auth_user, &show).await {
            warn!(show_id = %show.id, error = %e, "Failed to notify show change after artwork cache");
        }

        Ok(show)
    }

    /// Sync episodes for a show from TVMaze using generated Episode mutations.
    async fn sync_show_episodes_from_tvmaze(
        &self,
        auth_user: &AuthUser,
        show_id: &str,
        tvmaze_id: u32,
        wanted_default: bool,
    ) -> Result<()> {
        let tvmaze = self.get_tvmaze_client().await?;
        let episodes = tvmaze.get_episodes(tvmaze_id).await?;

        for ep in episodes {
            let existing = self
                .execute_query(
                    auth_user,
                    r#"query EpisodeByTvmaze($Where: EpisodeWhereInput, $Page: PageInput) {
                        Episodes(Where: $Where, Page: $Page) {
                            Edges { Node { Id Wanted } }
                        }
                    }"#,
                    serde_json::json!({
                        "Where": {
                            "ShowId": { "Eq": show_id },
                            "TvmazeId": { "Eq": ep.id as i32 }
                        },
                        "Page": { "Limit": 1, "Offset": 0 }
                    }),
                )
                .await?;

            let existing_edge = existing
                .get("Episodes")
                .and_then(|v| v.get("Edges"))
                .and_then(|v| v.as_array())
                .and_then(|edges| edges.first())
                .and_then(|edge| edge.get("Node"));

            let title = Some(ep.name.clone());
            let overview = ep.clean_summary();
            let season = ep.season as i32;
            let episode_number = ep.number as i32;
            let runtime = ep.runtime.map(|r| r as i32);
            let tvmaze_ep_id = Some(ep.id as i32);

            if let Some(node) = existing_edge {
                let existing_id = node
                    .get("Id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Episode id missing in existing lookup"))?;
                let existing_wanted = node
                    .get("Wanted")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(wanted_default);

                let updated = self
                    .execute_mutation(
                        auth_user,
                        r#"mutation UpdateEpisodeFromTvmaze($Id: String!, $Input: UpdateEpisodeInput!) {
                            UpdateEpisode(Id: $Id, Input: $Input) { Success Error }
                        }"#,
                        serde_json::json!({
                            "Id": existing_id,
                            "Input": {
                                "ShowId": show_id,
                                "Season": season,
                                "Episode": episode_number,
                                "Title": title,
                                "Overview": overview,
                                "AirDate": ep.airdate,
                                "Runtime": runtime,
                                "TvmazeId": tvmaze_ep_id,
                                "Wanted": existing_wanted
                            }
                        }),
                    )
                    .await?;

                let ok = updated
                    .get("UpdateEpisode")
                    .and_then(|v| v.get("Success"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if !ok {
                    let err = updated
                        .get("UpdateEpisode")
                        .and_then(|v| v.get("Error"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Failed to update episode");
                    anyhow::bail!(err.to_string());
                }
            } else {
                let created = self
                    .execute_mutation(
                        auth_user,
                        r#"mutation CreateEpisodeFromTvmaze($Input: CreateEpisodeInput!) {
                            CreateEpisode(Input: $Input) { Success Error }
                        }"#,
                        serde_json::json!({
                            "Input": {
                                "ShowId": show_id,
                                "Season": season,
                                "Episode": episode_number,
                                "Title": title,
                                "Overview": overview,
                                "AirDate": ep.airdate,
                                "Runtime": runtime,
                                "TvmazeId": tvmaze_ep_id,
                                "Wanted": wanted_default
                            }
                        }),
                    )
                    .await?;

                let ok = created
                    .get("CreateEpisode")
                    .and_then(|v| v.get("Success"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if !ok {
                    let err = created
                        .get("CreateEpisode")
                        .and_then(|v| v.get("Error"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Failed to create episode");
                    anyhow::bail!(err.to_string());
                }
            }
        }

        Ok(())
    }

    /// Add an album from MusicBrainz to a library.
    pub async fn add_album_from_provider(&self, options: AddAlbumOptions) -> Result<Album> {
        if options.provider != MetadataProvider::Musicbrainz {
            anyhow::bail!("Only MusicBrainz is supported for album metadata");
        }

        let provider_id = options.provider_id.clone();
        let release_group_id = Uuid::parse_str(&options.provider_id)
            .map_err(|e| anyhow::anyhow!("Invalid MusicBrainz ID: {}", e))?;
        let musicbrainz = self.get_musicbrainz_client().await?;
        let release_group = musicbrainz.get_release_group(release_group_id).await?;

        let auth_user = AuthUser {
            user_id: options.user_id.to_string(),
            email: None,
            role: None,
        };

        let existing = self
            .execute_query(
                &auth_user,
                r#"query AlbumByMusicbrainz($Where: AlbumWhereInput, $Page: PageInput) {
                    Albums(Where: $Where, Page: $Page) {
                        Edges { Node { Id } }
                    }
                }"#,
                serde_json::json!({
                    "Where": {
                        "LibraryId": { "Eq": options.library_id.to_string() },
                        "MusicbrainzId": { "Eq": provider_id.clone() }
                    },
                    "Page": { "Limit": 1, "Offset": 0 }
                }),
            )
            .await?;

        if let Some(existing_id) = existing
            .get("Albums")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| node.get("Id"))
            .and_then(|id| id.as_str())
        {
            return Album::get(&self.db, existing_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Album not found after lookup"));
        }

        let artist_name = release_group
            .artist_names()
            .unwrap_or_else(|| "Unknown Artist".to_string());
        let artist_mbid = release_group
            .artist_credit
            .as_ref()
            .and_then(|credits| credits.first())
            .map(|credit| credit.artist.id.to_string());

        let existing_artist = self
            .execute_query(
                &auth_user,
                r#"query ArtistLookup($Where: ArtistWhereInput, $Page: PageInput) {
                    Artists(Where: $Where, Page: $Page) {
                        Edges { Node { Id } }
                    }
                }"#,
                serde_json::json!({
                    "Where": {
                        "LibraryId": { "Eq": options.library_id.to_string() },
                        "Name": { "Eq": artist_name.clone() }
                    },
                    "Page": { "Limit": 1, "Offset": 0 }
                }),
            )
            .await?;

        let artist_id = if let Some(id) = existing_artist
            .get("Artists")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| node.get("Id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string())
        {
            id
        } else {
            let created = self
                .execute_mutation(
                    &auth_user,
                    r#"mutation CreateArtist($Input: CreateArtistInput!) {
                        CreateArtist(Input: $Input) { Success Error Artist { Id } }
                    }"#,
                    serde_json::json!({
                        "Input": {
                            "LibraryId": options.library_id.to_string(),
                            "UserId": options.user_id.to_string(),
                            "Name": artist_name.clone(),
                            "SortName": artist_name.clone(),
                            "MusicbrainzId": artist_mbid
                        }
                    }),
                )
                .await?;

            let success = created
                .get("CreateArtist")
                .and_then(|v| v.get("Success"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !success {
                let err = created
                    .get("CreateArtist")
                    .and_then(|v| v.get("Error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Failed to create artist");
                anyhow::bail!(err.to_string());
            }

            created
                .get("CreateArtist")
                .and_then(|v| v.get("Artist"))
                .and_then(|v| v.get("Id"))
                .and_then(|id| id.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow::anyhow!("Artist not found after creation"))?
        };

        let cover_url = musicbrainz
            .get_cover_art(release_group_id)
            .await
            .ok()
            .flatten();
        let created_album = self
            .execute_mutation(
                &auth_user,
                r#"mutation CreateAlbum($Input: CreateAlbumInput!) {
                    CreateAlbum(Input: $Input) { Success Error Album { Id } }
                }"#,
                serde_json::json!({
                    "Input": {
                        "ArtistId": artist_id,
                        "LibraryId": options.library_id.to_string(),
                        "UserId": options.user_id.to_string(),
                        "Name": release_group.title,
                        "SortName": release_group.title,
                        "Year": release_group.year(),
                        "MusicbrainzId": provider_id.clone(),
                        "AlbumType": release_group.normalized_type(),
                        "Genres": Vec::<String>::new(),
                        "AutoDownload": false,
                        "AutoDownloadMode": "NONE",
                        "HasFiles": false,
                        "CoverUrl": cover_url.clone()
                    }
                }),
            )
            .await?;

        let success = created_album
            .get("CreateAlbum")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !success {
            let err = created_album
                .get("CreateAlbum")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to create album");
            anyhow::bail!(err.to_string());
        }

        let created_album_id = created_album
            .get("CreateAlbum")
            .and_then(|v| v.get("Album"))
            .and_then(|v| v.get("Id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| anyhow::anyhow!("Album not found after creation"))?;

        let album = Album::get(&self.db, created_album_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Album not found after creation"))?;

        // Cache artwork, then emit a lightweight entity update so library/media
        // subscribers refresh once cached artwork is ready.
        let artwork_service = crate::services::ArtworkService::new(self.db.clone());
        let _ = artwork_service
            .cache_album_artwork(&album.id, cover_url.as_deref())
            .await;
        if let Err(e) = self.notify_album_changed(&auth_user, &album).await {
            warn!(album_id = %album.id, error = %e, "Failed to notify album change after artwork cache");
        }

        Ok(album)
    }

    /// Add an audiobook from OpenLibrary to a library.
    pub async fn add_audiobook_from_provider(
        &self,
        options: AddAudiobookOptions,
    ) -> Result<Audiobook> {
        if options.provider != MetadataProvider::OpenLibrary {
            anyhow::bail!("Only OpenLibrary is supported for audiobook metadata");
        }

        let details = self.get_audiobook(&options.provider_id).await?;
        let auth_user = AuthUser {
            user_id: options.user_id.to_string(),
            email: None,
            role: None,
        };

        let existing = self
            .execute_query(
                &auth_user,
                r#"query AudiobookByProvider($Where: AudiobookWhereInput, $Page: PageInput) {
                    Audiobooks(Where: $Where, Page: $Page) {
                        Edges { Node { Id } }
                    }
                }"#,
                serde_json::json!({
                    "Where": {
                        "LibraryId": { "Eq": options.library_id.to_string() },
                        "AudibleId": { "Eq": options.provider_id.clone() }
                    },
                    "Page": { "Limit": 1, "Offset": 0 }
                }),
            )
            .await?;

        if let Some(existing_id) = existing
            .get("Audiobooks")
            .and_then(|v| v.get("Edges"))
            .and_then(|v| v.as_array())
            .and_then(|edges| edges.first())
            .and_then(|edge| edge.get("Node"))
            .and_then(|node| node.get("Id"))
            .and_then(|id| id.as_str())
        {
            return Audiobook::get(&self.db, existing_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Audiobook not found after lookup"));
        }

        let created = self
            .execute_mutation(
                &auth_user,
                r#"mutation CreateAudiobook($Input: CreateAudiobookInput!) {
                    CreateAudiobook(Input: $Input) { Success Error Audiobook { Id } }
                }"#,
                serde_json::json!({
                    "Input": {
                        "LibraryId": options.library_id.to_string(),
                        "UserId": options.user_id.to_string(),
                        "Title": details.title,
                        "SortTitle": details.sort_title,
                        "AuthorName": details.author_name,
                        "Description": details.description,
                        "Publisher": details.publisher,
                        "Language": details.language,
                        "Isbn": details.isbn,
                        "PublishedDate": details.published_date,
                        "CoverUrl": details.cover_url,
                        "AudibleId": details.provider_id,
                        "AutoDownload": false,
                        "AutoDownloadMode": "NONE",
                        "HasFiles": false,
                        "Narrators": Vec::<String>::new()
                    }
                }),
            )
            .await?;

        let success = created
            .get("CreateAudiobook")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !success {
            let err = created
                .get("CreateAudiobook")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to create audiobook");
            anyhow::bail!(err.to_string());
        }

        let created_audiobook_id = created
            .get("CreateAudiobook")
            .and_then(|v| v.get("Audiobook"))
            .and_then(|v| v.get("Id"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| anyhow::anyhow!("Audiobook ID missing after creation"))?;

        let audiobook = Audiobook::get(&self.db, created_audiobook_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Audiobook not found after creation"))?;

        // Cache artwork, then emit a lightweight entity update so library/media
        // subscribers refresh once cached artwork is ready.
        let artwork_service = crate::services::ArtworkService::new(self.db.clone());
        let _ = artwork_service
            .cache_audiobook_artwork(&audiobook.id, details.cover_url.as_deref())
            .await;
        if let Err(e) = self.notify_audiobook_changed(&auth_user, &audiobook).await {
            warn!(audiobook_id = %audiobook.id, error = %e, "Failed to notify audiobook change after artwork cache");
        }

        Ok(audiobook)
    }

    /// Refresh movie metadata from provider and persist via generated GraphQL UpdateMovie mutation.
    pub async fn refresh_movie_from_provider(
        &self,
        movie_id: &str,
        user_id: Uuid,
    ) -> Result<Movie> {
        let movie = Movie::get(&self.db, movie_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Movie not found"))?;

        if movie.user_id != user_id.to_string() {
            anyhow::bail!("Movie not found");
        }

        let tmdb_id = movie
            .tmdb_id
            .ok_or_else(|| anyhow::anyhow!("Movie has no TmdbId"))?;

        let details = self.get_movie(tmdb_id as u32).await?;

        let auth_user = AuthUser {
            user_id: user_id.to_string(),
            email: None,
            role: None,
        };

        let data = self
            .execute_mutation(
                &auth_user,
                r#"mutation RefreshMovie($Id: String!, $Input: UpdateMovieInput!) {
                    UpdateMovie(Id: $Id, Input: $Input) { Success Error }
                }"#,
                serde_json::json!({
                    "Id": movie.id,
                    "Input": {
                        "Title": details.title,
                        "SortTitle": details.original_title.clone().unwrap_or_else(|| details.title.clone()),
                        "OriginalTitle": details.original_title,
                        "Year": details.year,
                        "TmdbId": details.provider_id as i32,
                        "ImdbId": details.imdb_id,
                        "Overview": details.overview,
                        "Tagline": details.tagline,
                        "Runtime": details.runtime,
                        "Genres": details.genres,
                        "Director": details.director,
                        "CastNames": details.cast_names,
                        "ProductionCountries": details.production_countries,
                        "SpokenLanguages": details.spoken_languages,
                        "TmdbRating": details.vote_average.map(|v| v.to_string()),
                        "TmdbVoteCount": details.vote_count,
                        "ReleaseDate": details.release_date,
                        "Certification": details.certification,
                        "CollectionId": details.collection_id,
                        "CollectionName": details.collection_name,
                        "CollectionPosterUrl": details.collection_poster_url,
                        "Status": details.tmdb_status,
                    }
                }),
            )
            .await?;

        let updated = data
            .get("UpdateMovie")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !updated {
            let err = data
                .get("UpdateMovie")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to refresh movie");
            anyhow::bail!(err.to_string());
        }

        let movie = Movie::get(&self.db, movie_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Movie not found after refresh"))?;

        // Refresh cached artwork, then emit a lightweight entity update so
        // subscribers refresh once cached artwork is ready.
        let artwork_service = crate::services::ArtworkService::new(self.db.clone());
        let _ = artwork_service
            .cache_movie_artwork(
                &movie.id,
                details.poster_url.as_deref(),
                details.backdrop_url.as_deref(),
            )
            .await;
        if let Err(e) = self.notify_movie_changed(&auth_user, &movie).await {
            warn!(movie_id = %movie.id, error = %e, "Failed to notify movie change after artwork refresh");
        }

        Ok(movie)
    }

    /// Refresh TV show metadata from provider and persist via generated GraphQL UpdateShow mutation.
    pub async fn refresh_tv_show_from_provider(
        &self,
        show_id: &str,
        user_id: Uuid,
    ) -> Result<Show> {
        let show = Show::get(&self.db, show_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Show not found"))?;

        if show.user_id != user_id.to_string() {
            anyhow::bail!("Show not found");
        }

        let tvmaze_id = show
            .tvmaze_id
            .ok_or_else(|| anyhow::anyhow!("Show has no TvmazeId"))?;

        let details = self.get_tv_show(tvmaze_id as u32).await?;

        let auth_user = AuthUser {
            user_id: user_id.to_string(),
            email: None,
            role: None,
        };

        let data = self
            .execute_mutation(
                &auth_user,
                r#"mutation RefreshShow($Id: String!, $Input: UpdateShowInput!) {
                    UpdateShow(Id: $Id, Input: $Input) { Success Error }
                }"#,
                serde_json::json!({
                    "Id": show.id,
                    "Input": {
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
                    }
                }),
            )
            .await?;

        let updated = data
            .get("UpdateShow")
            .and_then(|v| v.get("Success"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !updated {
            let err = data
                .get("UpdateShow")
                .and_then(|v| v.get("Error"))
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to refresh show");
            anyhow::bail!(err.to_string());
        }

        let show = Show::get(&self.db, show_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Show not found after refresh"))?;

        // Keep episodes in sync with provider metadata via generated mutations.
        self.sync_show_episodes_from_tvmaze(
            &auth_user,
            &show.id,
            tvmaze_id as u32,
            show.auto_download,
        )
        .await?;

        // Refresh cached artwork, then emit a lightweight entity update so
        // subscribers refresh once cached artwork is ready.
        let artwork_service = crate::services::ArtworkService::new(self.db.clone());
        let _ = artwork_service
            .cache_show_artwork(
                &show.id,
                details.poster_url.as_deref(),
                details.backdrop_url.as_deref(),
            )
            .await;
        if let Err(e) = self.notify_show_changed(&auth_user, &show).await {
            warn!(show_id = %show.id, error = %e, "Failed to notify show change after artwork refresh");
        }

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
