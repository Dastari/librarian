//! FFmpeg-based media analysis service
//!
//! Uses ffprobe (command-line) to extract detailed information about media files
//! including video, audio, and subtitle streams.
//!
//! This approach is more reliable than Rust FFmpeg bindings as ffprobe's JSON
//! output format is stable and well-documented.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{debug, info};

/// Complete media analysis result containing all extracted information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAnalysis {
    /// File path that was analyzed
    pub path: String,

    /// Container format (e.g., "matroska", "mp4", "avi")
    pub container_format: String,

    /// Total duration in seconds
    pub duration_secs: Option<f64>,

    /// Overall bitrate in bits per second
    pub bitrate: Option<i64>,

    /// File size in bytes
    pub size_bytes: Option<i64>,

    /// Video streams found in the file
    pub video_streams: Vec<VideoStream>,

    /// Audio streams found in the file
    pub audio_streams: Vec<AudioStream>,

    /// Subtitle streams found in the file (embedded)
    pub subtitle_streams: Vec<SubtitleStream>,

    /// Chapter information if present
    pub chapters: Vec<Chapter>,

    /// Container-level metadata
    pub metadata: HashMap<String, String>,
}

/// Video stream information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStream {
    /// Stream index in the container
    pub index: usize,

    /// Codec name (e.g., "h264", "hevc", "av1")
    pub codec: String,

    /// Codec long name
    pub codec_long_name: Option<String>,

    /// Width in pixels
    pub width: u32,

    /// Height in pixels
    pub height: u32,

    /// Display aspect ratio as string (e.g., "16:9")
    pub aspect_ratio: Option<String>,

    /// Frame rate as string (e.g., "23.976", "24000/1001")
    pub frame_rate: Option<String>,

    /// Average frame rate
    pub avg_frame_rate: Option<String>,

    /// Bitrate in bits per second
    pub bitrate: Option<i64>,

    /// Pixel format (e.g., "yuv420p", "yuv420p10le")
    pub pixel_format: Option<String>,

    /// Color space
    pub color_space: Option<String>,

    /// Color transfer (for HDR detection)
    pub color_transfer: Option<String>,

    /// Color primaries (for HDR detection)
    pub color_primaries: Option<String>,

    /// Detected HDR type based on color metadata
    pub hdr_type: Option<HdrType>,

    /// Bit depth (8, 10, 12)
    pub bit_depth: Option<u8>,

    /// Stream language if specified
    pub language: Option<String>,

    /// Stream title if specified
    pub title: Option<String>,

    /// Whether this is the default stream
    pub is_default: bool,

    /// Stream-level metadata
    pub metadata: HashMap<String, String>,
}

/// Audio stream information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStream {
    /// Stream index in the container
    pub index: usize,

    /// Codec name (e.g., "aac", "ac3", "dts", "flac", "truehd")
    pub codec: String,

    /// Codec long name
    pub codec_long_name: Option<String>,

    /// Number of channels
    pub channels: u16,

    /// Channel layout (e.g., "stereo", "5.1", "7.1")
    pub channel_layout: Option<String>,

    /// Sample rate in Hz
    pub sample_rate: u32,

    /// Bitrate in bits per second
    pub bitrate: Option<i64>,

    /// Bit depth
    pub bit_depth: Option<u8>,

    /// Stream language (ISO 639-1 or 639-2)
    pub language: Option<String>,

    /// Stream title if specified
    pub title: Option<String>,

    /// Whether this is the default stream
    pub is_default: bool,

    /// Whether this is a commentary track
    pub is_commentary: bool,

    /// Stream-level metadata
    pub metadata: HashMap<String, String>,
}

/// Subtitle stream information (embedded in container)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleStream {
    /// Stream index in the container
    pub index: usize,

    /// Codec/format (e.g., "subrip", "ass", "hdmv_pgs_subtitle", "dvd_subtitle")
    pub codec: String,

    /// Codec long name
    pub codec_long_name: Option<String>,

    /// Stream language (ISO 639-1 or 639-2)
    pub language: Option<String>,

    /// Stream title if specified
    pub title: Option<String>,

    /// Whether this is the default subtitle stream
    pub is_default: bool,

    /// Whether this is a forced subtitle stream (for foreign language parts)
    pub is_forced: bool,

    /// Whether this is for hearing impaired (SDH)
    pub is_hearing_impaired: bool,

    /// Stream-level metadata
    pub metadata: HashMap<String, String>,
}

/// Chapter information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    /// Chapter index
    pub index: usize,

    /// Start time in seconds
    pub start_secs: f64,

    /// End time in seconds
    pub end_secs: f64,

    /// Chapter title
    pub title: Option<String>,
}

/// HDR type detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HdrType {
    /// HDR10 (PQ transfer + BT.2020 primaries)
    Hdr10,
    /// HDR10+ (dynamic metadata)
    Hdr10Plus,
    /// Dolby Vision
    DolbyVision,
    /// Hybrid Log-Gamma
    Hlg,
}

impl HdrType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            HdrType::Hdr10 => "HDR10",
            HdrType::Hdr10Plus => "HDR10+",
            HdrType::DolbyVision => "Dolby Vision",
            HdrType::Hlg => "HLG",
        }
    }
}

impl std::fmt::Display for HdrType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// FFprobe JSON output structures
mod ffprobe {
    use super::*;

    #[derive(Debug, Deserialize)]
    pub struct FfprobeOutput {
        pub format: Option<Format>,
        pub streams: Option<Vec<Stream>>,
        pub chapters: Option<Vec<FfprobeChapter>>,
    }

    #[derive(Debug, Deserialize)]
    pub struct Format {
        pub filename: Option<String>,
        pub format_name: Option<String>,
        pub format_long_name: Option<String>,
        pub duration: Option<String>,
        pub size: Option<String>,
        pub bit_rate: Option<String>,
        pub tags: Option<HashMap<String, String>>,
    }

    #[derive(Debug, Deserialize)]
    pub struct Stream {
        pub index: usize,
        pub codec_name: Option<String>,
        pub codec_long_name: Option<String>,
        pub codec_type: Option<String>,
        pub profile: Option<String>,

        // Video specific
        pub width: Option<u32>,
        pub height: Option<u32>,
        pub coded_width: Option<u32>,
        pub coded_height: Option<u32>,
        pub display_aspect_ratio: Option<String>,
        pub pix_fmt: Option<String>,
        pub r_frame_rate: Option<String>,
        pub avg_frame_rate: Option<String>,
        pub color_space: Option<String>,
        pub color_transfer: Option<String>,
        pub color_primaries: Option<String>,
        pub bits_per_raw_sample: Option<String>,

        // Audio specific
        pub channels: Option<u16>,
        pub channel_layout: Option<String>,
        pub sample_rate: Option<String>,
        pub bits_per_sample: Option<u8>,

        // Common
        pub bit_rate: Option<String>,
        pub duration: Option<String>,
        pub disposition: Option<Disposition>,
        pub tags: Option<HashMap<String, String>>,

        // Side data for HDR detection
        pub side_data_list: Option<Vec<SideData>>,
    }

    #[derive(Debug, Deserialize)]
    pub struct Disposition {
        pub default: Option<i32>,
        pub forced: Option<i32>,
        pub hearing_impaired: Option<i32>,
        pub comment: Option<i32>,
    }

    #[derive(Debug, Deserialize)]
    pub struct SideData {
        pub side_data_type: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct FfprobeChapter {
        pub id: Option<i64>,
        pub start_time: Option<String>,
        pub end_time: Option<String>,
        pub tags: Option<HashMap<String, String>>,
    }
}

/// FFmpeg-based media analysis service using ffprobe
pub struct FfmpegService {
    /// Path to ffprobe executable
    ffprobe_path: String,
}

impl FfmpegService {
    /// Create a new FFmpeg service
    pub fn new() -> Self {
        Self {
            ffprobe_path: "ffprobe".to_string(),
        }
    }

    /// Create with a custom ffprobe path
    pub fn with_ffprobe_path(ffprobe_path: String) -> Self {
        Self { ffprobe_path }
    }

    /// Check if ffprobe is available
    pub async fn is_available(&self) -> bool {
        Command::new(&self.ffprobe_path)
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Analyze a media file and extract all stream information
    pub async fn analyze(&self, path: &Path) -> Result<MediaAnalysis> {
        debug!(path = %path.display(), "Analyzing media file with ffprobe");

        // Check if file exists first
        if !path.exists() {
            anyhow::bail!(
                "ffprobe failed for '{}': file does not exist",
                path.display()
            );
        }

        let output = Command::new(&self.ffprobe_path)
            .args(["-v", "error"]) // Show errors instead of quiet
            .args(["-print_format", "json"])
            .args(["-show_format", "-show_streams", "-show_chapters"])
            .arg(path)
            .output()
            .await
            .with_context(|| format!("Failed to execute ffprobe for '{}'", path.display()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output
                .status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            anyhow::bail!(
                "ffprobe failed for '{}' (exit code {}): {}",
                path.display(),
                exit_code,
                if stderr.is_empty() {
                    "no error output"
                } else {
                    stderr.trim()
                }
            );
        }

        let probe: ffprobe::FfprobeOutput = serde_json::from_slice(&output.stdout)
            .context("Failed to parse ffprobe JSON output")?;

        // Convert to our types
        let analysis = self.convert_probe_output(path, probe)?;

        info!(
            path = %path.display(),
            video_streams = analysis.video_streams.len(),
            audio_streams = analysis.audio_streams.len(),
            subtitle_streams = analysis.subtitle_streams.len(),
            chapters = analysis.chapters.len(),
            duration_secs = ?analysis.duration_secs,
            "Media analysis complete"
        );

        Ok(analysis)
    }

    /// Convert ffprobe output to our MediaAnalysis structure
    fn convert_probe_output(
        &self,
        path: &Path,
        probe: ffprobe::FfprobeOutput,
    ) -> Result<MediaAnalysis> {
        let format = probe.format.unwrap_or(ffprobe::Format {
            filename: None,
            format_name: None,
            format_long_name: None,
            duration: None,
            size: None,
            bit_rate: None,
            tags: None,
        });

        let container_format = format.format_name.clone().unwrap_or_default();
        let duration_secs = format.duration.as_ref().and_then(|d| d.parse::<f64>().ok());
        let bitrate = format.bit_rate.as_ref().and_then(|b| b.parse::<i64>().ok());
        let size_bytes = format.size.as_ref().and_then(|s| s.parse::<i64>().ok());
        let metadata = format.tags.unwrap_or_default();

        let mut video_streams = Vec::new();
        let mut audio_streams = Vec::new();
        let mut subtitle_streams = Vec::new();

        if let Some(streams) = probe.streams {
            for stream in streams {
                let codec_type = stream.codec_type.as_deref().unwrap_or("");
                match codec_type {
                    "video" => {
                        if let Some(video) = self.convert_video_stream(&stream) {
                            video_streams.push(video);
                        }
                    }
                    "audio" => {
                        if let Some(audio) = self.convert_audio_stream(&stream) {
                            audio_streams.push(audio);
                        }
                    }
                    "subtitle" => {
                        if let Some(subtitle) = self.convert_subtitle_stream(&stream) {
                            subtitle_streams.push(subtitle);
                        }
                    }
                    _ => {}
                }
            }
        }

        let chapters = probe
            .chapters
            .map(|chs| {
                chs.into_iter()
                    .enumerate()
                    .filter_map(|(i, ch)| {
                        let start_secs = ch.start_time.as_ref()?.parse::<f64>().ok()?;
                        let end_secs = ch.end_time.as_ref()?.parse::<f64>().ok()?;
                        let title = ch.tags.and_then(|t| t.get("title").cloned());
                        Some(Chapter {
                            index: i,
                            start_secs,
                            end_secs,
                            title,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(MediaAnalysis {
            path: path.to_string_lossy().to_string(),
            container_format,
            duration_secs,
            bitrate,
            size_bytes,
            video_streams,
            audio_streams,
            subtitle_streams,
            chapters,
            metadata,
        })
    }

    /// Convert ffprobe video stream to our VideoStream
    fn convert_video_stream(&self, stream: &ffprobe::Stream) -> Option<VideoStream> {
        let width = stream.width.or(stream.coded_width)?;
        let height = stream.height.or(stream.coded_height)?;

        if width == 0 || height == 0 {
            return None;
        }

        let codec = stream.codec_name.clone().unwrap_or_default();
        let codec_long_name = stream.codec_long_name.clone();

        let tags = stream.tags.clone().unwrap_or_default();
        let language = tags.get("language").cloned();
        let title = tags.get("title").cloned();

        let disposition = stream.disposition.as_ref();
        let is_default = disposition.and_then(|d| d.default).unwrap_or(0) == 1;

        // Parse bit depth from pix_fmt or bits_per_raw_sample
        let bit_depth = stream
            .bits_per_raw_sample
            .as_ref()
            .and_then(|b| b.parse::<u8>().ok())
            .or_else(|| detect_bit_depth(stream.pix_fmt.as_deref()));

        // Detect HDR type
        let hdr_type = detect_hdr_type(
            stream.color_transfer.as_deref(),
            stream.color_primaries.as_deref(),
            &codec,
            stream.side_data_list.as_ref(),
        );

        let bitrate = stream.bit_rate.as_ref().and_then(|b| b.parse::<i64>().ok());

        Some(VideoStream {
            index: stream.index,
            codec,
            codec_long_name,
            width,
            height,
            aspect_ratio: stream.display_aspect_ratio.clone(),
            frame_rate: stream.r_frame_rate.clone(),
            avg_frame_rate: stream.avg_frame_rate.clone(),
            bitrate,
            pixel_format: stream.pix_fmt.clone(),
            color_space: stream.color_space.clone(),
            color_transfer: stream.color_transfer.clone(),
            color_primaries: stream.color_primaries.clone(),
            hdr_type,
            bit_depth,
            language,
            title,
            is_default,
            metadata: tags,
        })
    }

    /// Convert ffprobe audio stream to our AudioStream
    fn convert_audio_stream(&self, stream: &ffprobe::Stream) -> Option<AudioStream> {
        let channels = stream.channels.unwrap_or(0);
        if channels == 0 {
            return None;
        }

        let codec = stream.codec_name.clone().unwrap_or_default();
        let codec_long_name = stream.codec_long_name.clone();

        let tags = stream.tags.clone().unwrap_or_default();
        let language = tags.get("language").cloned();
        let title = tags.get("title").cloned();

        let disposition = stream.disposition.as_ref();
        let is_default = disposition.and_then(|d| d.default).unwrap_or(0) == 1;
        let is_commentary = disposition.and_then(|d| d.comment).unwrap_or(0) == 1
            || title
                .as_ref()
                .map(|t| t.to_lowercase().contains("commentary"))
                .unwrap_or(false);

        let sample_rate = stream
            .sample_rate
            .as_ref()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        let bitrate = stream.bit_rate.as_ref().and_then(|b| b.parse::<i64>().ok());
        let bit_depth = stream.bits_per_sample;

        Some(AudioStream {
            index: stream.index,
            codec,
            codec_long_name,
            channels,
            channel_layout: stream.channel_layout.clone(),
            sample_rate,
            bitrate,
            bit_depth,
            language,
            title,
            is_default,
            is_commentary,
            metadata: tags,
        })
    }

    /// Convert ffprobe subtitle stream to our SubtitleStream
    fn convert_subtitle_stream(&self, stream: &ffprobe::Stream) -> Option<SubtitleStream> {
        let codec = stream.codec_name.clone().unwrap_or_default();
        let codec_long_name = stream.codec_long_name.clone();

        let tags = stream.tags.clone().unwrap_or_default();
        let language = tags.get("language").cloned();
        let title = tags.get("title").cloned();

        let disposition = stream.disposition.as_ref();
        let is_default = disposition.and_then(|d| d.default).unwrap_or(0) == 1;
        let is_forced = disposition.and_then(|d| d.forced).unwrap_or(0) == 1;
        let is_hearing_impaired = disposition.and_then(|d| d.hearing_impaired).unwrap_or(0) == 1;

        Some(SubtitleStream {
            index: stream.index,
            codec,
            codec_long_name,
            language,
            title,
            is_default,
            is_forced,
            is_hearing_impaired,
            metadata: tags,
        })
    }

    /// Get the primary video stream from an analysis
    pub fn primary_video_stream(analysis: &MediaAnalysis) -> Option<&VideoStream> {
        analysis
            .video_streams
            .iter()
            .find(|s| s.is_default)
            .or_else(|| analysis.video_streams.first())
    }

    /// Get the primary audio stream from an analysis
    pub fn primary_audio_stream(analysis: &MediaAnalysis) -> Option<&AudioStream> {
        analysis
            .audio_streams
            .iter()
            .find(|s| s.is_default && !s.is_commentary)
            .or_else(|| analysis.audio_streams.iter().find(|s| !s.is_commentary))
            .or_else(|| analysis.audio_streams.first())
    }

    /// Detect resolution category from video dimensions
    pub fn detect_resolution(width: u32, height: u32) -> &'static str {
        if height >= 2160 || width >= 3840 {
            "2160p"
        } else if height >= 1080 || width >= 1920 {
            "1080p"
        } else if height >= 720 || width >= 1280 {
            "720p"
        } else if height >= 480 || width >= 854 {
            "480p"
        } else {
            "SD"
        }
    }
}

impl Default for FfmpegService {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect HDR type from color metadata and side data
fn detect_hdr_type(
    color_transfer: Option<&str>,
    color_primaries: Option<&str>,
    codec_name: &str,
    side_data: Option<&Vec<ffprobe::SideData>>,
) -> Option<HdrType> {
    // Check for Dolby Vision in codec name or side data
    if codec_name.contains("dvhe") || codec_name.contains("dvh1") {
        return Some(HdrType::DolbyVision);
    }

    if let Some(side_data_list) = side_data {
        for sd in side_data_list {
            if let Some(ref sd_type) = sd.side_data_type {
                if sd_type.contains("Dolby Vision") {
                    return Some(HdrType::DolbyVision);
                }
                if sd_type.contains("HDR10+") || sd_type.contains("HDR10 Plus") {
                    return Some(HdrType::Hdr10Plus);
                }
            }
        }
    }

    // Check transfer characteristics for HDR10 or HLG
    match color_transfer {
        Some(transfer) if transfer.contains("smpte2084") => {
            // PQ transfer = HDR10 (could be HDR10+ but we need side data for that)
            // Also verify BT.2020 primaries for proper HDR10
            if color_primaries
                .map(|p| p.contains("bt2020"))
                .unwrap_or(false)
            {
                Some(HdrType::Hdr10)
            } else {
                Some(HdrType::Hdr10) // Still HDR10 even without bt2020
            }
        }
        Some(transfer) if transfer.contains("arib-std-b67") => Some(HdrType::Hlg),
        _ => None,
    }
}

/// Detect bit depth from pixel format string
fn detect_bit_depth(pixel_format: Option<&str>) -> Option<u8> {
    let pf = pixel_format?;

    if pf.contains("10le") || pf.contains("10be") || pf.contains("p10") || pf.ends_with("10") {
        Some(10)
    } else if pf.contains("12le") || pf.contains("12be") || pf.contains("p12") || pf.ends_with("12")
    {
        Some(12)
    } else if pf.contains("16le") || pf.contains("16be") {
        Some(16)
    } else if pf.contains("420p") || pf.contains("422p") || pf.contains("444p") || pf == "yuv420p" {
        Some(8)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_resolution() {
        assert_eq!(FfmpegService::detect_resolution(3840, 2160), "2160p");
        assert_eq!(FfmpegService::detect_resolution(1920, 1080), "1080p");
        assert_eq!(FfmpegService::detect_resolution(1280, 720), "720p");
        assert_eq!(FfmpegService::detect_resolution(854, 480), "480p");
        assert_eq!(FfmpegService::detect_resolution(640, 360), "SD");
    }

    #[test]
    fn test_detect_bit_depth() {
        assert_eq!(detect_bit_depth(Some("yuv420p")), Some(8));
        assert_eq!(detect_bit_depth(Some("yuv420p10le")), Some(10));
        assert_eq!(detect_bit_depth(Some("yuv420p12le")), Some(12));
        assert_eq!(detect_bit_depth(None), None);
    }

    #[test]
    fn test_hdr_type_display() {
        assert_eq!(HdrType::Hdr10.as_str(), "HDR10");
        assert_eq!(HdrType::DolbyVision.as_str(), "Dolby Vision");
        assert_eq!(HdrType::Hlg.as_str(), "HLG");
    }

    #[test]
    fn test_detect_hdr_type() {
        // HDR10
        assert_eq!(
            detect_hdr_type(Some("smpte2084"), Some("bt2020"), "hevc", None),
            Some(HdrType::Hdr10)
        );

        // HLG
        assert_eq!(
            detect_hdr_type(Some("arib-std-b67"), Some("bt2020"), "hevc", None),
            Some(HdrType::Hlg)
        );

        // Dolby Vision from codec
        assert_eq!(
            detect_hdr_type(None, None, "dvhe", None),
            Some(HdrType::DolbyVision)
        );

        // SDR
        assert_eq!(
            detect_hdr_type(Some("bt709"), Some("bt709"), "h264", None),
            None
        );
    }
}
