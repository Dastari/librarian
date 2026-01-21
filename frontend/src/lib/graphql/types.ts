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

/** A match between a file in a torrent and a library item */
export interface TorrentFileMatch {
  id: string;
  torrentId: string;
  fileIndex: number;
  filePath: string;
  fileSize: number;
  episodeId: string | null;
  movieId: string | null;
  trackId: string | null;
  chapterId: string | null;
  matchType: 'auto' | 'manual' | 'forced';
  matchConfidence: number | null;
  parsedResolution: string | null;
  parsedCodec: string | null;
  parsedSource: string | null;
  parsedAudio: string | null;
  skipDownload: boolean;
  processed: boolean;
  processedAt: string | null;
  mediaFileId: string | null;
  errorMessage: string | null;
  createdAt: string;
}

/** Quality status of a media file */
export type QualityStatus = 'UNKNOWN' | 'OPTIMAL' | 'SUBOPTIMAL' | 'EXCEEDS';

/** Download status of a media item */
export type DownloadStatus = 'MISSING' | 'WANTED' | 'DOWNLOADING' | 'DOWNLOADED' | 'SUBOPTIMAL' | 'IGNORED' | 'PARTIAL';

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
// LLM Parser Types
// ============================================================================

export interface LlmParserSettings {
  enabled: boolean;
  ollamaUrl: string;
  ollamaModel: string;
  timeoutSeconds: number;
  temperature: number;
  maxTokens: number;
  promptTemplate: string;
  confidenceThreshold: number;
  // Library-type-specific models
  modelMovies: string | null;
  modelTv: string | null;
  modelMusic: string | null;
  modelAudiobooks: string | null;
  // Library-type-specific prompts
  promptMovies: string | null;
  promptTv: string | null;
  promptMusic: string | null;
  promptAudiobooks: string | null;
}

export interface OllamaConnectionResult {
  success: boolean;
  availableModels: string[];
  error: string | null;
}

export interface FilenameParseResult {
  mediaType: string | null;
  title: string | null;
  year: number | null;
  season: number | null;
  episode: number | null;
  episodeEnd: number | null;
  resolution: string | null;
  source: string | null;
  videoCodec: string | null;
  audio: string | null;
  hdr: string | null;
  releaseGroup: string | null;
  edition: string | null;
  completeSeries: boolean;
  confidence: number;
}

export interface TestFilenameParserResult {
  regexResult: FilenameParseResult;
  regexTimeMs: number;
  llmResult: FilenameParseResult | null;
  llmTimeMs: number | null;
  llmError: string | null;
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

/** Event when a media file is updated (e.g., after FFmpeg analysis) */
export interface MediaFileUpdatedEvent {
  mediaFileId: string;
  libraryId: string;
  episodeId: string | null;
  movieId: string | null;
  resolution: string | null;
  videoCodec: string | null;
  audioCodec: string | null;
  audioChannels: string | null;
  isHdr: boolean | null;
  hdrType: string | null;
  duration: number | null;
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
  autoAddDiscovered: boolean;
  autoDownload: boolean;
  /** Automatically hunt for missing episodes using indexers */
  autoHunt: boolean;
  /** Whether a scan is currently in progress */
  scanning: boolean;
  itemCount: number;
  totalSizeBytes: number;
  showCount: number;
  movieCount: number;
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

export type LibraryChangeType = 'CREATED' | 'UPDATED' | 'DELETED';

export interface LibraryChangedEvent {
  changeType: LibraryChangeType;
  libraryId: string;
  libraryName: string | null;
  library: Library | null;
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

  // --- Media file metadata (from FFmpeg analysis) ---
  /** Video resolution (e.g., "1080p", "2160p", "720p") */
  resolution: string | null;
  /** Video codec (e.g., "hevc", "h264", "av1") */
  videoCodec: string | null;
  /** Audio codec (e.g., "aac", "dts", "truehd", "atmos") */
  audioCodec: string | null;
  /** Audio channel layout (e.g., "stereo", "5.1", "7.1") */
  audioChannels: string | null;
  /** Whether the video is HDR */
  isHdr: boolean | null;
  /** HDR format type (e.g., "HDR10", "Dolby Vision", "HDR10+") */
  hdrType: string | null;
  /** Video bitrate in kbps */
  videoBitrate: number | null;
  /** File size in bytes */
  fileSizeBytes: number | null;
  /** Human-readable file size */
  fileSizeFormatted: string | null;

  // --- Watch progress (per-user) ---
  /** User's watch progress (0.0 to 1.0, null if never watched) */
  watchProgress: number | null;
  /** User's current position in seconds (for resume) */
  watchPosition: number | null;
  /** Whether the user has watched this episode (>=90% or manually marked) */
  isWatched: boolean | null;
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
  path?: string;
}

export interface UpdateTvShowInput {
  monitored?: boolean;
  monitorType?: MonitorType;
  path?: string;
  /** Override library auto-download (null = inherit, true/false = override) */
  autoDownloadOverride?: boolean | null;
  /** Whether to backfill existing episodes */
  backfillExisting?: boolean;
  /** Override library organize_files (null = inherit) */
  organizeFilesOverride?: boolean | null;
  /** Override library rename_style (null = inherit) */
  renameStyleOverride?: string | null;
  /** Override library auto_hunt (null = inherit) */
  autoHuntOverride?: boolean | null;
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
// Movie Types
// ============================================================================

export type MovieStatus = 'RELEASED' | 'UPCOMING' | 'ANNOUNCED' | 'IN_PRODUCTION' | 'UNKNOWN';

export interface Movie {
  id: string;
  libraryId: string;
  title: string;
  sortTitle: string | null;
  originalTitle: string | null;
  year: number | null;
  tmdbId: number | null;
  imdbId: string | null;
  status: MovieStatus;
  overview: string | null;
  tagline: string | null;
  runtime: number | null;
  genres: string[];
  director: string | null;
  castNames: string[];
  posterUrl: string | null;
  backdropUrl: string | null;
  monitored: boolean;
  /** Whether a file exists for this movie */
  hasFile: boolean;
  sizeBytes: number;
  path: string | null;
  /** TMDB collection ID */
  collectionId: number | null;
  collectionName: string | null;
  collectionPosterUrl: string | null;
  /** Ratings */
  tmdbRating: number | null;
  tmdbVoteCount: number | null;
  certification: string | null;
  releaseDate: string | null;
  // Quality override settings (null = inherit from library)
  allowedResolutionsOverride: string[] | null;
  allowedVideoCodecsOverride: string[] | null;
  allowedAudioFormatsOverride: string[] | null;
  requireHdrOverride: boolean | null;
  allowedHdrTypesOverride: string[] | null;
  allowedSourcesOverride: string[] | null;
  releaseGroupBlacklistOverride: string[] | null;
  releaseGroupWhitelistOverride: string[] | null;
}

export interface MovieSearchResult {
  provider: string;
  providerId: number;
  title: string;
  originalTitle: string | null;
  year: number | null;
  overview: string | null;
  posterUrl: string | null;
  backdropUrl: string | null;
  imdbId: string | null;
  voteAverage: number | null;
  popularity: number | null;
}

export interface MovieResult {
  success: boolean;
  movie: Movie | null;
  error: string | null;
}

export interface AddMovieInput {
  tmdbId: number;
  monitored?: boolean;
  path?: string;
}

export interface UpdateMovieInput {
  monitored?: boolean;
  path?: string;
  // Quality override settings (null = inherit, [] = any)
  allowedResolutionsOverride?: string[] | null;
  allowedVideoCodecsOverride?: string[] | null;
  allowedAudioFormatsOverride?: string[] | null;
  requireHdrOverride?: boolean | null;
  allowedHdrTypesOverride?: string[] | null;
  allowedSourcesOverride?: string[] | null;
  releaseGroupBlacklistOverride?: string[] | null;
  releaseGroupWhitelistOverride?: string[] | null;
}

// ============================================================================
// Album/Music Types
// ============================================================================

export interface Artist {
  id: string;
  libraryId: string;
  name: string;
  sortName: string | null;
  musicbrainzId: string | null;
}

export interface Album {
  id: string;
  artistId: string;
  libraryId: string;
  name: string;
  sortName: string | null;
  year: number | null;
  musicbrainzId: string | null;
  albumType: string | null;
  genres: string[];
  label: string | null;
  country: string | null;
  releaseDate: string | null;
  coverUrl: string | null;
  trackCount: number | null;
  discCount: number | null;
  totalDurationSecs: number | null;
  hasFiles: boolean;
  sizeBytes: number | null;
  path: string | null;
}

export interface AlbumSearchResult {
  provider: string;
  providerId: string;
  title: string;
  artistName: string | null;
  year: number | null;
  albumType: string | null;
  coverUrl: string | null;
  score: number | null;
}

export interface AlbumResult {
  success: boolean;
  album: Album | null;
  error: string | null;
}

export interface AddAlbumInput {
  musicbrainzId: string;
  libraryId: string;
}

// ============================================================================
// Track Types
// ============================================================================

export interface Track {
  id: string;
  albumId: string;
  libraryId: string;
  title: string;
  trackNumber: number;
  discNumber: number;
  musicbrainzId: string | null;
  isrc: string | null;
  durationSecs: number | null;
  explicit: boolean;
  artistName: string | null;
  artistId: string | null;
  mediaFileId: string | null;
  hasFile: boolean;
}

export interface TrackWithStatus {
  track: Track;
  hasFile: boolean;
  filePath: string | null;
  fileSize: number | null;
  /** Audio codec (e.g., FLAC, AAC, MP3) */
  audioCodec: string | null;
  /** Bitrate in kbps */
  bitrate: number | null;
  /** Audio channels (e.g., "stereo", "5.1") */
  audioChannels: string | null;
}

export interface AlbumWithTracks {
  album: Album;
  tracks: TrackWithStatus[];
  trackCount: number;
  tracksWithFiles: number;
  missingTracks: number;
  completionPercent: number;
}

// ============================================================================
// Audiobook Types
// ============================================================================

export interface AudiobookAuthor {
  id: string;
  libraryId: string;
  name: string;
  sortName: string | null;
  openlibraryId: string | null;
}

export interface Audiobook {
  id: string;
  authorId: string | null;
  libraryId: string;
  title: string;
  sortTitle: string | null;
  subtitle: string | null;
  openlibraryId: string | null;
  isbn: string | null;
  description: string | null;
  publisher: string | null;
  language: string | null;
  narrators: string[];
  seriesName: string | null;
  durationSecs: number | null;
  coverUrl: string | null;
  hasFiles: boolean;
  sizeBytes: number | null;
  path: string | null;
}

export interface AudiobookSearchResult {
  provider: string;
  providerId: string;
  title: string;
  authorName: string | null;
  year: number | null;
  coverUrl: string | null;
  isbn: string | null;
  description: string | null;
}

export interface AudiobookResult {
  success: boolean;
  audiobook: Audiobook | null;
  error: string | null;
}

export interface AddAudiobookInput {
  openlibraryId: string;
  libraryId: string;
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
// Detailed Media File Types (for file properties dialog)
// ============================================================================

/** Video stream information from FFmpeg analysis */
export interface VideoStreamInfo {
  id: string;
  streamIndex: number;
  codec: string;
  codecLongName: string | null;
  width: number;
  height: number;
  aspectRatio: string | null;
  frameRate: string | null;
  bitrate: number | null;
  pixelFormat: string | null;
  hdrType: string | null;
  bitDepth: number | null;
  language: string | null;
  title: string | null;
  isDefault: boolean;
}

/** Audio stream information from FFmpeg analysis */
export interface AudioStreamInfo {
  id: string;
  streamIndex: number;
  codec: string;
  codecLongName: string | null;
  channels: number;
  channelLayout: string | null;
  sampleRate: number | null;
  bitrate: number | null;
  bitDepth: number | null;
  language: string | null;
  title: string | null;
  isDefault: boolean;
  isCommentary: boolean;
}

/** Subtitle track information */
export interface SubtitleInfo {
  id: string;
  streamIndex: number | null;
  sourceType: 'EMBEDDED' | 'EXTERNAL' | 'DOWNLOADED';
  codec: string | null;
  codecLongName: string | null;
  language: string | null;
  title: string | null;
  isDefault: boolean;
  isForced: boolean;
  isHearingImpaired: boolean;
  filePath: string | null;
}

/** Chapter information from media file */
export interface ChapterInfo {
  id: string;
  chapterIndex: number;
  startSecs: number;
  endSecs: number;
  title: string | null;
}

/** Detailed media file info including all streams */
export interface MediaFileDetails {
  id: string;
  file: MediaFile;
  videoStreams: VideoStreamInfo[];
  audioStreams: AudioStreamInfo[];
  subtitles: SubtitleInfo[];
  chapters: ChapterInfo[];
}

// ============================================================================
// Naming Pattern Types
// ============================================================================

/** A file naming pattern preset */
export interface NamingPattern {
  /** Unique identifier */
  id: string;
  /** Display name for the pattern */
  name: string;
  /** The actual pattern string (e.g., "{show}/Season {season:02}/...") */
  pattern: string;
  /** Human-readable description/example */
  description: string | null;
  /** Whether this is the default pattern for new libraries */
  isDefault: boolean;
  /** Whether this is a built-in system pattern (cannot be deleted) */
  isSystem: boolean;
}

/** Input for creating a custom naming pattern */
export interface CreateNamingPatternInput {
  name: string;
  pattern: string;
  description?: string;
}

/** Result of naming pattern mutation */
export interface NamingPatternResult {
  success: boolean;
  namingPattern: NamingPattern | null;
  error: string | null;
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

// ============================================================================
// Playback Session Types
// ============================================================================

/** Content type for playback */
export type PlaybackContentType = 'EPISODE' | 'MOVIE' | 'TRACK' | 'AUDIOBOOK';

/** A user's playback session (what they're currently watching/listening) */
export interface PlaybackSession {
  id: string;
  userId: string;
  /** Content type being played */
  contentType: PlaybackContentType | null;
  /** Media file being played */
  mediaFileId: string | null;
  /** Content ID (the episode/movie/track/audiobook ID) */
  contentId: string | null;
  /** Episode being played (for episodes) */
  episodeId: string | null;
  /** Movie being played (for movies) */
  movieId: string | null;
  /** Track being played (for music) */
  trackId: string | null;
  /** Audiobook being played (for audiobooks) */
  audiobookId: string | null;
  /** TV show ID (parent for episodes) */
  tvShowId: string | null;
  /** Album ID (parent for tracks) */
  albumId: string | null;
  currentPosition: number;
  duration: number | null;
  volume: number;
  isMuted: boolean;
  isPlaying: boolean;
  startedAt: string;
  lastUpdatedAt: string;
}

/** Input for starting playback (unified for all content types) */
export interface StartPlaybackInput {
  /** Content type being played */
  contentType: PlaybackContentType;
  /** Media file ID */
  mediaFileId: string;
  /** Content ID (the episode/movie/track/audiobook ID) */
  contentId: string;
  /** Parent ID for context (TV show for episodes, album for tracks) */
  parentId?: string;
  /** Starting position in seconds */
  startPosition?: number;
  /** Duration in seconds */
  duration?: number;
}

/** Input for updating playback */
export interface UpdatePlaybackInput {
  currentPosition?: number;
  duration?: number;
  volume?: number;
  isMuted?: boolean;
  isPlaying?: boolean;
}

/** Result of playback operations */
export interface PlaybackResult {
  success: boolean;
  session: PlaybackSession | null;
  error: string | null;
}

/** Playback settings (configurable by user) */
export interface PlaybackSettings {
  /** How often to sync watch progress to database (in seconds) */
  syncIntervalSeconds: number;
}

/** Input for updating playback settings */
export interface UpdatePlaybackSettingsInput {
  /** How often to sync watch progress to database (in seconds, 5-60) */
  syncIntervalSeconds?: number;
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

// ============================================================================
// Auto-Hunt Types
// ============================================================================

/** Result of an auto-hunt operation */
export interface AutoHuntResult {
  success: boolean;
  error: string | null;
  searched: number;
  matched: number;
  downloaded: number;
  skipped: number;
  failed: number;
}

// ============================================================================
// Filter Types (for GraphQL where clauses)
// ============================================================================

/** Filter for string fields */
export interface StringFilter {
  eq?: string;
  ne?: string;
  contains?: string;
  startsWith?: string;
  endsWith?: string;
  in?: string[];
  notIn?: string[];
}

/** Filter for integer fields */
export interface IntFilter {
  eq?: number;
  ne?: number;
  lt?: number;
  lte?: number;
  gt?: number;
  gte?: number;
  in?: number[];
  notIn?: number[];
}

/** Filter for boolean fields */
export interface BoolFilter {
  eq?: boolean;
  ne?: boolean;
}

/** Date range for between queries */
export interface DateRange {
  start?: string;
  end?: string;
}

/** Filter for date/timestamp fields */
export interface DateFilter {
  eq?: string;
  ne?: string;
  lt?: string;
  lte?: string;
  gt?: string;
  gte?: string;
  between?: DateRange;
}

// ============================================================================
// Pagination Types (cursor-based)
// ============================================================================

/** Order direction for sorting */
export type OrderDirection = 'ASC' | 'DESC';

/** Pagination info for connections */
export interface PageInfo {
  hasNextPage: boolean;
  hasPreviousPage: boolean;
  startCursor: string | null;
  endCursor: string | null;
  totalCount: number | null;
}

/** Generic edge type for paginated results */
export interface Edge<T> {
  node: T;
  cursor: string;
}

/** Generic connection type for paginated results */
export interface Connection<T> {
  edges: Edge<T>[];
  pageInfo: PageInfo;
}

// ============================================================================
// Entity-specific Filter Inputs
// ============================================================================

/** Filter input for movies query */
export interface MovieWhereInput {
  title?: StringFilter;
  year?: IntFilter;
  monitored?: BoolFilter;
  hasFile?: BoolFilter;
  status?: StringFilter;
  createdAt?: DateFilter;
}

/** Order by input for movies */
export interface MovieOrderByInput {
  field: 'title' | 'year' | 'releaseDate' | 'createdAt' | 'sortTitle';
  direction?: OrderDirection;
}

/** Filter input for TV shows query */
export interface TvShowWhereInput {
  name?: StringFilter;
  year?: IntFilter;
  status?: StringFilter;
  monitored?: BoolFilter;
  network?: StringFilter;
  createdAt?: DateFilter;
}

/** Order by input for TV shows */
export interface TvShowOrderByInput {
  field: 'name' | 'year' | 'createdAt' | 'sortName';
  direction?: OrderDirection;
}

/** Filter input for albums query */
export interface AlbumWhereInput {
  name?: StringFilter;
  year?: IntFilter;
  artistName?: StringFilter;
  hasFiles?: BoolFilter;
  albumType?: StringFilter;
}

/** Order by input for albums */
export interface AlbumOrderByInput {
  field: 'name' | 'year' | 'createdAt' | 'sortName';
  direction?: OrderDirection;
}

/** Filter input for audiobooks query */
export interface AudiobookWhereInput {
  title?: StringFilter;
  authorName?: StringFilter;
  hasFiles?: BoolFilter;
  language?: StringFilter;
}

/** Order by input for audiobooks */
export interface AudiobookOrderByInput {
  field: 'title' | 'createdAt' | 'sortTitle';
  direction?: OrderDirection;
}
