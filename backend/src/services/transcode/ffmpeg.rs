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
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tracing::warn;

use super::session::TranscodeKind;
use crate::services::library_storage::{TOOL_PROTOCOL_WHITELIST, is_tool_url};

/// Playlist filename written into every session's output directory.
pub const PLAYLIST_FILENAME: &str = "playlist.m3u8";

/// `ffmpeg -hls_segment_filename` pattern (relative segment naming), used
/// under an output directory that is unique per session.
pub const SEGMENT_FILENAME_PATTERN: &str = "seg_%03d.ts";

/// Length of a transcoded segment, in seconds. Key frames are forced at multiples of this.
pub const SEGMENT_SECONDS: u32 = 6;

/// The playlist `ffmpeg` keeps for itself in an on-demand session. Nothing reads it: clients get
/// [`vod_playlist`], which already lists the whole file.
const ENCODER_PLAYLIST_FILENAME: &str = "encoder.m3u8";

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

/// Transcoded audio is mixed down to stereo (mono stays mono). Surround folded down by the
/// player is not safe: some TVs' WebView plays 5.1 AAC by keeping only front left and right, which
/// drops the centre channel and with it nearly all dialogue (music and effects only). ffmpeg's
/// downmix folds the centre and surrounds into both sides. It also keeps the output in a layout AAC
/// has a standard code for: a file's 5.1(side), the usual layout of E-AC-3 and AC-3, would otherwise
/// make ffmpeg describe the channels in a way browsers can't append. Direct play still passes a
/// file's own surround track to players that can decode it.
const AAC_CHANNEL_LAYOUTS: &str = "aformat=channel_layouts=stereo|mono";

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
        "-sn".to_string(),
        "-dn".to_string(),
        "-af".to_string(),
        AAC_CHANNEL_LAYOUTS.to_string(),
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

/// How many segments a file of this length has.
pub fn segment_count(duration_seconds: u32) -> u32 {
    duration_seconds.div_ceil(SEGMENT_SECONDS)
}

/// The whole file as a finished playlist, before any of it has been encoded. Players then know
/// its real length and can seek anywhere in it, and asking for a segment is what encodes it.
pub fn vod_playlist(duration_seconds: u32) -> String {
    let count = segment_count(duration_seconds);
    let mut playlist = format!(
        "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:{}\n#EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-PLAYLIST-TYPE:VOD\n",
        SEGMENT_SECONDS + 1
    );
    for segment in 0..count {
        let remaining = duration_seconds - segment * SEGMENT_SECONDS;
        playlist.push_str(&format!(
            "#EXTINF:{}.000000,\nseg_{segment:03}.ts\n",
            remaining.min(SEGMENT_SECONDS)
        ));
    }
    playlist.push_str("#EXT-X-ENDLIST\n");
    playlist
}

/// A picture subtitle drawn into the video (Q120): PGS, VobSub and DVB subtitles cannot be
/// sent to a browser as text, so a variant with them burned in is encoded instead.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BurnIn {
    /// A subtitle stream of the file itself, by its absolute ffprobe index.
    Stream(i32),
    /// A VobSub sidecar on this machine, by its `.idx` (ffmpeg finds the `.sub` beside it).
    VobSub(PathBuf),
}

impl BurnIn {
    /// Names the variant in a session key.
    pub fn key(&self) -> String {
        match self {
            Self::Stream(index) => format!("stream:{index}"),
            Self::VobSub(path) => format!("vobsub:{}", path.display()),
        }
    }
}

/// Re-encode from `start_segment` to the end of the file. The output keeps the file's own
/// timeline, so a segment is the same wherever the encoder started: a later seek can reuse it.
pub fn build_on_demand_transcode_args(
    input: &Path,
    output_dir: &Path,
    start_segment: u32,
    audio_stream: Option<i32>,
    audio_only: bool,
) -> Vec<String> {
    build_on_demand_transcode_args_with_burn(
        input,
        output_dir,
        start_segment,
        audio_stream,
        audio_only,
        None,
    )
}

/// [`build_on_demand_transcode_args`], optionally with a picture subtitle burned in. The
/// subtitle input is seeked like the video, so its pictures stay on the file's timeline.
pub fn build_on_demand_transcode_args_with_burn(
    input: &Path,
    output_dir: &Path,
    start_segment: u32,
    audio_stream: Option<i32>,
    audio_only: bool,
    burn: Option<&BurnIn>,
) -> Vec<String> {
    let offset = (start_segment * SEGMENT_SECONDS).to_string();
    let mut args: Vec<String> = ["-y", "-ss", &offset, "-i"].map(str::to_owned).to_vec();
    args.push(input.to_string_lossy().into_owned());
    let burn = burn.filter(|_| !audio_only);
    if let Some(BurnIn::VobSub(index_file)) = burn {
        args.extend(["-ss".to_owned(), offset.clone(), "-i".to_owned()]);
        args.push(index_file.to_string_lossy().into_owned());
    }
    if let Some(burn) = burn {
        let subtitle = match burn {
            BurnIn::Stream(index) => format!("[0:{index}]"),
            BurnIn::VobSub(_) => "[1:s:0]".to_owned(),
        };
        // `eof_action=pass` keeps the picture going after the last subtitle.
        args.extend([
            "-filter_complex".to_owned(),
            format!("[0:v:0]{subtitle}overlay=eof_action=pass[burned]"),
            "-map".to_owned(),
            "[burned]".to_owned(),
            "-map".to_owned(),
            match audio_stream {
                Some(index) => format!("0:{index}"),
                None => "0:a:0?".to_owned(),
            },
        ]);
    } else if let Some(index) = audio_stream {
        args.extend([
            "-map".to_owned(),
            "0:v:0?".to_owned(),
            "-map".to_owned(),
            format!("0:{index}"),
        ]);
    }
    let forced_key_frames = format!("expr:gte(t,n_forced*{SEGMENT_SECONDS})");
    let segment_seconds = SEGMENT_SECONDS.to_string();
    let start_number = start_segment.to_string();
    if audio_only {
        // An audiobook's "video" is its cover. Stereo is what every browser's AAC decoder plays.
        args.extend(["-vn", "-sn", "-dn", "-ac", "2"].map(str::to_owned));
    } else {
        args.extend(
            [
                "-threads",
                "2",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-force_key_frames",
                &forced_key_frames,
                "-preset",
                "veryfast",
                // Subtitles are served by their own route. Converting them here wastes work, and
                // an image format would stop the encoder.
                "-sn",
                "-dn",
                "-af",
                AAC_CHANNEL_LAYOUTS,
            ]
            .map(str::to_owned),
        );
    }
    args.extend(
        [
            "-c:a",
            "aac",
            "-output_ts_offset",
            &offset,
            "-start_number",
            &start_number,
            "-f",
            "hls",
            "-hls_time",
            &segment_seconds,
            "-hls_list_size",
            "0",
            // A segment appears only once it's complete, so its existence means it can be served.
            "-hls_flags",
            "temp_file",
            "-hls_segment_filename",
        ]
        .map(str::to_owned),
    );
    args.push(
        segment_filename_pattern(output_dir)
            .to_string_lossy()
            .into_owned(),
    );
    args.push(
        output_dir
            .join(ENCODER_PLAYLIST_FILENAME)
            .to_string_lossy()
            .into_owned(),
    );
    args
}

/// Start an on-demand encoder at `start_segment`.
#[allow(clippy::too_many_arguments)]
pub fn spawn_on_demand_transcode(
    ffmpeg_path: &str,
    input: &Path,
    output_dir: &Path,
    session_id: &str,
    start_segment: u32,
    audio_stream: Option<i32>,
    audio_only: bool,
    burn: Option<&BurnIn>,
) -> Result<Child> {
    let args = build_on_demand_transcode_args_with_burn(
        input,
        output_dir,
        start_segment,
        audio_stream,
        audio_only,
        burn,
    );
    spawn_ffmpeg(ffmpeg_path, &args, session_id)
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
    // An input read through a tool URL (a file on a share) needs the http protocol allowed.
    let whitelist: &[&str] = if args.iter().any(|arg| is_tool_url(arg)) {
        &["-protocol_whitelist", TOOL_PROTOCOL_WHITELIST]
    } else {
        &[]
    };
    Command::new(ffmpeg_path)
        .args(whitelist)
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

/// Keep legacy default selection unchanged; explicit selection maps one video
/// (if present) and precisely the authorized absolute audio index. Subtitles
/// must be delivered separately rather than accidentally mapped into MPEG-TS.
pub fn spawn_hls_with_audio(
    ffmpeg_path: &str,
    input: &Path,
    output_dir: &Path,
    session_id: &str,
    kind: TranscodeKind,
    audio_stream: Option<i32>,
) -> Result<Child> {
    let mut args = match kind {
        TranscodeKind::Remux => build_remux_args(input, output_dir),
        TranscodeKind::Transcode => build_transcode_args(input, output_dir),
    };
    if let Some(index) = audio_stream {
        anyhow::ensure!(
            index >= 0,
            "negative audio stream index for HLS session {session_id}"
        );
        args.splice(
            3..3,
            [
                "-map".to_owned(),
                "0:v:0?".to_owned(),
                "-map".to_owned(),
                format!("0:{index}"),
                "-sn".to_owned(),
                "-dn".to_owned(),
            ],
        );
    }
    spawn_ffmpeg(ffmpeg_path, &args, session_id)
}

/// Hard allocation/output cap, including for embedded subtitles in large files.
pub const MAX_SUBTITLE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug)]
pub enum SubtitleError {
    Busy,
    Unavailable,
    Timeout,
    TooLarge,
    Conversion,
}

/// Read stdout incrementally under a hard cap. Never use `output()` here:
/// malformed/large input must not allocate an unbounded subtitle response.
/// `input_format` forces the sidecar demuxer so a disguised playlist cannot
/// cause ffmpeg to open additional local or remote sources.
pub async fn subtitle_webvtt(
    ffmpeg_path: &str,
    input: &Path,
    stream_index: Option<i32>,
    input_format: Option<&str>,
) -> std::result::Result<Vec<u8>, SubtitleError> {
    let mut command = Command::new(ffmpeg_path);
    // Local input may only be a file or a pipe; a file on a share arrives as a loopback URL.
    let whitelist = if is_tool_url(&input.to_string_lossy()) {
        TOOL_PROTOCOL_WHITELIST
    } else {
        "file,pipe"
    };
    command.args([
        "-nostdin",
        "-v",
        "error",
        "-threads",
        "1",
        "-protocol_whitelist",
        whitelist,
    ]);
    if let Some(format) = input_format {
        command.args(["-f", format]);
    }
    command.arg("-i").arg(input);
    let mapping = match stream_index {
        Some(index) if index >= 0 => format!("0:{index}"),
        Some(_) => return Err(SubtitleError::Conversion),
        None => "0:s:0".to_owned(),
    };
    let mut child = command
        .args(["-map", &mapping, "-c:s", "webvtt", "-f", "webvtt", "pipe:1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| {
            warn!(input_path = %input.display(), %error, "Failed to spawn subtitle conversion");
            SubtitleError::Unavailable
        })?;
    let stdout = child.stdout.take().ok_or(SubtitleError::Conversion)?;
    let result = tokio::time::timeout(Duration::from_secs(20), async {
        let mut bytes = Vec::new();
        stdout
            .take(MAX_SUBTITLE_BYTES + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| SubtitleError::Conversion)?;
        if bytes.len() as u64 > MAX_SUBTITLE_BYTES {
            return Err(SubtitleError::TooLarge);
        }
        let status = child.wait().await.map_err(|_| SubtitleError::Conversion)?;
        if !status.success() || !bytes.starts_with(b"WEBVTT") {
            return Err(SubtitleError::Conversion);
        }
        Ok(bytes)
    })
    .await
    .unwrap_or(Err(SubtitleError::Timeout));
    if result.is_err() {
        let _ = child.start_kill();
        let _ = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;
    }
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn burn_in_overlays_the_subtitle_and_keeps_the_audio() {
        let args = build_on_demand_transcode_args_with_burn(
            Path::new("/in.mkv"),
            Path::new("/out"),
            2,
            Some(1),
            false,
            Some(&BurnIn::Stream(4)),
        );
        let joined = args.join(" ");
        assert!(joined.starts_with("-y -ss 12 -i /in.mkv -filter_complex [0:v:0][0:4]overlay=eof_action=pass[burned] -map [burned] -map 0:1"), "{joined}");
        let vobsub = build_on_demand_transcode_args_with_burn(
            Path::new("/in.mkv"),
            Path::new("/out"),
            0,
            None,
            false,
            Some(&BurnIn::VobSub(PathBuf::from("/m/Film.idx"))),
        )
        .join(" ");
        assert!(vobsub.starts_with("-y -ss 0 -i /in.mkv -ss 0 -i /m/Film.idx -filter_complex [0:v:0][1:s:0]overlay=eof_action=pass[burned] -map [burned] -map 0:a:0?"), "{vobsub}");
        // Still no subtitle stream in the output.
        assert!(vobsub.contains(" -sn "));
    }

    use super::*;

    #[test]
    fn vod_playlist_lists_the_whole_file_and_is_closed() {
        let playlist = vod_playlist(14);
        assert!(playlist.contains("#EXT-X-PLAYLIST-TYPE:VOD"));
        assert!(playlist.contains("#EXTINF:6.000000,\nseg_000.ts"));
        assert!(playlist.contains("#EXTINF:6.000000,\nseg_001.ts"));
        // Two seconds left over.
        assert!(playlist.contains("#EXTINF:2.000000,\nseg_002.ts"));
        assert!(playlist.ends_with("#EXT-X-ENDLIST\n"));
        assert_eq!(segment_count(14), 3);
        assert_eq!(segment_count(12), 2);
    }

    /// The value after a flag, for asserting on what an encoder is told.
    fn after(args: &[String], flag: &str) -> Option<String> {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|index| args.get(index + 1).cloned())
    }

    #[test]
    fn video_transcodes_only_write_aac_layouts_browsers_can_append() {
        let on_demand =
            build_on_demand_transcode_args(Path::new("/in.mkv"), Path::new("/out"), 0, None, false);
        let full = build_transcode_args(Path::new("/in.mkv"), Path::new("/out"));
        for args in [&on_demand, &full] {
            assert_eq!(after(args, "-af").as_deref(), Some(AAC_CHANNEL_LAYOUTS));
            assert!(args.contains(&"-sn".to_string()));
            assert!(args.contains(&"-dn".to_string()));
        }
    }

    #[test]
    fn a_chosen_audio_track_gets_the_same_layouts() {
        let args = build_on_demand_transcode_args(
            Path::new("/in.mkv"),
            Path::new("/out"),
            0,
            Some(2),
            false,
        );
        assert_eq!(after(&args, "-af").as_deref(), Some(AAC_CHANNEL_LAYOUTS));
        assert!(args.contains(&"0:2".to_string()));
    }

    #[test]
    fn an_audiobook_is_not_encoded_as_video() {
        let args =
            build_on_demand_transcode_args(Path::new("/in.m4b"), Path::new("/out"), 10, None, true);
        assert!(args.contains(&"-vn".to_string()));
        assert!(!args.contains(&"libx264".to_string()));
        assert!(args.contains(&"aac".to_string()));
        assert!(args.contains(&"60".to_string()));
    }

    #[test]
    fn on_demand_args_seek_to_the_segment_and_keep_the_file_timeline() {
        let args = build_on_demand_transcode_args(
            Path::new("/in.mkv"),
            Path::new("/out"),
            700,
            Some(2),
            false,
        );
        let after = |flag: &str| args[args.iter().position(|arg| arg == flag).unwrap() + 1].clone();
        assert_eq!(after("-ss"), "4200");
        assert_eq!(after("-output_ts_offset"), "4200");
        assert_eq!(after("-start_number"), "700");
        assert_eq!(after("-hls_flags"), "temp_file");
        // The seek comes before the input, and the selected audio after it.
        let position = |flag: &str| args.iter().position(|arg| arg == flag).unwrap();
        assert!(position("-ss") < position("-i"));
        assert!(position("-i") < position("-map"));
        assert!(args.contains(&"0:2".to_string()));
    }

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
