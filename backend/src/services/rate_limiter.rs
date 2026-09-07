//! Rate limiting and retry logic for external API calls
//!
//! Provides rate-limited HTTP clients and retry utilities to prevent
//! overwhelming external APIs and handle transient failures gracefully.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
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
    client: Option<Client>,
    initialization_error: Option<Arc<str>>,
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

        let (client, initialization_error) = match crate::services::http_client::outbound_client(
            crate::services::http_client::OutboundHttpProfile::Metadata,
        ) {
            Ok(client) => (Some(client), None),
            Err(error) => (None, Some(Arc::<str>::from(error.to_string()))),
        };
        Self {
            client,
            initialization_error,
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

    /// Create a client for torrent indexers
    pub fn for_indexer() -> Self {
        // Be conservative with indexers
        Self::new(
            "indexer",
            RateLimitConfig {
                requests_per_second: 1,
                burst_size: 1,
            },
        )
    }

    /// Create a client for torrent indexers with a fixed minimum delay between requests.
    pub fn for_indexer_with_request_delay(request_delay: Duration) -> Self {
        let (quota, interval_error) = match Quota::with_period(request_delay) {
            Some(quota) => (quota.allow_burst(NonZeroU32::MIN), None),
            None => (
                Quota::per_second(NonZeroU32::MIN),
                Some(Arc::<str>::from(
                    "Indexer request delay must be greater than zero",
                )),
            ),
        };

        let limiter = Arc::new(RateLimiter::direct(quota));
        let (client, client_error) = match crate::services::http_client::outbound_client(
            crate::services::http_client::OutboundHttpProfile::Indexer,
        ) {
            Ok(client) => (Some(client), None),
            Err(error) => (None, Some(Arc::<str>::from(error.to_string()))),
        };
        Self {
            client,
            initialization_error: interval_error.or(client_error),
            limiter,
            name: "indexer".to_string(),
        }
    }

    /// Wait for rate limit and make a GET request
    pub async fn get(&self, url: &str) -> Result<Response> {
        self.wait_for_permit().await;
        let target = crate::services::http_client::sanitized_request_target(url);
        debug!(client = %self.name, target, "Making rate-limited GET request");

        self.client()?
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
        let target = crate::services::http_client::sanitized_request_target(url);
        debug!(client = %self.name, target, "Making rate-limited GET request with query");

        self.client()?
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
        let target = crate::services::http_client::sanitized_request_target(url);
        debug!(client = %self.name, target, "Making rate-limited GET request with headers and query");

        let mut request = self.client()?.get(url);
        for (key, value) in headers {
            request = request.header(*key, *value);
        }
        request
            .query(query)
            .send()
            .await
            .context("HTTP request failed")
    }

    fn client(&self) -> Result<&Client> {
        self.client.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "{}",
                self.initialization_error
                    .as_deref()
                    .unwrap_or("HTTP client is unavailable")
            )
        })
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
    fn delay_after(&self, failed_attempt: u32) -> Duration {
        let exponent = failed_attempt.saturating_sub(1).min(63);
        let multiplier = if self.multiplier.is_finite() && self.multiplier >= 1.0 {
            self.multiplier
        } else {
            1.0
        };
        let seconds = self.initial_interval.as_secs_f64() * multiplier.powf(f64::from(exponent));
        Duration::try_from_secs_f64(seconds)
            .unwrap_or(self.max_interval)
            .min(self.max_interval)
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

    loop {
        attempts += 1;
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempts >= config.max_retries {
                    warn!(
                        operation = %operation_name,
                        attempts = attempts,
                        error = %format!("{:#}", e),
                        "Operation failed after max retries: operation='{}', attempts={}, error={:#}",
                        operation_name,
                        attempts,
                        e
                    );
                    return Err(e);
                }

                let duration = config.delay_after(attempts);
                let retry_ms: u128 = duration.as_millis();
                warn!(
                    operation = %operation_name,
                    attempt = attempts,
                    error = %format!("{:#}", e),
                    retry_in_ms = retry_ms,
                    "Operation failed, retrying: operation='{}', attempt={}, retry_in_ms={}, error={:#}",
                    operation_name,
                    attempts,
                    retry_ms,
                    e
                );
                tokio::time::sleep(duration).await;
            }
        }
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
        assert_eq!(config.delay_after(1), Duration::from_millis(500));
        assert_eq!(config.delay_after(2), Duration::from_secs(1));
    }

    #[test]
    fn retry_delay_caps_at_max_interval() {
        let config = RetryConfig {
            initial_interval: Duration::from_secs(10),
            max_interval: Duration::from_secs(30),
            multiplier: 10.0,
            ..Default::default()
        };
        assert_eq!(config.delay_after(3), Duration::from_secs(30));
    }
}
