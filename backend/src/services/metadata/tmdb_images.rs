//! TMDB's image lists: `/movie/{id}/images`, `/tv/{id}/images`, and a season's or an episode's.
//! The artwork picker shows them all, with each image's language, size and votes.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::TmdbClient;
use crate::services::rate_limiter::retry_async;

/// One image in a TMDB image list.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TmdbImage {
    pub file_path: String,
    #[serde(default)]
    pub iso_639_1: Option<String>,
    #[serde(default)]
    pub width: Option<i32>,
    #[serde(default)]
    pub height: Option<i32>,
    #[serde(default)]
    pub vote_average: Option<f64>,
    #[serde(default)]
    pub vote_count: Option<i32>,
}

/// A TMDB image list. Seasons have only posters and episodes only stills.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TmdbImages {
    #[serde(default)]
    pub posters: Vec<TmdbImage>,
    #[serde(default)]
    pub backdrops: Vec<TmdbImage>,
    #[serde(default)]
    pub logos: Vec<TmdbImage>,
    #[serde(default)]
    pub stills: Vec<TmdbImage>,
}

/// What a TMDB image list belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TmdbImageTarget {
    Movie(i32),
    Show(i32),
    Season {
        show: i32,
        season: i32,
    },
    Episode {
        show: i32,
        season: i32,
        episode: i32,
    },
}

impl TmdbImageTarget {
    pub fn path(self) -> String {
        match self {
            Self::Movie(id) => format!("/movie/{id}/images"),
            Self::Show(id) => format!("/tv/{id}/images"),
            Self::Season { show, season } => format!("/tv/{show}/season/{season}/images"),
            Self::Episode {
                show,
                season,
                episode,
            } => format!("/tv/{show}/season/{season}/episode/{episode}/images"),
        }
    }
}

impl TmdbClient {
    /// Every image TMDB has for a title, in every language (no `language` filter).
    pub async fn get_images(&self, target: TmdbImageTarget) -> Result<TmdbImages> {
        if !self.has_api_key() {
            anyhow::bail!("TMDB API key not configured");
        }
        let url = format!("{}{}", self.base_url, target.path());
        let client = self.client.clone();
        let api_key = self.api_key.clone();
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
                            image_target = ?target,
                            delay_ms,
                            "TMDB rate limit hit (HTTP 429) listing images for {target:?}; retrying"
                        );
                        anyhow::bail!("Rate limited (429)");
                    }
                    if response.status().as_u16() == 404 {
                        return Ok(TmdbImages::default());
                    }
                    if !response.status().is_success() {
                        anyhow::bail!(
                            "TMDB image list for {target:?} failed with status: {}",
                            response.status()
                        );
                    }
                    adaptive_client.reduce_adaptive_delay_on_success();
                    crate::services::http_client::response_json_limited(
                        response,
                        crate::services::http_client::METADATA_RESPONSE_LIMIT,
                    )
                    .await
                    .with_context(|| format!("Failed to parse the TMDB image list for {target:?}"))
                }
            },
            &self.retry_config,
            "tmdb_get_images",
        )
        .await
    }
}
