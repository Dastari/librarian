//! Embedded frontend asset serving for release builds.

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use mime_guess::MimeGuess;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../frontend/dist"]
struct FrontendAssets;

fn content_type_for(path: &str) -> HeaderValue {
    let mime: MimeGuess = mime_guess::from_path(path);
    let value = mime.first_or_octet_stream().to_string();
    HeaderValue::from_str(&value)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"))
}

fn asset_response(path: &str) -> Option<Response> {
    FrontendAssets::get(path).map(|asset| {
        let mut response = Response::new(Body::from(asset.data.into_owned()));
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, content_type_for(path));
        response
    })
}

pub async fn embedded_fallback(uri: Uri) -> impl IntoResponse {
    let raw_path = uri.path().trim_start_matches('/');
    let path = if raw_path.is_empty() {
        "index.html"
    } else {
        raw_path
    };

    if let Some(response) = asset_response(path) {
        return response;
    }

    let is_asset_request = path.contains('.');
    if !is_asset_request {
        if let Some(response) = asset_response("index.html") {
            return response;
        }
    }

    StatusCode::NOT_FOUND.into_response()
}
