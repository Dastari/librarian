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

use crate::services::rate_limiter::{RateLimitedClient, RetryConfig, retry_async};

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
    #[serde(default, deserialize_with = "nullable_episode_number")]
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

/// Season from TVMaze (`/shows/{id}/seasons`)
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

impl TvMazeSeason {
    /// Get clean summary (strip HTML tags)
    pub fn clean_summary(&self) -> Option<String> {
        self.summary
            .as_ref()
            .map(|summary| strip_summary_html(summary))
            .filter(|summary| !summary.is_empty())
    }
}

/// One image from `/shows/{id}/images` (posters, backgrounds, banners, typography).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMazeShowImage {
    pub id: u32,
    #[serde(rename = "type")]
    pub image_type: Option<String>,
    #[serde(default)]
    pub main: bool,
    pub resolutions: TvMazeImageResolutions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMazeImageResolutions {
    pub original: Option<TvMazeImageResolution>,
    pub medium: Option<TvMazeImageResolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TvMazeImageResolution {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// The show's wide background: the main one when flagged, else the widest.
pub fn best_background(images: &[TvMazeShowImage]) -> Option<String> {
    images
        .iter()
        .filter(|image| image.image_type.as_deref() == Some("background"))
        .filter_map(|image| {
            image
                .resolutions
                .original
                .as_ref()
                .map(|original| (image.main, original))
        })
        .max_by_key(|(main, original)| (*main, original.width.unwrap_or(0)))
        .map(|(_, original)| original.url.clone())
}

/// Broadcast schedules embed `show`; streaming schedules use `_embedded.show`.
#[derive(Debug, Clone, Deserialize)]
pub struct TvMazeScheduleEntry {
    #[serde(flatten)]
    pub episode: TvMazeEpisode,
    pub show: Option<TvMazeShow>,
    #[serde(rename = "_embedded")]
    pub embedded: Option<TvMazeScheduleEmbedded>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TvMazeScheduleEmbedded {
    pub show: TvMazeShow,
}

impl TvMazeScheduleEntry {
    pub fn show(&self) -> Option<&TvMazeShow> {
        self.show
            .as_ref()
            .or_else(|| self.embedded.as_ref().map(|embedded| &embedded.show))
    }
}

fn nullable_episode_number<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<u32, D::Error> {
    Ok(Option::<u32>::deserialize(deserializer)?.unwrap_or(0))
}

impl TvMazeClient {
    #[cfg(test)]
    pub(crate) fn with_test_url(base_url: String) -> Self {
        let mut client = Self::new();
        client.base_url = base_url;
        client.retry_config.max_retries = 0;
        client
    }
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

    pub async fn schedule(
        &self,
        country: &str,
        date: chrono::NaiveDate,
        streaming: bool,
    ) -> Result<Vec<TvMazeScheduleEntry>> {
        let path = if streaming {
            "schedule/web"
        } else {
            "schedule"
        };
        let url = format!("{}/{path}", self.base_url);
        let params = [("country", country.to_string()), ("date", date.to_string())];
        retry_async(
            || async {
                let response = self.client.get_with_query(&url, &params).await?;
                if !response.status().is_success() {
                    anyhow::bail!(
                        "TVmaze {path} for country={country} date={date} failed: {}",
                        response.status()
                    );
                }
                crate::services::http_client::response_json_limited(
                    response,
                    crate::services::http_client::METADATA_RESPONSE_LIMIT,
                )
                .await
            },
            &self.retry_config,
            "tvmaze_schedule",
        )
        .await
    }

    /// Search for shows by name (with rate limiting and retry)
    pub async fn search_shows(&self, query: &str) -> Result<Vec<TvMazeSearchResult>> {
        info!("Searching TVMaze for show query='{}'", query);

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
                    let response = client.get_with_query(&url, &[("q", &q)]).await?;

                    if response.status().as_u16() == 429 {
                        warn!(
                            "TVMaze rate limit hit (HTTP 429) while searching shows for query='{}'; retrying",
                            q
                        );
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("TVMaze search failed with status: {}", response.status());
                    }

                    let results: Vec<TvMazeSearchResult> =
                        crate::services::http_client::response_json_limited(
                            response,
                            crate::services::http_client::METADATA_RESPONSE_LIMIT,
                        )
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
                        warn!(
                            "TVMaze rate limit hit (HTTP 429) while fetching show details for tvmaze_id={}; retrying",
                            tvmaze_id
                        );
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("TVMaze get show failed with status: {}", response.status());
                    }

                    let show: TvMazeShow =
                        crate::services::http_client::response_json_limited(
                            response,
                            crate::services::http_client::METADATA_RESPONSE_LIMIT,
                        )
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

    /// Get all seasons for a show, with their posters (rate limited, retried).
    pub async fn get_seasons(&self, tvmaze_id: u32) -> Result<Vec<TvMazeSeason>> {
        self.get_json(&format!("/shows/{tvmaze_id}/seasons"), "tvmaze_get_seasons")
            .await
    }

    /// Get every image TVMaze holds for a show: posters, backgrounds, banners.
    pub async fn get_show_images(&self, tvmaze_id: u32) -> Result<Vec<TvMazeShowImage>> {
        self.get_json(
            &format!("/shows/{tvmaze_id}/images"),
            "tvmaze_get_show_images",
        )
        .await
    }

    /// Rate-limited, retried GET of a JSON document under the API base URL.
    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        operation: &'static str,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let client = self.client.clone();
        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                async move {
                    let response = client.get(&url).await?;
                    if response.status().as_u16() == 429 {
                        warn!(operation, "TVMaze rate limit hit (HTTP 429); retrying");
                        anyhow::bail!("Rate limited (429)");
                    }
                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TVMaze {operation} failed with status: {}",
                            response.status()
                        );
                    }
                    crate::services::http_client::response_json_limited(
                        response,
                        crate::services::http_client::METADATA_RESPONSE_LIMIT,
                    )
                    .await
                    .with_context(|| format!("Failed to parse TVMaze {operation} response"))
                }
            },
            &self.retry_config,
            operation,
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
                        warn!(
                            "TVMaze rate limit hit (HTTP 429) while fetching episodes for tvmaze_id={}; retrying",
                            tvmaze_id
                        );
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TVMaze get episodes failed with status: {}",
                            response.status()
                        );
                    }

                    let episodes: Vec<TvMazeEpisode> =
                        crate::services::http_client::response_json_limited(
                            response,
                            crate::services::http_client::METADATA_RESPONSE_LIMIT,
                        )
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
}

impl Default for TvMazeClient {
    fn default() -> Self {
        Self::new()
    }
}

impl TvMazeShow {
    /// Get clean summary (strip HTML tags)
    pub fn clean_summary(&self) -> Option<String> {
        self.summary
            .as_ref()
            .map(|summary| strip_summary_html(summary))
    }
}

impl TvMazeEpisode {
    /// Get clean summary (strip HTML tags)
    pub fn clean_summary(&self) -> Option<String> {
        self.summary
            .as_ref()
            .map(|summary| strip_summary_html(summary))
    }
}

fn strip_summary_html(summary: &str) -> String {
    static HTML_TAG: std::sync::LazyLock<Result<regex::Regex, regex::Error>> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"<[^>]+>"));
    HTML_TAG
        .as_ref()
        .map(|html_tag| html_tag.replace_all(summary, "").trim().to_string())
        .unwrap_or_else(|_| summary.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seasons_parse_with_posters_and_plain_summaries() {
        let seasons: Vec<TvMazeSeason> = serde_json::from_value(serde_json::json!([
            {"id": 1, "number": 1, "name": "", "episodeOrder": 10, "premiereDate": "2023-05-05",
             "endDate": "2023-06-30", "image": {"medium": "https://static.tvmaze.com/m.jpg", "original": "https://static.tvmaze.com/o.jpg"},
             "summary": "<p>Juliette <b>investigates</b>.</p>"},
            {"id": 2, "number": 2, "name": null, "episodeOrder": null, "premiereDate": null,
             "endDate": null, "image": null, "summary": null}
        ]))
        .expect("TVMaze seasons");
        assert_eq!(seasons[0].number, 1);
        assert_eq!(
            seasons[0]
                .image
                .as_ref()
                .and_then(|image| image.original.as_deref()),
            Some("https://static.tvmaze.com/o.jpg")
        );
        assert_eq!(
            seasons[0].clean_summary().as_deref(),
            Some("Juliette investigates.")
        );
        assert!(seasons[1].image.is_none() && seasons[1].clean_summary().is_none());
    }

    #[test]
    fn best_background_prefers_the_main_image_then_the_widest() {
        let images: Vec<TvMazeShowImage> = serde_json::from_value(serde_json::json!([
            {"id": 1, "type": "poster", "main": true, "resolutions": {"original": {"url": "poster", "width": 680}}},
            {"id": 2, "type": "background", "main": false, "resolutions": {"original": {"url": "wide", "width": 3840}}},
            {"id": 3, "type": "background", "main": true, "resolutions": {"original": {"url": "main", "width": 1920}}}
        ]))
        .expect("TVMaze images");
        assert_eq!(best_background(&images).as_deref(), Some("main"));
        assert_eq!(best_background(&images[..2]).as_deref(), Some("wide"));
        assert_eq!(best_background(&images[..1]), None);
    }
}
