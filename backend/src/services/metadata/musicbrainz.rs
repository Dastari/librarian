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

use crate::services::rate_limiter::{RateLimitConfig, RateLimitedClient, RetryConfig, retry_async};

/// MusicBrainz API client with rate limiting
#[derive(Clone)]
pub struct MusicBrainzClient {
    client: Arc<RateLimitedClient>,
    base_url: String,
    user_agent: String,
    retry_config: RetryConfig,
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

/// Cover Art Archive result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverArtArchiveResult {
    pub images: Vec<CoverArtImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverArtImage {
    /// Image ID - Cover Art Archive returns this as a string
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
    pub fn default_user_agent() -> String {
        "Librarian/0.1.0 ( https://github.com/librarian )".to_string()
    }

    /// Create with a fully specified User-Agent value
    pub fn new_with_user_agent(user_agent: String) -> Self {
        Self {
            client: Arc::new(RateLimitedClient::new(
                "musicbrainz",
                RateLimitConfig {
                    requests_per_second: 1,
                    burst_size: 1,
                },
            )),
            base_url: "https://musicbrainz.org/ws/2".to_string(),
            user_agent,
            retry_config: RetryConfig {
                max_retries: 3,
                initial_interval: Duration::from_millis(1500),
                max_interval: Duration::from_secs(10),
                multiplier: 2.0,
            },
        }
    }

    /// Escape special Lucene characters in search terms
    fn escape_lucene(s: &str) -> String {
        // Lucene special characters: + - && || ! ( ) { } [ ] ^ " ~ * ? : \ /
        let mut result = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '+' | '-' | '!' | '(' | ')' | '{' | '}' | '[' | ']' | '^' | '"' | '~' | '*'
                | '?' | ':' | '\\' | '/' => {
                    result.push('\\');
                    result.push(c);
                }
                '&' | '|' => {
                    // Only escape if doubled
                    result.push(c);
                }
                _ => result.push(c),
            }
        }
        result
    }

    /// Try to parse "Artist Name - Album Name" or similar patterns
    fn parse_artist_album_query(query: &str) -> Option<(String, String)> {
        // Try common separators: " - ", " – ", ": "
        for separator in [" - ", " – ", ": "] {
            if let Some(idx) = query.find(separator) {
                let artist = query[..idx].trim();
                let album = query[idx + separator.len()..].trim();
                if !artist.is_empty() && !album.is_empty() {
                    return Some((artist.to_string(), album.to_string()));
                }
            }
        }
        None
    }

    /// Search for albums (release groups) with specific type filtering
    ///
    /// Types can include: "Album", "EP", "Single", "Compilation", "Live", "Soundtrack"
    pub async fn search_albums_with_types(
        &self,
        query: &str,
        types: &[String],
    ) -> Result<Vec<MusicBrainzReleaseGroup>> {
        debug!(
            "Searching MusicBrainz for album '{}' with types {:?}",
            query, types
        );

        let url = format!("{}/release-group", self.base_url);
        let client = self.client.clone();
        let user_agent = self.user_agent.clone();
        let retry_config = self.retry_config.clone();

        // Build type filter for Lucene query
        // MusicBrainz uses "primarytype" field with values like "Album", "EP", "Single", etc.
        let type_filter = if types.is_empty() {
            "primarytype:album".to_string()
        } else {
            let type_conditions: Vec<String> = types
                .iter()
                .map(|t| format!("primarytype:{}", t.to_lowercase()))
                .collect();
            format!("({})", type_conditions.join(" OR "))
        };

        // Build Lucene query similar to search_albums but with custom type filter
        let lucene_query = if let Some(rest) = query.strip_prefix("artist:").map(|s| s.trim()) {
            if let Some((artist, album)) = Self::parse_artist_album_query(rest) {
                format!(
                    "artist:\"{}\" AND releasegroup:\"{}\" AND {}",
                    Self::escape_lucene(&artist),
                    Self::escape_lucene(&album),
                    type_filter
                )
            } else {
                format!("\"{}\" AND {}", Self::escape_lucene(rest), type_filter)
            }
        } else {
            format!(
                "(releasegroup:\"{}\" OR releasegroup:({})) AND {}",
                Self::escape_lucene(query),
                Self::escape_lucene(query),
                type_filter
            )
        };

        debug!(lucene_query = %lucene_query, "Built MusicBrainz Lucene query with types");

        let query_owned = lucene_query;

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
                        .get_with_headers_and_query(&url, &[("User-Agent", &ua)], &query_params)
                        .await?;

                    if response.status().as_u16() == 503 {
                        anyhow::bail!("Rate limited (503)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!(
                            "MusicBrainz album search failed with status: {}",
                            response.status()
                        );
                    }

                    let results: MusicBrainzReleaseGroupSearch =
                        crate::services::http_client::response_json_limited(
                            response,
                            crate::services::http_client::METADATA_RESPONSE_LIMIT,
                        )
                        .await
                        .context("Failed to parse MusicBrainz release group search results")?;

                    Ok(results.release_groups)
                }
            },
            &retry_config,
            "musicbrainz_search_albums_with_types",
        )
        .await?;

        // Sort by score descending
        let mut sorted_results = result;
        sorted_results.sort_by_key(|b| std::cmp::Reverse(b.score.unwrap_or(0)));

        debug!(
            count = sorted_results.len(),
            "MusicBrainz album search with types returned results"
        );
        Ok(sorted_results)
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
                        anyhow::bail!(
                            "MusicBrainz get release group failed with status: {}",
                            response.status()
                        );
                    }

                    let rg: MusicBrainzReleaseGroup =
                        crate::services::http_client::response_json_limited(
                            response,
                            crate::services::http_client::METADATA_RESPONSE_LIMIT,
                        )
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

        info!(
            release_group_id = %release_group_id,
            url = %url,
            "Fetching cover art from Cover Art Archive"
        );

        // Cover Art Archive returns 307 redirect to archive.org
        // Use a client that follows redirects
        let client = crate::services::http_client::outbound_client_builder(
            crate::services::http_client::OutboundHttpProfile::Metadata,
        )
        .redirect(reqwest::redirect::Policy::limited(5))
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")?;

        let response = match client
            .get(&url)
            .header("User-Agent", &self.user_agent)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                warn!(
                    release_group_id = %release_group_id,
                    url = %url,
                    error = %e,
                    "HTTP request to Cover Art Archive failed"
                );
                return Err(e.into());
            }
        };

        let status = response.status();
        info!(
            release_group_id = %release_group_id,
            status = %status,
            "Cover Art Archive response"
        );

        if status.as_u16() == 404 {
            info!(
                release_group_id = %release_group_id,
                "No cover art available for this release group (404)"
            );
            return Ok(None);
        }

        if !status.is_success() {
            let body = crate::services::http_client::response_text_limited(response, 4 * 1024)
                .await
                .unwrap_or_default();
            warn!(
                release_group_id = %release_group_id,
                status = %status,
                body = %body,
                "Cover Art Archive request failed with non-success status"
            );
            return Ok(None);
        }

        let body = crate::services::http_client::response_text_limited(
            response,
            crate::services::http_client::METADATA_RESPONSE_LIMIT,
        )
        .await
        .context("Failed to read bounded response body")?;
        debug!("Cover Art Archive response length: {} bytes", body.len());

        let result: CoverArtArchiveResult = match serde_json::from_str(&body) {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    release_group_id = %release_group_id,
                    error = %e,
                    body_preview = %body.chars().take(500).collect::<String>(),
                    "Failed to parse Cover Art Archive JSON response"
                );
                return Err(e.into());
            }
        };

        // Get the front cover image
        let front_cover = result.images.into_iter().find(|img| img.front);

        if let Some(ref cover) = front_cover {
            let image_url = cover.thumbnails.large.as_deref().unwrap_or(&cover.image);
            info!(
                release_group_id = %release_group_id,
                image_url = %image_url,
                "Found front cover image"
            );
        } else {
            info!(
                release_group_id = %release_group_id,
                "No front cover found in release group images"
            );
        }

        Ok(front_cover.and_then(|img| img.thumbnails.large.or(Some(img.image))))
    }

    /// Fetch the track listing for a release group.
    ///
    /// A release group has many releases (editions, reissues, regional
    /// pressings). MusicBrainz only stores tracks on releases, so this picks
    /// the most representative release — preferring `Official` status, then
    /// the earliest date, then the highest track count — and returns its
    /// tracks. Used by `addAlbum` so an album is created with real Track rows
    /// instead of an empty shell that auto-download can never fulfil.
    pub async fn get_release_group_tracks(&self, mbid: Uuid) -> Result<Vec<MusicBrainzTrack>> {
        debug!(
            "Fetching track listing for MusicBrainz release group {}",
            mbid
        );

        let browse_url = format!("{}/release", self.base_url);
        let client = self.client.clone();
        let user_agent = self.user_agent.clone();
        let retry_config = self.retry_config.clone();
        let mbid_string = mbid.to_string();

        let releases: Vec<ReleaseSummary> = retry_async(
            || {
                let url = browse_url.clone();
                let client = client.clone();
                let ua = user_agent.clone();
                let mbid_string = mbid_string.clone();
                async move {
                    let query_params = [
                        ("fmt", "json".to_string()),
                        ("release-group", mbid_string),
                        ("inc", "media".to_string()),
                        ("limit", "25".to_string()),
                    ];
                    let response = client
                        .get_with_headers_and_query(&url, &[("User-Agent", &ua)], &query_params)
                        .await?;
                    if response.status().as_u16() == 503 {
                        anyhow::bail!("Rate limited (503)");
                    }
                    if !response.status().is_success() {
                        anyhow::bail!(
                            "MusicBrainz release browse failed with status: {}",
                            response.status()
                        );
                    }
                    let parsed: ReleaseBrowseResponse =
                        crate::services::http_client::response_json_limited(
                            response,
                            crate::services::http_client::METADATA_RESPONSE_LIMIT,
                        )
                        .await
                        .context("Failed to parse MusicBrainz release browse")?;
                    Ok(parsed.releases)
                }
            },
            &retry_config,
            "musicbrainz_browse_releases",
        )
        .await?;

        let Some(best) = releases.into_iter().max_by_key(|release| {
            let official = i32::from(
                release
                    .status
                    .as_deref()
                    .map(|status| status.eq_ignore_ascii_case("official"))
                    .unwrap_or(false),
            );
            let tracks: i32 = release.media.iter().map(|medium| medium.track_count).sum();
            // Earlier dates win, so invert the date for the max_by_key.
            let date_rank = release
                .date
                .as_deref()
                .and_then(|date| date.split('-').next())
                .and_then(|year| year.parse::<i32>().ok())
                .map(|year| -year)
                .unwrap_or(i32::MIN);
            (official, date_rank, tracks)
        }) else {
            debug!(
                "MusicBrainz release group {} has no releases; no tracks available",
                mbid
            );
            return Ok(Vec::new());
        };

        let detail_url = format!("{}/release/{}", self.base_url, best.id);
        let client = self.client.clone();
        let user_agent = self.user_agent.clone();
        let retry_config = self.retry_config.clone();
        let detail: ReleaseDetail = retry_async(
            || {
                let url = detail_url.clone();
                let client = client.clone();
                let ua = user_agent.clone();
                async move {
                    let query_params = [
                        ("fmt", "json".to_string()),
                        ("inc", "recordings".to_string()),
                    ];
                    let response = client
                        .get_with_headers_and_query(&url, &[("User-Agent", &ua)], &query_params)
                        .await?;
                    if response.status().as_u16() == 503 {
                        anyhow::bail!("Rate limited (503)");
                    }
                    if !response.status().is_success() {
                        anyhow::bail!(
                            "MusicBrainz release fetch failed with status: {}",
                            response.status()
                        );
                    }
                    let parsed: ReleaseDetail =
                        crate::services::http_client::response_json_limited(
                            response,
                            crate::services::http_client::METADATA_RESPONSE_LIMIT,
                        )
                        .await
                        .context("Failed to parse MusicBrainz release detail")?;
                    Ok(parsed)
                }
            },
            &retry_config,
            "musicbrainz_get_release",
        )
        .await?;

        Ok(Self::tracks_from_release(&detail))
    }

    /// Flatten a release's media/tracks into ordered [`MusicBrainzTrack`]s.
    pub(crate) fn tracks_from_release(detail: &ReleaseDetail) -> Vec<MusicBrainzTrack> {
        let mut tracks = Vec::new();
        for (medium_index, medium) in detail.media.iter().enumerate() {
            let disc_number = medium.position.unwrap_or(medium_index as i32 + 1).max(1);
            for (track_index, track) in medium.tracks.iter().enumerate() {
                let position = track
                    .position
                    .or_else(|| track.number.as_deref().and_then(|n| n.parse::<i32>().ok()))
                    .unwrap_or(track_index as i32 + 1);
                let title = track
                    .title
                    .clone()
                    .or_else(|| track.recording.as_ref().and_then(|r| r.title.clone()))
                    .unwrap_or_else(|| format!("Track {position}"));
                let length_ms = track
                    .length
                    .or_else(|| track.recording.as_ref().and_then(|r| r.length));
                tracks.push(MusicBrainzTrack {
                    position,
                    disc_number,
                    title,
                    duration_secs: length_ms.map(|ms| (ms / 1000) as i32),
                    recording_id: track.recording.as_ref().and_then(|r| r.id.clone()),
                });
            }
        }
        tracks
    }
}

/// One track of a MusicBrainz release, normalized for the Track entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicBrainzTrack {
    /// Track number within its disc.
    pub position: i32,
    /// Disc (medium) number, 1-based.
    pub disc_number: i32,
    /// Track title.
    pub title: String,
    /// Track length in whole seconds, when MusicBrainz knows it.
    pub duration_secs: Option<i32>,
    /// MusicBrainz recording MBID.
    pub recording_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ReleaseBrowseResponse {
    #[serde(default)]
    releases: Vec<ReleaseSummary>,
}

#[derive(Debug, Clone, Deserialize)]
struct ReleaseSummary {
    id: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    media: Vec<MediumSummary>,
}

#[derive(Debug, Clone, Deserialize)]
struct MediumSummary {
    #[serde(rename = "track-count", default)]
    track_count: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ReleaseDetail {
    #[serde(default)]
    media: Vec<MediumDetail>,
}

#[derive(Debug, Clone, Deserialize)]
struct MediumDetail {
    #[serde(default)]
    position: Option<i32>,
    #[serde(default)]
    tracks: Vec<TrackDetail>,
}

#[derive(Debug, Clone, Deserialize)]
struct TrackDetail {
    #[serde(default)]
    position: Option<i32>,
    #[serde(default)]
    number: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    length: Option<i64>,
    #[serde(default)]
    recording: Option<RecordingDetail>,
}

#[derive(Debug, Clone, Deserialize)]
struct RecordingDetail {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    length: Option<i64>,
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

#[cfg(test)]
mod tests {
    use super::{MusicBrainzClient, ReleaseDetail};

    #[test]
    fn tracks_from_release_flattens_multi_disc_listings() {
        let detail: ReleaseDetail = serde_json::from_str(
            r#"{
                "media": [
                    {
                        "position": 1,
                        "tracks": [
                            {"position": 1, "number": "1", "title": "Welcome to the Jungle", "length": 273000},
                            {"position": 2, "number": "2", "recording": {"id": "rec-2", "title": "It's So Easy", "length": 203000}}
                        ]
                    },
                    {
                        "position": 2,
                        "tracks": [
                            {"position": 1, "number": "1", "title": "Bonus Take"}
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

        let tracks = MusicBrainzClient::tracks_from_release(&detail);
        assert_eq!(tracks.len(), 3);
        assert_eq!(tracks[0].title, "Welcome to the Jungle");
        assert_eq!(tracks[0].disc_number, 1);
        assert_eq!(tracks[0].duration_secs, Some(273));
        // Falls back to the recording when the track has no title of its own.
        assert_eq!(tracks[1].title, "It's So Easy");
        assert_eq!(tracks[1].recording_id.as_deref(), Some("rec-2"));
        assert_eq!(tracks[2].disc_number, 2);
        assert_eq!(tracks[2].position, 1);
        assert_eq!(tracks[2].duration_secs, None);
    }

    #[test]
    fn tracks_from_release_handles_empty_media() {
        let detail: ReleaseDetail = serde_json::from_str(r#"{"media": []}"#).unwrap();
        assert!(MusicBrainzClient::tracks_from_release(&detail).is_empty());
    }
}
