import type { TablerIcon } from '@tabler/icons-react';
import {
  IconMovie,
  IconDeviceTv,
  IconMusic,
  IconHeadphones,
  IconFolder,
} from '@tabler/icons-react';

// ============================================================================
// Library Type Helpers
// ============================================================================

/** Library type info for UI display */
export interface LibraryTypeInfo {
  value: LibraryType;
  label: string;
  Icon: TablerIcon;
  color: string;
}

/** Available library types with display info */
export const LIBRARY_TYPES: LibraryTypeInfo[] = [
  { value: 'MOVIES', label: 'Movies', Icon: IconMovie, color: 'purple' },
  { value: 'TV', label: 'TV Shows', Icon: IconDeviceTv, color: 'blue' },
  { value: 'MUSIC', label: 'Music', Icon: IconMusic, color: 'green' },
  { value: 'AUDIOBOOKS', label: 'Audiobooks', Icon: IconHeadphones, color: 'orange' },
  { value: 'OTHER', label: 'Other', Icon: IconFolder, color: 'slate' },
];

/** Get display info for a library type */
export const getLibraryTypeInfo = (type: LibraryType): LibraryTypeInfo =>
  LIBRARY_TYPES.find((t) => t.value === type) || LIBRARY_TYPES[4];

// ============================================================================
// Media Item Types (for legacy compatibility)
// ============================================================================

/** Media item for display */
export interface MediaItem {
  id: string;
  title: string;
  mediaType: 'movie' | 'episode';
  year: number | null;
  overview: string | null;
  runtime: number | null;
  posterUrl: string | null;
  backdropUrl: string | null;
}

// ============================================================================
// Subscription Types (legacy)
// ============================================================================

/** Subscription for RSS/auto-download */
export interface Subscription {
  id: string;
  name: string;
  tvdbId: number | null;
  tmdbId: number | null;
  qualityProfileId: string | null;
  monitored: boolean;
  lastCheckedAt: string | null;
  episodeCount: number;
}

// ============================================================================
// Torrent Types
// ============================================================================

export interface Torrent {
  id: number;
  infoHash: string;
  name: string;
  state: TorrentState;
  progress: number;
  progressPercent: number;
  size: number;
  sizeFormatted: string;
  downloaded: number;
  uploaded: number;
  downloadSpeed: number;
  downloadSpeedFormatted: string;
  uploadSpeed: number;
  uploadSpeedFormatted: string;
  peers: number;
  eta: number | null;
  addedAt: string | null;
}

export type TorrentState = 'QUEUED' | 'CHECKING' | 'DOWNLOADING' | 'SEEDING' | 'PAUSED' | 'ERROR';

export interface TorrentProgress {
  id: number;
  infoHash: string;
  progress: number;
  downloadSpeed: number;
  uploadSpeed: number;
  peers: number;
  state: TorrentState;
}

export interface AddTorrentResult {
  success: boolean;
  torrent: Torrent | null;
  error: string | null;
}

export interface TorrentActionResult {
  success: boolean;
  error: string | null;
}

export interface OrganizeTorrentResult {
  success: boolean;
  organizedCount: number;
  failedCount: number;
  messages: string[];
}

export interface PeerStats {
  queued: number;
  connecting: number;
  live: number;
  seen: number;
  dead: number;
  notNeeded: number;
}

export interface TorrentDetails {
  id: number;
  infoHash: string;
  name: string;
  state: TorrentState;
  progress: number;
  progressPercent: number;
  size: number;
  sizeFormatted: string;
  downloaded: number;
  downloadedFormatted: string;
  uploaded: number;
  uploadedFormatted: string;
  downloadSpeed: number;
  downloadSpeedFormatted: string;
  uploadSpeed: number;
  uploadSpeedFormatted: string;
  savePath: string;
  files: TorrentFileInfo[];
  pieceCount: number;
  piecesDownloaded: number;
  averagePieceDownloadMs: number | null;
  timeRemainingSecs: number | null;
  timeRemainingFormatted: string | null;
  peerStats: PeerStats;
  error: string | null;
  finished: boolean;
  ratio: number;
}

export interface TorrentFileInfo {
  index: number;
  path: string;
  size: number;
  progress: number;
}

// ============================================================================
// Settings Types
// ============================================================================

export interface TorrentSettings {
  downloadDir: string;
  sessionDir: string;
  enableDht: boolean;
  listenPort: number;
  maxConcurrent: number;
  uploadLimit: number;
  downloadLimit: number;
}

export interface SettingsResult {
  success: boolean;
  error: string | null;
}

// ============================================================================
// Filesystem Types
// ============================================================================

export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  sizeFormatted: string;
  readable: boolean;
  writable: boolean;
  mimeType: string | null;
  modifiedAt: string | null;
}

export interface QuickPath {
  name: string;
  path: string;
}

export interface BrowseDirectoryResult {
  currentPath: string;
  parentPath: string | null;
  entries: FileEntry[];
  quickPaths: QuickPath[];
  isLibraryPath: boolean;
  libraryId: string | null;
}

// Alias for backward compatibility with FolderBrowserInput
export interface BrowseResponse {
  currentPath: string;
  parentPath: string | null;
  entries: FileEntry[];
  quickPaths: QuickPath[];
}

export interface BrowseDirectoryInput {
  path?: string | null;
  dirsOnly?: boolean | null;
  showHidden?: boolean | null;
}

export interface FileOperationResult {
  success: boolean;
  error: string | null;
  affectedCount: number;
  messages: string[];
  path: string | null;
}

export interface CreateDirectoryInput {
  path: string;
}

export interface DeleteFilesInput {
  paths: string[];
  recursive?: boolean | null;
}

export interface CopyFilesInput {
  sources: string[];
  destination: string;
  overwrite?: boolean | null;
}

export interface MoveFilesInput {
  sources: string[];
  destination: string;
  overwrite?: boolean | null;
}

export interface RenameFileInput {
  path: string;
  newName: string;
}

export interface DirectoryChangeEvent {
  path: string;
  changeType: 'created' | 'modified' | 'deleted' | 'renamed';
  name: string | null;
  newName: string | null;
  timestamp: string;
}

export interface PathValidationResult {
  isValid: boolean;
  isLibraryPath: boolean;
  libraryId: string | null;
  libraryName: string | null;
  error: string | null;
}

// Raw response types from the backend (snake_case) - kept for legacy REST API
export interface RawFileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  readable: boolean;
  writable: boolean;
}

export interface RawBrowseResponse {
  current_path: string;
  parent_path: string | null;
  entries: RawFileEntry[];
  quick_paths: QuickPath[];
}

// ============================================================================
// Library Types
// ============================================================================

export type LibraryType = 'MOVIES' | 'TV' | 'MUSIC' | 'AUDIOBOOKS' | 'OTHER';
export type PostDownloadAction = 'COPY' | 'MOVE' | 'HARDLINK';

export interface Library {
  id: string;
  name: string;
  path: string;
  libraryType: LibraryType;
  icon: string;
  color: string;
  autoScan: boolean;
  scanIntervalMinutes: number;
  watchForChanges: boolean;
  postDownloadAction: PostDownloadAction;
  organizeFiles: boolean;
  renameStyle: string;
  namingPattern: string | null;
  defaultQualityProfileId: string | null;
  autoAddDiscovered: boolean;
  autoDownload: boolean;
  /** Automatically hunt for missing episodes using indexers */
  autoHunt: boolean;
  /** Whether a scan is currently in progress */
  scanning: boolean;
  itemCount: number;
  totalSizeBytes: number;
  showCount: number;
  lastScannedAt: string | null;
  // Inline quality settings (empty = any)
  /** Allowed resolutions: 2160p, 1080p, 720p, 480p. Empty = any. */
  allowedResolutions: string[];
  /** Allowed video codecs: hevc, h264, av1, xvid. Empty = any. */
  allowedVideoCodecs: string[];
  /** Allowed audio formats: atmos, truehd, dtshd, dts, dd51, aac. Empty = any. */
  allowedAudioFormats: string[];
  /** If true, only accept releases with HDR. */
  requireHdr: boolean;
  /** Allowed HDR types: hdr10, hdr10plus, dolbyvision, hlg. Empty with requireHdr=true = any HDR. */
  allowedHdrTypes: string[];
  /** Allowed sources: webdl, webrip, bluray, hdtv. Empty = any. */
  allowedSources: string[];
  /** Blacklisted release groups. */
  releaseGroupBlacklist: string[];
  /** Whitelisted release groups (if set, only allow these). */
  releaseGroupWhitelist: string[];
}

export interface LibraryResult {
  success: boolean;
  library: Library | null;
  error: string | null;
}

export interface CreateLibraryInput {
  name: string;
  path: string;
  libraryType: LibraryType;
  icon?: string;
  color?: string;
  autoScan?: boolean;
  scanIntervalMinutes?: number;
  watchForChanges?: boolean;
  postDownloadAction?: PostDownloadAction;
  organizeFiles?: boolean;
  namingPattern?: string;
  defaultQualityProfileId?: string;
  autoAddDiscovered?: boolean;
  // Inline quality settings
  allowedResolutions?: string[];
  allowedVideoCodecs?: string[];
  allowedAudioFormats?: string[];
  requireHdr?: boolean;
  allowedHdrTypes?: string[];
  allowedSources?: string[];
  releaseGroupBlacklist?: string[];
  releaseGroupWhitelist?: string[];
}

export interface UpdateLibraryInput {
  name?: string;
  path?: string;
  icon?: string;
  color?: string;
  autoScan?: boolean;
  scanIntervalMinutes?: number;
  watchForChanges?: boolean;
  postDownloadAction?: PostDownloadAction;
  organizeFiles?: boolean;
  namingPattern?: string;
  defaultQualityProfileId?: string | null;
  autoAddDiscovered?: boolean;
  autoDownload?: boolean;
  autoHunt?: boolean;
  // Inline quality settings
  allowedResolutions?: string[];
  allowedVideoCodecs?: string[];
  allowedAudioFormats?: string[];
  requireHdr?: boolean;
  allowedHdrTypes?: string[];
  allowedSources?: string[];
  releaseGroupBlacklist?: string[];
  releaseGroupWhitelist?: string[];
}

// ============================================================================
// TV Show Types
// ============================================================================

export type TvShowStatus = 'CONTINUING' | 'ENDED' | 'UPCOMING' | 'CANCELLED' | 'UNKNOWN';
export type MonitorType = 'ALL' | 'FUTURE' | 'NONE';
export type EpisodeStatus = 'MISSING' | 'WANTED' | 'AVAILABLE' | 'DOWNLOADING' | 'DOWNLOADED' | 'IGNORED';

export interface TvShow {
  id: string;
  libraryId: string;
  name: string;
  sortName: string | null;
  year: number | null;
  status: TvShowStatus;
  tvmazeId: number | null;
  tmdbId: number | null;
  tvdbId: number | null;
  imdbId: string | null;
  overview: string | null;
  network: string | null;
  runtime: number | null;
  genres: string[];
  posterUrl: string | null;
  backdropUrl: string | null;
  monitored: boolean;
  monitorType: MonitorType;
  qualityProfileId: string | null;
  path: string | null;
  /** Override library auto-download setting (null = inherit) */
  autoDownloadOverride: boolean | null;
  /** Whether to backfill existing episodes when added */
  backfillExisting: boolean;
  /** Override library organize_files setting (null = inherit) */
  organizeFilesOverride: boolean | null;
  /** Override library rename_style setting (null = inherit) */
  renameStyleOverride: string | null;
  /** Override library auto_hunt setting (null = inherit) */
  autoHuntOverride: boolean | null;
  episodeCount: number;
  episodeFileCount: number;
  sizeBytes: number;
  // Quality override settings (null = inherit from library)
  /** Override allowed resolutions (null = inherit) */
  allowedResolutionsOverride: string[] | null;
  /** Override allowed video codecs (null = inherit) */
  allowedVideoCodecsOverride: string[] | null;
  /** Override allowed audio formats (null = inherit) */
  allowedAudioFormatsOverride: string[] | null;
  /** Override HDR requirement (null = inherit) */
  requireHdrOverride: boolean | null;
  /** Override allowed HDR types (null = inherit) */
  allowedHdrTypesOverride: string[] | null;
  /** Override allowed sources (null = inherit) */
  allowedSourcesOverride: string[] | null;
  /** Override release group blacklist (null = inherit) */
  releaseGroupBlacklistOverride: string[] | null;
  /** Override release group whitelist (null = inherit) */
  releaseGroupWhitelistOverride: string[] | null;
}

export interface TvShowSearchResult {
  provider: string;
  providerId: number;
  name: string;
  year: number | null;
  status: string | null;
  network: string | null;
  overview: string | null;
  posterUrl: string | null;
  tvdbId: number | null;
  imdbId: string | null;
  score: number;
}

export interface Episode {
  id: string;
  tvShowId: string;
  season: number;
  episode: number;
  absoluteNumber: number | null;
  title: string | null;
  overview: string | null;
  airDate: string | null;
  runtime: number | null;
  status: EpisodeStatus;
  tvmazeId: number | null;
  tmdbId: number | null;
  tvdbId: number | null;
  /** URL/magnet link to download this episode (when status is 'available') */
  torrentLink: string | null;
  /** When the torrent link was found in RSS */
  torrentLinkAddedAt: string | null;
  /** Media file ID if episode has been downloaded (for playback) */
  mediaFileId: string | null;
}

export interface TvShowResult {
  success: boolean;
  tvShow: TvShow | null;
  error: string | null;
}

export interface DownloadEpisodeResult {
  success: boolean;
  episode: Episode | null;
  error: string | null;
}

export interface AddTvShowInput {
  provider: string;
  providerId: number;
  monitorType?: MonitorType;
  qualityProfileId?: string;
  path?: string;
}

export interface UpdateTvShowInput {
  monitored?: boolean;
  monitorType?: MonitorType;
  qualityProfileId?: string;
  path?: string;
  /** Override library auto-download (null = inherit, true/false = override) */
  autoDownloadOverride?: boolean | null;
  /** Whether to backfill existing episodes */
  backfillExisting?: boolean;
  /** Override library organize_files (null = inherit) */
  organizeFilesOverride?: boolean | null;
  /** Override library rename_style (null = inherit) */
  renameStyleOverride?: string | null;
  // Quality override settings (null = inherit, [] = any)
  /** Override allowed resolutions (null = inherit) */
  allowedResolutionsOverride?: string[] | null;
  /** Override allowed video codecs (null = inherit) */
  allowedVideoCodecsOverride?: string[] | null;
  /** Override allowed audio formats (null = inherit) */
  allowedAudioFormatsOverride?: string[] | null;
  /** Override HDR requirement (null = inherit) */
  requireHdrOverride?: boolean | null;
  /** Override allowed HDR types (null = inherit) */
  allowedHdrTypesOverride?: string[] | null;
  /** Override allowed sources (null = inherit) */
  allowedSourcesOverride?: string[] | null;
  /** Override release group blacklist (null = inherit) */
  releaseGroupBlacklistOverride?: string[] | null;
  /** Override release group whitelist (null = inherit) */
  releaseGroupWhitelistOverride?: string[] | null;
}

// ============================================================================
// Media File Types
// ============================================================================

export interface MediaFile {
  id: string;
  libraryId: string;
  path: string;
  relativePath: string | null;
  originalName: string | null;
  sizeBytes: number;
  sizeFormatted: string;
  container: string | null;
  videoCodec: string | null;
  audioCodec: string | null;
  resolution: string | null;
  isHdr: boolean | null;
  hdrType: string | null;
  width: number | null;
  height: number | null;
  duration: number | null;
  bitrate: number | null;
  episodeId: string | null;
  organized: boolean;
  addedAt: string;
}

// ============================================================================
// Quality Profile Types
// ============================================================================

export interface QualityProfile {
  id: string;
  name: string;
  preferredResolution: string | null;
  minResolution: string | null;
  preferredCodec: string | null;
  preferredAudio: string | null;
  requireHdr: boolean;
  hdrTypes: string[];
  preferredLanguage: string | null;
  maxSizeGb: number | null;
  minSeeders: number | null;
  releaseGroupWhitelist: string[];
  releaseGroupBlacklist: string[];
  upgradeUntil: string | null;
}

// ============================================================================
// RSS Feed Types
// ============================================================================

export interface RssFeed {
  id: string;
  libraryId: string | null;
  name: string;
  url: string;
  enabled: boolean;
  pollIntervalMinutes: number;
  lastPolledAt: string | null;
  lastSuccessfulAt: string | null;
  lastError: string | null;
  consecutiveFailures: number;
}

export interface RssFeedResult {
  success: boolean;
  rssFeed: RssFeed | null;
  error: string | null;
}

export interface RssItem {
  title: string;
  link: string;
  pubDate: string | null;
  description: string | null;
  parsedShowName: string | null;
  parsedSeason: number | null;
  parsedEpisode: number | null;
  parsedResolution: string | null;
  parsedCodec: string | null;
}

export interface RssFeedTestResult {
  success: boolean;
  itemCount: number;
  sampleItems: RssItem[];
  error: string | null;
}

export interface CreateRssFeedInput {
  libraryId?: string;
  name: string;
  url: string;
  enabled?: boolean;
  pollIntervalMinutes?: number;
}

export interface UpdateRssFeedInput {
  libraryId?: string;
  name?: string;
  url?: string;
  enabled?: boolean;
  pollIntervalMinutes?: number;
}

// ============================================================================
// Parse and Identify Types
// ============================================================================

export interface ParsedEpisodeInfo {
  originalTitle: string;
  showName: string | null;
  season: number | null;
  episode: number | null;
  year: number | null;
  date: string | null;
  resolution: string | null;
  source: string | null;
  codec: string | null;
  hdr: string | null;
  audio: string | null;
  releaseGroup: string | null;
  isProper: boolean;
  isRepack: boolean;
}

export interface ParseAndIdentifyResult {
  parsed: ParsedEpisodeInfo;
  matches: TvShowSearchResult[];
}

// ============================================================================
// Log Types
// ============================================================================

export type LogLevel = 'TRACE' | 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';

export interface LogEntry {
  id: string;
  timestamp: string;
  level: LogLevel;
  target: string;
  message: string;
  fields: Record<string, unknown> | null;
  spanName: string | null;
}

export interface PaginatedLogResult {
  logs: LogEntry[];
  totalCount: number;
  hasMore: boolean;
  nextCursor: string | null;
}

export interface LogFilterInput {
  levels?: LogLevel[];
  target?: string;
  keyword?: string;
  fromTimestamp?: string;
  toTimestamp?: string;
}

export interface LogStats {
  traceCount: number;
  debugCount: number;
  infoCount: number;
  warnCount: number;
  errorCount: number;
  totalCount: number;
}

export interface ClearLogsResult {
  success: boolean;
  deletedCount: number;
  error: string | null;
}

export interface LogEventSubscription {
  timestamp: string;
  level: LogLevel;
  target: string;
  message: string;
  fields: Record<string, unknown> | null;
  spanName: string | null;
}

// ============================================================================
// Upcoming Episode Types (for home page)
// ============================================================================

/** Show information embedded in upcoming episode */
export interface UpcomingEpisodeShow {
  tvmazeId: number;
  name: string;
  network: string | null;
  posterUrl: string | null;
  genres: string[];
}

/** An upcoming episode from TVMaze with show info */
export interface UpcomingEpisode {
  tvmazeId: number;
  name: string;
  season: number;
  episode: number;
  airDate: string;
  airTime: string | null;
  airStamp: string | null;
  runtime: number | null;
  summary: string | null;
  episodeImageUrl: string | null;
  show: UpcomingEpisodeShow;
}

/** Show information for library upcoming episodes */
export interface LibraryUpcomingShow {
  id: string;
  name: string;
  year: number | null;
  network: string | null;
  posterUrl: string | null;
  libraryId: string;
}

/** An upcoming episode from the user's library */
export interface LibraryUpcomingEpisode {
  id: string;
  tvmazeId: number | null;
  name: string | null;
  season: number;
  episode: number;
  airDate: string;
  status: EpisodeStatus;
  show: LibraryUpcomingShow;
}

// ============================================================================
// Indexer Types
// ============================================================================

/** An indexer configuration */
export interface IndexerConfig {
  id: string;
  indexerType: string;
  name: string;
  enabled: boolean;
  priority: number;
  siteUrl: string | null;
  isHealthy: boolean;
  lastError: string | null;
  errorCount: number;
  lastSuccessAt: string | null;
  createdAt: string;
  updatedAt: string;
  capabilities: IndexerCapabilities;
}

/** Indexer capabilities */
export interface IndexerCapabilities {
  supportsSearch: boolean;
  supportsTvSearch: boolean;
  supportsMovieSearch: boolean;
  supportsMusicSearch: boolean;
  supportsBookSearch: boolean;
  supportsImdbSearch: boolean;
  supportsTvdbSearch: boolean;
}

/** Information about an available indexer type */
export interface IndexerTypeInfo {
  id: string;
  name: string;
  description: string;
  trackerType: string;
  language: string;
  siteLink: string;
  requiredCredentials: string[];
  isNative: boolean;
}

/** A setting definition for an indexer */
export interface IndexerSettingDefinition {
  key: string;
  label: string;
  settingType: 'text' | 'password' | 'checkbox' | 'select';
  defaultValue: string | null;
  options: IndexerSettingOption[] | null;
}

/** An option for a select setting */
export interface IndexerSettingOption {
  value: string;
  label: string;
}

/** Input for creating an indexer */
export interface CreateIndexerInput {
  indexerType: string;
  name: string;
  siteUrl?: string | null;
  credentials: IndexerCredentialInput[];
  settings: IndexerSettingInput[];
}

/** Input for updating an indexer */
export interface UpdateIndexerInput {
  name?: string | null;
  enabled?: boolean | null;
  priority?: number | null;
  siteUrl?: string | null;
  credentials?: IndexerCredentialInput[] | null;
  settings?: IndexerSettingInput[] | null;
}

/** Input for a credential */
export interface IndexerCredentialInput {
  credentialType: string;
  value: string;
}

/** Input for a setting */
export interface IndexerSettingInput {
  key: string;
  value: string;
}

/** Result of an indexer mutation */
export interface IndexerResult {
  success: boolean;
  error: string | null;
  indexer: IndexerConfig | null;
}

/** Result of testing an indexer */
export interface IndexerTestResult {
  success: boolean;
  error: string | null;
  releasesFound: number | null;
  elapsedMs: number | null;
}

/** Input for searching indexers */
export interface IndexerSearchInput {
  query: string;
  indexerIds?: string[] | null;
  categories?: number[] | null;
  season?: number | null;
  episode?: string | null;
  imdbId?: string | null;
  limit?: number | null;
}

/** Result of an indexer search */
export interface IndexerSearchResultSet {
  indexers: IndexerSearchResultItem[];
  totalReleases: number;
  totalElapsedMs: number;
}

/** Search results from a single indexer */
export interface IndexerSearchResultItem {
  indexerId: string;
  indexerName: string;
  releases: TorrentRelease[];
  elapsedMs: number;
  fromCache: boolean;
  error: string | null;
}

/** A torrent release from an indexer search */
export interface TorrentRelease {
  title: string;
  guid: string;
  link: string | null;
  magnetUri: string | null;
  infoHash: string | null;
  details: string | null;
  publishDate: string;
  categories: number[];
  size: number | null;
  sizeFormatted: string | null;
  seeders: number | null;
  leechers: number | null;
  peers: number | null;
  grabs: number | null;
  isFreeleech: boolean;
  imdbId: string | null;
  poster: string | null;
  description: string | null;
  indexerId: string | null;
  indexerName: string | null;
}

// ============================================================================
// Security Settings Types
// ============================================================================

/** Security settings */
export interface SecuritySettings {
  encryptionKeySet: boolean;
  encryptionKeyPreview: string | null;
  encryptionKeyLastModified: string | null;
}

/** Result of security settings operation */
export interface SecuritySettingsResult {
  success: boolean;
  error: string | null;
  settings: SecuritySettings | null;
}

/** Input for generating encryption key */
export interface GenerateEncryptionKeyInput {
  confirmInvalidation: boolean;
}

// ============================================================================
// Cast Types (Chromecast / Media Casting)
// ============================================================================

/** Cast device types */
export type CastDeviceType = 'CHROMECAST' | 'CHROMECAST_AUDIO' | 'GOOGLE_HOME' | 'GOOGLE_NEST_HUB' | 'ANDROID_TV' | 'UNKNOWN';

/** Cast player states */
export type CastPlayerState = 'IDLE' | 'BUFFERING' | 'PLAYING' | 'PAUSED';

/** A discovered or saved cast device */
export interface CastDevice {
  id: string;
  name: string;
  address: string;
  port: number;
  model: string | null;
  deviceType: CastDeviceType;
  isFavorite: boolean;
  isManual: boolean;
  isConnected: boolean;
  lastSeenAt: string | null;
}

/** An active cast session */
export interface CastSession {
  id: string;
  deviceId: string | null;
  deviceName: string | null;
  mediaFileId: string | null;
  episodeId: string | null;
  streamUrl: string;
  playerState: CastPlayerState;
  currentTime: number;
  duration: number | null;
  volume: number;
  isMuted: boolean;
  startedAt: string;
}

/** Cast settings (global configuration) */
export interface CastSettings {
  autoDiscoveryEnabled: boolean;
  discoveryIntervalSeconds: number;
  defaultVolume: number;
  transcodeIncompatible: boolean;
  preferredQuality: string | null;
}

/** Input for adding a cast device manually */
export interface AddCastDeviceInput {
  address: string;
  port?: number;
  name?: string;
}

/** Input for updating a cast device */
export interface UpdateCastDeviceInput {
  name?: string;
  isFavorite?: boolean;
}

/** Input for casting media to a device */
export interface CastMediaInput {
  deviceId: string;
  mediaFileId: string;
  episodeId?: string;
  startPosition?: number;
}

/** Input for updating cast settings */
export interface UpdateCastSettingsInput {
  autoDiscoveryEnabled?: boolean;
  discoveryIntervalSeconds?: number;
  defaultVolume?: number;
  transcodeIncompatible?: boolean;
  preferredQuality?: string;
}

/** Result of a cast device mutation */
export interface CastDeviceResult {
  success: boolean;
  device: CastDevice | null;
  error: string | null;
}

/** Result of a cast session mutation */
export interface CastSessionResult {
  success: boolean;
  session: CastSession | null;
  error: string | null;
}

/** Result of cast settings mutation */
export interface CastSettingsResult {
  success: boolean;
  settings: CastSettings | null;
  error: string | null;
}
