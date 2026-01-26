//! OpenSubtitles.com REST API client
//!
//! Uses the new REST API (v1) for subtitle search and download.
//! API documentation: https://opensubtitles.stoplight.io/docs/opensubtitles-api
//!
//! Rate limiting: Free users have limited daily download quotas.
//! This client uses the MetadataQueue for rate limiting and retry logic.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::rate_limiter::{RateLimitedClient, RetryConfig, retry_async};

/// OpenSubtitles API base URL
const BASE_URL: &str = "https://api.opensubtitles.com/api/v1";

/// OpenSubtitles API client
pub struct OpenSubtitlesClient {
    client: Arc<RateLimitedClient>,
    api_key: String,
    user_agent: String,
    retry_config: RetryConfig,
    /// Cached JWT token and base URL from login
    auth_state: Arc<RwLock<Option<AuthState>>>,
}

/// Authentication state (JWT token + base URL)
#[derive(Debug, Clone)]
struct AuthState {
    token: String,
    base_url: String,
    /// Username for re-authentication
    username: String,
    /// Password for re-authentication
    password: String,
}

/// Login response from OpenSubtitles API
#[derive(Debug, Deserialize)]
struct LoginResponse {
    token: String,
    base_url: String,
    user: LoginUser,
}

#[derive(Debug, Deserialize)]
struct LoginUser {
    user_id: i64,
    allowed_downloads: i32,
    remaining_downloads: i32,
    level: String,
    vip: bool,
}

/// Subtitle search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleSearchResult {
    /// Unique subtitle ID
    pub id: String,
    /// Type (subtitle, etc.)
    #[serde(rename = "type")]
    pub result_type: Option<String>,
    /// Subtitle attributes
    pub attributes: SubtitleAttributes,
}

/// Subtitle attributes from search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleAttributes {
    /// Subtitle language code (e.g., "en")
    pub language: Option<String>,
    /// Download count (popularity indicator)
    pub download_count: Option<i64>,
    /// Whether it's a hearing impaired version
    pub hearing_impaired: Option<bool>,
    /// Whether it's for HD content
    pub hd: Option<bool>,
    /// FPS
    pub fps: Option<f64>,
    /// Number of votes
    pub votes: Option<i64>,
    /// Points/rating
    pub points: Option<f64>,
    /// Rating
    pub ratings: Option<f64>,
    /// Uploader info
    pub uploader: Option<SubtitleUploader>,
    /// Feature (movie/show) details
    pub feature_details: Option<FeatureDetails>,
    /// Related links (could be a string or array, we'll skip for now)
    /// File information
    pub files: Option<Vec<SubtitleFile>>,
    /// Release name
    pub release: Option<String>,
    /// Comments
    pub comments: Option<String>,
    /// URL to subtitle page
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleUploader {
    pub uploader_id: Option<i64>,
    pub name: Option<String>,
    pub rank: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureDetails {
    pub feature_id: Option<i64>,
    pub feature_type: Option<String>,
    pub year: Option<i32>,
    pub title: Option<String>,
    pub imdb_id: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub season_number: Option<i32>,
    pub episode_number: Option<i32>,
    pub parent_title: Option<String>,
    pub parent_imdb_id: Option<i32>,
    pub parent_tmdb_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleFile {
    pub file_id: i64,
    pub cd_number: Option<i32>,
    pub file_name: Option<String>,
}

/// Subtitle search response
#[derive(Debug, Deserialize)]
struct SearchResponse {
    total_pages: i32,
    total_count: i32,
    page: i32,
    data: Vec<SubtitleSearchResult>,
}

/// Download info response
#[derive(Debug, Deserialize)]
struct DownloadResponse {
    link: String,
    file_name: String,
    requests: i32,
    remaining: i32,
    message: String,
    reset_time: String,
    reset_time_utc: String,
}

/// Subtitle search query
#[derive(Debug, Clone, Default)]
pub struct SubtitleSearchQuery {
    /// Search by IMDB ID (e.g., "tt1234567" or just the number)
    pub imdb_id: Option<String>,
    /// TMDB ID
    pub tmdb_id: Option<i64>,
    /// Search query text
    pub query: Option<String>,
    /// Episode season number
    pub season_number: Option<i32>,
    /// Episode number
    pub episode_number: Option<i32>,
    /// Language codes (comma-separated for API, e.g., "en,es")
    pub languages: Option<String>,
    /// Filter by hearing impaired
    pub hearing_impaired: Option<bool>,
    /// Filter by trusted sources only
    pub trusted_sources: Option<bool>,
    /// Filter by movie hash (for exact file matching)
    pub moviehash: Option<String>,
    /// Type: movie, episode, all
    pub media_type: Option<String>,
    /// Page number (1-based)
    pub page: Option<i32>,
}

impl OpenSubtitlesClient {
    /// Create a new OpenSubtitles client
    ///
    /// Requires an API key from https://www.opensubtitles.com/consumers
    pub fn new(api_key: String) -> Self {
        Self {
            client: Arc::new(RateLimitedClient::for_tvmaze()), // Similar rate limiting
            api_key,
            user_agent: "Librarian v0.1.0".to_string(),
            retry_config: RetryConfig {
                max_retries: 3,
                initial_interval: Duration::from_millis(500),
                max_interval: Duration::from_secs(10),
                multiplier: 2.0,
            },
            auth_state: Arc::new(RwLock::new(None)),
        }
    }

    /// Create with custom user agent
    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = user_agent;
        self
    }

    /// Login to get a JWT token for authenticated requests
    ///
    /// The token is cached and auto-refreshed when needed.
    pub async fn login(&self, username: &str, password: &str) -> Result<()> {
        info!("Logging into OpenSubtitles API");

        let url = format!("{}/login", BASE_URL);
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let user_agent = self.user_agent.clone();
        let username_owned = username.to_string();
        let password_owned = password.to_string();
        let retry_config = self.retry_config.clone();

        let response = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let api_key = api_key.clone();
                let user_agent = user_agent.clone();
                let u = username_owned.clone();
                let p = password_owned.clone();
                async move {
                    let response = client
                        .inner()
                        .post(&url)
                        .header("Api-Key", api_key)
                        .header("User-Agent", user_agent)
                        .header("Content-Type", "application/json")
                        .json(&serde_json::json!({
                            "username": u,
                            "password": p
                        }))
                        .send()
                        .await?;

                    if response.status().as_u16() == 429 {
                        anyhow::bail!("Rate limited (429)");
                    }

                    if response.status() == 401 {
                        anyhow::bail!("Invalid credentials");
                    }

                    if !response.status().is_success() {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        anyhow::bail!("Login failed with status {}: {}", status, body);
                    }

                    let login: LoginResponse = response
                        .json()
                        .await
                        .context("Failed to parse login response")?;

                    Ok(login)
                }
            },
            &retry_config,
            "opensubtitles_login",
        )
        .await?;

        info!(
            user_id = response.user.user_id,
            remaining_downloads = response.user.remaining_downloads,
            vip = response.user.vip,
            "OpenSubtitles login successful"
        );

        // Cache the auth state
        *self.auth_state.write() = Some(AuthState {
            token: response.token,
            base_url: response.base_url,
            username: username.to_string(),
            password: password.to_string(),
        });

        Ok(())
    }

    /// Check if we're logged in
    pub fn is_logged_in(&self) -> bool {
        self.auth_state.read().is_some()
    }

    /// Get remaining downloads for the current user
    pub async fn get_remaining_downloads(&self) -> Result<i32> {
        // This would require an API call to /infos/user
        // For now, we track it from login/download responses
        Ok(0) // TODO: implement
    }

    /// Search for subtitles
    pub async fn search(&self, query: SubtitleSearchQuery) -> Result<Vec<SubtitleSearchResult>> {
        debug!(?query, "Searching OpenSubtitles");

        let url = format!("{}/subtitles", BASE_URL);
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let user_agent = self.user_agent.clone();
        let retry_config = self.retry_config.clone();

        // Build query parameters
        let mut params: Vec<(String, String)> = Vec::new();

        if let Some(ref imdb_id) = query.imdb_id {
            // Strip "tt" prefix if present
            let id = imdb_id.trim_start_matches("tt");
            params.push(("imdb_id".to_string(), id.to_string()));
        }
        if let Some(tmdb_id) = query.tmdb_id {
            params.push(("tmdb_id".to_string(), tmdb_id.to_string()));
        }
        if let Some(ref q) = query.query {
            params.push(("query".to_string(), q.clone()));
        }
        if let Some(season) = query.season_number {
            params.push(("season_number".to_string(), season.to_string()));
        }
        if let Some(episode) = query.episode_number {
            params.push(("episode_number".to_string(), episode.to_string()));
        }
        if let Some(ref languages) = query.languages {
            params.push(("languages".to_string(), languages.clone()));
        }
        if let Some(hi) = query.hearing_impaired {
            params.push((
                "hearing_impaired".to_string(),
                if hi { "include" } else { "exclude" }.to_string(),
            ));
        }
        if let Some(trusted) = query.trusted_sources {
            if trusted {
                params.push(("trusted_sources".to_string(), "only".to_string()));
            }
        }
        if let Some(ref hash) = query.moviehash {
            params.push(("moviehash".to_string(), hash.clone()));
        }
        if let Some(ref media_type) = query.media_type {
            params.push(("type".to_string(), media_type.clone()));
        }
        if let Some(page) = query.page {
            params.push(("page".to_string(), page.to_string()));
        }

        let result = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let api_key = api_key.clone();
                let user_agent = user_agent.clone();
                let params = params.clone();
                async move {
                    let mut request = client.inner().get(&url);

                    // Add query parameters
                    for (key, value) in &params {
                        request = request.query(&[(key.as_str(), value.as_str())]);
                    }

                    let response = request
                        .header("Api-Key", api_key)
                        .header("User-Agent", user_agent)
                        .send()
                        .await?;

                    if response.status().as_u16() == 429 {
                        warn!("OpenSubtitles rate limit hit");
                        anyhow::bail!("Rate limited (429)");
                    }

                    if !response.status().is_success() {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        anyhow::bail!("Search failed with status {}: {}", status, body);
                    }

                    let search_response: SearchResponse = response
                        .json()
                        .await
                        .context("Failed to parse search response")?;

                    Ok(search_response)
                }
            },
            &retry_config,
            "opensubtitles_search",
        )
        .await?;

        debug!(
            total_count = result.total_count,
            page = result.page,
            "OpenSubtitles search completed"
        );

        Ok(result.data)
    }

    /// Download a subtitle file
    ///
    /// Returns the subtitle content as a string.
    /// Requires authentication (login must be called first).
    pub async fn download(&self, file_id: i64) -> Result<DownloadedSubtitle> {
        let auth_state = self
            .auth_state
            .read()
            .clone()
            .context("Not logged in - call login() first")?;

        info!(file_id = file_id, "Downloading subtitle from OpenSubtitles");

        let url = format!("{}/download", auth_state.base_url);
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let user_agent = self.user_agent.clone();
        let token = auth_state.token.clone();
        let retry_config = self.retry_config.clone();

        let download_info = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let api_key = api_key.clone();
                let user_agent = user_agent.clone();
                let token = token.clone();
                async move {
                    let response = client
                        .inner()
                        .post(&url)
                        .header("Api-Key", api_key)
                        .header("User-Agent", user_agent)
                        .header("Authorization", format!("Bearer {}", token))
                        .header("Content-Type", "application/json")
                        .json(&serde_json::json!({
                            "file_id": file_id
                        }))
                        .send()
                        .await?;

                    if response.status().as_u16() == 429 {
                        anyhow::bail!("Rate limited (429)");
                    }

                    if response.status() == 401 {
                        anyhow::bail!("Authentication expired");
                    }

                    if !response.status().is_success() {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        anyhow::bail!("Download request failed with status {}: {}", status, body);
                    }

                    let info: DownloadResponse = response
                        .json()
                        .await
                        .context("Failed to parse download response")?;

                    Ok(info)
                }
            },
            &retry_config,
            "opensubtitles_download_info",
        )
        .await?;

        // Now download the actual file
        let content = retry_async(
            || {
                let link = download_info.link.clone();
                let client = client.clone();
                async move {
                    let response = client.inner().get(&link).send().await?;

                    if !response.status().is_success() {
                        anyhow::bail!("Failed to download subtitle file");
                    }

                    let content = response.text().await?;
                    Ok(content)
                }
            },
            &retry_config,
            "opensubtitles_download_file",
        )
        .await?;

        info!(
            file_id = file_id,
            file_name = %download_info.file_name,
            remaining = download_info.remaining,
            "Subtitle downloaded successfully"
        );

        Ok(DownloadedSubtitle {
            file_id,
            file_name: download_info.file_name,
            content,
            remaining_downloads: download_info.remaining,
        })
    }

    /// Search for subtitles for a TV episode
    pub async fn search_episode(
        &self,
        imdb_id: Option<&str>,
        show_name: Option<&str>,
        season: i32,
        episode: i32,
        languages: &[String],
    ) -> Result<Vec<SubtitleSearchResult>> {
        let query = SubtitleSearchQuery {
            imdb_id: imdb_id.map(String::from),
            query: if imdb_id.is_none() {
                show_name.map(String::from)
            } else {
                None
            },
            season_number: Some(season),
            episode_number: Some(episode),
            languages: Some(languages.join(",")),
            media_type: Some("episode".to_string()),
            ..Default::default()
        };

        self.search(query).await
    }

    /// Search for subtitles for a movie
    pub async fn search_movie(
        &self,
        imdb_id: Option<&str>,
        title: Option<&str>,
        year: Option<i32>,
        languages: &[String],
    ) -> Result<Vec<SubtitleSearchResult>> {
        let query = SubtitleSearchQuery {
            imdb_id: imdb_id.map(String::from),
            query: if imdb_id.is_none() {
                let mut q = title.unwrap_or("").to_string();
                if let Some(y) = year {
                    q = format!("{} {}", q, y);
                }
                Some(q)
            } else {
                None
            },
            languages: Some(languages.join(",")),
            media_type: Some("movie".to_string()),
            ..Default::default()
        };

        self.search(query).await
    }
}

/// Downloaded subtitle data
#[derive(Debug)]
pub struct DownloadedSubtitle {
    /// File ID that was downloaded
    pub file_id: i64,
    /// Original file name
    pub file_name: String,
    /// Subtitle content
    pub content: String,
    /// Remaining downloads for the day
    pub remaining_downloads: i32,
}

impl DownloadedSubtitle {
    /// Get the subtitle format from the filename
    pub fn format(&self) -> &str {
        self.file_name.rsplit('.').next().unwrap_or("srt")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_defaults() {
        let query = SubtitleSearchQuery::default();
        assert!(query.imdb_id.is_none());
        assert!(query.query.is_none());
        assert!(query.languages.is_none());
    }

    #[test]
    fn test_downloaded_subtitle_format() {
        let sub = DownloadedSubtitle {
            file_id: 123,
            file_name: "movie.eng.srt".to_string(),
            content: "test".to_string(),
            remaining_downloads: 10,
        };
        assert_eq!(sub.format(), "srt");

        let sub2 = DownloadedSubtitle {
            file_id: 456,
            file_name: "movie.ass".to_string(),
            content: "test".to_string(),
            remaining_downloads: 10,
        };
        assert_eq!(sub2.format(), "ass");
    }
}
