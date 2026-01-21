/**
 * Shared formatting utilities for displaying file sizes, speeds, and durations.
 */

/**
 * Format a byte count as a human-readable string (e.g., "1.5 GB")
 */
export function formatBytes(bytes: number | null | undefined): string {
  if (bytes == null || bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let unitIndex = 0
  let size = bytes
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex++
  }
  return `${size.toFixed(1)} ${units[unitIndex]}`
}

/**
 * Format a speed in bytes per second as a human-readable string (e.g., "1.5 MB/s")
 */
export function formatSpeed(bytesPerSecond: number | null | undefined): string {
  return `${formatBytes(bytesPerSecond)}/s`
}

/**
 * Format a duration in seconds as a human-readable string (e.g., "2h 30m")
 */
export function formatEta(seconds: number | null | undefined): string {
  if (seconds == null || seconds <= 0) return '—'
  if (seconds < 60) return `${Math.floor(seconds)}s`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`
  return `${Math.floor(seconds / 86400)}d ${Math.floor((seconds % 86400) / 3600)}h`
}

/**
 * Format a duration in seconds as a video timestamp (e.g., "1:23:45")
 */
export function formatDuration(seconds: number | null | undefined): string {
  if (seconds == null || seconds <= 0) return '0:00'
  const hrs = Math.floor(seconds / 3600)
  const mins = Math.floor((seconds % 3600) / 60)
  const secs = Math.floor(seconds % 60)
  if (hrs > 0) {
    return `${hrs}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`
  }
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

/**
 * Format a date string for display (e.g., "Jan 9, 2026")
 * @param dateStr - The date string to format
 * @param fallback - Text to show when date is null/undefined (default: "Never")
 */
export function formatDate(dateStr: string | null | undefined, fallback: string = 'Never'): string {
  if (!dateStr) return fallback
  const date = new Date(dateStr)
  return date.toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}

/**
 * Format a date string with time (e.g., "Jan 9, 2026, 3:45 PM")
 */
export function formatDateTime(dateStr: string | null | undefined): string {
  if (!dateStr) return 'Never'
  const date = new Date(dateStr)
  return date.toLocaleString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  })
}

/**
 * Format a date as relative time (e.g., "2 hours ago", "in 3 days")
 */
export function formatRelativeTime(dateStr: string | null | undefined): string {
  if (!dateStr) return 'Never'
  const date = new Date(dateStr)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffSecs = Math.floor(diffMs / 1000)
  const diffMins = Math.floor(diffSecs / 60)
  const diffHours = Math.floor(diffMins / 60)
  const diffDays = Math.floor(diffHours / 24)

  if (diffSecs < 0) {
    // Future
    const absSecs = Math.abs(diffSecs)
    const absMins = Math.floor(absSecs / 60)
    const absHours = Math.floor(absMins / 60)
    const absDays = Math.floor(absHours / 24)
    if (absDays > 0) return `in ${absDays}d`
    if (absHours > 0) return `in ${absHours}h`
    if (absMins > 0) return `in ${absMins}m`
    return 'soon'
  }

  if (diffDays > 0) return `${diffDays}d ago`
  if (diffHours > 0) return `${diffHours}h ago`
  if (diffMins > 0) return `${diffMins}m ago`
  return 'just now'
}

/**
 * Sanitize error messages for display (strips HTML, truncates long messages)
 */
export function sanitizeError(error: unknown): string {
  if (!error) return 'Unknown error'
  const message = typeof error === 'string' ? error : (error as Error).message || String(error)
  // If the error contains HTML, show a generic message
  if (message.includes('<!DOCTYPE') || message.includes('<html')) {
    return 'Failed to connect to server. Please check that the backend is running.'
  }
  // Truncate very long messages
  if (message.length > 200) {
    return message.substring(0, 200) + '...'
  }
  return message
}

/**
 * Preview a naming pattern with sample data
 * Replaces all pattern variables with example values based on library type
 */
export function previewNamingPattern(pattern: string, libraryType?: string): string {
  if (!pattern) return ''
  
  let result = pattern
  
  // Common variables
  result = result.replaceAll('{year}', '2008')
  result = result.replaceAll('{ext}', libraryType === 'music' ? 'flac' : libraryType === 'audiobooks' ? 'm4b' : 'mkv')
  result = result.replaceAll('{original}', 'original_filename')
  
  // TV Show variables
  result = result.replaceAll('{show}', 'Breaking Bad')
  result = result.replaceAll('{season:02}', '01')
  result = result.replaceAll('{season}', '1')
  result = result.replaceAll('{episode:02}', '05')
  result = result.replaceAll('{episode}', '5')
  
  // Music variables
  result = result.replaceAll('{artist}', 'Pink Floyd')
  result = result.replaceAll('{album}', 'The Dark Side of the Moon')
  result = result.replaceAll('{track:02}', '03')
  result = result.replaceAll('{track}', '3')
  result = result.replaceAll('{disc}', '1')
  
  // Movie variables
  result = result.replaceAll('{quality}', '1080p BluRay')
  
  // Audiobook variables
  result = result.replaceAll('{author}', 'Brandon Sanderson')
  result = result.replaceAll('{series}', 'The Stormlight Archive')
  result = result.replaceAll('{series_position}', '1')
  result = result.replaceAll('{narrator}', 'Michael Kramer')
  result = result.replaceAll('{chapter:02}', '01')
  result = result.replaceAll('{chapter}', '1')
  result = result.replaceAll('{chapter_title}', 'Prologue')
  
  // Title - use context-appropriate sample based on what other variables are present
  // This must come last since multiple types use {title}
  if (libraryType === 'music' || pattern.includes('{artist}') || pattern.includes('{album}')) {
    result = result.replaceAll('{title}', 'Time')
  } else if (libraryType === 'audiobooks' || pattern.includes('{author}')) {
    result = result.replaceAll('{title}', 'The Way of Kings')
  } else if (libraryType === 'movies' || (pattern.includes('{quality}') && !pattern.includes('{show}'))) {
    result = result.replaceAll('{title}', 'The Matrix')
  } else {
    // Default to TV show episode title
    result = result.replaceAll('{title}', 'Gray Matter')
  }
  
  return result
}
