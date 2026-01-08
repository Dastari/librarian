//! GraphQL schema definition with queries, mutations, and subscriptions
//!
//! This is the single API surface for the Librarian backend.
//! All operations require authentication unless explicitly noted.

use std::sync::Arc;

use async_graphql::{Context, Object, Result, Schema};
use uuid::Uuid;

use crate::db::{
    CreateLibrary, CreateQualityProfile, CreateRssFeed, Database, LibraryStats, UpdateLibrary,
    UpdateQualityProfile, UpdateRssFeed, UpdateTvShow,
};
use crate::services::{MetadataService, ScannerService, TorrentService};

use super::auth::AuthExt;
use super::subscriptions::SubscriptionRoot;
use super::types::*;

/// The GraphQL schema type
pub type LibrarianSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

/// Build the GraphQL schema with all resolvers
pub fn build_schema(
    torrent_service: Arc<TorrentService>,
    metadata_service: Arc<MetadataService>,
    scanner_service: Arc<ScannerService>,
    db: Database,
) -> LibrarianSchema {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(torrent_service)
        .data(metadata_service)
        .data(scanner_service)
        .data(db)
        .finish()
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
                Ok(s) => {
                    tracing::info!(
                        library_id = %r.id,
                        file_count = ?s.file_count,
                        total_size = ?s.total_size_bytes,
                        show_count = ?s.show_count,
                        "Library stats retrieved"
                    );
                    s
                }
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
                auto_rename: r.auto_rename,
                naming_pattern: r.naming_pattern,
                default_quality_profile_id: r.default_quality_profile_id.map(|id| id.to_string()),
                auto_add_discovered: r.auto_add_discovered,
                item_count: stats.file_count.unwrap_or(0) as i32,
                total_size_bytes: stats.total_size_bytes.unwrap_or(0),
                show_count: stats.show_count.unwrap_or(0) as i32,
                last_scanned_at: r.last_scanned_at.map(|t| t.to_rfc3339()),
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
                auto_rename: r.auto_rename,
                naming_pattern: r.naming_pattern,
                default_quality_profile_id: r.default_quality_profile_id.map(|id| id.to_string()),
                auto_add_discovered: r.auto_add_discovered,
                item_count: stats.file_count.unwrap_or(0) as i32,
                total_size_bytes: stats.total_size_bytes.unwrap_or(0),
                show_count: stats.show_count.unwrap_or(0) as i32,
                last_scanned_at: r.last_scanned_at.map(|t| t.to_rfc3339()),
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
                episode_count: r.episode_count.unwrap_or(0),
                episode_file_count: r.episode_file_count.unwrap_or(0),
                size_bytes: r.size_bytes.unwrap_or(0),
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
            episode_count: r.episode_count.unwrap_or(0),
            episode_file_count: r.episode_file_count.unwrap_or(0),
            size_bytes: r.size_bytes.unwrap_or(0),
        }))
    }

    /// Search for TV shows from metadata providers
    async fn search_tv_shows(&self, ctx: &Context<'_>, query: String) -> Result<Vec<TvShowSearchResult>> {
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
                    "downloading" => EpisodeStatus::Downloading,
                    "downloaded" => EpisodeStatus::Downloaded,
                    "ignored" => EpisodeStatus::Ignored,
                    _ => EpisodeStatus::Missing,
                },
                tvmaze_id: r.tvmaze_id,
                tmdb_id: r.tmdb_id,
                tvdb_id: r.tvdb_id,
            })
            .collect())
    }

    /// Get wanted (missing) episodes
    async fn wanted_episodes(&self, ctx: &Context<'_>, library_id: Option<String>) -> Result<Vec<Episode>> {
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
                status: EpisodeStatus::Wanted,
                tvmaze_id: r.tvmaze_id,
                tmdb_id: r.tmdb_id,
                tvdb_id: r.tvdb_id,
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
    async fn quality_profile(&self, ctx: &Context<'_>, id: String) -> Result<Option<QualityProfile>> {
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
    async fn rss_feeds(&self, ctx: &Context<'_>, library_id: Option<String>) -> Result<Vec<RssFeed>> {
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
    // Torrents
    // ------------------------------------------------------------------------

    /// Get all torrents
    async fn torrents(&self, ctx: &Context<'_>) -> Result<Vec<Torrent>> {
        let _user = ctx.auth_user()?;
        let service = ctx.data_unchecked::<Arc<TorrentService>>();
        let torrents = service.list_torrents().await;
        Ok(torrents.into_iter().map(|t| t.into()).collect())
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
        input: CreateLibraryFullInput,
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
                auto_rename: input.auto_rename.unwrap_or(true),
                naming_pattern: input.naming_pattern,
                default_quality_profile_id: input
                    .default_quality_profile_id
                    .map(|id| Uuid::parse_str(&id).ok())
                    .flatten(),
                auto_add_discovered: input.auto_add_discovered.unwrap_or(true),
            })
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

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
        input: UpdateLibraryFullInput,
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
                    auto_rename: input.auto_rename,
                    naming_pattern: input.naming_pattern,
                    default_quality_profile_id: input
                        .default_quality_profile_id
                        .map(|id| Uuid::parse_str(&id).ok())
                        .flatten(),
                    auto_add_discovered: input.auto_add_discovered,
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

        let library_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        tracing::info!("Scan requested for library: {}", id);

        // Spawn the scan in the background so the mutation returns immediately
        let scanner = scanner.clone();
        tokio::spawn(async move {
            if let Err(e) = scanner.scan_library(library_id).await {
                tracing::error!(library_id = %library_id, error = %e, "Library scan failed");
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
                episode_count: record.episode_count.unwrap_or(0),
                episode_file_count: record.episode_file_count.unwrap_or(0),
                size_bytes: record.size_bytes.unwrap_or(0),
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
                        .map(|id| Uuid::parse_str(&id).ok())
                        .flatten(),
                    path: input.path,
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
                    episode_count: record.episode_count.unwrap_or(0),
                    episode_file_count: record.episode_file_count.unwrap_or(0),
                    size_bytes: record.size_bytes.unwrap_or(0),
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
                    air_date: ep.air_date.and_then(|d| {
                        chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok()
                    }),
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
                episode_count: record.episode_count.unwrap_or(0),
                episode_file_count: record.episode_file_count.unwrap_or(0),
                size_bytes: record.size_bytes.unwrap_or(0),
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
                    .map(|f| rust_decimal::Decimal::try_from(f).ok())
                    .flatten(),
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
                        .map(|f| rust_decimal::Decimal::try_from(f).ok())
                        .flatten(),
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
    async fn delete_quality_profile(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
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
                library_id: input
                    .library_id
                    .map(|id| Uuid::parse_str(&id).ok())
                    .flatten(),
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
                    library_id: input
                        .library_id
                        .map(|id| Uuid::parse_str(&id).ok())
                        .flatten(),
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

    // ------------------------------------------------------------------------
    // Media
    // ------------------------------------------------------------------------

    /// Get a cast session for Chromecast/AirPlay
    async fn create_cast_session(&self, ctx: &Context<'_>, media_id: String) -> Result<CastSession> {
        let _user = ctx.auth_user()?;
        let _id = media_id;
        // TODO: Generate cast session
        Ok(CastSession {
            session_token: Uuid::new_v4().to_string(),
            stream_url: String::new(),
            expires_at: String::new(),
        })
    }

    // ------------------------------------------------------------------------
    // Torrents
    // ------------------------------------------------------------------------

    /// Add a new torrent from magnet link or URL to a .torrent file
    async fn add_torrent(&self, ctx: &Context<'_>, input: AddTorrentInput) -> Result<AddTorrentResult> {
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
            Ok(info) => Ok(AddTorrentResult {
                success: true,
                torrent: Some(info.into()),
                error: None,
            }),
            Err(e) => Ok(AddTorrentResult {
                success: false,
                torrent: None,
                error: Some(e.to_string()),
            }),
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
    async fn search_subscription(&self, ctx: &Context<'_>, id: String) -> Result<Vec<SearchResult>> {
        let _user = ctx.auth_user()?;
        let _id = id;
        // TODO: Implement Torznab search
        Ok(vec![])
    }
}
