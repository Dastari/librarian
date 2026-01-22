/**
 * Template data for shimmer-from-structure loading states.
 * These mock objects allow components to render their actual structure
 * during loading, enabling automatic shimmer effect generation.
 */

import type {
  Movie,
  TvShow,
  Library,
  AlbumWithTracks,
  AudiobookWithChapters,
  Episode,
  RssFeed,
} from './graphql'

// =============================================================================
// Movie Template
// =============================================================================

export const movieTemplate: Movie = {
  id: 'template',
  libraryId: 'template',
  tmdbId: null,
  imdbId: null,
  title: 'Loading Movie Title Here',
  sortTitle: null,
  originalTitle: null,
  year: 2024,
  runtime: 120,
  overview: 'Loading movie description text that spans multiple lines to simulate a realistic overview length for the shimmer effect...',
  posterUrl: null,
  backdropUrl: null,
  status: 'RELEASED',
  mediaFileId: null,
  genres: ['Genre', 'Genre'],
  tagline: 'Loading tagline text here',
  releaseDate: '2024-01-01',
  tmdbRating: 7.5,
  tmdbVoteCount: 1000,
  certification: 'PG-13',
  director: 'Loading Director',
  castNames: ['Actor One', 'Actor Two', 'Actor Three'],
  collectionId: null,
  collectionName: null,
  collectionPosterUrl: null,
  monitored: true,
}

// =============================================================================
// TV Show Template
// =============================================================================

export const showTemplate: TvShow = {
  id: 'template',
  libraryId: 'template',
  tvmazeId: null,
  tmdbId: null,
  tvdbId: null,
  imdbId: null,
  name: 'Loading Show Title Here',
  sortName: null,
  year: 2024,
  status: 'CONTINUING',
  network: 'Network',
  runtime: null,
  genres: [],
  overview: 'Loading show description text that spans multiple lines to simulate a realistic overview length for the shimmer effect...',
  posterUrl: null,
  backdropUrl: null,
  sizeBytes: 0,
  path: null,
  monitored: true,
  monitorType: 'ALL',
  backfillExisting: false,
  autoDownloadOverride: null,
  autoHuntOverride: null,
  organizeFilesOverride: null,
  renameStyleOverride: null,
  episodeCount: 0,
  episodeFileCount: 0,
  allowedResolutionsOverride: null,
  allowedVideoCodecsOverride: null,
  allowedAudioFormatsOverride: null,
  requireHdrOverride: null,
  allowedHdrTypesOverride: null,
  allowedSourcesOverride: null,
  releaseGroupBlacklistOverride: null,
  releaseGroupWhitelistOverride: null,
}

export const episodesTemplate: Episode[] = Array.from({ length: 5 }, (_, i) => ({
  id: `template-${i}`,
  tvShowId: 'template',
  season: 1,
  episode: i + 1,
  absoluteNumber: null,
  title: `Episode ${i + 1} Title`,
  overview: 'Loading episode description...',
  airDate: '2024-01-01',
  runtime: 45,
  status: 'MISSING' as const,
  tvmazeId: null,
  tmdbId: null,
  tvdbId: null,
  torrentLink: null,
  torrentLinkAddedAt: null,
  mediaFileId: null,
  resolution: null,
  videoCodec: null,
  audioCodec: null,
  audioChannels: null,
  isHdr: null,
  hdrType: null,
  videoBitrate: null,
  fileSizeBytes: null,
  fileSizeFormatted: null,
  watchProgress: null,
  watchPosition: null,
  isWatched: null,
  downloadProgress: null,
}))

// =============================================================================
// Album Template
// =============================================================================

export const albumTemplate: AlbumWithTracks = {
  album: {
    id: 'template',
    artistId: 'template-artist',
    libraryId: 'template',
    musicbrainzId: null,
    name: 'Loading Album Title Here',
    sortName: null,
    albumType: 'album',
    genres: [],
    year: 2024,
    label: 'Record Label',
    country: null,
    releaseDate: null,
    coverUrl: null,
    trackCount: 12,
    discCount: 1,
    totalDurationSecs: 3600,
    hasFiles: false,
    sizeBytes: 0,
    path: null,
    downloadedTrackCount: 0,
  },
  artistName: 'Loading Artist Name',
  trackCount: 12,
  tracksWithFiles: 0,
  missingTracks: 12,
  completionPercent: 0,
  tracks: Array.from({ length: 5 }, (_, i) => ({
    track: {
      id: `template-track-${i}`,
      albumId: 'template',
      libraryId: 'template',
      musicbrainzId: null,
      isrc: null,
      title: `Track ${i + 1} Title`,
      artistName: 'Artist Name',
      artistId: null,
      trackNumber: i + 1,
      discNumber: 1,
      durationSecs: 240,
      explicit: false,
      mediaFileId: null,
      hasFile: false,
      status: 'missing' as const,
      downloadProgress: null,
    },
    hasFile: false,
    filePath: null,
    fileSize: null,
    audioCodec: null,
    bitrate: null,
    audioChannels: null,
  })),
}

// =============================================================================
// Audiobook Template
// =============================================================================

export const audiobookTemplate: AudiobookWithChapters = {
  audiobook: {
    id: 'template',
    libraryId: 'template',
    authorId: null,
    openlibraryId: null,
    isbn: null,
    title: 'Loading Audiobook Title Here',
    sortTitle: null,
    subtitle: 'Loading Subtitle',
    description: 'Loading audiobook description text that spans multiple lines...',
    coverUrl: null,
    language: 'en',
    publisher: 'Publisher Name',
    narrators: ['Narrator One', 'Narrator Two'],
    seriesName: 'Series Name',
    durationSecs: 36000,
    hasFiles: false,
    sizeBytes: 0,
    path: null,
    chapterCount: 12,
    downloadedChapterCount: 0,
  },
  author: {
    id: 'template-author',
    name: 'Loading Author Name',
    sortName: null,
    openlibraryId: null,
    libraryId: 'template',
  },
  chapterCount: 20,
  chaptersWithFiles: 0,
  missingChapters: 20,
  completionPercent: 0,
  chapters: Array.from({ length: 5 }, (_, i) => ({
    id: `template-chapter-${i}`,
    audiobookId: 'template',
    chapterNumber: i + 1,
    title: `Chapter ${i + 1}`,
    startSecs: i * 1800,
    endSecs: (i + 1) * 1800,
    durationSecs: 1800,
    mediaFileId: null,
    status: 'missing' as const,
    downloadProgress: null,
  })),
}

// =============================================================================
// Library Templates
// =============================================================================

export const libraryTemplate: Library = {
  id: 'template',
  name: 'Loading Library',
  path: '/path/to/library',
  libraryType: 'TV',
  icon: '',
  color: '',
  autoScan: false,
  scanIntervalMinutes: 60,
  watchForChanges: false,
  organizeFiles: false,
  renameStyle: '',
  namingPattern: null,
  autoAddDiscovered: false,
  autoDownload: false,
  autoHunt: false,
  scanning: false,
  itemCount: 0,
  showCount: 0,
  movieCount: 0,
  totalSizeBytes: 0,
  lastScannedAt: null,
  allowedResolutions: [],
  allowedVideoCodecs: [],
  allowedAudioFormats: [],
  requireHdr: false,
  allowedHdrTypes: [],
  allowedSources: [],
  releaseGroupBlacklist: [],
  releaseGroupWhitelist: [],
}

// Template for library grid (multiple cards)
export const librariesTemplate: Library[] = Array.from({ length: 6 }, (_, i) => ({
  ...libraryTemplate,
  id: `template-${i}`,
  name: `Library ${i + 1}`,
}))

// =============================================================================
// RSS Feed Template
// =============================================================================

export const rssFeedsTemplate: RssFeed[] = Array.from({ length: 3 }, (_, i) => ({
  id: `template-${i}`,
  libraryId: null,
  name: `RSS Feed ${i + 1}`,
  url: 'https://example.com/rss',
  enabled: true,
  pollIntervalMinutes: 15,
  lastPolledAt: null,
  lastSuccessfulAt: null,
  lastError: null,
  consecutiveFailures: 0,
}))
