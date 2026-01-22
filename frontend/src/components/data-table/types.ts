import type { ReactNode, Key } from 'react'

// ============================================================================
// Column Configuration
// ============================================================================

/** Sort direction */
export type SortDirection = 'asc' | 'desc'

/** Column sizing configuration */
export interface ColumnSizing {
  /** Fixed width in pixels - column will not grow or shrink */
  width?: number
  /** Minimum width in pixels (default: 50) */
  minWidth?: number
  /** Maximum width in pixels */
  maxWidth?: number
  /** 
   * Flex grow factor - columns with grow will share remaining space proportionally.
   * A column with grow: 2 will take twice as much space as grow: 1.
   * If not specified, defaults to 1 for columns without fixed width.
   * Set to 0 to prevent growing.
   */
  grow?: number
  /** Whether this column can be resized by the user (default: true if resizing enabled) */
  resizable?: boolean
}

/** Column definition for the data table */
export interface DataTableColumn<T> {
  /** Unique key for this column (should match a property of T or be custom) */
  key: string
  /** Display label for the column header */
  label: string
  /** Whether this column is sortable (default: true) */
  sortable?: boolean
  /** 
   * Column width configuration.
   * - number: Fixed width in pixels (shorthand for { width: number })
   * - string: CSS width value (e.g., '100px', '20%') - legacy support
   * - ColumnSizing: Full sizing configuration
   */
  width?: number | string | ColumnSizing
  /** Text alignment */
  align?: 'start' | 'center' | 'end'
  /** Custom render function for cell content */
  render?: (item: T, index: number) => ReactNode
  /** Custom skeleton render function for loading state */
  skeleton?: () => ReactNode
  /** Custom sort function (return negative, zero, or positive) */
  sortFn?: (a: T, b: T) => number
  /** Whether to hide this column on mobile */
  hideOnMobile?: boolean
  /** Whether this column should be hidden */
  hidden?: boolean
  /** 
   * Whether to truncate text in this column (default: true).
   * When true, text will be truncated with ellipsis when it overflows.
   * When false, text will wrap to multiple lines.
   */
  truncate?: boolean
  /** Whether this column can be reordered via drag-and-drop (default: true if reordering enabled) */
  reorderable?: boolean
}

// ============================================================================
// Filter Configuration
// ============================================================================

/** Type of filter */
export type FilterType = 'search' | 'select' | 'multiselect' | 'date' | 'custom'

/** Option for select/multiselect filters */
export interface FilterOption {
  key: string
  label: string
  icon?: string | ReactNode
  color?: 'default' | 'primary' | 'secondary' | 'success' | 'warning' | 'danger'
  count?: number
}

/** Filter definition */
export interface DataTableFilter<T> {
  /** Unique key for this filter */
  key: string
  /** Display label */
  label: string
  /** Filter type */
  type: FilterType
  /** Placeholder text for search inputs */
  placeholder?: string
  /** Options for select/multiselect filters */
  options?: FilterOption[]
  /** Custom filter function */
  filterFn?: (item: T, value: unknown) => boolean
  /** Default value */
  defaultValue?: unknown
  /** Whether to show in toolbar or dropdown (default: 'toolbar') */
  position?: 'toolbar' | 'dropdown'
}

/** Active filter values */
export type FilterValues = Record<string, unknown>

// ============================================================================
// Selection & Actions
// ============================================================================

/** Selection mode */
export type SelectionMode = 'none' | 'single' | 'multiple'

/** Bulk action definition */
export interface BulkAction<T> {
  /** Unique key */
  key: string
  /** Display label */
  label: string
  /** Icon */
  icon?: ReactNode
  /** Action color */
  color?: 'default' | 'primary' | 'secondary' | 'success' | 'warning' | 'danger'
  /** Handler function (receives selected items) */
  onAction: (selectedItems: T[]) => void | Promise<void>
  /** Whether to show confirmation dialog */
  confirm?: boolean
  /** Confirmation message */
  confirmMessage?: string
  /** Whether this action is destructive */
  isDestructive?: boolean
  /** Whether to disable this action */
  disabled?: boolean | ((selectedItems: T[]) => boolean)
}

/** Row action definition */
export interface RowAction<T> {
  /** Unique key */
  key: string
  /** Display label (string or function that receives the item) */
  label: string | ((item: T) => string)
  /** Icon (ReactNode or function that receives the item) */
  icon?: ReactNode | ((item: T) => ReactNode)
  /** Action color */
  color?: 'default' | 'primary' | 'secondary' | 'success' | 'warning' | 'danger'
  /** Handler function (receives the item) */
  onAction: (item: T) => void | Promise<void>
  /** Whether this action is visible for a given item */
  isVisible?: (item: T) => boolean
  /** Whether this action is disabled for a given item */
  isDisabled?: (item: T) => boolean
  /** Whether to show in dropdown (default: false) */
  inDropdown?: boolean
  /** Whether this action is destructive */
  isDestructive?: boolean
}

// ============================================================================
// View Mode
// ============================================================================

/** View mode */
export type ViewMode = 'table' | 'cards'

/** Card renderer props */
export interface CardRendererProps<T> {
  item: T
  index: number
  isSelected: boolean
  onSelect: () => void
  actions: RowAction<T>[]
}

// ============================================================================
// Pagination & Loading
// ============================================================================

/** Pagination mode */
export type PaginationMode = 'none' | 'pagination' | 'infinite'

/** Pagination state */
export interface PaginationState {
  page: number
  pageSize: number
  totalItems: number
  totalPages: number
}

// ============================================================================
// Table State
// ============================================================================

/** Complete table state (for persistence) */
export interface DataTableState {
  sortColumn: string | null
  sortDirection: SortDirection
  filterValues: FilterValues
  viewMode: ViewMode
  pageSize: number
  selectedKeys: Set<Key>
  /** Ordered list of column keys (for reordering) */
  columnOrder: string[]
  /** Map of column key to width in pixels (for resizing) */
  columnWidths: Record<string, number>
}

/** Default table state */
export const DEFAULT_TABLE_STATE: DataTableState = {
  sortColumn: null,
  sortDirection: 'asc',
  filterValues: {},
  viewMode: 'table',
  pageSize: 20,
  selectedKeys: new Set(),
  columnOrder: [],
  columnWidths: {},
}

// ============================================================================
// Grouping Configuration
// ============================================================================

/** Group definition for grouped card views */
export interface DataTableGroup<T> {
  /** Unique key for this group */
  key: string
  /** Display label for the group header */
  label: string
  /** Items in this group */
  items: T[]
}

/** Grouping function */
export type GroupByFn<T> = (items: T[]) => DataTableGroup<T>[]

// ============================================================================
// Main Component Props
// ============================================================================

export interface DataTableProps<T> {
  /** Unique key for state persistence (uses localStorage) */
  stateKey?: string

  /** Data items to display */
  data: T[]

  /** Column definitions */
  columns: DataTableColumn<T>[]

  /** Function to extract unique key from item */
  getRowKey: (item: T) => Key

  /** Whether data is loading */
  isLoading?: boolean

  /** Number of skeleton rows to show during initial loading (default: 5) */
  skeletonRowCount?: number

  /** Delay in ms before showing skeletons (avoids flash for fast loads, default: 0) */
  skeletonDelay?: number

  /** Whether to enable skeleton testing mode (adds a toggle to show/hide skeletons) */
  enableSkeletonTesting?: boolean

  /** Error message to display */
  error?: string | null

  /** Custom empty state content */
  emptyContent?: ReactNode

  /** Function to determine if an item should be pinned to the top (ignores sorting) */
  isPinned?: (item: T) => boolean

  // --- Selection ---
  /** Selection mode */
  selectionMode?: SelectionMode
  /** Controlled selected keys */
  selectedKeys?: Set<Key>
  /** Selection change handler */
  onSelectionChange?: (keys: Set<Key>) => void
  /** Function to determine if a row is selectable (default: all rows are selectable) */
  isRowSelectable?: (item: T) => boolean
  /** Whether selection only happens on checkbox click (not row click). Default: false */
  checkboxSelectionOnly?: boolean

  // --- Search ---
  /** Custom search function. If not provided, searches all string properties. */
  searchFn?: (item: T, searchTerm: string) => boolean
  /** Search placeholder */
  searchPlaceholder?: string

  // --- Sorting ---
  /** Default sort column */
  defaultSortColumn?: string
  /** Default sort direction */
  defaultSortDirection?: SortDirection
  /** Controlled sort column */
  sortColumn?: string
  /** Controlled sort direction */
  sortDirection?: SortDirection
  /** Sort change handler */
  onSortChange?: (column: string, direction: SortDirection) => void
  /** Custom default sort function (for columns without sortFn) */
  defaultSortFn?: (a: T, b: T, column: string) => number

  // --- View Mode ---
  /** Whether to show view mode toggle */
  showViewModeToggle?: boolean
  /** Default view mode */
  defaultViewMode?: ViewMode
  /** Controlled view mode */
  viewMode?: ViewMode
  /** View mode change handler */
  onViewModeChange?: (mode: ViewMode) => void
  /** Custom card renderer (required if showViewModeToggle is true) */
  cardRenderer?: (props: CardRendererProps<T>) => ReactNode
  /** Custom skeleton card renderer for loading state in card view */
  cardSkeleton?: () => ReactNode
  /** Number of skeleton cards to show during loading (default: 6) */
  skeletonCardCount?: number
  /** Card grid classes (default: responsive grid) */
  cardGridClassName?: string
  /** Grouping function for card view (items are grouped in card view) */
  groupBy?: GroupByFn<T>
  /** Custom group header renderer */
  groupHeaderRenderer?: (group: DataTableGroup<T>) => ReactNode

  // --- Actions ---
  /** Bulk actions (shown when items are selected) */
  bulkActions?: BulkAction<T>[]
  /** Row actions (shown in each row) */
  rowActions?: RowAction<T>[]

  // --- Pagination ---
  /** Pagination mode */
  paginationMode?: PaginationMode
  /** Page size options */
  pageSizeOptions?: number[]
  /** Default page size */
  defaultPageSize?: number
  /** Load more handler for infinite scroll */
  onLoadMore?: () => void
  /** Whether there are more items to load */
  hasMore?: boolean
  /** Whether a load-more operation is in progress (separate from initial loading) */
  isLoadingMore?: boolean

  // --- Server-Side Mode ---
  /**
   * Enable server-side mode. When true:
   * - Client-side filtering is disabled (data is already filtered by server)
   * - Client-side sorting is disabled (data is already sorted by server)
   * - Search is passed to onSearchChange callback instead of filtering locally
   * - Use with paginationMode="infinite" for server-side pagination
   */
  serverSide?: boolean
  /** Total count of items on server (for display, used in server-side mode) */
  serverTotalCount?: number
  /** Callback when search term changes (for server-side filtering) */
  onSearchChange?: (searchTerm: string) => void

  // --- Layout ---
  /** Custom header content (above toolbar) */
  headerContent?: ReactNode
  /** Custom footer content (below table) */
  footerContent?: ReactNode
  /** Custom toolbar content (added to toolbar) */
  toolbarContent?: ReactNode
  /** Toolbar position for custom content */
  toolbarContentPosition?: 'start' | 'end'
  /** Custom content to render in the filter row (alongside filter chips) */
  filterRowContent?: ReactNode
  /** Whether to show item count in footer */
  showItemCount?: boolean
  /** Whether to hide the toolbar (search, filters, view toggle) */
  hideToolbar?: boolean
  /** Custom class names */
  classNames?: {
    wrapper?: string
    toolbar?: string
    table?: string
    tableContainer?: string
    footer?: string
  }
  /** Whether the table should fill available height with sticky header */
  fillHeight?: boolean

  // --- Table Props ---
  /** Aria label for the table */
  ariaLabel?: string
  /** Whether to remove table wrapper */
  removeWrapper?: boolean
  /** Whether table is striped */
  isStriped?: boolean
  /** Whether table is compact */
  isCompact?: boolean

  // --- Column Features ---
  /** 
   * Enable column reordering via drag-and-drop.
   * User can drag column headers to reorder them.
   */
  enableColumnReorder?: boolean
  /** Controlled column order (array of column keys) */
  columnOrder?: string[]
  /** Callback when column order changes */
  onColumnOrderChange?: (order: string[]) => void
  
  /**
   * Enable column resizing via drag handles.
   * User can drag the edge of column headers to resize.
   */
  enableColumnResize?: boolean
  /** Controlled column widths (map of column key to width in pixels) */
  columnWidths?: Record<string, number>
  /** Callback when column widths change */
  onColumnWidthsChange?: (widths: Record<string, number>) => void
  /** Minimum column width when resizing (default: 50) */
  minColumnWidth?: number
  /** Maximum column width when resizing */
  maxColumnWidth?: number
}
