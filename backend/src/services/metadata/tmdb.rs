//! TMDB (The Movie Database) API client for movie metadata
//!
//! TMDB is a popular movie/TV database with a free API.
//! Base URL: https://api.themoviedb.org/3
//!
//! Rate limiting: TMDB allows ~40 requests per 10 seconds.
//! This client uses rate limiting and retry logic to handle this gracefully.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::header::RETRY_AFTER;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::services::rate_limiter::{RateLimitConfig, RateLimitedClient, RetryConfig, retry_async};

/// TMDB API client with rate limiting and retry logic
#[derive(Clone)]
pub struct TmdbClient {
    client: Arc<RateLimitedClient>,
    base_url: String,
    api_key: String,
    retry_config: RetryConfig,
    adaptive_delay_ms: Arc<AtomicU64>,
}

/// Movie search result from TMDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbMovieSearchResult {
    pub page: i32,
    pub results: Vec<TmdbMovie>,
    pub total_pages: i32,
    pub total_results: i32,
}

/// Collection search result from TMDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbCollectionSearchResult {
    pub page: i32,
    pub results: Vec<TmdbCollectionSummary>,
    pub total_pages: i32,
    pub total_results: i32,
}

/// Collection summary returned by TMDB search endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbCollectionSummary {
    pub id: i32,
    pub name: String,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
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
    const ADAPTIVE_DELAY_BASE_MS: u64 = 250;
    const ADAPTIVE_DELAY_STEP_DOWN_MS: u64 = 25;
    const ADAPTIVE_DELAY_MAX_MS: u64 = 20_000;

    /// Create a new TMDB client with the given API key
    pub fn new(api_key: String) -> Self {
        Self {
            // TMDB guidance indicates higher soft limits than legacy.
            // Keep this conservative relative to the ~40 rps ceiling.
            client: Arc::new(RateLimitedClient::new(
                "tmdb",
                RateLimitConfig {
                    requests_per_second: 12,
                    burst_size: 24,
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
            adaptive_delay_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    async fn wait_for_adaptive_delay(&self) {
        let delay_ms = self.adaptive_delay_ms.load(Ordering::Relaxed);
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    fn parse_retry_after_ms(response: &reqwest::Response) -> Option<u64> {
        response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<u64>().ok())
            .map(|seconds| seconds.saturating_mul(1000))
    }

    fn increase_adaptive_delay_from_429(&self, response: &reqwest::Response) -> u64 {
        let retry_after_ms = Self::parse_retry_after_ms(response).unwrap_or(0);
        let mut current = self.adaptive_delay_ms.load(Ordering::Relaxed);
        loop {
            let doubled = if current == 0 {
                Self::ADAPTIVE_DELAY_BASE_MS
            } else {
                current.saturating_mul(2)
            };
            let next = doubled
                .max(retry_after_ms)
                .clamp(Self::ADAPTIVE_DELAY_BASE_MS, Self::ADAPTIVE_DELAY_MAX_MS);
            match self.adaptive_delay_ms.compare_exchange(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return next,
                Err(actual) => current = actual,
            }
        }
    }

    fn reduce_adaptive_delay_on_success(&self) {
        let mut current = self.adaptive_delay_ms.load(Ordering::Relaxed);
        while current > 0 {
            let next = current.saturating_sub(Self::ADAPTIVE_DELAY_STEP_DOWN_MS);
            match self.adaptive_delay_ms.compare_exchange(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(actual) => current = actual,
            }
        }
    }

    /// Get the API key this client was created with
    pub fn api_key(&self) -> &str {
        &self.api_key
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

        info!(
            "Searching TMDB for movie query='{}'{}",
            query,
            year.map(|y| format!(" ({})", y)).unwrap_or_default()
        );

        let url = format!("{}/search/movie", self.base_url);
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let query_owned = query.to_string();
        let retry_config = self.retry_config.clone();
        let adaptive_client = self.clone();

        let result = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let q = query_owned.clone();
                let key = api_key.clone();
                let adaptive_client = adaptive_client.clone();
                async move {
                    let q_for_log = q.clone();
                    let mut query_params: Vec<(&str, String)> = vec![
                        ("api_key", key),
                        ("query", q),
                        ("include_adult", "false".to_string()),
                    ];
                    if let Some(y) = year {
                        query_params.push(("year", y.to_string()));
                    }

                    adaptive_client.wait_for_adaptive_delay().await;
                    let response = client.get_with_query(&url, &query_params).await?;

                    if response.status().as_u16() == 429 {
                        let delay_ms = adaptive_client.increase_adaptive_delay_from_429(&response);
                        warn!(
                            "TMDB rate limit hit (HTTP 429) while searching movies for query='{}'; retrying with adaptive delay={}ms",
                            q_for_log,
                            delay_ms
                        );
                        anyhow::bail!("Rate limited (429)");
                    }

                    if response.status().as_u16() == 401 {
                        anyhow::bail!("TMDB API key is invalid");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("TMDB search failed with status: {}", response.status());
                    }
                    adaptive_client.reduce_adaptive_delay_on_success();

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

    /// Search for collections by name
    pub async fn search_collections(&self, query: &str) -> Result<Vec<TmdbCollectionSummary>> {
        if !self.has_api_key() {
            anyhow::bail!("TMDB API key not configured");
        }

        info!("Searching TMDB collections for query='{}'", query);

        let url = format!("{}/search/collection", self.base_url);
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let query_owned = query.to_string();
        let retry_config = self.retry_config.clone();
        let adaptive_client = self.clone();

        let result = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let q = query_owned.clone();
                let key = api_key.clone();
                let adaptive_client = adaptive_client.clone();
                async move {
                    let q_for_log = q.clone();
                    let query_params: Vec<(&str, String)> = vec![
                        ("api_key", key),
                        ("query", q),
                        ("include_adult", "false".to_string()),
                    ];

                    adaptive_client.wait_for_adaptive_delay().await;
                    let response = client.get_with_query(&url, &query_params).await?;

                    if response.status().as_u16() == 429 {
                        let delay_ms = adaptive_client.increase_adaptive_delay_from_429(&response);
                        warn!(
                            "TMDB rate limit hit (HTTP 429) while searching collections for query='{}'; retrying with adaptive delay={}ms",
                            q_for_log,
                            delay_ms
                        );
                        anyhow::bail!("Rate limited (429)");
                    }

                    if response.status().as_u16() == 401 {
                        anyhow::bail!("TMDB API key is invalid");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TMDB collection search failed with status: {}",
                            response.status()
                        );
                    }
                    adaptive_client.reduce_adaptive_delay_on_success();

                    let results: TmdbCollectionSearchResult = response
                        .json()
                        .await
                        .context("Failed to parse TMDB collection search results")?;

                    Ok(results.results)
                }
            },
            &retry_config,
            "tmdb_search_collections",
        )
        .await?;

        debug!(
            count = result.len(),
            "TMDB collection search returned results"
        );
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
        let adaptive_client = self.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let key = api_key.clone();
                let adaptive_client = adaptive_client.clone();
                async move {
                    adaptive_client.wait_for_adaptive_delay().await;
                    let response = client.get_with_query(&url, &[("api_key", &key)]).await?;

                    if response.status().as_u16() == 429 {
                        let delay_ms = adaptive_client.increase_adaptive_delay_from_429(&response);
                        warn!(
                            "TMDB rate limit hit (HTTP 429) while fetching movie details for tmdb_id={}; retrying with adaptive delay={}ms",
                            tmdb_id,
                            delay_ms
                        );
                        anyhow::bail!("Rate limited (429)");
                    }

                    if response.status().as_u16() == 404 {
                        anyhow::bail!("Movie not found on TMDB");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("TMDB get movie failed with status: {}", response.status());
                    }
                    adaptive_client.reduce_adaptive_delay_on_success();

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
        let adaptive_client = self.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let key = api_key.clone();
                let adaptive_client = adaptive_client.clone();
                async move {
                    adaptive_client.wait_for_adaptive_delay().await;
                    let response = client.get_with_query(&url, &[("api_key", &key)]).await?;

                    if response.status().as_u16() == 429 {
                        let delay_ms = adaptive_client.increase_adaptive_delay_from_429(&response);
                        warn!(
                            "TMDB rate limit hit (HTTP 429) while fetching credits for tmdb_id={}; retrying with adaptive delay={}ms",
                            tmdb_id,
                            delay_ms
                        );
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("TMDB get credits failed with status: {}", response.status());
                    }
                    adaptive_client.reduce_adaptive_delay_on_success();

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
        let adaptive_client = self.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let key = api_key.clone();
                let adaptive_client = adaptive_client.clone();
                async move {
                    adaptive_client.wait_for_adaptive_delay().await;
                    let response = client.get_with_query(&url, &[("api_key", &key)]).await?;

                    if response.status().as_u16() == 429 {
                        let delay_ms = adaptive_client.increase_adaptive_delay_from_429(&response);
                        warn!(
                            "TMDB rate limit hit (HTTP 429) while fetching release dates for tmdb_id={}; retrying with adaptive delay={}ms",
                            tmdb_id,
                            delay_ms
                        );
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TMDB get release dates failed with status: {}",
                            response.status()
                        );
                    }
                    adaptive_client.reduce_adaptive_delay_on_success();

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
        let adaptive_client = self.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let key = api_key.clone();
                let adaptive_client = adaptive_client.clone();
                async move {
                    adaptive_client.wait_for_adaptive_delay().await;
                    let response = client.get_with_query(&url, &[("api_key", &key)]).await?;

                    if response.status().as_u16() == 429 {
                        let delay_ms = adaptive_client.increase_adaptive_delay_from_429(&response);
                        warn!(
                            "TMDB rate limit hit (HTTP 429) while fetching collection details for collection_id={}; retrying with adaptive delay={}ms",
                            collection_id,
                            delay_ms
                        );
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TMDB get collection failed with status: {}",
                            response.status()
                        );
                    }
                    adaptive_client.reduce_adaptive_delay_on_success();

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
        let adaptive_client = self.clone();

        #[derive(Deserialize)]
        struct FindResult {
            movie_results: Vec<TmdbMovie>,
        }

        let result = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let key = api_key.clone();
                let adaptive_client = adaptive_client.clone();
                async move {
                    adaptive_client.wait_for_adaptive_delay().await;
                    let response = client
                        .get_with_query(
                            &url,
                            &[("api_key", key), ("external_source", "imdb_id".to_string())],
                        )
                        .await?;

                    if response.status().as_u16() == 429 {
                        let delay_ms = adaptive_client.increase_adaptive_delay_from_429(&response);
                        warn!(
                            "TMDB rate limit hit (HTTP 429) while finding imdb_id={}; retrying with adaptive delay={}ms",
                            imdb_id,
                            delay_ms
                        );
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("TMDB find failed with status: {}", response.status());
                    }
                    adaptive_client.reduce_adaptive_delay_on_success();

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
        cast.into_iter().take(limit).map(|c| c.name).collect()
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
