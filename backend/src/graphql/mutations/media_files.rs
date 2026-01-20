use super::prelude::*;

#[derive(Default)]
pub struct MediaFileMutations;

#[Object]
impl MediaFileMutations {
    /// Update subtitle settings for a library
    async fn update_library_subtitle_settings(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        input: SubtitleSettingsInput,
    ) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        // Update library subtitle settings
        sqlx::query(
            r#"
            UPDATE libraries SET
                auto_download_subtitles = COALESCE($2, auto_download_subtitles),
                preferred_subtitle_languages = COALESCE($3, preferred_subtitle_languages),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(lib_id)
        .bind(input.auto_download)
        .bind(input.languages.as_ref())
        .execute(db.pool())
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        tracing::debug!("Updated library subtitle settings for {}", library_id);

        Ok(MutationResult {
            success: true,
            error: None,
        })
    }

    /// Update subtitle settings for a TV show (override library settings)
    async fn update_show_subtitle_settings(
        &self,
        ctx: &Context<'_>,
        show_id: String,
        input: SubtitleSettingsInput,
    ) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let show_uuid = Uuid::parse_str(&show_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid show ID: {}", e)))?;

        // Build the override JSON
        let override_json = serde_json::json!({
            "auto_download": input.auto_download,
            "languages": input.languages,
        });

        sqlx::query(
            r#"
            UPDATE tv_shows SET
                subtitle_settings_override = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(show_uuid)
        .bind(&override_json)
        .execute(db.pool())
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        tracing::debug!("Updated show subtitle settings for {}", show_id);

        Ok(MutationResult {
            success: true,
            error: None,
        })
    }

    /// Analyze a media file with FFmpeg to extract stream information
    async fn analyze_media_file(
        &self,
        ctx: &Context<'_>,
        media_file_id: String,
    ) -> Result<AnalyzeMediaFileResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let file_id = Uuid::parse_str(&media_file_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid media file ID: {}", e)))?;

        // Get the media file
        let file = db
            .media_files()
            .get_by_id(file_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Media file not found"))?;

        // Run FFmpeg analysis
        let ffmpeg = crate::services::FfmpegService::new();
        let analysis = match ffmpeg.analyze(std::path::Path::new(&file.path)).await {
            Ok(a) => a,
            Err(e) => {
                return Ok(AnalyzeMediaFileResult {
                    success: false,
                    error: Some(format!("FFmpeg analysis failed: {}", e)),
                    video_stream_count: None,
                    audio_stream_count: None,
                    subtitle_stream_count: None,
                    chapter_count: None,
                });
            }
        };

        // Store the analysis results
        // This is a simplified version - the full implementation is in queues.rs
        let video_count = analysis.video_streams.len() as i32;
        let audio_count = analysis.audio_streams.len() as i32;
        let subtitle_count = analysis.subtitle_streams.len() as i32;
        let chapter_count = analysis.chapters.len() as i32;

        tracing::info!(
            media_file_id = %media_file_id,
            video_streams = video_count,
            audio_streams = audio_count,
            subtitle_streams = subtitle_count,
            chapters = chapter_count,
            "Media file analyzed"
        );

        Ok(AnalyzeMediaFileResult {
            success: true,
            error: None,
            video_stream_count: Some(video_count),
            audio_stream_count: Some(audio_count),
            subtitle_stream_count: Some(subtitle_count),
            chapter_count: Some(chapter_count),
        })
    }

    /// Delete a subtitle (external or downloaded only)
    async fn delete_subtitle(
        &self,
        ctx: &Context<'_>,
        subtitle_id: String,
    ) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let sub_id = Uuid::parse_str(&subtitle_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid subtitle ID: {}", e)))?;

        // Get the subtitle first to check its type
        let subtitle = db
            .subtitles()
            .get_by_id(sub_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Subtitle not found"))?;

        // Can only delete external or downloaded subtitles, not embedded
        if subtitle.source_type == "embedded" {
            return Ok(MutationResult {
                success: false,
                error: Some("Cannot delete embedded subtitles".to_string()),
            });
        }

        // Delete the file if it's external or downloaded
        if let Some(ref file_path) = subtitle.file_path {
            if let Err(e) = tokio::fs::remove_file(file_path).await {
                tracing::warn!(path = %file_path, error = %e, "Failed to delete subtitle file");
            }
        }

        // Delete from database
        db.subtitles()
            .delete(sub_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: true,
            error: None,
        })
    }

    /// Manually trigger download for an available episode
    async fn download_episode(
        &self,
        ctx: &Context<'_>,
        episode_id: String,
    ) -> Result<DownloadEpisodeResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();

        let ep_id = Uuid::parse_str(&episode_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid episode ID: {}", e)))?;

        // Get the episode
        let episode = db
            .episodes()
            .get_by_id(ep_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Episode not found"))?;

        // Check if it has a torrent link
        let torrent_link = episode.torrent_link.clone().ok_or_else(|| {
            async_graphql::Error::new("Episode has no torrent link - not available for download")
        })?;

        // Get show info for logging
        let show = db
            .tv_shows()
            .get_by_id(episode.tv_show_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Show not found"))?;

        tracing::info!(
            user_id = %user.user_id,
            show_name = %show.name,
            season = episode.season,
            episode = episode.episode,
            "User manually downloading {} S{:02}E{:02}",
            show.name,
            episode.season,
            episode.episode
        );

        // Start the download
        let add_result = if torrent_link.starts_with("magnet:") {
            torrent_service.add_magnet(&torrent_link, None).await
        } else {
            torrent_service.add_torrent_url(&torrent_link, None).await
        };

        match add_result {
            Ok(_torrent_info) => {
                // Note: File-level matching happens automatically when torrent is processed
                // via torrent_file_matches table

                // Update episode status
                if let Err(e) = db.episodes().mark_downloading(episode.id).await {
                    tracing::error!("Failed to update episode status: {:?}", e);
                }

                // Episode just started downloading - no media file yet
                let mut ep = Episode::from_record(episode, None);
                ep.status = EpisodeStatus::Downloading;
                ep.torrent_link = Some(torrent_link);

                Ok(DownloadEpisodeResult {
                    success: true,
                    episode: Some(ep),
                    error: None,
                })
            }
            Err(e) => Ok(DownloadEpisodeResult {
                success: false,
                episode: None,
                error: Some(format!("Failed to start download: {}", e)),
            }),
        }
    }
}
