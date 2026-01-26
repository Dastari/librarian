// Re-export everything from sub-modules for easy importing

// Client
export { apolloClient, graphqlClient, onGraphQLError } from "./client";

// Codegen node types (use these + PascalCase everywhere)
export type { LibraryNode, ShowNode, ScheduleCacheNode } from "./codegen-nodes";

// Types
export type {
  // Auth Types
  AuthResult,
  LogoutResult,
  AuthUserInfo,
  LoginInput,
  RegisterInput,
  // Library Type Helpers
  LibraryTypeInfo,
  // Media Item (legacy)
  MediaItem,
  // Torrent
  Torrent,
  TorrentState,
  TorrentProgress,
  ActiveDownloadCount,
  AddTorrentResult,
  TorrentActionResult,
  OrganizeTorrentResult,
  TorrentDetails,
  TorrentFileInfo,
  PendingFileMatch,
  RematchSourceResult,
  ProcessSourceResult,
  SetMatchResult,
  RemoveMatchResult,
  QualityStatus,
  DownloadStatus,
  PeerStats,
  // Settings
  TorrentSettings,
  SettingsResult,
  // UPnP and Port Testing
  UpnpResult,
  PortTestResult,
  // Filesystem
  FileEntry,
  QuickPath,
  BrowseResponse,
  BrowseDirectoryResult,
  BrowseDirectoryInput,
  FileOperationResult,
  CreateDirectoryInput,
  DeleteFilesInput,
  CopyFilesInput,
  MoveFilesInput,
  RenameFileInput,
  DirectoryChangeEvent,
  MediaFileUpdatedEvent,
  PathValidationResult,
  RawFileEntry,
  RawBrowseResponse,
  // Library
  LibraryType,
  PostDownloadAction,
  Library,
  LibraryResult,
  LibraryChangeType,
  LibraryChangedEvent,
  CreateLibraryInput,
  UpdateLibraryInput,
  // TV Show
  TvShowStatus,
  MonitorType,
  TvShow,
  TvShowSearchResult,
  Episode,
  TvShowResult,
  AddTvShowInput,
  UpdateTvShowInput,
  DownloadEpisodeResult,
  // Movie
  MovieStatus,
  Movie,
  MovieSearchResult,
  MovieResult,
  AddMovieInput,
  UpdateMovieInput,
  // Album/Music
  Artist,
  Album,
  AlbumSearchResult,
  AlbumResult,
  AddAlbumInput,
  Track,
  TrackStatus,
  TrackWithStatus,
  AlbumWithTracks,
  // Auto-Hunt
  AutoHuntResult,
  // Audiobook
  AudiobookAuthor,
  Audiobook,
  AudiobookSearchResult,
  AudiobookResult,
  AddAudiobookInput,
  AudiobookChapter,
  ChapterStatus,
  AudiobookWithChapters,
  // Naming Patterns
  NamingPattern,
  CreateNamingPatternInput,
  NamingPatternResult,
  // RSS Feed
  RssFeed,
  RssFeedResult,
  RssItem,
  RssFeedTestResult,
  CreateRssFeedInput,
  UpdateRssFeedInput,
  // Parse and Identify
  ParsedEpisodeInfo,
  ParseAndIdentifyResult,
  // Logs
  LogLevel,
  LogEntry,
  PaginatedLogResult,
  LogFilterInput,
  LogStats,
  ClearLogsResult,
  LogEventSubscription,
  // Upcoming Episodes
  UpcomingEpisode,
  UpcomingEpisodeShow,
  LibraryUpcomingEpisode,
  LibraryUpcomingShow,
  // Media Files
  MediaFile,
  MediaFileDetails,
  ManualMatchResult,
  EmbeddedMetadata,
  VideoStreamInfo,
  AudioStreamInfo,
  SubtitleInfo,
  ChapterInfo,
  // Cast Types
  CastDeviceType,
  CastPlayerState,
  CastDevice,
  CastSession,
  CastSettings,
  AddCastDeviceInput,
  UpdateCastDeviceInput,
  CastMediaInput,
  UpdateCastSettingsInput,
  CastDeviceResult,
  CastSessionResult,
  CastSettingsResult,
  // Playback Session Types
  PlaybackContentType,
  PlaybackSession,
  PlaybackSettings,
  StartPlaybackInput,
  UpdatePlaybackInput,
  UpdatePlaybackSettingsInput,
  PlaybackResult,
  // Indexer Search Types
  IndexerSearchInput,
  IndexerSearchResultSet,
  IndexerSearchResultItem,
  TorrentRelease,
  IndexerConfig,
  // LLM Parser Types
  LlmParserSettings,
  OllamaConnectionResult,
  FilenameParseResult,
  TestFilenameParserResult,
  // Notification Types
  NotificationType,
  NotificationCategory,
  NotificationActionType,
  NotificationResolution,
  NotificationEventType,
  Notification,
  PaginatedNotifications,
  NotificationCounts,
  NotificationFilterInput,
  ResolveNotificationInput,
  NotificationResult,
  MarkAllReadResult,
  NotificationEvent,
  // Content Download Progress Types
  ContentDownloadType,
  ContentDownloadProgressEvent,
} from "./types";

// Constants
export { LIBRARY_TYPES, getLibraryTypeInfo } from "./types";

// Auth and app: generated TypedDocumentNodes (prefer importing from ./generated/graphql)
export {
  NeedsSetupDocument,
  MeDocument,
  LoginDocument,
  RegisterDocument,
  RefreshTokenDocument,
  LogoutDocument,
  PlaybackSyncIntervalDocument,
  UpdateAppSettingDocument,
  PlaybackSessionsDocument,
  ActiveDownloadCountDocument,
  TorrentChangedDocument,
} from "./generated/graphql";

// Queries
export {
  // Torrent Queries
  TORRENTS_QUERY,
  TORRENT_QUERY,
  TORRENT_DETAILS_QUERY,
  PENDING_FILE_MATCHES_QUERY,
  ACTIVE_DOWNLOAD_COUNT_QUERY,
  TORRENT_SETTINGS_QUERY,
  UPnP_STATUS_QUERY,
  TEST_PORT_ACCESSIBILITY_QUERY,
  LIBRARIES_QUERY,
  LIBRARY_QUERY,
  ALL_TV_SHOWS_QUERY,
  TV_SHOWS_QUERY,
  TV_SHOWS_CONNECTION_QUERY,
  TV_SHOW_QUERY,
  SEARCH_TV_SHOWS_QUERY,
  // Movie Queries
  ALL_MOVIES_QUERY,
  MOVIES_QUERY,
  MOVIES_CONNECTION_QUERY,
  MOVIE_QUERY,
  SEARCH_MOVIES_QUERY,
  // Album/Music Queries
  ALBUMS_QUERY,
  ALBUMS_CONNECTION_QUERY,
  ALBUM_QUERY,
  ALBUM_WITH_TRACKS_QUERY,
  TRACKS_QUERY,
  TRACKS_CONNECTION_QUERY,
  ARTISTS_QUERY,
  ARTISTS_CONNECTION_QUERY,
  SEARCH_ALBUMS_QUERY,
  // Audiobook Queries
  AUDIOBOOKS_QUERY,
  AUDIOBOOKS_CONNECTION_QUERY,
  AUDIOBOOK_QUERY,
  AUDIOBOOK_WITH_CHAPTERS_QUERY,
  AUDIOBOOK_CHAPTERS_QUERY,
  AUDIOBOOK_AUTHORS_QUERY,
  AUDIOBOOK_AUTHORS_CONNECTION_QUERY,
  SEARCH_AUDIOBOOKS_QUERY,
  EPISODES_QUERY,
  WANTED_EPISODES_QUERY,
  NAMING_PATTERNS_QUERY,
  RSS_FEEDS_QUERY,
  PARSE_AND_IDENTIFY_QUERY,
  LOGS_QUERY,
  LOG_TARGETS_QUERY,
  LOG_STATS_QUERY,
  UPCOMING_EPISODES_QUERY,
  LIBRARY_UPCOMING_EPISODES_QUERY,
  UNMATCHED_FILES_QUERY,
  UNMATCHED_FILES_COUNT_QUERY,
  MEDIA_FILE_BY_PATH_QUERY,
  MOVIE_MEDIA_FILE_QUERY,
  MEDIA_FILE_DETAILS_QUERY,
  // Cast Queries
  CAST_DEVICES_QUERY,
  CAST_DEVICE_QUERY,
  CAST_SESSIONS_QUERY,
  CAST_SESSION_QUERY,
  CAST_SETTINGS_QUERY,
  // Filesystem Queries
  BROWSE_DIRECTORY_QUERY,
  QUICK_PATHS_QUERY,
  VALIDATE_PATH_QUERY,
  // Playback Queries
  PLAYBACK_SESSION_QUERY,
  PLAYBACK_SETTINGS_QUERY,
  // Indexer Search Queries
  SEARCH_INDEXERS_QUERY,
  INDEXER_CONFIGS_QUERY,
  // LLM Parser Queries
  LLM_PARSER_SETTINGS_QUERY,
  // Notification Queries
  NOTIFICATIONS_QUERY,
  RECENT_NOTIFICATIONS_QUERY,
  NOTIFICATION_COUNTS_QUERY,
  UNREAD_NOTIFICATION_COUNT_QUERY,
} from "./queries";

// Mutations
export {
  // Torrent Mutations
  ADD_TORRENT_MUTATION,
  PAUSE_TORRENT_MUTATION,
  RESUME_TORRENT_MUTATION,
  REMOVE_TORRENT_MUTATION,
  ORGANIZE_TORRENT_MUTATION,
  // File Match Mutations (Source-Agnostic)
  REMATCH_SOURCE_MUTATION,
  PROCESS_SOURCE_MUTATION,
  SET_MATCH_MUTATION,
  REMOVE_MATCH_MUTATION,
  UPDATE_TORRENT_SETTINGS_MUTATION,
  ATTEMPT_UPNP_PORT_FORWARDING_MUTATION,
  CREATE_LIBRARY_MUTATION,
  UPDATE_LIBRARY_MUTATION,
  DELETE_LIBRARY_MUTATION,
  SCAN_LIBRARY_MUTATION,
  CONSOLIDATE_LIBRARY_MUTATION,
  ADD_TV_SHOW_MUTATION,
  DELETE_TV_SHOW_MUTATION,
  REFRESH_TV_SHOW_MUTATION,
  UPDATE_TV_SHOW_MUTATION,
  // Movie Mutations
  ADD_MOVIE_MUTATION,
  UPDATE_MOVIE_MUTATION,
  DELETE_MOVIE_MUTATION,
  REFRESH_MOVIE_MUTATION,
  // Album Mutations
  ADD_ALBUM_MUTATION,
  DELETE_ALBUM_MUTATION,
  // Audiobook Mutations
  ADD_AUDIOBOOK_MUTATION,
  DELETE_AUDIOBOOK_MUTATION,
  CREATE_RSS_FEED_MUTATION,
  UPDATE_RSS_FEED_MUTATION,
  DELETE_RSS_FEED_MUTATION,
  TEST_RSS_FEED_MUTATION,
  POLL_RSS_FEED_MUTATION,
  DOWNLOAD_EPISODE_MUTATION,
  // Naming Pattern Mutations
  CREATE_NAMING_PATTERN_MUTATION,
  UPDATE_NAMING_PATTERN_MUTATION,
  DELETE_NAMING_PATTERN_MUTATION,
  SET_DEFAULT_NAMING_PATTERN_MUTATION,
  CLEAR_ALL_LOGS_MUTATION,
  CLEAR_OLD_LOGS_MUTATION,
  // Cast Mutations
  DISCOVER_CAST_DEVICES_MUTATION,
  ADD_CAST_DEVICE_MUTATION,
  UPDATE_CAST_DEVICE_MUTATION,
  REMOVE_CAST_DEVICE_MUTATION,
  CAST_MEDIA_MUTATION,
  CAST_PLAY_MUTATION,
  CAST_PAUSE_MUTATION,
  CAST_STOP_MUTATION,
  CAST_SEEK_MUTATION,
  CAST_SET_VOLUME_MUTATION,
  CAST_SET_MUTED_MUTATION,
  UPDATE_CAST_SETTINGS_MUTATION,
  // Filesystem Mutations
  CREATE_DIRECTORY_MUTATION,
  DELETE_FILES_MUTATION,
  COPY_FILES_MUTATION,
  MOVE_FILES_MUTATION,
  RENAME_FILE_MUTATION,
  // Playback Mutations
  START_PLAYBACK_MUTATION,
  UPDATE_PLAYBACK_MUTATION,
  STOP_PLAYBACK_MUTATION,
  UPDATE_PLAYBACK_SETTINGS_MUTATION,
  // Auto-Hunt Mutations
  TRIGGER_AUTO_HUNT_MUTATION,
  // Notification Mutations
  MARK_NOTIFICATION_READ_MUTATION,
  MARK_ALL_NOTIFICATIONS_READ_MUTATION,
  RESOLVE_NOTIFICATION_MUTATION,
  RESOLVE_NOTIFICATION_WITH_ACTION_MUTATION,
  DELETE_NOTIFICATION_MUTATION,
  // Manual Match Mutations
  MANUAL_MATCH_MUTATION,
  UNMATCH_MEDIA_FILE_MUTATION,
} from "./mutations";

// Subscriptions
export {
  TORRENT_PROGRESS_SUBSCRIPTION,
  TORRENT_ADDED_SUBSCRIPTION,
  TORRENT_COMPLETED_SUBSCRIPTION,
  TORRENT_REMOVED_SUBSCRIPTION,
  ACTIVE_DOWNLOAD_COUNT_SUBSCRIPTION,
  LOG_EVENTS_SUBSCRIPTION,
  ERROR_LOGS_SUBSCRIPTION,
  // Library Subscriptions
  LIBRARY_CHANGED_SUBSCRIPTION,
  // Media File Subscriptions
  MEDIA_FILE_UPDATED_SUBSCRIPTION,
  // Filesystem Subscriptions
  DIRECTORY_CONTENTS_CHANGED_SUBSCRIPTION,
  // Notification Subscriptions
  NOTIFICATION_RECEIVED_SUBSCRIPTION,
  NOTIFICATION_COUNTS_SUBSCRIPTION,
  // Content Download Progress Subscriptions
  CONTENT_DOWNLOAD_PROGRESS_SUBSCRIPTION,
} from "./subscriptions";

// GraphQL-based filesystem functions (replaces REST API)
export {
  browseDirectory,
  createDirectory,
  deleteFiles,
  copyFiles,
  moveFiles,
  renameFile,
} from "./filesystem";
