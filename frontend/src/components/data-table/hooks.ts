import { useState, useCallback, useEffect, useMemo, useRef, type Key } from 'react'
import type {
  DataTableState,
  SortDirection,
  ViewMode,
  FilterValues,
  DataTableColumn,
  DataTableFilter,
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
// Filtering Hook
// ============================================================================

export function useFilteredData<T>(
  data: T[],
  filters: DataTableFilter<T>[],
  filterValues: FilterValues,
  searchTerm: string,
  searchFn?: (item: T, term: string) => boolean
): T[] {
  return useMemo(() => {
    let result = data

    // Apply search filter
    if (searchTerm) {
      const lowerSearch = searchTerm.toLowerCase()
      if (searchFn) {
        result = result.filter((item) => searchFn(item, searchTerm))
      } else {
        // Default: search all string properties
        result = result.filter((item) => {
          return Object.values(item as Record<string, unknown>).some((value) => {
            if (typeof value === 'string') {
              return value.toLowerCase().includes(lowerSearch)
            }
            return false
          })
        })
      }
    }

    // Apply each filter
    for (const filter of filters) {
      const value = filterValues[filter.key]
      if (value === null || value === undefined || value === '') continue
      if (Array.isArray(value) && value.length === 0) continue

      if (filter.filterFn) {
        result = result.filter((item) => filter.filterFn!(item, value))
      }
    }

    return result
  }, [data, filters, filterValues, searchTerm, searchFn])
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
  /** Maximum attempts to load when data length doesn't change (guards against infinite loops) */
  maxAttempts?: number
}

export function useInfiniteScroll(options: UseInfiniteScrollOptions) {
  const {
    hasMore,
    isLoading,
    onLoadMore,
    dataLength = 0,
    threshold = 0.1,
    maxAttempts = 3,
  } = options
  
  const observerRef = useRef<IntersectionObserver | null>(null)
  const lastDataLength = useRef<number>(dataLength)
  const loadMoreAttempts = useRef<number>(0)
  
  // Store latest values in refs so the observer callback always has fresh data
  const isLoadingRef = useRef(isLoading)
  const hasMoreRef = useRef(hasMore)
  const dataLengthRef = useRef(dataLength)
  const onLoadMoreRef = useRef(onLoadMore)
  
  // Keep refs updated
  isLoadingRef.current = isLoading
  hasMoreRef.current = hasMore
  dataLengthRef.current = dataLength
  onLoadMoreRef.current = onLoadMore

  // Track data length changes to detect when load more produces no visible results
  useEffect(() => {
    if (dataLength !== lastDataLength.current) {
      // Data changed, reset attempts counter
      loadMoreAttempts.current = 0
      lastDataLength.current = dataLength
    }
  }, [dataLength])

  // Callback ref that sets up the IntersectionObserver when the element mounts
  const sentinelRef = useCallback(
    (node: HTMLDivElement | null) => {
      // Disconnect previous observer
      if (observerRef.current) {
        observerRef.current.disconnect()
        observerRef.current = null
      }

      // Don't observe if no node or no more items
      if (!node || !hasMore) return

      observerRef.current = new IntersectionObserver(
        (entries) => {
          if (entries[0].isIntersecting && !isLoadingRef.current && hasMoreRef.current) {
            // Guard against infinite loops when filters hide all data:
            // If we've tried loading more maxAttempts times without dataLength changing,
            // stop attempting to load more until data or filters change.
            if (dataLengthRef.current === 0 && loadMoreAttempts.current >= maxAttempts) {
              return
            }
            loadMoreAttempts.current++
            onLoadMoreRef.current()
          }
        },
        { threshold }
      )

      observerRef.current.observe(node)
    },
    [hasMore, threshold, maxAttempts]
  )

  // Clean up observer on unmount
  useEffect(() => {
    return () => {
      if (observerRef.current) {
        observerRef.current.disconnect()
      }
    }
  }, [])

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
