import { useState, useEffect, useCallback, useRef } from 'react'
import { graphqlClient } from '../lib/graphql'
import { sanitizeError } from '../lib/format'

// ============================================================================
// useGraphQLQuery Hook
// ============================================================================

export interface UseGraphQLQueryOptions {
  /** Skip the initial fetch (useful for conditional queries) */
  skip?: boolean
  /** Refetch when these dependencies change */
  deps?: unknown[]
}

export interface UseGraphQLQueryResult<T> {
  /** The query result data */
  data: T | null
  /** Whether the query is currently loading */
  isLoading: boolean
  /** Error message if the query failed */
  error: string | null
  /** Manually refetch the query */
  refetch: () => Promise<void>
}

/**
 * Hook for executing GraphQL queries with loading/error state management.
 * 
 * @example
 * const { data, isLoading, error, refetch } = useGraphQLQuery<{ torrents: Torrent[] }>(
 *   TORRENTS_QUERY,
 *   { limit: 10 }
 * )
 */
export function useGraphQLQuery<T>(
  query: string,
  variables?: Record<string, unknown>,
  options?: UseGraphQLQueryOptions
): UseGraphQLQueryResult<T> {
  const [data, setData] = useState<T | null>(null)
  const [isLoading, setIsLoading] = useState(!options?.skip)
  const [error, setError] = useState<string | null>(null)
  
  // Use ref to track if component is mounted
  const isMountedRef = useRef(true)
  
  // Serialize variables for dependency tracking
  const variablesKey = JSON.stringify(variables ?? {})

  const refetch = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    try {
      const result = await graphqlClient.query<T>(query, variables).toPromise()
      if (!isMountedRef.current) return
      
      if (result.error) {
        setError(sanitizeError(result.error))
      } else if (result.data) {
        setData(result.data)
      }
    } catch (e) {
      if (isMountedRef.current) {
        setError(sanitizeError(e))
      }
    } finally {
      if (isMountedRef.current) {
        setIsLoading(false)
      }
    }
  }, [query, variablesKey]) // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    isMountedRef.current = true
    
    if (!options?.skip) {
      refetch()
    }
    
    return () => {
      isMountedRef.current = false
    }
  }, [refetch, options?.skip, ...(options?.deps ?? [])])

  return { data, isLoading, error, refetch }
}

// ============================================================================
// useMutation Hook
// ============================================================================

export interface UseMutationResult<T, V> {
  /** Execute the mutation */
  mutate: (variables: V) => Promise<T | null>
  /** Whether the mutation is currently in progress */
  isLoading: boolean
  /** Error message if the mutation failed */
  error: string | null
  /** Reset the error state */
  resetError: () => void
}

/**
 * Hook for executing GraphQL mutations with loading/error state management.
 * 
 * @example
 * const { mutate, isLoading, error } = useMutation<AddTorrentResult, AddTorrentInput>(
 *   ADD_TORRENT_MUTATION
 * )
 * 
 * const handleSubmit = async () => {
 *   const result = await mutate({ magnet: '...' })
 *   if (result?.success) {
 *     // Handle success
 *   }
 * }
 */
export function useMutation<T, V extends Record<string, unknown> = Record<string, unknown>>(
  mutation: string
): UseMutationResult<T, V> {
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const mutate = useCallback(async (variables: V): Promise<T | null> => {
    setIsLoading(true)
    setError(null)
    try {
      const result = await graphqlClient.mutation<T>(mutation, variables as Record<string, unknown>).toPromise()
      if (result.error) {
        const errorMsg = sanitizeError(result.error)
        setError(errorMsg)
        return null
      }
      return result.data ?? null
    } catch (e) {
      const errorMsg = sanitizeError(e)
      setError(errorMsg)
      return null
    } finally {
      setIsLoading(false)
    }
  }, [mutation])

  const resetError = useCallback(() => {
    setError(null)
  }, [])

  return { mutate, isLoading, error, resetError }
}

// ============================================================================
// useLazyQuery Hook
// ============================================================================

export interface UseLazyQueryResult<T, V> {
  /** Execute the query */
  execute: (variables?: V) => Promise<T | null>
  /** The query result data */
  data: T | null
  /** Whether the query is currently loading */
  isLoading: boolean
  /** Error message if the query failed */
  error: string | null
  /** Reset the state */
  reset: () => void
}

/**
 * Hook for lazily executing GraphQL queries (executed on demand, not on mount).
 * 
 * @example
 * const { execute, data, isLoading } = useLazyQuery<{ tvShow: TvShow }>(TV_SHOW_QUERY)
 * 
 * const handleSearch = async () => {
 *   const result = await execute({ id: showId })
 *   if (result?.tvShow) {
 *     // Handle result
 *   }
 * }
 */
export function useLazyQuery<T, V extends Record<string, unknown> = Record<string, unknown>>(
  query: string
): UseLazyQueryResult<T, V> {
  const [data, setData] = useState<T | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const execute = useCallback(async (variables?: V): Promise<T | null> => {
    setIsLoading(true)
    setError(null)
    try {
      const result = await graphqlClient.query<T>(query, variables as Record<string, unknown>).toPromise()
      if (result.error) {
        const errorMsg = sanitizeError(result.error)
        setError(errorMsg)
        return null
      }
      if (result.data) {
        setData(result.data)
        return result.data
      }
      return null
    } catch (e) {
      const errorMsg = sanitizeError(e)
      setError(errorMsg)
      return null
    } finally {
      setIsLoading(false)
    }
  }, [query])

  const reset = useCallback(() => {
    setData(null)
    setError(null)
    setIsLoading(false)
  }, [])

  return { execute, data, isLoading, error, reset }
}
