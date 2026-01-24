import { useEffect, useState, useCallback, useRef } from 'react'
import {
  graphqlClient,
  CONTENT_DOWNLOAD_PROGRESS_SUBSCRIPTION,
  type ContentDownloadProgressEvent,
  type ContentDownloadType,
} from '../lib/graphql'

/** Map of content ID to download progress (0-1) */
export type ContentProgressMap = Map<string, number>

/** Hook options */
export interface UseContentDownloadProgressOptions {
  /** Library ID to filter progress events */
  libraryId?: string
  /** Parent ID (show, album, audiobook) to filter progress events */
  parentId?: string
  /** Content type to filter (optional - if not provided, all types are included) */
  contentType?: ContentDownloadType
  /** Enable/disable the subscription */
  enabled?: boolean
}

/**
 * Hook to subscribe to content download progress updates
 *
 * Returns a map of content IDs to their download progress (0-1).
 * Use this on library detail pages to show real-time download progress.
 *
 * @example
 * ```tsx
 * const progressMap = useContentDownloadProgress({
 *   libraryId: library.id,
 *   contentType: ContentDownloadType.TRACK,
 * })
 *
 * // In render
 * const progress = progressMap.get(track.id)
 * if (progress !== undefined) {
 *   return <Progress value={progress * 100} />
 * }
 * ```
 */
export function useContentDownloadProgress(
  options: UseContentDownloadProgressOptions = {}
): ContentProgressMap {
  const { libraryId, parentId, contentType, enabled = true } = options
  const [progressMap, setProgressMap] = useState<ContentProgressMap>(new Map())
  const progressMapRef = useRef<ContentProgressMap>(new Map())

  // Update ref when state changes
  useEffect(() => {
    progressMapRef.current = progressMap
  }, [progressMap])

  // Handle incoming progress events
  const handleProgress = useCallback(
    (event: ContentDownloadProgressEvent) => {
      // Filter by content type if specified
      if (contentType && event.contentType !== contentType) {
        return
      }

      // Update the progress map
      const newMap = new Map(progressMapRef.current)

      if (event.progress >= 1.0) {
        // Remove completed items
        newMap.delete(event.contentId)
      } else {
        newMap.set(event.contentId, event.progress)
      }

      setProgressMap(newMap)
    },
    [contentType]
  )

  // Subscribe to content download progress
  useEffect(() => {
    if (!enabled) {
      return
    }

    const sub = graphqlClient
      .subscription<{ contentDownloadProgress: ContentDownloadProgressEvent }>(
        CONTENT_DOWNLOAD_PROGRESS_SUBSCRIPTION,
        {
          libraryId: libraryId ?? null,
          parentId: parentId ?? null,
        }
      )
      .subscribe({
        next: (result) => {
          if (result.data?.contentDownloadProgress) {
            handleProgress(result.data.contentDownloadProgress)
          }
        },
        error: (err) => {
          console.debug('[ContentProgress] Subscription error:', err)
        },
      })

    return () => sub.unsubscribe()
  }, [libraryId, parentId, enabled, handleProgress])

  return progressMap
}

/**
 * Helper hook to get download progress for a specific content item
 *
 * @example
 * ```tsx
 * const progress = useContentItemProgress(progressMap, track.id)
 * // progress is number | undefined
 * ```
 */
export function useContentItemProgress(
  progressMap: ContentProgressMap,
  contentId: string | undefined
): number | undefined {
  if (!contentId) return undefined
  return progressMap.get(contentId)
}
