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
#[cfg(feature = "sqlite")]
use crate::db::sqlite_helpers::uuid_to_str;
use crate::services::graphql::types::MediaFileUpdatedEvent;

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

/// Job payload for audio fingerprinting
#[derive(Debug, Clone)]
pub struct FingerprintJob {
    /// ID of the media file in the database
    pub media_file_id: Uuid,
    /// Path to the media file
    pub path: PathBuf,
    /// Whether to look up the fingerprint in AcoustID
    pub lookup_acoustid: bool,
}

/// Media analysis work queue
pub type MediaAnalysisQueue = WorkQueue<MediaAnalysisJob>;

/// Subtitle download work queue
pub type SubtitleDownloadQueue = WorkQueue<SubtitleDownloadJob>;

/// Audio fingerprint work queue
pub type FingerprintQueue = WorkQueue<FingerprintJob>;

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

/// Configuration for the fingerprint queue
pub fn fingerprint_queue_config() -> JobQueueConfig {
    JobQueueConfig {
        max_concurrent: 2, // fpcalc is moderately CPU-intensive
        queue_capacity: 500,
        job_delay: Duration::from_millis(100),
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
                let error_str = e.to_string();
                // FOREIGN KEY constraint failures indicate the media file was deleted
                // during analysis (common during duplicate cleanup) - this is not an error
                if error_str.contains("FOREIGN KEY constraint failed") 
                    || error_str.contains("no longer exists") 
                {
                    debug!(
                        error = %e, 
                        "Media file was deleted during analysis (likely duplicate cleanup), skipping"
                    );
                } else {
                    error!(error = %e, "Media analysis job failed");
                }
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
            "Failed to verify quality for '{}': {}",
            filename, e
        );
    }

    // Extract embedded metadata (ID3/Vorbis tags for audio, container tags for video)
    if let Err(e) = extract_and_store_embedded_metadata(&db, job.media_file_id, &job.path).await {
        debug!(
            "No embedded metadata extracted from '{}' (may not have tags): {}",
            filename, e
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
pub struct AnalysisStoredInfo {
    pub library_id: Uuid,
    pub episode_id: Option<Uuid>,
    pub movie_id: Option<Uuid>,
    pub resolution: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub audio_channels: Option<String>,
    pub is_hdr: Option<bool>,
    pub hdr_type: Option<String>,
    pub duration: Option<i32>,
}

/// Store media analysis results in the database
///
/// This function persists FFprobe analysis data to the media_files record,
/// including video/audio stream details, resolution, HDR info, etc.
/// Also stores detailed stream info in media_file_video_streams,
/// media_file_audio_streams, media_file_subtitles, and media_file_chapters tables.
pub async fn store_media_analysis(
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
    let query = r#"
        UPDATE media_files SET
            duration = COALESCE(?2, duration),
            bitrate = COALESCE(?3, bitrate),
            width = COALESCE(?4, width),
            height = COALESCE(?5, height),
            video_codec = COALESCE(?6, video_codec),
            audio_codec = COALESCE(?7, audio_codec),
            audio_channels = COALESCE(?8, audio_channels),
            audio_language = COALESCE(?9, audio_language),
            container_format = ?10,
            frame_rate = ?11,
            pixel_format = ?12,
            hdr_type = ?13,
            bit_depth = ?14,
            resolution = COALESCE(?15, resolution),
            sample_rate = ?16,
            chapter_count = ?17,
            analysis_data = ?18,
            ffprobe_analyzed_at = datetime('now'),
            modified_at = datetime('now')
        WHERE id = ?1
        "#;

    sqlx::query(query)
    .bind(uuid_to_str(media_file_id))
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
    let media_file_id_str = uuid_to_str(media_file_id);
    
    sqlx::query("DELETE FROM video_streams WHERE media_file_id = ?1")
        .bind(&media_file_id_str)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM audio_streams WHERE media_file_id = ?1")
        .bind(&media_file_id_str)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM subtitles WHERE media_file_id = ?1 AND source_type = 'embedded'")
        .bind(&media_file_id_str)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM media_chapters WHERE media_file_id = ?1")
        .bind(&media_file_id_str)
        .execute(pool)
        .await?;

    // Insert video streams
    for video in &analysis.video_streams {
        let metadata_json = serde_json::to_value(&video.metadata)?;
        let stream_id = uuid_to_str(Uuid::new_v4());
        sqlx::query(
            r#"
            INSERT INTO video_streams (
                id, media_file_id, stream_index, codec, codec_long_name,
                width, height, aspect_ratio, frame_rate, avg_frame_rate,
                bitrate, pixel_format, color_space, color_transfer, color_primaries,
                hdr_type, bit_depth, language, title, is_default, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
            "#,
        )
        .bind(&stream_id)
        .bind(&media_file_id_str)
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
        let stream_id = uuid_to_str(Uuid::new_v4());
        sqlx::query(
            r#"
            INSERT INTO audio_streams (
                id, media_file_id, stream_index, codec, codec_long_name,
                channels, channel_layout, sample_rate, bitrate, bit_depth,
                language, title, is_default, is_commentary, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
        )
        .bind(&stream_id)
        .bind(&media_file_id_str)
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
        let subtitle_id = uuid_to_str(Uuid::new_v4());
        sqlx::query(
            r#"
            INSERT INTO subtitles (
                id, media_file_id, source_type, stream_index, codec, codec_long_name,
                language, title, is_default, is_forced, is_hearing_impaired, metadata
            ) VALUES (?1, ?2, 'embedded', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
        )
        .bind(&subtitle_id)
        .bind(&media_file_id_str)
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
        let chapter_id = uuid_to_str(Uuid::new_v4());
        sqlx::query(
            r#"
            INSERT INTO media_chapters (id, media_file_id, chapter_index, start_secs, end_secs, title)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(&chapter_id)
        .bind(&media_file_id_str)
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

/// Create the fingerprint queue with its processor
pub fn create_fingerprint_queue(
    db: Database,
    acoustid_api_key: Option<String>,
) -> FingerprintQueue {
    let config = fingerprint_queue_config();

    WorkQueue::new("fingerprint", config, move |job: FingerprintJob| {
        let db = db.clone();
        let api_key = acoustid_api_key.clone();

        async move {
            if let Err(e) = process_fingerprint_job(db, api_key, job).await {
                error!(error = %e, "Fingerprint job failed");
            }
        }
    })
}

/// Process a single fingerprint job
async fn process_fingerprint_job(
    db: Database,
    acoustid_api_key: Option<String>,
    job: FingerprintJob,
) -> Result<()> {
    use super::fingerprint::FingerprintService;

    // Check if file still exists in database
    if db.media_files().get_by_id(job.media_file_id).await?.is_none() {
        debug!(
            media_file_id = %job.media_file_id,
            "Media file no longer exists, skipping fingerprinting"
        );
        return Ok(());
    }

    // Check if fpcalc is available
    let service = FingerprintService::with_config(db.clone(), None, acoustid_api_key.clone());
    if !service.is_available().await {
        debug!("fpcalc not available, skipping fingerprinting");
        return Ok(());
    }

    let filename = job.path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    info!(
        media_file_id = %job.media_file_id,
        path = %job.path.display(),
        "Generating fingerprint for '{}'",
        filename
    );

    // Generate fingerprint
    if job.lookup_acoustid && acoustid_api_key.is_some() {
        // Full identification with AcoustID lookup
        match service.identify_track(job.media_file_id, &job.path).await {
            Ok(Some(match_info)) => {
                info!(
                    media_file_id = %job.media_file_id,
                    acoustid = %match_info.acoustid_id,
                    title = ?match_info.title,
                    artist = ?match_info.artist,
                    "Track identified: '{}'",
                    filename
                );
            }
            Ok(None) => {
                info!(
                    media_file_id = %job.media_file_id,
                    "Fingerprint stored but no AcoustID match for '{}'",
                    filename
                );
            }
            Err(e) => {
                warn!(
                    media_file_id = %job.media_file_id,
                    error = %e,
                    "Fingerprinting failed for '{}'",
                    filename
                );
            }
        }
    } else {
        // Just generate and store fingerprint without lookup
        match service.fingerprint_and_store(job.media_file_id, &job.path).await {
            Ok(_) => {
                debug!(
                    media_file_id = %job.media_file_id,
                    "Fingerprint stored for '{}'",
                    filename
                );
            }
            Err(e) => {
                warn!(
                    media_file_id = %job.media_file_id,
                    error = %e,
                    "Fingerprinting failed for '{}'",
                    filename
                );
            }
        }
    }

    Ok(())
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
                episode_id = %episode_id,
                resolution = %actual_resolution,
                "Episode has suboptimal quality - media file exists but quality could be upgraded"
            );
            // Note: Episode status is derived from media_file_id presence.
            // "suboptimal" quality is tracked via the media_file metadata,
            // and auto-hunt can be used to find better versions.
        } else if let Some(movie_id) = info.movie_id {
            info!(
                movie_id = %movie_id,
                resolution = %actual_resolution,
                "Movie has suboptimal quality - media file exists but quality could be upgraded"
            );
            // Note: Movie download_status is derived from media_file_id presence.
            // Quality evaluation metadata is stored on the media_file record.
        }
    }

    Ok(())
}

/// Update the quality_status field on a media file
async fn update_quality_status(db: &Database, media_file_id: Uuid, status: &str) -> Result<()> {
    sqlx::query("UPDATE media_files SET quality_status = ?1 WHERE id = ?2")
        .bind(status)
        .bind(uuid_to_str(media_file_id))
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
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use crate::db::EmbeddedMetadata;
    use crate::services::file_utils::is_audio_file;

    let path_str = path.to_string_lossy();

    // Check if file is audio or video
    let embedded = if is_audio_file(&path_str) {
        // Use lofty for audio files - extract all metadata including album art and lyrics
        extract_audio_metadata_with_art(&path_str)
    } else {
        // For video files, we don't currently extract embedded metadata
        // TODO: Add ffprobe metadata extraction for video containers
        None
    };

    if let Some(meta) = embedded {
        let has_art = meta.cover_art_base64.is_some();
        let has_lyrics = meta.lyrics.is_some();
        
        db.media_files()
            .update_embedded_metadata(media_file_id, &meta)
            .await?;

        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
        info!(
            "Extracted metadata from '{}': Artist={:?}, Album={:?}, Title={:?}, CoverArt={}, Lyrics={}",
            filename,
            meta.artist,
            meta.album,
            meta.title,
            has_art,
            has_lyrics
        );
    } else {
        // Mark as extracted even if no metadata found (to prevent re-extraction)
        #[cfg(feature = "sqlite")]
        let query = "UPDATE media_files SET metadata_extracted_at = datetime('now') WHERE id = ?1";

        sqlx::query(query)
        .bind(uuid_to_str(media_file_id))
        .execute(db.pool())
        .await?;

        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
        debug!(
            "No embedded metadata found in '{}'",
            filename
        );
    }

    Ok(())
}

/// Extract comprehensive audio metadata including album art and lyrics using lofty
/// 
/// Supports metadata from:
/// - ID3v1/v2 (MP3)
/// - Vorbis Comments (FLAC, OGG, Opus)
/// - APEv2 (APE, WavPack)
/// - MP4/iTunes atoms (M4A, AAC, ALAC)
/// - WMA (ASF) tags
/// - RIFF INFO (WAV)
fn extract_audio_metadata_with_art(path: &str) -> Option<crate::db::EmbeddedMetadata> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use lofty::prelude::*;
    use lofty::probe::Probe;
    use lofty::tag::ItemKey;

    let tagged_file = Probe::open(path).ok()?.read().ok()?;
    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag())?;

    // Helper to get a string tag value
    let get_str = |key: ItemKey| tag.get_string(&key).map(|s| s.to_string());
    
    // =========================================================================
    // Basic metadata
    // =========================================================================
    let title = tag.title().map(|s| s.to_string());
    let artist = tag.artist().map(|s| s.to_string());
    let album = tag.album().map(|s| s.to_string());
    let year = tag.year().map(|y| y as i32);
    let track_number = tag.track().map(|n| n as i32);
    let disc_number = tag.disk().map(|n| n as i32);
    let genre = tag.genre().map(|s| s.to_string());

    // =========================================================================
    // Extended audio metadata
    // =========================================================================
    
    // Album Artist (iTunes: aART, ID3: TPE2, Vorbis: ALBUMARTIST)
    let album_artist = get_str(ItemKey::AlbumArtist);
    
    // Composer (iTunes: ©wrt, ID3: TCOM, Vorbis: COMPOSER)
    let composer = get_str(ItemKey::Composer);
    
    // Conductor (ID3: TPE3, Vorbis: CONDUCTOR)
    let conductor = get_str(ItemKey::Conductor);
    
    // Label/Publisher (ID3: TPUB, Vorbis: LABEL/ORGANIZATION)
    let label = get_str(ItemKey::Label)
        .or_else(|| get_str(ItemKey::Publisher));
    
    // Catalog number (Vorbis: CATALOGNUMBER)
    let catalog_number = get_str(ItemKey::CatalogNumber);
    
    // ISRC (ID3: TSRC, Vorbis: ISRC)
    let isrc = get_str(ItemKey::Isrc);
    
    // BPM (ID3: TBPM, Vorbis: BPM)
    let bpm = get_str(ItemKey::Bpm).and_then(|s| s.parse::<i32>().ok());
    
    // Initial Key (ID3: TKEY, Vorbis: INITIALKEY)
    let initial_key = get_str(ItemKey::InitialKey);
    
    // Compilation flag (iTunes: cpil, ID3: TCMP, Vorbis: COMPILATION)
    let is_compilation = get_str(ItemKey::FlagCompilation)
        .map(|s| s == "1" || s.to_lowercase() == "true");
    
    // Gapless playback (iTunes: pgap)
    // Note: lofty may not have a direct key for this
    let gapless_playback: Option<bool> = None;
    
    // Rating/Popularity (ID3: POPM, various formats)
    // Ratings are often 0-255 in ID3, we normalize to 0-100
    let rating = get_str(ItemKey::Popularimeter)
        .and_then(|s| s.parse::<i32>().ok())
        .map(|r| (r as f32 / 255.0 * 100.0) as i32);
    
    // Play count (from embedded metadata, not common)
    let play_count: Option<i32> = None;

    // =========================================================================
    // ReplayGain / Loudness normalization
    // =========================================================================
    
    // ReplayGain tags (Vorbis: REPLAYGAIN_*, ID3: TXXX with specific descriptions)
    let replaygain_track_gain = get_str(ItemKey::ReplayGainTrackGain)
        .and_then(|s| parse_replaygain_value(&s));
    
    let replaygain_track_peak = get_str(ItemKey::ReplayGainTrackPeak)
        .and_then(|s| s.parse::<f64>().ok());
    
    let replaygain_album_gain = get_str(ItemKey::ReplayGainAlbumGain)
        .and_then(|s| parse_replaygain_value(&s));
    
    let replaygain_album_peak = get_str(ItemKey::ReplayGainAlbumPeak)
        .and_then(|s| s.parse::<f64>().ok());
    
    // SoundCheck (iTunes normalization)
    // Stored in iTunes as iTunNORM in a comment
    let soundcheck_value = get_str(ItemKey::Unknown("iTunNORM".to_string()));

    // =========================================================================
    // Album art
    // =========================================================================
    let (cover_art_base64, cover_art_mime) = tag.pictures().first().map(|pic| {
        let base64_data = BASE64.encode(pic.data());
        let mime = match pic.mime_type() {
            Some(lofty::picture::MimeType::Jpeg) => "image/jpeg",
            Some(lofty::picture::MimeType::Png) => "image/png",
            Some(lofty::picture::MimeType::Gif) => "image/gif",
            Some(lofty::picture::MimeType::Bmp) => "image/bmp",
            Some(lofty::picture::MimeType::Tiff) => "image/tiff",
            _ => "image/jpeg",
        };
        (Some(base64_data), Some(mime.to_string()))
    }).unwrap_or((None, None));

    // =========================================================================
    // Lyrics
    // =========================================================================
    let lyrics = get_str(ItemKey::Lyrics)
        .or_else(|| get_str(ItemKey::Unknown("UNSYNCEDLYRICS".to_string())));

    // =========================================================================
    // Video/Container metadata (for video files with embedded audio)
    // =========================================================================
    let director: Option<String> = None; // Would need FFmpeg for video files
    let producer: Option<String> = None;
    let copyright = get_str(ItemKey::CopyrightMessage);
    let encoder = get_str(ItemKey::EncodedBy)
        .or_else(|| get_str(ItemKey::EncoderSoftware));
    let creation_date: Option<String> = None;

    // =========================================================================
    // BWF (Broadcast Wave Format) - typically extracted via FFmpeg for WAV
    // =========================================================================
    let bwf_originator: Option<String> = None;
    let bwf_originator_reference: Option<String> = None;
    let bwf_origination_date: Option<String> = None;
    let bwf_origination_time: Option<String> = None;
    let bwf_time_reference: Option<i64> = None;
    let bwf_umid: Option<String> = None;
    let bwf_coding_history: Option<String> = None;

    Some(crate::db::EmbeddedMetadata {
        artist,
        album,
        title,
        track_number,
        disc_number,
        year,
        genre,
        show_name: None,
        season: None,
        episode: None,
        cover_art_base64,
        cover_art_mime,
        lyrics,
        album_artist,
        composer,
        conductor,
        label,
        catalog_number,
        isrc,
        bpm,
        initial_key,
        is_compilation,
        gapless_playback,
        rating,
        play_count,
        replaygain_track_gain,
        replaygain_track_peak,
        replaygain_album_gain,
        replaygain_album_peak,
        soundcheck_value,
        director,
        producer,
        copyright,
        encoder,
        creation_date,
        bwf_originator,
        bwf_originator_reference,
        bwf_origination_date,
        bwf_origination_time,
        bwf_time_reference,
        bwf_umid,
        bwf_coding_history,
    })
}

/// Parse a ReplayGain value string like "-6.5 dB" or "-6.5"
fn parse_replaygain_value(s: &str) -> Option<f64> {
    // Remove "dB" suffix if present and parse
    let cleaned = s.trim().trim_end_matches("dB").trim_end_matches("db").trim();
    cleaned.parse::<f64>().ok()
}
