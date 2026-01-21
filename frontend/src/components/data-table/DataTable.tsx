import { useMemo, useCallback, useState, type Key, type ReactNode } from 'react'
import { Table, TableHeader, TableColumn, TableBody, TableRow, TableCell, type SortDescriptor } from '@heroui/table'
import { Button, ButtonGroup } from '@heroui/button'
import { Input } from '@heroui/input'
import { Switch } from '@heroui/switch'
import { Dropdown, DropdownTrigger, DropdownMenu, DropdownItem } from '@heroui/dropdown'
import { Tooltip } from '@heroui/tooltip'
import { Spinner } from '@heroui/spinner'
import { Pagination } from '@heroui/pagination'
import { Select, SelectItem } from '@heroui/select'
import { Divider } from '@heroui/divider'
import { Checkbox } from '@heroui/checkbox'
import { Skeleton } from '@heroui/skeleton'
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter, useDisclosure } from '@heroui/modal'
import type {
  DataTableProps,
  DataTableColumn,
  DataTableGroup,
  RowAction,
  ViewMode,
  SortDirection,
  ColumnSizing,
} from './types'
import {
  useDataTableState,
  useFilteredData,
  useSortedData,
  usePagination,
  useInfiniteScroll,
  useColumnReorder,
  useColumnResize,
} from './hooks'

// ============================================================================
// Column Sizing Utilities
// ============================================================================

/** Parse column width configuration into a ColumnSizing object */
function parseColumnSizing<T>(column: DataTableColumn<T>): ColumnSizing {
  const { width } = column
  if (width === undefined) {
    return { grow: 1 } // Default: flexible, grows to fill space
  }
  if (typeof width === 'number') {
    return { width, grow: 0 } // Fixed width, no grow
  }
  if (typeof width === 'string') {
    // Legacy string support - try to parse as px
    const match = width.match(/^(\d+)px$/)
    if (match) {
      return { width: parseInt(match[1], 10), grow: 0 }
    }
    // For percentages or other CSS values, just treat as flexible
    return { grow: 1 }
  }
  // It's a ColumnSizing object
  return width
}

/** Calculate column styles based on sizing configuration */
function getColumnStyle(
  sizing: ColumnSizing,
  overrideWidth?: number
): React.CSSProperties {
  const style: React.CSSProperties = {}
  
  if (overrideWidth !== undefined) {
    // User has resized this column
    style.width = overrideWidth
    style.minWidth = overrideWidth
    style.maxWidth = overrideWidth
    style.flexGrow = 0
    style.flexShrink = 0
  } else if (sizing.width !== undefined) {
    // Fixed width column
    style.width = sizing.width
    style.minWidth = sizing.minWidth ?? sizing.width
    style.maxWidth = sizing.maxWidth ?? sizing.width
    style.flexGrow = 0
    style.flexShrink = 0
  } else {
    // Flexible column
    style.flexGrow = sizing.grow ?? 1
    style.flexShrink = 1
    style.flexBasis = 0
    if (sizing.minWidth) {
      style.minWidth = sizing.minWidth
    } else {
      style.minWidth = 50 // Reasonable default minimum
    }
    if (sizing.maxWidth) {
      style.maxWidth = sizing.maxWidth
    }
  }
  
  return style
}
import {
  IconSearch,
  IconDotsVertical,
  IconTable,
  IconLayoutGrid,
} from '@tabler/icons-react'

// ============================================================================
// Sub-components
// ============================================================================

/** Resolve a dynamic label (string or function) to a string */
function resolveLabel<T>(label: string | ((item: T) => string), item: T): string {
  return typeof label === 'function' ? label(item) : label;
}

/** Resolve a dynamic icon (ReactNode or function) to a ReactNode */
function resolveIcon<T>(icon: React.ReactNode | ((item: T) => React.ReactNode) | undefined, item: T): React.ReactNode {
  return typeof icon === 'function' ? icon(item) : icon;
}

interface RowActionsDropdownProps<T> {
  item: T
  actions: RowAction<T>[]
}

function RowActionsDropdown<T>({ item, actions }: RowActionsDropdownProps<T>) {
  const visibleActions = actions.filter((action) =>
    action.isVisible ? action.isVisible(item) : true
  )
  const dropdownActions = visibleActions.filter((action) => action.inDropdown !== false)

  if (dropdownActions.length === 0) return null

  return (
    <Dropdown>
      <DropdownTrigger>
        <Button isIconOnly size="sm" variant="light" aria-label="More actions">
          <IconDotsVertical size={18} />
        </Button>
      </DropdownTrigger>
      <DropdownMenu aria-label="Row actions">
        {dropdownActions.map((action) => (
          <DropdownItem
            key={action.key}
            color={action.isDestructive ? 'danger' : action.color}
            className={action.isDestructive ? 'text-danger' : ''}
            startContent={resolveIcon(action.icon, item)}
            isDisabled={action.isDisabled ? action.isDisabled(item) : false}
            onPress={() => action.onAction(item)}
          >
            {resolveLabel(action.label, item)}
          </DropdownItem>
        ))}
      </DropdownMenu>
    </Dropdown>
  )
}

interface InlineRowActionsProps<T> {
  item: T
  actions: RowAction<T>[]
}

function InlineRowActions<T>({ item, actions }: InlineRowActionsProps<T>) {
  const visibleActions = actions.filter(
    (action) =>
      action.inDropdown === false && (action.isVisible ? action.isVisible(item) : true)
  )

  return (
    <>
      {visibleActions.map((action) => (
        <Tooltip key={action.key} content={resolveLabel(action.label, item)}>
          <Button
            isIconOnly
            size="sm"
            variant="light"
            color={action.isDestructive ? 'danger' : action.color}
            isDisabled={action.isDisabled ? action.isDisabled(item) : false}
            onPress={() => action.onAction(item)}
          >
            {resolveIcon(action.icon, item)}
          </Button>
        </Tooltip>
      ))}
    </>
  )
}

// ============================================================================
// Table View Wrapper - Handles fill height with sticky header
// ============================================================================

interface TableViewWrapperProps {
  fillHeight: boolean
  className?: string
  children: ReactNode
  /** Ref for infinite scroll sentinel - placed at the bottom of scroll area */
  sentinelRef?: React.RefObject<HTMLDivElement | null>
  /** Whether to show the sentinel */
  showSentinel?: boolean
}

function TableViewWrapper({ 
  fillHeight, 
  className, 
  children, 
  sentinelRef,
  showSentinel = false,
}: TableViewWrapperProps) {
  if (!fillHeight) {
    return (
      <div className={className}>
        {children}
        {/* Sentinel for infinite scroll - outside table, inside scroll area */}
        {showSentinel && (
          <div ref={sentinelRef} className="h-px w-full" />
        )}
      </div>
    )
  }

  // For fillHeight: we create our own scroll container that wraps the table
  // Apply the Table's wrapper styling here since we use removeWrapper on the Table
  // The sentinel is placed INSIDE the scroll container, after the table
  return (
    <div className={`grow h-0 overflow-auto p-4 bg-content1 shadow-small rounded-large ${className ?? ''}`}>
      {children}
      {/* Sentinel for infinite scroll - inside our scroll container, after table */}
      {showSentinel && (
        <div ref={sentinelRef} className="h-px w-full" />
      )}
    </div>
  )
}


// ============================================================================
// Main DataTable Component
// ============================================================================

export function DataTable<T>({
  stateKey,
  data,
  columns,
  getRowKey,
  isLoading = false,
  skeletonRowCount = 5,
  enableSkeletonTesting = false,
  emptyContent,
  isPinned,

  // Selection
  selectionMode = 'none',
  selectedKeys: controlledSelectedKeys,
  onSelectionChange,
  isRowSelectable,
  checkboxSelectionOnly = false,

  // Search
  searchFn,
  searchPlaceholder = 'Search...',

  // Sorting
  defaultSortColumn,
  defaultSortDirection = 'asc',
  sortColumn: controlledSortColumn,
  sortDirection: controlledSortDirection,
  onSortChange,
  defaultSortFn,

  // View Mode
  showViewModeToggle = false,
  defaultViewMode = 'table',
  viewMode: controlledViewMode,
  onViewModeChange,
  cardRenderer,
  cardGridClassName = 'grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4',
  groupBy,
  groupHeaderRenderer,

  // Actions
  bulkActions = [],
  rowActions = [],

  // Pagination
  paginationMode = 'none',
  pageSizeOptions = [10, 20, 50, 100],
  defaultPageSize = 20,
  onLoadMore,
  hasMore = false,
  isLoadingMore = false,

  // Server-side mode
  serverSide = false,
  serverTotalCount,
  onSearchChange,

  // Layout
  headerContent,
  footerContent,
  toolbarContent,
  toolbarContentPosition = 'end',
  filterRowContent,
  showItemCount = true,
  hideToolbar = false,
  classNames = {},
  fillHeight = false,

  // Table Props
  ariaLabel = 'Data table',
  removeWrapper = false,
  isStriped = false,
  isCompact = false,

  // Column Features
  enableColumnReorder = false,
  columnOrder: controlledColumnOrder,
  onColumnOrderChange,
  enableColumnResize: _enableColumnResize = false,
  columnWidths: controlledColumnWidths,
  onColumnWidthsChange,
  minColumnWidth = 50,
  maxColumnWidth,
}: DataTableProps<T>) {
  // ============================================================================
  // State Management
  // ============================================================================

  const tableState = useDataTableState({
    stateKey,
    defaultSortColumn,
    defaultSortDirection,
    defaultViewMode,
    defaultPageSize,
  })

  // Confirm modal state for bulk actions
  const { isOpen: isConfirmOpen, onOpen: onConfirmOpen, onClose: onConfirmClose } = useDisclosure()
  const [confirmAction, setConfirmAction] = useState<{
    message: string
    onConfirm: () => Promise<void> | void
  } | null>(null)

  // Skeleton testing state
  const [isSkeletonTesting, setIsSkeletonTesting] = useState(false)

  // Use controlled state or internal state
  const sortColumn = controlledSortColumn ?? tableState.sortColumn
  const sortDirection = controlledSortDirection ?? tableState.sortDirection
  const viewMode = controlledViewMode ?? tableState.viewMode
  const selectedKeys = controlledSelectedKeys ?? tableState.selectedKeys
  const pageSize = tableState.pageSize

  // Column order and widths
  const columnWidths = controlledColumnWidths ?? tableState.columnWidths
  
  // Compute default column order from columns if not specified
  const defaultColumnOrder = useMemo(
    () => columns.filter((c) => !c.hidden).map((c) => c.key),
    [columns]
  )
  const columnOrder = controlledColumnOrder ?? 
    (tableState.columnOrder.length > 0 ? tableState.columnOrder : defaultColumnOrder)

  // Column reorder hook
  const handleColumnOrderChange = useCallback(
    (newOrder: string[]) => {
      if (onColumnOrderChange) {
        onColumnOrderChange(newOrder)
      } else {
        tableState.setColumnOrder(newOrder)
      }
    },
    [onColumnOrderChange, tableState]
  )

  const { getDragProps, draggingColumn, dragOverColumn } = useColumnReorder({
    columnOrder,
    onOrderChange: handleColumnOrderChange,
  })

  // Column resize hook
  const handleColumnWidthsChange = useCallback(
    (newWidths: Record<string, number>) => {
      if (onColumnWidthsChange) {
        onColumnWidthsChange(newWidths)
      } else {
        tableState.setColumnWidths(newWidths)
      }
    },
    [onColumnWidthsChange, tableState]
  )

  const { resizingColumn } = useColumnResize({
    columnWidths,
    onWidthChange: handleColumnWidthsChange,
    minWidth: minColumnWidth,
    maxWidth: maxColumnWidth,
  })

  // Search term state (managed internally)
  const [searchTerm, setSearchTerm] = useState('')

  // Sort descriptor for HeroUI Table
  const sortDescriptor: SortDescriptor | undefined = useMemo(() => {
    if (!sortColumn) return undefined
    return {
      column: sortColumn,
      direction: sortDirection === 'asc' ? 'ascending' : 'descending',
    }
  }, [sortColumn, sortDirection])

  // Handle HeroUI Table sort change
  const handleTableSortChange = useCallback(
    (descriptor: SortDescriptor) => {
      const column = descriptor.column as string
      const direction: SortDirection = descriptor.direction === 'ascending' ? 'asc' : 'desc'
      if (onSortChange) {
        onSortChange(column, direction)
      } else {
        tableState.setSortColumn(column)
        tableState.setSortDirection(direction)
      }
    },
    [onSortChange, tableState]
  )

  const handleSearchChange = useCallback(
    (value: string) => {
      setSearchTerm(value)
      // In server-side mode, notify parent of search changes
      if (serverSide && onSearchChange) {
        onSearchChange(value)
      }
    },
    [serverSide, onSearchChange]
  )

  const handleViewModeChange = useCallback(
    (mode: ViewMode) => {
      if (onViewModeChange) {
        onViewModeChange(mode)
      } else {
        tableState.setViewMode(mode)
      }
    },
    [onViewModeChange, tableState]
  )

  const handleSelectionChange = useCallback(
    (keys: Set<Key> | 'all') => {
      // Handle 'all' selection
      const newKeys = keys === 'all' ? new Set(data.map(getRowKey)) : keys
      if (onSelectionChange) {
        onSelectionChange(newKeys)
      } else {
        tableState.setSelectedKeys(newKeys)
      }
    },
    [data, getRowKey, onSelectionChange, tableState]
  )

  // ============================================================================
  // Data Processing
  // ============================================================================

  // In server-side mode, skip client-side filtering (server already filtered)
  const filteredData = useFilteredData(
    data,
    serverSide ? '' : searchTerm, // Skip search filter in server-side mode
    serverSide ? undefined : searchFn
  )
  
  // In server-side mode, skip client-side sorting (server already sorted)
  const baseSortedData = useSortedData(
    filteredData,
    columns,
    serverSide ? null : sortColumn, // Skip sorting in server-side mode
    sortDirection,
    serverSide ? undefined : defaultSortFn
  )

  // Move pinned items to the top (after sorting) - still works in server-side mode
  const sortedData = useMemo(() => {
    if (!isPinned) return baseSortedData
    const pinned = baseSortedData.filter((item) => isPinned(item))
    const unpinned = baseSortedData.filter((item) => !isPinned(item))
    return [...pinned, ...unpinned]
  }, [baseSortedData, isPinned])

  // Pagination
  const pagination = usePagination({
    totalItems: sortedData.length,
    pageSize,
  })

  // Get paginated data
  const paginatedData = useMemo(() => {
    if (paginationMode === 'pagination') {
      return sortedData.slice(pagination.startIndex, pagination.endIndex)
    }
    return sortedData
  }, [sortedData, paginationMode, pagination.startIndex, pagination.endIndex])

  // Infinite scroll with guard against infinite loops
  // Use isLoadingMore if provided, otherwise fall back to isLoading
  const { sentinelRef } = useInfiniteScroll({
    hasMore,
    isLoading: isLoadingMore || isLoading,
    onLoadMore: onLoadMore ?? (() => { }),
    dataLength: paginatedData.length,
    // Pass totalCount to prevent loading when we already have all items
    totalCount: serverTotalCount ?? null,
  })

  // Normalize selected keys to strings for consistent comparison (HeroUI may convert keys to strings)
  const selectedKeyStrings = useMemo(
    () => new Set(Array.from(selectedKeys).map(String)),
    [selectedKeys]
  )

  // Helper to check if a key is selected (handles type differences)
  const isKeySelected = useCallback(
    (key: Key) => selectedKeyStrings.has(String(key)),
    [selectedKeyStrings]
  )

  // Get selected items
  const selectedItems = useMemo(() => {
    if (selectedKeys.size === 0) return []
    return data.filter((item) => selectedKeyStrings.has(String(getRowKey(item))))
  }, [data, selectedKeys, selectedKeyStrings, getRowKey])

  // Get disabled keys (rows that are not selectable) - convert to strings for HeroUI compatibility
  const disabledKeys = useMemo(() => {
    if (!isRowSelectable) return new Set<Key>()
    return new Set(
      data.filter((item) => !isRowSelectable(item)).map((item) => String(getRowKey(item)))
    )
  }, [data, isRowSelectable, getRowKey])


  // Visible columns (respecting order)
  const visibleColumns = useMemo(() => {
    const visible = columns.filter((col) => !col.hidden)
    
    // If column order is specified, sort by it
    if (columnOrder.length > 0) {
      const orderMap = new Map(columnOrder.map((key, index) => [key, index]))
      return [...visible].sort((a, b) => {
        const aIndex = orderMap.get(a.key) ?? Infinity
        const bIndex = orderMap.get(b.key) ?? Infinity
        return aIndex - bIndex
      })
    }
    
    return visible
  }, [columns, columnOrder])

  // Check if all selectable items are selected (for header checkbox)
  const allSelectableItems = useMemo(() => {
    if (!isRowSelectable) return paginatedData
    return paginatedData.filter(item => isRowSelectable(item))
  }, [paginatedData, isRowSelectable])

  const allSelected = allSelectableItems.length > 0 && 
    allSelectableItems.every(item => isKeySelected(getRowKey(item)))
  
  const someSelected = allSelectableItems.some(item => isKeySelected(getRowKey(item))) && !allSelected

  // Handle select all toggle for checkbox-only mode
  const handleSelectAllToggle = useCallback((checked: boolean) => {
    if (checked) {
      // Select all selectable items
      const newKeys = new Set(selectedKeys)
      allSelectableItems.forEach(item => newKeys.add(getRowKey(item)))
      handleSelectionChange(newKeys)
    } else {
      // Deselect all items on current page
      const newKeys = new Set(selectedKeys)
      paginatedData.forEach(item => newKeys.delete(getRowKey(item)))
      handleSelectionChange(newKeys)
    }
  }, [selectedKeys, allSelectableItems, paginatedData, getRowKey, handleSelectionChange])

  // Grouped data for card view
  const groupedData: DataTableGroup<T>[] = useMemo(() => {
    if (!groupBy) return []
    return groupBy(paginatedData)
  }, [groupBy, paginatedData])

  // ============================================================================
  // Render Helpers
  // ============================================================================

  const renderCell = useCallback(
    (item: T, column: DataTableColumn<T>, index: number) => {
      const shouldTruncate = column.truncate !== false // Default to true
      let content: ReactNode

      if (column.render) {
        content = column.render(item, index)
      } else {
        const value = (item as Record<string, unknown>)[column.key]
        if (value === null || value === undefined) {
          content = '—'
        } else {
          content = String(value)
        }
      }

      // Wrap content in truncate container if needed
      // This ensures text truncation works even without explicit column widths
      if (shouldTruncate && typeof content === 'string') {
        return (
          <span className="block truncate" title={content}>
            {content}
          </span>
        )
      }

      // For custom rendered content, wrap in overflow-hidden container
      if (shouldTruncate) {
        return (
          <div className="overflow-hidden">
            {content}
          </div>
        )
      }

      return content
    },
    []
  )

  // ============================================================================
  // Render
  // ============================================================================

  return (
    <div className={`${fillHeight ? 'flex flex-col h-full' : 'space-y-4'} ${classNames.wrapper ?? ''}`}>
      {/* Header Content */}
      {headerContent}

      {/* Toolbar */}
      {!hideToolbar && (
        <div className={`flex flex-col gap-4 shrink-0 ${fillHeight ? 'sticky top-0 z-20 pb-4' : ''} ${classNames.toolbar ?? ''}`}>
          {/* Search, Actions, View Toggle Row */}
          <div className="flex flex-col sm:flex-row gap-4 items-start sm:items-center justify-between">
            {/* Search and custom toolbar content (start) */}
            <div className="flex gap-2 items-center w-full sm:w-auto grow">
              <Input
                className="w-full sm:max-w-xs"
                classNames={{
                  mainWrapper: 'bg-content1 rounded-lg',
                  inputWrapper: 'bg-content1',
                }}
                variant="flat"
                placeholder={searchPlaceholder}
                value={searchTerm}
                onValueChange={handleSearchChange}
                startContent={<IconSearch size={18} />}
                isClearable
                onClear={() => handleSearchChange('')}
              />
              {toolbarContentPosition === 'start' && toolbarContent}
            </div>

            {/* Bulk actions and view toggle */}
            <div className="flex gap-2 items-center">
              {/* Selection info and bulk actions */}
              {selectionMode !== 'none' && selectedItems.length > 0 && (
                <>
                  <span className="text-sm text-default-500">
                    {selectedItems.length} selected
                  </span>
                  {bulkActions.map((action) => {
                    const isDisabled =
                      typeof action.disabled === 'function'
                        ? action.disabled(selectedItems)
                        : action.disabled

                    return (
                      <Tooltip key={action.key} content={action.label}>
                        <Button
                          size="sm"
                          variant="flat"
                          color={action.isDestructive ? 'danger' : action.color}
                          isIconOnly={!!action.icon}
                          isDisabled={isDisabled}
                          onPress={() => {
                            if (action.confirm) {
                              const msg =
                                action.confirmMessage ??
                                `Are you sure you want to ${action.label.toLowerCase()} ${selectedItems.length} item(s)?`
                              setConfirmAction({
                                message: msg,
                                onConfirm: async () => {
                                  await action.onAction(selectedItems)
                                  tableState.clearSelection()
                                  onConfirmClose()
                                },
                              })
                              onConfirmOpen()
                            } else {
                              action.onAction(selectedItems)
                              tableState.clearSelection()
                            }
                          }}
                        >
                          {action.icon ?? action.label}
                        </Button>
                      </Tooltip>
                    )
                  })}
                  <Divider orientation="vertical" className="h-6" />
                </>
              )}

              {/* View mode toggle */}
              {showViewModeToggle && cardRenderer && (
                <ButtonGroup size="sm">
                  <Tooltip content="Table view">
                    <Button
                      isIconOnly
                      aria-label="Table view"
                      variant={viewMode === 'table' ? 'solid' : 'flat'}
                      onPress={() => handleViewModeChange('table')}
                    >
                      <IconTable size={18} />
                    </Button>
                  </Tooltip>
                  <Tooltip content="Card view">
                    <Button
                      isIconOnly
                      aria-label="Card view"
                      variant={viewMode === 'cards' ? 'solid' : 'flat'}
                      onPress={() => handleViewModeChange('cards')}
                    >
                      <IconLayoutGrid size={18} />
                    </Button>
                  </Tooltip>
                </ButtonGroup>
              )}

              {/* Skeleton testing toggle */}
              {enableSkeletonTesting && (
                <Switch
                  size="sm"
                  isSelected={isSkeletonTesting}
                  onValueChange={setIsSkeletonTesting}
                  classNames={{
                    label: 'text-sm text-default-500',
                  }}
                >
                  Skeletons
                </Switch>
              )}

              {toolbarContentPosition === 'end' && toolbarContent}
            </div>
          </div>

          {/* Custom filter row content - callers supply their own filter UI */}
          {filterRowContent && (
            <div className="flex flex-wrap gap-2 items-center">
              {filterRowContent}
            </div>
          )}
        </div>
      )}

      {/* Table View - always show table structure, empty state is inside TableBody */}
      {viewMode === 'table' && (
        <TableViewWrapper 
          fillHeight={fillHeight} 
          className={classNames.tableContainer}
          sentinelRef={sentinelRef}
          showSentinel={paginationMode === 'infinite' && hasMore}
        >
          <Table
            aria-label={ariaLabel}
            selectionMode={selectionMode === 'none' || checkboxSelectionOnly ? 'none' : selectionMode}
            selectedKeys={checkboxSelectionOnly ? undefined : selectedKeyStrings}
            onSelectionChange={checkboxSelectionOnly ? undefined : handleSelectionChange}
            disabledKeys={checkboxSelectionOnly ? undefined : disabledKeys as any}
            removeWrapper={fillHeight || removeWrapper}
            isStriped={isStriped}
            isCompact={isCompact}
            sortDescriptor={sortDescriptor}
            onSortChange={handleTableSortChange}
            isHeaderSticky={fillHeight}
            classNames={{
              // When fillHeight: TableViewWrapper provides the wrapper styling
              // Table's wrapper is removed so we control scrolling and styling
              base: `w-full max-w-full ${resizingColumn ? 'select-none' : ''}`,
              wrapper: fillHeight ? '' : `${classNames.table ?? ''} max-w-full`,
              table: 'table-fixed w-full',
              th: `text-default-600 first:rounded-l-lg last:rounded-r-lg relative group ${selectionMode !== 'none' && !checkboxSelectionOnly ? 'first:w-[50px] first:min-w-[50px] first:max-w-[50px]' : ''}`,
              thead: fillHeight ? 'sticky top-0 z-20 bg-content1' : '',
              td: `overflow-hidden ${selectionMode !== 'none' && !checkboxSelectionOnly ? 'first:w-[50px] first:min-w-[50px] first:max-w-[50px]' : ''}`,
            }}
          >
            <TableHeader>
              {[
                // Manual checkbox column when checkboxSelectionOnly is true
                ...(checkboxSelectionOnly && selectionMode !== 'none'
                  ? [
                    <TableColumn
                      key="_checkbox"
                      style={{ width: 50, minWidth: 50, maxWidth: 50, flexGrow: 0, flexShrink: 0 }}
                    >
                      <Checkbox
                        isSelected={allSelected}
                        isIndeterminate={someSelected}
                        onValueChange={handleSelectAllToggle}
                        aria-label="Select all"
                      />
                    </TableColumn>,
                  ]
                  : []),
                ...visibleColumns.map((column) => {
                  const sizing = parseColumnSizing(column)
                  const overrideWidth = columnWidths[column.key]
                  const columnStyle = getColumnStyle(sizing, overrideWidth)
                  const isReorderable = enableColumnReorder && column.reorderable !== false
                  const isDragging = draggingColumn === column.key
                  const isDragOver = dragOverColumn === column.key

                  return (
                    <TableColumn
                      key={column.key}
                      align={column.align}
                      allowsSorting={column.sortable !== false}
                      className={`
                        ${isDragging ? 'opacity-50' : ''}
                        ${isDragOver ? 'bg-primary/20 border-l-2 border-primary' : ''}
                        ${isReorderable ? 'cursor-grab active:cursor-grabbing' : ''}
                      `}
                      style={columnStyle}
                      {...(isReorderable ? getDragProps(column.key) : {})}
                    >
                      {column.label}
                    </TableColumn>
                  )
                }),
                ...(rowActions.length > 0
                  ? [
                    <TableColumn 
                      key="_actions" 
                      align="end"
                      style={{ width: 100, minWidth: 100, maxWidth: 100, flexGrow: 0, flexShrink: 0 }}
                    >
                      ACTIONS
                    </TableColumn>,
                  ]
                  : []),
              ]}
            </TableHeader>
            <TableBody
              items={(() => {
                // Build items array with skeleton rows, data rows, and optional sentinel row
                if (isSkeletonTesting) {
                  // Show skeleton rows during skeleton testing mode
                  return Array.from({ length: skeletonRowCount }).map((_, i) => ({ _skeletonId: i }) as unknown as T)
                }

                if (isLoading && data.length === 0) {
                  // Show skeleton rows during initial loading
                  return Array.from({ length: skeletonRowCount }).map((_, i) => ({ _skeletonId: i }) as unknown as T)
                }

                const items: T[] = [...paginatedData]
                
                // Add skeleton rows when loading more
                if (paginationMode === 'infinite' && isLoadingMore) {
                  const loadingSkeletons = Array.from({ length: skeletonRowCount }).map((_, i) => ({ 
                    _skeletonId: i,
                    _isLoadMoreSkeleton: true 
                  }) as unknown as T)
                  items.push(...loadingSkeletons)
                }
                
                // Note: Sentinel is now placed OUTSIDE the table via TableViewWrapper
                // This ensures it's truly invisible and doesn't affect table layout
                
                return items
              })()}
              emptyContent={
                emptyContent ?? (
                  <div className="py-8 text-center">
                    <p className="text-default-500">No records found</p>
                    {searchTerm && (
                      <Button
                        variant="light"
                        color="primary"
                        size="sm"
                        className="mt-2"
                        onPress={() => handleSearchChange('')}
                      >
                        Clear search
                      </Button>
                    )}
                  </div>
                )
              }
            >
              {(item) => {
                // Check if this is a skeleton row (initial load or load-more)
                const isSkeleton = '_skeletonId' in (item as object)
                
                if (isSkeleton) {
                  const skeletonData = item as unknown as { _skeletonId: number; _isLoadMoreSkeleton?: boolean }
                  const skeletonId = skeletonData._skeletonId
                  const keyPrefix = skeletonData._isLoadMoreSkeleton ? 'load-more-skeleton' : 'skeleton'
                  return (
                    <TableRow key={`${keyPrefix}-${skeletonId}`}>
                      {[
                        // Checkbox column skeleton
                        ...(checkboxSelectionOnly && selectionMode !== 'none'
                          ? [
                            <TableCell
                              key="_checkbox"
                              style={{ width: 50, minWidth: 50, maxWidth: 50, flexGrow: 0, flexShrink: 0 }}
                            >
                              <Skeleton className="w-4 h-4 rounded" />
                            </TableCell>,
                          ]
                          : []),
                        ...visibleColumns.map((column) => {
                          const sizing = parseColumnSizing(column)
                          const overrideWidth = columnWidths[column.key]
                          const columnStyle = getColumnStyle(sizing, overrideWidth)
                          
                          return (
                            <TableCell key={column.key} style={columnStyle} className="overflow-hidden">
                              {column.skeleton ? column.skeleton() : <Skeleton className="w-full h-4 rounded" />}
                            </TableCell>
                          )
                        }),
                        ...(rowActions.length > 0
                          ? [
                            <TableCell 
                              key="_actions"
                              style={{ width: 100, minWidth: 100, maxWidth: 100, flexGrow: 0, flexShrink: 0 }}
                            >
                              <Skeleton className="ml-auto w-6 h-6 rounded" />
                            </TableCell>,
                          ]
                          : []),
                      ]}
                    </TableRow>
                  )
                }

                // Note: Sentinel is now rendered outside the table via TableViewWrapper
                
                const index = paginatedData.indexOf(item)
                const rowKey = getRowKey(item)
                const isSelected = isKeySelected(rowKey)
                const isSelectable = !isRowSelectable || isRowSelectable(item)
                
                const cells = [
                  // Manual checkbox cell when checkboxSelectionOnly is true
                  ...(checkboxSelectionOnly && selectionMode !== 'none'
                    ? [
                      <TableCell
                        key="_checkbox"
                        style={{ width: 50, minWidth: 50, maxWidth: 50, flexGrow: 0, flexShrink: 0 }}
                      >
                        <Checkbox
                          isSelected={isSelected}
                          isDisabled={!isSelectable}
                          onValueChange={(checked) => {
                            const newKeys = new Set(selectedKeys)
                            if (checked) {
                              newKeys.add(rowKey)
                            } else {
                              newKeys.delete(rowKey)
                            }
                            handleSelectionChange(newKeys)
                          }}
                          aria-label={`Select row ${rowKey}`}
                        />
                      </TableCell>,
                    ]
                    : []),
                  ...visibleColumns.map((column) => {
                    const sizing = parseColumnSizing(column)
                    const overrideWidth = columnWidths[column.key]
                    const columnStyle = getColumnStyle(sizing, overrideWidth)
                    
                    return (
                      <TableCell 
                        key={column.key}
                        style={columnStyle}
                        className="overflow-hidden"
                      >
                        {renderCell(item, column, index)}
                      </TableCell>
                    )
                  }),
                ]
                if (rowActions.length > 0) {
                  cells.push(
                    <TableCell 
                      key="_actions"
                      style={{ width: 100, minWidth: 100, maxWidth: 100, flexGrow: 0, flexShrink: 0 }}
                    >
                      <div className="flex gap-1 justify-end">
                        <InlineRowActions item={item} actions={rowActions} />
                        <RowActionsDropdown item={item} actions={rowActions} />
                      </div>
                    </TableCell>
                  )
                }
                return (
                  <TableRow key={rowKey}>
                    {cells}
                  </TableRow>
                )
              }}
            </TableBody>
          </Table>
        </TableViewWrapper>
      )}

      {/* Card View - Ungrouped */}
      {viewMode === 'cards' && cardRenderer && !groupBy && (
        <div className={`${fillHeight ? 'flex-1 min-h-0 overflow-y-auto' : ''}`}>
          {paginatedData.length === 0 ? (
            <div className="py-8 text-center">
              {emptyContent ?? (
                <>
                  <p className="text-default-500">No records found</p>
                  {searchTerm && (
                    <Button
                      variant="light"
                      color="primary"
                      size="sm"
                      className="mt-2"
                      onPress={() => handleSearchChange('')}
                    >
                      Clear search
                    </Button>
                  )}
                </>
              )}
            </div>
          ) : (
            <>
              <div className={`${cardGridClassName} ${fillHeight ? 'pb-4' : ''}`}>
                {paginatedData.map((item, index) => {
                  const key = getRowKey(item)
                  const isSelected = isKeySelected(key)
                  const isSelectable = !isRowSelectable || isRowSelectable(item)
                  return (
                    <div key={key} className="relative">
                      {selectionMode !== 'none' && isSelectable && (
                        <div className="absolute top-2 left-2 z-10">
                          <Checkbox
                            color="danger"
                            isSelected={isSelected}
                            aria-label={`Select item ${key}`}
                            onValueChange={(checked) => {
                              const newKeys = new Set(selectedKeys)
                              if (checked) {
                                newKeys.add(key)
                              } else {
                                newKeys.delete(key)
                              }
                              handleSelectionChange(newKeys)
                            }}
                          />
                        </div>
                      )}
                      {cardRenderer({
                        item,
                        index,
                        isSelected,
                        onSelect: () => {
                          if (!isSelectable) return
                          const newKeys = new Set(selectedKeys)
                          if (isSelected) {
                            newKeys.delete(key)
                          } else {
                            newKeys.add(key)
                          }
                          handleSelectionChange(newKeys)
                        },
                        actions: rowActions,
                      })}
                    </div>
                  )
                })}
              </div>
              {/* Infinite scroll sentinel for card view */}
              {paginationMode === 'infinite' && hasMore && (
                <div ref={sentinelRef} className="flex justify-center py-4">
                  {(isLoadingMore || isLoading) && <Spinner size="sm" />}
                </div>
              )}
            </>
          )}
        </div>
      )}

      {/* Card View - Grouped */}
      {viewMode === 'cards' && cardRenderer && groupBy && (
        <div className={`${fillHeight ? 'flex-1 min-h-0 overflow-y-auto' : ''}`}>
          {groupedData.length === 0 ? (
            <div className="py-8 text-center">
              {emptyContent ?? (
                <>
                  <p className="text-default-500">No records found</p>
                  {searchTerm && (
                    <Button
                      variant="light"
                      color="primary"
                      size="sm"
                      className="mt-2"
                      onPress={() => handleSearchChange('')}
                    >
                      Clear search
                    </Button>
                  )}
                </>
              )}
            </div>
          ) : (
            <>
              <div className={`space-y-6 ${fillHeight ? 'pb-4' : ''}`}>
                {groupedData.map((group) => (
                  <div key={group.key}>
                    {/* Group Header */}
                    {groupHeaderRenderer ? (
                      groupHeaderRenderer(group)
                    ) : (
                      <div className="flex items-center gap-2 mb-3 sticky top-0 bg-background/95  backdrop-blur py-2 z-10">
                        <span className="text-xl font-bold text-primary">{group.label}</span>
                        <span className="text-sm text-default-400">
                          {group.items.length} item{group.items.length !== 1 ? 's' : ''}
                        </span>
                      </div>
                    )}
                    {/* Group Items */}
                    <div className={cardGridClassName}>
                      {group.items.map((item, index) => {
                        const key = getRowKey(item)
                        const isSelected = isKeySelected(key)
                        const isSelectable = !isRowSelectable || isRowSelectable(item)
                        return (
                          <div key={key} className="relative">
                            {selectionMode !== 'none' && isSelectable && (
                              <div className="absolute top-2 left-2 z-10">
                                <Checkbox
                                  color="danger"
                                  isSelected={isSelected}
                                  aria-label={`Select item ${key}`}
                                  onValueChange={(checked) => {
                                    const newKeys = new Set(selectedKeys)
                                    if (checked) {
                                      newKeys.add(key)
                                    } else {
                                      newKeys.delete(key)
                                    }
                                    handleSelectionChange(newKeys)
                                  }}
                                />
                              </div>
                            )}
                            {cardRenderer({
                              item,
                              index,
                              isSelected,
                              onSelect: () => {
                                if (!isSelectable) return
                                const newKeys = new Set(selectedKeys)
                                if (isSelected) {
                                  newKeys.delete(key)
                                } else {
                                  newKeys.add(key)
                                }
                                handleSelectionChange(newKeys)
                              },
                              actions: rowActions,
                            })}
                          </div>
                        )
                      })}
                    </div>
                  </div>
                ))}
              </div>
              {/* Infinite scroll sentinel for grouped card view */}
              {paginationMode === 'infinite' && hasMore && (
                <div ref={sentinelRef} className="flex justify-center py-4">
                  {(isLoadingMore || isLoading) && <Spinner size="sm" />}
                </div>
              )}
            </>
          )}
        </div>
      )}


      {/* Pagination */}
      {paginationMode === 'pagination' && sortedData.length > 0 && (
        <div className="flex flex-col sm:flex-row justify-between items-center gap-4 py-2">
          <div className="flex items-center gap-2">
            <span className="text-sm text-default-500">Rows per page:</span>
            <Select
              aria-label="Page size"
              size="sm"
              className="w-20"
              selectedKeys={[String(pageSize)]}
              onChange={(e) => tableState.setPageSize(Number(e.target.value))}
            >
              {pageSizeOptions.map((size) => (
                <SelectItem key={String(size)}>{size}</SelectItem>
              ))}
            </Select>
          </div>

          <Pagination
            total={pagination.totalPages}
            page={pagination.page}
            onChange={pagination.setPage}
            showControls
            size="sm"
          />

          <span className="text-sm text-default-500">
            {pagination.startIndex + 1}–{pagination.endIndex} of {sortedData.length}
          </span>
        </div>
      )}

      {/* Item count footer */}
      {showItemCount && paginationMode !== 'pagination' && (
        <div className={`h-14 flex justify-between items-center text-sm text-default-500 px-2 ${classNames.footer ?? ''}`}>
          <span className="flex items-center gap-2">
            {serverSide && serverTotalCount !== undefined
              ? // Server-side mode: show loaded/total
                `${data.length} of ${serverTotalCount} item${serverTotalCount !== 1 ? 's' : ''}`
              : // Client-side mode: show filtered/total or just total
                sortedData.length === data.length
                  ? `${data.length} item${data.length !== 1 ? 's' : ''}`
                  : `${sortedData.length} of ${data.length} items`}
            {(isLoadingMore || (paginationMode === 'infinite' && isLoading && data.length > 0)) && (
              <span className="flex items-center gap-1 text-default-400">
                <Spinner size="sm" />
                <span>Loading...</span>
              </span>
            )}
          </span>
        </div>
      )}

      {/* Footer Content */}
      {footerContent}

      {/* Confirm Modal for bulk actions */}
      <Modal isOpen={isConfirmOpen} onClose={onConfirmClose} size="sm">
        <ModalContent>
          <ModalHeader>Confirm Action</ModalHeader>
          <ModalBody>
            <p>{confirmAction?.message}</p>
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={onConfirmClose}>
              Cancel
            </Button>
            <Button
              color="danger"
              onPress={() => confirmAction?.onConfirm()}
            >
              Confirm
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </div>
  )
}
