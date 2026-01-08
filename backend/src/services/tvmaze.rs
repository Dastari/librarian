//! TVMaze API client for TV show metadata
//!
//! TVMaze is a free API that doesn't require authentication.
//! Base URL: https://api.tvmaze.com

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// TVMaze API client
pub struct TvMazeClient {
    client: Client,
    base_url: String,
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
    pub runtime: Option<u32>,
    pub image: Option<TvMazeImage>,
    pub summary: Option<String>,
    pub rating: Option<TvMazeRating>,
}

/// Season from TVMaze
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
            client: Client::new(),
            base_url: "https://api.tvmaze.com".to_string(),
        }
    }

    /// Search for shows by name
    pub async fn search_shows(&self, query: &str) -> Result<Vec<TvMazeSearchResult>> {
        info!(query = %query, "Searching TVMaze for shows");

        let url = format!("{}/search/shows", self.base_url);
        let response = self
            .client
            .get(&url)
            .query(&[("q", query)])
            .send()
            .await
            .context("Failed to search TVMaze")?;

        if !response.status().is_success() {
            anyhow::bail!("TVMaze search failed with status: {}", response.status());
        }

        let results: Vec<TvMazeSearchResult> = response
            .json()
            .await
            .context("Failed to parse TVMaze search results")?;

        debug!(count = results.len(), "TVMaze search returned results");
        Ok(results)
    }

    /// Get show details by TVMaze ID
    pub async fn get_show(&self, tvmaze_id: u32) -> Result<TvMazeShow> {
        info!(tvmaze_id = tvmaze_id, "Fetching show from TVMaze");

        let url = format!("{}/shows/{}", self.base_url, tvmaze_id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch show from TVMaze")?;

        if !response.status().is_success() {
            anyhow::bail!("TVMaze get show failed with status: {}", response.status());
        }

        let show: TvMazeShow = response
            .json()
            .await
            .context("Failed to parse TVMaze show")?;

        Ok(show)
    }

    /// Get all episodes for a show
    pub async fn get_episodes(&self, tvmaze_id: u32) -> Result<Vec<TvMazeEpisode>> {
        info!(tvmaze_id = tvmaze_id, "Fetching episodes from TVMaze");

        let url = format!("{}/shows/{}/episodes", self.base_url, tvmaze_id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch episodes from TVMaze")?;

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

        debug!(count = episodes.len(), "TVMaze returned episodes");
        Ok(episodes)
    }

    /// Get seasons for a show
    pub async fn get_seasons(&self, tvmaze_id: u32) -> Result<Vec<TvMazeSeason>> {
        info!(tvmaze_id = tvmaze_id, "Fetching seasons from TVMaze");

        let url = format!("{}/shows/{}/seasons", self.base_url, tvmaze_id);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch seasons from TVMaze")?;

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

        debug!(count = seasons.len(), "TVMaze returned seasons");
        Ok(seasons)
    }

    /// Search for a single show (returns best match)
    pub async fn search_single(&self, query: &str) -> Result<Option<TvMazeShow>> {
        let url = format!("{}/singlesearch/shows", self.base_url);
        let response = self
            .client
            .get(&url)
            .query(&[("q", query)])
            .send()
            .await
            .context("Failed to single search TVMaze")?;

        if response.status().is_client_error() {
            return Ok(None);
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

    /// Look up show by TVDB ID
    pub async fn lookup_by_tvdb(&self, tvdb_id: u32) -> Result<Option<TvMazeShow>> {
        let url = format!("{}/lookup/shows", self.base_url);
        let response = self
            .client
            .get(&url)
            .query(&[("thetvdb", tvdb_id.to_string())])
            .send()
            .await
            .context("Failed to lookup by TVDB")?;

        if response.status().is_client_error() {
            return Ok(None);
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

    /// Look up show by IMDB ID
    pub async fn lookup_by_imdb(&self, imdb_id: &str) -> Result<Option<TvMazeShow>> {
        let url = format!("{}/lookup/shows", self.base_url);
        let response = self
            .client
            .get(&url)
            .query(&[("imdb", imdb_id)])
            .send()
            .await
            .context("Failed to lookup by IMDB")?;

        if response.status().is_client_error() {
            return Ok(None);
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
}

impl Default for TvMazeClient {
    fn default() -> Self {
        Self::new()
    }
}

impl TvMazeShow {
    /// Get the premiere year from the premiered date
    pub fn premiere_year(&self) -> Option<i32> {
        self.premiered.as_ref().and_then(|p| {
            p.split('-').next().and_then(|y| y.parse().ok())
        })
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
    /// Parse air date to NaiveDate
    pub fn air_date(&self) -> Option<chrono::NaiveDate> {
        self.airdate.as_ref().and_then(|d| {
            chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()
        })
    }

    /// Get clean summary (strip HTML tags)
    pub fn clean_summary(&self) -> Option<String> {
        self.summary.as_ref().map(|s| {
            let re = regex::Regex::new(r"<[^>]+>").unwrap();
            re.replace_all(s, "").trim().to_string()
        })
    }
}
