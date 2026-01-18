//! GraphQL schema definition with queries, mutations, and subscriptions
//!
//! This is the single API surface for the Librarian backend.
//! All operations require authentication unless explicitly noted.

use std::collections::HashMap;
use std::sync::Arc;

use async_graphql::{Context, Object, Result, Schema};
use uuid::Uuid;

use crate::db::{
    CreateLibrary, CreateQualityProfile, CreateRssFeed, Database, LibraryStats, LogFilter,
    UpdateLibrary, UpdateQualityProfile, UpdateRssFeed, UpdateTvShow,
};
use crate::services::{
    CastService, FilesystemService, LogEvent, MetadataService, ScannerService, TorrentService,
};

use super::auth::AuthExt;
use super::subscriptions::SubscriptionRoot;
use super::types::{
    AddTorrentInput,
    AddTorrentResult,
    AddTvShowInput,
    AppSetting,
    // Cast types
    AddCastDeviceInput,
    CastDevice,
    CastDeviceResult,
    CastMediaInput,
    CastSession,
    CastSessionResult,
    CastSettings,
    CastSettingsResult,
    UpdateCastDeviceInput,
    UpdateCastSettingsInput,
    ClearLogsResult,
    // Filesystem types
    BrowseDirectoryInput,
    BrowseDirectoryResult,
    CopyFilesInput,
    CreateDirectoryInput,
    DeleteFilesInput,
    DirectoryChangeEvent,
    FileEntry,
    FileOperationResult,
    MoveFilesInput,
    PathValidationResult,
    QuickPath,
    RenameFileInput,
    // Indexer types
    CreateIndexerInput,
    CreateLibraryInput,
    CreateQualityProfileInput,
    CreateRssFeedInput,
    CreateSubscriptionInput,
    DownloadEpisodeResult,
    Episode,
    EpisodeStatus,
    GenerateEncryptionKeyInput,
    IndexerCapabilities,
    IndexerConfig,
    IndexerResult,
    IndexerSearchInput,
    IndexerSearchResultItem,
    IndexerSearchResultSet,
    IndexerSettingDefinition,
    IndexerSettingOption,
    IndexerTestResult,
    IndexerTypeInfo,
    Library,
    LibraryFull,
    LibraryResult,
    LibraryType,
    LibraryUpcomingEpisode,
    LibraryUpcomingShow,
    LogEntry,
    LogFilterInput,
    LogLevel,
    LogStats,
    MediaFile,
    MediaItem,
    MonitorType,
    MutationResult,
    OrganizeTorrentResult,
    PaginatedLogResult,
    ParseAndIdentifyMediaResult,
    ParsedEpisodeInfo,
    PostDownloadAction,
    QualityProfile,
    QualityProfileResult,
    RssFeed,
    RssFeedResult,
    RssFeedTestResult,
    RssItem,
    ScanStatus,
    SearchResult,
    // Security types
    SecuritySettings,
    SecuritySettingsResult,
    SettingsResult,
    StreamInfo,
    Subscription,
    SubscriptionResult,
    Torrent,
    TorrentActionResult,
    TorrentDetails,
    TorrentRelease,
    TorrentSettings,
    TvShow,
    TvShowResult,
    TvShowSearchResult,
    TvShowStatus,
    UpcomingEpisode,
    UpcomingEpisodeShow,
    UpdateIndexerInput,
    UpdateLibraryInput,
    UpdatePreferencesInput,
    UpdateQualityProfileInput,
    UpdateRssFeedInput,
    UpdateSubscriptionInput,
    UpdateTorrentSettingsInput,
    UpdateTvShowInput,
    User,
    UserPreferences,
    // Helpers
    format_bytes,
};

/// The GraphQL schema type
pub type LibrarianSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

/// Build the GraphQL schema with all resolvers
pub fn build_schema(
    torrent_service: Arc<TorrentService>,
    metadata_service: Arc<MetadataService>,
    scanner_service: Arc<ScannerService>,
    cast_service: Arc<CastService>,
    filesystem_service: Arc<FilesystemService>,
    db: Database,
    log_broadcast: Option<tokio::sync::broadcast::Sender<LogEvent>>,
) -> LibrarianSchema {
    let mut schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(torrent_service)
        .data(metadata_service)
        .data(scanner_service)
        .data(cast_service)
        .data(filesystem_service)
        .data(db);

    // Add log broadcast sender if provided
    if let Some(sender) = log_broadcast {
        schema = schema.data(sender);
    }

    schema.finish()
}

// ============================================================================
// Query Root
// ============================================================================

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    // ------------------------------------------------------------------------
    // User / Auth
    // ------------------------------------------------------------------------

    /// Get the current authenticated user
    async fn me(&self, ctx: &Context<'_>) -> Result<User> {
        let user = ctx.auth_user()?;
        Ok(User {
            id: user.user_id.clone(),
            email: user.email.clone(),
            role: user.role.clone(),
        })
    }

    /// Get user preferences
    async fn my_preferences(&self, ctx: &Context<'_>) -> Result<UserPreferences> {
        let _user = ctx.auth_user()?;
        // TODO: Fetch from database
        Ok(UserPreferences {
            theme: "system".to_string(),
            default_quality_profile: None,
            notifications_enabled: true,
        })
    }

    // ------------------------------------------------------------------------
    // Libraries
    // ------------------------------------------------------------------------

    /// Get all libraries for the current user
    async fn libraries(&self, ctx: &Context<'_>) -> Result<Vec<LibraryFull>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .libraries()
            .list_by_user(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let mut libraries = Vec::new();
        for r in records {
            let stats = match db.libraries().get_stats(r.id).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(library_id = %r.id, error = %e, "Failed to get library stats");
                    LibraryStats::default()
                }
            };

            libraries.push(LibraryFull {
                id: r.id.to_string(),
                name: r.name,
                path: r.path,
                library_type: match r.library_type.as_str() {
                    "movies" => LibraryType::Movies,
                    "tv" => LibraryType::Tv,
                    "music" => LibraryType::Music,
                    "audiobooks" => LibraryType::Audiobooks,
                    _ => LibraryType::Other,
                },
                icon: r.icon.unwrap_or_else(|| "folder".to_string()),
                color: r.color.unwrap_or_else(|| "slate".to_string()),
                auto_scan: r.auto_scan,
                scan_interval_minutes: r.scan_interval_minutes,
                watch_for_changes: r.watch_for_changes,
                post_download_action: match r.post_download_action.as_str() {
                    "move" => PostDownloadAction::Move,
                    "hardlink" => PostDownloadAction::Hardlink,
                    _ => PostDownloadAction::Copy,
                },
                organize_files: r.organize_files,
                rename_style: r.rename_style.clone(),
                naming_pattern: r.naming_pattern,
                default_quality_profile_id: r.default_quality_profile_id.map(|id| id.to_string()),
                auto_add_discovered: r.auto_add_discovered,
                auto_download: r.auto_download,
                auto_hunt: r.auto_hunt,
                scanning: r.scanning,
                item_count: stats.file_count.unwrap_or(0) as i32,
                total_size_bytes: stats.total_size_bytes.unwrap_or(0),
                show_count: stats.show_count.unwrap_or(0) as i32,
                last_scanned_at: r.last_scanned_at.map(|t| t.to_rfc3339()),
                // Inline quality settings
                allowed_resolutions: r.allowed_resolutions,
                allowed_video_codecs: r.allowed_video_codecs,
                allowed_audio_formats: r.allowed_audio_formats,
                require_hdr: r.require_hdr,
                allowed_hdr_types: r.allowed_hdr_types,
                allowed_sources: r.allowed_sources,
                release_group_blacklist: r.release_group_blacklist,
                release_group_whitelist: r.release_group_whitelist,
            });
        }

        Ok(libraries)
    }

    /// Get a specific library by ID
    async fn library(&self, ctx: &Context<'_>, id: String) -> Result<Option<LibraryFull>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let record = db
            .libraries()
            .get_by_id_and_user(lib_id, user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if let Some(r) = record {
            let stats = db.libraries().get_stats(r.id).await.unwrap_or_default();

            Ok(Some(LibraryFull {
                id: r.id.to_string(),
                name: r.name,
                path: r.path,
                library_type: match r.library_type.as_str() {
                    "movies" => LibraryType::Movies,
                    "tv" => LibraryType::Tv,
                    "music" => LibraryType::Music,
                    "audiobooks" => LibraryType::Audiobooks,
                    _ => LibraryType::Other,
                },
                icon: r.icon.unwrap_or_else(|| "folder".to_string()),
                color: r.color.unwrap_or_else(|| "slate".to_string()),
                auto_scan: r.auto_scan,
                scan_interval_minutes: r.scan_interval_minutes,
                watch_for_changes: r.watch_for_changes,
                post_download_action: match r.post_download_action.as_str() {
                    "move" => PostDownloadAction::Move,
                    "hardlink" => PostDownloadAction::Hardlink,
                    _ => PostDownloadAction::Copy,
                },
                organize_files: r.organize_files,
                rename_style: r.rename_style.clone(),
                naming_pattern: r.naming_pattern,
                default_quality_profile_id: r.default_quality_profile_id.map(|id| id.to_string()),
                auto_add_discovered: r.auto_add_discovered,
                auto_download: r.auto_download,
                auto_hunt: r.auto_hunt,
                scanning: r.scanning,
                item_count: stats.file_count.unwrap_or(0) as i32,
                total_size_bytes: stats.total_size_bytes.unwrap_or(0),
                show_count: stats.show_count.unwrap_or(0) as i32,
                last_scanned_at: r.last_scanned_at.map(|t| t.to_rfc3339()),
                // Inline quality settings
                allowed_resolutions: r.allowed_resolutions,
                allowed_video_codecs: r.allowed_video_codecs,
                allowed_audio_formats: r.allowed_audio_formats,
                require_hdr: r.require_hdr,
                allowed_hdr_types: r.allowed_hdr_types,
                allowed_sources: r.allowed_sources,
                release_group_blacklist: r.release_group_blacklist,
                release_group_whitelist: r.release_group_whitelist,
            }))
        } else {
            Ok(None)
        }
    }

    // ------------------------------------------------------------------------
    // TV Shows
    // ------------------------------------------------------------------------

    /// Get all TV shows in a library
    async fn tv_shows(&self, ctx: &Context<'_>, library_id: String) -> Result<Vec<TvShow>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let records = db
            .tv_shows()
            .list_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| TvShow {
                id: r.id.to_string(),
                library_id: r.library_id.to_string(),
                name: r.name,
                sort_name: r.sort_name,
                year: r.year,
                status: match r.status.as_str() {
                    "continuing" | "Running" => TvShowStatus::Continuing,
                    "ended" | "Ended" => TvShowStatus::Ended,
                    "upcoming" => TvShowStatus::Upcoming,
                    "cancelled" => TvShowStatus::Cancelled,
                    _ => TvShowStatus::Unknown,
                },
                tvmaze_id: r.tvmaze_id,
                tmdb_id: r.tmdb_id,
                tvdb_id: r.tvdb_id,
                imdb_id: r.imdb_id,
                overview: r.overview,
                network: r.network,
                runtime: r.runtime,
                genres: r.genres,
                poster_url: r.poster_url,
                backdrop_url: r.backdrop_url,
                monitored: r.monitored,
                monitor_type: match r.monitor_type.as_str() {
                    "all" => MonitorType::All,
                    "future" => MonitorType::Future,
                    _ => MonitorType::None,
                },
                quality_profile_id: r.quality_profile_id.map(|id| id.to_string()),
                path: r.path,
                auto_download_override: r.auto_download_override,
                backfill_existing: r.backfill_existing,
                organize_files_override: r.organize_files_override,
                rename_style_override: r.rename_style_override,
                auto_hunt_override: r.auto_hunt_override,
                episode_count: r.episode_count.unwrap_or(0),
                episode_file_count: r.episode_file_count.unwrap_or(0),
                size_bytes: r.size_bytes.unwrap_or(0),
                // Quality override settings
                allowed_resolutions_override: r.allowed_resolutions_override,
                allowed_video_codecs_override: r.allowed_video_codecs_override,
                allowed_audio_formats_override: r.allowed_audio_formats_override,
                require_hdr_override: r.require_hdr_override,
                allowed_hdr_types_override: r.allowed_hdr_types_override,
                allowed_sources_override: r.allowed_sources_override,
                release_group_blacklist_override: r.release_group_blacklist_override,
                release_group_whitelist_override: r.release_group_whitelist_override,
            })
            .collect())
    }

    /// Get a specific TV show by ID
    async fn tv_show(&self, ctx: &Context<'_>, id: String) -> Result<Option<TvShow>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let show_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid show ID: {}", e)))?;

        let record = db
            .tv_shows()
            .get_by_id(show_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(record.map(|r| TvShow {
            id: r.id.to_string(),
            library_id: r.library_id.to_string(),
            name: r.name,
            sort_name: r.sort_name,
            year: r.year,
            status: match r.status.as_str() {
                "continuing" | "Running" => TvShowStatus::Continuing,
                "ended" | "Ended" => TvShowStatus::Ended,
                "upcoming" => TvShowStatus::Upcoming,
                "cancelled" => TvShowStatus::Cancelled,
                _ => TvShowStatus::Unknown,
            },
            tvmaze_id: r.tvmaze_id,
            tmdb_id: r.tmdb_id,
            tvdb_id: r.tvdb_id,
            imdb_id: r.imdb_id,
            overview: r.overview,
            network: r.network,
            runtime: r.runtime,
            genres: r.genres,
            poster_url: r.poster_url,
            backdrop_url: r.backdrop_url,
            monitored: r.monitored,
            monitor_type: match r.monitor_type.as_str() {
                "all" => MonitorType::All,
                "future" => MonitorType::Future,
                _ => MonitorType::None,
            },
            quality_profile_id: r.quality_profile_id.map(|id| id.to_string()),
            path: r.path,
            auto_download_override: r.auto_download_override,
            backfill_existing: r.backfill_existing,
            organize_files_override: r.organize_files_override,
            rename_style_override: r.rename_style_override,
            auto_hunt_override: r.auto_hunt_override,
            episode_count: r.episode_count.unwrap_or(0),
            episode_file_count: r.episode_file_count.unwrap_or(0),
            size_bytes: r.size_bytes.unwrap_or(0),
            // Quality override settings
            allowed_resolutions_override: r.allowed_resolutions_override,
            allowed_video_codecs_override: r.allowed_video_codecs_override,
            allowed_audio_formats_override: r.allowed_audio_formats_override,
            require_hdr_override: r.require_hdr_override,
            allowed_hdr_types_override: r.allowed_hdr_types_override,
            allowed_sources_override: r.allowed_sources_override,
            release_group_blacklist_override: r.release_group_blacklist_override,
            release_group_whitelist_override: r.release_group_whitelist_override,
        }))
    }

    /// Search for TV shows from metadata providers
    async fn search_tv_shows(
        &self,
        ctx: &Context<'_>,
        query: String,
    ) -> Result<Vec<TvShowSearchResult>> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let results = metadata
            .search_shows(&query)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|r| TvShowSearchResult {
                provider: format!("{:?}", r.provider).to_lowercase(),
                provider_id: r.provider_id as i32,
                name: r.name,
                year: r.year,
                status: r.status,
                network: r.network,
                overview: r.overview,
                poster_url: r.poster_url,
                tvdb_id: r.tvdb_id.map(|id| id as i32),
                imdb_id: r.imdb_id,
                score: r.score,
            })
            .collect())
    }

    // ------------------------------------------------------------------------
    // Media Files
    // ------------------------------------------------------------------------

    /// Get unmatched files for a library (files not linked to any episode)
    async fn unmatched_files(
        &self,
        ctx: &Context<'_>,
        library_id: String,
    ) -> Result<Vec<MediaFile>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let records = db
            .media_files()
            .list_unmatched_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(MediaFile::from_record).collect())
    }

    /// Get count of unmatched files for a library
    async fn unmatched_files_count(&self, ctx: &Context<'_>, library_id: String) -> Result<i32> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let count = db
            .media_files()
            .count_unmatched_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(count as i32)
    }

    // ------------------------------------------------------------------------
    // Episodes
    // ------------------------------------------------------------------------

    /// Get all episodes for a TV show
    async fn episodes(&self, ctx: &Context<'_>, tv_show_id: String) -> Result<Vec<Episode>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let show_id = Uuid::parse_str(&tv_show_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid show ID: {}", e)))?;

        let records = db
            .episodes()
            .list_by_show(show_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // For downloaded episodes, look up the media file ID
        let mut episodes = Vec::with_capacity(records.len());
        for r in records {
            let media_file_id = if r.status == "downloaded" {
                // Try to get the media file for this episode
                db.media_files()
                    .get_by_episode_id(r.id)
                    .await
                    .ok()
                    .flatten()
                    .map(|f| f.id.to_string())
            } else {
                None
            };

            episodes.push(Episode {
                id: r.id.to_string(),
                tv_show_id: r.tv_show_id.to_string(),
                season: r.season,
                episode: r.episode,
                absolute_number: r.absolute_number,
                title: r.title,
                overview: r.overview,
                air_date: r.air_date.map(|d| d.to_string()),
                runtime: r.runtime,
                status: match r.status.as_str() {
                    "missing" => EpisodeStatus::Missing,
                    "wanted" => EpisodeStatus::Wanted,
                    "available" => EpisodeStatus::Available,
                    "downloading" => EpisodeStatus::Downloading,
                    "downloaded" => EpisodeStatus::Downloaded,
                    "ignored" => EpisodeStatus::Ignored,
                    _ => EpisodeStatus::Missing,
                },
                tvmaze_id: r.tvmaze_id,
                tmdb_id: r.tmdb_id,
                tvdb_id: r.tvdb_id,
                torrent_link: r.torrent_link,
                torrent_link_added_at: r.torrent_link_added_at.map(|t| t.to_rfc3339()),
                media_file_id,
            });
        }

        Ok(episodes)
    }

    /// Get wanted (missing) episodes
    async fn wanted_episodes(
        &self,
        ctx: &Context<'_>,
        library_id: Option<String>,
    ) -> Result<Vec<Episode>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let records = if let Some(lib_id) = library_id {
            let lib_uuid = Uuid::parse_str(&lib_id)
                .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
            db.episodes().list_wanted_by_library(lib_uuid).await
        } else {
            let user_id = Uuid::parse_str(&user.user_id)
                .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;
            db.episodes().list_wanted_by_user(user_id).await
        }
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| Episode {
                id: r.id.to_string(),
                tv_show_id: r.tv_show_id.to_string(),
                season: r.season,
                episode: r.episode,
                absolute_number: r.absolute_number,
                title: r.title,
                overview: r.overview,
                air_date: r.air_date.map(|d| d.to_string()),
                runtime: r.runtime,
                status: match r.status.as_str() {
                    "missing" => EpisodeStatus::Missing,
                    "wanted" => EpisodeStatus::Wanted,
                    "available" => EpisodeStatus::Available,
                    "downloading" => EpisodeStatus::Downloading,
                    "downloaded" => EpisodeStatus::Downloaded,
                    "ignored" => EpisodeStatus::Ignored,
                    _ => EpisodeStatus::Wanted,
                },
                tvmaze_id: r.tvmaze_id,
                tmdb_id: r.tmdb_id,
                tvdb_id: r.tvdb_id,
                torrent_link: r.torrent_link,
                torrent_link_added_at: r.torrent_link_added_at.map(|t| t.to_rfc3339()),
                media_file_id: None, // Wanted episodes don't have media files
            })
            .collect())
    }

    // ------------------------------------------------------------------------
    // Quality Profiles
    // ------------------------------------------------------------------------

    /// Get all quality profiles for the current user
    async fn quality_profiles(&self, ctx: &Context<'_>) -> Result<Vec<QualityProfile>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        // Create default profiles if none exist
        let profiles = db.quality_profiles();
        if !profiles.has_profiles(user_id).await.unwrap_or(true) {
            let _ = profiles.create_defaults(user_id).await;
        }

        let records = profiles
            .list_by_user(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| QualityProfile {
                id: r.id.to_string(),
                name: r.name,
                preferred_resolution: r.preferred_resolution,
                min_resolution: r.min_resolution,
                preferred_codec: r.preferred_codec,
                preferred_audio: r.preferred_audio,
                require_hdr: r.require_hdr,
                hdr_types: r.hdr_types,
                preferred_language: r.preferred_language,
                max_size_gb: r.max_size_gb.map(|d| d.to_string().parse().unwrap_or(0.0)),
                min_seeders: r.min_seeders,
                release_group_whitelist: r.release_group_whitelist,
                release_group_blacklist: r.release_group_blacklist,
                upgrade_until: r.upgrade_until,
            })
            .collect())
    }

    /// Get a specific quality profile by ID
    async fn quality_profile(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Option<QualityProfile>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let profile_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid profile ID: {}", e)))?;

        let record = db
            .quality_profiles()
            .get_by_id(profile_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(record.map(|r| QualityProfile {
            id: r.id.to_string(),
            name: r.name,
            preferred_resolution: r.preferred_resolution,
            min_resolution: r.min_resolution,
            preferred_codec: r.preferred_codec,
            preferred_audio: r.preferred_audio,
            require_hdr: r.require_hdr,
            hdr_types: r.hdr_types,
            preferred_language: r.preferred_language,
            max_size_gb: r.max_size_gb.map(|d| d.to_string().parse().unwrap_or(0.0)),
            min_seeders: r.min_seeders,
            release_group_whitelist: r.release_group_whitelist,
            release_group_blacklist: r.release_group_blacklist,
            upgrade_until: r.upgrade_until,
        }))
    }

    // ------------------------------------------------------------------------
    // RSS Feeds
    // ------------------------------------------------------------------------

    /// Get all RSS feeds for the current user
    async fn rss_feeds(
        &self,
        ctx: &Context<'_>,
        library_id: Option<String>,
    ) -> Result<Vec<RssFeed>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let records = if let Some(lib_id) = library_id {
            let lib_uuid = Uuid::parse_str(&lib_id)
                .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
            db.rss_feeds().list_by_library(lib_uuid).await
        } else {
            let user_id = Uuid::parse_str(&user.user_id)
                .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;
            db.rss_feeds().list_by_user(user_id).await
        }
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| RssFeed {
                id: r.id.to_string(),
                library_id: r.library_id.map(|id| id.to_string()),
                name: r.name,
                url: r.url,
                enabled: r.enabled,
                poll_interval_minutes: r.poll_interval_minutes,
                last_polled_at: r.last_polled_at.map(|t| t.to_rfc3339()),
                last_successful_at: r.last_successful_at.map(|t| t.to_rfc3339()),
                last_error: r.last_error,
                consecutive_failures: r.consecutive_failures.unwrap_or(0),
            })
            .collect())
    }

    // ------------------------------------------------------------------------
    // Parse and Identify
    // ------------------------------------------------------------------------

    /// Parse a filename and identify the media
    async fn parse_and_identify_media(
        &self,
        ctx: &Context<'_>,
        title: String,
    ) -> Result<ParseAndIdentifyMediaResult> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let result = metadata
            .parse_and_identify(&title)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(ParseAndIdentifyMediaResult {
            parsed: ParsedEpisodeInfo {
                original_title: result.parsed.original_title,
                show_name: result.parsed.show_name,
                season: result.parsed.season.map(|s| s as i32),
                episode: result.parsed.episode.map(|e| e as i32),
                year: result.parsed.year.map(|y| y as i32),
                date: result.parsed.date,
                resolution: result.parsed.resolution,
                source: result.parsed.source,
                codec: result.parsed.codec,
                hdr: result.parsed.hdr,
                audio: result.parsed.audio,
                release_group: result.parsed.release_group,
                is_proper: result.parsed.is_proper,
                is_repack: result.parsed.is_repack,
            },
            matches: result
                .matches
                .into_iter()
                .map(|r| TvShowSearchResult {
                    provider: format!("{:?}", r.provider).to_lowercase(),
                    provider_id: r.provider_id as i32,
                    name: r.name,
                    year: r.year,
                    status: r.status,
                    network: r.network,
                    overview: r.overview,
                    poster_url: r.poster_url,
                    tvdb_id: r.tvdb_id.map(|id| id as i32),
                    imdb_id: r.imdb_id,
                    score: r.score,
                })
                .collect(),
        })
    }

    // ------------------------------------------------------------------------
    // Media
    // ------------------------------------------------------------------------

    /// Get media items, optionally filtered by library
    async fn media_items(
        &self,
        ctx: &Context<'_>,
        library_id: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<MediaItem>> {
        let _user = ctx.auth_user()?;
        let _library_id = library_id;
        let _limit = limit.unwrap_or(50);
        let _offset = offset.unwrap_or(0);
        // TODO: Query from database
        Ok(vec![])
    }

    /// Get a specific media item by ID
    async fn media_item(&self, ctx: &Context<'_>, id: String) -> Result<Option<MediaItem>> {
        let _user = ctx.auth_user()?;
        let _id = id;
        // TODO: Query from database
        Ok(None)
    }

    /// Search media items by title
    async fn search_media(
        &self,
        ctx: &Context<'_>,
        query: String,
        limit: Option<i32>,
    ) -> Result<Vec<MediaItem>> {
        let _user = ctx.auth_user()?;
        let _query = query;
        let _limit = limit.unwrap_or(20);
        // TODO: Implement search
        Ok(vec![])
    }

    /// Get stream information for a media item
    async fn stream_info(&self, ctx: &Context<'_>, media_id: String) -> Result<Option<StreamInfo>> {
        let _user = ctx.auth_user()?;
        let _id = media_id;
        // TODO: Generate stream URLs
        Ok(Some(StreamInfo {
            playlist_url: String::new(),
            direct_play_supported: true,
            direct_url: None,
            subtitles: vec![],
            audio_tracks: vec![],
        }))
    }

    // ------------------------------------------------------------------------
    // Cast Devices & Sessions
    // ------------------------------------------------------------------------

    /// Get all discovered and saved cast devices
    async fn cast_devices(&self, ctx: &Context<'_>) -> Result<Vec<CastDevice>> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let devices = cast_service
            .get_devices()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // TODO: Track connected state per device
        Ok(devices
            .into_iter()
            .map(|d| CastDevice::from_record(d, false))
            .collect())
    }

    /// Get a specific cast device by ID
    async fn cast_device(&self, ctx: &Context<'_>, id: String) -> Result<Option<CastDevice>> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let device_id = Uuid::parse_str(&id)
            .map_err(|_| async_graphql::Error::new("Invalid device ID"))?;

        let device = cast_service
            .get_device(device_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(device.map(|d| CastDevice::from_record(d, false)))
    }

    /// Get all active cast sessions
    async fn cast_sessions(&self, ctx: &Context<'_>) -> Result<Vec<CastSession>> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let sessions = cast_service
            .get_active_sessions()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Get device names for each session
        let mut results = Vec::new();
        for session in sessions {
            let device_name = if let Some(device_id) = session.device_id {
                cast_service
                    .get_device(device_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|d| d.name)
            } else {
                None
            };
            results.push(CastSession::from_record(session, device_name));
        }

        Ok(results)
    }

    /// Get a specific cast session by ID
    async fn cast_session(&self, ctx: &Context<'_>, id: String) -> Result<Option<CastSession>> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let session_id = Uuid::parse_str(&id)
            .map_err(|_| async_graphql::Error::new("Invalid session ID"))?;

        let session = cast_service
            .get_session(session_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if let Some(session) = session {
            let device_name = if let Some(device_id) = session.device_id {
                cast_service
                    .get_device(device_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|d| d.name)
            } else {
                None
            };
            Ok(Some(CastSession::from_record(session, device_name)))
        } else {
            Ok(None)
        }
    }

    /// Get cast settings
    async fn cast_settings(&self, ctx: &Context<'_>) -> Result<Option<CastSettings>> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let settings = cast_service
            .get_settings()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(settings.map(CastSettings::from_record))
    }

    // ------------------------------------------------------------------------
    // Torrents
    // ------------------------------------------------------------------------

    /// Get all torrents
    async fn torrents(&self, ctx: &Context<'_>) -> Result<Vec<Torrent>> {
        let user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();
        let db = ctx.data_unchecked::<Database>();

        // Get live torrents from the service
        let torrents = service.list_torrents().await;
        let mut result: Vec<Torrent> = torrents.into_iter().map(|t| t.into()).collect();

        // Try to get added_at timestamps from database
        let user_uuid = Uuid::parse_str(&user.user_id).unwrap_or_default();
        if let Ok(records) = db.torrents().list_by_user(user_uuid).await {
            // Create a map of info_hash -> added_at
            let added_at_map: HashMap<String, String> = records
                .into_iter()
                .filter_map(|r| {
                    r.added_at
                        .format(&time::format_description::well_known::Rfc3339)
                        .ok()
                        .map(|ts| (r.info_hash, ts))
                })
                .collect();

            // Merge added_at into the result
            for torrent in &mut result {
                if let Some(added_at) = added_at_map.get(&torrent.info_hash) {
                    torrent.added_at = Some(added_at.clone());
                }
            }
        }

        Ok(result)
    }

    /// Get a specific torrent by ID
    async fn torrent(&self, ctx: &Context<'_>, id: i32) -> Result<Option<Torrent>> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();
        match service.get_torrent_info(id as usize).await {
            Ok(info) => Ok(Some(info.into())),
            Err(_) => Ok(None),
        }
    }

    /// Get detailed information about a torrent (for info modal)
    async fn torrent_details(&self, ctx: &Context<'_>, id: i32) -> Result<Option<TorrentDetails>> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();
        match service.get_torrent_details(id as usize).await {
            Ok(details) => Ok(Some(details.into())),
            Err(_) => Ok(None),
        }
    }

    // ------------------------------------------------------------------------
    // Subscriptions (legacy - use tv_shows instead)
    // ------------------------------------------------------------------------

    /// Get all subscriptions (legacy)
    async fn subscriptions(&self, ctx: &Context<'_>) -> Result<Vec<Subscription>> {
        let _user = ctx.auth_user()?;
        // TODO: Query from database
        Ok(vec![])
    }

    /// Get a specific subscription by ID (legacy)
    async fn subscription(&self, ctx: &Context<'_>, id: String) -> Result<Option<Subscription>> {
        let _user = ctx.auth_user()?;
        let _id = id;
        // TODO: Query from database
        Ok(None)
    }

    /// Search for torrents via Torznab indexers
    async fn search_torrents(
        &self,
        ctx: &Context<'_>,
        query: String,
        _category: Option<String>,
    ) -> Result<Vec<SearchResult>> {
        let _user = ctx.auth_user()?;
        let _query = query;
        // TODO: Implement Torznab search via Prowlarr
        Ok(vec![])
    }

    // ------------------------------------------------------------------------
    // System & Settings
    // ------------------------------------------------------------------------

    /// Health check (no auth required)
    async fn health(&self) -> Result<bool> {
        Ok(true)
    }

    /// Server version
    async fn version(&self) -> Result<String> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }

    /// Get torrent client settings
    async fn torrent_settings(&self, ctx: &Context<'_>) -> Result<TorrentSettings> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        Ok(TorrentSettings {
            download_dir: settings
                .get_or_default("torrent.download_dir", "/data/downloads".to_string())
                .await
                .unwrap_or_default(),
            session_dir: settings
                .get_or_default("torrent.session_dir", "/data/session".to_string())
                .await
                .unwrap_or_default(),
            enable_dht: settings
                .get_or_default("torrent.enable_dht", true)
                .await
                .unwrap_or(true),
            listen_port: settings
                .get_or_default("torrent.listen_port", 6881)
                .await
                .unwrap_or(6881),
            max_concurrent: settings
                .get_or_default("torrent.max_concurrent", 5)
                .await
                .unwrap_or(5),
            upload_limit: settings
                .get_or_default("torrent.upload_limit", 0i64)
                .await
                .unwrap_or(0),
            download_limit: settings
                .get_or_default("torrent.download_limit", 0i64)
                .await
                .unwrap_or(0),
        })
    }

    /// Get all settings in a category
    async fn settings_by_category(
        &self,
        ctx: &Context<'_>,
        category: String,
    ) -> Result<Vec<AppSetting>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        let records = settings
            .list_by_category(&category)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| AppSetting {
                key: r.key,
                value: r.value,
                description: r.description,
                category: r.category,
            })
            .collect())
    }

    // ------------------------------------------------------------------------
    // Logs
    // ------------------------------------------------------------------------

    /// Get logs with optional filtering and pagination
    async fn logs(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Filter options")] filter: Option<LogFilterInput>,
        #[graphql(default = 50, desc = "Number of logs to return")] limit: i32,
        #[graphql(default = 0, desc = "Offset for pagination")] offset: i32,
    ) -> Result<PaginatedLogResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let log_filter = filter
            .map(|f| {
                let levels = f.levels.map(|ls| {
                    ls.into_iter()
                        .map(|l| match l {
                            LogLevel::Trace => "TRACE".to_string(),
                            LogLevel::Debug => "DEBUG".to_string(),
                            LogLevel::Info => "INFO".to_string(),
                            LogLevel::Warn => "WARN".to_string(),
                            LogLevel::Error => "ERROR".to_string(),
                        })
                        .collect()
                });

                let from_timestamp = f.from_timestamp.and_then(|s| {
                    time::OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339)
                        .ok()
                });

                let to_timestamp = f.to_timestamp.and_then(|s| {
                    time::OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339)
                        .ok()
                });

                LogFilter {
                    levels,
                    target: f.target,
                    keyword: f.keyword,
                    from_timestamp,
                    to_timestamp,
                }
            })
            .unwrap_or_default();

        let result = db
            .logs()
            .list(log_filter, limit as i64, offset as i64)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let next_cursor = result.logs.last().map(|l| {
            l.timestamp
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default()
        });

        Ok(PaginatedLogResult {
            logs: result
                .logs
                .into_iter()
                .map(|r| LogEntry {
                    id: r.id.to_string(),
                    timestamp: r
                        .timestamp
                        .format(&time::format_description::well_known::Rfc3339)
                        .unwrap_or_default(),
                    level: LogLevel::from(r.level.as_str()),
                    target: r.target,
                    message: r.message,
                    fields: r.fields,
                    span_name: r.span_name,
                })
                .collect(),
            total_count: result.total_count,
            has_more: result.has_more,
            next_cursor,
        })
    }

    /// Get distinct log targets for filtering
    async fn log_targets(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50, desc = "Maximum number of targets to return")] limit: i32,
    ) -> Result<Vec<String>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let targets = db
            .logs()
            .get_distinct_targets(limit as i64)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(targets)
    }

    /// Get log statistics by level
    async fn log_stats(&self, ctx: &Context<'_>) -> Result<LogStats> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let counts = db
            .logs()
            .get_counts_by_level()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let mut stats = LogStats {
            trace_count: 0,
            debug_count: 0,
            info_count: 0,
            warn_count: 0,
            error_count: 0,
            total_count: 0,
        };

        for (level, count) in counts {
            match level.to_uppercase().as_str() {
                "TRACE" => stats.trace_count = count,
                "DEBUG" => stats.debug_count = count,
                "INFO" => stats.info_count = count,
                "WARN" => stats.warn_count = count,
                "ERROR" => stats.error_count = count,
                _ => {}
            }
            stats.total_count += count;
        }

        Ok(stats)
    }

    // ------------------------------------------------------------------------
    // Upcoming Episodes
    // ------------------------------------------------------------------------

    /// Get upcoming TV episodes from TVMaze for the next N days
    ///
    /// This fetches the global TV schedule from TVMaze, showing what's airing
    /// on broadcast/cable networks. Use country filter to narrow results.
    async fn upcoming_episodes(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 7, desc = "Number of days to look ahead")] days: i32,
        #[graphql(desc = "Country code filter (e.g., 'US', 'GB')")] country: Option<String>,
    ) -> Result<Vec<UpcomingEpisode>> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let schedule = metadata
            .get_upcoming_schedule(days as u32, country.as_deref())
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(schedule
            .into_iter()
            .map(|entry| UpcomingEpisode {
                tvmaze_id: entry.id as i32,
                name: entry.name,
                season: entry.season as i32,
                episode: entry.number as i32,
                air_date: entry.airdate.unwrap_or_default(),
                air_time: entry.airtime,
                air_stamp: entry.air_stamp,
                runtime: entry.runtime.map(|r| r as i32),
                summary: entry.summary.map(|s| {
                    // Strip HTML tags from summary
                    let re = regex::Regex::new(r"<[^>]+>").unwrap();
                    re.replace_all(&s, "").trim().to_string()
                }),
                episode_image_url: entry.image.as_ref().and_then(|i| i.medium.clone()),
                show: UpcomingEpisodeShow {
                    tvmaze_id: entry.show.id as i32,
                    name: entry.show.name,
                    network: entry
                        .show
                        .network
                        .as_ref()
                        .map(|n| n.name.clone())
                        .or_else(|| entry.show.web_channel.as_ref().map(|w| w.name.clone())),
                    poster_url: entry.show.image.as_ref().and_then(|i| i.medium.clone()),
                    genres: entry.show.genres,
                },
            })
            .collect())
    }

    /// Get upcoming episodes from the user's libraries
    ///
    /// Returns episodes from shows in the user's TV libraries that are
    /// airing in the next N days. Only includes monitored shows.
    async fn library_upcoming_episodes(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 7, desc = "Number of days to look ahead")] days: i32,
    ) -> Result<Vec<LibraryUpcomingEpisode>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .episodes()
            .list_upcoming_by_user(user_id, days)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| LibraryUpcomingEpisode {
                id: r.id.to_string(),
                tvmaze_id: r.episode_tvmaze_id,
                name: r.episode_title,
                season: r.season,
                episode: r.episode,
                air_date: r.air_date.map(|d| d.to_string()).unwrap_or_default(),
                status: match r.status.as_str() {
                    "missing" => EpisodeStatus::Missing,
                    "wanted" => EpisodeStatus::Wanted,
                    "available" => EpisodeStatus::Available,
                    "downloading" => EpisodeStatus::Downloading,
                    "downloaded" => EpisodeStatus::Downloaded,
                    "ignored" => EpisodeStatus::Ignored,
                    _ => EpisodeStatus::Missing,
                },
                show: LibraryUpcomingShow {
                    id: r.show_id.to_string(),
                    name: r.show_name,
                    year: r.show_year,
                    network: r.show_network,
                    poster_url: r.show_poster_url,
                    library_id: r.library_id.to_string(),
                },
            })
            .collect())
    }

    // ------------------------------------------------------------------------
    // Indexers
    // ------------------------------------------------------------------------

    /// Get all configured indexers for the current user
    async fn indexers(&self, ctx: &Context<'_>) -> Result<Vec<IndexerConfig>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .indexers()
            .list_by_user(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| IndexerConfig {
                id: r.id.to_string(),
                indexer_type: r.indexer_type,
                name: r.name,
                enabled: r.enabled,
                priority: r.priority,
                site_url: r.site_url,
                is_healthy: r.error_count == 0,
                last_error: r.last_error,
                error_count: r.error_count,
                last_success_at: r.last_success_at.map(|dt| dt.to_rfc3339()),
                created_at: r.created_at.to_rfc3339(),
                updated_at: r.updated_at.to_rfc3339(),
                capabilities: IndexerCapabilities {
                    supports_search: r.supports_search.unwrap_or(true),
                    supports_tv_search: r.supports_tv_search.unwrap_or(true),
                    supports_movie_search: r.supports_movie_search.unwrap_or(true),
                    supports_music_search: r.supports_music_search.unwrap_or(false),
                    supports_book_search: r.supports_book_search.unwrap_or(false),
                    supports_imdb_search: r.supports_imdb_search.unwrap_or(false),
                    supports_tvdb_search: r.supports_tvdb_search.unwrap_or(false),
                },
            })
            .collect())
    }

    /// Get a specific indexer by ID
    async fn indexer(&self, ctx: &Context<'_>, id: String) -> Result<Option<IndexerConfig>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let config_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid indexer ID: {}", e)))?;

        let record = db
            .indexers()
            .get(config_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Verify ownership
        if let Some(ref r) = record {
            let user_id = Uuid::parse_str(&user.user_id)?;
            if r.user_id != user_id {
                return Ok(None);
            }
        }

        Ok(record.map(|r| IndexerConfig {
            id: r.id.to_string(),
            indexer_type: r.indexer_type,
            name: r.name,
            enabled: r.enabled,
            priority: r.priority,
            site_url: r.site_url,
            is_healthy: r.error_count == 0,
            last_error: r.last_error,
            error_count: r.error_count,
            last_success_at: r.last_success_at.map(|dt| dt.to_rfc3339()),
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
            capabilities: IndexerCapabilities {
                supports_search: r.supports_search.unwrap_or(true),
                supports_tv_search: r.supports_tv_search.unwrap_or(true),
                supports_movie_search: r.supports_movie_search.unwrap_or(true),
                supports_music_search: r.supports_music_search.unwrap_or(false),
                supports_book_search: r.supports_book_search.unwrap_or(false),
                supports_imdb_search: r.supports_imdb_search.unwrap_or(false),
                supports_tvdb_search: r.supports_tvdb_search.unwrap_or(false),
            },
        }))
    }

    /// Get available indexer types (for creating new indexers)
    async fn available_indexer_types(&self, ctx: &Context<'_>) -> Result<Vec<IndexerTypeInfo>> {
        let _user = ctx.auth_user()?;

        use crate::indexer::definitions::get_available_indexers;

        let types = get_available_indexers()
            .iter()
            .map(|info| IndexerTypeInfo {
                id: info.id.to_string(),
                name: info.name.to_string(),
                description: info.description.to_string(),
                tracker_type: info.tracker_type.to_string(),
                language: info.language.to_string(),
                site_link: info.site_link.to_string(),
                required_credentials: info
                    .required_credentials
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                is_native: info.is_native,
            })
            .collect();

        Ok(types)
    }

    /// Get setting definitions for an indexer type
    async fn indexer_setting_definitions(
        &self,
        ctx: &Context<'_>,
        indexer_type: String,
    ) -> Result<Vec<IndexerSettingDefinition>> {
        let _user = ctx.auth_user()?;

        use crate::indexer::definitions::{SettingType, get_indexer_info};

        let info = get_indexer_info(&indexer_type).ok_or_else(|| {
            async_graphql::Error::new(format!("Unknown indexer type: {}", indexer_type))
        })?;

        let settings = info
            .optional_settings
            .iter()
            .map(|s| IndexerSettingDefinition {
                key: s.key.to_string(),
                label: s.label.to_string(),
                setting_type: match s.setting_type {
                    SettingType::Text => "text".to_string(),
                    SettingType::Password => "password".to_string(),
                    SettingType::Checkbox => "checkbox".to_string(),
                    SettingType::Select => "select".to_string(),
                },
                default_value: s.default_value.map(|s| s.to_string()),
                options: s.options.map(|opts| {
                    opts.iter()
                        .map(|(value, label)| IndexerSettingOption {
                            value: value.to_string(),
                            label: label.to_string(),
                        })
                        .collect()
                }),
            })
            .collect();

        Ok(settings)
    }

    // ========================================================================
    // Security Settings Queries
    // ========================================================================

    /// Get security settings
    async fn security_settings(&self, ctx: &Context<'_>) -> Result<SecuritySettings> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let key = db
            .settings()
            .get_indexer_encryption_key()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let (encryption_key_set, encryption_key_preview) = match &key {
            Some(k) if k.len() >= 8 => {
                // Show first 4 and last 4 characters
                let preview = format!("{}...{}", &k[..4], &k[k.len() - 4..]);
                (true, Some(preview))
            }
            Some(_) => (true, Some("****".to_string())),
            None => (false, None),
        };

        Ok(SecuritySettings {
            encryption_key_set,
            encryption_key_preview,
            encryption_key_last_modified: None, // TODO: Track this if needed
        })
    }

    // ========================================================================
    // Filesystem Queries
    // ========================================================================

    /// Browse a directory on the server filesystem
    async fn browse_directory(
        &self,
        ctx: &Context<'_>,
        input: Option<BrowseDirectoryInput>,
    ) -> Result<BrowseDirectoryResult> {
        let user = ctx.auth_user()?;
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let input = input.unwrap_or_else(|| BrowseDirectoryInput {
            path: None,
            dirs_only: Some(true),
            show_hidden: Some(false),
        });

        let result = fs_service
            .browse(
                input.path.as_deref(),
                input.dirs_only.unwrap_or(true),
                input.show_hidden.unwrap_or(false),
                user_id,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(BrowseDirectoryResult {
            current_path: result.current_path,
            parent_path: result.parent_path,
            entries: result
                .entries
                .into_iter()
                .map(|e| FileEntry {
                    name: e.name,
                    path: e.path,
                    is_dir: e.is_dir,
                    size: e.size as i64,
                    size_formatted: format_bytes(e.size),
                    readable: e.readable,
                    writable: e.writable,
                    mime_type: e.mime_type,
                    modified_at: e.modified_at.map(|t| t.to_rfc3339()),
                })
                .collect(),
            quick_paths: result
                .quick_paths
                .into_iter()
                .map(|p| QuickPath {
                    name: p.name,
                    path: p.path,
                })
                .collect(),
            is_library_path: result.is_library_path,
            library_id: result.library_id.map(|id| id.to_string()),
        })
    }

    /// Get quick-access filesystem paths
    async fn quick_paths(&self, _ctx: &Context<'_>) -> Result<Vec<QuickPath>> {
        // Quick paths don't require auth - they're just common paths on the system
        let paths = crate::services::FilesystemService::get_quick_paths();
        Ok(paths
            .into_iter()
            .map(|p| QuickPath {
                name: p.name,
                path: p.path,
            })
            .collect())
    }

    /// Validate if a path is inside a library
    async fn validate_path(
        &self,
        ctx: &Context<'_>,
        path: String,
    ) -> Result<PathValidationResult> {
        let user = ctx.auth_user()?;
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let validation = fs_service
            .validate_path(&path, user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(PathValidationResult {
            is_valid: validation.is_valid,
            is_library_path: validation.is_library_path,
            library_id: validation.library_id.map(|id| id.to_string()),
            library_name: validation.library_name,
            error: validation.error,
        })
    }
}

// ============================================================================
// Mutation Root
// ============================================================================

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    // ------------------------------------------------------------------------
    // User Preferences
    // ------------------------------------------------------------------------

    /// Update user preferences
    async fn update_preferences(
        &self,
        ctx: &Context<'_>,
        input: UpdatePreferencesInput,
    ) -> Result<UserPreferences> {
        let _user = ctx.auth_user()?;
        // TODO: Update in database
        Ok(UserPreferences {
            theme: input.theme.unwrap_or_else(|| "system".to_string()),
            default_quality_profile: input.default_quality_profile,
            notifications_enabled: input.notifications_enabled.unwrap_or(true),
        })
    }

    // ------------------------------------------------------------------------
    // Libraries
    // ------------------------------------------------------------------------

    /// Create a new library
    async fn create_library(
        &self,
        ctx: &Context<'_>,
        input: CreateLibraryInput,
    ) -> Result<LibraryResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let library_type = match input.library_type {
            LibraryType::Movies => "movies",
            LibraryType::Tv => "tv",
            LibraryType::Music => "music",
            LibraryType::Audiobooks => "audiobooks",
            LibraryType::Other => "other",
        }
        .to_string();

        let post_download_action = match input.post_download_action {
            Some(PostDownloadAction::Move) => "move",
            Some(PostDownloadAction::Hardlink) => "hardlink",
            _ => "copy",
        }
        .to_string();

        let record = db
            .libraries()
            .create(CreateLibrary {
                user_id,
                name: input.name,
                path: input.path,
                library_type,
                icon: input.icon,
                color: input.color,
                auto_scan: input.auto_scan.unwrap_or(true),
                scan_interval_minutes: input.scan_interval_minutes.unwrap_or(60),
                watch_for_changes: input.watch_for_changes.unwrap_or(false),
                post_download_action,
                organize_files: input.organize_files.unwrap_or(true),
                rename_style: input.rename_style.unwrap_or_else(|| "none".to_string()),
                naming_pattern: input.naming_pattern,
                default_quality_profile_id: input
                    .default_quality_profile_id
                    .and_then(|id| Uuid::parse_str(&id).ok()),
                auto_add_discovered: input.auto_add_discovered.unwrap_or(true),
                auto_download: input.auto_download.unwrap_or(true),
                auto_hunt: input.auto_hunt.unwrap_or(false),
                // Inline quality settings (empty = any)
                allowed_resolutions: input.allowed_resolutions.unwrap_or_default(),
                allowed_video_codecs: input.allowed_video_codecs.unwrap_or_default(),
                allowed_audio_formats: input.allowed_audio_formats.unwrap_or_default(),
                require_hdr: input.require_hdr.unwrap_or(false),
                allowed_hdr_types: input.allowed_hdr_types.unwrap_or_default(),
                allowed_sources: input.allowed_sources.unwrap_or_default(),
                release_group_blacklist: input.release_group_blacklist.unwrap_or_default(),
                release_group_whitelist: input.release_group_whitelist.unwrap_or_default(),
            })
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        tracing::info!(
            user_id = %user.user_id,
            library_id = %record.id,
            library_name = %record.name,
            library_type = %record.library_type,
            "User created library: {}",
            record.name
        );

        Ok(LibraryResult {
            success: true,
            library: Some(Library {
                id: record.id.to_string(),
                name: record.name,
                path: record.path,
                library_type: input.library_type,
                icon: record.icon.unwrap_or_else(|| "folder".to_string()),
                color: record.color.unwrap_or_else(|| "slate".to_string()),
                auto_scan: record.auto_scan,
                scan_interval_hours: record.scan_interval_minutes / 60,
                item_count: 0,
                total_size_bytes: 0,
                last_scanned_at: None,
            }),
            error: None,
        })
    }

    /// Update an existing library
    async fn update_library(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateLibraryInput,
    ) -> Result<LibraryResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let post_download_action = input.post_download_action.map(|a| {
            match a {
                PostDownloadAction::Move => "move",
                PostDownloadAction::Hardlink => "hardlink",
                PostDownloadAction::Copy => "copy",
            }
            .to_string()
        });

        let result = db
            .libraries()
            .update(
                lib_id,
                UpdateLibrary {
                    name: input.name,
                    path: input.path,
                    icon: input.icon,
                    color: input.color,
                    auto_scan: input.auto_scan,
                    scan_interval_minutes: input.scan_interval_minutes,
                    watch_for_changes: input.watch_for_changes,
                    post_download_action,
                    organize_files: input.organize_files,
                    rename_style: input.rename_style,
                    naming_pattern: input.naming_pattern,
                    default_quality_profile_id: input
                        .default_quality_profile_id
                        .and_then(|id| Uuid::parse_str(&id).ok()),
                    auto_add_discovered: input.auto_add_discovered,
                    auto_download: input.auto_download,
                    auto_hunt: input.auto_hunt,
                    // Inline quality settings
                    allowed_resolutions: input.allowed_resolutions,
                    allowed_video_codecs: input.allowed_video_codecs,
                    allowed_audio_formats: input.allowed_audio_formats,
                    require_hdr: input.require_hdr,
                    allowed_hdr_types: input.allowed_hdr_types,
                    allowed_sources: input.allowed_sources,
                    release_group_blacklist: input.release_group_blacklist,
                    release_group_whitelist: input.release_group_whitelist,
                },
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if let Some(record) = result {
            Ok(LibraryResult {
                success: true,
                library: Some(Library {
                    id: record.id.to_string(),
                    name: record.name,
                    path: record.path,
                    library_type: match record.library_type.as_str() {
                        "movies" => LibraryType::Movies,
                        "tv" => LibraryType::Tv,
                        "music" => LibraryType::Music,
                        "audiobooks" => LibraryType::Audiobooks,
                        _ => LibraryType::Other,
                    },
                    icon: record.icon.unwrap_or_else(|| "folder".to_string()),
                    color: record.color.unwrap_or_else(|| "slate".to_string()),
                    auto_scan: record.auto_scan,
                    scan_interval_hours: record.scan_interval_minutes / 60,
                    item_count: 0,
                    total_size_bytes: 0,
                    last_scanned_at: record.last_scanned_at.map(|t| t.to_rfc3339()),
                }),
                error: None,
            })
        } else {
            Ok(LibraryResult {
                success: false,
                library: None,
                error: Some("Library not found".to_string()),
            })
        }
    }

    /// Delete a library
    async fn delete_library(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let deleted = db
            .libraries()
            .delete(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: deleted,
            error: if deleted {
                None
            } else {
                Some("Library not found".to_string())
            },
        })
    }

    /// Trigger a library scan
    async fn scan_library(&self, ctx: &Context<'_>, id: String) -> Result<ScanStatus> {
        let _user = ctx.auth_user()?;
        let scanner = ctx.data_unchecked::<Arc<ScannerService>>();
        let db = ctx.data_unchecked::<Database>().clone();

        let library_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        tracing::info!("Scan requested for library: {}", id);

        // Spawn the scan in the background so the mutation returns immediately
        let scanner = scanner.clone();
        tokio::spawn(async move {
            if let Err(e) = scanner.scan_library(library_id).await {
                tracing::error!(library_id = %library_id, error = %e, "Library scan failed");
                // Ensure scanning state is reset on error
                if let Err(reset_err) = db.libraries().set_scanning(library_id, false).await {
                    tracing::error!(library_id = %library_id, error = %reset_err, "Failed to reset scanning state");
                }
            }
        });

        Ok(ScanStatus {
            library_id: id,
            status: "started".to_string(),
            message: Some("Scan has been started".to_string()),
        })
    }

    // ------------------------------------------------------------------------
    // TV Shows
    // ------------------------------------------------------------------------

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
                quality_profile_id: input
                    .quality_profile_id
                    .and_then(|id| Uuid::parse_str(&id).ok()),
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
                quality_profile_id: record.quality_profile_id.map(|id| id.to_string()),
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
                    quality_profile_id: input
                        .quality_profile_id
                        .and_then(|id| Uuid::parse_str(&id).ok()),
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
                    quality_profile_id: record.quality_profile_id.map(|id| id.to_string()),
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
                    status: None,
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
                quality_profile_id: record.quality_profile_id.map(|id| id.to_string()),
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

    // ------------------------------------------------------------------------
    // Quality Profiles
    // ------------------------------------------------------------------------

    /// Create a quality profile
    async fn create_quality_profile(
        &self,
        ctx: &Context<'_>,
        input: CreateQualityProfileInput,
    ) -> Result<QualityProfileResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let record = db
            .quality_profiles()
            .create(CreateQualityProfile {
                user_id,
                name: input.name,
                preferred_resolution: input.preferred_resolution,
                min_resolution: input.min_resolution,
                preferred_codec: input.preferred_codec,
                preferred_audio: input.preferred_audio,
                require_hdr: input.require_hdr.unwrap_or(false),
                hdr_types: input.hdr_types.unwrap_or_default(),
                preferred_language: input.preferred_language,
                max_size_gb: input
                    .max_size_gb
                    .and_then(|f| rust_decimal::Decimal::try_from(f).ok()),
                min_seeders: input.min_seeders,
                release_group_whitelist: input.release_group_whitelist.unwrap_or_default(),
                release_group_blacklist: input.release_group_blacklist.unwrap_or_default(),
                upgrade_until: input.upgrade_until,
            })
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(QualityProfileResult {
            success: true,
            quality_profile: Some(QualityProfile {
                id: record.id.to_string(),
                name: record.name,
                preferred_resolution: record.preferred_resolution,
                min_resolution: record.min_resolution,
                preferred_codec: record.preferred_codec,
                preferred_audio: record.preferred_audio,
                require_hdr: record.require_hdr,
                hdr_types: record.hdr_types,
                preferred_language: record.preferred_language,
                max_size_gb: record
                    .max_size_gb
                    .map(|d| d.to_string().parse().unwrap_or(0.0)),
                min_seeders: record.min_seeders,
                release_group_whitelist: record.release_group_whitelist,
                release_group_blacklist: record.release_group_blacklist,
                upgrade_until: record.upgrade_until,
            }),
            error: None,
        })
    }

    /// Update a quality profile
    async fn update_quality_profile(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateQualityProfileInput,
    ) -> Result<QualityProfileResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let profile_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid profile ID: {}", e)))?;

        let result = db
            .quality_profiles()
            .update(
                profile_id,
                UpdateQualityProfile {
                    name: input.name,
                    preferred_resolution: input.preferred_resolution,
                    min_resolution: input.min_resolution,
                    preferred_codec: input.preferred_codec,
                    preferred_audio: input.preferred_audio,
                    require_hdr: input.require_hdr,
                    hdr_types: input.hdr_types,
                    preferred_language: input.preferred_language,
                    max_size_gb: input
                        .max_size_gb
                        .and_then(|f| rust_decimal::Decimal::try_from(f).ok()),
                    min_seeders: input.min_seeders,
                    release_group_whitelist: input.release_group_whitelist,
                    release_group_blacklist: input.release_group_blacklist,
                    upgrade_until: input.upgrade_until,
                },
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if let Some(record) = result {
            Ok(QualityProfileResult {
                success: true,
                quality_profile: Some(QualityProfile {
                    id: record.id.to_string(),
                    name: record.name,
                    preferred_resolution: record.preferred_resolution,
                    min_resolution: record.min_resolution,
                    preferred_codec: record.preferred_codec,
                    preferred_audio: record.preferred_audio,
                    require_hdr: record.require_hdr,
                    hdr_types: record.hdr_types,
                    preferred_language: record.preferred_language,
                    max_size_gb: record
                        .max_size_gb
                        .map(|d| d.to_string().parse().unwrap_or(0.0)),
                    min_seeders: record.min_seeders,
                    release_group_whitelist: record.release_group_whitelist,
                    release_group_blacklist: record.release_group_blacklist,
                    upgrade_until: record.upgrade_until,
                }),
                error: None,
            })
        } else {
            Ok(QualityProfileResult {
                success: false,
                quality_profile: None,
                error: Some("Quality profile not found".to_string()),
            })
        }
    }

    /// Delete a quality profile
    async fn delete_quality_profile(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let profile_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid profile ID: {}", e)))?;

        let deleted = db
            .quality_profiles()
            .delete(profile_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: deleted,
            error: if deleted {
                None
            } else {
                Some("Quality profile not found".to_string())
            },
        })
    }

    // ------------------------------------------------------------------------
    // RSS Feeds
    // ------------------------------------------------------------------------

    /// Create an RSS feed
    async fn create_rss_feed(
        &self,
        ctx: &Context<'_>,
        input: CreateRssFeedInput,
    ) -> Result<RssFeedResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let record = db
            .rss_feeds()
            .create(CreateRssFeed {
                user_id,
                library_id: input.library_id.and_then(|id| Uuid::parse_str(&id).ok()),
                name: input.name,
                url: input.url,
                enabled: input.enabled.unwrap_or(true),
                poll_interval_minutes: input.poll_interval_minutes.unwrap_or(15),
            })
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(RssFeedResult {
            success: true,
            rss_feed: Some(RssFeed {
                id: record.id.to_string(),
                library_id: record.library_id.map(|id| id.to_string()),
                name: record.name,
                url: record.url,
                enabled: record.enabled,
                poll_interval_minutes: record.poll_interval_minutes,
                last_polled_at: record.last_polled_at.map(|t| t.to_rfc3339()),
                last_successful_at: record.last_successful_at.map(|t| t.to_rfc3339()),
                last_error: record.last_error,
                consecutive_failures: record.consecutive_failures.unwrap_or(0),
            }),
            error: None,
        })
    }

    /// Update an RSS feed
    async fn update_rss_feed(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateRssFeedInput,
    ) -> Result<RssFeedResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let feed_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid feed ID: {}", e)))?;

        let result = db
            .rss_feeds()
            .update(
                feed_id,
                UpdateRssFeed {
                    library_id: input.library_id.and_then(|id| Uuid::parse_str(&id).ok()),
                    name: input.name,
                    url: input.url,
                    enabled: input.enabled,
                    poll_interval_minutes: input.poll_interval_minutes,
                },
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if let Some(record) = result {
            Ok(RssFeedResult {
                success: true,
                rss_feed: Some(RssFeed {
                    id: record.id.to_string(),
                    library_id: record.library_id.map(|id| id.to_string()),
                    name: record.name,
                    url: record.url,
                    enabled: record.enabled,
                    poll_interval_minutes: record.poll_interval_minutes,
                    last_polled_at: record.last_polled_at.map(|t| t.to_rfc3339()),
                    last_successful_at: record.last_successful_at.map(|t| t.to_rfc3339()),
                    last_error: record.last_error,
                    consecutive_failures: record.consecutive_failures.unwrap_or(0),
                }),
                error: None,
            })
        } else {
            Ok(RssFeedResult {
                success: false,
                rss_feed: None,
                error: Some("RSS feed not found".to_string()),
            })
        }
    }

    /// Delete an RSS feed
    async fn delete_rss_feed(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let feed_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid feed ID: {}", e)))?;

        let deleted = db
            .rss_feeds()
            .delete(feed_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: deleted,
            error: if deleted {
                None
            } else {
                Some("RSS feed not found".to_string())
            },
        })
    }

    /// Test an RSS feed by fetching and parsing its items (without storing)
    async fn test_rss_feed(&self, ctx: &Context<'_>, url: String) -> Result<RssFeedTestResult> {
        let _user = ctx.auth_user()?;

        let rss_service = crate::services::RssService::new();
        match rss_service.fetch_feed(&url).await {
            Ok(items) => {
                let sample_items: Vec<RssItem> = items
                    .into_iter()
                    .take(10)
                    .map(|item| RssItem {
                        title: item.title,
                        link: item.link,
                        pub_date: item.pub_date.map(|d| d.to_rfc3339()),
                        description: item.description,
                        parsed_show_name: item.parsed_show_name,
                        parsed_season: item.parsed_season,
                        parsed_episode: item.parsed_episode,
                        parsed_resolution: item.parsed_resolution,
                        parsed_codec: item.parsed_codec,
                    })
                    .collect();

                Ok(RssFeedTestResult {
                    success: true,
                    item_count: sample_items.len() as i32,
                    sample_items,
                    error: None,
                })
            }
            Err(e) => Ok(RssFeedTestResult {
                success: false,
                item_count: 0,
                sample_items: vec![],
                error: Some(e.to_string()),
            }),
        }
    }

    /// Manually poll an RSS feed
    async fn poll_rss_feed(&self, ctx: &Context<'_>, id: String) -> Result<RssFeedResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let feed_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid feed ID: {}", e)))?;

        // Poll the feed using the same logic as the background job
        // This ensures items are stored AND matched to episodes
        match crate::jobs::rss_poller::poll_single_feed_by_id(db, feed_id).await {
            Ok((new_items, matched_episodes)) => {
                tracing::info!(
                    user_id = %_user.user_id,
                    feed_id = %feed_id,
                    new_items = new_items,
                    matched_episodes = matched_episodes,
                    "User manually polled RSS feed: {} new items, {} matched episodes",
                    new_items, matched_episodes
                );
                // Get updated feed
                let updated_feed = db
                    .rss_feeds()
                    .get_by_id(feed_id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("RSS feed not found"))?;

                Ok(RssFeedResult {
                    success: true,
                    rss_feed: Some(RssFeed {
                        id: updated_feed.id.to_string(),
                        library_id: updated_feed.library_id.map(|id| id.to_string()),
                        name: updated_feed.name,
                        url: updated_feed.url,
                        enabled: updated_feed.enabled,
                        poll_interval_minutes: updated_feed.poll_interval_minutes,
                        last_polled_at: updated_feed.last_polled_at.map(|t| t.to_rfc3339()),
                        last_successful_at: updated_feed.last_successful_at.map(|t| t.to_rfc3339()),
                        last_error: updated_feed.last_error,
                        consecutive_failures: updated_feed.consecutive_failures.unwrap_or(0),
                    }),
                    error: None,
                })
            }
            Err(e) => {
                // Mark poll failure
                let _ = db
                    .rss_feeds()
                    .mark_poll_failure(feed_id, &e.to_string())
                    .await;

                Ok(RssFeedResult {
                    success: false,
                    rss_feed: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    // ------------------------------------------------------------------------
    // Episodes
    // ------------------------------------------------------------------------

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
        let torrent_link = episode.torrent_link.ok_or_else(|| {
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
            Ok(torrent_info) => {
                // Link torrent to episode
                if let Err(e) = db
                    .torrents()
                    .link_to_episode(&torrent_info.info_hash, episode.id)
                    .await
                {
                    tracing::error!("Failed to link torrent to episode: {:?}", e);
                }

                // Update episode status
                if let Err(e) = db.episodes().mark_downloading(episode.id).await {
                    tracing::error!("Failed to update episode status: {:?}", e);
                }

                Ok(DownloadEpisodeResult {
                    success: true,
                    episode: Some(Episode {
                        id: episode.id.to_string(),
                        tv_show_id: episode.tv_show_id.to_string(),
                        season: episode.season,
                        episode: episode.episode,
                        absolute_number: episode.absolute_number,
                        title: episode.title,
                        overview: episode.overview,
                        air_date: episode.air_date.map(|d| d.to_string()),
                        runtime: episode.runtime,
                        status: EpisodeStatus::Downloading,
                        tvmaze_id: episode.tvmaze_id,
                        tmdb_id: episode.tmdb_id,
                        tvdb_id: episode.tvdb_id,
                        torrent_link: Some(torrent_link),
                        torrent_link_added_at: episode
                            .torrent_link_added_at
                            .map(|t| t.to_rfc3339()),
                        media_file_id: None, // Episode is being downloaded, no file yet
                    }),
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

    // ------------------------------------------------------------------------
    // Cast Devices & Sessions
    // ------------------------------------------------------------------------

    /// Trigger mDNS device discovery scan
    async fn discover_cast_devices(&self, ctx: &Context<'_>) -> Result<Vec<CastDevice>> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        // Start discovery (it runs in background)
        cast_service
            .start_discovery()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Return current list of devices
        let devices = cast_service
            .get_devices()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(devices
            .into_iter()
            .map(|d| CastDevice::from_record(d, false))
            .collect())
    }

    /// Manually add a cast device by IP address
    async fn add_cast_device(
        &self,
        ctx: &Context<'_>,
        input: AddCastDeviceInput,
    ) -> Result<CastDeviceResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let address: std::net::IpAddr = input
            .address
            .parse()
            .map_err(|_| async_graphql::Error::new("Invalid IP address"))?;

        match cast_service
            .add_device_manual(address, input.port.map(|p| p as u16), input.name)
            .await
        {
            Ok(device) => Ok(CastDeviceResult {
                success: true,
                device: Some(CastDevice::from_record(device, false)),
                error: None,
            }),
            Err(e) => Ok(CastDeviceResult {
                success: false,
                device: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update a cast device (name, favorite status)
    async fn update_cast_device(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateCastDeviceInput,
    ) -> Result<CastDeviceResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let device_id = Uuid::parse_str(&id)
            .map_err(|_| async_graphql::Error::new("Invalid device ID"))?;

        match cast_service
            .update_device(device_id, input.name, input.is_favorite)
            .await
        {
            Ok(Some(device)) => Ok(CastDeviceResult {
                success: true,
                device: Some(CastDevice::from_record(device, false)),
                error: None,
            }),
            Ok(None) => Ok(CastDeviceResult {
                success: false,
                device: None,
                error: Some("Device not found".to_string()),
            }),
            Err(e) => Ok(CastDeviceResult {
                success: false,
                device: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Remove a cast device
    async fn remove_cast_device(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let device_id = Uuid::parse_str(&id)
            .map_err(|_| async_graphql::Error::new("Invalid device ID"))?;

        match cast_service.remove_device(device_id).await {
            Ok(true) => Ok(MutationResult {
                success: true,
                error: None,
            }),
            Ok(false) => Ok(MutationResult {
                success: false,
                error: Some("Device not found".to_string()),
            }),
            Err(e) => Ok(MutationResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Cast media to a device
    async fn cast_media(&self, ctx: &Context<'_>, input: CastMediaInput) -> Result<CastSessionResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let device_id = Uuid::parse_str(&input.device_id)
            .map_err(|_| async_graphql::Error::new("Invalid device ID"))?;
        let media_file_id = Uuid::parse_str(&input.media_file_id)
            .map_err(|_| async_graphql::Error::new("Invalid media file ID"))?;
        let episode_id = input
            .episode_id
            .as_ref()
            .map(|id| Uuid::parse_str(id))
            .transpose()
            .map_err(|_| async_graphql::Error::new("Invalid episode ID"))?;

        match cast_service
            .cast_media(device_id, media_file_id, episode_id, input.start_position)
            .await
        {
            Ok(session) => {
                let device_name = cast_service
                    .get_device(device_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|d| d.name);
                Ok(CastSessionResult {
                    success: true,
                    session: Some(CastSession::from_record(session, device_name)),
                    error: None,
                })
            }
            Err(e) => Ok(CastSessionResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Play/resume current cast session
    async fn cast_play(&self, ctx: &Context<'_>, session_id: String) -> Result<CastSessionResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let id = Uuid::parse_str(&session_id)
            .map_err(|_| async_graphql::Error::new("Invalid session ID"))?;

        match cast_service.play(id).await {
            Ok(session) => {
                let device_name = if let Some(device_id) = session.device_id {
                    cast_service
                        .get_device(device_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|d| d.name)
                } else {
                    None
                };
                Ok(CastSessionResult {
                    success: true,
                    session: Some(CastSession::from_record(session, device_name)),
                    error: None,
                })
            }
            Err(e) => Ok(CastSessionResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Pause current cast session
    async fn cast_pause(&self, ctx: &Context<'_>, session_id: String) -> Result<CastSessionResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let id = Uuid::parse_str(&session_id)
            .map_err(|_| async_graphql::Error::new("Invalid session ID"))?;

        match cast_service.pause(id).await {
            Ok(session) => {
                let device_name = if let Some(device_id) = session.device_id {
                    cast_service
                        .get_device(device_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|d| d.name)
                } else {
                    None
                };
                Ok(CastSessionResult {
                    success: true,
                    session: Some(CastSession::from_record(session, device_name)),
                    error: None,
                })
            }
            Err(e) => Ok(CastSessionResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Stop casting and end session
    async fn cast_stop(&self, ctx: &Context<'_>, session_id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let id = Uuid::parse_str(&session_id)
            .map_err(|_| async_graphql::Error::new("Invalid session ID"))?;

        match cast_service.stop(id).await {
            Ok(_) => Ok(MutationResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(MutationResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Seek to position in current cast session
    async fn cast_seek(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        position: f64,
    ) -> Result<CastSessionResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let id = Uuid::parse_str(&session_id)
            .map_err(|_| async_graphql::Error::new("Invalid session ID"))?;

        match cast_service.seek(id, position).await {
            Ok(session) => {
                let device_name = if let Some(device_id) = session.device_id {
                    cast_service
                        .get_device(device_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|d| d.name)
                } else {
                    None
                };
                Ok(CastSessionResult {
                    success: true,
                    session: Some(CastSession::from_record(session, device_name)),
                    error: None,
                })
            }
            Err(e) => Ok(CastSessionResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Set volume for current cast session
    async fn cast_set_volume(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        volume: f32,
    ) -> Result<CastSessionResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let id = Uuid::parse_str(&session_id)
            .map_err(|_| async_graphql::Error::new("Invalid session ID"))?;

        match cast_service.set_volume(id, volume).await {
            Ok(session) => {
                let device_name = if let Some(device_id) = session.device_id {
                    cast_service
                        .get_device(device_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|d| d.name)
                } else {
                    None
                };
                Ok(CastSessionResult {
                    success: true,
                    session: Some(CastSession::from_record(session, device_name)),
                    error: None,
                })
            }
            Err(e) => Ok(CastSessionResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Set mute state for current cast session
    async fn cast_set_muted(
        &self,
        ctx: &Context<'_>,
        session_id: String,
        muted: bool,
    ) -> Result<CastSessionResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let id = Uuid::parse_str(&session_id)
            .map_err(|_| async_graphql::Error::new("Invalid session ID"))?;

        match cast_service.set_muted(id, muted).await {
            Ok(session) => {
                let device_name = if let Some(device_id) = session.device_id {
                    cast_service
                        .get_device(device_id)
                        .await
                        .ok()
                        .flatten()
                        .map(|d| d.name)
                } else {
                    None
                };
                Ok(CastSessionResult {
                    success: true,
                    session: Some(CastSession::from_record(session, device_name)),
                    error: None,
                })
            }
            Err(e) => Ok(CastSessionResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update cast settings
    async fn update_cast_settings(
        &self,
        ctx: &Context<'_>,
        input: UpdateCastSettingsInput,
    ) -> Result<CastSettingsResult> {
        let _user = ctx.auth_user()?;
        let cast_service = ctx.data_unchecked::<Arc<CastService>>();

        let db_input = crate::db::UpdateCastSettings {
            auto_discovery_enabled: input.auto_discovery_enabled,
            discovery_interval_seconds: input.discovery_interval_seconds,
            default_volume: input.default_volume,
            transcode_incompatible: input.transcode_incompatible,
            preferred_quality: input.preferred_quality,
        };

        match cast_service.update_settings(db_input).await {
            Ok(settings) => Ok(CastSettingsResult {
                success: true,
                settings: Some(CastSettings::from_record(settings)),
                error: None,
            }),
            Err(e) => Ok(CastSettingsResult {
                success: false,
                settings: None,
                error: Some(e.to_string()),
            }),
        }
    }

    // ------------------------------------------------------------------------
    // Torrents
    // ------------------------------------------------------------------------

    /// Add a new torrent from magnet link or URL to a .torrent file
    async fn add_torrent(
        &self,
        ctx: &Context<'_>,
        input: AddTorrentInput,
    ) -> Result<AddTorrentResult> {
        let user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();

        // Parse user_id for database persistence
        let user_id = Uuid::parse_str(&user.user_id).ok();

        let result = if let Some(magnet) = input.magnet {
            // Magnet links go through add_magnet
            service.add_magnet(&magnet, user_id).await
        } else if let Some(url) = input.url {
            // URLs (to .torrent files or magnet links) go through add_torrent_url
            service.add_torrent_url(&url, user_id).await
        } else {
            return Ok(AddTorrentResult {
                success: false,
                torrent: None,
                error: Some("Either magnet or url must be provided".to_string()),
            });
        };

        match result {
            Ok(info) => {
                tracing::info!(
                    user_id = %user.user_id,
                    torrent_id = info.id,
                    torrent_name = %info.name,
                    info_hash = %info.info_hash,
                    "User added torrent: {}",
                    info.name
                );
                Ok(AddTorrentResult {
                    success: true,
                    torrent: Some(info.into()),
                    error: None,
                })
            }
            Err(e) => {
                tracing::warn!(
                    user_id = %user.user_id,
                    error = %e,
                    "User failed to add torrent"
                );
                Ok(AddTorrentResult {
                    success: false,
                    torrent: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Pause a torrent
    async fn pause_torrent(&self, ctx: &Context<'_>, id: i32) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();

        match service.pause(id as usize).await {
            Ok(_) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Resume a paused torrent
    async fn resume_torrent(&self, ctx: &Context<'_>, id: i32) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();

        match service.resume(id as usize).await {
            Ok(_) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Remove a torrent
    async fn remove_torrent(
        &self,
        ctx: &Context<'_>,
        id: i32,
        #[graphql(default = false)] delete_files: bool,
    ) -> Result<TorrentActionResult> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();

        match service.remove(id as usize, delete_files).await {
            Ok(_) => Ok(TorrentActionResult {
                success: true,
                error: None,
            }),
            Err(e) => Ok(TorrentActionResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Organize a completed torrent's files into the library structure
    ///
    /// This processes the torrent files:
    /// 1. Parses filenames to identify show/episode
    /// 2. Matches to existing shows in the library
    /// 3. Copies/moves/hardlinks files based on library settings
    /// 4. Creates folder structure (Show Name/Season XX/)
    /// 5. Updates episode status to downloaded
    async fn organize_torrent(
        &self,
        ctx: &Context<'_>,
        id: i32,
        #[graphql(
            desc = "Optional library ID to organize into (uses first TV library if not specified)"
        )]
        library_id: Option<String>,
    ) -> Result<OrganizeTorrentResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();

        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let library_uuid = if let Some(ref lib_id) = library_id {
            Some(
                Uuid::parse_str(lib_id)
                    .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?,
            )
        } else {
            None
        };

        // Get torrent info to get info_hash
        let torrent_info = match torrent_service.get_torrent_info(id as usize).await {
            Ok(info) => info,
            Err(e) => {
                return Ok(OrganizeTorrentResult {
                    success: false,
                    organized_count: 0,
                    failed_count: 0,
                    messages: vec![format!("Failed to get torrent info: {}", e)],
                });
            }
        };

        // Get files from the torrent
        let files = match torrent_service
            .get_files_for_torrent(&torrent_info.info_hash)
            .await
        {
            Ok(f) => f,
            Err(e) => {
                return Ok(OrganizeTorrentResult {
                    success: false,
                    organized_count: 0,
                    failed_count: 0,
                    messages: vec![format!("Failed to get torrent files: {}", e)],
                });
            }
        };

        // Convert to organizer format
        let files_for_organize: Vec<crate::services::TorrentFileForOrganize> = files
            .into_iter()
            .map(|f| crate::services::TorrentFileForOrganize {
                path: f.path,
                size: f.size,
            })
            .collect();

        // Run the organizer
        let organizer = crate::services::OrganizerService::new(db.clone());

        match organizer
            .organize_torrent(
                &torrent_info.info_hash,
                files_for_organize,
                user_id,
                library_uuid,
            )
            .await
        {
            Ok(result) => Ok(OrganizeTorrentResult {
                success: result.success,
                organized_count: result.organized_count,
                failed_count: result.failed_count,
                messages: result.messages,
            }),
            Err(e) => Ok(OrganizeTorrentResult {
                success: false,
                organized_count: 0,
                failed_count: 0,
                messages: vec![format!("Organization failed: {}", e)],
            }),
        }
    }

    // ------------------------------------------------------------------------
    // Settings
    // ------------------------------------------------------------------------

    /// Update torrent client settings
    async fn update_torrent_settings(
        &self,
        ctx: &Context<'_>,
        input: UpdateTorrentSettingsInput,
    ) -> Result<SettingsResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        // Update each setting if provided
        if let Some(v) = input.download_dir {
            settings
                .set_with_category("torrent.download_dir", v, "torrent", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.session_dir {
            settings
                .set_with_category("torrent.session_dir", v, "torrent", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.enable_dht {
            settings
                .set_with_category("torrent.enable_dht", v, "torrent", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.listen_port {
            settings
                .set_with_category("torrent.listen_port", v, "torrent", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.max_concurrent {
            settings
                .set_with_category("torrent.max_concurrent", v, "torrent", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.upload_limit {
            settings
                .set_with_category("torrent.upload_limit", v, "torrent", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.download_limit {
            settings
                .set_with_category("torrent.download_limit", v, "torrent", None)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }

        Ok(SettingsResult {
            success: true,
            error: None,
        })
    }

    // ------------------------------------------------------------------------
    // Subscriptions (legacy)
    // ------------------------------------------------------------------------

    /// Create a new subscription (legacy)
    async fn create_subscription(
        &self,
        ctx: &Context<'_>,
        input: CreateSubscriptionInput,
    ) -> Result<SubscriptionResult> {
        let _user = ctx.auth_user()?;

        let subscription = Subscription {
            id: Uuid::new_v4().to_string(),
            name: input.name,
            tvdb_id: input.tvdb_id,
            tmdb_id: input.tmdb_id,
            quality_profile_id: input.quality_profile_id,
            monitored: input.monitored.unwrap_or(true),
            last_checked_at: None,
            episode_count: 0,
        };

        // TODO: Insert into database

        Ok(SubscriptionResult {
            success: true,
            subscription: Some(subscription),
            error: None,
        })
    }

    /// Update an existing subscription (legacy)
    async fn update_subscription(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateSubscriptionInput,
    ) -> Result<SubscriptionResult> {
        let _user = ctx.auth_user()?;
        let _id = id;
        let _input = input;
        // TODO: Update in database
        Ok(SubscriptionResult {
            success: false,
            subscription: None,
            error: Some("Not implemented".to_string()),
        })
    }

    /// Delete a subscription (legacy)
    async fn delete_subscription(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let _id = id;
        // TODO: Delete from database
        Ok(MutationResult {
            success: true,
            error: None,
        })
    }

    /// Manually trigger a search for a subscription (legacy)
    async fn search_subscription(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Vec<SearchResult>> {
        let _user = ctx.auth_user()?;
        let _id = id;
        // TODO: Implement Torznab search
        Ok(vec![])
    }

    // ------------------------------------------------------------------------
    // Logs
    // ------------------------------------------------------------------------

    /// Clear all logs
    async fn clear_all_logs(&self, ctx: &Context<'_>) -> Result<ClearLogsResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let deleted = db
            .logs()
            .delete_all()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(ClearLogsResult {
            success: true,
            deleted_count: deleted as i64,
            error: None,
        })
    }

    /// Clear logs older than a specified number of days
    async fn clear_old_logs(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Delete logs older than this many days")] days: i32,
    ) -> Result<ClearLogsResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let before = time::OffsetDateTime::now_utc() - time::Duration::days(days as i64);

        let deleted = db
            .logs()
            .delete_before(before)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(ClearLogsResult {
            success: true,
            deleted_count: deleted as i64,
            error: None,
        })
    }

    // ------------------------------------------------------------------------
    // Indexers
    // ------------------------------------------------------------------------

    /// Create a new indexer
    async fn create_indexer(
        &self,
        ctx: &Context<'_>,
        input: CreateIndexerInput,
    ) -> Result<IndexerResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        // Validate indexer type exists
        use crate::indexer::definitions::get_indexer_info;
        if get_indexer_info(&input.indexer_type).is_none() {
            return Ok(IndexerResult {
                success: false,
                error: Some(format!("Unknown indexer type: {}", input.indexer_type)),
                indexer: None,
            });
        }

        // Create the indexer config
        let create_data = crate::db::CreateIndexerConfig {
            user_id,
            indexer_type: input.indexer_type.clone(),
            definition_id: None,
            name: input.name.clone(),
            site_url: input.site_url.clone(),
        };

        let record = match db.indexers().create(create_data).await {
            Ok(r) => r,
            Err(e) => {
                return Ok(IndexerResult {
                    success: false,
                    error: Some(format!("Failed to create indexer: {}", e)),
                    indexer: None,
                });
            }
        };

        // Get encryption key from database (auto-generates if not set)
        let encryption_key = match db.settings().get_or_create_indexer_encryption_key().await {
            Ok(key) => key,
            Err(e) => {
                return Ok(IndexerResult {
                    success: false,
                    error: Some(format!("Failed to get encryption key: {}", e)),
                    indexer: None,
                });
            }
        };

        let encryption = match crate::indexer::encryption::CredentialEncryption::from_base64_key(
            &encryption_key,
        ) {
            Ok(e) => e,
            Err(e) => {
                return Ok(IndexerResult {
                    success: false,
                    error: Some(format!("Encryption error: {}", e)),
                    indexer: None,
                });
            }
        };

        // Store encrypted credentials
        for cred in input.credentials {
            let (encrypted_value, nonce) = match encryption.encrypt(&cred.value) {
                Ok(v) => v,
                Err(e) => {
                    return Ok(IndexerResult {
                        success: false,
                        error: Some(format!("Failed to encrypt credential: {}", e)),
                        indexer: None,
                    });
                }
            };

            let upsert = crate::db::UpsertCredential {
                credential_type: cred.credential_type,
                encrypted_value,
                nonce,
            };

            if let Err(e) = db.indexers().upsert_credential(record.id, upsert).await {
                return Ok(IndexerResult {
                    success: false,
                    error: Some(format!("Failed to save credential: {}", e)),
                    indexer: None,
                });
            }
        }

        // Store settings
        for setting in input.settings {
            if let Err(e) = db
                .indexers()
                .upsert_setting(record.id, &setting.key, &setting.value)
                .await
            {
                return Ok(IndexerResult {
                    success: false,
                    error: Some(format!("Failed to save setting: {}", e)),
                    indexer: None,
                });
            }
        }

        Ok(IndexerResult {
            success: true,
            error: None,
            indexer: Some(IndexerConfig {
                id: record.id.to_string(),
                indexer_type: record.indexer_type,
                name: record.name,
                enabled: record.enabled,
                priority: record.priority,
                site_url: record.site_url,
                is_healthy: true,
                last_error: None,
                error_count: 0,
                last_success_at: None,
                created_at: record.created_at.to_rfc3339(),
                updated_at: record.updated_at.to_rfc3339(),
                capabilities: IndexerCapabilities {
                    supports_search: true,
                    supports_tv_search: true,
                    supports_movie_search: true,
                    supports_music_search: false,
                    supports_book_search: false,
                    supports_imdb_search: false,
                    supports_tvdb_search: false,
                },
            }),
        })
    }

    /// Update an existing indexer
    async fn update_indexer(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateIndexerInput,
    ) -> Result<IndexerResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let config_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid indexer ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)?;

        // Verify ownership
        let existing = db
            .indexers()
            .get(config_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let _existing = match existing {
            Some(r) if r.user_id == user_id => r,
            Some(_) => {
                return Ok(IndexerResult {
                    success: false,
                    error: Some("Indexer not found".to_string()),
                    indexer: None,
                });
            }
            None => {
                return Ok(IndexerResult {
                    success: false,
                    error: Some("Indexer not found".to_string()),
                    indexer: None,
                });
            }
        };

        // Update config
        let update_data = crate::db::UpdateIndexerConfig {
            name: input.name,
            enabled: input.enabled,
            priority: input.priority,
            site_url: input.site_url,
            ..Default::default()
        };

        let record = match db.indexers().update(config_id, update_data).await {
            Ok(Some(r)) => r,
            Ok(None) => {
                return Ok(IndexerResult {
                    success: false,
                    error: Some("Indexer not found".to_string()),
                    indexer: None,
                });
            }
            Err(e) => {
                return Ok(IndexerResult {
                    success: false,
                    error: Some(format!("Failed to update indexer: {}", e)),
                    indexer: None,
                });
            }
        };

        // Update credentials if provided
        if let Some(credentials) = input.credentials {
            let encryption_key = db
                .settings()
                .get_or_create_indexer_encryption_key()
                .await
                .map_err(|e| {
                    async_graphql::Error::new(format!("Failed to get encryption key: {}", e))
                })?;
            let encryption =
                crate::indexer::encryption::CredentialEncryption::from_base64_key(&encryption_key)
                    .map_err(|e| async_graphql::Error::new(format!("Encryption error: {}", e)))?;

            for cred in credentials {
                let (encrypted_value, nonce) = encryption
                    .encrypt(&cred.value)
                    .map_err(|e| async_graphql::Error::new(format!("Encryption error: {}", e)))?;

                let upsert = crate::db::UpsertCredential {
                    credential_type: cred.credential_type,
                    encrypted_value,
                    nonce,
                };

                db.indexers()
                    .upsert_credential(config_id, upsert)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            }
        }

        // Update settings if provided
        if let Some(settings) = input.settings {
            for setting in settings {
                db.indexers()
                    .upsert_setting(config_id, &setting.key, &setting.value)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            }
        }

        Ok(IndexerResult {
            success: true,
            error: None,
            indexer: Some(IndexerConfig {
                id: record.id.to_string(),
                indexer_type: record.indexer_type,
                name: record.name,
                enabled: record.enabled,
                priority: record.priority,
                site_url: record.site_url,
                is_healthy: record.error_count == 0,
                last_error: record.last_error,
                error_count: record.error_count,
                last_success_at: record.last_success_at.map(|dt| dt.to_rfc3339()),
                created_at: record.created_at.to_rfc3339(),
                updated_at: record.updated_at.to_rfc3339(),
                capabilities: IndexerCapabilities {
                    supports_search: record.supports_search.unwrap_or(true),
                    supports_tv_search: record.supports_tv_search.unwrap_or(true),
                    supports_movie_search: record.supports_movie_search.unwrap_or(true),
                    supports_music_search: record.supports_music_search.unwrap_or(false),
                    supports_book_search: record.supports_book_search.unwrap_or(false),
                    supports_imdb_search: record.supports_imdb_search.unwrap_or(false),
                    supports_tvdb_search: record.supports_tvdb_search.unwrap_or(false),
                },
            }),
        })
    }

    /// Delete an indexer
    async fn delete_indexer(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let config_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid indexer ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)?;

        // Verify ownership
        let existing = db
            .indexers()
            .get(config_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        match existing {
            Some(r) if r.user_id == user_id => {}
            _ => {
                return Ok(MutationResult {
                    success: false,
                    error: Some("Indexer not found".to_string()),
                });
            }
        }

        let deleted = db
            .indexers()
            .delete(config_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: deleted,
            error: if deleted {
                None
            } else {
                Some("Failed to delete indexer".to_string())
            },
        })
    }

    /// Test an indexer connection
    async fn test_indexer(&self, ctx: &Context<'_>, id: String) -> Result<IndexerTestResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let config_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid indexer ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)?;

        tracing::info!(
            indexer_id = %config_id,
            user_id = %user_id,
            "Testing indexer connection"
        );

        // Verify ownership
        let config = match db
            .indexers()
            .get(config_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
        {
            Some(r) if r.user_id == user_id => r,
            _ => {
                tracing::warn!(indexer_id = %config_id, "Indexer not found or not owned by user");
                return Ok(IndexerTestResult {
                    success: false,
                    error: Some("Indexer not found".to_string()),
                    releases_found: None,
                    elapsed_ms: None,
                });
            }
        };

        tracing::debug!(
            indexer_id = %config_id,
            indexer_name = %config.name,
            indexer_type = %config.indexer_type,
            "Found indexer config"
        );

        // Get credentials
        let credentials = db
            .indexers()
            .get_credentials(config_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        tracing::debug!(
            indexer_id = %config_id,
            credential_count = credentials.len(),
            credential_types = ?credentials.iter().map(|c| &c.credential_type).collect::<Vec<_>>(),
            "Retrieved credentials"
        );

        let settings = db
            .indexers()
            .get_settings(config_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        tracing::debug!(
            indexer_id = %config_id,
            settings_count = settings.len(),
            "Retrieved settings"
        );

        // Decrypt credentials using database-stored key
        let encryption_key = match db.settings().get_or_create_indexer_encryption_key().await {
            Ok(key) => key,
            Err(e) => {
                tracing::error!(error = %e, "Failed to get encryption key from database");
                return Ok(IndexerTestResult {
                    success: false,
                    error: Some(format!("Failed to get encryption key: {}", e)),
                    releases_found: None,
                    elapsed_ms: None,
                });
            }
        };
        let encryption = match crate::indexer::encryption::CredentialEncryption::from_base64_key(
            &encryption_key,
        ) {
            Ok(e) => e,
            Err(e) => {
                tracing::error!(error = %e, "Failed to initialize encryption");
                return Ok(IndexerTestResult {
                    success: false,
                    error: Some(format!("Encryption error: {}", e)),
                    releases_found: None,
                    elapsed_ms: None,
                });
            }
        };

        let mut decrypted_creds: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for cred in credentials {
            match encryption.decrypt(&cred.encrypted_value, &cred.nonce) {
                Ok(value) => {
                    tracing::debug!(
                        credential_type = %cred.credential_type,
                        value_len = value.len(),
                        "Decrypted credential"
                    );
                    decrypted_creds.insert(cred.credential_type, value);
                }
                Err(e) => {
                    tracing::error!(
                        credential_type = %cred.credential_type,
                        error = %e,
                        "Failed to decrypt credential"
                    );
                    return Ok(IndexerTestResult {
                        success: false,
                        error: Some(format!("Failed to decrypt credential: {}", e)),
                        releases_found: None,
                        elapsed_ms: None,
                    });
                }
            }
        }

        let settings_map: std::collections::HashMap<String, String> = settings
            .into_iter()
            .map(|s| (s.setting_key, s.setting_value))
            .collect();

        // Create indexer instance and test
        let start = std::time::Instant::now();

        match config.indexer_type.as_str() {
            "iptorrents" => {
                use crate::indexer::definitions::iptorrents::IPTorrentsIndexer;
                use crate::indexer::{Indexer, TorznabQuery};

                let cookie = decrypted_creds.get("cookie").cloned().unwrap_or_default();
                let user_agent = decrypted_creds
                    .get("user_agent")
                    .cloned()
                    .unwrap_or_default();

                tracing::info!(
                    indexer_id = %config_id,
                    indexer_name = %config.name,
                    cookie_len = cookie.len(),
                    has_user_agent = !user_agent.is_empty(),
                    "Creating IPTorrents indexer instance"
                );

                if cookie.is_empty() {
                    tracing::warn!(indexer_id = %config_id, "Cookie is empty - authentication will likely fail");
                }

                let indexer = match IPTorrentsIndexer::new(
                    config_id.to_string(),
                    config.name.clone(),
                    config.site_url.clone(),
                    &cookie,
                    &user_agent,
                    settings_map,
                ) {
                    Ok(idx) => idx,
                    Err(e) => {
                        tracing::error!(
                            indexer_id = %config_id,
                            error = %e,
                            "Failed to create indexer instance"
                        );
                        return Ok(IndexerTestResult {
                            success: false,
                            error: Some(format!("Failed to create indexer: {}", e)),
                            releases_found: None,
                            elapsed_ms: Some(start.elapsed().as_millis() as i64),
                        });
                    }
                };

                tracing::debug!(indexer_id = %config_id, "Testing connection...");

                // Test connection
                match indexer.test_connection().await {
                    Ok(true) => {
                        tracing::info!(indexer_id = %config_id, "Connection test passed, performing search...");

                        // Try a simple search
                        let query = TorznabQuery::search("");
                        match indexer.search(&query).await {
                            Ok(releases) => {
                                tracing::info!(
                                    indexer_id = %config_id,
                                    releases_found = releases.len(),
                                    elapsed_ms = start.elapsed().as_millis(),
                                    "Indexer test successful"
                                );

                                // Record success
                                let _ = db.indexers().record_success(config_id).await;

                                Ok(IndexerTestResult {
                                    success: true,
                                    error: None,
                                    releases_found: Some(releases.len() as i32),
                                    elapsed_ms: Some(start.elapsed().as_millis() as i64),
                                })
                            }
                            Err(e) => {
                                tracing::error!(
                                    indexer_id = %config_id,
                                    error = %e,
                                    elapsed_ms = start.elapsed().as_millis(),
                                    "Search failed"
                                );
                                let _ = db.indexers().record_error(config_id, &e.to_string()).await;

                                Ok(IndexerTestResult {
                                    success: false,
                                    error: Some(format!("Search failed: {}", e)),
                                    releases_found: None,
                                    elapsed_ms: Some(start.elapsed().as_millis() as i64),
                                })
                            }
                        }
                    }
                    Ok(false) => {
                        tracing::warn!(
                            indexer_id = %config_id,
                            elapsed_ms = start.elapsed().as_millis(),
                            "Connection test returned false - likely invalid cookie"
                        );
                        let _ = db
                            .indexers()
                            .record_error(config_id, "Connection test failed")
                            .await;

                        Ok(IndexerTestResult {
                            success: false,
                            error: Some("Connection test failed - check your cookie".to_string()),
                            releases_found: None,
                            elapsed_ms: Some(start.elapsed().as_millis() as i64),
                        })
                    }
                    Err(e) => {
                        tracing::error!(
                            indexer_id = %config_id,
                            error = %e,
                            elapsed_ms = start.elapsed().as_millis(),
                            "Connection test error"
                        );
                        let _ = db.indexers().record_error(config_id, &e.to_string()).await;

                        Ok(IndexerTestResult {
                            success: false,
                            error: Some(format!("Connection error: {}", e)),
                            releases_found: None,
                            elapsed_ms: Some(start.elapsed().as_millis() as i64),
                        })
                    }
                }
            }
            other => {
                tracing::error!(
                    indexer_id = %config_id,
                    indexer_type = other,
                    "Unsupported indexer type"
                );
                Ok(IndexerTestResult {
                    success: false,
                    error: Some(format!("Unsupported indexer type: {}", other)),
                    releases_found: None,
                    elapsed_ms: Some(start.elapsed().as_millis() as i64),
                })
            }
        }
    }

    /// Search indexers for torrents
    async fn search_indexers(
        &self,
        ctx: &Context<'_>,
        input: IndexerSearchInput,
    ) -> Result<IndexerSearchResultSet> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let start = std::time::Instant::now();

        // Get indexers to search
        let indexer_ids: Option<Vec<Uuid>> = input.indexer_ids.as_ref().map(|ids| {
            ids.iter()
                .filter_map(|id| Uuid::parse_str(id).ok())
                .collect()
        });

        let configs = if let Some(ref ids) = indexer_ids {
            let all_configs = db
                .indexers()
                .list_enabled_by_user(user_id)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            all_configs
                .into_iter()
                .filter(|c| ids.contains(&c.id))
                .collect()
        } else {
            db.indexers()
                .list_enabled_by_user(user_id)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?
        };

        // Get encryption key from database
        let encryption_key = db
            .settings()
            .get_or_create_indexer_encryption_key()
            .await
            .map_err(|e| {
                async_graphql::Error::new(format!("Failed to get encryption key: {}", e))
            })?;
        let encryption =
            crate::indexer::encryption::CredentialEncryption::from_base64_key(&encryption_key)
                .map_err(|e| async_graphql::Error::new(format!("Encryption error: {}", e)))?;

        // Build query
        use crate::indexer::{QueryType, TorznabQuery};
        let query = TorznabQuery {
            query_type: if input.season.is_some() || input.episode.is_some() {
                QueryType::TvSearch
            } else if input.imdb_id.is_some() {
                QueryType::MovieSearch
            } else {
                QueryType::Search
            },
            search_term: Some(input.query.clone()),
            categories: input.categories.unwrap_or_default(),
            season: input.season,
            episode: input.episode,
            imdb_id: input.imdb_id,
            limit: input.limit,
            cache: true,
            ..Default::default()
        };

        let mut results: Vec<IndexerSearchResultItem> = Vec::new();
        let mut total_releases = 0;

        // Search each indexer
        for config in configs {
            let indexer_start = std::time::Instant::now();

            // Get and decrypt credentials
            let credentials = match db.indexers().get_credentials(config.id).await {
                Ok(c) => c,
                Err(e) => {
                    results.push(IndexerSearchResultItem {
                        indexer_id: config.id.to_string(),
                        indexer_name: config.name.clone(),
                        releases: vec![],
                        elapsed_ms: indexer_start.elapsed().as_millis() as i64,
                        from_cache: false,
                        error: Some(format!("Failed to get credentials: {}", e)),
                    });
                    continue;
                }
            };

            let settings = db
                .indexers()
                .get_settings(config.id)
                .await
                .unwrap_or_default();

            let mut decrypted_creds: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for cred in credentials {
                if let Ok(value) = encryption.decrypt(&cred.encrypted_value, &cred.nonce) {
                    decrypted_creds.insert(cred.credential_type, value);
                }
            }

            let settings_map: std::collections::HashMap<String, String> = settings
                .into_iter()
                .map(|s| (s.setting_key, s.setting_value))
                .collect();

            // Create and search indexer
            match config.indexer_type.as_str() {
                "iptorrents" => {
                    use crate::indexer::Indexer;
                    use crate::indexer::definitions::iptorrents::IPTorrentsIndexer;

                    let cookie = decrypted_creds.get("cookie").cloned().unwrap_or_default();
                    let user_agent = decrypted_creds
                        .get("user_agent")
                        .cloned()
                        .unwrap_or_default();

                    let indexer = match IPTorrentsIndexer::new(
                        config.id.to_string(),
                        config.name.clone(),
                        config.site_url.clone(),
                        &cookie,
                        &user_agent,
                        settings_map,
                    ) {
                        Ok(idx) => idx,
                        Err(e) => {
                            results.push(IndexerSearchResultItem {
                                indexer_id: config.id.to_string(),
                                indexer_name: config.name,
                                releases: vec![],
                                elapsed_ms: indexer_start.elapsed().as_millis() as i64,
                                from_cache: false,
                                error: Some(format!("Failed to create indexer: {}", e)),
                            });
                            continue;
                        }
                    };

                    match indexer.search(&query).await {
                        Ok(releases) => {
                            let _ = db.indexers().record_success(config.id).await;

                            let torrent_releases: Vec<TorrentRelease> = releases
                                .iter()
                                .map(|r| TorrentRelease {
                                    title: r.title.clone(),
                                    guid: r.guid.clone(),
                                    link: r.link.clone(),
                                    magnet_uri: r.magnet_uri.clone(),
                                    info_hash: r.info_hash.clone(),
                                    details: r.details.clone(),
                                    publish_date: r.publish_date.to_rfc3339(),
                                    categories: r.categories.clone(),
                                    size: r.size,
                                    size_formatted: r.size.map(|s| format_bytes(s as u64)),
                                    seeders: r.seeders,
                                    leechers: r.leechers(),
                                    peers: r.peers,
                                    grabs: r.grabs,
                                    is_freeleech: r.is_freeleech(),
                                    imdb_id: r.imdb.map(|id| format!("tt{:07}", id)),
                                    poster: r.poster.clone(),
                                    description: r.description.clone(),
                                    indexer_id: Some(config.id.to_string()),
                                    indexer_name: Some(config.name.clone()),
                                })
                                .collect();

                            total_releases += torrent_releases.len() as i32;

                            results.push(IndexerSearchResultItem {
                                indexer_id: config.id.to_string(),
                                indexer_name: config.name,
                                releases: torrent_releases,
                                elapsed_ms: indexer_start.elapsed().as_millis() as i64,
                                from_cache: false,
                                error: None,
                            });
                        }
                        Err(e) => {
                            let _ = db.indexers().record_error(config.id, &e.to_string()).await;

                            results.push(IndexerSearchResultItem {
                                indexer_id: config.id.to_string(),
                                indexer_name: config.name,
                                releases: vec![],
                                elapsed_ms: indexer_start.elapsed().as_millis() as i64,
                                from_cache: false,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                }
                _ => {
                    results.push(IndexerSearchResultItem {
                        indexer_id: config.id.to_string(),
                        indexer_name: config.name,
                        releases: vec![],
                        elapsed_ms: indexer_start.elapsed().as_millis() as i64,
                        from_cache: false,
                        error: Some(format!("Unsupported indexer type: {}", config.indexer_type)),
                    });
                }
            }
        }

        Ok(IndexerSearchResultSet {
            indexers: results,
            total_releases,
            total_elapsed_ms: start.elapsed().as_millis() as i64,
        })
    }

    // ========================================================================
    // Security Settings Mutations
    // ========================================================================

    /// Generate a new encryption key for indexer credentials
    ///
    /// WARNING: This will invalidate ALL existing indexer credentials!
    /// You will need to re-enter the credentials for all indexers.
    async fn regenerate_encryption_key(
        &self,
        ctx: &Context<'_>,
        input: GenerateEncryptionKeyInput,
    ) -> Result<SecuritySettingsResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        if !input.confirm_invalidation {
            return Ok(SecuritySettingsResult {
                success: false,
                error: Some("You must confirm that you understand this will invalidate existing credentials".to_string()),
                settings: None,
            });
        }

        // Generate a new key (32 bytes = 256 bits for AES-256)
        use rand::RngCore;
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let new_key = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key_bytes);

        // Store the new key
        if let Err(e) = db.settings().set_indexer_encryption_key(&new_key).await {
            return Ok(SecuritySettingsResult {
                success: false,
                error: Some(format!("Failed to save new encryption key: {}", e)),
                settings: None,
            });
        }

        tracing::warn!(
            "Indexer encryption key regenerated - all existing credentials are now invalid"
        );

        // Return the new settings
        let preview = format!("{}...{}", &new_key[..4], &new_key[new_key.len() - 4..]);

        Ok(SecuritySettingsResult {
            success: true,
            error: None,
            settings: Some(SecuritySettings {
                encryption_key_set: true,
                encryption_key_preview: Some(preview),
                encryption_key_last_modified: Some(chrono::Utc::now().to_rfc3339()),
            }),
        })
    }

    /// Initialize the encryption key if not already set
    ///
    /// This is safe to call multiple times - it will only create a key if one doesn't exist.
    async fn initialize_encryption_key(&self, ctx: &Context<'_>) -> Result<SecuritySettingsResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        // This will create a key only if one doesn't exist
        let key = match db.settings().get_or_create_indexer_encryption_key().await {
            Ok(k) => k,
            Err(e) => {
                return Ok(SecuritySettingsResult {
                    success: false,
                    error: Some(format!("Failed to initialize encryption key: {}", e)),
                    settings: None,
                });
            }
        };

        let preview = if key.len() >= 8 {
            format!("{}...{}", &key[..4], &key[key.len() - 4..])
        } else {
            "****".to_string()
        };

        Ok(SecuritySettingsResult {
            success: true,
            error: None,
            settings: Some(SecuritySettings {
                encryption_key_set: true,
                encryption_key_preview: Some(preview),
                encryption_key_last_modified: None,
            }),
        })
    }

    // ========================================================================
    // Filesystem Mutations
    // ========================================================================

    /// Create a directory on the filesystem
    async fn create_directory(
        &self,
        ctx: &Context<'_>,
        input: CreateDirectoryInput,
    ) -> Result<FileOperationResult> {
        let user = ctx.auth_user()?;
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match fs_service.create_directory(&input.path, user_id).await {
            Ok(path) => Ok(FileOperationResult {
                success: true,
                error: None,
                affected_count: 1,
                messages: vec![format!("Created directory: {}", path)],
                path: Some(path),
            }),
            Err(e) => Ok(FileOperationResult {
                success: false,
                error: Some(e.to_string()),
                affected_count: 0,
                messages: vec![],
                path: None,
            }),
        }
    }

    /// Delete files or directories (must be inside a library)
    async fn delete_files(
        &self,
        ctx: &Context<'_>,
        input: DeleteFilesInput,
    ) -> Result<FileOperationResult> {
        let user = ctx.auth_user()?;
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match fs_service
            .delete_files(&input.paths, input.recursive.unwrap_or(false), user_id)
            .await
        {
            Ok((count, messages)) => Ok(FileOperationResult {
                success: count > 0,
                error: if count == 0 {
                    Some("No files were deleted".to_string())
                } else {
                    None
                },
                affected_count: count,
                messages,
                path: None,
            }),
            Err(e) => Ok(FileOperationResult {
                success: false,
                error: Some(e.to_string()),
                affected_count: 0,
                messages: vec![],
                path: None,
            }),
        }
    }

    /// Copy files or directories (source and destination must be inside libraries)
    async fn copy_files(
        &self,
        ctx: &Context<'_>,
        input: CopyFilesInput,
    ) -> Result<FileOperationResult> {
        let user = ctx.auth_user()?;
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match fs_service
            .copy_files(
                &input.sources,
                &input.destination,
                input.overwrite.unwrap_or(false),
                user_id,
            )
            .await
        {
            Ok((count, messages)) => Ok(FileOperationResult {
                success: count > 0,
                error: if count == 0 {
                    Some("No files were copied".to_string())
                } else {
                    None
                },
                affected_count: count,
                messages,
                path: None,
            }),
            Err(e) => Ok(FileOperationResult {
                success: false,
                error: Some(e.to_string()),
                affected_count: 0,
                messages: vec![],
                path: None,
            }),
        }
    }

    /// Move files or directories (source and destination must be inside libraries)
    async fn move_files(
        &self,
        ctx: &Context<'_>,
        input: MoveFilesInput,
    ) -> Result<FileOperationResult> {
        let user = ctx.auth_user()?;
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match fs_service
            .move_files(
                &input.sources,
                &input.destination,
                input.overwrite.unwrap_or(false),
                user_id,
            )
            .await
        {
            Ok((count, messages)) => Ok(FileOperationResult {
                success: count > 0,
                error: if count == 0 {
                    Some("No files were moved".to_string())
                } else {
                    None
                },
                affected_count: count,
                messages,
                path: None,
            }),
            Err(e) => Ok(FileOperationResult {
                success: false,
                error: Some(e.to_string()),
                affected_count: 0,
                messages: vec![],
                path: None,
            }),
        }
    }

    /// Rename a file or directory (must be inside a library)
    async fn rename_file(
        &self,
        ctx: &Context<'_>,
        input: RenameFileInput,
    ) -> Result<FileOperationResult> {
        let user = ctx.auth_user()?;
        let fs_service = ctx.data_unchecked::<Arc<FilesystemService>>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match fs_service
            .rename_file(&input.path, &input.new_name, user_id)
            .await
        {
            Ok(new_path) => Ok(FileOperationResult {
                success: true,
                error: None,
                affected_count: 1,
                messages: vec![format!("Renamed to: {}", new_path)],
                path: Some(new_path),
            }),
            Err(e) => Ok(FileOperationResult {
                success: false,
                error: Some(e.to_string()),
                affected_count: 0,
                messages: vec![],
                path: None,
            }),
        }
    }
}
