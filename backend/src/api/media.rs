//! Media streaming API endpoints
//!
//! Provides HTTP endpoints for streaming media files to cast devices
//! and browser-based playback with Range header support.

use std::io::SeekFrom;
use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, RANGE,
};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde::Deserialize;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::db::Database;

/// App state for media routes
#[derive(Clone)]
pub struct MediaState {
    pub db: Database,
}

/// Create media routes
pub fn media_routes() -> Router<MediaState> {
    Router::new()
        .route("/media/{file_id}/stream", get(stream_media))
        .route("/media/{file_id}/info", get(media_info))
}

/// Query params for stream endpoint
#[derive(Debug, Deserialize)]
pub struct StreamParams {
    /// Transcode to this format if needed
    pub transcode: Option<String>,
    /// Target quality (e.g., "1080p", "720p")
    pub quality: Option<String>,
}

/// Stream a media file with Range header support
async fn stream_media(
    State(state): State<MediaState>,
    AxumPath(file_id): AxumPath<Uuid>,
    headers: HeaderMap,
    Query(_params): Query<StreamParams>,
) -> Result<Response, StatusCode> {
    // Get file info from database
    let media_file = state
        .db
        .media_files()
        .get_by_id(file_id)
        .await
        .map_err(|e| {
            error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            warn!("Media file not found: {}", file_id);
            StatusCode::NOT_FOUND
        })?;

    let path = Path::new(&media_file.path);

    // Check if file exists
    if !path.exists() {
        warn!("Media file path does not exist: {}", media_file.path);
        return Err(StatusCode::NOT_FOUND);
    }

    // Open the file
    let mut file = File::open(path).await.map_err(|e| {
        error!("Failed to open media file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get file metadata
    let metadata = file.metadata().await.map_err(|e| {
        error!("Failed to get file metadata: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let file_size = metadata.len();

    // Determine content type
    let content_type = get_content_type(&media_file.path);

    // Parse Range header
    let range = headers
        .get(RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| parse_range_header(s, file_size));

    match range {
        Some((start, end)) => {
            // Partial content response
            let length = end - start + 1;

            debug!(
                "Serving partial content: bytes {}-{}/{} for {}",
                start, end, file_size, file_id
            );

            // Seek to start position
            file.seek(SeekFrom::Start(start)).await.map_err(|e| {
                error!("Failed to seek: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            // Create a limited reader
            let limited_file = file.take(length);
            let stream = ReaderStream::new(limited_file);
            let body = Body::from_stream(stream);

            let content_range = format!("bytes {}-{}/{}", start, end, file_size);

            Ok(Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(CONTENT_TYPE, content_type)
                .header(CONTENT_LENGTH, length.to_string())
                .header(CONTENT_RANGE, content_range)
                .header(ACCEPT_RANGES, "bytes")
                .header(CACHE_CONTROL, "public, max-age=3600")
                .body(body)
                .unwrap())
        }
        None => {
            // Full file response
            debug!("Serving full file: {} bytes for {}", file_size, file_id);

            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, content_type)
                .header(CONTENT_LENGTH, file_size.to_string())
                .header(ACCEPT_RANGES, "bytes")
                .header(CACHE_CONTROL, "public, max-age=3600")
                .body(body)
                .unwrap())
        }
    }
}

/// Get media file information
async fn media_info(
    State(state): State<MediaState>,
    AxumPath(file_id): AxumPath<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let media_file = state
        .db
        .media_files()
        .get_by_id(file_id)
        .await
        .map_err(|e| {
            error!("Database error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let path = Path::new(&media_file.path);
    let exists = path.exists();
    let content_type = get_content_type(&media_file.path);

    // Check Chromecast compatibility
    let chromecast_compatible = is_chromecast_compatible(
        media_file.container.as_deref(),
        media_file.video_codec.as_deref(),
        media_file.audio_codec.as_deref(),
    );

    let info = serde_json::json!({
        "id": media_file.id.to_string(),
        "path": media_file.path,
        "exists": exists,
        "size_bytes": media_file.size_bytes,
        "content_type": content_type,
        "container": media_file.container,
        "video_codec": media_file.video_codec,
        "audio_codec": media_file.audio_codec,
        "resolution": media_file.resolution,
        "width": media_file.width,
        "height": media_file.height,
        "duration": media_file.duration,
        "is_hdr": media_file.is_hdr,
        "hdr_type": media_file.hdr_type,
        "chromecast_compatible": chromecast_compatible,
    });

    Ok(axum::Json(info))
}

/// Parse HTTP Range header
fn parse_range_header(header: &str, file_size: u64) -> Option<(u64, u64)> {
    // Format: "bytes=start-end" or "bytes=start-"
    let header = header.strip_prefix("bytes=")?;
    let parts: Vec<&str> = header.split('-').collect();

    if parts.len() != 2 {
        return None;
    }

    let start: u64 = parts[0].parse().ok()?;
    let end: u64 = if parts[1].is_empty() {
        file_size - 1
    } else {
        parts[1].parse().ok()?
    };

    // Validate range
    if start >= file_size || end >= file_size || start > end {
        return None;
    }

    Some((start, end))
}

/// Get content type from file path
fn get_content_type(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "wmv" => "video/x-ms-wmv",
        "ts" => "video/mp2t",
        "m3u8" => "application/vnd.apple.mpegurl",
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "aac" => "audio/aac",
        "m4a" => "audio/mp4",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

/// Check if media file is Chromecast compatible
/// Chromecast supports:
/// - Containers: MP4, WebM
/// - Video: H.264 (up to level 4.2), VP8, VP9
/// - Audio: AAC, MP3, Opus, Vorbis, FLAC
fn is_chromecast_compatible(
    container: Option<&str>,
    video_codec: Option<&str>,
    audio_codec: Option<&str>,
) -> bool {
    let container_ok = match container {
        Some(c) => {
            let c = c.to_lowercase();
            c.contains("mp4") || c.contains("m4v") || c.contains("webm")
        }
        None => false,
    };

    let video_ok = match video_codec {
        Some(v) => {
            let v = v.to_lowercase();
            v.contains("h264") || v.contains("avc") || v.contains("vp8") || v.contains("vp9")
        }
        None => true, // No video is OK (audio only)
    };

    let audio_ok = match audio_codec {
        Some(a) => {
            let a = a.to_lowercase();
            a.contains("aac")
                || a.contains("mp3")
                || a.contains("opus")
                || a.contains("vorbis")
                || a.contains("flac")
        }
        None => true, // No audio is OK (video only, though uncommon)
    };

    container_ok && video_ok && audio_ok
}
