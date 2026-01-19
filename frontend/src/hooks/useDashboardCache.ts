/**
 * Dashboard data cache with stale-while-revalidate pattern
 *
 * This hook provides cached dashboard data that loads instantly from cache
 * while fetching fresh data in the background. Uses React 19's concurrent
 * features for optimal performance.
 */

import { useState, useEffect, useCallback, useRef, useMemo } from 'react'
import {
  graphqlClient,
  LIBRARIES_QUERY,
  TV_SHOWS_QUERY,
  UPCOMING_EPISODES_QUERY,
  LIBRARY_UPCOMING_EPISODES_QUERY,
  LIBRARY_CHANGED_SUBSCRIPTION,
  TORRENT_COMPLETED_SUBSCRIPTION,
  type Library,
  type TvShow,
  type UpcomingEpisode,
  type LibraryUpcomingEpisode,
  type LibraryChangedEvent,
} from '../lib/graphql'

// Cache configuration
const CACHE_KEY = 'librarian:dashboard_cache'
const CACHE_TTL_MS = 5 * 60 * 1000 // 5 minutes
const STALE_TTL_MS = 30 * 60 * 1000 // 30 minutes (serve stale data up to this long)

interface DashboardCache {
  libraries: Library[]
  recentShows: TvShow[]
  libraryUpcoming: LibraryUpcomingEpisode[]
  globalUpcoming: UpcomingEpisode[]
  timestamp: number
  userId: string
}

interface DashboardData {
  libraries: Library[]
  recentShows: TvShow[]
  libraryUpcoming: LibraryUpcomingEpisode[]
  globalUpcoming: UpcomingEpisode[]
}

interface UseDashboardCacheResult {
  data: DashboardData
  isLoading: boolean
  isStale: boolean
  isFetching: boolean
  refetch: () => Promise<void>
}

// Read cache from localStorage
function readCache(userId: string): DashboardCache | null {
  try {
    const cached = localStorage.getItem(CACHE_KEY)
    if (!cached) return null
    
    const parsed: DashboardCache = JSON.parse(cached)
    
    // Check if cache belongs to current user
    if (parsed.userId !== userId) return null
    
    // Check if cache is too old to serve
    const age = Date.now() - parsed.timestamp
    if (age > STALE_TTL_MS) return null
    
    return parsed
  } catch {
    return null
  }
}

// Write cache to localStorage
function writeCache(data: DashboardData, userId: string): void {
  try {
    const cache: DashboardCache = {
      ...data,
      timestamp: Date.now(),
      userId,
    }
    localStorage.setItem(CACHE_KEY, JSON.stringify(cache))
  } catch {
    // Ignore localStorage errors
  }
}

// Check if cache is fresh
function isCacheFresh(cache: DashboardCache | null): boolean {
  if (!cache) return false
  return Date.now() - cache.timestamp < CACHE_TTL_MS
}

/**
 * Hook for cached dashboard data with stale-while-revalidate pattern
 *
 * @param userId - Current user ID
 * @returns Dashboard data with loading states
 */
export function useDashboardCache(userId: string | null): UseDashboardCacheResult {
  const [data, setData] = useState<DashboardData>({
    libraries: [],
    recentShows: [],
    libraryUpcoming: [],
    globalUpcoming: [],
  })
  const [isLoading, setIsLoading] = useState(true)
  const [isFetching, setIsFetching] = useState(false)
  const [isStale, setIsStale] = useState(false)
  const fetchInProgress = useRef(false)
  const initialLoadDone = useRef(false)

  // Fetch fresh data from the API
  const fetchData = useCallback(async (): Promise<DashboardData | null> => {
    if (!userId) return null
    
    try {
      // Fetch all data in parallel for maximum performance
      const [librariesResult, libraryUpcomingResult, globalUpcomingResult] = await Promise.all([
        graphqlClient.query<{ libraries: Library[] }>(LIBRARIES_QUERY).toPromise(),
        graphqlClient.query<{ libraryUpcomingEpisodes: LibraryUpcomingEpisode[] }>(
          LIBRARY_UPCOMING_EPISODES_QUERY,
          { days: 7 }
        ).toPromise(),
        graphqlClient.query<{ upcomingEpisodes: UpcomingEpisode[] }>(
          UPCOMING_EPISODES_QUERY,
          { days: 7, country: 'US' }
        ).toPromise(),
      ])

      const libraries = librariesResult.data?.libraries ?? []
      const libraryUpcoming = libraryUpcomingResult.data?.libraryUpcomingEpisodes ?? []
      const globalUpcoming = globalUpcomingResult.data?.upcomingEpisodes ?? []

      // Fetch recent shows from TV libraries
      const tvLibraries = libraries.filter((lib) => lib.libraryType === 'TV')
      const allShows: TvShow[] = []

      // Limit to 2 libraries for performance
      for (const library of tvLibraries.slice(0, 2)) {
        const showsResult = await graphqlClient
          .query<{ tvShows: TvShow[] }>(TV_SHOWS_QUERY, { libraryId: library.id })
          .toPromise()

        if (showsResult.data?.tvShows) {
          allShows.push(...showsResult.data.tvShows)
        }
      }

      // Filter global upcoming to unique shows
      const seenShows = new Set<number>()
      const filteredGlobalUpcoming = globalUpcoming.filter((ep) => {
        if (seenShows.has(ep.show.tvmazeId)) return false
        seenShows.add(ep.show.tvmazeId)
        return true
      }).slice(0, 12)

      return {
        libraries,
        recentShows: allShows.slice(0, 6),
        libraryUpcoming,
        globalUpcoming: filteredGlobalUpcoming,
      }
    } catch (err) {
      console.error('Failed to fetch dashboard data:', err)
      return null
    }
  }, [userId])

  // Refetch function exposed to components
  const refetch = useCallback(async () => {
    if (!userId || fetchInProgress.current) return

    fetchInProgress.current = true
    setIsFetching(true)

    const freshData = await fetchData()
    if (freshData) {
      setData(freshData)
      setIsStale(false)
      writeCache(freshData, userId)
    }

    setIsFetching(false)
    setIsLoading(false)
    fetchInProgress.current = false
  }, [userId, fetchData])

  // Initial load with cache-first strategy
  useEffect(() => {
    if (!userId) {
      setIsLoading(false)
      return
    }

    if (initialLoadDone.current) return
    initialLoadDone.current = true

    // Try to load from cache immediately
    const cached = readCache(userId)
    if (cached) {
      setData({
        libraries: cached.libraries,
        recentShows: cached.recentShows,
        libraryUpcoming: cached.libraryUpcoming,
        globalUpcoming: cached.globalUpcoming,
      })
      
      const isFresh = isCacheFresh(cached)
      setIsStale(!isFresh)
      setIsLoading(false)

      // If cache is stale, fetch in background
      if (!isFresh) {
        refetch()
      }
    } else {
      // No cache, fetch fresh data
      refetch()
    }
  }, [userId, refetch])

  // Subscribe to real-time updates for library changes and torrent completions
  useEffect(() => {
    if (!userId) return

    // Subscribe to library changes (created, updated, deleted)
    const librarySub = graphqlClient
      .subscription<{ libraryChanged: LibraryChangedEvent }>(
        LIBRARY_CHANGED_SUBSCRIPTION,
        {}
      )
      .subscribe({
        next: () => {
          // Refetch dashboard data when any library changes
          refetch()
        },
      })

    // Subscribe to torrent completions (triggers media organization)
    const torrentSub = graphqlClient
      .subscription<{ torrentCompleted: { id: number } }>(
        TORRENT_COMPLETED_SUBSCRIPTION,
        {}
      )
      .subscribe({
        next: () => {
          // Refetch dashboard data when a torrent completes
          refetch()
        },
      })

    return () => {
      librarySub.unsubscribe()
      torrentSub.unsubscribe()
    }
  }, [userId, refetch])

  // Memoize the result to prevent unnecessary re-renders
  const result = useMemo(
    () => ({
      data,
      isLoading,
      isStale,
      isFetching,
      refetch,
    }),
    [data, isLoading, isStale, isFetching, refetch]
  )

  return result
}

/**
 * Clear the dashboard cache (call on logout)
 */
export function clearDashboardCache(): void {
  try {
    localStorage.removeItem(CACHE_KEY)
  } catch {
    // Ignore errors
  }
}
