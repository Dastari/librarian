//! Application-specific work queues for background processing
//!
//! This module defines typed work queues for different background operations,
//! using the generic WorkQueue from job_queue.rs.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use sqlx;

use super::ffmpeg::{FfmpegService, MediaAnalysis};
use super::job_queue::{JobQueueConfig, WorkQueue};
use super::quality_evaluator::{EffectiveQualitySettings, QualityEvaluator, QualityStatus};
use crate::db::Database;
use crate::graphql::types::MediaFileUpdatedEvent;

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
        max_concurrent: 2,   // FFmpeg is CPU-intensive
        queue_capacity: 500, // Allow queueing during large scans
        job_delay: Duration::from_millis(100),
    }
}

/// Configuration for the subtitle download queue
pub fn subtitle_download_queue_config() -> JobQueueConfig {
    JobQueueConfig {
        max_concurrent: 1, // Strict rate limiting for API
        queue_capacity: 200,
        job_delay: Duration::from_secs(2), // API allows ~1 req/sec, be conservative
    }
}

/// Create the media analysis queue with its processor
pub fn create_media_analysis_queue(
    ffmpeg: Arc<FfmpegService>,
    db: Database,
    subtitle_queue: Option<Arc<SubtitleDownloadQueue>>,
    event_sender: Option<broadcast::Sender<MediaFileUpdatedEvent>>,
) -> MediaAnalysisQueue {
    let config = media_analysis_queue_config();

    WorkQueue::new("media_analysis", config, move |job: MediaAnalysisJob| {
        let ffmpeg = ffmpeg.clone();
        let db = db.clone();
        let subtitle_queue = subtitle_queue.clone();
        let event_sender = event_sender.clone();

        async move {
            if let Err(e) =
                process_media_analysis(ffmpeg, db, subtitle_queue, event_sender, job).await
            {
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
    event_sender: Option<broadcast::Sender<MediaFileUpdatedEvent>>,
    job: MediaAnalysisJob,
) -> Result<()> {
    // First check if the media file still exists before doing any work
    // (it may have been deleted as a duplicate during organization)
    if db
        .media_files()
        .get_by_id(job.media_file_id)
        .await?
        .is_none()
    {
        debug!(
            media_file_id = %job.media_file_id,
            path = %job.path.display(),
            "Media file no longer exists (likely deleted as duplicate), skipping analysis"
        );
        return Ok(());
    }

    let filename = job.path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    info!(
        media_file_id = %job.media_file_id,
        path = %job.path.display(),
        "Analyzing media file: '{}'",
        filename
    );

    // Run FFmpeg analysis
    let analysis = match ffmpeg.analyze(&job.path).await {
        Ok(analysis) => analysis,
        Err(e) => {
            warn!(
                media_file_id = %job.media_file_id,
                path = %job.path.display(),
                error = %e,
                "Failed to analyze '{}': {}",
                filename, e
            );
            return Err(e);
        }
    };

    // Store analysis results in database and get updated file info
    // Note: We check again inside store_media_analysis in case the file was deleted
    // between our check and now (during the FFmpeg analysis)
    let updated_info = match store_media_analysis(&db, job.media_file_id, &analysis).await {
        Ok(info) => info,
        Err(e) => {
            // Check if this is due to the media file being deleted (common during reorganization)
            if e.to_string().contains("no longer exists") {
                debug!(
                    media_file_id = %job.media_file_id,
                    "Media file was deleted during analysis (likely duplicate), skipping storage"
                );
                return Ok(());
            }
            return Err(e);
        }
    };

    // Build summary of analysis
    let video_summary = analysis.video_streams.first().map(|v| {
        let res = format!("{}x{}", v.width, v.height);
        format!("{} {}", res, &v.codec)
    }).unwrap_or_else(|| "no video".to_string());
    
    info!(
        media_file_id = %job.media_file_id,
        video_streams = analysis.video_streams.len(),
        audio_streams = analysis.audio_streams.len(),
        subtitle_streams = analysis.subtitle_streams.len(),
        "Analysis stored for '{}': {} | {} audio | {} subs",
        filename, video_summary, analysis.audio_streams.len(), analysis.subtitle_streams.len()
    );

    // Verify quality and update quality_status
    if let Err(e) =
        verify_and_update_quality(&db, job.media_file_id, &analysis, &updated_info).await
    {
        warn!(
            media_file_id = %job.media_file_id,
            error = %e,
            "Failed to verify quality"
        );
    }

    // Extract embedded metadata (ID3/Vorbis tags for audio, container tags for video)
    if let Err(e) = extract_and_store_embedded_metadata(&db, job.media_file_id, &job.path).await {
        debug!(
            media_file_id = %job.media_file_id,
            error = %e,
            "Failed to extract embedded metadata (may not have tags)"
        );
    }

    // Emit event for subscribers (UI updates)
    if let Some(sender) = event_sender {
        let event = MediaFileUpdatedEvent {
            media_file_id: job.media_file_id.to_string(),
            library_id: updated_info.library_id.to_string(),
            episode_id: updated_info.episode_id.map(|id| id.to_string()),
            movie_id: updated_info.movie_id.map(|id| id.to_string()),
            resolution: updated_info.resolution,
            video_codec: updated_info.video_codec,
            audio_codec: updated_info.audio_codec,
            audio_channels: updated_info.audio_channels,
            is_hdr: updated_info.is_hdr,
            hdr_type: updated_info.hdr_type,
            duration: updated_info.duration,
        };
        if let Err(e) = sender.send(event) {
            debug!(error = %e, "No subscribers for media file updated event");
        }
    }

    // TODO: If check_subtitles is true and auto-download is enabled,
    // queue subtitle download jobs for missing languages

    Ok(())
}

/// Info returned after storing analysis (for event emission)
struct AnalysisStoredInfo {
    library_id: Uuid,
    episode_id: Option<Uuid>,
    movie_id: Option<Uuid>,
    resolution: Option<String>,
    video_codec: Option<String>,
    audio_codec: Option<String>,
    audio_channels: Option<String>,
    is_hdr: Option<bool>,
    hdr_type: Option<String>,
    duration: Option<i32>,
}

/// Store media analysis results in the database
async fn store_media_analysis(
    db: &Database,
    media_file_id: Uuid,
    analysis: &MediaAnalysis,
) -> Result<AnalysisStoredInfo> {
    let pool = db.pool();

    // First, check if the media file still exists (it may have been deleted as a duplicate
    // during organization that runs concurrently with analysis)
    let media_file = db
        .media_files()
        .get_by_id(media_file_id)
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Media file {} no longer exists (likely deleted as duplicate during organization)",
                media_file_id
            )
        })?;

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

    sqlx::query("DELETE FROM media_chapters WHERE media_file_id = $1")
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

    // Insert chapters into media_chapters table
    for chapter in &analysis.chapters {
        sqlx::query(
            r#"
            INSERT INTO media_chapters (media_file_id, chapter_index, start_secs, end_secs, title)
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

    // Use the media_file we already fetched at the start for event emission
    // (library_id and episode_id won't change during analysis)
    Ok(AnalysisStoredInfo {
        library_id: media_file.library_id,
        episode_id: media_file.episode_id,
        movie_id: media_file.movie_id,
        resolution: primary_video
            .map(|v| FfmpegService::detect_resolution(v.width, v.height).to_string()),
        video_codec: primary_video.map(|v| v.codec.clone()),
        audio_codec: primary_audio.map(|a| a.codec.clone()),
        audio_channels: primary_audio.and_then(|a| a.channel_layout.clone()),
        is_hdr: primary_video.and_then(|v| v.hdr_type.map(|_| true)),
        hdr_type: primary_video.and_then(|v| v.hdr_type.map(|h| h.as_str().to_string())),
        duration: analysis.duration_secs.map(|d| d as i32),
    })
}

/// Placeholder for subtitle download queue processor
/// Will be implemented when OpenSubtitles client is ready
pub fn create_subtitle_download_queue(db: Database) -> SubtitleDownloadQueue {
    let config = subtitle_download_queue_config();

    WorkQueue::new(
        "subtitle_download",
        config,
        move |job: SubtitleDownloadJob| {
            let _db = db.clone();
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

/// Verify quality against library/item targets and update quality_status
///
/// After FFprobe analysis, we know the true quality of the file.
/// This function compares it against the target quality settings and:
/// 1. Updates media_files.quality_status
/// 2. Updates item status to 'suboptimal' if quality is below target
async fn verify_and_update_quality(
    db: &Database,
    media_file_id: Uuid,
    analysis: &MediaAnalysis,
    info: &AnalysisStoredInfo,
) -> Result<()> {
    // Get library to check quality settings
    let library = match db.libraries().get_by_id(info.library_id).await? {
        Some(l) => l,
        None => return Ok(()), // Library not found, skip
    };

    // Only verify quality for video files
    if analysis.video_streams.is_empty() {
        return Ok(());
    }

    // Get the primary video stream
    let primary_video = match FfmpegService::primary_video_stream(analysis) {
        Some(v) => v,
        None => return Ok(()),
    };

    // Determine actual resolution from video dimensions
    let actual_height = primary_video.height;
    let actual_resolution = FfmpegService::detect_resolution(primary_video.width, actual_height);

    // Get quality settings from item or library
    let quality_settings = if let Some(episode_id) = info.episode_id {
        // Get from TV show (with library fallback)
        if let Ok(Some(episode)) = db.episodes().get_by_id(episode_id).await {
            if let Ok(Some(show)) = db.tv_shows().get_by_id(episode.tv_show_id).await {
                EffectiveQualitySettings::from_tv_show(&show, &library)
            } else {
                EffectiveQualitySettings::from_library(&library)
            }
        } else {
            EffectiveQualitySettings::from_library(&library)
        }
    } else if let Some(movie_id) = info.movie_id {
        // Get from movie (with library fallback)
        if let Ok(Some(movie)) = db.movies().get_by_id(movie_id).await {
            EffectiveQualitySettings::from_movie(&movie, &library)
        } else {
            EffectiveQualitySettings::from_library(&library)
        }
    } else {
        EffectiveQualitySettings::from_library(&library)
    };

    // If library allows any quality, mark as optimal
    if quality_settings.allows_any() {
        update_quality_status(db, media_file_id, "optimal").await?;
        return Ok(());
    }

    // Evaluate the actual quality against settings
    let evaluation = QualityEvaluator::evaluate_analysis(analysis, &quality_settings);

    let quality_status_str = match evaluation.quality_status {
        QualityStatus::Optimal => "optimal",
        QualityStatus::Exceeds => "exceeds",
        QualityStatus::Suboptimal => "suboptimal",
        QualityStatus::Unknown => "unknown",
    };

    // Update quality_status on media_file
    update_quality_status(db, media_file_id, quality_status_str).await?;

    // If suboptimal, update the item's status
    if matches!(evaluation.quality_status, QualityStatus::Suboptimal) {
        if let Some(episode_id) = info.episode_id {
            info!(
                "Episode has suboptimal quality ({}p vs target), marking as suboptimal",
                actual_resolution
            );
            db.episodes()
                .update_status(episode_id, "suboptimal")
                .await
                .ok();
        } else if let Some(movie_id) = info.movie_id {
            info!(
                "Movie has suboptimal quality ({}p vs target), marking as suboptimal",
                actual_resolution
            );
            // Update movie download_status to suboptimal
            sqlx::query("UPDATE movies SET download_status = 'suboptimal' WHERE id = $1")
                .bind(movie_id)
                .execute(db.pool())
                .await
                .ok();
        }
    }

    Ok(())
}

/// Update the quality_status field on a media file
async fn update_quality_status(db: &Database, media_file_id: Uuid, status: &str) -> Result<()> {
    sqlx::query("UPDATE media_files SET quality_status = $1 WHERE id = $2")
        .bind(status)
        .execute(db.pool())
        .await?;

    debug!(
        media_file_id = %media_file_id,
        quality_status = %status,
        "Updated media file quality status"
    );

    Ok(())
}

/// Extract embedded metadata (ID3/Vorbis for audio, container tags for video)
/// and store in the media_files table
async fn extract_and_store_embedded_metadata(
    db: &Database,
    media_file_id: Uuid,
    path: &std::path::Path,
) -> Result<()> {
    use crate::db::EmbeddedMetadata;
    use crate::services::file_matcher::read_audio_metadata;
    use crate::services::file_utils::is_audio_file;

    let path_str = path.to_string_lossy();

    // Check if file is audio or video
    let metadata = if is_audio_file(&path_str) {
        // Use lofty for audio files (ID3/Vorbis/etc)
        read_audio_metadata(&path_str)
    } else {
        // For video files, we don't currently extract embedded metadata
        // TODO: Add ffprobe metadata extraction for video containers
        None
    };

    if let Some(meta) = metadata {
        let embedded = EmbeddedMetadata {
            artist: meta.artist,
            album: meta.album,
            title: meta.title,
            track_number: meta.track_number.map(|n| n as i32),
            disc_number: meta.disc_number.map(|n| n as i32),
            year: meta.year,
            genre: None, // extend if lofty exposes genre
            show_name: None,
            season: meta.season,
            episode: meta.episode,
        };

        db.media_files()
            .update_embedded_metadata(media_file_id, &embedded)
            .await?;

        info!(
            media_file_id = %media_file_id,
            artist = ?embedded.artist,
            album = ?embedded.album,
            title = ?embedded.title,
            "Extracted and stored embedded metadata"
        );
    } else {
        // Mark as extracted even if no metadata found (to prevent re-extraction)
        sqlx::query(
            "UPDATE media_files SET metadata_extracted_at = NOW() WHERE id = $1"
        )
        .bind(media_file_id)
        .execute(db.pool())
        .await?;

        debug!(
            media_file_id = %media_file_id,
            "No embedded metadata found in file"
        );
    }

    Ok(())
}
