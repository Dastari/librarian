//! Rate limiting and retry logic for external API calls
//!
//! Provides rate-limited HTTP clients and retry utilities to prevent
//! overwhelming external APIs and handle transient failures gracefully.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use backoff::ExponentialBackoff;
use backoff::backoff::Backoff;
use governor::{
    Quota, RateLimiter,
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
};
use reqwest::{Client, Response};
use tracing::{debug, warn};

/// Configuration for rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per second
    pub requests_per_second: u32,
    /// Burst capacity (allows short bursts above the rate)
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 2,
            burst_size: 5,
        }
    }
}

/// A rate-limited HTTP client wrapper
pub struct RateLimitedClient {
    client: Client,
    limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    name: String,
}

impl RateLimitedClient {
    /// Create a new rate-limited client
    pub fn new(name: &str, config: RateLimitConfig) -> Self {
        let quota = Quota::per_second(
            NonZeroU32::new(config.requests_per_second).unwrap_or(NonZeroU32::MIN),
        )
        .allow_burst(NonZeroU32::new(config.burst_size).unwrap_or(NonZeroU32::MIN));

        let limiter = Arc::new(RateLimiter::direct(quota));

        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            limiter,
            name: name.to_string(),
        }
    }

    /// Create a client with custom settings for a specific API
    pub fn for_tvmaze() -> Self {
        // TVMaze allows ~20 requests per 10 seconds, so ~2/sec with burst of 5
        Self::new(
            "tvmaze",
            RateLimitConfig {
                requests_per_second: 2,
                burst_size: 5,
            },
        )
    }

    /// Create a client for TMDB API
    pub fn for_tmdb() -> Self {
        // TMDB allows ~40 requests per 10 seconds, so ~4/sec with burst of 10
        Self::new(
            "tmdb",
            RateLimitConfig {
                requests_per_second: 4,
                burst_size: 10,
            },
        )
    }

    /// Create a client for MusicBrainz API
    pub fn for_musicbrainz() -> Self {
        // MusicBrainz requires max 1 request per second
        Self::new(
            "musicbrainz",
            RateLimitConfig {
                requests_per_second: 1,
                burst_size: 1,
            },
        )
    }

    /// Create a client for Audible API
    pub fn for_audible() -> Self {
        // Conservative rate for Audible (no official limits published)
        Self::new(
            "audible",
            RateLimitConfig {
                requests_per_second: 2,
                burst_size: 5,
            },
        )
    }

    /// Create a client for OpenSubtitles API
    pub fn for_opensubtitles() -> Self {
        // OpenSubtitles REST API has rate limits
        Self::new(
            "opensubtitles",
            RateLimitConfig {
                requests_per_second: 2,
                burst_size: 5,
            },
        )
    }

    /// Create a client for RSS feed fetching (more lenient)
    pub fn for_rss() -> Self {
        Self::new(
            "rss",
            RateLimitConfig {
                requests_per_second: 5,
                burst_size: 10,
            },
        )
    }

    /// Create a client for torrent indexers
    pub fn for_indexer() -> Self {
        // Be conservative with indexers
        Self::new(
            "indexer",
            RateLimitConfig {
                requests_per_second: 1,
                burst_size: 3,
            },
        )
    }

    /// Wait for rate limit and make a GET request
    pub async fn get(&self, url: &str) -> Result<Response> {
        self.wait_for_permit().await;
        debug!(client = %self.name, url = %url, "Making rate-limited GET request");

        self.client
            .get(url)
            .send()
            .await
            .context("HTTP request failed")
    }

    /// Wait for rate limit and make a GET request with query parameters
    pub async fn get_with_query<T: serde::Serialize + ?Sized>(
        &self,
        url: &str,
        query: &T,
    ) -> Result<Response> {
        self.wait_for_permit().await;
        debug!(client = %self.name, url = %url, "Making rate-limited GET request with query");

        self.client
            .get(url)
            .query(query)
            .send()
            .await
            .context("HTTP request failed")
    }

    /// Wait for rate limit and make a GET request with headers and query parameters
    pub async fn get_with_headers_and_query<T: serde::Serialize + ?Sized>(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        query: &T,
    ) -> Result<Response> {
        self.wait_for_permit().await;
        debug!(client = %self.name, url = %url, "Making rate-limited GET request with headers and query");

        let mut request = self.client.get(url);
        for (key, value) in headers {
            request = request.header(*key, *value);
        }
        request
            .query(query)
            .send()
            .await
            .context("HTTP request failed")
    }

    /// Get a reference to the underlying client for custom requests
    /// (caller is responsible for calling wait_for_permit first)
    pub fn inner(&self) -> &Client {
        &self.client
    }

    /// Wait for a rate limit permit
    pub async fn wait_for_permit(&self) {
        self.limiter.until_ready().await;
    }
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial backoff duration
    pub initial_interval: Duration,
    /// Maximum backoff duration
    pub max_interval: Duration,
    /// Multiplier for exponential backoff
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_interval: Duration::from_millis(500),
            max_interval: Duration::from_secs(30),
            multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Create an ExponentialBackoff from this config
    pub fn to_backoff(&self) -> ExponentialBackoff {
        ExponentialBackoff {
            initial_interval: self.initial_interval,
            max_interval: self.max_interval,
            multiplier: self.multiplier,
            max_elapsed_time: Some(Duration::from_secs(120)),
            ..Default::default()
        }
    }
}

/// Execute an async operation with retry logic
pub async fn retry_async<T, E, Fut, F>(
    operation: F,
    config: &RetryConfig,
    operation_name: &str,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempts = 0;
    let mut backoff = config.to_backoff();

    loop {
        attempts += 1;
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempts >= config.max_retries {
                    warn!(
                        operation = %operation_name,
                        attempts = attempts,
                        error = %e,
                        "Operation failed after max retries"
                    );
                    return Err(e);
                }

                if let Some(duration) = backoff.next_backoff() {
                    let retry_ms: u128 = duration.as_millis();
                    warn!(
                        operation = %operation_name,
                        attempt = attempts,
                        error = %e,
                        retry_in_ms = retry_ms,
                        "Operation failed, retrying"
                    );
                    tokio::time::sleep(duration).await;
                } else {
                    return Err(e);
                }
            }
        }
    }
}

/// Helper trait for retrying HTTP responses that might indicate rate limiting
pub trait ResponseExt {
    /// Check if the response indicates rate limiting (429)
    fn is_rate_limited(&self) -> bool;

    /// Check if the response indicates a transient error that should be retried
    fn is_transient_error(&self) -> bool;
}

impl ResponseExt for Response {
    fn is_rate_limited(&self) -> bool {
        self.status().as_u16() == 429
    }

    fn is_transient_error(&self) -> bool {
        let status = self.status().as_u16();
        // 429 (rate limit), 500-599 (server errors), 408 (timeout)
        status == 429 || status == 408 || (500..600).contains(&status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_second, 2);
        assert_eq!(config.burst_size, 5);
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
    }
}
