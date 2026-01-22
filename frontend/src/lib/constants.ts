/**
 * Centralized constants for status colors, labels, and other configuration.
 * Import from this file to ensure consistency across the application.
 */

import type { TorrentState, LibraryType, TvShowStatus, MovieStatus } from './graphql'
import type { DerivedEpisodeStatus } from '../components/shared/EpisodeStatusChip'

// ============================================================================
// Chip Color Type (HeroUI compatible)
// ============================================================================

export type ChipColor = 'success' | 'warning' | 'danger' | 'default' | 'primary' | 'secondary'

export interface StatusConfig {
  color: ChipColor
  label: string
}

// ============================================================================
// Episode Status Configuration (derived from mediaFileId)
// ============================================================================

export const EPISODE_STATUS_CONFIG: Record<DerivedEpisodeStatus, StatusConfig> = {
  DOWNLOADED: { color: 'success', label: 'Downloaded' },
  DOWNLOADING: { color: 'primary', label: 'Downloading' },
  WANTED: { color: 'warning', label: 'Wanted' },
}

export function getEpisodeStatusConfig(status: DerivedEpisodeStatus): StatusConfig {
  return EPISODE_STATUS_CONFIG[status] ?? { color: 'default', label: status }
}

// ============================================================================
// Torrent State Configuration
// ============================================================================

export const TORRENT_STATE_CONFIG: Record<TorrentState, StatusConfig> = {
  QUEUED: { color: 'default', label: 'Queued' },
  CHECKING: { color: 'warning', label: 'Checking' },
  DOWNLOADING: { color: 'primary', label: 'Downloading' },
  SEEDING: { color: 'success', label: 'Seeding' },
  PAUSED: { color: 'warning', label: 'Paused' },
  ERROR: { color: 'danger', label: 'Error' },
}

export function getTorrentStateConfig(state: TorrentState): StatusConfig {
  return TORRENT_STATE_CONFIG[state] ?? { color: 'default', label: state }
}

// ============================================================================
// TV Show Status Configuration
// ============================================================================

export const TV_SHOW_STATUS_CONFIG: Record<TvShowStatus, StatusConfig> = {
  CONTINUING: { color: 'success', label: 'Continuing' },
  ENDED: { color: 'default', label: 'Ended' },
  UPCOMING: { color: 'primary', label: 'Upcoming' },
  CANCELLED: { color: 'danger', label: 'Cancelled' },
  UNKNOWN: { color: 'default', label: 'Unknown' },
}

export function getTvShowStatusConfig(status: TvShowStatus): StatusConfig {
  return TV_SHOW_STATUS_CONFIG[status] ?? { color: 'default', label: status }
}

// ============================================================================
// Movie Status Configuration
// ============================================================================

export const MOVIE_STATUS_CONFIG: Record<MovieStatus, StatusConfig> = {
  RELEASED: { color: 'success', label: 'Released' },
  UPCOMING: { color: 'primary', label: 'Upcoming' },
  ANNOUNCED: { color: 'secondary', label: 'Announced' },
  IN_PRODUCTION: { color: 'warning', label: 'In Production' },
  UNKNOWN: { color: 'default', label: 'Unknown' },
}

export function getMovieStatusConfig(status: MovieStatus): StatusConfig {
  return MOVIE_STATUS_CONFIG[status] ?? { color: 'default', label: status }
}

// ============================================================================
// Library Type Colors
// ============================================================================

export const LIBRARY_TYPE_COLORS: Record<LibraryType, string> = {
  MOVIES: 'purple',
  TV: 'blue',
  MUSIC: 'green',
  AUDIOBOOKS: 'orange',
  OTHER: 'slate',
}

// ============================================================================
// Quality/Resolution Labels
// ============================================================================

export const RESOLUTION_LABELS: Record<string, string> = {
  '2160p': '4K',
  '1080p': '1080p',
  '720p': '720p',
  '480p': '480p',
  'sd': 'SD',
}

export const SOURCE_LABELS: Record<string, string> = {
  bluray: 'Blu-ray',
  webdl: 'WEB-DL',
  webrip: 'WEBRip',
  hdtv: 'HDTV',
  dvd: 'DVD',
  cam: 'CAM',
}

export const HDR_LABELS: Record<string, string> = {
  hdr10: 'HDR10',
  hdr10plus: 'HDR10+',
  dolbyvision: 'Dolby Vision',
  hlg: 'HLG',
}

// ============================================================================
// Common Time Intervals
// ============================================================================

export const POLL_INTERVALS = {
  /** Fast polling for active operations (5 seconds) */
  FAST: 5000,
  /** Normal polling for background updates (15 seconds) */
  NORMAL: 15000,
  /** Slow polling for infrequent updates (60 seconds) */
  SLOW: 60000,
} as const

// ============================================================================
// Size Thresholds
// ============================================================================

export const SIZE_THRESHOLDS = {
  /** Consider files larger than this as likely full media (1 GB) */
  LIKELY_MEDIA: 1024 * 1024 * 1024,
  /** Minimum size for video files to be considered valid (50 MB) */
  MIN_VIDEO: 50 * 1024 * 1024,
} as const
