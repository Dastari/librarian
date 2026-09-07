import { useMemo, useCallback, useRef, type Key, type ReactNode } from "react";
import { DataTable as ProDataTable } from "data-table-pro/heroui";
import type {
  DataTableCardRendererProps as ProCardRendererProps,
  DataTableColumnDef,
  DataTableRowAction as ProRowAction,
  DataTableSelectionAction,
  DataTableToolbarAction,
  DataTableViewMode,
} from "data-table-pro/types";
import type { SortingState } from "@tanstack/react-table";
import { RowActionsCell } from "./RowActionsCell";

import type {
  BulkAction,
  CardRendererProps,
  DataTableColumn,
  DataTableProps,
  RowAction,
  SortDirection,
  ToolbarAction,
  ViewMode,
} from "./types";

function toProViewMode(
  mode: ViewMode | undefined,
): DataTableViewMode | undefined {
  if (!mode) {
    return undefined;
  }
  return mode === "cards" ? "card" : "table";
}

function fromProViewMode(mode: DataTableViewMode): ViewMode {
  return mode === "card" ? "cards" : "table";
}

function toSortingState(
  column?: string | null,
  direction?: SortDirection,
): SortingState {
  if (!column) {
    return [];
  }
  return [{ id: column, desc: direction === "desc" }];
}

function fromSortingState(
  sorting: SortingState,
): [string, SortDirection] | null {
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

function toPascalKey(key: string): string {
  return key ? key.charAt(0).toUpperCase() + key.slice(1) : key;
}

function getComparableValue<T>(row: T, key: string): unknown {
  const record = row as Record<string, unknown>;
  const pascalKey = toPascalKey(key);

  if (key in record) {
    return record[key];
  }

  if (pascalKey in record) {
    return record[pascalKey];
  }

  const nestedCandidates = [
    "track",
    "chapter",
    "episode",
    "movie",
    "show",
    "album",
  ];
  for (const nestedKey of nestedCandidates) {
    const nested = record[nestedKey];
    if (!nested || typeof nested !== "object") {
      continue;
    }

    const nestedRecord = nested as Record<string, unknown>;
    if (key in nestedRecord) {
      return nestedRecord[key];
    }
    if (pascalKey in nestedRecord) {
      return nestedRecord[pascalKey];
    }
  }

  return undefined;
}

function compareValues(left: unknown, right: unknown): number {
  if (left == null && right == null) {
    return 0;
  }
  if (left == null) {
    return -1;
  }
  if (right == null) {
    return 1;
  }
  if (typeof left === "number" && typeof right === "number") {
    return left - right;
  }
  if (left instanceof Date && right instanceof Date) {
    return left.getTime() - right.getTime();
  }
  if (typeof left === "boolean" && typeof right === "boolean") {
    return Number(left) - Number(right);
  }

  return String(left).localeCompare(String(right), undefined, {
    numeric: true,
    sensitivity: "base",
  });
}

function toColumnDef<T>(
  column: DataTableColumn<T>,
): DataTableColumnDef<T, unknown> {
  const numericWidth =
    typeof column.width === "number"
      ? column.width
      : typeof column.width === "object" && column.width !== null
        ? column.width.width
        : undefined;
  const minWidth =
    typeof column.width === "number"
      ? column.width
      : typeof column.width === "object" && column.width !== null
        ? column.width.minWidth
        : undefined;
  const maxWidth =
    typeof column.width === "number"
      ? column.width
      : typeof column.width === "object" && column.width !== null
        ? column.width.maxWidth
        : undefined;
  const resizable =
    typeof column.width === "object" && column.width !== null
      ? column.width.resizable
      : undefined;

  return {
    id: column.key,
    accessorKey: column.key as keyof T & string,
    header: column.label,
    size: numericWidth,
    minSize: minWidth,
    maxSize: maxWidth,
    enableResizing: resizable,
    enableSorting: column.sortable !== false,
    sortFn:
      column.sortable === false
        ? undefined
        : (rowA, rowB) =>
            column.sortFn?.(rowA.original, rowB.original) ??
            compareValues(
              getComparableValue(rowA.original, column.key),
              getComparableValue(rowB.original, column.key),
            ),
    cell: column.render
      ? ({ row }) => normalizeContent(column.render?.(row.original, row.index))
      : undefined,
    meta: {
      align: column.align,
      hideOn: column.hideOnMobile ? "md" : undefined,
      minWidth,
      cellClassName:
        column.truncate === false ? "whitespace-normal" : undefined,
      skeleton: column.skeleton ? () => column.skeleton?.() : undefined,
    },
  };
}

function toRowAction<T>(action: RowAction<T>): ProRowAction<T> {
  return {
    key: action.key,
    label: action.label,
    onClick: action.onAction,
    variant:
      action.isDestructive || action.color === "danger"
        ? "destructive"
        : "default",
    hidden: action.isVisible ? (row) => !action.isVisible?.(row) : undefined,
    disabled: action.isDisabled,
  };
}

function toSelectionAction<T>(
  action: BulkAction<T>,
): DataTableSelectionAction<T> {
  return {
    key: action.key,
    label: action.label,
    onClick: ({ rows }) => action.onAction(rows),
    variant:
      action.isDestructive || action.color === "danger"
        ? "destructive"
        : "default",
    disabled:
      typeof action.disabled === "function"
        ? (rows) =>
            Boolean((action.disabled as (selectedItems: T[]) => boolean)(rows))
        : action.disabled,
  };
}

function toToolbarAction<T>(
  action: ToolbarAction<T>,
): DataTableToolbarAction<T> {
  return {
    key: action.key,
    label: action.label,
    icon: action.icon,
    iconOnly: action.iconOnly,
    placement: action.placement,
    onClick: ({ rows }) => action.onAction(rows),
    variant: action.variant,
    disabled: action.disabled,
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
  toolbarQueryPlaceholder,
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
  cardGridClassName,
  toolbarActions,
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
  searchFn,
  filterRowContent,
  headerContent,
  footerContent,
  toolbarContent,
  showItemCount,
  hideToolbar,
  toolbarVisibility,
  classNames,
  fillHeight = false,
  ariaLabel,
  enableColumnResize,
}: DataTableProps<T>) {
  const rowKey = useRef(getRowKey);
  rowKey.current = getRowKey;
  const getRowId = useCallback(
    (row: T) => keyToString(rowKey.current(row)),
    [],
  );
  const hasInlineActions = rowActions?.some(action => action.inDropdown === false) ?? false;
  const proColumns = useMemo(() => {
    const mapped = columns.filter(column => !column.hidden).map(toColumnDef);
    if (hasInlineActions) mapped.push({
      id: "row-actions", header: "Actions", size: 128, minSize: 128,
      enableSorting: false, enableHiding: false, enableResizing: false,
      cell: ({ row }) => <RowActionsCell row={row.original} actions={rowActions ?? []} />,
    });
    return mapped;
  }, [columns, hasInlineActions, rowActions]);
  const proRowActions = useMemo(
    () => (rowActions ?? []).map(toRowAction),
    [rowActions],
  );
  const proSelectionActions = useMemo(
    () => (bulkActions ?? []).map(toSelectionAction),
    [bulkActions],
  );
  const proToolbarActions = useMemo(
    () => (toolbarActions ?? []).map(toToolbarAction),
    [toolbarActions],
  );
  const rowSelection = useMemo(() => {
    if (!selectedKeys) {
      return undefined;
    }
    return Object.fromEntries(
      Array.from(selectedKeys).map((key) => [keyToString(key), true]),
    );
  }, [selectedKeys]);

  const controlledSorting =
    sortColumn !== undefined
      ? toSortingState(sortColumn, sortDirection)
      : undefined;
  const initialSorting =
    controlledSorting ??
    toSortingState(defaultSortColumn, defaultSortDirection);
  const wrapperClassName = [
    fillHeight ? "flex h-0 min-h-0 flex-1 flex-col" : undefined,
    classNames?.wrapper,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={wrapperClassName}>
      {headerContent && <div className="shrink-0 mb-4">{headerContent}</div>}
      {toolbarContent && (
        <div className="shrink-0 flex flex-wrap items-center gap-2 mb-3">
          {toolbarContent}
        </div>
      )}
      {filterRowContent && (
        <div className="shrink-0 mb-3">{filterRowContent}</div>
      )}
      {!isLoading && data.length === 0 ? (
        <div className="flex min-h-0 flex-1 items-center justify-center rounded-xl bg-content1 p-6 text-center">
          {emptyContent ?? "No records found"}
        </div>
      ) : (
        <ProDataTable
          columns={proColumns}
          data={data}
          getRowId={getRowId}
          toolbarQueryPlaceholder={toolbarQueryPlaceholder}
          onToolbarQueryValueChange={onSearchChange}
          manualSorting={Boolean(serverSide)}
          manualFiltering={Boolean(serverSide)}
          manualPagination={
            paginationMode === "none" || paginationMode === "infinite"
          }
          globalFilterFn={
            searchFn
              ? (row, _columnId, query) =>
                  searchFn(row.original, String(query ?? ""))
              : undefined
          }
          sorting={controlledSorting}
          initialState={{ sorting: initialSorting }}
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
          toolbarActions={proToolbarActions}
          rowActions={hasInlineActions ? [] : proRowActions}
          getRowClassName={hasInlineActions ? () => "group" : undefined}
          onRowClick={onRowClick ? ({ row }) => onRowClick(row) : undefined}
          cardRenderer={toCardRenderer(cardRenderer, rowActions)}
          cardSizing={cardRenderer ? "fluid" : undefined}
          cardGridClassName={cardGridClassName}
          viewMode={toProViewMode(viewMode ?? defaultViewMode)}
          onViewModeChange={(nextMode) =>
            onViewModeChange?.(fromProViewMode(nextMode))
          }
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
          toolbarVisibility={toolbarVisibility}
          enableColumnResizing={enableColumnResize}
          layoutMode="fill"
          flexGrow={fillHeight}
          className={fillHeight ? "min-h-0 flex-1" : undefined}
          tableClassName={classNames?.table}
          tableContainerClassName={classNames?.tableContainer}
          stickyHeader={fillHeight}
          aria-label={ariaLabel}
        >
          {footerContent}
        </ProDataTable>
      )}
    </div>
  );
}
