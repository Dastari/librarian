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

/// Release (specific pressing/edition of an album)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzRelease {
    pub id: Uuid,
    pub title: String,
    pub status: Option<String>,
    pub date: Option<String>,
    pub country: Option<String>,
    pub barcode: Option<String>,
    #[serde(rename = "release-group")]
    pub release_group: Option<MusicBrainzReleaseGroupRef>,
    #[serde(rename = "artist-credit")]
    pub artist_credit: Option<Vec<MusicBrainzArtistCredit>>,
    pub media: Option<Vec<MusicBrainzMedium>>,
}

/// Simplified release group reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzReleaseGroupRef {
    pub id: Uuid,
    pub title: String,
    #[serde(rename = "primary-type")]
    pub primary_type: Option<String>,
}

/// Medium (disc) in a release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzMedium {
    pub position: Option<i32>,
    pub format: Option<String>,
    #[serde(rename = "track-count")]
    pub track_count: Option<i32>,
    pub tracks: Option<Vec<MusicBrainzTrack>>,
}

/// Track position on a medium
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzTrack {
    pub id: Uuid,
    pub number: String,
    pub position: Option<i32>,
    pub title: String,
    pub length: Option<i64>, // milliseconds
    pub recording: MusicBrainzRecording,
}

/// Release browse result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzReleaseBrowse {
    #[serde(rename = "release-count")]
    pub release_count: i32,
    #[serde(rename = "release-offset")]
    pub release_offset: Option<i32>,
    pub releases: Vec<MusicBrainzRelease>,
}

/// Track information for creating database records
#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub musicbrainz_id: Uuid,
    pub title: String,
    pub track_number: i32,
    pub disc_number: i32,
    pub duration_secs: Option<i32>,
    pub isrc: Option<String>,
    pub artist_name: Option<String>,
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
        debug!("Searching MusicBrainz for artist '{}'", query);

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
        debug!("Searching MusicBrainz for album '{}'", query);

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
        debug!("Fetching artist {} from MusicBrainz", mbid);

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
        debug!("Fetching release group {} from MusicBrainz", mbid);

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

    /// Get releases for a release group, including track listings
    /// 
    /// This fetches all releases (editions) of an album and their track listings.
    /// We use the "Official" status release with the most complete track listing.
    pub async fn get_releases_for_release_group(
        &self,
        release_group_id: Uuid,
    ) -> Result<Vec<MusicBrainzRelease>> {
        debug!("Fetching releases with tracks for {} from MusicBrainz", release_group_id);

        let url = format!("{}/release", self.base_url);
        let client = self.client.clone();
        let user_agent = self.user_agent.clone();
        let retry_config = self.retry_config.clone();
        let rg_id = release_group_id.to_string();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                let ua = user_agent.clone();
                let rg = rg_id.clone();
                async move {
                    // Query releases belonging to this release group, with media and recordings
                    let query_params = [
                        ("release-group", rg),
                        ("fmt", "json".to_string()),
                        ("inc", "recordings+artist-credits+isrcs".to_string()),
                        ("limit", "100".to_string()),
                    ];

                    let response = client
                        .get_with_headers_and_query(&url, &[("User-Agent", &ua)], &query_params)
                        .await?;

                    if response.status().as_u16() == 503 {
                        anyhow::bail!("Rate limited (503)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "MusicBrainz get releases failed with status: {}",
                            response.status()
                        );
                    }

                    let browse_result: MusicBrainzReleaseBrowse = response
                        .json()
                        .await
                        .context("Failed to parse MusicBrainz release browse results")?;

                    Ok(browse_result.releases)
                }
            },
            &retry_config,
            "musicbrainz_get_releases",
        )
        .await
    }

    /// Get tracks for a release group
    ///
    /// Fetches the track listing from the best available release.
    /// Prefers "Official" status releases with complete track listings.
    pub async fn get_tracks_for_release_group(
        &self,
        release_group_id: Uuid,
    ) -> Result<Vec<TrackInfo>> {
        let releases = self.get_releases_for_release_group(release_group_id).await?;

        if releases.is_empty() {
            warn!(release_group_id = %release_group_id, "No releases found for release group");
            return Ok(vec![]);
        }

        // Find the best release (prefer Official, then by track count)
        let best_release = releases
            .iter()
            .filter(|r| r.status.as_deref() == Some("Official"))
            .max_by_key(|r| {
                r.media
                    .as_ref()
                    .map(|media| media.iter().filter_map(|m| m.track_count).sum::<i32>())
                    .unwrap_or(0)
            })
            .or_else(|| {
                // Fallback to any release with the most tracks
                releases.iter().max_by_key(|r| {
                    r.media
                        .as_ref()
                        .map(|media| media.iter().filter_map(|m| m.track_count).sum::<i32>())
                        .unwrap_or(0)
                })
            });

        let Some(release) = best_release else {
            return Ok(vec![]);
        };

        info!(
            release_id = %release.id,
            release_title = %release.title,
            status = ?release.status,
            "Selected release for track listing"
        );

        // Extract tracks from all media (discs)
        let mut tracks = Vec::new();

        if let Some(ref media) = release.media {
            for medium in media {
                let disc_number = medium.position.unwrap_or(1);

                if let Some(ref medium_tracks) = medium.tracks {
                    for track in medium_tracks {
                        let track_number = track.position.unwrap_or_else(|| {
                            track.number.parse().unwrap_or(1)
                        });

                        // Get artist name from recording's artist credit
                        let artist_name = track
                            .recording
                            .artist_credit
                            .as_ref()
                            .and_then(|credits| {
                                credits.first().map(|c| {
                                    c.name.clone().unwrap_or_else(|| c.artist.name.clone())
                                })
                            });

                        // Get first ISRC if available
                        let isrc = track
                            .recording
                            .isrcs
                            .as_ref()
                            .and_then(|isrcs| isrcs.first().cloned());

                        // Convert duration from milliseconds to seconds
                        let duration_secs = track
                            .recording
                            .length
                            .or(track.length)
                            .map(|ms| (ms / 1000) as i32);

                        tracks.push(TrackInfo {
                            musicbrainz_id: track.recording.id,
                            title: track.recording.title.clone(),
                            track_number,
                            disc_number,
                            duration_secs,
                            isrc,
                            artist_name,
                        });
                    }
                }
            }
        }

        info!(
            track_count = tracks.len(),
            "Extracted tracks from release"
        );

        Ok(tracks)
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
