// Re-export everything from sub-modules for easy importing

// Client
export { apolloClient, graphqlClient, onGraphQLError } from './client';

// Types
export type {
  // Library Type Helpers
  LibraryTypeInfo,
  // Media Item (legacy)
  MediaItem,
  // Subscription (legacy)
  Subscription,
  // Torrent
  Torrent,
  TorrentState,
  TorrentProgress,
  AddTorrentResult,
  TorrentActionResult,
  OrganizeTorrentResult,
  TorrentDetails,
  TorrentFileInfo,
  PeerStats,
  // Settings
  TorrentSettings,
  SettingsResult,
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
  EpisodeStatus,
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
  // Quality Profile
  QualityProfile,
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
  // Security Settings
  SecuritySettings,
  SecuritySettingsResult,
  GenerateEncryptionKeyInput,
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
  PlaybackSession,
  StartPlaybackInput,
  UpdatePlaybackInput,
  PlaybackResult,
  // Indexer Search Types
  IndexerSearchInput,
  IndexerSearchResultSet,
  IndexerSearchResultItem,
  TorrentRelease,
  IndexerConfig,
} from './types';

// Constants
export { LIBRARY_TYPES, getLibraryTypeInfo } from './types';

// Queries
export {
  TORRENTS_QUERY,
  TORRENT_QUERY,
  TORRENT_DETAILS_QUERY,
  TORRENT_SETTINGS_QUERY,
  LIBRARIES_QUERY,
  LIBRARY_QUERY,
  ALL_TV_SHOWS_QUERY,
  TV_SHOWS_QUERY,
  TV_SHOW_QUERY,
  SEARCH_TV_SHOWS_QUERY,
  // Movie Queries
  ALL_MOVIES_QUERY,
  MOVIES_QUERY,
  MOVIE_QUERY,
  SEARCH_MOVIES_QUERY,
  EPISODES_QUERY,
  WANTED_EPISODES_QUERY,
  QUALITY_PROFILES_QUERY,
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
  SECURITY_SETTINGS_QUERY,
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
  // Indexer Search Queries
  SEARCH_INDEXERS_QUERY,
  INDEXER_CONFIGS_QUERY,
} from './queries';

// Mutations
export {
  ADD_TORRENT_MUTATION,
  PAUSE_TORRENT_MUTATION,
  RESUME_TORRENT_MUTATION,
  REMOVE_TORRENT_MUTATION,
  ORGANIZE_TORRENT_MUTATION,
  UPDATE_TORRENT_SETTINGS_MUTATION,
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
  CREATE_RSS_FEED_MUTATION,
  UPDATE_RSS_FEED_MUTATION,
  DELETE_RSS_FEED_MUTATION,
  TEST_RSS_FEED_MUTATION,
  POLL_RSS_FEED_MUTATION,
  DOWNLOAD_EPISODE_MUTATION,
  // Naming Pattern Mutations
  CREATE_NAMING_PATTERN_MUTATION,
  DELETE_NAMING_PATTERN_MUTATION,
  SET_DEFAULT_NAMING_PATTERN_MUTATION,
  CLEAR_ALL_LOGS_MUTATION,
  CLEAR_OLD_LOGS_MUTATION,
  INITIALIZE_ENCRYPTION_KEY_MUTATION,
  REGENERATE_ENCRYPTION_KEY_MUTATION,
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
} from './mutations';

// Subscriptions
export {
  TORRENT_PROGRESS_SUBSCRIPTION,
  TORRENT_ADDED_SUBSCRIPTION,
  TORRENT_COMPLETED_SUBSCRIPTION,
  TORRENT_REMOVED_SUBSCRIPTION,
  LOG_EVENTS_SUBSCRIPTION,
  ERROR_LOGS_SUBSCRIPTION,
  // Library Subscriptions
  LIBRARY_CHANGED_SUBSCRIPTION,
  // Filesystem Subscriptions
  DIRECTORY_CONTENTS_CHANGED_SUBSCRIPTION,
} from './subscriptions';

// GraphQL-based filesystem functions (replaces REST API)
export { browseDirectory, createDirectory } from './filesystem';
