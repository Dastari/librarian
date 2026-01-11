import { useEffect, useRef, useCallback } from 'react'
import { graphqlClient, TORRENT_COMPLETED_SUBSCRIPTION } from '../lib/graphql'

/**
 * Hook to subscribe to torrent completion events and trigger a callback.
 * Use this to refresh data when downloads complete.
 */
export function useTorrentCompletionRefresh(onComplete: () => void) {
  const callbackRef = useRef(onComplete)
  callbackRef.current = onComplete

  useEffect(() => {
    const sub = graphqlClient.subscription<{ torrentCompleted: { id: number; name: string } }>(
      TORRENT_COMPLETED_SUBSCRIPTION,
      {}
    ).subscribe({
      next: () => {
        callbackRef.current()
      },
    })

    return () => sub.unsubscribe()
  }, [])
}

/**
 * Hook to set up periodic data refresh with configurable interval.
 * Useful for data that doesn't have a subscription but should stay fresh.
 */
export function usePeriodicRefresh(
  onRefresh: () => void,
  intervalMs: number = 30000,
  enabled: boolean = true
) {
  const callbackRef = useRef(onRefresh)
  callbackRef.current = onRefresh

  useEffect(() => {
    if (!enabled) return

    const interval = setInterval(() => {
      callbackRef.current()
    }, intervalMs)

    return () => clearInterval(interval)
  }, [intervalMs, enabled])
}

/**
 * Hook to refresh data when the window regains focus.
 * Good for keeping data fresh when users switch back to the tab.
 */
export function useFocusRefresh(onRefresh: () => void, enabled: boolean = true) {
  const callbackRef = useRef(onRefresh)
  callbackRef.current = onRefresh

  useEffect(() => {
    if (!enabled) return

    const handleFocus = () => {
      callbackRef.current()
    }

    window.addEventListener('focus', handleFocus)
    return () => window.removeEventListener('focus', handleFocus)
  }, [enabled])
}

/**
 * Combined hook for keeping data reactive through multiple mechanisms.
 * Use this when you want data to stay fresh through torrent completions,
 * periodic refreshes, and focus events.
 */
export function useDataReactivity(
  onRefresh: () => void,
  options: {
    onTorrentComplete?: boolean
    periodicInterval?: number | false
    onFocus?: boolean
  } = {}
) {
  const {
    onTorrentComplete = true,
    periodicInterval = 30000,
    onFocus = true,
  } = options

  // Debounce the refresh to avoid multiple rapid calls
  const timeoutRef = useRef<NodeJS.Timeout | null>(null)
  const callbackRef = useRef(onRefresh)
  callbackRef.current = onRefresh

  const debouncedRefresh = useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current)
    }
    timeoutRef.current = setTimeout(() => {
      callbackRef.current()
    }, 500)
  }, [])

  // Clean up timeout on unmount
  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
      }
    }
  }, [])

  useTorrentCompletionRefresh(onTorrentComplete ? debouncedRefresh : () => {})
  usePeriodicRefresh(debouncedRefresh, periodicInterval || 30000, periodicInterval !== false)
  useFocusRefresh(debouncedRefresh, onFocus)
}
