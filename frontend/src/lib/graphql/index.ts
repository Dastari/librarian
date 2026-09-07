// Re-export everything from sub-modules for easy importing

// Client
export { apolloClient, graphqlClient, onGraphQLError } from "./client";

// Library types from generated (source of truth)
export type {
  Library,
  LibraryResult,
  LibraryChangedEvent,
  CreateLibraryInput,
  UpdateLibraryInput,
  ChangeAction,
  BrowseDirectoryEntry,
  BrowseQuickPath,
} from "./generated/graphql";

// Node types derived from query results (for components that need typed query results)
export type { LibraryNode, ScheduleCacheNode } from "../../hooks/useDashboardCache";

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
  DownloadsTorrentRow,
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
  // Filesystem (BrowseDirectoryResult from filesystem; entry/quick path from generated)
  FileEntry,
  QuickPath,
  BrowseResponse,
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
  // Library (Library, LibraryResult, etc. from generated/graphql above)
  LibraryType,
  PostDownloadAction,
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
  // Playback Session Types
  PlaybackContentType,
  PlaybackSession,
  PlaybackSettings,
  StartPlaybackInput,
  UpdatePlaybackInput,
  UpdatePlaybackSettingsInput,
  PlaybackResult,
  // LLM Parser Types
  LlmParserSettings,
  OllamaConnectionResult,
  FilenameParseResult,
  TestFilenameParserResult,
  // Content Download Progress Types
  ContentDownloadType,
  ContentDownloadProgressEvent,
} from "./types";

// Constants and helpers
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
  TorrentProgressDocument,
} from "./generated/graphql";

// GraphQL-based filesystem functions (replaces REST API)
export {
  browseDirectory,
  getFilesystemRuntimeInfo,
  getLibraryPathAvailability,
  configureNetworkPath,
  reconnectLibraryPath,
  createDirectory,
  deleteFiles,
  copyFiles,
  moveFiles,
  renameFile,
} from "./filesystem";
export type {
  BrowseDirectoryResult,
  RuntimeFilesystemInfo,
  LibraryPathAvailabilityStatus,
} from "./filesystem";
