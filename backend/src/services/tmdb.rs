//! TMDB (The Movie Database) API client for movie metadata
//!
//! TMDB is a popular movie/TV database with a free API.
//! Base URL: https://api.themoviedb.org/3
//!
//! Rate limiting: TMDB allows ~40 requests per 10 seconds.
//! This client uses rate limiting and retry logic to handle this gracefully.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::rate_limiter::{RateLimitedClient, RetryConfig, retry_async};

/// TMDB API client with rate limiting and retry logic
pub struct TmdbClient {
    client: Arc<RateLimitedClient>,
    base_url: String,
    api_key: String,
    retry_config: RetryConfig,
}

/// Movie search result from TMDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbMovieSearchResult {
    pub page: i32,
    pub results: Vec<TmdbMovie>,
    pub total_pages: i32,
    pub total_results: i32,
}

/// Movie details from TMDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbMovie {
    pub id: i32,
    pub title: String,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub release_date: Option<String>,
    pub runtime: Option<i32>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub adult: bool,
    pub video: bool,
    pub vote_average: Option<f64>,
    pub vote_count: Option<i32>,
    pub popularity: Option<f64>,
    pub original_language: Option<String>,
    pub genre_ids: Option<Vec<i32>>,
    pub genres: Option<Vec<TmdbGenre>>,
    /// Collection info (only in movie details, not search)
    pub belongs_to_collection: Option<TmdbCollectionInfo>,
    /// Production countries (only in movie details)
    pub production_countries: Option<Vec<TmdbProductionCountry>>,
    /// Spoken languages (only in movie details)
    pub spoken_languages: Option<Vec<TmdbSpokenLanguage>>,
    /// IMDB ID (only in movie details)
    pub imdb_id: Option<String>,
    /// Status (only in movie details)
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbGenre {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbCollectionInfo {
    pub id: i32,
    pub name: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbProductionCountry {
    pub iso_3166_1: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbSpokenLanguage {
    pub iso_639_1: String,
    pub name: String,
    pub english_name: Option<String>,
}

/// Movie credits from TMDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbCredits {
    pub id: i32,
    pub cast: Vec<TmdbCastMember>,
    pub crew: Vec<TmdbCrewMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbCastMember {
    pub id: i32,
    pub name: String,
    pub character: Option<String>,
    pub order: Option<i32>,
    pub profile_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbCrewMember {
    pub id: i32,
    pub name: String,
    pub job: String,
    pub department: String,
    pub profile_path: Option<String>,
}

/// Collection details from TMDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbCollection {
    pub id: i32,
    pub name: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub parts: Vec<TmdbMovie>,
}

/// Release dates from TMDB (for certification)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbReleaseDates {
    pub id: i32,
    pub results: Vec<TmdbReleaseDateResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbReleaseDateResult {
    pub iso_3166_1: String,
    pub release_dates: Vec<TmdbReleaseDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbReleaseDate {
    pub certification: Option<String>,
    pub release_date: Option<String>,
    #[serde(rename = "type")]
    pub release_type: Option<i32>,
}

/// Configuration for TMDB image URLs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbConfiguration {
    pub images: TmdbImagesConfiguration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbImagesConfiguration {
    pub base_url: String,
    pub secure_base_url: String,
    pub poster_sizes: Vec<String>,
    pub backdrop_sizes: Vec<String>,
    pub profile_sizes: Vec<String>,
}

impl TmdbClient {
    /// Create a new TMDB client with the given API key
    pub fn new(api_key: String) -> Self {
        Self {
            // TMDB allows ~40 requests per 10 seconds, so ~4/sec with burst of 10
            client: Arc::new(RateLimitedClient::new(
                "tmdb",
                super::rate_limiter::RateLimitConfig {
                    requests_per_second: 4,
                    burst_size: 10,
                },
            )),
            base_url: "https://api.themoviedb.org/3".to_string(),
            api_key,
            retry_config: RetryConfig {
                max_retries: 3,
                initial_interval: Duration::from_millis(500),
                max_interval: Duration::from_secs(10),
                multiplier: 2.0,
            },
        }
    }

    /// Check if the client has a valid API key configured
    pub fn has_api_key(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// Get the image base URL for poster/backdrop images
    pub fn image_url(&self, path: &str, size: &str) -> String {
        format!("https://image.tmdb.org/t/p/{}{}", size, path)
    }

    /// Get full poster URL (w500 size - good for display)
    pub fn poster_url(&self, path: Option<&str>) -> Option<String> {
        path.map(|p| self.image_url(p, "w500"))
    }

    /// Get full backdrop URL (w1280 size - good for backgrounds)
    pub fn backdrop_url(&self, path: Option<&str>) -> Option<String> {
        path.map(|p| self.image_url(p, "w1280"))
    }

    /// Get original size image URL (for caching)
    pub fn original_url(&self, path: Option<&str>) -> Option<String> {
        path.map(|p| self.image_url(p, "original"))
    }

    /// Search for movies by name
    pub async fn search_movies(&self, query: &str, year: Option<i32>) -> Result<Vec<TmdbMovie>> {
        if !self.has_api_key() {
            anyhow::bail!("TMDB API key not configured");
        }

        info!("Searching TMDB for movie '{}'{}",
            query,
            year.map(|y| format!(" ({})", y)).unwrap_or_default()
        );

        let url = format!("{}/search/movie", self.base_url);
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let query_owned = query.to_string();
        let retry_config = self.retry_config.clone();

        let result = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let q = query_owned.clone();
                let key = api_key.clone();
                async move {
                    let mut query_params: Vec<(&str, String)> = vec![
                        ("api_key", key),
                        ("query", q),
                        ("include_adult", "false".to_string()),
                    ];
                    if let Some(y) = year {
                        query_params.push(("year", y.to_string()));
                    }

                    let response = client.get_with_query(&url, &query_params).await?;

                    if response.status().as_u16() == 429 {
                        warn!("TMDB rate limit hit, will retry");
                        anyhow::bail!("Rate limited (429)");
                    }

                    if response.status().as_u16() == 401 {
                        anyhow::bail!("TMDB API key is invalid");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("TMDB search failed with status: {}", response.status());
                    }

                    let results: TmdbMovieSearchResult = response
                        .json()
                        .await
                        .context("Failed to parse TMDB search results")?;

                    Ok(results.results)
                }
            },
            &retry_config,
            "tmdb_search_movies",
        )
        .await?;

        debug!(count = result.len(), "TMDB search returned results");
        Ok(result)
    }

    /// Get movie details by TMDB ID
    pub async fn get_movie(&self, tmdb_id: i32) -> Result<TmdbMovie> {
        if !self.has_api_key() {
            anyhow::bail!("TMDB API key not configured");
        }

        debug!("Fetching movie details from TMDB (ID: {})", tmdb_id);

        let url = format!("{}/movie/{}", self.base_url, tmdb_id);
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let retry_config = self.retry_config.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let key = api_key.clone();
                async move {
                    let response = client
                        .get_with_query(&url, &[("api_key", &key)])
                        .await?;

                    if response.status().as_u16() == 429 {
                        warn!("TMDB rate limit hit, will retry");
                        anyhow::bail!("Rate limited (429)");
                    }

                    if response.status().as_u16() == 404 {
                        anyhow::bail!("Movie not found on TMDB");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("TMDB get movie failed with status: {}", response.status());
                    }

                    let movie: TmdbMovie = response
                        .json()
                        .await
                        .context("Failed to parse TMDB movie")?;

                    Ok(movie)
                }
            },
            &retry_config,
            "tmdb_get_movie",
        )
        .await
    }

    /// Get movie credits (cast and crew)
    pub async fn get_credits(&self, tmdb_id: i32) -> Result<TmdbCredits> {
        if !self.has_api_key() {
            anyhow::bail!("TMDB API key not configured");
        }

        let url = format!("{}/movie/{}/credits", self.base_url, tmdb_id);
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let retry_config = self.retry_config.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let key = api_key.clone();
                async move {
                    let response = client
                        .get_with_query(&url, &[("api_key", &key)])
                        .await?;

                    if response.status().as_u16() == 429 {
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("TMDB get credits failed with status: {}", response.status());
                    }

                    let credits: TmdbCredits = response
                        .json()
                        .await
                        .context("Failed to parse TMDB credits")?;

                    Ok(credits)
                }
            },
            &retry_config,
            "tmdb_get_credits",
        )
        .await
    }

    /// Get release dates (for certification/rating)
    pub async fn get_release_dates(&self, tmdb_id: i32) -> Result<TmdbReleaseDates> {
        if !self.has_api_key() {
            anyhow::bail!("TMDB API key not configured");
        }

        let url = format!("{}/movie/{}/release_dates", self.base_url, tmdb_id);
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let retry_config = self.retry_config.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let key = api_key.clone();
                async move {
                    let response = client
                        .get_with_query(&url, &[("api_key", &key)])
                        .await?;

                    if response.status().as_u16() == 429 {
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TMDB get release dates failed with status: {}",
                            response.status()
                        );
                    }

                    let dates: TmdbReleaseDates = response
                        .json()
                        .await
                        .context("Failed to parse TMDB release dates")?;

                    Ok(dates)
                }
            },
            &retry_config,
            "tmdb_get_release_dates",
        )
        .await
    }

    /// Get collection details
    pub async fn get_collection(&self, collection_id: i32) -> Result<TmdbCollection> {
        if !self.has_api_key() {
            anyhow::bail!("TMDB API key not configured");
        }

        debug!("Fetching collection from TMDB (ID: {})", collection_id);

        let url = format!("{}/collection/{}", self.base_url, collection_id);
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let retry_config = self.retry_config.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let key = api_key.clone();
                async move {
                    let response = client
                        .get_with_query(&url, &[("api_key", &key)])
                        .await?;

                    if response.status().as_u16() == 429 {
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TMDB get collection failed with status: {}",
                            response.status()
                        );
                    }

                    let collection: TmdbCollection = response
                        .json()
                        .await
                        .context("Failed to parse TMDB collection")?;

                    Ok(collection)
                }
            },
            &retry_config,
            "tmdb_get_collection",
        )
        .await
    }

    /// Find movie by external ID (IMDb)
    pub async fn find_by_imdb(&self, imdb_id: &str) -> Result<Option<TmdbMovie>> {
        if !self.has_api_key() {
            anyhow::bail!("TMDB API key not configured");
        }

        debug!("Looking up IMDb ID {} on TMDB", imdb_id);

        let url = format!("{}/find/{}", self.base_url, imdb_id);
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let retry_config = self.retry_config.clone();

        #[derive(Deserialize)]
        struct FindResult {
            movie_results: Vec<TmdbMovie>,
        }

        let result = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let key = api_key.clone();
                async move {
                    let response = client
                        .get_with_query(
                            &url,
                            &[("api_key", key), ("external_source", "imdb_id".to_string())],
                        )
                        .await?;

                    if response.status().as_u16() == 429 {
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("TMDB find failed with status: {}", response.status());
                    }

                    let find_result: FindResult = response
                        .json()
                        .await
                        .context("Failed to parse TMDB find result")?;

                    Ok(find_result.movie_results.into_iter().next())
                }
            },
            &retry_config,
            "tmdb_find_by_imdb",
        )
        .await?;

        Ok(result)
    }
}

impl TmdbMovie {
    /// Get the release year from the release_date
    pub fn year(&self) -> Option<i32> {
        self.release_date
            .as_ref()
            .and_then(|d| d.split('-').next().and_then(|y| y.parse().ok()))
    }

    /// Get genre names from genre list (if available from details endpoint)
    pub fn genre_names(&self) -> Vec<String> {
        self.genres
            .as_ref()
            .map(|g| g.iter().map(|genre| genre.name.clone()).collect())
            .unwrap_or_default()
    }

    /// Get production country codes
    pub fn country_codes(&self) -> Vec<String> {
        self.production_countries
            .as_ref()
            .map(|c| c.iter().map(|country| country.iso_3166_1.clone()).collect())
            .unwrap_or_default()
    }

    /// Get spoken language codes
    pub fn language_codes(&self) -> Vec<String> {
        self.spoken_languages
            .as_ref()
            .map(|l| l.iter().map(|lang| lang.iso_639_1.clone()).collect())
            .unwrap_or_default()
    }
}

impl TmdbCredits {
    /// Get director name from crew
    pub fn director(&self) -> Option<String> {
        self.crew
            .iter()
            .find(|c| c.job == "Director")
            .map(|c| c.name.clone())
    }

    /// Get top billed cast names (first 10)
    pub fn top_cast(&self, limit: usize) -> Vec<String> {
        let mut cast = self.cast.clone();
        cast.sort_by(|a, b| a.order.cmp(&b.order));
        cast.into_iter()
            .take(limit)
            .map(|c| c.name)
            .collect()
    }
}

impl TmdbReleaseDates {
    /// Get US certification (MPAA rating)
    pub fn us_certification(&self) -> Option<String> {
        self.results
            .iter()
            .find(|r| r.iso_3166_1 == "US")
            .and_then(|r| {
                r.release_dates
                    .iter()
                    .filter_map(|d| d.certification.clone())
                    .find(|c| !c.is_empty())
            })
    }

    /// Get certification for a specific country
    pub fn certification(&self, country_code: &str) -> Option<String> {
        self.results
            .iter()
            .find(|r| r.iso_3166_1 == country_code)
            .and_then(|r| {
                r.release_dates
                    .iter()
                    .filter_map(|d| d.certification.clone())
                    .find(|c| !c.is_empty())
            })
    }
}

/// Normalize movie status from TMDB to database-compatible values
pub fn normalize_movie_status(status: Option<&str>) -> Option<String> {
    status.map(|s| {
        match s.to_lowercase().as_str() {
            "released" => "released",
            "rumored" | "planned" => "announced",
            "in production" | "post production" => "in_production",
            "canceled" => "unknown",
            _ => "unknown",
        }
        .to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_url() {
        let client = TmdbClient::new("test_key".to_string());
        assert_eq!(
            client.image_url("/abc123.jpg", "w500"),
            "https://image.tmdb.org/t/p/w500/abc123.jpg"
        );
    }

    #[test]
    fn test_year_parsing() {
        let movie = TmdbMovie {
            id: 1,
            title: "Test".to_string(),
            release_date: Some("2023-05-15".to_string()),
            original_title: None,
            overview: None,
            tagline: None,
            runtime: None,
            poster_path: None,
            backdrop_path: None,
            adult: false,
            video: false,
            vote_average: None,
            vote_count: None,
            popularity: None,
            original_language: None,
            genre_ids: None,
            genres: None,
            belongs_to_collection: None,
            production_countries: None,
            spoken_languages: None,
            imdb_id: None,
            status: None,
        };
        assert_eq!(movie.year(), Some(2023));
    }
}
