//! Application-specific work queues for background processing
//!
//! This module defines typed work queues for different background operations,
//! using the generic WorkQueue from job_queue.rs.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::ffmpeg::{FfmpegService, MediaAnalysis};
use super::job_queue::{JobQueueConfig, WorkQueue};
use crate::db::Database;

/// Job payload for media file analysis
#[derive(Debug, Clone)]
pub struct MediaAnalysisJob {
    /// ID of the media file in the database
    pub media_file_id: Uuid,
    /// Path to the media file
    pub path: PathBuf,
    /// Whether to queue subtitle download after analysis (if enabled)
    pub check_subtitles: bool,
}

/// Job payload for subtitle download
#[derive(Debug, Clone)]
pub struct SubtitleDownloadJob {
    /// ID of the media file to download subtitles for
    pub media_file_id: Uuid,
    /// Episode ID if linked to an episode
    pub episode_id: Option<Uuid>,
    /// Languages to download (ISO 639-1 codes)
    pub languages: Vec<String>,
    /// IMDB ID for better matching (if available)
    pub imdb_id: Option<String>,
    /// Show name for fallback search
    pub show_name: Option<String>,
    /// Season number
    pub season: Option<i32>,
    /// Episode number
    pub episode: Option<i32>,
}

/// Media analysis work queue
pub type MediaAnalysisQueue = WorkQueue<MediaAnalysisJob>;

/// Subtitle download work queue
pub type SubtitleDownloadQueue = WorkQueue<SubtitleDownloadJob>;

/// Configuration for the media analysis queue
pub fn media_analysis_queue_config() -> JobQueueConfig {
    JobQueueConfig {
        max_concurrent: 2,       // FFmpeg is CPU-intensive
        queue_capacity: 500,     // Allow queueing during large scans
        job_delay: Duration::from_millis(100),
    }
}

/// Configuration for the subtitle download queue
pub fn subtitle_download_queue_config() -> JobQueueConfig {
    JobQueueConfig {
        max_concurrent: 1,       // Strict rate limiting for API
        queue_capacity: 200,
        job_delay: Duration::from_secs(2),  // API allows ~1 req/sec, be conservative
    }
}

/// Create the media analysis queue with its processor
pub fn create_media_analysis_queue(
    ffmpeg: Arc<FfmpegService>,
    db: Database,
    subtitle_queue: Option<Arc<SubtitleDownloadQueue>>,
) -> MediaAnalysisQueue {
    let config = media_analysis_queue_config();

    WorkQueue::new("media_analysis", config, move |job: MediaAnalysisJob| {
        let ffmpeg = ffmpeg.clone();
        let db = db.clone();
        let subtitle_queue = subtitle_queue.clone();

        async move {
            if let Err(e) = process_media_analysis(ffmpeg, db, subtitle_queue, job).await {
                error!(error = %e, "Media analysis job failed");
            }
        }
    })
}

/// Process a single media analysis job
async fn process_media_analysis(
    ffmpeg: Arc<FfmpegService>,
    db: Database,
    _subtitle_queue: Option<Arc<SubtitleDownloadQueue>>,
    job: MediaAnalysisJob,
) -> Result<()> {
    info!(
        media_file_id = %job.media_file_id,
        path = %job.path.display(),
        "Processing media analysis job"
    );

    // Run FFmpeg analysis
    let analysis = match ffmpeg.analyze(&job.path).await {
        Ok(analysis) => analysis,
        Err(e) => {
            warn!(
                media_file_id = %job.media_file_id,
                path = %job.path.display(),
                error = %e,
                "Failed to analyze media file"
            );
            return Err(e);
        }
    };

    // Store analysis results in database
    store_media_analysis(&db, job.media_file_id, &analysis).await?;

    info!(
        media_file_id = %job.media_file_id,
        video_streams = analysis.video_streams.len(),
        audio_streams = analysis.audio_streams.len(),
        subtitle_streams = analysis.subtitle_streams.len(),
        "Media analysis stored"
    );

    // TODO: If check_subtitles is true and auto-download is enabled,
    // queue subtitle download jobs for missing languages

    Ok(())
}

/// Store media analysis results in the database
async fn store_media_analysis(
    db: &Database,
    media_file_id: Uuid,
    analysis: &MediaAnalysis,
) -> Result<()> {
    let pool = db.pool();

    // Get primary video and audio streams for the main file record
    let primary_video = FfmpegService::primary_video_stream(analysis);
    let primary_audio = FfmpegService::primary_audio_stream(analysis);

    // Update the main media_files record
    let duration = analysis.duration_secs.map(|d| d as i32);
    let bitrate = analysis.bitrate.map(|b| b as i32);

    let (width, height, video_codec, frame_rate, pixel_format, hdr_type, bit_depth, resolution) =
        if let Some(video) = primary_video {
            let res = FfmpegService::detect_resolution(video.width, video.height);
            (
                Some(video.width as i32),
                Some(video.height as i32),
                Some(video.codec.clone()),
                video.frame_rate.clone(),
                video.pixel_format.clone(),
                video.hdr_type.map(|h| h.as_str().to_string()),
                video.bit_depth.map(|b| b as i32),
                Some(res.to_string()),
            )
        } else {
            (None, None, None, None, None, None, None, None)
        };

    let (audio_codec, audio_channels, audio_language, sample_rate) =
        if let Some(audio) = primary_audio {
            (
                Some(audio.codec.clone()),
                audio.channel_layout.clone(),
                audio.language.clone(),
                Some(audio.sample_rate as i32),
            )
        } else {
            (None, None, None, None)
        };

    // Serialize full analysis for complete data preservation
    let analysis_json = serde_json::to_value(analysis)?;

    // Update media_files record
    sqlx::query(
        r#"
        UPDATE media_files SET
            duration = COALESCE($2, duration),
            bitrate = COALESCE($3, bitrate),
            width = COALESCE($4, width),
            height = COALESCE($5, height),
            video_codec = COALESCE($6, video_codec),
            audio_codec = COALESCE($7, audio_codec),
            audio_channels = COALESCE($8, audio_channels),
            audio_language = COALESCE($9, audio_language),
            container_format = $10,
            frame_rate = $11,
            pixel_format = $12,
            hdr_type = $13,
            bit_depth = $14,
            resolution = COALESCE($15, resolution),
            sample_rate = $16,
            chapter_count = $17,
            analysis_data = $18,
            analyzed_at = NOW(),
            modified_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(media_file_id)
    .bind(duration)
    .bind(bitrate)
    .bind(width)
    .bind(height)
    .bind(&video_codec)
    .bind(&audio_codec)
    .bind(&audio_channels)
    .bind(&audio_language)
    .bind(&analysis.container_format)
    .bind(&frame_rate)
    .bind(&pixel_format)
    .bind(&hdr_type)
    .bind(bit_depth)
    .bind(&resolution)
    .bind(sample_rate)
    .bind(analysis.chapters.len() as i32)
    .bind(&analysis_json)
    .execute(pool)
    .await?;

    // Clear existing stream data for this file (in case of re-analysis)
    sqlx::query("DELETE FROM video_streams WHERE media_file_id = $1")
        .bind(media_file_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM audio_streams WHERE media_file_id = $1")
        .bind(media_file_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM subtitles WHERE media_file_id = $1 AND source_type = 'embedded'")
        .bind(media_file_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM chapters WHERE media_file_id = $1")
        .bind(media_file_id)
        .execute(pool)
        .await?;

    // Insert video streams
    for video in &analysis.video_streams {
        let metadata_json = serde_json::to_value(&video.metadata)?;
        sqlx::query(
            r#"
            INSERT INTO video_streams (
                media_file_id, stream_index, codec, codec_long_name,
                width, height, aspect_ratio, frame_rate, avg_frame_rate,
                bitrate, pixel_format, color_space, color_transfer, color_primaries,
                hdr_type, bit_depth, language, title, is_default, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)
            "#,
        )
        .bind(media_file_id)
        .bind(video.index as i32)
        .bind(&video.codec)
        .bind(&video.codec_long_name)
        .bind(video.width as i32)
        .bind(video.height as i32)
        .bind(&video.aspect_ratio)
        .bind(&video.frame_rate)
        .bind(&video.avg_frame_rate)
        .bind(video.bitrate)
        .bind(&video.pixel_format)
        .bind(&video.color_space)
        .bind(&video.color_transfer)
        .bind(&video.color_primaries)
        .bind(video.hdr_type.map(|h| h.as_str()))
        .bind(video.bit_depth.map(|b| b as i32))
        .bind(&video.language)
        .bind(&video.title)
        .bind(video.is_default)
        .bind(&metadata_json)
        .execute(pool)
        .await?;
    }

    // Insert audio streams
    for audio in &analysis.audio_streams {
        let metadata_json = serde_json::to_value(&audio.metadata)?;
        sqlx::query(
            r#"
            INSERT INTO audio_streams (
                media_file_id, stream_index, codec, codec_long_name,
                channels, channel_layout, sample_rate, bitrate, bit_depth,
                language, title, is_default, is_commentary, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )
        .bind(media_file_id)
        .bind(audio.index as i32)
        .bind(&audio.codec)
        .bind(&audio.codec_long_name)
        .bind(audio.channels as i32)
        .bind(&audio.channel_layout)
        .bind(audio.sample_rate as i32)
        .bind(audio.bitrate)
        .bind(audio.bit_depth.map(|b| b as i32))
        .bind(&audio.language)
        .bind(&audio.title)
        .bind(audio.is_default)
        .bind(audio.is_commentary)
        .bind(&metadata_json)
        .execute(pool)
        .await?;
    }

    // Insert embedded subtitle streams
    for subtitle in &analysis.subtitle_streams {
        let metadata_json = serde_json::to_value(&subtitle.metadata)?;
        sqlx::query(
            r#"
            INSERT INTO subtitles (
                media_file_id, source_type, stream_index, codec, codec_long_name,
                language, title, is_default, is_forced, is_hearing_impaired, metadata
            ) VALUES ($1, 'embedded', $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(media_file_id)
        .bind(subtitle.index as i32)
        .bind(&subtitle.codec)
        .bind(&subtitle.codec_long_name)
        .bind(&subtitle.language)
        .bind(&subtitle.title)
        .bind(subtitle.is_default)
        .bind(subtitle.is_forced)
        .bind(subtitle.is_hearing_impaired)
        .bind(&metadata_json)
        .execute(pool)
        .await?;
    }

    // Insert chapters
    for chapter in &analysis.chapters {
        sqlx::query(
            r#"
            INSERT INTO chapters (media_file_id, chapter_index, start_secs, end_secs, title)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(media_file_id)
        .bind(chapter.index as i32)
        .bind(chapter.start_secs)
        .bind(chapter.end_secs)
        .bind(&chapter.title)
        .execute(pool)
        .await?;
    }

    debug!(
        media_file_id = %media_file_id,
        "Stored all stream and chapter data"
    );

    Ok(())
}

/// Placeholder for subtitle download queue processor
/// Will be implemented when OpenSubtitles client is ready
pub fn create_subtitle_download_queue(db: Database) -> SubtitleDownloadQueue {
    let config = subtitle_download_queue_config();

    WorkQueue::new(
        "subtitle_download",
        config,
        move |job: SubtitleDownloadJob| {
            let db = db.clone();
            async move {
                // TODO: Implement when OpenSubtitles client is ready
                debug!(
                    media_file_id = %job.media_file_id,
                    languages = ?job.languages,
                    "Subtitle download job queued (not yet implemented)"
                );
            }
        },
    )
}
