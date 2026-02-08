//! Media streaming API endpoints
//!
//! Provides HTTP endpoints for browser playback with Range header support.

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

use crate::AppState;

/// Create media routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/media/{file_id}/stream", get(stream_media))
        .route("/media/{file_id}/info", get(media_info))
}

/// Query params for stream endpoint
#[derive(Debug, Deserialize)]
pub struct StreamParams {
    /// Transcode to this format if needed (placeholder for future support)
    pub transcode: Option<String>,
    /// Target quality (e.g., "1080p", "720p") (placeholder for future support)
    pub quality: Option<String>,
}

#[derive(Debug, Clone)]
struct MediaFileRecord {
    id: String,
    path: String,
    size: i64,
    container: Option<String>,
    video_codec: Option<String>,
    audio_codec: Option<String>,
    resolution: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    duration: Option<i32>,
    is_hdr: bool,
    hdr_type: Option<String>,
}

/// Stream a media file with Range header support
async fn stream_media(
    State(state): State<AppState>,
    AxumPath(file_id): AxumPath<String>,
    headers: HeaderMap,
    Query(params): Query<StreamParams>,
) -> Result<Response, StatusCode> {
    let media_file = fetch_media_file(&state, &file_id).await?;
    let path = Path::new(&media_file.path);

    if let Some(ref transcode) = params.transcode {
        debug!(
            file_id = %file_id,
            transcode = %transcode,
            "Stream request included transcode hint (not yet implemented)"
        );
    }
    if let Some(ref quality) = params.quality {
        debug!(
            file_id = %file_id,
            quality = %quality,
            "Stream request included quality hint (not yet implemented)"
        );
    }

    if !path.exists() {
        warn!(
            media_file_id = %file_id,
            media_file_path = %media_file.path,
            "Media stream path does not exist: media_file_id={}, path={}",
            file_id,
            media_file.path
        );
        return Err(StatusCode::NOT_FOUND);
    }

    let mut file = File::open(path).await.map_err(|e| {
        error!(
            media_file_id = %file_id,
            media_file_path = %media_file.path,
            error = %e,
            "Failed opening media file for stream: media_file_id={}, path={}, error={}",
            file_id,
            media_file.path,
            e
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let metadata = file.metadata().await.map_err(|e| {
        error!(
            media_file_id = %file_id,
            media_file_path = %media_file.path,
            error = %e,
            "Failed reading media metadata for stream: media_file_id={}, path={}, error={}",
            file_id,
            media_file.path,
            e
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let file_size = metadata.len();
    let content_type = get_content_type(&media_file.path);

    let range = headers
        .get(RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| parse_range_header(s, file_size));

    match range {
        Some((start, end)) => {
            let length = end - start + 1;
            debug!(
                media_file_id = %file_id,
                start = start,
                end = end,
                length = length,
                file_size = file_size,
                "Serving partial media content: media_file_id={}, bytes={}-{}, total_size={}",
                file_id,
                start,
                end,
                file_size
            );

            file.seek(SeekFrom::Start(start)).await.map_err(|e| {
                error!(
                    media_file_id = %file_id,
                    error = %e,
                    "Failed seeking media stream: media_file_id={}, error={}",
                    file_id,
                    e
                );
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

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
                .expect("valid partial content response"))
        }
        None => {
            debug!(
                media_file_id = %file_id,
                file_size = file_size,
                "Serving full media content: media_file_id={}, total_size={}",
                file_id,
                file_size
            );

            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, content_type)
                .header(CONTENT_LENGTH, file_size.to_string())
                .header(ACCEPT_RANGES, "bytes")
                .header(CACHE_CONTROL, "public, max-age=3600")
                .body(body)
                .expect("valid full content response"))
        }
    }
}

/// Get media file information
async fn media_info(
    State(state): State<AppState>,
    AxumPath(file_id): AxumPath<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let media_file = fetch_media_file(&state, &file_id).await?;
    let path = Path::new(&media_file.path);
    let exists = path.exists();
    let content_type = get_content_type(&media_file.path);

    let chromecast_compatible = is_chromecast_compatible(
        media_file.container.as_deref(),
        media_file.video_codec.as_deref(),
        media_file.audio_codec.as_deref(),
    );

    let info = serde_json::json!({
        "id": media_file.id,
        "path": media_file.path,
        "exists": exists,
        "size_bytes": media_file.size,
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

async fn fetch_media_file(state: &AppState, file_id: &str) -> Result<MediaFileRecord, StatusCode> {
    let row = sqlx::query_as::<
        _,
        (
            String,
            String,
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<i32>,
            Option<i32>,
            Option<i32>,
            bool,
            Option<String>,
        ),
    >(
        r#"SELECT id, path, size, container, video_codec, audio_codec, resolution, width, height, duration, is_hdr, hdr_type
           FROM media_files
           WHERE id = ?1
           LIMIT 1"#,
    )
    .bind(file_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        error!(
            media_file_id = %file_id,
            error = %e,
            "Failed querying media file for stream/info: media_file_id={}, error={}",
            file_id,
            e
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let Some((
        id,
        path,
        size,
        container,
        video_codec,
        audio_codec,
        resolution,
        width,
        height,
        duration,
        is_hdr,
        hdr_type,
    )) = row
    else {
        warn!(
            media_file_id = %file_id,
            "Media file not found for stream/info: media_file_id={}",
            file_id
        );
        return Err(StatusCode::NOT_FOUND);
    };

    Ok(MediaFileRecord {
        id,
        path,
        size,
        container,
        video_codec,
        audio_codec,
        resolution,
        width,
        height,
        duration,
        is_hdr,
        hdr_type,
    })
}

/// Parse HTTP Range header.
fn parse_range_header(header: &str, file_size: u64) -> Option<(u64, u64)> {
    let header = header.strip_prefix("bytes=")?;
    let parts: Vec<&str> = header.split('-').collect();
    if parts.len() != 2 {
        return None;
    }

    let start: u64 = parts[0].parse().ok()?;
    let end: u64 = if parts[1].is_empty() {
        file_size.saturating_sub(1)
    } else {
        parts[1].parse().ok()?
    };

    if start >= file_size || end >= file_size || start > end {
        return None;
    }
    Some((start, end))
}

/// Determine content type from file extension.
fn get_content_type(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "wmv" => "video/x-ms-wmv",
        "mpg" | "mpeg" => "video/mpeg",
        "ts" => "video/mp2t",
        "m2ts" => "video/mp2t",
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "aac" => "audio/aac",
        "m4a" => "audio/mp4",
        "ogg" => "audio/ogg",
        "opus" => "audio/opus",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    }
}

/// Check if media is Chromecast compatible without transcoding.
fn is_chromecast_compatible(
    container: Option<&str>,
    video_codec: Option<&str>,
    audio_codec: Option<&str>,
) -> bool {
    let container_ok = container
        .map(|c| {
            let c = c.to_ascii_lowercase();
            c.contains("mp4") || c.contains("webm") || c.contains("mp3")
        })
        .unwrap_or(false);

    let video_ok = video_codec
        .map(|v| {
            let v = v.to_ascii_lowercase();
            v.contains("h264") || v.contains("avc") || v.contains("vp8") || v.contains("vp9")
        })
        .unwrap_or(true);

    let audio_ok = audio_codec
        .map(|a| {
            let a = a.to_ascii_lowercase();
            a.contains("aac") || a.contains("mp3") || a.contains("opus") || a.contains("vorbis")
        })
        .unwrap_or(true);

    container_ok && video_ok && audio_ok
}
