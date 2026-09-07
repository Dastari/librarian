import { Checkbox, Dropdown, Select, ListBox, ListBoxItem } from "@heroui/react";
import { IconArrowDown, IconArrowUp, IconArrowsSort, IconChevronLeft, IconChevronRight, IconChevronsLeft, IconChevronsRight, IconDotsVertical, IconInbox } from "@tabler/icons-react";
import { useCallback, useEffect, useMemo, useRef, useState, type Key, type ReactNode } from "react";

import type { SortingState } from "@/hooks/useServerTable";
import { cn } from "@/lib/utils";

import { Button } from "./Button";

import { EmptyState } from "./EmptyState";
import { SkeletonCard } from "./Skeletons";
import type { ViewMode } from "./Toolbar";

/*
 * Librarian's data table. One component renders both the table and the card grid so a list
 * defines its columns and card once. Everything is themed through tokens; there is no external
 * table runtime.
 */

export type Breakpoint = "sm" | "md" | "lg" | "xl";

export interface DataTableColumn<TRow> {
  id: string;
  header: ReactNode;
  cell: (row: TRow) => ReactNode;
  /** Preferred width in px; the first column grows to fill. */
  size?: number;
  align?: "start" | "center" | "end";
  sortable?: boolean;
  /** Hide the column when the viewport is below this breakpoint. */
  hideBelow?: Breakpoint;
  className?: string;
  numeric?: boolean;
  /** Wrap long text instead of truncating (titles in narrow dialogs). */
  wrap?: boolean;
}

export interface DataTableRowAction<TRow> {
  key: string;
  label: string | ((row: TRow) => string);
  icon?: ReactNode;
  onAction: (row: TRow) => void | Promise<void>;
  destructive?: boolean;
  hidden?: (row: TRow) => boolean;
}

export interface DataTableBulkAction<TRow> {
  key: string;
  label: string;
  icon?: ReactNode;
  onAction: (rows: TRow[]) => void | Promise<void>;
  destructive?: boolean;
}

export interface DataTableProps<TRow> {
  columns: Array<DataTableColumn<TRow>>;
  rows: TRow[];
  getRowId: (row: TRow) => string;
  /** Card grid renderer; enables the card view. */
  renderCard?: (row: TRow, selected: boolean) => ReactNode;
  view?: ViewMode;
  isLoading?: boolean;
  skeletonRows?: number;
  emptyState?: ReactNode;
  error?: ReactNode;
  onRowClick?: (row: TRow) => void;
  rowActions?: Array<DataTableRowAction<TRow>>;
  bulkActions?: Array<DataTableBulkAction<TRow>>;
  selectable?: boolean;
  sorting?: SortingState[];
  onSortingChange?: (sorting: SortingState[]) => void;
  /** Server pagination. */
  pageIndex?: number;
  pageSize?: number;
  totalCount?: number;
  hasNextPage?: boolean;
  onPageIndexChange?: (index: number) => void;
  onPageSizeChange?: (size: number) => void;
  pageSizeOptions?: number[];
  density?: "compact" | "comfortable";
  className?: string;
  /** Text for the footer count, e.g. "movies". */
  noun?: string;
  cardGridClassName?: string;
  /** Draw the bordered surface around the table. Off when the table sits inside a Panel. */
  frame?: boolean;
  /** Infinite scrolling: loads the next page when the end comes into view. Replaces pagination. */
  infinite?: { hasMore: boolean; loadMore: () => void | Promise<unknown>; loadingMore?: boolean };
}

const HIDE_BELOW: Record<Breakpoint, string> = {
  sm: "max-sm:hidden",
  md: "max-md:hidden",
  lg: "max-lg:hidden",
  xl: "max-xl:hidden",
};

const ALIGN: Record<NonNullable<DataTableColumn<unknown>["align"]>, string> = {
  start: "text-left",
  center: "text-center",
  end: "text-right",
};

export function DataTable<TRow>({
  columns,
  rows,
  getRowId,
  renderCard,
  view = "table",
  isLoading,
  skeletonRows = 8,
  emptyState,
  error,
  onRowClick,
  rowActions,
  bulkActions,
  selectable,
  sorting = [],
  onSortingChange,
  pageIndex = 0,
  pageSize,
  totalCount,
  hasNextPage,
  onPageIndexChange,
  onPageSizeChange,
  pageSizeOptions = [25, 50, 100],
  density = "comfortable",
  className,
  noun = "items",
  cardGridClassName,
  frame = true,
  infinite,
}: DataTableProps<TRow>) {
  const sentinel = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const element = sentinel.current;
    if (!element || !infinite?.hasMore) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) void infinite.loadMore();
      },
      { rootMargin: "600px 0px" },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [infinite, infinite?.hasMore, rows.length]);
  const [selected, setSelected] = useState<Set<string>>(() => new Set());
  const hasSelection = Boolean(selectable && bulkActions?.length);
  const ids = useMemo(() => rows.map(getRowId), [rows, getRowId]);
  const allSelected = ids.length > 0 && ids.every((id) => selected.has(id));
  const someSelected = ids.some((id) => selected.has(id));
  const selectedRows = useMemo(() => rows.filter((row) => selected.has(getRowId(row))), [rows, selected, getRowId]);

  const toggleAll = useCallback(() => {
    setSelected((previous) => {
      const next = new Set(previous);
      if (allSelected) ids.forEach((id) => next.delete(id));
      else ids.forEach((id) => next.add(id));
      return next;
    });
  }, [allSelected, ids]);

  const toggleOne = useCallback((id: string, value: boolean) => {
    setSelected((previous) => {
      const next = new Set(previous);
      if (value) next.add(id);
      else next.delete(id);
      return next;
    });
  }, []);

  const toggleSort = (column: DataTableColumn<TRow>) => {
    if (!column.sortable || !onSortingChange) return;
    const current = sorting.find((sort) => sort.id === column.id);
    if (!current) onSortingChange([{ id: column.id, desc: false }]);
    else if (!current.desc) onSortingChange([{ id: column.id, desc: true }]);
    else onSortingChange([]);
  };

  const showCards = view === "card" && renderCard;
  const isEmpty = !isLoading && rows.length === 0;
  const cellPadding = density === "compact" ? "px-3 py-1.5" : "px-3 py-2.5";

  // The first column without an explicit size absorbs the remaining width.
  const growIndex = columns.findIndex((column) => column.size === undefined);

  const pagination = infinite ? (
    <div ref={sentinel} className="flex h-10 items-center justify-center text-label-sm text-muted" aria-live="polite">
      {infinite.loadingMore ? "Loading more…" : ""}
    </div>
  ) : pageSize && onPageIndexChange ? (
    <Pagination pageIndex={pageIndex} pageSize={pageSize} totalCount={totalCount} hasNextPage={hasNextPage} onPageIndexChange={onPageIndexChange} onPageSizeChange={onPageSizeChange} pageSizeOptions={pageSizeOptions} noun={noun} />
  ) : null;

  return (
    <div className={cn("flex min-h-0 flex-col gap-3", className)}>
      {hasSelection && selectedRows.length > 0 ? (
        <div className="glass-surface sticky top-2 z-10 flex flex-wrap items-center gap-2 rounded-card px-3 py-2">
          <span className="text-numeric text-label text-foreground">{selectedRows.length} selected</span>
          <div className="flex-1" />
          {bulkActions!.map((action) => (
            <Button key={action.key} size="sm" variant={action.destructive ? "danger-soft" : "secondary"} onPress={() => void action.onAction(selectedRows)}>
              {action.icon}
              {action.label}
            </Button>
          ))}
          <Button size="sm" variant="ghost" onPress={() => setSelected(new Set())}>
            Clear
          </Button>
        </div>
      ) : null}

      {error ? (
        error
      ) : isEmpty ? (
        emptyState ?? <EmptyState icon={IconInbox} title="Nothing here yet" compact />
      ) : showCards ? (
        <div className={cn("poster-grid", cardGridClassName)}>
          {isLoading && rows.length === 0
            ? Array.from({ length: skeletonRows }, (_, index) => <SkeletonCard key={index} />)
            : rows.map((row) => {
                const id = getRowId(row);
                return (
                  <div key={id} data-row-id={id} className="scroll-mt-4">
                    {renderCard(row, selected.has(id))}
                  </div>
                );
              })}
        </div>
      ) : (
        <div className={cn("scrollbar-thin -mx-1 overflow-x-auto", frame && "glass-surface rounded-card")}>
          <table className={cn("w-full border-collapse text-body-sm", growIndex >= 0 ? "table-fixed" : "table-auto")}>
            <colgroup>
              {hasSelection ? <col style={{ width: 44 }} /> : null}
              {columns.map((column, index) => (
                <col key={column.id} style={index === growIndex ? undefined : { width: column.size ?? 140 }} className={column.hideBelow ? HIDE_BELOW[column.hideBelow] : undefined} />
              ))}
              {rowActions?.length ? <col style={{ width: 52 }} /> : null}
            </colgroup>
            <thead className={cn("sticky top-0 z-[1] text-label text-muted", frame ? "glass-opaque backdrop-blur" : "bg-transparent")}>
              <tr>
                {hasSelection ? (
                  <th scope="col" className={cn("border-b border-separator", cellPadding)}>
                    <Checkbox isSelected={allSelected} isIndeterminate={someSelected && !allSelected} onChange={toggleAll} aria-label="Select all">
                      <Checkbox.Control>
                        <Checkbox.Indicator />
                      </Checkbox.Control>
                    </Checkbox>
                  </th>
                ) : null}
                {columns.map((column) => {
                  const sort = sorting.find((item) => item.id === column.id);
                  return (
                    <th
                      key={column.id}
                      scope="col"
                      aria-sort={sort ? (sort.desc ? "descending" : "ascending") : undefined}
                      className={cn("border-b border-separator font-medium", cellPadding, ALIGN[column.align ?? "start"], column.hideBelow && HIDE_BELOW[column.hideBelow], column.className)}
                    >
                      {column.sortable && onSortingChange ? (
                        <button type="button" data-focusable onClick={() => toggleSort(column)} className={cn("nav-focus inline-flex items-center gap-1 rounded hover:text-foreground", column.align === "end" && "flex-row-reverse")}>
                          {column.header}
                          {sort ? sort.desc ? <IconArrowDown size={14} /> : <IconArrowUp size={14} /> : <IconArrowsSort size={14} className="opacity-40" />}
                        </button>
                      ) : (
                        column.header
                      )}
                    </th>
                  );
                })}
                {rowActions?.length ? <th scope="col" className={cn("border-b border-separator", cellPadding)} /> : null}
              </tr>
            </thead>
            <tbody>
              {isLoading && rows.length === 0
                ? Array.from({ length: skeletonRows }, (_, index) => (
                    <tr key={index} className="border-b border-separator last:border-b-0">
                      {hasSelection ? <td className={cellPadding} /> : null}
                      {columns.map((column) => (
                        <td key={column.id} className={cn(cellPadding, column.hideBelow && HIDE_BELOW[column.hideBelow])}>
                          <div className="skeleton h-4 w-3/4" />
                        </td>
                      ))}
                      {rowActions?.length ? <td className={cellPadding} /> : null}
                    </tr>
                  ))
                : rows.map((row) => {
                    const id = getRowId(row);
                    const isSelected = selected.has(id);
                    return (
                      <tr
                        key={id}
                        data-row-id={id}
                        data-focusable={onRowClick ? true : undefined}
                        tabIndex={onRowClick ? 0 : undefined}
                        aria-selected={isSelected || undefined}
                        onClick={onRowClick ? () => onRowClick(row) : undefined}
                        onKeyDown={onRowClick ? (event) => {
                          if (event.key === "Enter" && event.target === event.currentTarget) onRowClick(row);
                        } : undefined}
                        className={cn(
                          "nav-focus border-b border-separator transition-colors duration-fast last:border-b-0",
                          onRowClick && "cursor-pointer hover:bg-surface-hover",
                          isSelected && "bg-brand-soft/60",
                        )}
                      >
                        {hasSelection ? (
                          <td className={cellPadding} onClick={(event) => event.stopPropagation()}>
                            <Checkbox isSelected={isSelected} onChange={(value) => toggleOne(id, value)} aria-label="Select row">
                              <Checkbox.Control>
                                <Checkbox.Indicator />
                              </Checkbox.Control>
                            </Checkbox>
                          </td>
                        ) : null}
                        {columns.map((column) => (
                          <td key={column.id} className={cn("align-middle", column.wrap ? "whitespace-normal break-words" : "truncate", cellPadding, ALIGN[column.align ?? "start"], column.numeric && "text-numeric", column.hideBelow && HIDE_BELOW[column.hideBelow], column.className)}>
                            {column.cell(row)}
                          </td>
                        ))}
                        {rowActions?.length ? (
                          <td className={cn(cellPadding, "text-right")} onClick={(event) => event.stopPropagation()}>
                            <RowActionsMenu row={row} actions={rowActions} />
                          </td>
                        ) : null}
                      </tr>
                    );
                  })}
            </tbody>
          </table>
        </div>
      )}
      {pagination}
    </div>
  );
}

function RowActionsMenu<TRow>({ row, actions }: { row: TRow; actions: Array<DataTableRowAction<TRow>> }) {
  const visible = actions.filter((action) => !action.hidden?.(row));
  if (visible.length === 0) return null;
  const onAction = (key: Key) => {
    const action = visible.find((item) => item.key === String(key));
    if (action) void action.onAction(row);
  };
  return (
    <Dropdown>
      <Dropdown.Trigger aria-label="Row actions" className="nav-focus grid size-8 place-items-center rounded-full text-muted hover:bg-surface-hover hover:text-foreground" data-focusable>
        <IconDotsVertical size={16} />
      </Dropdown.Trigger>
      <Dropdown.Popover placement="bottom end" className="glass-surface">
        <Dropdown.Menu aria-label="Row actions" onAction={onAction}>
          {visible.map((action) => (
            <Dropdown.Item key={action.key} id={action.key} textValue={typeof action.label === "function" ? action.label(row) : action.label} variant={action.destructive ? "danger" : "default"}>
              {action.icon}
              {typeof action.label === "function" ? action.label(row) : action.label}
            </Dropdown.Item>
          ))}
        </Dropdown.Menu>
      </Dropdown.Popover>
    </Dropdown>
  );
}

interface PaginationProps {
  pageIndex: number;
  pageSize: number;
  totalCount?: number;
  hasNextPage?: boolean;
  onPageIndexChange: (index: number) => void;
  onPageSizeChange?: (size: number) => void;
  pageSizeOptions: number[];
  noun: string;
}

function Pagination({ pageIndex, pageSize, totalCount, hasNextPage, onPageIndexChange, onPageSizeChange, pageSizeOptions, noun }: PaginationProps) {
  const pageCount = totalCount !== undefined ? Math.max(1, Math.ceil(totalCount / pageSize)) : undefined;
  const canPrevious = pageIndex > 0;
  const canNext = pageCount !== undefined ? pageIndex < pageCount - 1 : Boolean(hasNextPage);
  const from = totalCount === 0 ? 0 : pageIndex * pageSize + 1;
  const to = totalCount !== undefined ? Math.min(totalCount, (pageIndex + 1) * pageSize) : (pageIndex + 1) * pageSize;
  if (totalCount === 0) return null;
  if (totalCount !== undefined && totalCount <= Math.min(...pageSizeOptions) && pageIndex === 0 && !onPageSizeChange) return null;

  return (
    <div className="flex flex-wrap items-center justify-between gap-3 text-label text-muted">
      <span className="text-numeric">
        {totalCount !== undefined ? `${from.toLocaleString()}–${to.toLocaleString()} of ${totalCount.toLocaleString()} ${noun}` : `Page ${pageIndex + 1}`}
      </span>
      <div className="flex items-center gap-2">
        {onPageSizeChange ? (
          <Select aria-label="Rows per page" selectedKey={String(pageSize)} onSelectionChange={(key) => key !== null && onPageSizeChange(Number(key))} className="w-36" variant="secondary">
            <Select.Trigger className="h-8 text-label">
              <Select.Value />
              <Select.Indicator />
            </Select.Trigger>
            <Select.Popover>
              <ListBox>
                {pageSizeOptions.map((option) => (
                  <ListBoxItem key={option} id={String(option)} textValue={`${option} per page`}>
                    {option} per page
                  </ListBoxItem>
                ))}
              </ListBox>
            </Select.Popover>
          </Select>
        ) : null}
        <div className="flex items-center">
          <PageButton label="First page" disabled={!canPrevious} onPress={() => onPageIndexChange(0)} icon={<IconChevronsLeft size={16} />} />
          <PageButton label="Previous page" disabled={!canPrevious} onPress={() => onPageIndexChange(pageIndex - 1)} icon={<IconChevronLeft size={16} />} />
          <span className="text-numeric min-w-16 text-center text-foreground">{pageCount !== undefined ? `${pageIndex + 1} / ${pageCount}` : pageIndex + 1}</span>
          <PageButton label="Next page" disabled={!canNext} onPress={() => onPageIndexChange(pageIndex + 1)} icon={<IconChevronRight size={16} />} />
          {pageCount !== undefined ? <PageButton label="Last page" disabled={!canNext} onPress={() => onPageIndexChange(pageCount - 1)} icon={<IconChevronsRight size={16} />} /> : null}
        </div>
      </div>
    </div>
  );
}

function PageButton({ label, disabled, onPress, icon }: { label: string; disabled: boolean; onPress: () => void; icon: ReactNode }) {
  return (
    <button type="button" data-focusable aria-label={label} disabled={disabled} onClick={onPress} className="nav-focus grid size-8 place-items-center rounded-full text-foreground hover:bg-surface-hover disabled:opacity-30 disabled:hover:bg-transparent">
      {icon}
    </button>
  );
}
