//! Central policy for outbound HTTP clients.
//!
//! Callers choose a profile and may add request-specific headers or DNS
//! pinning, but deadlines, redirect limits, user agent, and pool behavior are
//! defined here. URL telemetry intentionally excludes query strings and user
//! information.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use bytes::{Bytes, BytesMut};
use reqwest::{Client, ClientBuilder, RequestBuilder, Response, Url, redirect};
use serde::de::DeserializeOwned;
use tracing::{debug, warn};

const USER_AGENT: &str = concat!("Librarian/", env!("CARGO_PKG_VERSION"));
pub const METADATA_RESPONSE_LIMIT: usize = 8 * 1024 * 1024;
pub const INDEXER_RESPONSE_LIMIT: usize = 16 * 1024 * 1024;
pub const DOWNLOAD_RESPONSE_LIMIT: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutboundHttpProfile {
    Metadata,
    Indexer,
    Artwork,
    LocalService,
}

impl OutboundHttpProfile {
    fn connect_timeout(self) -> Duration {
        match self {
            Self::LocalService => Duration::from_secs(2),
            _ => Duration::from_secs(5),
        }
    }

    fn request_timeout(self) -> Duration {
        match self {
            Self::Artwork => Duration::from_secs(12),
            Self::LocalService => Duration::from_secs(10),
            Self::Metadata | Self::Indexer => Duration::from_secs(30),
        }
    }

    fn redirect_policy(self) -> redirect::Policy {
        match self {
            Self::Artwork | Self::LocalService => redirect::Policy::none(),
            Self::Metadata | Self::Indexer => redirect::Policy::limited(3),
        }
    }
}

pub fn outbound_client_builder(profile: OutboundHttpProfile) -> ClientBuilder {
    Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(profile.connect_timeout())
        .timeout(profile.request_timeout())
        .pool_idle_timeout(Duration::from_secs(30))
        .redirect(profile.redirect_policy())
        .tcp_nodelay(true)
}

pub fn outbound_client(profile: OutboundHttpProfile) -> Result<Client> {
    outbound_client_builder(profile)
        .build()
        .with_context(|| format!("Failed to build {profile:?} outbound HTTP client"))
}

fn retryable_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::REQUEST_TIMEOUT
        || status.is_server_error()
}

fn retryable_error(error: &reqwest::Error) -> bool {
    error.is_connect() || error.is_timeout()
}

/// Send a cloneable request with the project's retry and telemetry policy.
///
/// Retries are deliberately limited to transient connection/timeout failures,
/// 408/429, and 5xx responses. Callers must not use this helper for
/// non-idempotent requests unless the remote operation has an idempotency key.
pub async fn send_with_policy(
    request: RequestBuilder,
    provider: &str,
    operation: &str,
) -> Result<Response> {
    const MAX_ATTEMPTS: u32 = 3;
    let target = request
        .try_clone()
        .and_then(|builder| builder.build().ok())
        .map(|request| sanitized_request_target(request.url().as_str()))
        .unwrap_or_else(|| "unavailable".to_string());
    let started = Instant::now();

    for attempt in 1..=MAX_ATTEMPTS {
        let attempt_request = request
            .try_clone()
            .context("Outbound request body cannot be retried safely")?;
        match attempt_request.send().await {
            Ok(response) if attempt < MAX_ATTEMPTS && retryable_status(response.status()) => {
                warn!(
                    provider,
                    operation,
                    target,
                    attempt,
                    status = response.status().as_u16(),
                    "Transient outbound HTTP response; retrying"
                );
            }
            Ok(response) => {
                debug!(
                    provider,
                    operation,
                    target,
                    attempts = attempt,
                    status = response.status().as_u16(),
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    "Outbound HTTP request completed"
                );
                return Ok(response);
            }
            Err(error) if attempt < MAX_ATTEMPTS && retryable_error(&error) => {
                warn!(
                    provider,
                    operation,
                    target,
                    attempt,
                    reason = if error.is_timeout() {
                        "timeout"
                    } else {
                        "connect"
                    },
                    "Transient outbound HTTP failure; retrying"
                );
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("Outbound HTTP request failed ({provider}/{operation})")
                });
            }
        }

        // Full jitter keeps simultaneous provider failures from synchronizing.
        let ceiling_ms = 250_u64.saturating_mul(1_u64 << (attempt - 1));
        let jitter_ms = rand::random::<u64>() % ceiling_ms.max(1);
        tokio::time::sleep(Duration::from_millis(jitter_ms)).await;
    }

    anyhow::bail!("Outbound HTTP retry policy exhausted without a terminal result")
}

/// Read a response body without allowing a peer to allocate unbounded memory.
pub async fn response_bytes_limited(mut response: Response, max_bytes: usize) -> Result<Bytes> {
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        anyhow::bail!("HTTP_RESPONSE_TOO_LARGE");
    }

    let initial_capacity = response
        .content_length()
        .and_then(|length| usize::try_from(length).ok())
        .unwrap_or(8 * 1024)
        .min(max_bytes);
    let mut body = BytesMut::with_capacity(initial_capacity);
    while let Some(chunk) = response
        .chunk()
        .await
        .context("Failed while reading outbound HTTP response")?
    {
        if body.len().saturating_add(chunk.len()) > max_bytes {
            anyhow::bail!("HTTP_RESPONSE_TOO_LARGE");
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body.freeze())
}

pub async fn response_text_limited(response: Response, max_bytes: usize) -> Result<String> {
    let body = response_bytes_limited(response, max_bytes).await?;
    String::from_utf8(body.to_vec()).context("Outbound HTTP response was not valid UTF-8")
}

pub async fn response_json_limited<T: DeserializeOwned>(
    response: Response,
    max_bytes: usize,
) -> Result<T> {
    let body = response_bytes_limited(response, max_bytes).await?;
    serde_json::from_slice(&body).context("Outbound HTTP response contained invalid JSON")
}

/// A sanitized telemetry label. It contains scheme/host/port/path, but never
/// URL credentials, query parameters, or fragments.
pub fn sanitized_request_target(input: &str) -> String {
    let Ok(mut url) = Url::parse(input) else {
        return "invalid-url".to_string();
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{response_bytes_limited, sanitized_request_target};

    #[test]
    fn telemetry_target_removes_credentials_query_and_fragment() {
        let target =
            sanitized_request_target("https://user:secret@example.com/a?api_key=secret#part");
        assert_eq!(target, "https://example.com/a");
        assert!(!target.contains("secret"));
        assert!(!target.contains("api_key"));
    }

    #[test]
    fn outbound_clients_are_created_only_by_the_policy_factory() {
        fn visit(path: &Path, violations: &mut Vec<String>) {
            for entry in std::fs::read_dir(path).expect("source directory should be readable") {
                let entry = entry.expect("source entry should be readable");
                let path = entry.path();
                if path.is_dir() {
                    visit(&path, violations);
                    continue;
                }
                if path.extension().and_then(|value| value.to_str()) != Some("rs")
                    || path.ends_with("services/http_client.rs")
                {
                    continue;
                }
                let source =
                    std::fs::read_to_string(&path).expect("Rust source should be readable");
                for (index, line) in source.lines().enumerate() {
                    if line.contains("reqwest::get(")
                        || line.contains("reqwest::Client::new(")
                        || line.contains("reqwest::Client::builder(")
                        || line.contains("= Client::new(")
                        || line.contains("Client::builder()")
                    {
                        violations.push(format!("{}:{}", path.display(), index + 1));
                    }
                }
            }
        }

        let mut violations = Vec::new();
        visit(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut violations,
        );
        assert!(
            violations.is_empty(),
            "bare outbound HTTP clients bypass policy: {violations:?}"
        );
    }

    #[tokio::test]
    async fn response_reader_enforces_declared_and_streamed_limits() {
        use axum::{Router, body::Body, http::Response, routing::get};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener should bind");
        let address = listener.local_addr().expect("test address should exist");
        let app = Router::new().route(
            "/body",
            get(|| async {
                Response::builder()
                    .body(Body::from(vec![b'x'; 32]))
                    .expect("test response should build")
            }),
        );
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test server should run")
        });

        let response = reqwest::get(format!("http://{address}/body"))
            .await
            .expect("test request should succeed");
        let error = response_bytes_limited(response, 16)
            .await
            .expect_err("oversized response must fail");
        assert!(error.to_string().contains("HTTP_RESPONSE_TOO_LARGE"));
        server.abort();
    }
}
