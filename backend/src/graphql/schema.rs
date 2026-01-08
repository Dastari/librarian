//! GraphQL schema definition with queries, mutations, and subscriptions
//!
//! This is the single API surface for the Librarian backend.
//! All operations require authentication unless explicitly noted.

use std::sync::Arc;

use async_graphql::{Context, Object, Result, Schema};
use uuid::Uuid;

use crate::db::Database;
use crate::services::TorrentService;

use super::auth::AuthExt;
use super::subscriptions::SubscriptionRoot;
use super::types::*;

/// The GraphQL schema type
pub type LibrarianSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

/// Build the GraphQL schema with all resolvers
pub fn build_schema(torrent_service: Arc<TorrentService>, db: Database) -> LibrarianSchema {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(torrent_service)
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
    async fn libraries(&self, ctx: &Context<'_>) -> Result<Vec<Library>> {
        let _user = ctx.auth_user()?;
        // TODO: Query from database
        Ok(vec![
            Library {
                id: Uuid::new_v4().to_string(),
                name: "Movies".to_string(),
                path: "/data/media/Movies".to_string(),
                library_type: LibraryType::Movies,
                icon: "film".to_string(),
                color: "purple".to_string(),
                auto_scan: true,
                scan_interval_hours: 24,
                item_count: 0,
                total_size_bytes: 0,
                last_scanned_at: None,
            },
            Library {
                id: Uuid::new_v4().to_string(),
                name: "TV Shows".to_string(),
                path: "/data/media/TV".to_string(),
                library_type: LibraryType::Tv,
                icon: "tv".to_string(),
                color: "blue".to_string(),
                auto_scan: true,
                scan_interval_hours: 6,
                item_count: 0,
                total_size_bytes: 0,
                last_scanned_at: None,
            },
        ])
    }

    /// Get a specific library by ID
    async fn library(&self, ctx: &Context<'_>, id: String) -> Result<Option<Library>> {
        let _user = ctx.auth_user()?;
        // TODO: Query from database
        Ok(Some(Library {
            id,
            name: "Movies".to_string(),
            path: "/data/media/Movies".to_string(),
            library_type: LibraryType::Movies,
            icon: "film".to_string(),
            color: "purple".to_string(),
            auto_scan: true,
            scan_interval_hours: 24,
            item_count: 0,
            total_size_bytes: 0,
            last_scanned_at: None,
        }))
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
    // Subscriptions (TV show monitoring)
    // ------------------------------------------------------------------------

    /// Get all subscriptions
    async fn subscriptions(&self, ctx: &Context<'_>) -> Result<Vec<Subscription>> {
        let _user = ctx.auth_user()?;
        // TODO: Query from database
        Ok(vec![])
    }

    /// Get a specific subscription by ID
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
            download_dir: settings.get_or_default("torrent.download_dir", "/data/downloads".to_string()).await.unwrap_or_default(),
            session_dir: settings.get_or_default("torrent.session_dir", "/data/session".to_string()).await.unwrap_or_default(),
            enable_dht: settings.get_or_default("torrent.enable_dht", true).await.unwrap_or(true),
            listen_port: settings.get_or_default("torrent.listen_port", 6881).await.unwrap_or(6881),
            max_concurrent: settings.get_or_default("torrent.max_concurrent", 5).await.unwrap_or(5),
            upload_limit: settings.get_or_default("torrent.upload_limit", 0i64).await.unwrap_or(0),
            download_limit: settings.get_or_default("torrent.download_limit", 0i64).await.unwrap_or(0),
        })
    }

    /// Get all settings in a category
    async fn settings_by_category(&self, ctx: &Context<'_>, category: String) -> Result<Vec<AppSetting>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        let records = settings.list_by_category(&category).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(|r| AppSetting {
            key: r.key,
            value: r.value,
            description: r.description,
            category: r.category,
        }).collect())
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
        let _user = ctx.auth_user()?;

        let library = Library {
            id: Uuid::new_v4().to_string(),
            name: input.name,
            path: input.path,
            library_type: input.library_type,
            icon: input.icon.unwrap_or_else(|| match input.library_type {
                LibraryType::Movies => "film",
                LibraryType::Tv => "tv",
                LibraryType::Music => "music",
                LibraryType::Audiobooks => "headphones",
                LibraryType::Other => "folder",
            }.to_string()),
            color: input.color.unwrap_or_else(|| match input.library_type {
                LibraryType::Movies => "purple",
                LibraryType::Tv => "blue",
                LibraryType::Music => "green",
                LibraryType::Audiobooks => "orange",
                LibraryType::Other => "slate",
            }.to_string()),
            auto_scan: input.auto_scan.unwrap_or(true),
            scan_interval_hours: input.scan_interval_hours.unwrap_or(24),
            item_count: 0,
            total_size_bytes: 0,
            last_scanned_at: None,
        };

        // TODO: Insert into database

        Ok(LibraryResult {
            success: true,
            library: Some(library),
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
        let _id = id;
        let _input = input;
        // TODO: Update in database
        Ok(LibraryResult {
            success: false,
            library: None,
            error: Some("Not implemented".to_string()),
        })
    }

    /// Delete a library
    async fn delete_library(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let _id = id;
        // TODO: Delete from database
        Ok(MutationResult {
            success: true,
            error: None,
        })
    }

    /// Trigger a library scan
    async fn scan_library(&self, ctx: &Context<'_>, id: String) -> Result<ScanStatus> {
        let _user = ctx.auth_user()?;
        tracing::info!("Scan requested for library: {}", id);
        // TODO: Enqueue scan job
        Ok(ScanStatus {
            library_id: id,
            status: "queued".to_string(),
            message: Some("Scan has been queued".to_string()),
        })
    }

    // ------------------------------------------------------------------------
    // Media
    // ------------------------------------------------------------------------

    /// Get a cast session for Chromecast/AirPlay
    async fn create_cast_session(
        &self,
        ctx: &Context<'_>,
        media_id: String,
    ) -> Result<CastSession> {
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
            settings.set_with_category("torrent.download_dir", v, "torrent", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.session_dir {
            settings.set_with_category("torrent.session_dir", v, "torrent", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.enable_dht {
            settings.set_with_category("torrent.enable_dht", v, "torrent", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.listen_port {
            settings.set_with_category("torrent.listen_port", v, "torrent", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.max_concurrent {
            settings.set_with_category("torrent.max_concurrent", v, "torrent", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.upload_limit {
            settings.set_with_category("torrent.upload_limit", v, "torrent", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.download_limit {
            settings.set_with_category("torrent.download_limit", v, "torrent", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }

        Ok(SettingsResult {
            success: true,
            error: None,
        })
    }

    // ------------------------------------------------------------------------
    // Subscriptions (TV show monitoring)
    // ------------------------------------------------------------------------

    /// Create a new subscription
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

    /// Update an existing subscription
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

    /// Delete a subscription
    async fn delete_subscription(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let _id = id;
        // TODO: Delete from database
        Ok(MutationResult {
            success: true,
            error: None,
        })
    }

    /// Manually trigger a search for a subscription
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
}
