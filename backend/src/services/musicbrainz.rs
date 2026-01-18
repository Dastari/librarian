//! MusicBrainz API client for music metadata
//!
//! MusicBrainz is a free, open music encyclopedia that provides metadata.
//! Base URL: https://musicbrainz.org/ws/2
//!
//! Rate limiting: MusicBrainz requires at least 1 second between requests.
//! User-Agent header is required with app name, version, and contact.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::rate_limiter::{RateLimitConfig, RateLimitedClient, RetryConfig, retry_async};

/// MusicBrainz API client with rate limiting
pub struct MusicBrainzClient {
    client: Arc<RateLimitedClient>,
    base_url: String,
    user_agent: String,
    retry_config: RetryConfig,
}

/// Artist search result from MusicBrainz
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzArtistSearch {
    pub count: i32,
    pub offset: i32,
    pub artists: Vec<MusicBrainzArtist>,
}

/// Artist from MusicBrainz
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzArtist {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "sort-name")]
    pub sort_name: Option<String>,
    pub disambiguation: Option<String>,
    pub country: Option<String>,
    #[serde(rename = "type")]
    pub artist_type: Option<String>,
    #[serde(rename = "life-span")]
    pub life_span: Option<MusicBrainzLifeSpan>,
    pub score: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzLifeSpan {
    pub begin: Option<String>,
    pub end: Option<String>,
    pub ended: Option<bool>,
}

/// Release group (album) search from MusicBrainz
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzReleaseGroupSearch {
    pub count: i32,
    pub offset: i32,
    #[serde(rename = "release-groups")]
    pub release_groups: Vec<MusicBrainzReleaseGroup>,
}

/// Release group (album) from MusicBrainz
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzReleaseGroup {
    pub id: Uuid,
    pub title: String,
    #[serde(rename = "primary-type")]
    pub primary_type: Option<String>,
    #[serde(rename = "secondary-types")]
    pub secondary_types: Option<Vec<String>>,
    #[serde(rename = "first-release-date")]
    pub first_release_date: Option<String>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<MusicBrainzArtistCredit>>,
    pub score: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzArtistCredit {
    pub artist: MusicBrainzArtist,
    pub name: Option<String>,
    pub joinphrase: Option<String>,
}

/// Recording (track) from MusicBrainz
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzRecording {
    pub id: Uuid,
    pub title: String,
    pub length: Option<i64>, // milliseconds
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<MusicBrainzArtistCredit>>,
    pub isrcs: Option<Vec<String>>,
}

/// Cover Art Archive result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverArtArchiveResult {
    pub images: Vec<CoverArtImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverArtImage {
    pub id: String,
    pub image: String,
    pub thumbnails: CoverArtThumbnails,
    pub front: bool,
    pub back: bool,
    #[serde(rename = "comment")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverArtThumbnails {
    #[serde(rename = "250")]
    pub small: Option<String>,
    #[serde(rename = "500")]
    pub medium: Option<String>,
    #[serde(rename = "1200")]
    pub large: Option<String>,
}

impl MusicBrainzClient {
    /// Create a new MusicBrainz client
    pub fn new(app_name: &str, app_version: &str, contact: &str) -> Self {
        Self {
            // MusicBrainz requires at least 1 second between requests
            client: Arc::new(RateLimitedClient::new(
                "musicbrainz",
                RateLimitConfig {
                    requests_per_second: 1,
                    burst_size: 1,
                },
            )),
            base_url: "https://musicbrainz.org/ws/2".to_string(),
            user_agent: format!("{}/{} ( {} )", app_name, app_version, contact),
            retry_config: RetryConfig {
                max_retries: 3,
                initial_interval: Duration::from_millis(1500),
                max_interval: Duration::from_secs(10),
                multiplier: 2.0,
            },
        }
    }

    /// Create with default values
    pub fn new_default() -> Self {
        Self::new("Librarian", "0.1.0", "https://github.com/librarian")
    }

    /// Search for artists
    pub async fn search_artists(&self, query: &str) -> Result<Vec<MusicBrainzArtist>> {
        info!(query = %query, "Searching MusicBrainz for artists");

        let url = format!("{}/artist", self.base_url);
        let client = self.client.clone();
        let user_agent = self.user_agent.clone();
        let query_owned = query.to_string();
        let retry_config = self.retry_config.clone();

        let result = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let q = query_owned.clone();
                let ua = user_agent.clone();
                async move {
                    let query_params = [
                        ("query", q),
                        ("fmt", "json".to_string()),
                        ("limit", "25".to_string()),
                    ];

                    let response = client
                        .get_with_headers_and_query(
                            &url,
                            &[("User-Agent", &ua)],
                            &query_params,
                        )
                        .await?;

                    if response.status().as_u16() == 503 {
                        warn!("MusicBrainz rate limit hit, will retry");
                        anyhow::bail!("Rate limited (503)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("MusicBrainz search failed with status: {}", response.status());
                    }

                    let results: MusicBrainzArtistSearch = response
                        .json()
                        .await
                        .context("Failed to parse MusicBrainz artist search results")?;

                    Ok(results.artists)
                }
            },
            &retry_config,
            "musicbrainz_search_artists",
        )
        .await?;

        debug!(count = result.len(), "MusicBrainz artist search returned results");
        Ok(result)
    }

    /// Search for albums (release groups)
    pub async fn search_albums(&self, query: &str) -> Result<Vec<MusicBrainzReleaseGroup>> {
        info!(query = %query, "Searching MusicBrainz for albums");

        let url = format!("{}/release-group", self.base_url);
        let client = self.client.clone();
        let user_agent = self.user_agent.clone();
        let query_owned = query.to_string();
        let retry_config = self.retry_config.clone();

        let result = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let q = query_owned.clone();
                let ua = user_agent.clone();
                async move {
                    let query_params = [
                        ("query", q),
                        ("fmt", "json".to_string()),
                        ("limit", "25".to_string()),
                    ];

                    let response = client
                        .get_with_headers_and_query(
                            &url,
                            &[("User-Agent", &ua)],
                            &query_params,
                        )
                        .await?;

                    if response.status().as_u16() == 503 {
                        anyhow::bail!("Rate limited (503)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("MusicBrainz album search failed with status: {}", response.status());
                    }

                    let results: MusicBrainzReleaseGroupSearch = response
                        .json()
                        .await
                        .context("Failed to parse MusicBrainz release group search results")?;

                    Ok(results.release_groups)
                }
            },
            &retry_config,
            "musicbrainz_search_albums",
        )
        .await?;

        debug!(count = result.len(), "MusicBrainz album search returned results");
        Ok(result)
    }

    /// Get artist details by MBID
    pub async fn get_artist(&self, mbid: Uuid) -> Result<MusicBrainzArtist> {
        info!(mbid = %mbid, "Fetching artist from MusicBrainz");

        let url = format!("{}/artist/{}", self.base_url, mbid);
        let client = self.client.clone();
        let user_agent = self.user_agent.clone();
        let retry_config = self.retry_config.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let ua = user_agent.clone();
                async move {
                    let response = client
                        .get_with_headers_and_query(
                            &url,
                            &[("User-Agent", &ua)],
                            &[("fmt", "json".to_string())],
                        )
                        .await?;

                    if response.status().as_u16() == 503 {
                        anyhow::bail!("Rate limited (503)");
                    }

                    if response.status().as_u16() == 404 {
                        anyhow::bail!("Artist not found on MusicBrainz");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("MusicBrainz get artist failed with status: {}", response.status());
                    }

                    let artist: MusicBrainzArtist = response
                        .json()
                        .await
                        .context("Failed to parse MusicBrainz artist")?;

                    Ok(artist)
                }
            },
            &retry_config,
            "musicbrainz_get_artist",
        )
        .await
    }

    /// Get release group (album) details by MBID
    pub async fn get_release_group(&self, mbid: Uuid) -> Result<MusicBrainzReleaseGroup> {
        info!(mbid = %mbid, "Fetching release group from MusicBrainz");

        let url = format!("{}/release-group/{}", self.base_url, mbid);
        let client = self.client.clone();
        let user_agent = self.user_agent.clone();
        let retry_config = self.retry_config.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let ua = user_agent.clone();
                async move {
                    let query_params = [
                        ("fmt", "json".to_string()),
                        ("inc", "artist-credits".to_string()),
                    ];

                    let response = client
                        .get_with_headers_and_query(&url, &[("User-Agent", &ua)], &query_params)
                        .await?;

                    if response.status().as_u16() == 503 {
                        anyhow::bail!("Rate limited (503)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("MusicBrainz get release group failed with status: {}", response.status());
                    }

                    let rg: MusicBrainzReleaseGroup = response
                        .json()
                        .await
                        .context("Failed to parse MusicBrainz release group")?;

                    Ok(rg)
                }
            },
            &retry_config,
            "musicbrainz_get_release_group",
        )
        .await
    }

    /// Get cover art for a release group from Cover Art Archive
    pub async fn get_cover_art(&self, release_group_id: Uuid) -> Result<Option<String>> {
        let url = format!(
            "https://coverartarchive.org/release-group/{}",
            release_group_id
        );

        // Cover Art Archive doesn't require strict rate limiting
        let response = reqwest::Client::new()
            .get(&url)
            .header("User-Agent", &self.user_agent)
            .send()
            .await?;

        if response.status().as_u16() == 404 {
            return Ok(None);
        }

        if !response.status().is_success() {
            return Ok(None);
        }

        let result: CoverArtArchiveResult = response.json().await?;

        // Get the front cover image
        let front_cover = result
            .images
            .into_iter()
            .find(|img| img.front)
            .or_else(|| None);

        Ok(front_cover.and_then(|img| img.thumbnails.large.or(Some(img.image))))
    }
}

impl MusicBrainzReleaseGroup {
    /// Get release year from first-release-date
    pub fn year(&self) -> Option<i32> {
        self.first_release_date
            .as_ref()
            .and_then(|d| d.split('-').next().and_then(|y| y.parse().ok()))
    }

    /// Get artist names as a combined string
    pub fn artist_names(&self) -> Option<String> {
        self.artist_credit.as_ref().map(|credits| {
            credits
                .iter()
                .map(|c| c.name.clone().unwrap_or_else(|| c.artist.name.clone()))
                .collect::<Vec<_>>()
                .join(", ")
        })
    }

    /// Normalize album type to database-compatible value
    pub fn normalized_type(&self) -> String {
        match self.primary_type.as_deref() {
            Some("Album") => "album",
            Some("Single") => "single",
            Some("EP") => "ep",
            Some("Compilation") => "compilation",
            Some("Soundtrack") => "soundtrack",
            Some("Live") => "live",
            Some("Remix") => "remix",
            _ => "other",
        }
        .to_string()
    }
}
