import { useMemo, type Key, type ReactNode } from "react";
import {
  DataTable as ProDataTable,
  type DataTableCardRendererProps as ProCardRendererProps,
  type DataTableColumnDef,
  type DataTableRowAction as ProRowAction,
  type DataTableSelectionAction,
  type DataTableViewMode,
} from "data-table-pro";
import type { SortingState } from "@tanstack/react-table";

import type {
  BulkAction,
  CardRendererProps,
  DataTableColumn,
  DataTableProps,
  RowAction,
  SortDirection,
  ViewMode,
} from "./types";

function toProViewMode(mode: ViewMode | undefined): DataTableViewMode | undefined {
  if (!mode) {
    return undefined;
  }
  return mode === "cards" ? "card" : "table";
}

function fromProViewMode(mode: DataTableViewMode): ViewMode {
  return mode === "card" ? "cards" : "table";
}

function toSortingState(column?: string | null, direction?: SortDirection): SortingState {
  if (!column) {
    return [];
  }
  return [{ id: column, desc: direction === "desc" }];
}

function fromSortingState(sorting: SortingState): [string, SortDirection] | null {
  const first = sorting[0];
  if (!first) {
    return null;
  }
  return [first.id, first.desc ? "desc" : "asc"];
}

function keyToString(key: Key): string {
  return String(key);
}

function normalizeContent(content: ReactNode): ReactNode {
  return content ?? null;
}

function toColumnDef<T>(column: DataTableColumn<T>): DataTableColumnDef<T, unknown> {
  return {
    id: column.key,
    accessorKey: column.key as keyof T & string,
    header: column.label,
    enableSorting: column.sortable !== false,
    sortingFn: column.sortFn
      ? (rowA, rowB) => column.sortFn?.(rowA.original, rowB.original) ?? 0
      : undefined,
    cell: column.render
      ? ({ row }) => normalizeContent(column.render?.(row.original, row.index))
      : undefined,
    meta: {
      align: column.align,
      hideOn: column.hideOnMobile ? "md" : undefined,
      minWidth:
        typeof column.width === "object" && column.width !== null
          ? column.width.minWidth
          : typeof column.width === "number"
            ? column.width
            : undefined,
      cellClassName: column.truncate === false ? "whitespace-normal" : undefined,
      skeleton: column.skeleton ? () => column.skeleton?.() : undefined,
    },
  };
}

function toRowAction<T>(action: RowAction<T>): ProRowAction<T> {
  return {
    key: action.key,
    label: action.label,
    onClick: action.onAction,
    variant: action.isDestructive || action.color === "danger" ? "destructive" : "default",
    hidden: action.isVisible ? (row) => !action.isVisible?.(row) : undefined,
    disabled: action.isDisabled,
  };
}

function toSelectionAction<T>(action: BulkAction<T>): DataTableSelectionAction<T> {
  return {
    key: action.key,
    label: action.label,
    onClick: ({ rows }) => action.onAction(rows),
    variant: action.isDestructive || action.color === "danger" ? "destructive" : "default",
    disabled:
      typeof action.disabled === "function"
        ? (rows) => Boolean((action.disabled as (selectedItems: T[]) => boolean)(rows))
        : action.disabled,
  };
}

function toCardRenderer<T>(
  cardRenderer: ((props: CardRendererProps<T>) => ReactNode) | undefined,
  rowActions: RowAction<T>[] | undefined,
) {
  if (!cardRenderer) {
    return undefined;
  }

  return (props: ProCardRendererProps<T>) =>
    cardRenderer({
      item: props.row,
      index: 0,
      isSelected: props.isSelected,
      onSelect: () => props.onSelectedChange(!props.isSelected),
      actions: rowActions ?? [],
    });
}

export function DataTable<T>({
  data,
  columns,
  getRowKey,
  isLoading,
  skeletonRowCount,
  emptyContent,
  selectionMode = "none",
  selectedKeys,
  onSelectionChange,
  searchPlaceholder,
  sortColumn,
  sortDirection,
  defaultSortColumn,
  defaultSortDirection,
  onSortChange,
  showViewModeToggle,
  viewMode,
  defaultViewMode,
  onViewModeChange,
  cardRenderer,
  bulkActions,
  rowActions,
  onRowClick,
  paginationMode,
  pageSizeOptions,
  defaultPageSize,
  onLoadMore,
  hasMore,
  isLoadingMore,
  serverSide,
  serverTotalCount,
  onSearchChange,
  headerContent,
  footerContent,
  toolbarContent,
  showItemCount,
  hideToolbar,
  classNames,
  fillHeight,
  ariaLabel,
  enableColumnResize,
}: DataTableProps<T>) {
  const proColumns = useMemo(() => columns.filter((column) => !column.hidden).map(toColumnDef), [columns]);
  const proRowActions = useMemo(() => (rowActions ?? []).map(toRowAction), [rowActions]);
  const proSelectionActions = useMemo(
    () => (bulkActions ?? []).map(toSelectionAction),
    [bulkActions],
  );
  const rowSelection = useMemo(() => {
    if (!selectedKeys) {
      return undefined;
    }
    return Object.fromEntries(Array.from(selectedKeys).map((key) => [keyToString(key), true]));
  }, [selectedKeys]);

  const controlledSorting =
    sortColumn !== undefined ? toSortingState(sortColumn, sortDirection) : undefined;
  const initialSorting =
    controlledSorting ?? toSortingState(defaultSortColumn, defaultSortDirection);

  return (
    <ProDataTable
      columns={proColumns}
      data={data}
      getRowId={(row) => keyToString(getRowKey(row))}
      title={typeof headerContent === "string" ? headerContent : undefined}
      customToolbar={toolbarContent ?? (typeof headerContent !== "string" ? headerContent : undefined)}
      searchPlaceholder={searchPlaceholder}
      onSearchValueChange={onSearchChange}
      manualSorting={Boolean(serverSide)}
      sorting={controlledSorting ?? initialSorting}
      onSortingChange={(nextSorting) => {
        const next = fromSortingState(nextSorting);
        if (next) {
          onSortChange?.(next[0], next[1]);
        }
      }}
      rowsPerPageOptions={pageSizeOptions}
      pageSize={defaultPageSize}
      totalRowCount={serverTotalCount ?? data.length}
      enableRowSelection={selectionMode !== "none"}
      rowSelection={rowSelection}
      onRowSelectionChange={(nextSelection) => {
        onSelectionChange?.(
          new Set(
            Object.entries(nextSelection)
              .filter(([, selected]) => selected)
              .map(([key]) => key),
          ),
        );
      }}
      selectionActions={proSelectionActions}
      rowActions={proRowActions}
      onRowClick={onRowClick ? ({ row }) => onRowClick(row) : undefined}
      cardRenderer={toCardRenderer(cardRenderer, rowActions)}
      viewMode={toProViewMode(viewMode ?? defaultViewMode)}
      onViewModeChange={(nextMode) => onViewModeChange?.(fromProViewMode(nextMode))}
      enableViewToggle={Boolean(showViewModeToggle && cardRenderer)}
      emptyState={emptyContent}
      isLoading={isLoading}
      loadingRowCount={skeletonRowCount}
      infiniteScroll={
        paginationMode === "infinite" && onLoadMore
          ? {
              enabled: true,
              hasMore: Boolean(hasMore),
              isLoadingMore,
              onLoadMore,
            }
          : undefined
      }
      showFooter={showItemCount !== false && paginationMode !== "none"}
      showToolbar={!hideToolbar}
      enableColumnResizing={enableColumnResize}
      className={classNames?.wrapper}
      tableClassName={classNames?.table}
      tableContainerClassName={classNames?.tableContainer}
      stickyHeader={fillHeight !== false}
      aria-label={ariaLabel}
    >
      {footerContent}
    </ProDataTable>
  );
}
