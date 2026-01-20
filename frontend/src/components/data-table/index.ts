// Main component
export { DataTable } from './DataTable'

// Alphabet filter component and utilities
export { AlphabetFilter, getFirstLetter, useAlphabetFilter, type AlphabetFilterProps } from './AlphabetFilter'

// Types
export type {
  DataTableProps,
  DataTableColumn,
  DataTableFilter,
  FilterOption,
  BulkAction,
  RowAction,
  CardRendererProps,
  SortDirection,
  ViewMode,
  FilterType,
  SelectionMode,
  PaginationMode,
  FilterValues,
  DataTableState,
  DataTableGroup,
  GroupByFn,
  ColumnSizing,
} from './types'

// Hooks
export {
  useDataTableState,
  useFilteredData,
  useSortedData,
  usePagination,
  useInfiniteScroll,
  useColumnReorder,
  useColumnResize,
} from './hooks'

// Re-export tabler icons (for use in custom renderers)
export {
  IconSearch,
  IconFilter,
  IconX,
  IconDotsVertical,
  IconCheck,
  IconChecks,
  IconTable,
  IconLayoutGrid,
  IconChevronLeft,
  IconChevronRight,
  IconChevronsLeft,
  IconChevronsRight,
  IconSortAscending,
  IconSortDescending,
  IconLoader,
  IconMoodEmpty,
} from '@tabler/icons-react'
