// ============================================================================
// Library Type Helpers
// ============================================================================

/** Library type info for UI display */
export interface LibraryTypeInfo {
  value: LibraryType;
  label: string;
  icon: string;
  color: string;
}

/** Available library types with display info */
export const LIBRARY_TYPES: LibraryTypeInfo[] = [
  { value: 'MOVIES', label: 'Movies', icon: '🎬', color: 'purple' },
  { value: 'TV', label: 'TV Shows', icon: '📺', color: 'blue' },
  { value: 'MUSIC', label: 'Music', icon: '🎵', color: 'green' },
  { value: 'AUDIOBOOKS', label: 'Audiobooks', icon: '🎧', color: 'orange' },
  { value: 'OTHER', label: 'Other', icon: '📁', color: 'slate' },
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
  readable: boolean;
  writable: boolean;
}

export interface QuickPath {
  name: string;
  path: string;
}

export interface BrowseResponse {
  currentPath: string;
  parentPath: string | null;
  entries: FileEntry[];
  quickPaths: QuickPath[];
}

// Raw response types from the backend (snake_case)
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
