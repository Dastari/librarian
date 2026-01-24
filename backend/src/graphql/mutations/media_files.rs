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
        // NOTE: Legacy implementation removed; SQLite handles this path.
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

        // NOTE: Legacy implementation removed; SQLite handles this path.
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
    ///
    /// Runs FFprobe on the file and stores all stream metadata (video, audio,
    /// subtitles, chapters) in the database. This is the same analysis that
    /// runs automatically during library scans.
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

        tracing::info!(
            media_file_id = %media_file_id,
            path = %file.path,
            "Running FFprobe analysis on media file"
        );

        // Run FFmpeg analysis
        let ffmpeg = crate::services::FfmpegService::new();
        let analysis = match ffmpeg.analyze(std::path::Path::new(&file.path)).await {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!(
                    media_file_id = %media_file_id,
                    path = %file.path,
                    error = %e,
                    "FFprobe analysis failed"
                );
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

        // Get counts before storing
        let video_count = analysis.video_streams.len() as i32;
        let audio_count = analysis.audio_streams.len() as i32;
        let subtitle_count = analysis.subtitle_streams.len() as i32;
        let chapter_count = analysis.chapters.len() as i32;

        // Store the analysis results using the same function as the background queue
        if let Err(e) = crate::services::queues::store_media_analysis(db, file_id, &analysis).await
        {
            tracing::error!(
                media_file_id = %media_file_id,
                error = %e,
                "Failed to store analysis results"
            );
            return Ok(AnalyzeMediaFileResult {
                success: false,
                error: Some(format!("Failed to store analysis: {}", e)),
                video_stream_count: Some(video_count),
                audio_stream_count: Some(audio_count),
                subtitle_stream_count: Some(subtitle_count),
                chapter_count: Some(chapter_count),
            });
        }

        tracing::info!(
            media_file_id = %media_file_id,
            video_streams = video_count,
            audio_streams = audio_count,
            subtitle_streams = subtitle_count,
            chapters = chapter_count,
            "Media file analyzed and stored"
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

    /// Queue all unanalyzed media files in a library for FFprobe analysis
    ///
    /// Finds all media files that have no video_codec set (indicating they
    /// haven't been analyzed) and queues them for analysis.
    async fn analyze_unanalyzed_files(
        &self,
        ctx: &Context<'_>,
        library_id: String,
    ) -> Result<AnalyzeUnanalyzedResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        // Get the analysis queue
        let queue = ctx.data_opt::<std::sync::Arc<crate::services::queues::MediaAnalysisQueue>>();
        if queue.is_none() {
            return Ok(AnalyzeUnanalyzedResult {
                success: false,
                queued_count: 0,
                error: Some("Analysis queue not available".to_string()),
            });
        }
        let queue = queue.unwrap();

        // Find files that haven't been analyzed (ffprobe_analyzed_at is null)
        let unanalyzed_files = db
            .media_files()
            .list_needing_ffprobe(lib_id, 10000)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let count = unanalyzed_files.len();

        if count == 0 {
            tracing::info!(library_id = %library_id, "No unanalyzed files found in library");
            return Ok(AnalyzeUnanalyzedResult {
                success: true,
                queued_count: 0,
                error: None,
            });
        }

        tracing::info!(
            library_id = %library_id,
            file_count = count,
            "Queueing unanalyzed files for FFprobe analysis"
        );

        // Queue each file for analysis
        let mut queued = 0;
        for file in unanalyzed_files {
            let job = crate::services::queues::MediaAnalysisJob {
                media_file_id: file.id,
                path: std::path::PathBuf::from(&file.path),
                check_subtitles: true,
            };
            if queue.submit(job).await.is_ok() {
                queued += 1;
            }
        }

        tracing::info!(
            library_id = %library_id,
            queued_count = queued,
            "Queued files for analysis"
        );

        Ok(AnalyzeUnanalyzedResult {
            success: true,
            queued_count: queued as i32,
            error: None,
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
                    ..Default::default()
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

    /// Manually match a media file to a library item
    ///
    /// Manual matches take precedence over automatic matches and will never be
    /// overwritten by the scanner or automatic matching systems.
    async fn manual_match(
        &self,
        ctx: &Context<'_>,
        media_file_id: String,
        #[graphql(desc = "Episode ID to match to")] episode_id: Option<String>,
        #[graphql(desc = "Movie ID to match to")] movie_id: Option<String>,
        #[graphql(desc = "Track ID to match to")] track_id: Option<String>,
        #[graphql(desc = "Album ID to match to")] album_id: Option<String>,
        #[graphql(desc = "Audiobook ID to match to")] audiobook_id: Option<String>,
        #[graphql(desc = "Chapter ID to match to")] chapter_id: Option<String>,
    ) -> Result<ManualMatchResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let file_id = Uuid::parse_str(&media_file_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid media file ID: {}", e)))?;

        // Parse target IDs
        let episode_uuid = episode_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| async_graphql::Error::new(format!("Invalid episode ID: {}", e)))?;

        let movie_uuid = movie_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| async_graphql::Error::new(format!("Invalid movie ID: {}", e)))?;

        let track_uuid = track_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| async_graphql::Error::new(format!("Invalid track ID: {}", e)))?;

        let album_uuid = album_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| async_graphql::Error::new(format!("Invalid album ID: {}", e)))?;

        let audiobook_uuid = audiobook_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| async_graphql::Error::new(format!("Invalid audiobook ID: {}", e)))?;

        let chapter_uuid = chapter_id
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| async_graphql::Error::new(format!("Invalid chapter ID: {}", e)))?;

        // Ensure at least one target is specified
        if episode_uuid.is_none()
            && movie_uuid.is_none()
            && track_uuid.is_none()
            && audiobook_uuid.is_none()
            && chapter_uuid.is_none()
        {
            return Ok(ManualMatchResult {
                success: false,
                error: Some("At least one target (episode, movie, track, audiobook, or chapter) must be specified".to_string()),
                media_file: None,
            });
        }

        // Verify the media file exists
        let existing = db.media_files().get_by_id(file_id).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if existing.is_none() {
            return Ok(ManualMatchResult {
                success: false,
                error: Some("Media file not found".to_string()),
                media_file: None,
            });
        }

        // Perform the manual match
        match db.media_files().manual_match(
            file_id,
            user_id,
            episode_uuid,
            movie_uuid,
            track_uuid,
            album_uuid,
            audiobook_uuid,
            chapter_uuid,
        ).await {
            Ok(media_file) => {
                tracing::info!(
                    media_file_id = %media_file_id,
                    user_id = %user.user_id,
                    episode_id = ?episode_id,
                    movie_id = ?movie_id,
                    track_id = ?track_id,
                    "Manual match applied"
                );

                Ok(ManualMatchResult {
                    success: true,
                    error: None,
                    media_file: Some(MediaFile::from(media_file)),
                })
            }
            Err(e) => Ok(ManualMatchResult {
                success: false,
                error: Some(e.to_string()),
                media_file: None,
            }),
        }
    }

    /// Remove a match from a media file
    ///
    /// Clears any match (manual or automatic) and allows the file to be re-matched.
    async fn unmatch_media_file(
        &self,
        ctx: &Context<'_>,
        media_file_id: String,
    ) -> Result<ManualMatchResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let file_id = Uuid::parse_str(&media_file_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid media file ID: {}", e)))?;

        // Verify the media file exists
        let existing = db.media_files().get_by_id(file_id).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if existing.is_none() {
            return Ok(ManualMatchResult {
                success: false,
                error: Some("Media file not found".to_string()),
                media_file: None,
            });
        }

        // Perform the unmatch
        match db.media_files().unmatch(file_id).await {
            Ok(media_file) => {
                tracing::info!(
                    media_file_id = %media_file_id,
                    "Match removed from media file"
                );

                Ok(ManualMatchResult {
                    success: true,
                    error: None,
                    media_file: Some(MediaFile::from(media_file)),
                })
            }
            Err(e) => Ok(ManualMatchResult {
                success: false,
                error: Some(e.to_string()),
                media_file: None,
            }),
        }
    }
}

/// Result of a manual match operation
#[derive(SimpleObject)]
pub struct ManualMatchResult {
    pub success: bool,
    pub error: Option<String>,
    pub media_file: Option<MediaFile>,
}
