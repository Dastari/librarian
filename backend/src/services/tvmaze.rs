//! TVMaze API client for TV show metadata
//!
//! TVMaze is a free API that doesn't require authentication.
//! Base URL: https://api.tvmaze.com
//!
//! Rate limiting: TVMaze allows ~20 requests per 10 seconds.
//! This client uses rate limiting and retry logic to handle this gracefully.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::rate_limiter::{RateLimitedClient, RetryConfig, retry_async};

/// TVMaze API client with rate limiting and retry logic
pub struct TvMazeClient {
    client: Arc<RateLimitedClient>,
    base_url: String,
    retry_config: RetryConfig,
}

/// Show search result from TVMaze
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMazeSearchResult {
    pub score: f64,
    pub show: TvMazeShow,
}

/// Show details from TVMaze
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMazeShow {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub show_type: Option<String>,
    pub language: Option<String>,
    pub genres: Vec<String>,
    pub status: Option<String>,
    pub runtime: Option<u32>,
    #[serde(rename = "averageRuntime")]
    pub average_runtime: Option<u32>,
    pub premiered: Option<String>,
    pub ended: Option<String>,
    #[serde(rename = "officialSite")]
    pub official_site: Option<String>,
    pub network: Option<TvMazeNetwork>,
    #[serde(rename = "webChannel")]
    pub web_channel: Option<TvMazeWebChannel>,
    pub image: Option<TvMazeImage>,
    pub summary: Option<String>,
    pub rating: Option<TvMazeRating>,
    pub externals: Option<TvMazeExternals>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMazeNetwork {
    pub id: u32,
    pub name: String,
    pub country: Option<TvMazeCountry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMazeWebChannel {
    pub id: u32,
    pub name: String,
    pub country: Option<TvMazeCountry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMazeCountry {
    pub name: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMazeImage {
    pub medium: Option<String>,
    pub original: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMazeRating {
    pub average: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMazeExternals {
    pub tvrage: Option<u32>,
    pub thetvdb: Option<u32>,
    pub imdb: Option<String>,
}

/// Episode from TVMaze
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMazeEpisode {
    pub id: u32,
    pub name: String,
    pub season: u32,
    pub number: u32,
    #[serde(rename = "type")]
    pub episode_type: Option<String>,
    pub airdate: Option<String>,
    pub airtime: Option<String>,
    #[serde(rename = "airstamp")]
    pub air_stamp: Option<String>,
    pub runtime: Option<u32>,
    pub image: Option<TvMazeImage>,
    pub summary: Option<String>,
    pub rating: Option<TvMazeRating>,
}

/// Schedule entry from TVMaze (episode with embedded show)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMazeScheduleEntry {
    pub id: u32,
    pub name: String,
    /// Season number - can be null for specials
    pub season: Option<u32>,
    /// Episode number - can be null for specials
    pub number: Option<u32>,
    #[serde(rename = "type")]
    pub episode_type: Option<String>,
    pub airdate: Option<String>,
    pub airtime: Option<String>,
    #[serde(rename = "airstamp")]
    pub air_stamp: Option<String>,
    pub runtime: Option<u32>,
    pub image: Option<TvMazeImage>,
    pub summary: Option<String>,
    pub rating: Option<TvMazeRating>,
    /// The show this episode belongs to
    pub show: TvMazeShow,
}

/// Season from TVMaze (for future season-level features)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMazeSeason {
    pub id: u32,
    pub number: u32,
    pub name: Option<String>,
    #[serde(rename = "episodeOrder")]
    pub episode_order: Option<u32>,
    #[serde(rename = "premiereDate")]
    pub premiere_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    pub image: Option<TvMazeImage>,
    pub summary: Option<String>,
}

impl TvMazeClient {
    pub fn new() -> Self {
        Self {
            client: Arc::new(RateLimitedClient::for_tvmaze()),
            base_url: "https://api.tvmaze.com".to_string(),
            retry_config: RetryConfig {
                max_retries: 3,
                initial_interval: Duration::from_millis(500),
                max_interval: Duration::from_secs(10),
                multiplier: 2.0,
            },
        }
    }

    /// Search for shows by name (with rate limiting and retry)
    pub async fn search_shows(&self, query: &str) -> Result<Vec<TvMazeSearchResult>> {
        info!("Searching TVMaze for show '{}'", query);

        let url = format!("{}/search/shows", self.base_url);
        let client = self.client.clone();
        let query_owned = query.to_string();
        let retry_config = self.retry_config.clone();

        let result = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let q = query_owned.clone();
                async move {
                    let response = client
                        .get_with_query(&url, &[("q", &q)])
                        .await?;

                    if response.status().as_u16() == 429 {
                        warn!("TVMaze rate limit hit, will retry");
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("TVMaze search failed with status: {}", response.status());
                    }

                    let results: Vec<TvMazeSearchResult> = response
                        .json()
                        .await
                        .context("Failed to parse TVMaze search results")?;

                    Ok(results)
                }
            },
            &retry_config,
            "tvmaze_search",
        )
        .await?;

        debug!(count = result.len(), "TVMaze search returned results");
        Ok(result)
    }

    /// Get show details by TVMaze ID (with rate limiting and retry)
    pub async fn get_show(&self, tvmaze_id: u32) -> Result<TvMazeShow> {
        debug!("Fetching show details from TVMaze (ID: {})", tvmaze_id);

        let url = format!("{}/shows/{}", self.base_url, tvmaze_id);
        let client = self.client.clone();
        let retry_config = self.retry_config.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                async move {
                    let response = client.get(&url).await?;

                    if response.status().as_u16() == 429 {
                        warn!("TVMaze rate limit hit, will retry");
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("TVMaze get show failed with status: {}", response.status());
                    }

                    let show: TvMazeShow = response
                        .json()
                        .await
                        .context("Failed to parse TVMaze show")?;

                    Ok(show)
                }
            },
            &retry_config,
            "tvmaze_get_show",
        )
        .await
    }

    /// Get all episodes for a show (with rate limiting and retry)
    pub async fn get_episodes(&self, tvmaze_id: u32) -> Result<Vec<TvMazeEpisode>> {
        debug!("Fetching episodes from TVMaze for show {}", tvmaze_id);

        let url = format!("{}/shows/{}/episodes", self.base_url, tvmaze_id);
        let client = self.client.clone();
        let retry_config = self.retry_config.clone();

        let result = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                async move {
                    let response = client.get(&url).await?;

                    if response.status().as_u16() == 429 {
                        warn!("TVMaze rate limit hit, will retry");
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TVMaze get episodes failed with status: {}",
                            response.status()
                        );
                    }

                    let episodes: Vec<TvMazeEpisode> = response
                        .json()
                        .await
                        .context("Failed to parse TVMaze episodes")?;

                    Ok(episodes)
                }
            },
            &retry_config,
            "tvmaze_get_episodes",
        )
        .await?;

        debug!(count = result.len(), "TVMaze returned episodes");
        Ok(result)
    }

    /// Get seasons for a show (for future season-level features)
    #[allow(dead_code)]
    pub async fn get_seasons(&self, tvmaze_id: u32) -> Result<Vec<TvMazeSeason>> {
        debug!("Fetching seasons from TVMaze for show {}", tvmaze_id);

        let url = format!("{}/shows/{}/seasons", self.base_url, tvmaze_id);
        let client = self.client.clone();
        let retry_config = self.retry_config.clone();

        let result = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                async move {
                    let response = client.get(&url).await?;

                    if response.status().as_u16() == 429 {
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TVMaze get seasons failed with status: {}",
                            response.status()
                        );
                    }

                    let seasons: Vec<TvMazeSeason> = response
                        .json()
                        .await
                        .context("Failed to parse TVMaze seasons")?;

                    Ok(seasons)
                }
            },
            &retry_config,
            "tvmaze_get_seasons",
        )
        .await?;

        debug!(count = result.len(), "TVMaze returned seasons");
        Ok(result)
    }

    /// Search for a single show (returns best match) - for future use
    #[allow(dead_code)]
    pub async fn search_single(&self, query: &str) -> Result<Option<TvMazeShow>> {
        let url = format!("{}/singlesearch/shows", self.base_url);
        let client = self.client.clone();
        let query_owned = query.to_string();
        let retry_config = self.retry_config.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let q = query_owned.clone();
                async move {
                    let response = client
                        .get_with_query(&url, &[("q", &q)])
                        .await?;

                    if response.status().is_client_error() && response.status().as_u16() != 429 {
                        return Ok(None);
                    }

                    if response.status().as_u16() == 429 {
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TVMaze single search failed with status: {}",
                            response.status()
                        );
                    }

                    let show: TvMazeShow = response
                        .json()
                        .await
                        .context("Failed to parse TVMaze show")?;

                    Ok(Some(show))
                }
            },
            &retry_config,
            "tvmaze_search_single",
        )
        .await
    }

    /// Look up show by TVDB ID - for future cross-provider matching
    #[allow(dead_code)]
    pub async fn lookup_by_tvdb(&self, tvdb_id: u32) -> Result<Option<TvMazeShow>> {
        let url = format!("{}/lookup/shows", self.base_url);
        let client = self.client.clone();
        let retry_config = self.retry_config.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let tvdb = tvdb_id.to_string();
                async move {
                    let response = client
                        .get_with_query(&url, &[("thetvdb", &tvdb)])
                        .await?;

                    if response.status().is_client_error() && response.status().as_u16() != 429 {
                        return Ok(None);
                    }

                    if response.status().as_u16() == 429 {
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TVMaze TVDB lookup failed with status: {}",
                            response.status()
                        );
                    }

                    let show: TvMazeShow = response
                        .json()
                        .await
                        .context("Failed to parse TVMaze show")?;

                    Ok(Some(show))
                }
            },
            &retry_config,
            "tvmaze_lookup_tvdb",
        )
        .await
    }

    /// Look up show by IMDB ID - for future cross-provider matching
    #[allow(dead_code)]
    pub async fn lookup_by_imdb(&self, imdb_id: &str) -> Result<Option<TvMazeShow>> {
        let url = format!("{}/lookup/shows", self.base_url);
        let client = self.client.clone();
        let imdb_owned = imdb_id.to_string();
        let retry_config = self.retry_config.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let imdb = imdb_owned.clone();
                async move {
                    let response = client
                        .get_with_query(&url, &[("imdb", &imdb)])
                        .await?;

                    if response.status().is_client_error() && response.status().as_u16() != 429 {
                        return Ok(None);
                    }

                    if response.status().as_u16() == 429 {
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TVMaze IMDB lookup failed with status: {}",
                            response.status()
                        );
                    }

                    let show: TvMazeShow = response
                        .json()
                        .await
                        .context("Failed to parse TVMaze show")?;

                    Ok(Some(show))
                }
            },
            &retry_config,
            "tvmaze_lookup_imdb",
        )
        .await
    }

    /// Get TV schedule for a specific date (with rate limiting and retry)
    ///
    /// Returns all episodes airing on the given date.
    /// If no date is provided, defaults to today.
    pub async fn get_schedule(
        &self,
        date: Option<&str>,
        country: Option<&str>,
    ) -> Result<Vec<TvMazeScheduleEntry>> {
        debug!(date = ?date, country = ?country, "Fetching TV schedule from TVMaze");

        let url = format!("{}/schedule", self.base_url);
        let client = self.client.clone();
        let date_owned = date.map(String::from);
        let country_owned = country.map(String::from);
        let retry_config = self.retry_config.clone();

        let result = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let d = date_owned.clone();
                let c = country_owned.clone();
                async move {
                    let mut query_params: Vec<(&str, String)> = Vec::new();
                    if let Some(ref date) = d {
                        query_params.push(("date", date.clone()));
                    }
                    if let Some(ref country) = c {
                        query_params.push(("country", country.clone()));
                    }

                    let response = if query_params.is_empty() {
                        client.get(&url).await?
                    } else {
                        client.get_with_query(&url, &query_params).await?
                    };

                    if response.status().as_u16() == 429 {
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TVMaze schedule request failed with status: {}",
                            response.status()
                        );
                    }

                    let schedule: Vec<TvMazeScheduleEntry> = response
                        .json()
                        .await
                        .context("Failed to parse TVMaze schedule")?;

                    Ok(schedule)
                }
            },
            &retry_config,
            "tvmaze_get_schedule",
        )
        .await?;

        debug!(count = result.len(), "TVMaze schedule returned episodes");
        Ok(result)
    }

    /// Get upcoming episodes for the next N days (with rate limiting)
    ///
    /// Fetches schedules for multiple days and combines them.
    /// Rate limiting ensures we don't overwhelm the API.
    pub async fn get_upcoming_schedule(
        &self,
        days: u32,
        country: Option<&str>,
    ) -> Result<Vec<TvMazeScheduleEntry>> {
        debug!("Fetching {} day TV schedule from TVMaze for {}", days, country.unwrap_or("US"));

        let today = chrono::Utc::now().date_naive();
        let mut all_episodes = Vec::new();

        for day_offset in 0..days {
            let date = today + chrono::Duration::days(day_offset as i64);
            let date_str = date.format("%Y-%m-%d").to_string();

            match self.get_schedule(Some(&date_str), country).await {
                Ok(episodes) => {
                    all_episodes.extend(episodes);
                }
                Err(e) => {
                    debug!(date = %date_str, error = %e, "Failed to fetch schedule for date, continuing");
                }
            }

            // Small delay between day fetches to be extra nice to the API
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        debug!(
            count = all_episodes.len(),
            "TVMaze returned total upcoming episodes"
        );
        Ok(all_episodes)
    }

    /// Get web channel schedule (streaming services)
    ///
    /// Returns episodes from streaming platforms like Netflix, Hulu, etc.
    #[allow(dead_code)]
    pub async fn get_web_schedule(&self, date: Option<&str>) -> Result<Vec<TvMazeScheduleEntry>> {
        debug!("Fetching web schedule from TVMaze for {}", date.unwrap_or("today"));

        let url = format!("{}/schedule/web", self.base_url);
        let client = self.client.clone();
        let date_owned = date.map(String::from);
        let retry_config = self.retry_config.clone();

        let result = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let d = date_owned.clone();
                async move {
                    let response = if let Some(ref date) = d {
                        client.get_with_query(&url, &[("date", date)]).await?
                    } else {
                        client.get(&url).await?
                    };

                    if response.status().as_u16() == 429 {
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TVMaze web schedule request failed with status: {}",
                            response.status()
                        );
                    }

                    let schedule: Vec<TvMazeScheduleEntry> = response
                        .json()
                        .await
                        .context("Failed to parse TVMaze web schedule")?;

                    Ok(schedule)
                }
            },
            &retry_config,
            "tvmaze_get_web_schedule",
        )
        .await?;

        debug!(
            count = result.len(),
            "TVMaze web schedule returned episodes"
        );
        Ok(result)
    }
}

impl Default for TvMazeClient {
    fn default() -> Self {
        Self::new()
    }
}

impl TvMazeShow {
    /// Get the premiere year from the premiered date
    pub fn premiere_year(&self) -> Option<i32> {
        self.premiered
            .as_ref()
            .and_then(|p| p.split('-').next().and_then(|y| y.parse().ok()))
    }

    /// Get the network name
    pub fn network_name(&self) -> Option<&str> {
        self.network
            .as_ref()
            .map(|n| n.name.as_str())
            .or_else(|| self.web_channel.as_ref().map(|w| w.name.as_str()))
    }

    /// Get the poster URL (medium size)
    pub fn poster_url(&self) -> Option<&str> {
        self.image.as_ref().and_then(|i| i.medium.as_deref())
    }

    /// Get the poster URL (original/large size)
    pub fn poster_url_original(&self) -> Option<&str> {
        self.image.as_ref().and_then(|i| i.original.as_deref())
    }

    /// Get clean summary (strip HTML tags)
    pub fn clean_summary(&self) -> Option<String> {
        self.summary.as_ref().map(|s| {
            // Simple HTML tag stripping
            let re = regex::Regex::new(r"<[^>]+>").unwrap();
            re.replace_all(s, "").trim().to_string()
        })
    }

    /// Get TVDB ID if available
    pub fn tvdb_id(&self) -> Option<u32> {
        self.externals.as_ref().and_then(|e| e.thetvdb)
    }

    /// Get IMDB ID if available
    pub fn imdb_id(&self) -> Option<&str> {
        self.externals.as_ref().and_then(|e| e.imdb.as_deref())
    }
}

impl TvMazeEpisode {
    /// Parse air date to NaiveDate - for future date-based matching
    #[allow(dead_code)]
    pub fn air_date(&self) -> Option<chrono::NaiveDate> {
        self.airdate
            .as_ref()
            .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
    }

    /// Get clean summary (strip HTML tags)
    pub fn clean_summary(&self) -> Option<String> {
        self.summary.as_ref().map(|s| {
            let re = regex::Regex::new(r"<[^>]+>").unwrap();
            re.replace_all(s, "").trim().to_string()
        })
    }
}
