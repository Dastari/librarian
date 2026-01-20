//! GraphQL schema definition with queries, mutations, and subscriptions
//!
//! This is the single API surface for the Librarian backend.
//! All operations require authentication unless explicitly noted.

use std::collections::HashMap;
use std::sync::Arc;

use async_graphql::{Context, Object, Result, Schema};
use uuid::Uuid;

use crate::db::{
    CreateLibrary, CreateRssFeed, Database, LibraryStats, LogFilter,
    UpdateLibrary, UpdateRssFeed, UpdateTvShow,
};
use crate::services::{
    CastService, FilesystemService, LogEvent, MetadataService, ScannerService, TorrentService,
};

use super::auth::AuthExt;
use super::pagination::{Connection, parse_pagination_args};
use super::subscriptions::SubscriptionRoot;
use super::types::{
    AddTorrentInput,
    AddTorrentResult,
    AddTvShowInput,
    AnalyzeMediaFileResult,
    AutoHuntResult,
    AppSetting,
    AudioStreamInfo,
    ChapterInfo,
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
    // Playback types
    PlaybackContentType,
    PlaybackSession,
    PlaybackResult,
    PlaybackSettings,
    StartPlaybackInput,
    UpdatePlaybackInput,
    UpdatePlaybackSettingsInput,
    // Filesystem types
    BrowseDirectoryInput,
    BrowseDirectoryResult,
    CopyFilesInput,
    CreateDirectoryInput,
    DeleteFilesInput,
    FileEntry,
    FileOperationResult,
    MoveFilesInput,
    PathValidationResult,
    QuickPath,
    RenameFileInput,
    // Indexer types
    CreateIndexerInput,
    CreateLibraryInput,
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
    LibraryChangedEvent,
    LibraryChangeType,
    LibraryFull,
    LibraryResult,
    LibraryType,
    LibraryUpcomingEpisode,
    LibraryUpcomingShow,
    LogEntry,
    LogFilterInput,
    LogLevel,
    LogOrderByInput,
    LogSortField,
    LogStats,
    MediaFile,
    MediaFileUpdatedEvent,
    MediaItem,
    MonitorType,
    // Movie types
    Movie,
    MovieConnection,
    MovieOrderByInput,
    MovieResult,
    MovieSearchResult,
    MovieSortField,
    MovieStatus,
    MovieWhereInput,
    AddMovieInput,
    UpdateMovieInput,
    // TV Show connection types
    TvShowConnection,
    TvShowOrderByInput,
    TvShowSortField,
    TvShowWhereInput,
    // Album connection types
    AlbumConnection,
    AlbumOrderByInput,
    AlbumSortField,
    AlbumWhereInput,
    // Artist connection types
    ArtistConnection,
    ArtistOrderByInput,
    ArtistSortField,
    ArtistWhereInput,
    MutationResult,
    // Album/Music types
    Album,
    AlbumResult,
    AlbumSearchResult,
    AlbumWithTracks,
    AddAlbumInput,
    Artist,
    Track,
    // Audiobook types
    Audiobook,
    AudiobookResult,
    AudiobookSearchResult,
    AddAudiobookInput,
    AudiobookAuthor,
    // Audiobook connection types
    AudiobookConnection,
    AudiobookOrderByInput,
    AudiobookSortField,
    AudiobookWhereInput,
    AudiobookAuthorConnection,
    AudiobookAuthorOrderByInput,
    AudiobookAuthorSortField,
    AudiobookAuthorWhereInput,
    // Audiobook chapter connection types
    AudiobookChapter,
    AudiobookChapterConnection,
    AudiobookChapterOrderByInput,
    AudiobookChapterSortField,
    AudiobookChapterWhereInput,
    // Naming pattern types
    CreateNamingPatternInput,
    NamingPattern,
    NamingPatternResult,
    OrganizeTorrentResult,
    PaginatedLogResult,
    ParseAndIdentifyMediaResult,
    ParsedEpisodeInfo,
    PostDownloadAction,
    RssFeed,
    RssFeedResult,
    RssFeedTestResult,
    RssItem,
    ScanStatus,
    ConsolidateLibraryResult,
    SearchResult,
    // Subtitle types
    MediaFileDetails,
    Subtitle,
    SubtitleSettings,
    SubtitleSettingsInput,
    VideoStreamInfo,
    // Security types
    SecuritySettings,
    SecuritySettingsResult,
    SettingsResult,
    // LLM Parser types
    LlmParserSettings,
    UpdateLlmParserSettingsInput,
    OllamaConnectionResult,
    FilenameParseResult,
    TestFilenameParserResult,
    // Schedule cache
    RefreshScheduleResult,
    StreamInfo,
    Subscription,
    SubscriptionResult,
    Torrent,
    TorrentActionResult,
    TorrentDetails,
    TorrentFileMatch,
    TorrentRelease,
    DownloadStatus,
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
    UpdateRssFeedInput,
    UpdateSubscriptionInput,
    UpdateTorrentSettingsInput,
    UpdateTvShowInput,
    User,
    UserPreferences,
    // Helpers
    format_bytes,
};

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a MovieRecord from the database to a GraphQL Movie type
fn movie_record_to_graphql(r: crate::db::MovieRecord) -> Movie {
    Movie {
        id: r.id.to_string(),
        library_id: r.library_id.to_string(),
        title: r.title,
        sort_title: r.sort_title,
        original_title: r.original_title,
        year: r.year,
        tmdb_id: r.tmdb_id,
        imdb_id: r.imdb_id,
        status: r
            .status
            .as_deref()
            .map(MovieStatus::from)
            .unwrap_or_default(),
        overview: r.overview,
        tagline: r.tagline,
        runtime: r.runtime,
        genres: r.genres,
        director: r.director,
        cast_names: r.cast_names,
        poster_url: r.poster_url,
        backdrop_url: r.backdrop_url,
        monitored: r.monitored,
        has_file: r.has_file,
        size_bytes: r.size_bytes.unwrap_or(0),
        path: r.path,
        download_status: DownloadStatus::from(r.download_status.as_str()),
        collection_id: r.collection_id,
        collection_name: r.collection_name,
        collection_poster_url: r.collection_poster_url,
        tmdb_rating: r.tmdb_rating.and_then(|d| d.to_string().parse::<f64>().ok()),
        tmdb_vote_count: r.tmdb_vote_count,
        certification: r.certification,
        release_date: r.release_date.map(|d| d.to_string()),
        allowed_resolutions_override: r.allowed_resolutions_override,
        allowed_video_codecs_override: r.allowed_video_codecs_override,
        allowed_audio_formats_override: r.allowed_audio_formats_override,
        require_hdr_override: r.require_hdr_override,
        allowed_hdr_types_override: r.allowed_hdr_types_override,
        allowed_sources_override: r.allowed_sources_override,
        release_group_blacklist_override: r.release_group_blacklist_override,
        release_group_whitelist_override: r.release_group_whitelist_override,
    }
}

/// Convert MovieSortField enum to database column name
fn sort_field_to_column(field: MovieSortField) -> String {
    match field {
        MovieSortField::Title => "title".to_string(),
        MovieSortField::SortTitle => "sort_title".to_string(),
        MovieSortField::Year => "year".to_string(),
        MovieSortField::CreatedAt => "created_at".to_string(),
        MovieSortField::ReleaseDate => "release_date".to_string(),
    }
}

/// Convert TvShowSortField enum to database column name
fn tv_sort_field_to_column(field: TvShowSortField) -> String {
    match field {
        TvShowSortField::Name => "name".to_string(),
        TvShowSortField::SortName => "sort_name".to_string(),
        TvShowSortField::Year => "year".to_string(),
        TvShowSortField::CreatedAt => "created_at".to_string(),
    }
}

/// Convert AlbumSortField enum to database column name
fn album_sort_field_to_column(field: AlbumSortField) -> String {
    match field {
        AlbumSortField::Name => "name".to_string(),
        AlbumSortField::SortName => "sort_name".to_string(),
        AlbumSortField::Year => "year".to_string(),
        AlbumSortField::CreatedAt => "created_at".to_string(),
        AlbumSortField::Artist => "artist_id".to_string(),
    }
}

/// Convert ArtistSortField enum to database column name
fn artist_sort_field_to_column(field: ArtistSortField) -> String {
    match field {
        ArtistSortField::Name => "name".to_string(),
        ArtistSortField::SortName => "sort_name".to_string(),
    }
}

/// Convert AudiobookSortField enum to database column name
fn audiobook_sort_field_to_column(field: AudiobookSortField) -> String {
    match field {
        AudiobookSortField::Title => "title".to_string(),
        AudiobookSortField::SortTitle => "sort_title".to_string(),
        AudiobookSortField::CreatedAt => "created_at".to_string(),
    }
}

/// Convert AudiobookAuthorSortField enum to database column name
fn audiobook_author_sort_field_to_column(field: AudiobookAuthorSortField) -> String {
    match field {
        AudiobookAuthorSortField::Name => "name".to_string(),
        AudiobookAuthorSortField::SortName => "sort_name".to_string(),
    }
}

/// Convert AudiobookChapterSortField enum to database column name
fn audiobook_chapter_sort_field_to_column(field: AudiobookChapterSortField) -> String {
    match field {
        AudiobookChapterSortField::ChapterNumber => "chapter_number".to_string(),
        AudiobookChapterSortField::Title => "title".to_string(),
        AudiobookChapterSortField::CreatedAt => "created_at".to_string(),
    }
}

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
    analysis_queue: Arc<crate::services::MediaAnalysisQueue>,
    log_broadcast: Option<tokio::sync::broadcast::Sender<LogEvent>>,
    library_broadcast: Option<tokio::sync::broadcast::Sender<LibraryChangedEvent>>,
    media_file_broadcast: Option<tokio::sync::broadcast::Sender<MediaFileUpdatedEvent>>,
) -> LibrarianSchema {
    // Create library events broadcast channel (use provided or create new)
    let library_tx = library_broadcast
        .unwrap_or_else(|| tokio::sync::broadcast::channel::<LibraryChangedEvent>(100).0);

    // Create media file updated broadcast channel (use provided or create new)
    let media_file_tx = media_file_broadcast
        .unwrap_or_else(|| tokio::sync::broadcast::channel::<MediaFileUpdatedEvent>(100).0);

    let mut schema = Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .data(torrent_service)
        .data(metadata_service)
        .data(scanner_service)
        .data(cast_service)
        .data(filesystem_service)
        .data(db)
        .data(analysis_queue)
        .data(library_tx)
        .data(media_file_tx);

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

            libraries.push(LibraryFull::from_record_with_stats(r, stats));
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
            Ok(Some(LibraryFull::from_record_with_stats(r, stats)))
        } else {
            Ok(None)
        }
    }

    // ------------------------------------------------------------------------
    // TV Shows
    // ------------------------------------------------------------------------

    /// Get all TV shows for the current user (across all libraries)
    async fn all_tv_shows(&self, ctx: &Context<'_>) -> Result<Vec<TvShow>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .tv_shows()
            .list_by_user(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(TvShow::from).collect())
    }

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

        Ok(records.into_iter().map(TvShow::from).collect())
    }

    /// Get TV shows in a library with cursor-based pagination and filtering
    async fn tv_shows_connection(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<TvShowWhereInput>,
        order_by: Option<TvShowOrderByInput>,
    ) -> Result<TvShowConnection> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let (offset, limit) = parse_pagination_args(first, after)
            .map_err(|e| async_graphql::Error::new(e))?;

        // Build filter conditions
        let name_filter = r#where.as_ref().and_then(|w| {
            w.name.as_ref().and_then(|f| f.contains.clone())
        });
        let year_filter = r#where.as_ref().and_then(|w| {
            w.year.as_ref().and_then(|f| f.eq)
        });
        let monitored_filter = r#where.as_ref().and_then(|w| {
            w.monitored.as_ref().and_then(|f| f.eq)
        });
        let status_filter = r#where.as_ref().and_then(|w| {
            w.status.as_ref().and_then(|f| f.eq.clone())
        });

        let sort_field = order_by
            .as_ref()
            .and_then(|o| o.field)
            .unwrap_or(TvShowSortField::SortName);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(super::filters::OrderDirection::Asc);

        let (records, total) = db
            .tv_shows()
            .list_by_library_paginated(
                lib_id,
                offset,
                limit,
                name_filter.as_deref(),
                year_filter,
                monitored_filter,
                status_filter.as_deref(),
                &tv_sort_field_to_column(sort_field),
                sort_dir == super::filters::OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let shows: Vec<TvShow> = records.into_iter().map(TvShow::from).collect();
        let connection = Connection::from_items(shows, offset, limit, total);
        
        Ok(TvShowConnection::from_connection(connection))
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

        Ok(record.map(TvShow::from))
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
    // Movies
    // ------------------------------------------------------------------------

    /// Get all movies for the current user (across all libraries)
    async fn all_movies(&self, ctx: &Context<'_>) -> Result<Vec<Movie>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .movies()
            .list_by_user(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(movie_record_to_graphql).collect())
    }

    /// Get all movies in a library
    async fn movies(&self, ctx: &Context<'_>, library_id: String) -> Result<Vec<Movie>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let records = db
            .movies()
            .list_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(movie_record_to_graphql).collect())
    }

    /// Get movies in a library with cursor-based pagination and filtering
    async fn movies_connection(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<MovieWhereInput>,
        order_by: Option<MovieOrderByInput>,
    ) -> Result<MovieConnection> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        // Parse pagination args
        let (offset, limit) = parse_pagination_args(first, after)
            .map_err(|e| async_graphql::Error::new(e))?;

        // Build filter conditions
        let title_filter = r#where.as_ref().and_then(|w| {
            w.title.as_ref().and_then(|f| f.contains.clone())
        });
        let year_filter = r#where.as_ref().and_then(|w| {
            w.year.as_ref().and_then(|f| f.eq)
        });
        let monitored_filter = r#where.as_ref().and_then(|w| {
            w.monitored.as_ref().and_then(|f| f.eq)
        });
        let has_file_filter = r#where.as_ref().and_then(|w| {
            w.has_file.as_ref().and_then(|f| f.eq)
        });
        let download_status_filter = r#where.as_ref().and_then(|w| {
            w.download_status.as_ref().and_then(|f| f.eq.clone())
        });

        // Determine sort field and direction
        let sort_field = order_by
            .as_ref()
            .and_then(|o| o.field)
            .unwrap_or(MovieSortField::SortTitle);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(super::filters::OrderDirection::Asc);

        // Get paginated movies from database
        let (records, total) = db
            .movies()
            .list_by_library_paginated(
                lib_id,
                offset,
                limit,
                title_filter.as_deref(),
                year_filter,
                monitored_filter,
                has_file_filter,
                download_status_filter.as_deref(),
                &sort_field_to_column(sort_field),
                sort_dir == super::filters::OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let movies: Vec<Movie> = records.into_iter().map(movie_record_to_graphql).collect();
        let connection = Connection::from_items(movies, offset, limit, total);
        
        Ok(MovieConnection::from_connection(connection))
    }

    /// Get a specific movie by ID
    async fn movie(&self, ctx: &Context<'_>, id: String) -> Result<Option<Movie>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let movie_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid movie ID: {}", e)))?;

        let record = db
            .movies()
            .get_by_id(movie_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(record.map(movie_record_to_graphql))
    }

    /// Search for movies on TMDB
    async fn search_movies(
        &self,
        ctx: &Context<'_>,
        query: String,
        year: Option<i32>,
    ) -> Result<Vec<MovieSearchResult>> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        if !metadata.has_tmdb() {
            return Err(async_graphql::Error::new(
                "TMDB API key not configured. Add tmdb_api_key to settings.",
            ));
        }

        let results = metadata
            .search_movies(&query, year)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|m| MovieSearchResult {
                provider: "tmdb".to_string(),
                provider_id: m.provider_id as i32,
                title: m.title,
                original_title: m.original_title,
                year: m.year,
                overview: m.overview,
                poster_url: m.poster_url,
                backdrop_url: m.backdrop_url,
                imdb_id: m.imdb_id,
                vote_average: m.vote_average,
                popularity: m.popularity,
            })
            .collect())
    }

    // ------------------------------------------------------------------------
    // Albums/Music
    // ------------------------------------------------------------------------

    /// Get all albums in a library
    async fn albums(&self, ctx: &Context<'_>, library_id: String) -> Result<Vec<Album>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let records = db
            .albums()
            .list_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(Album::from).collect())
    }

    /// Get albums in a library with cursor-based pagination and filtering
    async fn albums_connection(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<AlbumWhereInput>,
        order_by: Option<AlbumOrderByInput>,
    ) -> Result<AlbumConnection> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let (offset, limit) = parse_pagination_args(first, after)
            .map_err(|e| async_graphql::Error::new(e))?;

        let name_filter = r#where.as_ref().and_then(|w| {
            w.name.as_ref().and_then(|f| f.contains.clone())
        });
        let year_filter = r#where.as_ref().and_then(|w| {
            w.year.as_ref().and_then(|f| f.eq)
        });
        let has_files_filter = r#where.as_ref().and_then(|w| {
            w.has_files.as_ref().and_then(|f| f.eq)
        });

        let sort_field = order_by
            .as_ref()
            .and_then(|o| o.field)
            .unwrap_or(AlbumSortField::Name);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(super::filters::OrderDirection::Asc);

        let (records, total) = db
            .albums()
            .list_by_library_paginated(
                lib_id,
                offset,
                limit,
                name_filter.as_deref(),
                year_filter,
                has_files_filter,
                &album_sort_field_to_column(sort_field),
                sort_dir == super::filters::OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let albums: Vec<Album> = records.into_iter().map(Album::from).collect();
        let connection = Connection::from_items(albums, offset, limit, total);
        
        Ok(AlbumConnection::from_connection(connection))
    }

    /// Get a specific album by ID
    async fn album(&self, ctx: &Context<'_>, id: String) -> Result<Option<Album>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let album_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid album ID: {}", e)))?;

        let record = db
            .albums()
            .get_by_id(album_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(record.map(Album::from))
    }

    /// Get all artists in a library
    async fn artists(&self, ctx: &Context<'_>, library_id: String) -> Result<Vec<Artist>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let records = db
            .albums()
            .list_artists_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(Artist::from).collect())
    }

    /// Get artists in a library with cursor-based pagination and filtering
    async fn artists_connection(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<ArtistWhereInput>,
        order_by: Option<ArtistOrderByInput>,
    ) -> Result<ArtistConnection> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let (offset, limit) = parse_pagination_args(first, after)
            .map_err(|e| async_graphql::Error::new(e))?;

        let name_filter = r#where.as_ref().and_then(|w| {
            w.name.as_ref().and_then(|f| f.contains.clone())
        });

        let sort_field = order_by
            .as_ref()
            .and_then(|o| o.field)
            .unwrap_or(ArtistSortField::Name);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(super::filters::OrderDirection::Asc);

        let (records, total) = db
            .albums()
            .list_artists_by_library_paginated(
                lib_id,
                offset,
                limit,
                name_filter.as_deref(),
                &artist_sort_field_to_column(sort_field),
                sort_dir == super::filters::OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let artists: Vec<Artist> = records.into_iter().map(Artist::from).collect();
        let connection = Connection::from_items(artists, offset, limit, total);
        
        Ok(ArtistConnection::from_connection(connection))
    }

    /// Get an album with all its tracks and file status
    async fn album_with_tracks(&self, ctx: &Context<'_>, id: String) -> Result<Option<AlbumWithTracks>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let album_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid album ID: {}", e)))?;

        let album_record = db
            .albums()
            .get_by_id(album_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let Some(album_record) = album_record else {
            return Ok(None);
        };

        let tracks_with_status = db
            .tracks()
            .list_with_status(album_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let track_count = tracks_with_status.len() as i32;
        let tracks_with_files = tracks_with_status.iter().filter(|t| t.has_file).count() as i32;
        let missing_tracks = track_count - tracks_with_files;
        let completion_percent = if track_count > 0 {
            (tracks_with_files as f64 / track_count as f64) * 100.0
        } else {
            0.0
        };

        Ok(Some(AlbumWithTracks {
            album: album_record.into(),
            tracks: tracks_with_status.into_iter().map(|t| t.into()).collect(),
            track_count,
            tracks_with_files,
            missing_tracks,
            completion_percent,
        }))
    }

    /// Get tracks for an album
    async fn tracks(&self, ctx: &Context<'_>, album_id: String) -> Result<Vec<Track>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let album_uuid = Uuid::parse_str(&album_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid album ID: {}", e)))?;

        let records = db
            .tracks()
            .list_by_album(album_uuid)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(Track::from).collect())
    }

    /// Search for albums on MusicBrainz
    async fn search_albums(&self, ctx: &Context<'_>, query: String) -> Result<Vec<AlbumSearchResult>> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let results = metadata
            .search_albums(&query)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|a| AlbumSearchResult {
                provider: "musicbrainz".to_string(),
                provider_id: a.provider_id.to_string(),
                title: a.title,
                artist_name: a.artist_name,
                year: a.year,
                album_type: a.album_type,
                cover_url: a.cover_url,
                score: a.score,
            })
            .collect())
    }

    // ------------------------------------------------------------------------
    // Audiobooks
    // ------------------------------------------------------------------------

    /// Get all audiobooks in a library
    async fn audiobooks(&self, ctx: &Context<'_>, library_id: String) -> Result<Vec<Audiobook>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let records = db
            .audiobooks()
            .list_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(Audiobook::from).collect())
    }

    /// Get audiobooks in a library with cursor-based pagination and filtering
    async fn audiobooks_connection(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<AudiobookWhereInput>,
        order_by: Option<AudiobookOrderByInput>,
    ) -> Result<AudiobookConnection> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let (offset, limit) = parse_pagination_args(first, after)
            .map_err(|e| async_graphql::Error::new(e))?;

        let title_filter = r#where.as_ref().and_then(|w| {
            w.title.as_ref().and_then(|f| f.contains.clone())
        });
        let has_files_filter = r#where.as_ref().and_then(|w| {
            w.has_files.as_ref().and_then(|f| f.eq)
        });

        let sort_field = order_by
            .as_ref()
            .and_then(|o| o.field)
            .unwrap_or(AudiobookSortField::Title);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(super::filters::OrderDirection::Asc);

        let (records, total) = db
            .audiobooks()
            .list_by_library_paginated(
                lib_id,
                offset,
                limit,
                title_filter.as_deref(),
                has_files_filter,
                &audiobook_sort_field_to_column(sort_field),
                sort_dir == super::filters::OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let audiobooks: Vec<Audiobook> = records.into_iter().map(Audiobook::from).collect();
        let connection = Connection::from_items(audiobooks, offset, limit, total);
        
        Ok(AudiobookConnection::from_connection(connection))
    }

    /// Get a specific audiobook by ID
    async fn audiobook(&self, ctx: &Context<'_>, id: String) -> Result<Option<Audiobook>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let audiobook_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid audiobook ID: {}", e)))?;

        let record = db
            .audiobooks()
            .get_by_id(audiobook_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(record.map(Audiobook::from))
    }

    /// Get all audiobook authors in a library
    async fn audiobook_authors(&self, ctx: &Context<'_>, library_id: String) -> Result<Vec<AudiobookAuthor>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let records = db
            .audiobooks()
            .list_authors_by_library(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(AudiobookAuthor::from).collect())
    }

    /// Get audiobook authors in a library with cursor-based pagination and filtering
    async fn audiobook_authors_connection(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<AudiobookAuthorWhereInput>,
        order_by: Option<AudiobookAuthorOrderByInput>,
    ) -> Result<AudiobookAuthorConnection> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let (offset, limit) = parse_pagination_args(first, after)
            .map_err(|e| async_graphql::Error::new(e))?;

        let name_filter = r#where.as_ref().and_then(|w| {
            w.name.as_ref().and_then(|f| f.contains.clone())
        });

        let sort_field = order_by
            .as_ref()
            .and_then(|o| o.field)
            .unwrap_or(AudiobookAuthorSortField::Name);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(super::filters::OrderDirection::Asc);

        let (records, total) = db
            .audiobooks()
            .list_authors_by_library_paginated(
                lib_id,
                offset,
                limit,
                name_filter.as_deref(),
                &audiobook_author_sort_field_to_column(sort_field),
                sort_dir == super::filters::OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let authors: Vec<AudiobookAuthor> = records.into_iter().map(AudiobookAuthor::from).collect();
        let connection = Connection::from_items(authors, offset, limit, total);
        
        Ok(AudiobookAuthorConnection::from_connection(connection))
    }

    /// Get all chapters for an audiobook
    async fn audiobook_chapters(&self, ctx: &Context<'_>, audiobook_id: String) -> Result<Vec<AudiobookChapter>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let book_id = Uuid::parse_str(&audiobook_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid audiobook ID: {}", e)))?;

        let records = db
            .chapters()
            .list_by_audiobook(book_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(AudiobookChapter::from).collect())
    }

    /// Get audiobook chapters with cursor-based pagination and filtering
    async fn audiobook_chapters_connection(
        &self,
        ctx: &Context<'_>,
        audiobook_id: String,
        #[graphql(default = 50)] first: Option<i32>,
        after: Option<String>,
        r#where: Option<AudiobookChapterWhereInput>,
        order_by: Option<AudiobookChapterOrderByInput>,
    ) -> Result<AudiobookChapterConnection> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let book_id = Uuid::parse_str(&audiobook_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid audiobook ID: {}", e)))?;

        let (offset, limit) = parse_pagination_args(first, after)
            .map_err(|e| async_graphql::Error::new(e))?;

        let status_filter = r#where.as_ref().and_then(|w| {
            w.status.as_ref().and_then(|f| f.eq.clone())
        });

        let sort_field = order_by
            .as_ref()
            .and_then(|o| o.field)
            .unwrap_or(AudiobookChapterSortField::ChapterNumber);
        let sort_dir = order_by
            .as_ref()
            .and_then(|o| o.direction)
            .unwrap_or(super::filters::OrderDirection::Asc);

        let (records, total) = db
            .chapters()
            .list_by_audiobook_paginated(
                book_id,
                offset,
                limit,
                status_filter.as_deref(),
                &audiobook_chapter_sort_field_to_column(sort_field),
                sort_dir == super::filters::OrderDirection::Asc,
            )
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let chapters: Vec<AudiobookChapter> = records.into_iter().map(AudiobookChapter::from).collect();
        let connection = Connection::from_items(chapters, offset, limit, total);
        
        Ok(AudiobookChapterConnection::from_connection(connection))
    }

    /// Search for audiobooks on OpenLibrary
    async fn search_audiobooks(&self, ctx: &Context<'_>, query: String) -> Result<Vec<AudiobookSearchResult>> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let results = metadata
            .search_audiobooks(&query)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(results
            .into_iter()
            .map(|a| AudiobookSearchResult {
                provider: "openlibrary".to_string(),
                provider_id: a.provider_id,
                title: a.title,
                author_name: a.author_name,
                year: a.year,
                cover_url: a.cover_url,
                isbn: a.isbn,
                description: a.description,
            })
            .collect())
    }

    // ------------------------------------------------------------------------
    // Episodes
    // ------------------------------------------------------------------------

    /// Get all episodes for a TV show
    async fn episodes(&self, ctx: &Context<'_>, tv_show_id: String) -> Result<Vec<Episode>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let show_id = Uuid::parse_str(&tv_show_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid show ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let records = db
            .episodes()
            .list_by_show(show_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Batch fetch watch progress for all episodes
        let episode_ids: Vec<Uuid> = records.iter().map(|r| r.id).collect();
        let watch_progress_list = db
            .watch_progress()
            .get_episode_progress_batch(user_id, &episode_ids)
            .await
            .unwrap_or_default();
        
        // Create a map for quick lookup
        let progress_map: std::collections::HashMap<Uuid, crate::db::WatchProgressRecord> = 
            watch_progress_list.into_iter()
                .filter_map(|wp| wp.episode_id.map(|eid| (eid, wp)))
                .collect();

        // For downloaded episodes, look up the media file with its metadata
        let mut episodes = Vec::with_capacity(records.len());
        for r in records {
            let episode_id = r.id;
            let media_file = if r.status == "downloaded" {
                // Try to get the media file for this episode (includes metadata from FFmpeg analysis)
                db.media_files()
                    .get_by_episode_id(r.id)
                    .await
                    .ok()
                    .flatten()
            } else {
                None
            };

            // Get watch progress for this episode
            let watch_progress = progress_map.get(&episode_id).cloned();

            episodes.push(Episode::from_record_with_progress(r, media_file, watch_progress));
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
            .map(|r| Episode::from_record(r, None)) // Wanted episodes don't have media files yet
            .collect())
    }

    // ------------------------------------------------------------------------
    // Subtitles and Media Analysis
    // ------------------------------------------------------------------------

    /// Get all subtitles for a media file
    async fn subtitles_for_media_file(
        &self,
        ctx: &Context<'_>,
        media_file_id: String,
    ) -> Result<Vec<Subtitle>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let file_id = Uuid::parse_str(&media_file_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid media file ID: {}", e)))?;

        let records = db
            .subtitles()
            .list_by_media_file(file_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(Subtitle::from_record).collect())
    }

    /// Get all subtitles for an episode (via linked media file)
    async fn subtitles_for_episode(
        &self,
        ctx: &Context<'_>,
        episode_id: String,
    ) -> Result<Vec<Subtitle>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let ep_id = Uuid::parse_str(&episode_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid episode ID: {}", e)))?;

        let records = db
            .subtitles()
            .list_by_episode(ep_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records.into_iter().map(Subtitle::from_record).collect())
    }

    /// Get detailed media file information including all streams
    async fn media_file_details(
        &self,
        ctx: &Context<'_>,
        media_file_id: String,
    ) -> Result<Option<MediaFileDetails>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let file_id = Uuid::parse_str(&media_file_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid media file ID: {}", e)))?;

        let file = match db.media_files().get_by_id(file_id).await {
            Ok(Some(f)) => f,
            Ok(None) => return Ok(None),
            Err(e) => return Err(async_graphql::Error::new(e.to_string())),
        };

        let video_streams = db
            .streams()
            .list_video_streams(file_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let audio_streams = db
            .streams()
            .list_audio_streams(file_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let subtitles = db
            .subtitles()
            .list_by_media_file(file_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let chapters = db
            .streams()
            .list_chapters(file_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(Some(MediaFileDetails {
            file: MediaFile::from_record(file),
            video_streams: video_streams.into_iter().map(VideoStreamInfo::from_record).collect(),
            audio_streams: audio_streams.into_iter().map(AudioStreamInfo::from_record).collect(),
            subtitles: subtitles.into_iter().map(Subtitle::from_record).collect(),
            chapters: chapters.into_iter().map(ChapterInfo::from_record).collect(),
        }))
    }

    /// Get media file by path
    ///
    /// Returns the media file record if found, null otherwise.
    /// Useful for file browsers to check if a file has been analyzed.
    async fn media_file_by_path(
        &self,
        ctx: &Context<'_>,
        path: String,
    ) -> Result<Option<MediaFile>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let file = db
            .media_files()
            .get_by_path(&path)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(file.map(MediaFile::from_record))
    }

    /// Get media file for a movie
    ///
    /// Returns the media file associated with a movie, if one exists.
    async fn movie_media_file(
        &self,
        ctx: &Context<'_>,
        movie_id: String,
    ) -> Result<Option<MediaFile>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let movie_uuid = Uuid::parse_str(&movie_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid movie ID: {}", e)))?;

        let file = db
            .media_files()
            .get_by_movie_id(movie_uuid)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(file.map(MediaFile::from_record))
    }

    /// Get subtitle settings for a library
    async fn library_subtitle_settings(
        &self,
        ctx: &Context<'_>,
        library_id: String,
    ) -> Result<SubtitleSettings> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        let library = db
            .libraries()
            .get_by_id(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Library not found"))?;

        Ok(SubtitleSettings {
            auto_download: library.auto_download_subtitles.unwrap_or(false),
            languages: library.preferred_subtitle_languages.unwrap_or_default(),
        })
    }

    // ------------------------------------------------------------------------
    // Naming Patterns
    // ------------------------------------------------------------------------

    /// Get all naming pattern presets
    async fn naming_patterns(&self, ctx: &Context<'_>) -> Result<Vec<NamingPattern>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let records = db
            .naming_patterns()
            .list_all()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(NamingPattern::from_record)
            .collect())
    }

    /// Get a specific naming pattern by ID
    async fn naming_pattern(&self, ctx: &Context<'_>, id: String) -> Result<Option<NamingPattern>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let pattern_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid pattern ID: {}", e)))?;

        let record = db
            .naming_patterns()
            .get_by_id(pattern_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(record.map(NamingPattern::from_record))
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
    // Playback Sessions
    // ------------------------------------------------------------------------

    /// Get the current user's active playback session
    async fn playback_session(&self, ctx: &Context<'_>) -> Result<Option<PlaybackSession>> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let session = db
            .playback()
            .get_active_session(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(session.map(PlaybackSession::from_record))
    }

    /// Get playback settings (sync interval, etc.)
    async fn playback_settings(&self, ctx: &Context<'_>) -> Result<PlaybackSettings> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        
        let sync_interval = db
            .settings()
            .get_or_default::<i32>("playback_sync_interval", 15)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        
        Ok(PlaybackSettings {
            sync_interval_seconds: sync_interval,
        })
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

    /// Get file matches for a torrent
    ///
    /// Returns the list of files in the torrent and what library items they match to.
    /// Accepts either a database UUID or an info_hash.
    async fn torrent_file_matches(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Torrent ID (UUID) or info_hash")] id: String,
    ) -> Result<Vec<TorrentFileMatch>> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        // Try to parse as UUID first, then fall back to info_hash lookup
        let torrent_id = if let Ok(uuid) = uuid::Uuid::parse_str(&id) {
            uuid
        } else {
            // Look up torrent by info_hash
            let torrent = db.torrents().get_by_info_hash(&id).await?
                .ok_or_else(|| async_graphql::Error::new("Torrent not found"))?;
            torrent.id
        };

        let records = db.torrent_file_matches().list_by_torrent(torrent_id).await?;
        
        Ok(records
            .into_iter()
            .map(TorrentFileMatch::from_record)
            .collect())
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
    // LLM Parser Settings
    // ------------------------------------------------------------------------

    /// Get LLM parser settings
    async fn llm_parser_settings(&self, ctx: &Context<'_>) -> Result<LlmParserSettings> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        // Helper to get optional string (returns None if value is JSON null, "null", or empty)
        async fn get_optional_string(
            settings: &crate::db::SettingsRepository,
            key: &str,
        ) -> Result<Option<String>, async_graphql::Error> {
            // Get the raw setting record to check if value is JSON null
            let record = settings.get(key).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            
            match record {
                Some(r) => {
                    // Check if the JSON value is null
                    if r.value.is_null() {
                        return Ok(None);
                    }
                    // Try to get as string
                    match r.value.as_str() {
                        Some(s) if s != "null" && !s.is_empty() => Ok(Some(s.to_string())),
                        _ => Ok(None),
                    }
                }
                None => Ok(None),
            }
        }

        Ok(LlmParserSettings {
            enabled: settings.get_or_default("llm.enabled", false).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            ollama_url: settings.get_or_default("llm.ollama_url", "http://localhost:11434".to_string()).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            ollama_model: settings.get_or_default("llm.ollama_model", "qwen2.5-coder:7b".to_string()).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            timeout_seconds: settings.get_or_default("llm.timeout_seconds", 30).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            temperature: settings.get_or_default("llm.temperature", 0.1).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            max_tokens: settings.get_or_default("llm.max_tokens", 256).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            prompt_template: settings.get_or_default("llm.prompt_template", 
                r#"Parse this media filename. Fill ALL fields. Use null if not found.
Clean the title (remove dots/underscores). Release group is after final hyphen.
Set type to "movie" or "tv" based on whether season/episode are present.

Filename: {filename}

{"type":null,"title":null,"year":null,"season":null,"episode":null,"resolution":null,"source":null,"video_codec":null,"audio":null,"hdr":null,"release_group":null,"edition":null}"#.to_string()).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            confidence_threshold: settings.get_or_default("llm.confidence_threshold", 0.7).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            // Library-type-specific models
            model_movies: get_optional_string(&settings, "llm.model.movies").await?,
            model_tv: get_optional_string(&settings, "llm.model.tv").await?,
            model_music: get_optional_string(&settings, "llm.model.music").await?,
            model_audiobooks: get_optional_string(&settings, "llm.model.audiobooks").await?,
            // Library-type-specific prompts
            prompt_movies: get_optional_string(&settings, "llm.prompt.movies").await?,
            prompt_tv: get_optional_string(&settings, "llm.prompt.tv").await?,
            prompt_music: get_optional_string(&settings, "llm.prompt.music").await?,
            prompt_audiobooks: get_optional_string(&settings, "llm.prompt.audiobooks").await?,
        })
    }

    // ------------------------------------------------------------------------
    // Logs
    // ------------------------------------------------------------------------

    /// Get logs with optional filtering and pagination
    async fn logs(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Filter options")] filter: Option<LogFilterInput>,
        #[graphql(desc = "Sort order")] order_by: Option<LogOrderByInput>,
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

        // Convert order_by to database format
        let order = order_by.map(|o| {
            use crate::db::logs::LogOrderBy;
            use crate::graphql::filters::OrderDirection;
            
            let field = match o.field.unwrap_or_default() {
                LogSortField::TIMESTAMP => "timestamp",
                LogSortField::LEVEL => "level",
                LogSortField::TARGET => "target",
            };
            let direction = match o.direction.unwrap_or(OrderDirection::Desc) {
                OrderDirection::Asc => "ASC",
                OrderDirection::Desc => "DESC",
            };
            LogOrderBy {
                field: field.to_string(),
                direction: direction.to_string(),
            }
        });

        let result = db
            .logs()
            .list(log_filter, order, limit as i64, offset as i64)
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
            .filter_map(|entry| {
                // Skip entries without season/episode numbers (specials)
                let season = entry.season?;
                let episode = entry.number?;

                Some(UpcomingEpisode {
                    tvmaze_id: entry.id as i32,
                    name: entry.name,
                    season: season as i32,
                    episode: episode as i32,
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

    // ------------------------------------------------------------------------
    // Indexer Search
    // ------------------------------------------------------------------------

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

        // Spawn initial scan in background (same pattern as scan_library mutation)
        let library_id = record.id;
        let library_name = record.name.clone();
        let scanner = ctx.data_unchecked::<Arc<ScannerService>>().clone();
        let db_clone = db.clone();
        tokio::spawn(async move {
            tracing::info!("Starting initial scan for library '{}'", library_name);
            if let Err(e) = scanner.scan_library(library_id).await {
                tracing::error!("Initial scan failed for '{}': {}", library_name, e);
                if let Err(reset_err) = db_clone.libraries().set_scanning(library_id, false).await {
                    tracing::error!(library_id = %library_id, error = %reset_err, "Failed to reset scanning state");
                }
            }
        });

        let library = Library {
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
            scanning: record.scanning,
        };

        // Emit library created event
        if let Ok(library_tx) =
            ctx.data::<tokio::sync::broadcast::Sender<LibraryChangedEvent>>()
        {
            let _ = library_tx.send(LibraryChangedEvent {
                change_type: LibraryChangeType::Created,
                library_id: library.id.clone(),
                library_name: Some(library.name.clone()),
                library: Some(library.clone()),
            });
        }

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
            let library = Library {
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
                scanning: record.scanning,
            };

            // Emit library updated event
            if let Ok(library_tx) =
                ctx.data::<tokio::sync::broadcast::Sender<LibraryChangedEvent>>()
            {
                let _ = library_tx.send(LibraryChangedEvent {
                    change_type: LibraryChangeType::Updated,
                    library_id: library.id.clone(),
                    library_name: Some(library.name.clone()),
                    library: Some(library.clone()),
                });
            }

            Ok(LibraryResult {
                success: true,
                library: Some(library),
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

        // Get library name before deleting for the event
        let library_name = db
            .libraries()
            .get_by_id(lib_id)
            .await
            .ok()
            .flatten()
            .map(|lib| lib.name);

        let deleted = db
            .libraries()
            .delete(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        // Emit library deleted event
        if deleted {
            if let Ok(library_tx) =
                ctx.data::<tokio::sync::broadcast::Sender<LibraryChangedEvent>>()
            {
                let _ = library_tx.send(LibraryChangedEvent {
                    change_type: LibraryChangeType::Deleted,
                    library_id: id.clone(),
                    library_name,
                    library: None,
                });
            }
        }

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

    /// Consolidate library folders - merge duplicate show folders, update paths
    /// This is useful after changing naming conventions to clean up old folder structures
    async fn consolidate_library(&self, ctx: &Context<'_>, id: String) -> Result<ConsolidateLibraryResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let library_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        tracing::debug!("Consolidation requested for library {}", id);

        let organizer = crate::services::OrganizerService::new(db.clone());
        
        match organizer.consolidate_library(library_id).await {
            Ok(result) => {
                tracing::info!(
                    library_id = %id,
                    folders_removed = result.folders_removed,
                    files_moved = result.files_moved,
                    "Library consolidation complete"
                );
                Ok(ConsolidateLibraryResult {
                    success: result.success,
                    folders_removed: result.folders_removed,
                    files_moved: result.files_moved,
                    messages: result.messages,
                })
            }
            Err(e) => {
                tracing::error!(library_id = %id, error = %e, "Library consolidation failed");
                Ok(ConsolidateLibraryResult {
                    success: false,
                    folders_removed: 0,
                    files_moved: 0,
                    messages: vec![format!("Consolidation failed: {}", e)],
                })
            }
        }
    }

    // ------------------------------------------------------------------------
    // Subtitles and Media Analysis
    // ------------------------------------------------------------------------

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
    // Movies
    // ------------------------------------------------------------------------

    /// Add a movie to a library
    async fn add_movie(
        &self,
        ctx: &Context<'_>,
        library_id: String,
        input: AddMovieInput,
    ) -> Result<MovieResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>().clone();
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>().clone();

        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        if !metadata.has_tmdb() {
            return Ok(MovieResult {
                success: false,
                movie: None,
                error: Some("TMDB API key not configured".to_string()),
            });
        }

        let is_monitored = input.monitored.unwrap_or(true);

        match metadata
            .add_movie_from_provider(crate::services::AddMovieOptions {
                provider: crate::services::MetadataProvider::Tmdb,
                provider_id: input.tmdb_id as u32,
                library_id: lib_id,
                user_id,
                monitored: is_monitored,
                path: input.path,
            })
            .await
        {
            Ok(record) => {
                tracing::info!(
                    user_id = %user.user_id,
                    movie_title = %record.title,
                    movie_id = %record.id,
                    library_id = %lib_id,
                    "User added movie: {}",
                    record.title
                );

                // Trigger immediate auto-hunt if the library has auto_hunt enabled and movie is monitored
                if is_monitored {
                    let db_clone = db.clone();
                    let movie_record = record.clone();
                    let torrent_svc = torrent_service.clone();

                    tokio::spawn(async move {
                        // Check if library has auto_hunt enabled
                        let library = match db_clone.libraries().get_by_id(lib_id).await {
                            Ok(Some(lib)) => lib,
                            Ok(None) => {
                                tracing::warn!(library_id = %lib_id, "Library not found for auto-hunt");
                                return;
                            }
                            Err(e) => {
                                tracing::warn!(library_id = %lib_id, error = %e, "Failed to get library for auto-hunt");
                                return;
                            }
                        };

                        if !library.auto_hunt {
                            tracing::debug!(
                                library_id = %lib_id,
                                movie_title = %movie_record.title,
                                "Library does not have auto_hunt enabled, skipping immediate hunt"
                            );
                            return;
                        }

                        tracing::info!(
                            movie_id = %movie_record.id,
                            movie_title = %movie_record.title,
                            "Triggering immediate auto-hunt for newly added movie"
                        );

                        // Get encryption key and create IndexerManager
                        let encryption_key = match db_clone.settings().get_or_create_indexer_encryption_key().await {
                            Ok(key) => key,
                            Err(e) => {
                                tracing::warn!(error = %e, "Failed to get encryption key for auto-hunt");
                                return;
                            }
                        };

                        let indexer_manager = match crate::indexer::manager::IndexerManager::new(db_clone.clone(), &encryption_key).await {
                            Ok(mgr) => std::sync::Arc::new(mgr),
                            Err(e) => {
                                tracing::warn!(error = %e, "Failed to create IndexerManager for auto-hunt");
                                return;
                            }
                        };

                        // Load user's indexers
                        if let Err(e) = indexer_manager.load_user_indexers(user_id).await {
                            tracing::warn!(user_id = %user_id, error = %e, "Failed to load indexers for auto-hunt");
                            return;
                        }

                        // Run hunt for this specific movie
                        match crate::jobs::auto_hunt::hunt_single_movie(
                            &db_clone,
                            &movie_record,
                            &library,
                            &torrent_svc,
                            &indexer_manager,
                        ).await {
                            Ok(result) => {
                                if result.downloaded > 0 {
                                    tracing::info!(
                                        movie_title = %movie_record.title,
                                        "Immediate auto-hunt successful, download started"
                                    );
                                } else if result.matched > 0 {
                                    tracing::info!(
                                        movie_title = %movie_record.title,
                                        "Found matching releases but download failed"
                                    );
                                } else {
                                    tracing::info!(
                                        movie_title = %movie_record.title,
                                        "No matching releases found for immediate auto-hunt"
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    movie_title = %movie_record.title,
                                    error = %e,
                                    "Immediate auto-hunt failed"
                                );
                            }
                        }
                    });
                }

                Ok(MovieResult {
                    success: true,
                    movie: Some(movie_record_to_graphql(record)),
                    error: None,
                })
            }
            Err(e) => Ok(MovieResult {
                success: false,
                movie: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update a movie
    async fn update_movie(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateMovieInput,
    ) -> Result<MovieResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let movie_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid movie ID: {}", e)))?;

        // Build update
        let update = crate::db::UpdateMovie {
            monitored: input.monitored,
            path: input.path,
            allowed_resolutions_override: input.allowed_resolutions_override.flatten(),
            allowed_video_codecs_override: input.allowed_video_codecs_override.flatten(),
            allowed_audio_formats_override: input.allowed_audio_formats_override.flatten(),
            require_hdr_override: input.require_hdr_override.flatten(),
            allowed_hdr_types_override: input.allowed_hdr_types_override.flatten(),
            allowed_sources_override: input.allowed_sources_override.flatten(),
            release_group_blacklist_override: input.release_group_blacklist_override.flatten(),
            release_group_whitelist_override: input.release_group_whitelist_override.flatten(),
            ..Default::default()
        };

        match db.movies().update(movie_id, update).await {
            Ok(Some(record)) => Ok(MovieResult {
                success: true,
                movie: Some(movie_record_to_graphql(record)),
                error: None,
            }),
            Ok(None) => Ok(MovieResult {
                success: false,
                movie: None,
                error: Some("Movie not found".to_string()),
            }),
            Err(e) => Ok(MovieResult {
                success: false,
                movie: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete a movie from a library
    async fn delete_movie(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let movie_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid movie ID: {}", e)))?;

        match db.movies().delete(movie_id).await {
            Ok(true) => Ok(MutationResult {
                success: true,
                error: None,
            }),
            Ok(false) => Ok(MutationResult {
                success: false,
                error: Some("Movie not found".to_string()),
            }),
            Err(e) => Ok(MutationResult {
                success: false,
                error: Some(e.to_string()),
            }),
        }
    }

    // ------------------------------------------------------------------------
    // Albums/Music
    // ------------------------------------------------------------------------

    /// Add an album to a library from MusicBrainz
    async fn add_album(
        &self,
        ctx: &Context<'_>,
        input: AddAlbumInput,
    ) -> Result<AlbumResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        let lib_id = Uuid::parse_str(&input.library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;
        let mbid = Uuid::parse_str(&input.musicbrainz_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid MusicBrainz ID: {}", e)))?;

        // Verify library exists and belongs to user
        let library = db
            .libraries()
            .get_by_id(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?
            .ok_or_else(|| async_graphql::Error::new("Library not found"))?;

        if library.user_id != user_id {
            return Err(async_graphql::Error::new("Not authorized to access this library"));
        }

        // Add album from MusicBrainz
        use crate::services::metadata::AddAlbumOptions;
        match metadata
            .add_album_from_provider(AddAlbumOptions {
                musicbrainz_id: mbid,
                library_id: lib_id,
                user_id,
                monitored: true,
            })
            .await
        {
            Ok(record) => {
                tracing::info!(
                    user_id = %user.user_id,
                    album_name = %record.name,
                    album_id = %record.id,
                    library_id = %lib_id,
                    "User added album: {}",
                    record.name
                );

                Ok(AlbumResult {
                    success: true,
                    album: Some(Album::from(record)),
                    error: None,
                })
            }
            Err(e) => Ok(AlbumResult {
                success: false,
                album: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Delete an album from a library
    async fn delete_album(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let album_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid album ID: {}", e)))?;

        // Verify album exists
        let album = db.albums().get_by_id(album_id).await.map_err(|e| {
            async_graphql::Error::new(format!("Failed to get album: {}", e))
        })?;

        if album.is_none() {
            return Ok(MutationResult {
                success: false,
                error: Some("Album not found".to_string()),
            });
        }

        // Delete the album and all associated data
        match db.albums().delete(album_id).await {
            Ok(deleted) => {
                if deleted {
                    tracing::info!("Deleted album {}", album_id);
                    Ok(MutationResult {
                        success: true,
                        error: None,
                    })
                } else {
                    Ok(MutationResult {
                        success: false,
                        error: Some("Album not found".to_string()),
                    })
                }
            }
            Err(e) => {
                tracing::error!(album_id = %album_id, error = %e, "Failed to delete album");
                Ok(MutationResult {
                    success: false,
                    error: Some(format!("Failed to delete album: {}", e)),
                })
            }
        }
    }

    // ------------------------------------------------------------------------
    // Audiobooks
    // ------------------------------------------------------------------------

    /// Add an audiobook to a library from OpenLibrary
    async fn add_audiobook(&self, ctx: &Context<'_>, input: AddAudiobookInput) -> Result<AudiobookResult> {
        let user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();
        let library_id = Uuid::parse_str(&input.library_id).map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;
        let user_id = Uuid::parse_str(&user.user_id).map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;
        use crate::services::metadata::AddAudiobookOptions;
        match metadata.add_audiobook_from_provider(AddAudiobookOptions { openlibrary_id: input.openlibrary_id, library_id, user_id, monitored: true }).await {
            Ok(record) => Ok(AudiobookResult { success: true, audiobook: Some(Audiobook::from(record)), error: None }),
            Err(e) => Ok(AudiobookResult { success: false, audiobook: None, error: Some(e.to_string()) }),
        }
    }

    /// Delete an audiobook from a library
    async fn delete_audiobook(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let audiobook_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid audiobook ID: {}", e)))?;

        // Verify audiobook exists
        let audiobook = db.audiobooks().get_by_id(audiobook_id).await.map_err(|e| {
            async_graphql::Error::new(format!("Failed to get audiobook: {}", e))
        })?;

        if audiobook.is_none() {
            return Ok(MutationResult {
                success: false,
                error: Some("Audiobook not found".to_string()),
            });
        }

        // Delete the audiobook and all associated data
        match db.audiobooks().delete(audiobook_id).await {
            Ok(deleted) => {
                if deleted {
                    tracing::info!(audiobook_id = %audiobook_id, "Deleted audiobook");
                    Ok(MutationResult {
                        success: true,
                        error: None,
                    })
                } else {
                    Ok(MutationResult {
                        success: false,
                        error: Some("Audiobook not found".to_string()),
                    })
                }
            }
            Err(e) => {
                tracing::error!(audiobook_id = %audiobook_id, error = %e, "Failed to delete audiobook");
                Ok(MutationResult {
                    success: false,
                    error: Some(format!("Failed to delete audiobook: {}", e)),
                })
            }
        }
    }

    // ------------------------------------------------------------------------
    // Naming Patterns
    // ------------------------------------------------------------------------

    /// Create a custom naming pattern
    async fn create_naming_pattern(
        &self,
        ctx: &Context<'_>,
        input: CreateNamingPatternInput,
    ) -> Result<NamingPatternResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();

        let record = db
            .naming_patterns()
            .create(crate::db::CreateNamingPattern {
                name: input.name,
                pattern: input.pattern,
                description: input.description,
                library_type: input.library_type.unwrap_or_else(|| "tv".to_string()),
            })
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(NamingPatternResult {
            success: true,
            naming_pattern: Some(NamingPattern::from_record(record)),
            error: None,
        })
    }

    /// Delete a custom naming pattern (system patterns cannot be deleted)
    async fn delete_naming_pattern(&self, ctx: &Context<'_>, id: String) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let pattern_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid pattern ID: {}", e)))?;

        let deleted = db
            .naming_patterns()
            .delete(pattern_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: deleted,
            error: if deleted {
                None
            } else {
                Some("Pattern not found or is a system pattern".to_string())
            },
        })
    }

    /// Set a naming pattern as the default
    async fn set_default_naming_pattern(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<MutationResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let pattern_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid pattern ID: {}", e)))?;

        let updated = db
            .naming_patterns()
            .set_default(pattern_id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(MutationResult {
            success: updated,
            error: if updated {
                None
            } else {
                Some("Pattern not found".to_string())
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
            Ok(torrent_info) => {
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
    // Playback Sessions
    // ------------------------------------------------------------------------

    /// Start or resume playback of any content type
    async fn start_playback(
        &self,
        ctx: &Context<'_>,
        input: StartPlaybackInput,
    ) -> Result<PlaybackResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;
        let content_id = Uuid::parse_str(&input.content_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid content ID: {}", e)))?;
        let media_file_id = Uuid::parse_str(&input.media_file_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid media file ID: {}", e)))?;
        let parent_id = input.parent_id.as_ref().map(|id| {
            Uuid::parse_str(id)
        }).transpose().map_err(|e| async_graphql::Error::new(format!("Invalid parent ID: {}", e)))?;

        // Set the appropriate IDs based on content type
        let (episode_id, movie_id, track_id, audiobook_id, tv_show_id, album_id) = match input.content_type {
            PlaybackContentType::Episode => (Some(content_id), None, None, None, parent_id, None),
            PlaybackContentType::Movie => (None, Some(content_id), None, None, None, None),
            PlaybackContentType::Track => (None, None, Some(content_id), None, None, parent_id),
            PlaybackContentType::Audiobook => (None, None, None, Some(content_id), None, None),
        };

        let db_input = crate::db::UpsertPlaybackSession {
            user_id,
            content_type: input.content_type.as_str().to_string(),
            media_file_id: Some(media_file_id),
            episode_id,
            movie_id,
            track_id,
            audiobook_id,
            tv_show_id,
            album_id,
            current_position: input.start_position.unwrap_or(0.0),
            duration: input.duration,
            volume: 1.0,
            is_muted: false,
            is_playing: true,
        };

        match db.playback().upsert_session(db_input).await {
            Ok(session) => Ok(PlaybackResult {
                success: true,
                session: Some(PlaybackSession::from_record(session)),
                error: None,
            }),
            Err(e) => Ok(PlaybackResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update playback position/state
    /// Also persists watch progress to the watch_progress table for resume functionality
    async fn update_playback(
        &self,
        ctx: &Context<'_>,
        input: UpdatePlaybackInput,
    ) -> Result<PlaybackResult> {
        use crate::db::watch_progress::ContentType as WPContentType;
        
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        let db_input = crate::db::UpdatePlaybackPosition {
            current_position: input.current_position,
            duration: input.duration,
            volume: input.volume,
            is_muted: input.is_muted,
            is_playing: input.is_playing,
        };

        match db.playback().update_position(user_id, db_input).await {
            Ok(Some(session)) => {
                // Persist watch progress for all content types
                if let Some(position) = input.current_position {
                    // Determine content type and ID from session
                    let content_info: Option<(WPContentType, Uuid)> = match session.content_type.as_deref() {
                        Some("episode") => session.episode_id.map(|id| (WPContentType::Episode, id)),
                        Some("movie") => session.movie_id.map(|id| (WPContentType::Movie, id)),
                        Some("track") => session.track_id.map(|id| (WPContentType::Track, id)),
                        Some("audiobook") => session.audiobook_id.map(|id| (WPContentType::Audiobook, id)),
                        _ => {
                            // Fallback to old behavior for backwards compatibility
                            session.episode_id.map(|id| (WPContentType::Episode, id))
                        }
                    };

                    if let Some((content_type, content_id)) = content_info {
                        tracing::info!(
                            "Persisting watch progress: user={}, type={:?}, content={}, position={:.1}s",
                            user_id, content_type, content_id, position
                        );
                        
                        let wp_input = crate::db::UpsertWatchProgress {
                            user_id,
                            content_type,
                            content_id,
                            media_file_id: session.media_file_id,
                            current_position: position,
                            duration: input.duration.or(session.duration),
                        };
                        
                        match db.watch_progress().upsert_progress(wp_input).await {
                            Ok(wp) => tracing::info!(
                                "Watch progress saved: content={}, progress={:.1}%, is_watched={}",
                                content_id, wp.progress_percent * 100.0, wp.is_watched
                            ),
                            Err(e) => tracing::warn!("Failed to persist watch progress: {}", e),
                        }
                    } else {
                        tracing::debug!(
                            "Skipping watch progress: no content ID found in session"
                        );
                    }
                }
                
                Ok(PlaybackResult {
                    success: true,
                    session: Some(PlaybackSession::from_record(session)),
                    error: None,
                })
            },
            Ok(None) => Ok(PlaybackResult {
                success: false,
                session: None,
                error: Some("No active playback session".to_string()),
            }),
            Err(e) => Ok(PlaybackResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Stop playback (mark session as completed)
    async fn stop_playback(&self, ctx: &Context<'_>) -> Result<PlaybackResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        match db.playback().complete_session(user_id).await {
            Ok(Some(session)) => Ok(PlaybackResult {
                success: true,
                session: Some(PlaybackSession::from_record(session)),
                error: None,
            }),
            Ok(None) => Ok(PlaybackResult {
                success: true,
                session: None,
                error: None, // No active session is not an error
            }),
            Err(e) => Ok(PlaybackResult {
                success: false,
                session: None,
                error: Some(e.to_string()),
            }),
        }
    }

    /// Update playback settings
    async fn update_playback_settings(
        &self,
        ctx: &Context<'_>,
        input: UpdatePlaybackSettingsInput,
    ) -> Result<PlaybackSettings> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        
        // Get current settings
        let mut sync_interval = db
            .settings()
            .get_or_default::<i32>("playback_sync_interval", 15)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        
        // Update if provided (clamp to 5-60 seconds)
        if let Some(new_interval) = input.sync_interval_seconds {
            sync_interval = new_interval.clamp(5, 60);
            db.settings()
                .set_with_category(
                    "playback_sync_interval",
                    sync_interval,
                    "playback",
                    Some("How often to sync watch progress to the database (in seconds)"),
                )
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        
        Ok(PlaybackSettings {
            sync_interval_seconds: sync_interval,
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
        let db = ctx.data_unchecked::<Database>();

        // Parse user_id for database persistence
        let user_id = Uuid::parse_str(&user.user_id).ok();

        let result = if let Some(magnet) = input.magnet {
            // Magnet links go through add_magnet
            service.add_magnet(&magnet, user_id).await
        } else if let Some(url) = input.url {
            // If indexer_id is provided, download the .torrent file with authentication
            if let Some(ref indexer_id_str) = input.indexer_id {
                if let Ok(indexer_id) = Uuid::parse_str(indexer_id_str) {
                    // Get indexer config and credentials
                    if let Ok(Some(config)) = db.indexers().get(indexer_id).await {
                        if let Ok(credentials) = db.indexers().get_credentials(indexer_id).await {
                            // Get encryption key
                            if let Ok(encryption_key) = db.settings().get_or_create_indexer_encryption_key().await {
                                if let Ok(encryption) = crate::indexer::encryption::CredentialEncryption::from_base64_key(&encryption_key) {
                                    // Decrypt credentials
                                    let mut decrypted_creds: std::collections::HashMap<String, String> = std::collections::HashMap::new();
                                    for cred in credentials {
                                        if let Ok(value) = encryption.decrypt(&cred.encrypted_value, &cred.nonce) {
                                            decrypted_creds.insert(cred.credential_type, value);
                                        }
                                    }

                                    // Download .torrent file with authentication
                                    let torrent_bytes = download_torrent_file_authenticated(
                                        &url,
                                        &config.indexer_type,
                                        &decrypted_creds,
                                    ).await;

                                    match torrent_bytes {
                                        Ok(bytes) => {
                                            // Add torrent from bytes
                                            return match service.add_torrent_bytes(&bytes, user_id).await {
                                                Ok(info) => {
                                                    tracing::info!(
                                                        user_id = %user.user_id,
                                                        torrent_id = info.id,
                                                        torrent_name = %info.name,
                                                        "User added torrent from authenticated download"
                                                    );

                                                    // Note: File-level matching happens automatically when torrent is processed
                                                    // via torrent_file_matches table

                                                    Ok(AddTorrentResult {
                                                        success: true,
                                                        torrent: Some(info.into()),
                                                        error: None,
                                                    })
                                                }
                                                Err(e) => Ok(AddTorrentResult {
                                                    success: false,
                                                    torrent: None,
                                                    error: Some(e.to_string()),
                                                }),
                                            };
                                        }
                                        Err(e) => {
                                            tracing::warn!(error = %e, "Failed to download .torrent file with auth, falling back to unauthenticated");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Fall back to unauthenticated download
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

                // Note: File-level matching happens automatically when torrent is processed
                // via torrent_file_matches table in TorrentFileMatcher
                let _ = &info.info_hash; // Used for tracing/debugging

                // Mark episode/movie as downloading if explicitly specified
                if let Some(ref episode_id_str) = input.episode_id {
                    if let Ok(ep_id) = Uuid::parse_str(episode_id_str) {
                        let _ = db.episodes().mark_downloading(ep_id).await;
                    }
                }
                // Note: Movie status is derived from media_files, no need to mark downloading

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
    /// This uses the unified TorrentProcessor to:
    /// 1. Parse filenames to identify show/episode
    /// 2. Match to existing shows in the library
    /// 3. Copy/move/hardlink files based on library settings
    /// 4. Create folder structure (Show Name/Season XX/)
    /// 5. Update episode status to downloaded
    ///
    /// If library_id is provided, the torrent will be linked to that library first.
    async fn organize_torrent(
        &self,
        ctx: &Context<'_>,
        id: i32,
        #[graphql(
            desc = "Optional library ID to organize into (links torrent to library first)"
        )]
        library_id: Option<String>,
        #[graphql(desc = "Optional album ID for music torrents")]
        album_id: Option<String>,
    ) -> Result<OrganizeTorrentResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();

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

        // Note: library_id and album_id hints are no longer used for torrent-level linking
        // File-level matching will happen automatically in process_torrent
        let _ = (&library_id, &album_id); // Suppress unused warnings

        // Use the unified TorrentProcessor with force=true to reprocess
        // Include metadata service for auto-adding movies from TMDB
        let metadata_service = ctx.data_unchecked::<Arc<crate::services::MetadataService>>();
        let processor = crate::services::TorrentProcessor::with_services(
            db.clone(),
            ctx.data_unchecked::<Arc<crate::services::MediaAnalysisQueue>>().clone(),
            metadata_service.clone(),
        );

        match processor
            .process_torrent(torrent_service, &torrent_info.info_hash, true)
            .await
        {
            Ok(result) => Ok(OrganizeTorrentResult {
                success: result.success,
                organized_count: result.files_processed,
                failed_count: result.files_failed,
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
    // LLM Parser Settings
    // ------------------------------------------------------------------------

    /// Update LLM parser settings
    async fn update_llm_parser_settings(
        &self,
        ctx: &Context<'_>,
        input: UpdateLlmParserSettingsInput,
    ) -> Result<SettingsResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        // Update each setting if provided
        if let Some(v) = input.enabled {
            settings.set_with_category("llm.enabled", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.ollama_url {
            settings.set_with_category("llm.ollama_url", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.ollama_model {
            settings.set_with_category("llm.ollama_model", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.timeout_seconds {
            settings.set_with_category("llm.timeout_seconds", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.temperature {
            settings.set_with_category("llm.temperature", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.max_tokens {
            settings.set_with_category("llm.max_tokens", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.prompt_template {
            settings.set_with_category("llm.prompt_template", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.confidence_threshold {
            settings.set_with_category("llm.confidence_threshold", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        // Library-type-specific models
        if let Some(v) = input.model_movies {
            settings.set_with_category("llm.model.movies", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.model_tv {
            settings.set_with_category("llm.model.tv", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.model_music {
            settings.set_with_category("llm.model.music", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.model_audiobooks {
            settings.set_with_category("llm.model.audiobooks", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        // Library-type-specific prompts
        if let Some(v) = input.prompt_movies {
            settings.set_with_category("llm.prompt.movies", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.prompt_tv {
            settings.set_with_category("llm.prompt.tv", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.prompt_music {
            settings.set_with_category("llm.prompt.music", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }
        if let Some(v) = input.prompt_audiobooks {
            settings.set_with_category("llm.prompt.audiobooks", v, "llm", None).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        }

        Ok(SettingsResult {
            success: true,
            error: None,
        })
    }

    /// Test connection to Ollama server and list available models
    async fn test_ollama_connection(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Ollama server URL (defaults to configured URL)")] url: Option<String>,
    ) -> Result<OllamaConnectionResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        // Get URL from input or settings
        let ollama_url = match url {
            Some(u) => u,
            None => settings.get_or_default("llm.ollama_url", "http://localhost:11434".to_string()).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?,
        };

        let config = crate::services::OllamaConfig {
            url: ollama_url,
            ..Default::default()
        };
        let ollama = crate::services::OllamaService::new(config);

        match ollama.test_connection().await {
            Ok(models) => Ok(OllamaConnectionResult {
                success: true,
                available_models: models,
                error: None,
            }),
            Err(e) => Ok(OllamaConnectionResult {
                success: false,
                available_models: vec![],
                error: Some(e.to_string()),
            }),
        }
    }

    /// Test filename parsing with both regex and LLM parsers
    async fn test_filename_parser(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "Filename to parse")] filename: String,
    ) -> Result<TestFilenameParserResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        // Run regex parser with timing
        let regex_start = std::time::Instant::now();
        let parsed_ep = crate::services::filename_parser::parse_episode(&filename);
        let parsed_quality = crate::services::filename_parser::parse_quality(&filename);
        let regex_time_ms = regex_start.elapsed().as_secs_f64() * 1000.0;

        // Determine media type based on whether season/episode info was found
        let media_type = if parsed_ep.season.is_some() || parsed_ep.episode.is_some() || parsed_ep.date.is_some() {
            Some("tv".to_string())
        } else {
            Some("movie".to_string())
        };

        let regex_result = FilenameParseResult {
            media_type,
            title: parsed_ep.show_name.clone(),
            year: parsed_ep.year.map(|y| y as i32),
            season: parsed_ep.season.map(|s| s as i32),
            episode: parsed_ep.episode.map(|e| e as i32),
            episode_end: None, // Not supported by current parser
            resolution: parsed_quality.resolution.or(parsed_ep.resolution),
            source: parsed_quality.source.or(parsed_ep.source),
            video_codec: parsed_quality.codec.or(parsed_ep.codec),
            audio: parsed_quality.audio.or(parsed_ep.audio),
            hdr: parsed_quality.hdr.or(parsed_ep.hdr),
            release_group: parsed_ep.release_group,
            edition: None, // Not supported by current parser
            complete_series: false, // Not supported by current parser
            confidence: 0.8, // Default confidence for regex parser
        };

        // Check if LLM parsing is enabled
        let llm_enabled: bool = settings.get_or_default("llm.enabled", false).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        let (llm_result, llm_time_ms, llm_error) = if llm_enabled {
            // Build LLM config from settings
            let config = crate::services::OllamaConfig {
                url: settings.get_or_default("llm.ollama_url", "http://localhost:11434".to_string()).await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?,
                model: settings.get_or_default("llm.ollama_model", "qwen2.5-coder:7b".to_string()).await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?,
                timeout_seconds: settings.get_or_default::<i32>("llm.timeout_seconds", 30).await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))? as u64,
                temperature: settings.get_or_default::<f64>("llm.temperature", 0.1).await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))? as f32,
                max_tokens: settings.get_or_default::<i32>("llm.max_tokens", 256).await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))? as u32,
                prompt_template: settings.get_or_default("llm.prompt_template", 
                    "Parse this media filename: {filename}".to_string()).await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?,
            };

            let ollama = crate::services::OllamaService::new(config);
            let llm_start = std::time::Instant::now();
            
            match ollama.parse_filename(&filename).await {
                Ok(parsed) => {
                    let time_ms = llm_start.elapsed().as_secs_f64() * 1000.0;
                    let result = FilenameParseResult {
                        media_type: parsed.media_type,
                        title: parsed.title,
                        year: parsed.year,
                        season: parsed.season,
                        episode: parsed.episode,
                        episode_end: parsed.episode_end,
                        resolution: parsed.resolution,
                        source: parsed.source,
                        video_codec: parsed.video_codec,
                        audio: parsed.audio,
                        hdr: parsed.hdr,
                        release_group: parsed.release_group,
                        edition: parsed.edition,
                        complete_series: parsed.complete_series,
                        confidence: parsed.confidence,
                    };
                    (Some(result), Some(time_ms), None)
                }
                Err(e) => {
                    let time_ms = llm_start.elapsed().as_secs_f64() * 1000.0;
                    (None, Some(time_ms), Some(e.to_string()))
                }
            }
        } else {
            (None, None, Some("LLM parsing is not enabled".to_string()))
        };

        Ok(TestFilenameParserResult {
            regex_result,
            regex_time_ms,
            llm_result,
            llm_time_ms,
            llm_error,
        })
    }

    /// Set a generic app setting by key
    ///
    /// This allows setting arbitrary key-value pairs in the app_settings table.
    /// The category is extracted from the key (e.g., "metadata.tmdb_api_key" → "metadata").
    async fn set_setting(
        &self,
        ctx: &Context<'_>,
        key: String,
        value: String,
    ) -> Result<SettingsResult> {
        let _user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let settings = db.settings();

        // Extract category from key (use first part before dot, or "general")
        let category = key.split('.').next().unwrap_or("general").to_string();

        settings
            .set_with_category(&key, &value, &category, None)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        tracing::debug!(key = %key, "Updated app setting");

        Ok(SettingsResult {
            success: true,
            error: None,
        })
    }

    // ------------------------------------------------------------------------
    // Schedule Cache
    // ------------------------------------------------------------------------

    /// Refresh the TV schedule cache
    ///
    /// Forces a refresh of the TV schedule cache from TVMaze.
    /// This is normally done automatically every 6 hours.
    async fn refresh_schedule_cache(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 14, desc = "Number of days to fetch")] days: i32,
        #[graphql(desc = "Country code (e.g., 'US', 'GB')")] country: Option<String>,
    ) -> Result<RefreshScheduleResult> {
        let _user = ctx.auth_user()?;
        let metadata = ctx.data_unchecked::<Arc<MetadataService>>();

        match metadata
            .refresh_schedule_cache(days as u32, country.as_deref())
            .await
        {
            Ok(count) => Ok(RefreshScheduleResult {
                success: true,
                entries_updated: count as i32,
                error: None,
            }),
            Err(e) => Ok(RefreshScheduleResult {
                success: false,
                entries_updated: 0,
                error: Some(e.to_string()),
            }),
        }
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

    // ========================================================================
    // Auto-Hunt Mutations
    // ========================================================================

    /// Manually trigger auto-hunt for a specific library
    ///
    /// This immediately searches indexers for missing content in the library.
    /// Returns the number of items searched, matched, and downloaded.
    async fn trigger_auto_hunt(
        &self,
        ctx: &Context<'_>,
        library_id: String,
    ) -> Result<AutoHuntResult> {
        let user = ctx.auth_user()?;
        let db = ctx.data_unchecked::<Database>();
        let torrent_service = ctx.data_unchecked::<Arc<TorrentService>>();

        let lib_id = Uuid::parse_str(&library_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid library ID: {}", e)))?;

        // Verify the library exists and belongs to this user
        let library = db
            .libraries()
            .get_by_id(lib_id)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Database error: {}", e)))?
            .ok_or_else(|| async_graphql::Error::new("Library not found"))?;

        let user_id = Uuid::parse_str(&user.user_id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid user ID: {}", e)))?;

        if library.user_id != user_id {
            return Err(async_graphql::Error::new("Library not found"));
        }

        // Get encryption key and create IndexerManager
        let encryption_key = db
            .settings()
            .get_or_create_indexer_encryption_key()
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to get encryption key: {}", e)))?;

        let indexer_manager = crate::indexer::manager::IndexerManager::new(db.clone(), &encryption_key)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to create IndexerManager: {}", e)))?;

        // Load user's indexers
        indexer_manager
            .load_user_indexers(user_id)
            .await
            .map_err(|e| async_graphql::Error::new(format!("Failed to load indexers: {}", e)))?;

        let indexer_manager = std::sync::Arc::new(indexer_manager);

        // Run auto-hunt for this library (case-insensitive)
        let result = match library.library_type.to_lowercase().as_str() {
            "movies" => {
                crate::jobs::auto_hunt::hunt_movies_for_library(
                    db,
                    &library,
                    torrent_service,
                    &indexer_manager,
                )
                .await
            }
            "tv" => {
                crate::jobs::auto_hunt::hunt_tv_for_library(
                    db,
                    &library,
                    torrent_service,
                    &indexer_manager,
                )
                .await
            }
            _ => {
                return Ok(AutoHuntResult {
                    success: false,
                    error: Some(format!(
                        "Auto-hunt not yet supported for {} libraries",
                        library.library_type
                    )),
                    searched: 0,
                    matched: 0,
                    downloaded: 0,
                    skipped: 0,
                    failed: 0,
                });
            }
        };

        match result {
            Ok(hunt_result) => {
                tracing::info!(
                    user_id = %user.user_id,
                    library_id = %library_id,
                    library_name = %library.name,
                    searched = hunt_result.searched,
                    matched = hunt_result.matched,
                    downloaded = hunt_result.downloaded,
                    "Manual auto-hunt completed"
                );

                Ok(AutoHuntResult {
                    success: true,
                    error: None,
                    searched: hunt_result.searched,
                    matched: hunt_result.matched,
                    downloaded: hunt_result.downloaded,
                    skipped: hunt_result.skipped,
                    failed: hunt_result.failed,
                })
            }
            Err(e) => Ok(AutoHuntResult {
                success: false,
                error: Some(e.to_string()),
                searched: 0,
                matched: 0,
                downloaded: 0,
                skipped: 0,
                failed: 0,
            }),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Download a .torrent file from a private tracker with authentication
async fn download_torrent_file_authenticated(
    url: &str,
    indexer_type: &str,
    credentials: &std::collections::HashMap<String, String>,
) -> anyhow::Result<Vec<u8>> {
    use anyhow::Context;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let mut request = client.get(url);

    // Add authentication based on indexer type
    match indexer_type {
        "iptorrents" => {
            // IPTorrents uses cookie-based authentication
            if let Some(cookie) = credentials.get("cookie") {
                request = request.header("Cookie", cookie);
            }
            if let Some(user_agent) = credentials.get("user_agent") {
                request = request.header("User-Agent", user_agent);
            } else {
                // Default user agent if not provided
                request = request.header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36");
            }
        }
        _ => {
            // Generic: try cookie auth if available
            if let Some(cookie) = credentials.get("cookie") {
                request = request.header("Cookie", cookie);
            }
            if let Some(api_key) = credentials.get("api_key") {
                // Some indexers use API key as query param
                request = request.query(&[("apikey", api_key)]);
            }
        }
    }

    let response = request
        .send()
        .await
        .context("Failed to send request")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to download .torrent file: HTTP {}",
            response.status()
        );
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read response body")?;

    // Verify it's actually a torrent file (starts with "d" for bencoded dict)
    if bytes.is_empty() || bytes[0] != b'd' {
        // Check if it might be an HTML error page
        let preview = String::from_utf8_lossy(&bytes[..std::cmp::min(200, bytes.len())]);
        if preview.contains("<!DOCTYPE") || preview.contains("<html") {
            anyhow::bail!("Received HTML instead of torrent file - authentication may have failed");
        }
        anyhow::bail!("Downloaded file does not appear to be a valid torrent");
    }

    Ok(bytes.to_vec())
}
