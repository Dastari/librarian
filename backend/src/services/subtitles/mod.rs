//! Subtitles beyond what ffprobe finds inside a file: sidecar files beside a video, subtitles
//! downloaded from OpenSubtitles, and the conversion of all of them to WebVTT for the player.
//! See `docs/design.md` Q118–Q120.

pub mod cues;
pub mod language;
pub mod opensubtitles;
pub mod provider;
pub mod sidecar;
pub mod store;

pub use cues::{Cue, TextFormat, decode_text, to_vtt};

/// Codecs that are pictures, not text: they can only be shown by burning them into the video.
pub fn is_bitmap_codec(codec: &str) -> bool {
    matches!(
        codec.to_ascii_lowercase().as_str(),
        "hdmv_pgs_subtitle"
            | "pgssub"
            | "pgs"
            | "dvd_subtitle"
            | "dvdsub"
            | "vobsub"
            | "dvb_subtitle"
            | "dvbsub"
            | "xsub"
    )
}

/// Text codecs ffmpeg can turn into WebVTT when they are inside a video file.
pub fn is_embedded_text_codec(codec: &str) -> bool {
    matches!(
        codec.to_ascii_lowercase().as_str(),
        "subrip" | "srt" | "ass" | "ssa" | "webvtt" | "mov_text" | "text" | "microdvd"
    )
}

/// Largest offset a viewer can ask for: ten minutes either way.
pub const MAX_OFFSET_MS: i64 = 10 * 60 * 1000;

/// Convert a sidecar file's bytes to WebVTT with an offset.
pub fn file_to_vtt(
    bytes: &[u8],
    format: TextFormat,
    fps: Option<f64>,
    offset_ms: i64,
) -> Result<String, cues::ParseError> {
    let text = decode_text(bytes);
    let cues = cues::parse(format, &text, fps)?;
    Ok(to_vtt(
        &cues,
        offset_ms.clamp(-MAX_OFFSET_MS, MAX_OFFSET_MS),
    ))
}
