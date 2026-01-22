use super::prelude::*;

#[derive(Default)]
pub struct TvShowMutations;

#[Object]
impl TvShowMutations {
    /// Add a TV show to a library
    ///
    /// Uses the unified add_tv_show_from_provider method which handles:
    /// - Creating the TV show record with normalized status
    /// - Fetching and creating all episodes
    /// - Updating show statistics
    async fn add_tv_show(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        input: AddTvShowInput,
    ) -> Result<TvShowResult> {
        let user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        // Parse the provider
        let provider = match input.provider.as_str() {
            "tvmaze" => crate::services::MetadataProvider::TvMaze,
            "tmdb" => crate::services::MetadataProvider::Tmdb,
            "tvdb" => crate::services::MetadataProvider::TvDb,
            _ => return Err(async_graphql::Error::new("Invalid provider")),
        };

        // Convert monitor type
        let monitor_type = input
            .monitor_type
            .map(|mt| match mt {
                MonitorType::All => "all",
                MonitorType::Future => "future",
                MonitorType::None => "none",
            })
            .unwrap_or("all")
            .to_string();

        // Use the unified service method
        let record: crate::db::TvShowRecord = metadata
            .add_tv_show_from_provider(crate::services::AddTvShowOptions {
                provider,
                provider_id: input.provider_id as u32,
                library_id: lib_id,
                user_id,
                monitored: true,
                monitor_type,
                path: input.path,
            })
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        tracing::info!(
            user_id = %user.user_id,
            show_name = %record.name,
            show_id = %record.id,
            library_id = %lib_id,
            "User added TV show: {}",
            record.name
        );

        // Trigger immediate downloads for available episodes if backfill is enabled
        if record.backfill_existing {
            let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();
            let db = ctx.data_unchecked::<Database>();

            // Run async download in background (don't block the response)
            let show_id = record.id;
            let show_name = record.name.clone();
            let db_clone = db.clone();
            let torrent_clone = torrent_service.clone();

            tokio::spawn(async move {
                match crate::jobs::auto_download::download_available_for_show(
                    &db_clone,
                    torrent_clone,
                    show_id,
                )
                .await
                {
                    Ok(count) => {
                        if count > 0 {
                            tracing::info!(
                                show_id = %show_id,
                                show_name = %show_name,
                                count = count,
                                "Started downloading available episodes for new show: {}",
                                show_name
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            show_id = %show_id,
                            show_name = %show_name,
                            error = %e,
                            "Failed to start downloads for new show: {}",
                            show_name
                        );
                    }
                }
            });
        }

        Ok(TvShowResult {
            success: true,
            tv_show: Some(TvShow {
                id: record.id.to_string(),
                library_id: record.library_id.to_string(),
                name: record.name,
                sort_name: record.sort_name,
                year: record.year,
                status: match record.status.as_str() {
                    "continuing" | "Running" => TvShowStatus::Continuing,
                    "ended" | "Ended" => TvShowStatus::Ended,
                    _ => TvShowStatus::Unknown,
                },
                tvmaze_id: record.tvmaze_id,
                tmdb_id: record.tmdb_id,
                tvdb_id: record.tvdb_id,
                imdb_id: record.imdb_id,
                overview: record.overview,
                network: record.network,
                runtime: record.runtime,
                genres: record.genres,
                poster_url: record.poster_url,
                backdrop_url: record.backdrop_url,
                monitored: record.monitored,
                monitor_type: match record.monitor_type.as_str() {
                    "all" => MonitorType::All,
                    "future" => MonitorType::Future,
                    _ => MonitorType::None,
                },
                path: record.path,
                auto_download_override: record.auto_download_override,
                backfill_existing: record.backfill_existing,
                organize_files_override: record.organize_files_override,
                rename_style_override: record.rename_style_override,
                auto_hunt_override: record.auto_hunt_override,
                episode_count: record.episode_count.unwrap_or(0),
                episode_file_count: record.episode_file_count.unwrap_or(0),
                size_bytes: record.size_bytes.unwrap_or(0),
                // Quality override settings
                allowed_resolutions_override: record.allowed_resolutions_override,
                allowed_video_codecs_override: record.allowed_video_codecs_override,
                allowed_audio_formats_override: record.allowed_audio_formats_override,
                require_hdr_override: record.require_hdr_override,
                allowed_hdr_types_override: record.allowed_hdr_types_override,
                allowed_sources_override: record.allowed_sources_override,
                release_group_blacklist_override: record.release_group_blacklist_override,
                release_group_whitelist_override: record.release_group_whitelist_override,
            }),
            error: None,
        })
    }

    /// Update a TV show
    async fn update_tv_show(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateTvShowInput,
    ) -> Result<TvShowResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let show_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid show ID: {}", e)))?;

        let monitor_type = input.monitor_type.map(|mt| {
            match mt {
                MonitorType::All => "all",
                MonitorType::Future => "future",
                MonitorType::None => "none",
            }
            .to_string()
        });

        let result = db
            .tv_shows()
            .update(
                show_id,
                UpdateTvShow {
                    monitored: input.monitored,
                    monitor_type,
                    path: input.path,
                    auto_download_override: input.auto_download_override,
                    backfill_existing: input.backfill_existing,
                    organize_files_override: input.organize_files_override,
                    rename_style_override: input.rename_style_override,
                    auto_hunt_override: input.auto_hunt_override,
                    // Quality override settings
                    allowed_resolutions_override: input.allowed_resolutions_override,
                    allowed_video_codecs_override: input.allowed_video_codecs_override,
                    allowed_audio_formats_override: input.allowed_audio_formats_override,
                    require_hdr_override: input.require_hdr_override,
                    allowed_hdr_types_override: input.allowed_hdr_types_override,
                    allowed_sources_override: input.allowed_sources_override,
                    release_group_blacklist_override: input.release_group_blacklist_override,
                    release_group_whitelist_override: input.release_group_whitelist_override,
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if let Some(record) = result {
            Ok(TvShowResult {
                success: true,
                tv_show: Some(TvShow {
                    id: record.id.to_string(),
                    library_id: record.library_id.to_string(),
                    name: record.name,
                    sort_name: record.sort_name,
                    year: record.year,
                    status: match record.status.as_str() {
                        "continuing" | "Running" => TvShowStatus::Continuing,
                        "ended" | "Ended" => TvShowStatus::Ended,
                        _ => TvShowStatus::Unknown,
                    },
                    tvmaze_id: record.tvmaze_id,
                    tmdb_id: record.tmdb_id,
                    tvdb_id: record.tvdb_id,
                    imdb_id: record.imdb_id,
                    overview: record.overview,
                    network: record.network,
                    runtime: record.runtime,
                    genres: record.genres,
                    poster_url: record.poster_url,
                    backdrop_url: record.backdrop_url,
                    monitored: record.monitored,
                    monitor_type: match record.monitor_type.as_str() {
                        "all" => MonitorType::All,
                        "future" => MonitorType::Future,
                        _ => MonitorType::None,
                    },
                    path: record.path,
                    auto_download_override: record.auto_download_override,
                    backfill_existing: record.backfill_existing,
                    organize_files_override: record.organize_files_override,
                    rename_style_override: record.rename_style_override,
                    auto_hunt_override: record.auto_hunt_override,
                    episode_count: record.episode_count.unwrap_or(0),
                    episode_file_count: record.episode_file_count.unwrap_or(0),
                    size_bytes: record.size_bytes.unwrap_or(0),
                    // Quality override settings
                    allowed_resolutions_override: record.allowed_resolutions_override,
                    allowed_video_codecs_override: record.allowed_video_codecs_override,
                    allowed_audio_formats_override: record.allowed_audio_formats_override,
                    require_hdr_override: record.require_hdr_override,
                    allowed_hdr_types_override: record.allowed_hdr_types_override,
                    allowed_sources_override: record.allowed_sources_override,
                    release_group_blacklist_override: record.release_group_blacklist_override,
                    release_group_whitelist_override: record.release_group_whitelist_override,
                }),
                error: None,
            })
        } else {
            Ok(TvShowResult {
                success: false,
                tv_show: None,
                error: Some("Show not found".to_string()),
            })
        }
    }

    /// Delete a TV show
    async fn delete_tv_show(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let show_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid show ID: {}", e)))?;

        let deleted = db
            .tv_shows()
            .delete(show_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: deleted,
            error: if deleted {
                None
            } else {
                Some("Show not found".to_string())
            },
        })
    }

    /// Refresh metadata for a TV show
    async fn refresh_tv_show(&self, ctx: &Context<'_>, id: String) -> Result<TvShowResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let show_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid show ID: {}", e)))?;

        let show = db
            .tv_shows()
            .get_by_id(show_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Show not found"))?;

        // Get provider and ID
        let (provider, provider_id) = if let Some(tvmaze_id) = show.tvmaze_id {
            (crate::services::MetadataProvider::TvMaze, tvmaze_id as u32)
        } else if let Some(tmdb_id) = show.tmdb_id {
            (crate::services::MetadataProvider::Tmdb, tmdb_id as u32)
        } else if let Some(tvdb_id) = show.tvdb_id {
            (crate::services::MetadataProvider::TvDb, tvdb_id as u32)
        } else {
            return Ok(TvShowResult {
                success: false,
                tv_show: None,
                error: Some("No provider ID found for show".to_string()),
            });
        };

        // Fetch fresh show details (including updated artwork)
        let show_details = metadata
            .get_show(provider, provider_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Cache artwork if artwork service is available
        let (cached_poster_url, cached_backdrop_url) =
            if let Some(artwork_service) = metadata.artwork_service() {
                let entity_id = format!("{}_{}", provider_id, show.library_id);

                let poster_url = artwork_service
                    .cache_image_optional(
                        show_details.poster_url.as_deref(),
                        crate::services::artwork::ArtworkType::Poster,
                        "show",
                        &entity_id,
                    )
                    .await;

                let backdrop_url = artwork_service
                    .cache_image_optional(
                        show_details.backdrop_url.as_deref(),
                        crate::services::artwork::ArtworkType::Backdrop,
                        "show",
                        &entity_id,
                    )
                    .await;

                tracing::info!(
                    poster_cached = poster_url.is_some(),
                    backdrop_cached = backdrop_url.is_some(),
                    "Refreshed artwork caching"
                );

                (poster_url, backdrop_url)
            } else {
                (
                    show_details.poster_url.clone(),
                    show_details.backdrop_url.clone(),
                )
            };

        // Update show metadata including artwork
        let _ = db
            .tv_shows()
            .update(
                show_id,
                crate::db::UpdateTvShow {
                    name: Some(show_details.name),
                    overview: show_details.overview,
                    status: Some(show_details.status.unwrap_or_else(|| "unknown".to_string())),
                    network: show_details.network,
                    runtime: show_details.runtime,
                    genres: Some(show_details.genres),
                    poster_url: cached_poster_url,
                    backdrop_url: cached_backdrop_url,
                    ..Default::default()
                },
            )
            .await;

        // Fetch fresh episodes
        let episodes = metadata
            .get_episodes(provider, provider_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        for ep in episodes {
            let _ = db
                .episodes()
                .create(crate::db::CreateEpisode {
                    tv_show_id: show_id,
                    season: ep.season,
                    episode: ep.episode,
                    absolute_number: ep.absolute_number,
                    title: ep.title,
                    overview: ep.overview,
                    air_date: ep
                        .air_date
                        .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()),
                    runtime: ep.runtime,
                    tvmaze_id: if provider == crate::services::MetadataProvider::TvMaze {
                        Some(ep.provider_id as i32)
                    } else {
                        None
                    },
                    tmdb_id: None,
                    tvdb_id: None,
                })
                .await;
        }

        // Update show stats
        let _ = db.tv_shows().update_stats(show_id).await;

        // Get updated show
        let updated_show = db.tv_shows().get_by_id(show_id).await.ok().flatten();

        Ok(TvShowResult {
            success: true,
            tv_show: updated_show.map(|record| TvShow {
                id: record.id.to_string(),
                library_id: record.library_id.to_string(),
                name: record.name,
                sort_name: record.sort_name,
                year: record.year,
                status: match record.status.as_str() {
                    "continuing" | "Running" => TvShowStatus::Continuing,
                    "ended" | "Ended" => TvShowStatus::Ended,
                    _ => TvShowStatus::Unknown,
                },
                tvmaze_id: record.tvmaze_id,
                tmdb_id: record.tmdb_id,
                tvdb_id: record.tvdb_id,
                imdb_id: record.imdb_id,
                overview: record.overview,
                network: record.network,
                runtime: record.runtime,
                genres: record.genres,
                poster_url: record.poster_url,
                backdrop_url: record.backdrop_url,
                monitored: record.monitored,
                monitor_type: match record.monitor_type.as_str() {
                    "all" => MonitorType::All,
                    "future" => MonitorType::Future,
                    _ => MonitorType::None,
                },
                path: record.path,
                auto_download_override: record.auto_download_override,
                backfill_existing: record.backfill_existing,
                organize_files_override: record.organize_files_override,
                rename_style_override: record.rename_style_override,
                auto_hunt_override: record.auto_hunt_override,
                episode_count: record.episode_count.unwrap_or(0),
                episode_file_count: record.episode_file_count.unwrap_or(0),
                size_bytes: record.size_bytes.unwrap_or(0),
                // Quality override settings
                allowed_resolutions_override: record.allowed_resolutions_override,
                allowed_video_codecs_override: record.allowed_video_codecs_override,
                allowed_audio_formats_override: record.allowed_audio_formats_override,
                require_hdr_override: record.require_hdr_override,
                allowed_hdr_types_override: record.allowed_hdr_types_override,
                allowed_sources_override: record.allowed_sources_override,
                release_group_blacklist_override: record.release_group_blacklist_override,
                release_group_whitelist_override: record.release_group_whitelist_override,
            }),
            error: None,
        })
    }
}
