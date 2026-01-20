import { useState, useCallback, useRef, useEffect } from 'react'
import { graphqlClient } from '../lib/graphql'
import { sanitizeError } from '../lib/format'
import type { Connection, PageInfo, Edge } from '../lib/graphql/types'

// ============================================================================
// Types
// ============================================================================

export interface UseInfiniteConnectionOptions<TData, TNode> {
  /** The GraphQL query string */
  query: string
  /** Query variables (excluding first/after which are managed internally) */
  variables?: Record<string, unknown>
  /** Function to extract the connection from the query response */
  getConnection: (data: TData) => Connection<TNode>
  /** Number of items to fetch per batch (default: 50) */
  batchSize?: number
  /** Whether the query is enabled (default: true) */
  enabled?: boolean
  /** Dependencies that trigger a refetch when changed */
  deps?: unknown[]
}

export interface UseInfiniteConnectionResult<TNode> {
  /** Accumulated items from all loaded pages */
  items: TNode[]
  /** Whether the initial load is in progress */
  isLoading: boolean
  /** Whether a "load more" operation is in progress */
  isLoadingMore: boolean
  /** Whether there are more items to load */
  hasMore: boolean
  /** Total count of items (if available from server) */
  totalCount: number | null
  /** Error message if query failed */
  error: string | null
  /** Function to load the next page */
  loadMore: () => void
  /** Function to refresh from the beginning */
  refresh: () => void
  /** Current page info */
  pageInfo: PageInfo | null
}

// Default batch size for infinite loading
const DEFAULT_BATCH_SIZE = 50

// ============================================================================
// Hook Implementation
// ============================================================================

/**
 * Hook for infinite loading with cursor-based pagination.
 * 
 * Manages:
 * - Cursor tracking for pagination
 * - Accumulating results across pages
 * - Loading states (initial load vs load more)
 * - Automatic refetch when variables change
 * 
 * @example
 * ```tsx
 * const { items, isLoading, hasMore, loadMore, isLoadingMore } = useInfiniteConnection({
 *   query: MOVIES_CONNECTION_QUERY,
 *   variables: { libraryId, where: filters },
 *   getConnection: (data) => data.moviesConnection,
 *   batchSize: 50,
 * })
 * 
 * return (
 *   <DataTable
 *     data={items}
 *     serverSide
 *     paginationMode="infinite"
 *     hasMore={hasMore}
 *     onLoadMore={loadMore}
 *     isLoadingMore={isLoadingMore}
 *   />
 * )
 * ```
 */
export function useInfiniteConnection<TData, TNode>({
  query,
  variables = {},
  getConnection,
  batchSize = DEFAULT_BATCH_SIZE,
  enabled = true,
  deps = [],
}: UseInfiniteConnectionOptions<TData, TNode>): UseInfiniteConnectionResult<TNode> {
  // State
  const [items, setItems] = useState<TNode[]>([])
  const [isLoading, setIsLoading] = useState(enabled)
  const [isLoadingMore, setIsLoadingMore] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [pageInfo, setPageInfo] = useState<PageInfo | null>(null)

  // Refs for tracking state across renders
  const isMountedRef = useRef(true)
  const endCursorRef = useRef<string | null>(null)
  const isLoadingRef = useRef(false) // Prevent duplicate requests

  // Serialize variables for dependency tracking (excluding pagination params)
  const variablesKey = JSON.stringify(variables)

  /**
   * Fetch a page of data
   */
  const fetchPage = useCallback(async (isInitial: boolean) => {
    // Prevent duplicate requests
    if (isLoadingRef.current) return
    isLoadingRef.current = true

    // Set appropriate loading state
    if (isInitial) {
      setIsLoading(true)
      setError(null)
      endCursorRef.current = null
    } else {
      setIsLoadingMore(true)
    }

    try {
      // Build query variables with pagination
      const paginatedVariables = {
        ...variables,
        first: batchSize,
        after: isInitial ? null : endCursorRef.current,
      }

      const result = await graphqlClient.query<TData>(query, paginatedVariables).toPromise()

      if (!isMountedRef.current) return

      if (result.error) {
        setError(sanitizeError(result.error))
        return
      }

      if (result.data) {
        const connection = getConnection(result.data)
        const newItems = connection.edges.map((edge: Edge<TNode>) => edge.node)

        // Update state
        if (isInitial) {
          setItems(newItems)
        } else {
          setItems(prev => [...prev, ...newItems])
        }

        setPageInfo(connection.pageInfo)
        endCursorRef.current = connection.pageInfo.endCursor
      }
    } catch (e) {
      if (isMountedRef.current) {
        setError(sanitizeError(e))
      }
    } finally {
      if (isMountedRef.current) {
        setIsLoading(false)
        setIsLoadingMore(false)
      }
      isLoadingRef.current = false
    }
  }, [query, variablesKey, batchSize, getConnection]) // eslint-disable-line react-hooks/exhaustive-deps

  /**
   * Load the next page
   */
  const loadMore = useCallback(() => {
    if (!pageInfo?.hasNextPage || isLoadingRef.current) return
    fetchPage(false)
  }, [fetchPage, pageInfo?.hasNextPage])

  /**
   * Refresh from the beginning
   */
  const refresh = useCallback(() => {
    setItems([])
    setPageInfo(null)
    endCursorRef.current = null
    fetchPage(true)
  }, [fetchPage])

  // Initial load and refetch on variable changes
  useEffect(() => {
    isMountedRef.current = true

    if (enabled) {
      // Reset state and fetch from beginning
      setItems([])
      setPageInfo(null)
      endCursorRef.current = null
      isLoadingRef.current = false
      fetchPage(true)
    } else {
      setIsLoading(false)
    }

    return () => {
      isMountedRef.current = false
    }
  }, [enabled, variablesKey, ...deps]) // eslint-disable-line react-hooks/exhaustive-deps

  return {
    items,
    isLoading,
    isLoadingMore,
    hasMore: pageInfo?.hasNextPage ?? false,
    totalCount: pageInfo?.totalCount ?? null,
    error,
    loadMore,
    refresh,
    pageInfo,
  }
}
