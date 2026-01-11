//! FFmpeg-based transcoding service
//!
//! NOTE: This module is scaffolded for future HLS transcoding support.
//! Currently not integrated - the system prefers direct play.

use anyhow::Result;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// Transcoding service for HLS generation (for future direct play fallback)
#[allow(dead_code)]
pub struct Transcoder {
    cache_path: String,
}

#[allow(dead_code)]
impl Transcoder {
    pub fn new(cache_path: String) -> Self {
        Self { cache_path }
    }

    /// Generate HLS playlist and segments for a media file
    pub async fn transcode_to_hls(
        &self,
        input_path: &Path,
        session_id: &str,
        profile: TranscodeProfile,
    ) -> Result<String> {
        let output_dir = format!("{}/{}", self.cache_path, session_id);
        tokio::fs::create_dir_all(&output_dir).await?;

        let playlist_path = format!("{}/index.m3u8", output_dir);

        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-i", input_path.to_str().unwrap()])
            .args(["-c:v", "libx264"])
            .args(["-preset", "veryfast"])
            .args(["-crf", "23"])
            .args(profile.video_args())
            .args(["-c:a", "aac"])
            .args(["-b:a", "128k"])
            .args(["-ac", "2"])
            .args(["-f", "hls"])
            .args(["-hls_time", "4"])
            .args(["-hls_playlist_type", "event"])
            .args(["-hls_segment_filename", &format!("{}/segment_%03d.ts", output_dir)])
            .arg(&playlist_path)
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("FFmpeg failed: {}", stderr);
        }

        Ok(playlist_path)
    }

    /// Probe media file for stream information
    pub async fn probe(&self, path: &Path) -> Result<MediaInfo> {
        let output = Command::new("ffprobe")
            .args(["-v", "quiet"])
            .args(["-print_format", "json"])
            .args(["-show_format", "-show_streams"])
            .arg(path)
            .output()
            .await?;

        let info: MediaInfo = serde_json::from_slice(&output.stdout)?;
        Ok(info)
    }
}

/// Transcoding profile presets (for future transcoding)
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum TranscodeProfile {
    /// 1080p profile
    P1080,
    /// 720p profile
    P720,
    /// 480p profile
    P480,
}

impl TranscodeProfile {
    fn video_args(&self) -> Vec<&str> {
        match self {
            TranscodeProfile::P1080 => vec!["-vf", "scale=1920:-2", "-b:v", "4000k"],
            TranscodeProfile::P720 => vec!["-vf", "scale=1280:-2", "-b:v", "2500k"],
            TranscodeProfile::P480 => vec!["-vf", "scale=854:-2", "-b:v", "1000k"],
        }
    }
}

/// Media file information from ffprobe (for future transcoding)
#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
pub struct MediaInfo {
    pub format: MediaFormat,
    pub streams: Vec<MediaStream>,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
pub struct MediaFormat {
    pub filename: String,
    pub duration: Option<String>,
    pub size: Option<String>,
    pub format_name: String,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
pub struct MediaStream {
    pub index: i32,
    pub codec_type: String,
    pub codec_name: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
}
