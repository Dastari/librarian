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
        // NOTE: PostgreSQL implementation commented out - keeping for reference
        // #[cfg(feature = "postgres")]
        // {
        //     sqlx::query(
        //         r#"
        //         UPDATE libraries SET
        //             auto_download_subtitles = COALESCE($2, auto_download_subtitles),
        //             preferred_subtitle_languages = COALESCE($3, preferred_subtitle_languages),
        //             updated_at = NOW()
        //         WHERE id = $1
        //         "#,
        //     )
        //     .bind(lib_id)
        //     .bind(input.auto_download)
        //     .bind(input.languages.as_ref())
        //     .execute(db.pool())
        //     .await
        //     .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        // }

        #[cfg(feature = "sqlite")]
        {
            use crate::db::sqlite_helpers::vec_to_json;
            let languages_json = input.languages.as_ref().map(|v| vec_to_json(v));
            sqlx::query(
                r#"
                UPDATE libraries SET
                    auto_download_subtitles = COALESCE(?2, auto_download_subtitles),
                    preferred_subtitle_languages = COALESCE(?3, preferred_subtitle_languages),
                    updated_at = datetime('now')
                WHERE id = ?1
                "#,
            )
            .bind(lib_id.to_string())
            .bind(input.auto_download)
            .bind(languages_json)
            .execute(db.pool())
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }

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

        // NOTE: PostgreSQL implementation commented out - keeping for reference
        // #[cfg(feature = "postgres")]
        // {
        //     sqlx::query(
        //         r#"
        //         UPDATE tv_shows SET
        //             subtitle_settings_override = $2,
        //             updated_at = NOW()
        //         WHERE id = $1
        //         "#,
        //     )
        //     .bind(show_uuid)
        //     .bind(&override_json)
        //     .execute(db.pool())
        //     .await
        //     .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        // }

        #[cfg(feature = "sqlite")]
        {
            sqlx::query(
                r#"
                UPDATE tv_shows SET
                    subtitle_settings_override = ?2,
                    updated_at = datetime('now')
                WHERE id = ?1
                "#,
            )
            .bind(show_uuid.to_string())
            .bind(&override_json)
            .execute(db.pool())
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }

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
    /// Note: This mutation is deprecated. Episodes no longer store torrent links directly.
    /// Use the auto-hunt workflow or add torrents manually via the downloads page.
    async fn download_episode(
        &self,
        ctx: &Context<'_>,
        episode_id: String,
    ) -> Result<DownloadEpisodeResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let ep_id = Uuid::parse_str(&episode_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid episode ID: {}", e)))?;

        // Get the episode
        let episode = db
            .episodes()
            .get_by_id(ep_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Episode not found"))?;

        // Episodes no longer store torrent links - downloads are initiated via auto-hunt
        // or by manually adding torrents and linking them to content
        Ok(DownloadEpisodeResult {
            success: false,
            episode: Some(Episode::from_record(episode, None)),
            error: Some("This mutation is deprecated. Use auto-hunt or add torrents manually.".to_string()),
        })
    }

    /// Re-match a media file against its library using stored metadata
    ///
    /// This uses the already-extracted embedded metadata stored in the media_files table
    /// to find the best matching album/show/movie/audiobook without re-reading from disk.
    async fn rematch_media_file(
        &self,
        ctx: &Context<'_>,
        media_file_id: String,
    ) -> Result<RematchMediaFileResult> {
        use crate::services::file_matcher::{FileMatcher, FileMatchTarget};

        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let file_id = Uuid::parse_str(&media_file_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid media file ID: {}", e)))?;

        // Get the media file
        let media_file = db
            .media_files()
            .get_by_id(file_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Media file not found"))?;

        // Get the library
        let library = db
            .libraries()
            .get_by_id(media_file.library_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Library not found"))?;

        // Check if metadata has been extracted
        if media_file.metadata_extracted_at.is_none() {
            return Ok(RematchMediaFileResult {
                success: false,
                matched: false,
                match_type: None,
                match_target_name: None,
                confidence: None,
                error: Some("Metadata has not been extracted for this file. Run a library scan first.".to_string()),
            });
        }

        // Create FileMatcher and run matching
        let matcher = FileMatcher::new(db.clone());
        match matcher.match_media_file(&media_file, &library).await {
            Ok(result) => {
                let matched = result.match_target.is_matched();
                
                if matched {
                    // Extract IDs from match_target
                    let (episode_id, movie_id, track_id, album_id, audiobook_id, match_type_str, target_name) = 
                        match &result.match_target {
                            FileMatchTarget::Episode { episode_id, show_name, season, episode, .. } => (
                                Some(*episode_id),
                                None,
                                None,
                                None,
                                None,
                                "episode",
                                format!("{} S{:02}E{:02}", show_name, season, episode),
                            ),
                            FileMatchTarget::Movie { movie_id, title, year, .. } => (
                                None,
                                Some(*movie_id),
                                None,
                                None,
                                None,
                                "movie",
                                if let Some(y) = year {
                                    format!("{} ({})", title, y)
                                } else {
                                    title.clone()
                                },
                            ),
                            FileMatchTarget::Track { track_id, album_id, title, .. } => (
                                None,
                                None,
                                Some(*track_id),
                                Some(*album_id),
                                None,
                                "track",
                                title.clone(),
                            ),
                            FileMatchTarget::Chapter { audiobook_id, chapter_number, .. } => (
                                None,
                                None,
                                None,
                                None,
                                Some(*audiobook_id),
                                "chapter",
                                format!("Chapter {}", chapter_number),
                            ),
                            _ => (None, None, None, None, None, "unknown", String::new()),
                        };

                    // Update the media_file with new match
                    let match_result = db.media_files()
                        .update_match(
                            file_id,
                            episode_id,
                            movie_id,
                            track_id,
                            album_id,
                            audiobook_id,
                            Some(match_type_str),
                        )
                        .await;

                    match match_result {
                        Ok(_) => {
                            tracing::info!(
                                media_file_id = %media_file_id,
                                match_type = %match_type_str,
                                target_name = %target_name,
                                confidence = result.confidence,
                                "Successfully rematched media file"
                            );

                            Ok(RematchMediaFileResult {
                                success: true,
                                matched: true,
                                match_type: Some(match_type_str.to_string()),
                                match_target_name: Some(target_name),
                                confidence: Some(format!("{:.2}", result.confidence)),
                                error: None,
                            })
                        }
                        Err(e) => Ok(RematchMediaFileResult {
                            success: false,
                            matched: false,
                            match_type: None,
                            match_target_name: None,
                            confidence: None,
                            error: Some(format!("Failed to update match: {}", e)),
                        }),
                    }
                } else {
                    let reason = match &result.match_target {
                        FileMatchTarget::Unmatched { reason } => reason.clone(),
                        FileMatchTarget::Sample => "File appears to be a sample".to_string(),
                        _ => "No match found".to_string(),
                    };
                    
                    Ok(RematchMediaFileResult {
                        success: true,
                        matched: false,
                        match_type: None,
                        match_target_name: None,
                        confidence: None,
                        error: Some(reason),
                    })
                }
            }
            Err(e) => Ok(RematchMediaFileResult {
                success: false,
                matched: false,
                match_type: None,
                match_target_name: None,
                confidence: None,
                error: Some(format!("Matching failed: {}", e)),
            }),
        }
    }

    /// Extract embedded metadata (ID3/Vorbis/container tags) from a media file
    ///
    /// This reads the file from disk and stores the extracted tags in the database.
    async fn extract_media_file_metadata(
        &self,
        ctx: &Context<'_>,
        media_file_id: String,
    ) -> Result<MutationResult> {
        use crate::services::file_matcher::read_audio_metadata;
        use crate::services::file_utils::is_audio_file;

        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let file_id = Uuid::parse_str(&media_file_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid media file ID: {}", e)))?;

        // Get the media file
        let media_file = db
            .media_files()
            .get_by_id(file_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Media file not found"))?;

        let file_path = std::path::Path::new(&media_file.path);

        // Check if file exists
        if !file_path.exists() {
            return Ok(MutationResult {
                success: false,
                error: Some("File does not exist on disk".to_string()),
            });
        }

        // Extract metadata based on file type
        let metadata = if is_audio_file(&media_file.path) {
            read_audio_metadata(&media_file.path)
        } else {
            // For video files, we'd use ffprobe - for now return None
            // TODO: Add ffprobe metadata extraction for video files
            None
        };

        match metadata {
            Some(meta) => {
                // Store in database
                // Note: For full album art/lyrics extraction, use the analysis queue
                let embedded = crate::db::EmbeddedMetadata {
                    artist: meta.artist,
                    album: meta.album,
                    title: meta.title,
                    track_number: meta.track_number.map(|n| n as i32),
                    disc_number: meta.disc_number.map(|n| n as i32),
                    year: meta.year,
                    genre: None,
                    show_name: None,
                    season: meta.season,
                    episode: meta.episode,
                    cover_art_base64: None, // Use analysis queue for full extraction
                    cover_art_mime: None,
                    lyrics: None,
                };

                db.media_files()
                    .update_embedded_metadata(file_id, &embedded)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                tracing::info!(
                    "Extracted metadata for file {}: Artist={:?}, Album={:?}, Title={:?}",
                    media_file_id,
                    embedded.artist,
                    embedded.album,
                    embedded.title
                );

                Ok(MutationResult {
                    success: true,
                    error: None,
                })
            }
            None => Ok(MutationResult {
                success: true,
                error: Some("No embedded metadata found in file".to_string()),
            }),
        }
    }
}
