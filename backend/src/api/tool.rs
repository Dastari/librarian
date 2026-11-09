//! `/internal/tool/<token>/<name>`: a file on a share, served to a tool on this machine.
//!
//! Not part of the public API: it is not under `/api`, takes no login, and answers only a peer on
//! the loopback address that holds a token the backend issued for one file (see
//! `services::tool_url`). It supports `HEAD` and `Range` so ffmpeg and ffprobe can seek.

use std::net::SocketAddr;

use axum::Router;
use axum::body::Body;
use axum::extract::{ConnectInfo, Path as AxumPath};
use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, RANGE,
};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::Response;
use axum::routing::get;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;
use tracing::{debug, warn};

use crate::api::media::parse_range_header;
use crate::services::library_storage::{OpenedRead, StorageErrorKind, library_storage};
use crate::services::tool_url::{ToolUrlService, tool_urls};

/// The route, to merge into the root router.
pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new().route("/internal/tool/{token}/{name}", get(serve))
}

async fn serve(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    method: Method,
    AxumPath((token, name)): AxumPath<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    respond(tool_urls(), peer, &method, &token, &name, &headers).await
}

/// Answer one request. Separate from the extractors so it can be tested against a service of
/// its own.
async fn respond(
    service: &ToolUrlService,
    peer: SocketAddr,
    method: &Method,
    token: &str,
    name: &str,
    headers: &HeaderMap,
) -> Result<Response, StatusCode> {
    if !service.peer_allowed(peer.ip()) {
        warn!(peer = %peer, "Refused a tool URL request from a peer that is not this machine");
        return Err(StatusCode::FORBIDDEN);
    }
    let path = service.resolve(token).ok_or(StatusCode::NOT_FOUND)?;
    let OpenedRead {
        reader: mut file,
        meta,
    } = library_storage()
        .open_read(&path)
        .await
        .map_err(|error| {
            warn!(file = %path, error = %error, "A tool could not open its file: file={}, error={}", path, error);
            match error.kind() {
                StorageErrorKind::NotFound => StatusCode::NOT_FOUND,
                StorageErrorKind::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;
    let size = meta.size;
    let range = match headers.get(RANGE) {
        Some(value) => {
            let parsed = value
                .to_str()
                .ok()
                .and_then(|value| parse_range_header(value, size));
            if headers.get_all(RANGE).iter().count() != 1 || parsed.is_none() {
                return Ok(Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(CONTENT_RANGE, format!("bytes */{size}"))
                    .header(ACCEPT_RANGES, "bytes")
                    .header(CONTENT_LENGTH, "0")
                    .body(Body::empty())
                    .expect("valid range rejection"));
            }
            parsed
        }
        None => None,
    };
    debug!(file = %path, name, range = ?range, size, "Serving a file to a tool: file={}, size={}", path, size);
    let head = method == Method::HEAD;
    let builder = Response::builder()
        .header(CONTENT_TYPE, "application/octet-stream")
        .header(ACCEPT_RANGES, "bytes")
        .header(CACHE_CONTROL, "no-store");
    match range {
        Some((start, end)) => {
            let length = end - start + 1;
            let body = if head {
                Body::empty()
            } else {
                file.seek(std::io::SeekFrom::Start(start))
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                Body::from_stream(ReaderStream::new(file.take(length)))
            };
            builder
                .status(StatusCode::PARTIAL_CONTENT)
                .header(CONTENT_LENGTH, length.to_string())
                .header(CONTENT_RANGE, format!("bytes {start}-{end}/{size}"))
                .body(body)
        }
        None => {
            let body = if head {
                Body::empty()
            } else {
                Body::from_stream(ReaderStream::new(file))
            };
            builder
                .status(StatusCode::OK)
                .header(CONTENT_LENGTH, size.to_string())
                .body(body)
        }
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::library_storage::StoragePath;
    use std::net::IpAddr;

    /// Serve `respond` on a real socket so a real HTTP client exercises it, with a service of
    /// its own so tests do not share tokens.
    async fn start(service: &'static ToolUrlService) -> u16 {
        async fn handler(
            axum::extract::State(service): axum::extract::State<&'static ToolUrlService>,
            ConnectInfo(peer): ConnectInfo<SocketAddr>,
            method: Method,
            AxumPath((token, name)): AxumPath<(String, String)>,
            headers: HeaderMap,
        ) -> Result<Response, StatusCode> {
            respond(service, peer, &method, &token, &name, &headers).await
        }
        let app = Router::new()
            .route("/internal/tool/{token}/{name}", get(handler))
            .with_state(service);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
        port
    }

    fn leaked_service(port_hint: u16) -> &'static ToolUrlService {
        let service = Box::leak(Box::new(ToolUrlService::default()));
        service.configure(IpAddr::from([127, 0, 0, 1]), port_hint);
        service
    }

    async fn setup(content: &[u8]) -> (String, tempfile::TempDir, u16) {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("Film.mkv");
        std::fs::write(&file, content).unwrap();
        let service = leaked_service(0);
        let port = start(service).await;
        service.configure(IpAddr::from([127, 0, 0, 1]), port);
        let url = service
            .issue_url(&StoragePath::new(file.to_string_lossy().into_owned()))
            .unwrap();
        (url, dir, port)
    }

    #[tokio::test]
    async fn a_range_request_returns_exactly_those_bytes() {
        let content: Vec<u8> = (0..=255u8).cycle().take(10_000).collect();
        let (url, _dir, _) = setup(&content).await;
        let client = reqwest::Client::new();

        let response = client
            .get(&url)
            .header("Range", "bytes=100-199")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 206);
        assert_eq!(response.headers()["content-range"], "bytes 100-199/10000");
        assert_eq!(response.bytes().await.unwrap().as_ref(), &content[100..200]);

        let response = client
            .get(&url)
            .header("Range", "bytes=-50")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 206);
        assert_eq!(response.bytes().await.unwrap().as_ref(), &content[9_950..]);

        let whole = client.get(&url).send().await.unwrap();
        assert_eq!(whole.status(), 200);
        assert_eq!(whole.headers()["accept-ranges"], "bytes");
        assert_eq!(whole.bytes().await.unwrap().as_ref(), content.as_slice());
    }

    #[tokio::test]
    async fn head_reports_the_size_without_a_body() {
        let (url, _dir, _) = setup(&[7u8; 4_321]).await;
        let response = reqwest::Client::new().head(&url).send().await.unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["content-length"], "4321");
        assert_eq!(response.headers()["accept-ranges"], "bytes");
        assert!(response.bytes().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn an_unsatisfiable_range_is_refused() {
        let (url, _dir, _) = setup(&[1u8; 100]).await;
        let response = reqwest::Client::new()
            .get(&url)
            .header("Range", "bytes=500-600")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 416);
        assert_eq!(response.headers()["content-range"], "bytes */100");
    }

    #[tokio::test]
    async fn an_unknown_token_is_not_found() {
        let (url, _dir, port) = setup(b"x").await;
        let bad = format!(
            "http://127.0.0.1:{port}/internal/tool/{}/Film.mkv",
            "0".repeat(64)
        );
        let client = reqwest::Client::new();
        assert_eq!(client.get(&bad).send().await.unwrap().status(), 404);
        assert_eq!(client.get(&url).send().await.unwrap().status(), 200);
    }

    #[tokio::test]
    async fn a_missing_file_is_not_found_and_a_foreign_peer_is_refused() {
        let service = leaked_service(1);
        let url = service
            .issue_url(&StoragePath::new("/definitely/not/here.mkv"))
            .unwrap();
        let token = url.split('/').nth(5).unwrap().to_string();
        let headers = HeaderMap::new();
        let loopback: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        let foreign: SocketAddr = "10.0.1.50:5000".parse().unwrap();
        assert_eq!(
            respond(
                service,
                loopback,
                &Method::GET,
                &token,
                "here.mkv",
                &headers
            )
            .await
            .unwrap_err(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            respond(service, foreign, &Method::GET, &token, "here.mkv", &headers)
                .await
                .unwrap_err(),
            StatusCode::FORBIDDEN
        );
    }
}
