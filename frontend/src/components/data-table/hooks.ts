import { useState, useCallback, useEffect, useMemo, useRef, type Key } from 'react'
import type {
  DataTableState,
  SortDirection,
  ViewMode,
  FilterValues,
  DataTableColumn,
} from './types'

// ============================================================================
// State Persistence Hook
// ============================================================================

interface UseDataTableStateOptions {
  stateKey?: string
  defaultSortColumn?: string | null
  defaultSortDirection?: SortDirection
  defaultViewMode?: ViewMode
  defaultPageSize?: number
  /** Default column order (array of column keys) */
  defaultColumnOrder?: string[]
  /** Default column widths (map of column key to width) */
  defaultColumnWidths?: Record<string, number>
}

interface UseDataTableStateReturn {
  // Sort state
  sortColumn: string | null
  sortDirection: SortDirection
  setSortColumn: (column: string | null) => void
  setSortDirection: (direction: SortDirection) => void
  handleSort: (column: string) => void

  // Filter state
  filterValues: FilterValues
  setFilterValues: (values: FilterValues) => void
  setFilterValue: (key: string, value: unknown) => void
  clearFilters: () => void
  hasActiveFilters: boolean

  // View mode
  viewMode: ViewMode
  setViewMode: (mode: ViewMode) => void

  // Selection
  selectedKeys: Set<Key>
  setSelectedKeys: (keys: Set<Key>) => void
  clearSelection: () => void

  // Page size
  pageSize: number
  setPageSize: (size: number) => void

  // Column order (for reordering)
  columnOrder: string[]
  setColumnOrder: (order: string[]) => void

  // Column widths (for resizing)
  columnWidths: Record<string, number>
  setColumnWidths: (widths: Record<string, number>) => void
  setColumnWidth: (columnKey: string, width: number) => void
}

const STORAGE_KEY_PREFIX = 'data-table-state:'

function loadState(stateKey: string): Partial<DataTableState> | null {
  try {
    const stored = localStorage.getItem(`${STORAGE_KEY_PREFIX}${stateKey}`)
    if (stored) {
      const parsed = JSON.parse(stored)
      // Convert selectedKeys back to Set if needed
      if (parsed.selectedKeys) {
        parsed.selectedKeys = new Set(parsed.selectedKeys)
      }
      return parsed
    }
  } catch {
    // Ignore errors
  }
  return null
}

function saveState(stateKey: string, state: Partial<DataTableState>) {
  try {
    // Convert Set to Array for JSON serialization
    const toSave = {
      ...state,
      selectedKeys: state.selectedKeys ? Array.from(state.selectedKeys) : [],
    }
    localStorage.setItem(`${STORAGE_KEY_PREFIX}${stateKey}`, JSON.stringify(toSave))
  } catch {
    // Ignore errors (e.g., localStorage full)
  }
}

export function useDataTableState(options: UseDataTableStateOptions = {}): UseDataTableStateReturn {
  const {
    stateKey,
    defaultSortColumn = null,
    defaultSortDirection = 'asc',
    defaultViewMode = 'table',
    defaultPageSize = 20,
    defaultColumnOrder = [],
    defaultColumnWidths = {},
  } = options

  // Load initial state from localStorage if stateKey is provided
  const initialState = useMemo(() => {
    if (stateKey) {
      return loadState(stateKey)
    }
    return null
  }, [stateKey])

  // State
  const [sortColumn, setSortColumn] = useState<string | null>(
    initialState?.sortColumn ?? defaultSortColumn
  )
  const [sortDirection, setSortDirection] = useState<SortDirection>(
    initialState?.sortDirection ?? defaultSortDirection
  )
  const [filterValues, setFilterValues] = useState<FilterValues>(
    initialState?.filterValues ?? {}
  )
  const [viewMode, setViewMode] = useState<ViewMode>(
    initialState?.viewMode ?? defaultViewMode
  )
  const [selectedKeys, setSelectedKeys] = useState<Set<Key>>(
    initialState?.selectedKeys ?? new Set()
  )
  const [pageSize, setPageSize] = useState<number>(
    initialState?.pageSize ?? defaultPageSize
  )
  const [columnOrder, setColumnOrder] = useState<string[]>(
    initialState?.columnOrder ?? defaultColumnOrder
  )
  const [columnWidths, setColumnWidths] = useState<Record<string, number>>(
    initialState?.columnWidths ?? defaultColumnWidths
  )

  // Persist state changes
  const isFirstMount = useRef(true)
  useEffect(() => {
    // Skip on first mount (we just loaded from storage)
    if (isFirstMount.current) {
      isFirstMount.current = false
      return
    }

    if (stateKey) {
      saveState(stateKey, {
        sortColumn,
        sortDirection,
        filterValues,
        viewMode,
        pageSize,
        columnOrder,
        columnWidths,
        // Don't persist selection - it should reset on navigation
      })
    }
  }, [stateKey, sortColumn, sortDirection, filterValues, viewMode, pageSize, columnOrder, columnWidths])

  // Handle sort toggle
  const handleSort = useCallback(
    (column: string) => {
      if (sortColumn === column) {
        setSortDirection((prev) => (prev === 'asc' ? 'desc' : 'asc'))
      } else {
        setSortColumn(column)
        setSortDirection('asc')
      }
    },
    [sortColumn]
  )

  // Filter helpers
  const setFilterValue = useCallback((key: string, value: unknown) => {
    setFilterValues((prev) => ({ ...prev, [key]: value }))
  }, [])

  const clearFilters = useCallback(() => {
    setFilterValues({})
  }, [])

  const hasActiveFilters = useMemo(() => {
    return Object.values(filterValues).some((value) => {
      if (value === null || value === undefined || value === '') return false
      if (Array.isArray(value) && value.length === 0) return false
      return true
    })
  }, [filterValues])

  const clearSelection = useCallback(() => {
    setSelectedKeys(new Set())
  }, [])

  // Column width helper
  const setColumnWidth = useCallback((columnKey: string, width: number) => {
    setColumnWidths((prev) => ({ ...prev, [columnKey]: width }))
  }, [])

  return {
    sortColumn,
    sortDirection,
    setSortColumn,
    setSortDirection,
    handleSort,
    filterValues,
    setFilterValues,
    setFilterValue,
    clearFilters,
    hasActiveFilters,
    viewMode,
    setViewMode,
    selectedKeys,
    setSelectedKeys,
    clearSelection,
    pageSize,
    setPageSize,
    columnOrder,
    setColumnOrder,
    columnWidths,
    setColumnWidths,
    setColumnWidth,
  }
}

// ============================================================================
// Search/Filtering Hook
// ============================================================================

/**
 * Hook to filter data by search term.
 * 
 * Note: Complex filtering (status filters, etc.) should be done by the caller
 * before passing data to DataTable. This hook only handles search functionality.
 */
export function useFilteredData<T>(
  data: T[],
  searchTerm: string,
  searchFn?: (item: T, term: string) => boolean
): T[] {
  return useMemo(() => {
    if (!searchTerm) return data

    const lowerSearch = searchTerm.toLowerCase()
    if (searchFn) {
      return data.filter((item) => searchFn(item, searchTerm))
    }
    
    // Default: search all string properties
    return data.filter((item) => {
      return Object.values(item as Record<string, unknown>).some((value) => {
        if (typeof value === 'string') {
          return value.toLowerCase().includes(lowerSearch)
        }
        return false
      })
    })
  }, [data, searchTerm, searchFn])
}

// ============================================================================
// Sorting Hook
// ============================================================================

export function useSortedData<T>(
  data: T[],
  columns: DataTableColumn<T>[],
  sortColumn: string | null,
  sortDirection: SortDirection,
  defaultSortFn?: (a: T, b: T, column: string) => number
): T[] {
  return useMemo(() => {
    if (!sortColumn) return data

    const column = columns.find((c) => c.key === sortColumn)
    if (!column || column.sortable === false) return data

    const sorted = [...data].sort((a, b) => {
      let comparison = 0

      if (column.sortFn) {
        comparison = column.sortFn(a, b)
      } else if (defaultSortFn) {
        comparison = defaultSortFn(a, b, sortColumn)
      } else {
        // Default sort by column key
        const aVal = (a as Record<string, unknown>)[sortColumn]
        const bVal = (b as Record<string, unknown>)[sortColumn]

        if (aVal === null || aVal === undefined) comparison = 1
        else if (bVal === null || bVal === undefined) comparison = -1
        else if (typeof aVal === 'string' && typeof bVal === 'string') {
          comparison = aVal.localeCompare(bVal)
        } else if (typeof aVal === 'number' && typeof bVal === 'number') {
          comparison = aVal - bVal
        } else {
          comparison = String(aVal).localeCompare(String(bVal))
        }
      }

      return sortDirection === 'desc' ? -comparison : comparison
    })

    return sorted
  }, [data, columns, sortColumn, sortDirection, defaultSortFn])
}

// ============================================================================
// Pagination Hook
// ============================================================================

interface UsePaginationOptions {
  totalItems: number
  pageSize: number
  defaultPage?: number
}

interface UsePaginationReturn {
  page: number
  setPage: (page: number) => void
  totalPages: number
  startIndex: number
  endIndex: number
  goToFirstPage: () => void
  goToLastPage: () => void
  goToNextPage: () => void
  goToPrevPage: () => void
  canGoNext: boolean
  canGoPrev: boolean
}

export function usePagination(options: UsePaginationOptions): UsePaginationReturn {
  const { totalItems, pageSize, defaultPage = 1 } = options
  const [page, setPage] = useState(defaultPage)

  const totalPages = Math.max(1, Math.ceil(totalItems / pageSize))

  // Reset to page 1 if current page is out of bounds
  useEffect(() => {
    if (page > totalPages) {
      setPage(1)
    }
  }, [page, totalPages])

  const startIndex = (page - 1) * pageSize
  const endIndex = Math.min(startIndex + pageSize, totalItems)

  const goToFirstPage = useCallback(() => setPage(1), [])
  const goToLastPage = useCallback(() => setPage(totalPages), [totalPages])
  const goToNextPage = useCallback(() => setPage((p) => Math.min(p + 1, totalPages)), [totalPages])
  const goToPrevPage = useCallback(() => setPage((p) => Math.max(p - 1, 1)), [])

  return {
    page,
    setPage,
    totalPages,
    startIndex,
    endIndex,
    goToFirstPage,
    goToLastPage,
    goToNextPage,
    goToPrevPage,
    canGoNext: page < totalPages,
    canGoPrev: page > 1,
  }
}

// ============================================================================
// Infinite Scroll Hook
// ============================================================================

interface UseInfiniteScrollOptions {
  hasMore: boolean
  isLoading: boolean
  onLoadMore: () => void
  /** Current data length - used to detect when loads produce no visible results */
  dataLength?: number
  /** Intersection threshold (0-1), default 0.1 */
  threshold?: number
  /** Known total count - if dataLength >= totalCount, don't try to load more */
  totalCount?: number | null
}

/**
 * Simple infinite scroll hook based on IntersectionObserver.
 * Uses a regular ref + effect pattern for stability.
 */
export function useInfiniteScroll(options: UseInfiniteScrollOptions) {
  const {
    hasMore,
    isLoading,
    onLoadMore,
    dataLength = 0,
    threshold = 0.1,
    totalCount = null,
  } = options
  
  // Regular ref for the sentinel element - more stable than callback ref
  const sentinelRef = useRef<HTMLDivElement | null>(null)
  const observerRef = useRef<IntersectionObserver | null>(null)
  const lastDataLength = useRef<number>(0)
  const loadMoreAttempts = useRef<number>(0)
  const lastLoadTime = useRef<number>(0)
  
  // Minimum time between loads (ms) - prevents rapid-fire even if observer misbehaves
  const LOAD_COOLDOWN_MS = 500
  
  // Use refs for values checked in the observer callback
  const isLoadingRef = useRef(isLoading)
  const hasMoreRef = useRef(hasMore)
  const dataLengthRef = useRef(dataLength)
  const totalCountRef = useRef(totalCount)
  const onLoadMoreRef = useRef(onLoadMore)
  
  // Keep refs updated
  isLoadingRef.current = isLoading
  hasMoreRef.current = hasMore
  dataLengthRef.current = dataLength
  totalCountRef.current = totalCount
  onLoadMoreRef.current = onLoadMore
  
  // Track data length changes to reset the attempts counter
  useEffect(() => {
    if (dataLength !== lastDataLength.current) {
      loadMoreAttempts.current = 0
      lastDataLength.current = dataLength
    }
  }, [dataLength])

  // Set up observer - recreate only when onLoadMore changes
  useEffect(() => {
    if (!onLoadMore) return

    // Clean up previous observer
    if (observerRef.current) {
      observerRef.current.disconnect()
    }

    const observer = new IntersectionObserver(
      (entries) => {
        const entry = entries[0]
        
        // Only trigger on intersection, not on un-intersection
        if (!entry.isIntersecting) return
        
        // Check all conditions via refs for fresh values
        if (isLoadingRef.current) {
          if (import.meta.env.DEV) console.log('[InfiniteScroll] Blocked: isLoading')
          return
        }
        if (!hasMoreRef.current) {
          if (import.meta.env.DEV) console.log('[InfiniteScroll] Blocked: !hasMore')
          return
        }
        if (totalCountRef.current !== null && dataLengthRef.current >= totalCountRef.current) {
          if (import.meta.env.DEV) console.log('[InfiniteScroll] Blocked: dataLength >= totalCount')
          return
        }

        // Cooldown check - prevent rapid-fire loads
        const now = Date.now()
        const timeSinceLastLoad = now - lastLoadTime.current
        if (timeSinceLastLoad < LOAD_COOLDOWN_MS) {
          if (import.meta.env.DEV) console.log(`[InfiniteScroll] Blocked: cooldown (${timeSinceLastLoad}ms < ${LOAD_COOLDOWN_MS}ms)`)
          return
        }

        // Guard against infinite loops when no data
        if (dataLengthRef.current === 0 && loadMoreAttempts.current >= 3) {
          if (import.meta.env.DEV) console.log('[InfiniteScroll] Blocked: max attempts with 0 data')
          return
        }

        if (import.meta.env.DEV) console.log(`[InfiniteScroll] Loading more... (dataLength: ${dataLengthRef.current}, attempts: ${loadMoreAttempts.current})`)
        loadMoreAttempts.current++
        lastLoadTime.current = now
        onLoadMoreRef.current()
      },
      { threshold }
    )

    observerRef.current = observer

    // Observe the sentinel if it exists
    if (sentinelRef.current) {
      observer.observe(sentinelRef.current)
    }

    return () => observer.disconnect()
  }, [onLoadMore, threshold])

  // Separate effect to handle sentinel element changes
  useEffect(() => {
    const sentinel = sentinelRef.current
    const observer = observerRef.current
    
    if (sentinel && observer) {
      // Make sure we're observing the current sentinel
      observer.observe(sentinel)
    }
    
    return () => {
      if (sentinel && observer) {
        observer.unobserve(sentinel)
      }
    }
  }) // Run on every render to catch sentinel changes

  return { sentinelRef }
}

// ============================================================================
// Column Reorder Hook
// ============================================================================

interface UseColumnReorderOptions {
  /** Array of column keys in current order */
  columnOrder: string[]
  /** Callback when order changes */
  onOrderChange: (newOrder: string[]) => void
}

interface UseColumnReorderReturn {
  /** Drag and drop handlers for a column header */
  getDragProps: (columnKey: string) => {
    draggable: boolean
    onDragStart: (e: React.DragEvent) => void
    onDragOver: (e: React.DragEvent) => void
    onDragEnter: (e: React.DragEvent) => void
    onDragLeave: (e: React.DragEvent) => void
    onDrop: (e: React.DragEvent) => void
    onDragEnd: (e: React.DragEvent) => void
    'data-column-key': string
  }
  /** Currently dragging column key */
  draggingColumn: string | null
  /** Column currently being dragged over */
  dragOverColumn: string | null
}

export function useColumnReorder(options: UseColumnReorderOptions): UseColumnReorderReturn {
  const { columnOrder, onOrderChange } = options
  const [draggingColumn, setDraggingColumn] = useState<string | null>(null)
  const [dragOverColumn, setDragOverColumn] = useState<string | null>(null)

  const getDragProps = useCallback(
    (columnKey: string) => ({
      draggable: true,
      'data-column-key': columnKey,
      onDragStart: (e: React.DragEvent) => {
        setDraggingColumn(columnKey)
        e.dataTransfer.effectAllowed = 'move'
        e.dataTransfer.setData('text/plain', columnKey)
        // Add a slight delay to allow the drag image to be captured
        requestAnimationFrame(() => {
          const target = e.target as HTMLElement
          target.style.opacity = '0.5'
        })
      },
      onDragOver: (e: React.DragEvent) => {
        e.preventDefault()
        e.dataTransfer.dropEffect = 'move'
      },
      onDragEnter: (e: React.DragEvent) => {
        e.preventDefault()
        if (draggingColumn && columnKey !== draggingColumn) {
          setDragOverColumn(columnKey)
        }
      },
      onDragLeave: (e: React.DragEvent) => {
        // Only clear if we're actually leaving the element (not entering a child)
        const relatedTarget = e.relatedTarget as HTMLElement
        const currentTarget = e.currentTarget as HTMLElement
        if (!currentTarget.contains(relatedTarget)) {
          setDragOverColumn(null)
        }
      },
      onDrop: (e: React.DragEvent) => {
        e.preventDefault()
        const sourceKey = e.dataTransfer.getData('text/plain')
        if (sourceKey && sourceKey !== columnKey) {
          const newOrder = [...columnOrder]
          const sourceIndex = newOrder.indexOf(sourceKey)
          const targetIndex = newOrder.indexOf(columnKey)
          if (sourceIndex !== -1 && targetIndex !== -1) {
            // Remove from source position and insert at target
            newOrder.splice(sourceIndex, 1)
            newOrder.splice(targetIndex, 0, sourceKey)
            onOrderChange(newOrder)
          }
        }
        setDragOverColumn(null)
        setDraggingColumn(null)
      },
      onDragEnd: (e: React.DragEvent) => {
        const target = e.target as HTMLElement
        target.style.opacity = '1'
        setDraggingColumn(null)
        setDragOverColumn(null)
      },
    }),
    [columnOrder, onOrderChange, draggingColumn]
  )

  return { getDragProps, draggingColumn, dragOverColumn }
}

// ============================================================================
// Column Resize Hook
// ============================================================================

interface UseColumnResizeOptions {
  /** Current column widths */
  columnWidths: Record<string, number>
  /** Callback when widths change */
  onWidthChange: (widths: Record<string, number>) => void
  /** Minimum width (default: 50) */
  minWidth?: number
  /** Maximum width */
  maxWidth?: number
}

interface UseColumnResizeReturn {
  /** Get resize handle props for a column */
  getResizeHandleProps: (columnKey: string, initialWidth: number) => {
    onMouseDown: (e: React.MouseEvent) => void
    className: string
  }
  /** Currently resizing column key */
  resizingColumn: string | null
}

export function useColumnResize(options: UseColumnResizeOptions): UseColumnResizeReturn {
  const { columnWidths, onWidthChange, minWidth = 50, maxWidth } = options
  const [resizingColumn, setResizingColumn] = useState<string | null>(null)
  const startXRef = useRef<number>(0)
  const startWidthRef = useRef<number>(0)

  const getResizeHandleProps = useCallback(
    (columnKey: string, initialWidth: number) => ({
      className:
        'absolute right-0 top-0 h-full w-1 cursor-col-resize hover:bg-primary/50 active:bg-primary transition-colors group-hover:bg-default-300',
      onMouseDown: (e: React.MouseEvent) => {
        e.preventDefault()
        e.stopPropagation()
        setResizingColumn(columnKey)
        startXRef.current = e.clientX
        startWidthRef.current = columnWidths[columnKey] ?? initialWidth

        const handleMouseMove = (moveEvent: MouseEvent) => {
          const delta = moveEvent.clientX - startXRef.current
          let newWidth = startWidthRef.current + delta
          
          // Apply constraints
          newWidth = Math.max(minWidth, newWidth)
          if (maxWidth) {
            newWidth = Math.min(maxWidth, newWidth)
          }

          onWidthChange({ ...columnWidths, [columnKey]: newWidth })
        }

        const handleMouseUp = () => {
          setResizingColumn(null)
          document.removeEventListener('mousemove', handleMouseMove)
          document.removeEventListener('mouseup', handleMouseUp)
          document.body.style.cursor = ''
          document.body.style.userSelect = ''
        }

        document.addEventListener('mousemove', handleMouseMove)
        document.addEventListener('mouseup', handleMouseUp)
        document.body.style.cursor = 'col-resize'
        document.body.style.userSelect = 'none'
      },
    }),
    [columnWidths, onWidthChange, minWidth, maxWidth]
  )

  return { getResizeHandleProps, resizingColumn }
}
