import type { Key, ReactNode } from "react";
import {
  DataTable,
  type DataTableColumn,
  type RowAction,
  type CardRendererProps,
} from "../data-table";

interface DetailItemsTableProps<T> {
  tableKey: string;
  stateKey?: string;
  data: T[];
  columns: DataTableColumn<T>[];
  getRowKey: (item: T) => string;
  ariaLabel: string;
  rowActions?: RowAction<T>[];
  selectedKeys?: Set<Key>;
  selectionMode?: "none" | "single" | "multiple";
  defaultSortColumn?: string;
  defaultSortDirection?: "asc" | "desc";
  searchPlaceholder?: string;
  headerContent?: ReactNode;
  emptyContent?: ReactNode;
  isLoading?: boolean;
  isCompact?: boolean;
  hideToolbar?: boolean;
  removeWrapper?: boolean;
  showItemCount?: boolean;
  showViewModeToggle?: boolean;
  defaultViewMode?: "table" | "cards";
  cardRenderer?: (props: CardRendererProps<T>) => ReactNode;
  cardGridClassName?: string;
  skeletonDelay?: number;
}

export function DetailItemsTable<T>({
  tableKey,
  stateKey,
  data,
  columns,
  getRowKey,
  ariaLabel,
  rowActions,
  selectedKeys,
  selectionMode = "none",
  defaultSortColumn,
  defaultSortDirection = "asc",
  searchPlaceholder,
  headerContent,
  emptyContent,
  isLoading = false,
  isCompact = true,
  hideToolbar = true,
  removeWrapper = false,
  showItemCount = false,
  showViewModeToggle = false,
  defaultViewMode = "table",
  cardRenderer,
  cardGridClassName,
  skeletonDelay = 500,
}: DetailItemsTableProps<T>) {
  return (
    <DataTable
      key={tableKey}
      stateKey={stateKey}
      skeletonDelay={skeletonDelay}
      data={data}
      columns={columns}
      getRowKey={getRowKey}
      ariaLabel={ariaLabel}
      rowActions={rowActions}
      selectedKeys={selectedKeys}
      selectionMode={selectionMode}
      defaultSortColumn={defaultSortColumn}
      defaultSortDirection={defaultSortDirection}
      searchPlaceholder={searchPlaceholder}
      headerContent={headerContent}
      emptyContent={emptyContent}
      isLoading={isLoading}
      isCompact={isCompact}
      hideToolbar={hideToolbar}
      removeWrapper={removeWrapper}
      showItemCount={showItemCount}
      showViewModeToggle={showViewModeToggle}
      defaultViewMode={defaultViewMode}
      cardRenderer={cardRenderer}
      cardGridClassName={cardGridClassName}
    />
  );
}
