//! Media streaming API endpoints
//!
//! Provides HTTP endpoints for browser playback with Range header support.

use std::io::SeekFrom;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::{ConnectInfo, Path as AxumPath, Query, State};
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
use crate::api::auth_guard::require_authenticated_user;
use crate::graphql::AuthUser;
use crate::graphql::entities::{CastSession, Library, MediaFile};
use crate::services::transcode::TranscodeKind;
use crate::services::transcode::ffmpeg::PLAYLIST_FILENAME;
use crate::services::transcode::session::sanitize_segment_name;

/// Create media routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/media/{file_id}/stream", get(stream_media))
        .route("/media/{file_id}/info", get(media_info))
        .route("/media/{file_id}/hls/playlist.m3u8", get(hls_playlist))
        .route("/media/{file_id}/hls/{segment}", get(hls_segment))
}

/// Query params for stream endpoint
#[derive(Debug, Deserialize)]
pub struct StreamParams {
    /// Short-lived receiver/session/media-bound grant used only by Cast
    /// devices. This is not a user access token and is never persisted.
    #[serde(rename = "castGrant")]
    pub cast_grant: Option<String>,
    /// Cast-owned HLS mode selected by the capability decision.
    #[serde(rename = "castMode")]
    pub cast_mode: Option<String>,
}

#[derive(Debug, Clone)]
struct MediaFileRecord {
    id: String,
    library_id: Option<String>,
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
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    AxumPath(file_id): AxumPath<String>,
    headers: HeaderMap,
    Query(params): Query<StreamParams>,
) -> Result<Response, StatusCode> {
    let media_file = fetch_media_file(&state, &file_id).await?;
    if let Some(grant) = params.cast_grant.as_deref() {
        authorize_cast_grant(&state, grant, &file_id, peer).await?;
    } else {
        authorize_browser_media(&state, &headers, &media_file).await?;
    }
    let path = Path::new(&media_file.path);

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
                .header(CACHE_CONTROL, "private, max-age=3600")
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
                .header(CACHE_CONTROL, "private, max-age=3600")
                .body(body)
                .expect("valid full content response"))
        }
    }
}

/// Get media file information
async fn media_info(
    State(state): State<AppState>,
    AxumPath(file_id): AxumPath<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let media_file = fetch_media_file(&state, &file_id).await?;
    authorize_browser_media(&state, &headers, &media_file).await?;
    let path = Path::new(&media_file.path);
    let exists = path.exists();
    let content_type = get_content_type(&media_file.path);

    let chromecast_compatible = is_chromecast_compatible(
        media_file.container.as_deref(),
        media_file.video_codec.as_deref(),
        media_file.audio_codec.as_deref(),
    );

    // Browser-playback decision (distinct from `chromecast_compatible` above:
    // cast devices often have hardware HEVC decoders a browser tab doesn't,
    // so the two must stay independently computed — see
    // `docs/design.md`'s transcode-decision-matrix Q entry).
    let transcode_decision = needs_transcode(
        media_file.container.as_deref(),
        media_file.video_codec.as_deref(),
        media_file.audio_codec.as_deref(),
    );
    let needs_hls = transcode_decision != TranscodeDecision::None;
    let playback_url = if needs_hls {
        format!("/api/media/{}/hls/playlist.m3u8", media_file.id)
    } else {
        format!("/api/media/{}/stream", media_file.id)
    };

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
        "needs_hls": needs_hls,
        "transcode_decision": transcode_decision.as_str(),
        "playback_url": playback_url,
    });

    Ok(axum::Json(info))
}

/// Serve the HLS playlist for a media file, starting a remux/transcode
/// session if one isn't already running. Same auth guard as `stream_media`
/// (must not regress the audit's auth-on-`/api/media` requirement).
async fn hls_playlist(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    AxumPath(file_id): AxumPath<String>,
    headers: HeaderMap,
    Query(params): Query<StreamParams>,
) -> Result<Response, StatusCode> {
    let media_file = fetch_media_file(&state, &file_id).await?;
    if let Some(grant) = params.cast_grant.as_deref() {
        let session = authorize_cast_grant(&state, grant, &file_id, peer).await?;
        let requested_mode = params
            .cast_mode
            .as_deref()
            .map(str::to_ascii_uppercase)
            .ok_or(StatusCode::BAD_REQUEST)?;
        if session.playback_decision.as_deref() != Some(requested_mode.as_str()) {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        authorize_browser_media(&state, &headers, &media_file).await?;
    }
    let path = Path::new(&media_file.path);
    if !path.exists() {
        warn!(
            media_file_id = %file_id,
            media_file_path = %media_file.path,
            "HLS playlist requested but source path does not exist: media_file_id={}, path={}",
            file_id,
            media_file.path
        );
        return Err(StatusCode::NOT_FOUND);
    }

    // A client that explicitly hits this endpoint has already decided (via
    // `media_info`'s `needsHls` hint) that HLS is needed; a `None` decision
    // here just means "codecs are fine, only the container needs repackaging"
    // which the (cheap, lossless) remux path handles too.
    let kind = match (
        params.cast_grant.as_deref(),
        params.cast_mode.as_deref().map(str::to_ascii_lowercase),
    ) {
        (Some(_), Some(mode)) if mode == "transcode" => TranscodeKind::Transcode,
        (Some(_), Some(mode)) if mode == "remux" => TranscodeKind::Remux,
        (Some(_), _) => return Err(StatusCode::BAD_REQUEST),
        (None, _) => match needs_transcode(
            media_file.container.as_deref(),
            media_file.video_codec.as_deref(),
            media_file.audio_codec.as_deref(),
        ) {
            TranscodeDecision::Transcode => TranscodeKind::Transcode,
            TranscodeDecision::Remux | TranscodeDecision::None => TranscodeKind::Remux,
        },
    };

    let transcode = state.services.get_transcode().await.ok_or_else(|| {
        error!(
            media_file_id = %file_id,
            "Transcode service unavailable for HLS playlist request: media_file_id={}",
            file_id
        );
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    let output_dir = transcode
        .get_or_start_session(&file_id, path, kind)
        .await
        .map_err(|e| {
            error!(
                media_file_id = %file_id,
                error = %e,
                "Failed to start HLS session: media_file_id={}, error={}",
                file_id,
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let playlist_path = output_dir.join(PLAYLIST_FILENAME);
    if !wait_for_nonempty_file(
        &playlist_path,
        Duration::from_millis(250),
        Duration::from_secs(20),
    )
    .await
    {
        warn!(
            media_file_id = %file_id,
            playlist_path = %playlist_path.display(),
            "Timed out waiting for HLS playlist to be generated: media_file_id={}",
            file_id
        );
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let mut bytes = tokio::fs::read(&playlist_path).await.map_err(|e| {
        error!(
            media_file_id = %file_id,
            playlist_path = %playlist_path.display(),
            error = %e,
            "Failed reading HLS playlist: media_file_id={}, error={}",
            file_id,
            e
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    if let Some(grant) = params.cast_grant.as_deref() {
        let playlist = String::from_utf8(bytes).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let encoded_grant = urlencoding::encode(grant);
        bytes = playlist
            .lines()
            .map(|line| {
                if line.starts_with('#') || line.trim().is_empty() {
                    line.to_string()
                } else {
                    format!("{line}?castGrant={encoded_grant}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/vnd.apple.mpegurl")
        .header(CACHE_CONTROL, "private, no-cache")
        .body(Body::from(bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Serve one `.ts` segment for an already-started HLS session. Same auth
/// guard as `stream_media`/`hls_playlist`.
async fn hls_segment(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    AxumPath((file_id, segment)): AxumPath<(String, String)>,
    headers: HeaderMap,
    Query(params): Query<StreamParams>,
) -> Result<Response, StatusCode> {
    let media_file = fetch_media_file(&state, &file_id).await?;
    if let Some(grant) = params.cast_grant.as_deref() {
        authorize_cast_grant(&state, grant, &file_id, peer).await?;
    } else {
        authorize_browser_media(&state, &headers, &media_file).await?;
    }

    let Some(segment_name) = sanitize_segment_name(&segment) else {
        warn!(
            media_file_id = %file_id,
            segment = %segment,
            "Rejected HLS segment request with unsafe name: media_file_id={}, segment={}",
            file_id,
            segment
        );
        return Err(StatusCode::BAD_REQUEST);
    };

    let transcode = state
        .services
        .get_transcode()
        .await
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let Some(output_dir) = transcode.touch_session(&file_id).await else {
        warn!(
            media_file_id = %file_id,
            "HLS segment requested with no active transcode session: media_file_id={}",
            file_id
        );
        return Err(StatusCode::NOT_FOUND);
    };

    let segment_path = output_dir.join(segment_name);
    if !wait_for_nonempty_file(
        &segment_path,
        Duration::from_millis(200),
        Duration::from_secs(15),
    )
    .await
    {
        warn!(
            media_file_id = %file_id,
            segment = %segment_name,
            "Timed out waiting for HLS segment to be generated: media_file_id={}, segment={}",
            file_id,
            segment_name
        );
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let bytes = tokio::fs::read(&segment_path).await.map_err(|e| {
        error!(
            media_file_id = %file_id,
            segment = %segment_name,
            error = %e,
            "Failed reading HLS segment: media_file_id={}, segment={}, error={}",
            file_id,
            segment_name,
            e
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "video/mp2t")
        .header(CACHE_CONTROL, "private, max-age=3600")
        .body(Body::from(bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Poll for a file to exist and be non-empty, up to `timeout`. ffmpeg writes
/// the playlist/segments asynchronously; this bridges the gap between
/// "session started" and "first output ready" without the client needing to
/// retry the whole request.
async fn wait_for_nonempty_file(path: &Path, poll_interval: Duration, timeout: Duration) -> bool {
    let start = tokio::time::Instant::now();
    loop {
        if tokio::fs::metadata(path)
            .await
            .map(|m| m.len() > 0)
            .unwrap_or(false)
        {
            return true;
        }
        if start.elapsed() >= timeout {
            return false;
        }
        tokio::time::sleep(poll_interval).await;
    }
}

async fn fetch_media_file(state: &AppState, file_id: &str) -> Result<MediaFileRecord, StatusCode> {
    let id = file_id.to_string();
    let row = MediaFile::get(state.db.pool(), &id).await.map_err(|e| {
        error!(
            media_file_id = %file_id,
            error = %e,
            "Failed querying media file for stream/info: media_file_id={}, error={}",
            file_id,
            e
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let Some(media_file) = row else {
        warn!(
            media_file_id = %file_id,
            "Media file not found for stream/info: media_file_id={}",
            file_id
        );
        return Err(StatusCode::NOT_FOUND);
    };

    Ok(MediaFileRecord {
        id: media_file.id,
        library_id: media_file.library_id,
        path: media_file.path,
        size: media_file.size,
        container: media_file.container,
        video_codec: media_file.video_codec,
        audio_codec: media_file.audio_codec,
        resolution: media_file.resolution,
        width: media_file.width,
        height: media_file.height,
        duration: media_file.duration,
        is_hdr: media_file.is_hdr,
        hdr_type: media_file.hdr_type,
    })
}

async fn authorize_browser_media(
    state: &AppState,
    headers: &HeaderMap,
    media_file: &MediaFileRecord,
) -> Result<AuthUser, StatusCode> {
    let user = require_authenticated_user(state, headers, None).await?;
    if user.is_admin() {
        return Ok(user);
    }
    let library_id = media_file
        .library_id
        .as_ref()
        .ok_or(StatusCode::NOT_FOUND)?;
    let library = Library::get(state.db.pool(), library_id)
        .await
        .map_err(|error| {
            tracing::error!(
                media_file_id = %media_file.id,
                library_id,
                error = %error,
                "Failed to authorize media library ownership"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;
    if library.user_id != user.user_id {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(user)
}

async fn authorize_cast_grant(
    state: &AppState,
    grant: &str,
    media_file_id: &str,
    peer: SocketAddr,
) -> Result<CastSession, StatusCode> {
    let cast = state
        .services
        .get_cast()
        .await
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let claims = cast
        .verify_grant(grant, chrono::Utc::now().timestamp())
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    if claims.media_file_id != media_file_id {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let receiver_ip: std::net::IpAddr = claims
        .receiver_address
        .parse()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    if receiver_ip != peer.ip() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let session = CastSession::get(state.db.pool(), &claims.session_id)
        .await
        .map_err(|error| {
            tracing::error!(
                cast_session_id = %claims.session_id,
                error = %error,
                "Failed to verify Cast grant session"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let active_state = matches!(
        session.player_state.as_str(),
        "STARTING" | "PLAYING" | "PAUSED" | "BUFFERING"
    );
    if !active_state
        || session.ended_at.is_some()
        || session.user_id.as_deref() != Some(claims.user_id.as_str())
        || session.media_file_id.as_deref() != Some(media_file_id)
        || session.receiver_address.as_deref() != Some(claims.receiver_address.as_str())
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(session)
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

/// Whether ffprobe's reported container is one a browser `<video>`/`<audio>`
/// element can demux directly (independent of the codecs inside it).
fn compatible_container(container: Option<&str>) -> bool {
    container
        .map(|c| {
            let c = c.to_ascii_lowercase();
            // ffprobe's `format_name` for Matroska (.mkv) and for genuine
            // WebM are indistinguishable — both report "matroska,webm",
            // since WebM is a constrained profile of the same Matroska
            // demuxer. Treat any "matroska"-containing format name as NOT
            // directly browser-playable: this correctly flags the common
            // MKV-rip case (`docs/tier1-features-plan.md` §4's primary
            // remux scenario) at the cost of also conservatively remuxing
            // genuine standalone .webm files, which is still fully correct
            // output (cheap stream-copy), just not the fastest possible path
            // for that rarer case.
            if c.contains("matroska") {
                return false;
            }
            c.contains("mp4") || c.contains("webm") || c.contains("mp3")
        })
        .unwrap_or(false)
}

/// Whether ffprobe's reported video codec is browser-decodable. `None`
/// (audio-only file, no video stream) is trivially compatible.
fn compatible_video_codec(video_codec: Option<&str>) -> bool {
    video_codec
        .map(|v| {
            let v = v.to_ascii_lowercase();
            v.contains("h264") || v.contains("avc") || v.contains("vp8") || v.contains("vp9")
        })
        .unwrap_or(true)
}

/// Whether ffprobe's reported audio codec is browser-decodable. `None`
/// (video-only stream, no audio) is trivially compatible.
fn compatible_audio_codec(audio_codec: Option<&str>) -> bool {
    audio_codec
        .map(|a| {
            let a = a.to_ascii_lowercase();
            a.contains("aac") || a.contains("mp3") || a.contains("opus") || a.contains("vorbis")
        })
        .unwrap_or(true)
}

/// Check if media is Chromecast compatible without transcoding. Kept
/// separate from [needs_transcode]/[TranscodeDecision] on purpose: cast
/// devices often have hardware HEVC/10-bit decoders a browser tab doesn't, so
/// "does this need transcoding for a browser" and "does this need
/// transcoding for the cast receiver" are genuinely different questions that
/// happen to share the same compatible-container/codec predicates today.
fn is_chromecast_compatible(
    container: Option<&str>,
    video_codec: Option<&str>,
    audio_codec: Option<&str>,
) -> bool {
    compatible_container(container)
        && compatible_video_codec(video_codec)
        && compatible_audio_codec(audio_codec)
}

/// Browser HLS-transcode decision: whether (and how) a file needs on-demand
/// repackaging/re-encoding before a browser `<video>`/`<audio>` element can
/// play it. See `docs/design.md`'s transcode-decision-matrix Q entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TranscodeDecision {
    /// Direct play: container and codecs are all browser-compatible.
    None,
    /// Stream-copy repackage into HLS: codecs are fine, only the container
    /// isn't natively playable (e.g. an MKV with H.264/AAC inside).
    Remux,
    /// Full re-encode into HLS: the video or audio codec itself isn't
    /// browser-decodable (e.g. HEVC/Dolby Vision, DTS).
    Transcode,
}

impl TranscodeDecision {
    fn as_str(self) -> &'static str {
        match self {
            TranscodeDecision::None => "none",
            TranscodeDecision::Remux => "remux",
            TranscodeDecision::Transcode => "transcode",
        }
    }
}

/// Generalizes [is_chromecast_compatible] into the three-way browser
/// playback decision: direct play, stream-copy remux, or full transcode.
fn needs_transcode(
    container: Option<&str>,
    video_codec: Option<&str>,
    audio_codec: Option<&str>,
) -> TranscodeDecision {
    let container_ok = compatible_container(container);
    let video_ok = compatible_video_codec(video_codec);
    let audio_ok = compatible_audio_codec(audio_codec);

    if container_ok && video_ok && audio_ok {
        TranscodeDecision::None
    } else if video_ok && audio_ok {
        // Codecs are already browser-decodable; only the container needs
        // repackaging, which the cheap, lossless stream-copy remux handles.
        TranscodeDecision::Remux
    } else {
        // At least one codec itself isn't browser-decodable; a real
        // re-encode is required.
        TranscodeDecision::Transcode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_transcode_none_for_compatible_mp4() {
        assert_eq!(
            needs_transcode(Some("mov,mp4,m4a"), Some("h264"), Some("aac")),
            TranscodeDecision::None
        );
    }

    #[test]
    fn needs_transcode_none_for_compatible_audio_only_file() {
        assert_eq!(
            needs_transcode(Some("mp3"), None, Some("mp3")),
            TranscodeDecision::None
        );
    }

    #[test]
    fn needs_transcode_remux_for_incompatible_container_compatible_codecs() {
        // e.g. an MKV with H.264/AAC inside: no re-encode needed, only
        // repackaging into HLS segments. ffprobe reports this container as
        // "matroska,webm" for both real MKV and real WebM files.
        assert_eq!(
            needs_transcode(Some("matroska,webm"), Some("h264"), Some("aac")),
            TranscodeDecision::Remux
        );
    }

    #[test]
    fn compatible_container_treats_any_matroska_format_name_as_incompatible() {
        // Regression guard: a naive `.contains("webm")` substring check
        // against ffprobe's "matroska,webm" format name would wrongly treat
        // every MKV rip as direct-playable.
        assert!(!compatible_container(Some("matroska,webm")));
        assert!(compatible_container(Some("mov,mp4,m4a,3gp,3g2,mj2")));
        assert!(compatible_container(Some("mp3")));
    }

    #[test]
    fn needs_transcode_transcode_for_incompatible_video_codec() {
        // e.g. an HEVC rip: even repackaged into HLS, a browser still can't
        // decode HEVC, so a real re-encode is required.
        assert_eq!(
            needs_transcode(Some("matroska,webm"), Some("hevc"), Some("aac")),
            TranscodeDecision::Transcode
        );
    }

    #[test]
    fn needs_transcode_transcode_for_incompatible_audio_codec() {
        assert_eq!(
            needs_transcode(Some("mov,mp4,m4a"), Some("h264"), Some("dts")),
            TranscodeDecision::Transcode
        );
    }

    #[test]
    fn needs_transcode_remux_for_unknown_container_with_unknown_codecs() {
        // Unknown codecs default to "assume compatible" (same convention
        // `is_chromecast_compatible` already used for its `unwrap_or(true)`
        // codec defaults); only the container defaults to "assume
        // incompatible". So fully-unknown media conservatively remuxes
        // (repackage-only) rather than paying for a full re-encode it may
        // not need.
        assert_eq!(needs_transcode(None, None, None), TranscodeDecision::Remux);
    }

    #[test]
    fn transcode_decision_as_str_matches_json_hint_values() {
        assert_eq!(TranscodeDecision::None.as_str(), "none");
        assert_eq!(TranscodeDecision::Remux.as_str(), "remux");
        assert_eq!(TranscodeDecision::Transcode.as_str(), "transcode");
    }

    #[test]
    fn is_chromecast_compatible_still_matches_none_decision() {
        // Chromecast compatibility and the direct-play (`None`) transcode
        // decision share the same underlying predicates today, even though
        // they're intentionally separate functions (see doc comment above).
        let cases: &[(Option<&str>, Option<&str>, Option<&str>)] = &[
            (Some("mov,mp4,m4a"), Some("h264"), Some("aac")),
            (Some("matroska,webm"), Some("h264"), Some("aac")),
            (Some("matroska,webm"), Some("hevc"), Some("aac")),
            (None, None, None),
        ];
        for (container, video, audio) in cases {
            assert_eq!(
                is_chromecast_compatible(*container, *video, *audio),
                needs_transcode(*container, *video, *audio) == TranscodeDecision::None,
                "container={container:?} video={video:?} audio={audio:?}"
            );
        }
    }

    #[test]
    fn parse_range_header_parses_valid_ranges_and_open_end() {
        assert_eq!(parse_range_header("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(parse_range_header("bytes=500-", 1000), Some((500, 999)));
        assert_eq!(parse_range_header("bytes=1000-1001", 1000), None);
        assert_eq!(parse_range_header("bytes=100-50", 1000), None);
        assert_eq!(parse_range_header("not-a-range", 1000), None);
    }
}
