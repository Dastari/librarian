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
  RawFileEntry,
  RawBrowseResponse,
  // Library
  LibraryType,
  PostDownloadAction,
  Library,
  LibraryResult,
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
  // Quality Profile
  QualityProfile,
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
  TV_SHOWS_QUERY,
  TV_SHOW_QUERY,
  SEARCH_TV_SHOWS_QUERY,
  EPISODES_QUERY,
  WANTED_EPISODES_QUERY,
  QUALITY_PROFILES_QUERY,
  RSS_FEEDS_QUERY,
  PARSE_AND_IDENTIFY_QUERY,
  LOGS_QUERY,
  LOG_TARGETS_QUERY,
  LOG_STATS_QUERY,
  UPCOMING_EPISODES_QUERY,
  LIBRARY_UPCOMING_EPISODES_QUERY,
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
  ADD_TV_SHOW_MUTATION,
  DELETE_TV_SHOW_MUTATION,
  REFRESH_TV_SHOW_MUTATION,
  UPDATE_TV_SHOW_MUTATION,
  CREATE_RSS_FEED_MUTATION,
  UPDATE_RSS_FEED_MUTATION,
  DELETE_RSS_FEED_MUTATION,
  TEST_RSS_FEED_MUTATION,
  POLL_RSS_FEED_MUTATION,
  DOWNLOAD_EPISODE_MUTATION,
  CLEAR_ALL_LOGS_MUTATION,
  CLEAR_OLD_LOGS_MUTATION,
} from './mutations';

// Subscriptions
export {
  TORRENT_PROGRESS_SUBSCRIPTION,
  TORRENT_ADDED_SUBSCRIPTION,
  TORRENT_COMPLETED_SUBSCRIPTION,
  TORRENT_REMOVED_SUBSCRIPTION,
  LOG_EVENTS_SUBSCRIPTION,
  ERROR_LOGS_SUBSCRIPTION,
} from './subscriptions';

// REST API
export { browseDirectory, createDirectory } from './api';
