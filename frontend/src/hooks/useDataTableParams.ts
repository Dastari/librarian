import { useQueryState, parseAsString, parseAsStringLiteral } from 'nuqs'
import { useCallback, useMemo } from 'react'

// ============================================================================
// Types
// ============================================================================

export type SortDirection = 'asc' | 'desc'

export interface UseDataTableParamsOptions {
  /** Prefix for URL params (for multi-table pages, e.g., 'season1' -> 'season1_sort') */
  prefix?: string
  /** Default sort column (null means no sorting) */
  defaultSort?: string | null
  /** Default sort direction */
  defaultOrder?: SortDirection
  /** Default search term */
  defaultSearch?: string
}

export interface DataTableParams {
  /** Current sort column (null if no sorting) */
  sort: string | null
  /** Current sort direction */
  order: SortDirection
  /** Current search term */
  search: string
  /** Set the sort column */
  setSort: (sort: string | null) => void
  /** Set the sort direction */
  setOrder: (order: SortDirection) => void
  /** Set the search term */
  setSearch: (search: string) => void
  /** Combined sort change handler for DataTable */
  handleSortChange: (column: string | null, direction: SortDirection) => void
  /** Clear all params to defaults */
  reset: () => void
}

// ============================================================================
// Hook Implementation
// ============================================================================

/**
 * Hook for managing DataTable state via URL search params (nuqs).
 * 
 * Features:
 * - Clean URLs: no params shown when values match defaults
 * - Prefixed params for multi-table pages
 * - Server-side sorting/filtering support
 * 
 * @example
 * ```tsx
 * // Basic usage
 * const { sort, order, search, handleSortChange, setSearch } = useDataTableParams({
 *   defaultSort: 'name',
 *   defaultOrder: 'asc',
 * })
 * 
 * // With prefix for multi-table pages
 * const seasonParams = useDataTableParams({
 *   prefix: 'season1',
 *   defaultSort: 'episode',
 * })
 * // Results in URL params like: ?season1_sort=episode&season1_order=asc
 * ```
 */
export function useDataTableParams(options: UseDataTableParamsOptions = {}): DataTableParams {
  const {
    prefix,
    defaultSort = null,
    defaultOrder = 'asc',
    defaultSearch = '',
  } = options

  // Build param names with optional prefix
  const paramPrefix = prefix ? `${prefix}_` : ''
  const sortParam = `${paramPrefix}sort`
  const orderParam = `${paramPrefix}order`
  const searchParam = `${paramPrefix}q`

  // Sort column - uses parseAsString, null when cleared
  const [sort, setSortRaw] = useQueryState(sortParam, 
    parseAsString.withDefault(defaultSort ?? '')
  )
  
  // Sort direction - uses parseAsStringLiteral for type safety
  const [order, setOrderRaw] = useQueryState(orderParam,
    parseAsStringLiteral(['asc', 'desc'] as const).withDefault(defaultOrder)
  )
  
  // Search term
  const [search, setSearchRaw] = useQueryState(searchParam,
    parseAsString.withDefault(defaultSearch)
  )

  // Normalize sort: empty string becomes null
  const normalizedSort = sort === '' ? null : sort

  // Setters with proper typing
  const setSort = useCallback((value: string | null) => {
    setSortRaw(value === null ? '' : value)
  }, [setSortRaw])

  const setOrder = useCallback((value: SortDirection) => {
    setOrderRaw(value)
  }, [setOrderRaw])

  const setSearch = useCallback((value: string) => {
    setSearchRaw(value || '')
  }, [setSearchRaw])

  // Combined sort change handler for DataTable
  const handleSortChange = useCallback((column: string | null, direction: SortDirection) => {
    // Use Promise.all to batch the updates (nuqs supports this)
    setSortRaw(column === null ? '' : column)
    setOrderRaw(direction)
  }, [setSortRaw, setOrderRaw])

  // Reset all params to defaults
  const reset = useCallback(() => {
    setSortRaw(defaultSort ?? '')
    setOrderRaw(defaultOrder)
    setSearchRaw(defaultSearch)
  }, [setSortRaw, setOrderRaw, setSearchRaw, defaultSort, defaultOrder, defaultSearch])

  return useMemo(() => ({
    sort: normalizedSort,
    order,
    search,
    setSort,
    setOrder,
    setSearch,
    handleSortChange,
    reset,
  }), [normalizedSort, order, search, setSort, setOrder, setSearch, handleSortChange, reset])
}

// ============================================================================
// Helper Types for GraphQL Integration
// ============================================================================

export interface GraphQLOrderByInput {
  field: string
  direction: 'ASC' | 'DESC'
}

/**
 * Convert DataTableParams to GraphQL orderBy input.
 * Returns null if no sorting is active.
 */
export function toGraphQLOrderBy(params: Pick<DataTableParams, 'sort' | 'order'>): GraphQLOrderByInput | null {
  if (!params.sort) return null
  return {
    field: params.sort,
    direction: params.order.toUpperCase() as 'ASC' | 'DESC',
  }
}
