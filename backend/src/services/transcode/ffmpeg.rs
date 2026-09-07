//! `ffmpeg` invocation for on-demand HLS packaging.
//!
//! Mirrors the `ffprobe` availability-check + invocation pattern in
//! `services/library_scan.rs` (`check_ffprobe_available`, `ffprobe_analyze`):
//! same `Command::new(..).arg("-version").output()` shape for the
//! availability check, same "log a warning with stderr/error, never panic"
//! posture for invocation failures.
//!
//! Two spawn paths, sequenced per `docs/tier1-features-plan.md` §4:
//! - [`spawn_hls_remux`]: stream-copy (`-c copy`) repackage into HLS segments.
//!   Covers the common case where the *container* isn't browser-playable
//!   (e.g. an MKV) but the codecs inside (H.264/AAC) already are — no
//!   re-encode needed, so this is fast and lossless.
//! - [`spawn_hls_transcode`]: full re-encode to H.264/AAC. Needed when the
//!   *codec* itself isn't browser-decodable (HEVC/Dolby Vision rips).
//!   Sequenced second (order of magnitude more expensive than remux).

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::process::{Child, Command};
use tracing::warn;

/// Playlist filename written into every session's output directory.
pub const PLAYLIST_FILENAME: &str = "playlist.m3u8";

/// `ffmpeg -hls_segment_filename` pattern (relative segment naming), used
/// under an output directory that is unique per session.
pub const SEGMENT_FILENAME_PATTERN: &str = "seg_%03d.ts";

/// Check whether the configured `ffmpeg` binary is invocable. Copies the
/// shape of `LibraryScanService::check_ffprobe_available` exactly: a
/// `-version` call, `Ok(status_success)` -> true, anything else -> false with
/// a `warn!` including stderr or the spawn error so an operator can see why
/// HLS transcoding is unavailable without the server refusing to start.
pub async fn ffmpeg_available(ffmpeg_path: &str) -> bool {
    match Command::new(ffmpeg_path).arg("-version").output().await {
        Ok(output) if output.status.success() => true,
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                status = ?output.status.code(),
                stderr = %stderr,
                "ffmpeg startup check failed: command returned non-zero status"
            );
            false
        }
        Err(e) => {
            warn!(
                error = %e,
                "ffmpeg startup check failed: command not executable or not found"
            );
            false
        }
    }
}

/// Full path to the playlist file inside a session's output directory.
pub fn playlist_path(output_dir: &Path) -> PathBuf {
    output_dir.join(PLAYLIST_FILENAME)
}

/// Full path (with the `%03d` pattern) ffmpeg should write segments to.
pub fn segment_filename_pattern(output_dir: &Path) -> PathBuf {
    output_dir.join(SEGMENT_FILENAME_PATTERN)
}

/// Build the argument vector for a stream-copy HLS remux. Pulled out as a
/// pure function (no process spawn) so the exact arguments are unit-testable
/// without running `ffmpeg`.
pub fn build_remux_args(input: &Path, output_dir: &Path) -> Vec<String> {
    vec![
        "-y".to_string(),
        "-i".to_string(),
        input.to_string_lossy().into_owned(),
        "-c".to_string(),
        "copy".to_string(),
        "-f".to_string(),
        "hls".to_string(),
        "-hls_time".to_string(),
        "6".to_string(),
        // Publish segments during encoding; VOD withholds the playlist until EOF.
        "-hls_playlist_type".to_string(),
        "event".to_string(),
        "-hls_segment_filename".to_string(),
        segment_filename_pattern(output_dir)
            .to_string_lossy()
            .into_owned(),
        playlist_path(output_dir).to_string_lossy().into_owned(),
    ]
}

/// Build the argument vector for a full re-encode to H.264/AAC HLS.
pub fn build_transcode_args(input: &Path, output_dir: &Path) -> Vec<String> {
    vec![
        "-y".to_string(),
        "-i".to_string(),
        input.to_string_lossy().into_owned(),
        "-threads".to_string(),
        "2".to_string(),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-force_key_frames".to_string(),
        "expr:gte(t,n_forced*6)".to_string(),
        "-preset".to_string(),
        "veryfast".to_string(),
        "-c:a".to_string(),
        "aac".to_string(),
        "-f".to_string(),
        "hls".to_string(),
        "-hls_time".to_string(),
        "6".to_string(),
        // Publish segments during encoding; VOD withholds the playlist until EOF.
        "-hls_playlist_type".to_string(),
        "event".to_string(),
        "-hls_segment_filename".to_string(),
        segment_filename_pattern(output_dir)
            .to_string_lossy()
            .into_owned(),
        playlist_path(output_dir).to_string_lossy().into_owned(),
    ]
}

/// Spawn `ffmpeg` with the given argument vector. `stdout`/`stderr` are
/// discarded (`Stdio::null()`) rather than piped: ffmpeg writes a continuous
/// stream of progress lines to stderr, and an unread piped fd would fill its
/// OS buffer and deadlock the child once ffmpeg blocks on a full pipe. No
/// `.unwrap()` on any `Child` operation here or in callers (`panic = "abort"`
/// makes an unwrap-on-child-op a full server crash, not just a lost session).
/// `kill_on_drop(true)` is a last-resort safety net if a `Child` handle is
/// ever dropped without an explicit kill (normal shutdown/reap paths always
/// kill explicitly first).
fn spawn_ffmpeg(ffmpeg_path: &str, args: &[String], session_id: &str) -> Result<Child> {
    Command::new(ffmpeg_path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("failed to spawn ffmpeg for HLS session {session_id}"))
}

/// Start a stream-copy HLS remux: `ffmpeg -i {input} -c copy -f hls
/// -hls_time 6 -hls_playlist_type event -hls_segment_filename {out}/seg_%03d.ts
/// {out}/playlist.m3u8`. The primary transcode path — covers incompatible
/// container / compatible codec, the common case.
pub fn spawn_hls_remux(
    ffmpeg_path: &str,
    input: &Path,
    output_dir: &Path,
    session_id: &str,
) -> Result<Child> {
    let args = build_remux_args(input, output_dir);
    spawn_ffmpeg(ffmpeg_path, &args, session_id)
}

/// Start a full re-encode HLS transcode (`-c:v libx264 -c:a aac`) for the
/// incompatible-codec case (HEVC/Dolby Vision). Secondary path per
/// `docs/tier1-features-plan.md` §4 — remux is sequenced first.
pub fn spawn_hls_transcode(
    ffmpeg_path: &str,
    input: &Path,
    output_dir: &Path,
    session_id: &str,
) -> Result<Child> {
    let args = build_transcode_args(input, output_dir);
    spawn_ffmpeg(ffmpeg_path, &args, session_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remux_args_use_stream_copy_and_expected_hls_flags() {
        let input = Path::new("/media/movies/Movie (2020)/movie.mkv");
        let output_dir = Path::new("/data/cache/hls/mf1/sess1");
        let args = build_remux_args(input, output_dir);

        assert_eq!(args[0], "-y");
        assert_eq!(args[1], "-i");
        assert_eq!(args[2], "/media/movies/Movie (2020)/movie.mkv");
        assert_eq!(args[3], "-c");
        assert_eq!(args[4], "copy");
        assert_eq!(args[5], "-f");
        assert_eq!(args[6], "hls");
        assert_eq!(args[7], "-hls_time");
        assert_eq!(args[8], "6");
        assert_eq!(args[9], "-hls_playlist_type");
        assert_eq!(args[10], "event");
        assert_eq!(args[11], "-hls_segment_filename");
        assert_eq!(args[12], "/data/cache/hls/mf1/sess1/seg_%03d.ts");
        assert_eq!(args[13], "/data/cache/hls/mf1/sess1/playlist.m3u8");
        // Never re-encodes in the remux path.
        assert!(!args.contains(&"libx264".to_string()));
    }

    #[test]
    fn transcode_args_reencode_video_and_audio() {
        let input = Path::new("/media/movies/Movie (2020)/movie.mkv");
        let output_dir = Path::new("/data/cache/hls/mf1/sess2");
        let args = build_transcode_args(input, output_dir);

        assert!(args.contains(&"-c:v".to_string()));
        assert!(args.contains(&"libx264".to_string()));
        assert!(args.contains(&"-c:a".to_string()));
        assert!(args.contains(&"aac".to_string()));
        assert!(args.contains(&"-hls_time".to_string()));
        assert_eq!(
            args.last().unwrap(),
            "/data/cache/hls/mf1/sess2/playlist.m3u8"
        );
    }

    #[test]
    fn playlist_and_segment_paths_are_under_output_dir() {
        let output_dir = Path::new("/tmp/session-x");
        assert_eq!(
            playlist_path(output_dir),
            PathBuf::from("/tmp/session-x/playlist.m3u8")
        );
        assert_eq!(
            segment_filename_pattern(output_dir),
            PathBuf::from("/tmp/session-x/seg_%03d.ts")
        );
    }
}
