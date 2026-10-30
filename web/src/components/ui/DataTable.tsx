import { ActionBar, defineActions, type ActionDefinition } from "glass-ux/actions";
import { Button } from "glass-ux/button";
import {
  createDataTableColumns, DataTable as GlassDataTable, filters, localPage,
  type CardRole, type ColumnFilterSetting, type ColumnPriority,
  type DataTableColumn as GlassColumn, type DataTableInfinite,
  type DataTableSearch, type DataTableState,
} from "glass-ux/data-table";
import { SearchField } from "glass-ux/input";
import { ScrollArea } from "glass-ux/scroll-area";
import { Surface } from "glass-ux/surface";
import { createContext, isValidElement, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import { useTableUrlState } from "@/hooks/useTableUrlState";
import { usePrintMode } from "@/hooks/usePrintMode";
import type { SortingState } from "@/hooks/useServerTable";
import { cn } from "@/lib/utils";

import { SkeletonCard } from "./Skeletons";
import type { ViewMode } from "./Toolbar";

export type Breakpoint = "sm" | "md" | "lg" | "xl";

export interface DataTableColumn<TRow> {
  id: string;
  header: ReactNode;
  cell: (row: TRow) => ReactNode;
  /** Preferred width in legacy CSS pixels; converted to Glass rem at 16px/rem. */
  size?: number;
  align?: "start" | "center" | "end";
  sortable?: boolean;
  /** Lower-priority fields fold into details as the container narrows. */
  hideBelow?: Breakpoint;
  className?: string;
  numeric?: boolean;
  wrap?: boolean;
  /** Typed value for local sorting/filtering, especially computed or formatted cells. */
  value?: (row: TRow) => unknown;
  /** Complete collections infer readable text/numeric filters; override for units or other types. */
  filter?: ColumnFilterSetting;
  priority?: ColumnPriority;
  cardRole?: CardRole;
}

export interface DataTableRowAction<TRow> {
  key: string;
  label: string | ((row: TRow) => string);
  icon?: ReactNode;
  onAction: (row: TRow) => void | Promise<void>;
  destructive?: boolean;
  hidden?: (row: TRow) => boolean;
  disabledReason?: (row: TRow) => string | false | undefined;
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
  /** Application-owned poster gallery, separate from Glass's responsive record cards. */
  renderCard?: (row: TRow, selected: boolean) => ReactNode;
  view?: ViewMode;
  isLoading?: boolean;
  /** Background replacement: retain rows and pause infinite requests until it finishes. */
  refetching?: boolean;
  /** Serializable dataset identity (scope and external server filters); stable on append. */
  queryKey?: unknown;
  /** Refresh generation: rearm loaders at equal row counts without clearing selection. */
  loadRevision?: number;
  /** Retained for the poster gallery; Glass owns its table skeleton geometry. */
  skeletonRows?: number;
  emptyState?: ReactNode;
  error?: ReactNode;
  onRowClick?: (row: TRow) => void;
  rowActions?: Array<DataTableRowAction<TRow>>;
  bulkActions?: Array<DataTableBulkAction<TRow>>;
  selectable?: boolean;
  sorting?: SortingState[];
  onSortingChange?: (sorting: SortingState[]) => void;
  /** @deprecated Supply accumulated rows and infinite. No pagination controls render. */
  pageIndex?: number;
  /** @deprecated Use initialBatchSize for complete collections. */
  pageSize?: number;
  totalCount?: number;
  /** @deprecated Supply infinite.hasMore. */
  hasNextPage?: boolean;
  /** @deprecated Supply infinite.loadMore that appends rows. */
  onPageIndexChange?: (index: number) => void;
  /** @deprecated No page-size control is rendered. */
  onPageSizeChange?: (size: number) => void;
  /** @deprecated No page-size control is rendered. */
  pageSizeOptions?: number[];
  density?: "compact" | "comfortable";
  className?: string;
  noun?: string;
  cardGridClassName?: string;
  /** Optional surface for embedded compositions; main-page tables sit on the page. */
  frame?: boolean;
  /** Fill a bounded application pane; auto is for tables embedded in scrolling detail pages. */
  height?: "fill" | "auto";
  infinite?: { hasMore: boolean; loadMore: () => void | Promise<unknown>; loadingMore?: boolean; error?: ReactNode };
  /** Never locally filter an incomplete server result. Inferred from infinite when omitted. */
  dataSource?: "complete" | "server-infinite";
  /** A caller-owned search requests a new complete server result in server-infinite mode. */
  search?: { value: string; onChange: (value: string) => void; placeholder?: string };
  /** The browser already provides dataset-wide search/filter controls. */
  externalBrowserToolbar?: boolean;
  /** Search complete rows, including computed labels absent from the raw data. */
  getSearchText?: (row: TRow) => string;
  initialBatchSize?: number;
  /** Stable preference namespace; recommended when several similar tables share a route. */
  id?: string;
  label?: string;
  /** Container width in rem below which records become cards. Defaults to 48. */
  cardBreakpoint?: number;
}

const PRIORITY: Record<Breakpoint, ColumnPriority> = { sm: 2, md: 3, lg: 4, xl: 5 };
const NO_MORE: DataTableInfinite = { hasMore: false, onLoadMore: () => {} };
const EMPTY_SELECTION: DataTableState["selection"] = {};

/** The old action API accepts rendered icons; Glass expects a named icon component. */
function compatibilityIcon(node: ReactNode, name: string): ActionDefinition<unknown>["icon"] {
  return node ? Object.assign(() => <>{node}</>, { iconName: name }) : undefined;
}

/** Read plain labels without rendering components or invoking their hooks. */
function nodeText(node: ReactNode): string {
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(nodeText).join(" ");
  if (isValidElement<{ children?: ReactNode; title?: ReactNode; label?: ReactNode }>(node)) {
    return nodeText(node.props.children ?? node.props.title ?? node.props.label);
  }
  return "";
}

function rowText(value: unknown, seen = new Set<object>()): string {
  if (value == null) return "";
  if (typeof value !== "object") return typeof value === "function" ? "" : String(value);
  if (value instanceof Date) return value.toISOString();
  if (seen.has(value)) return "";
  seen.add(value);
  return Object.entries(value).filter(([key]) => key !== "__typename").map(([, item]) => rowText(item, seen)).join(" ");
}

function columnValue<TRow>(column: DataTableColumn<TRow>, row: TRow): unknown {
  if (column.value) return column.value(row);
  const raw = column.id.split(".").reduce<unknown>((value, key) => value && typeof value === "object" ? (value as Record<string, unknown>)[key] : undefined, row);
  if (raw !== undefined && (raw === null || typeof raw !== "object")) return raw;
  const rendered = column.cell(row);
  // Preserve counts as numbers. Formatted strings ("3/∞", "1.2 GiB") stay text
  // unless the caller supplies an explicit typed value and, where needed, units.
  return typeof rendered === "number" ? rendered : nodeText(rendered) || null;
}

function defaultFilterKind<TRow>(column: DataTableColumn<TRow>, rows: TRow[]): "number" | "text" | null {
  let readable = false;
  let numeric = Boolean(column.numeric);
  for (const row of rows) {
    const value = columnValue(column, row);
    if (value == null || value === "") continue;
    if (typeof value === "number" && Number.isFinite(value)) readable = true;
    else if (typeof value === "string" || typeof value === "boolean") { readable = true; numeric = false; }
    else numeric = false;
  }
  // Opaque React components cannot be rendered just to discover their label. A
  // value callback (or explicit filter) is the contract for composed domain cells.
  return readable || column.value ? numeric ? "number" : "text" : null;
}

// TanStack uses cell functions as component types. These stable components consult
// context so inline caller columns and updated action/status closures never remount cells.
interface Presentation {
  cell: (id: string, row: unknown) => ReactNode;
  header: (id: string) => ReactNode;
}
const PresentationContext = createContext<Presentation | null>(null);
function usePresentation() {
  const value = useContext(PresentationContext);
  if (!value) throw new Error("Table cells require their presentation provider");
  return value;
}

function createCell(id: string) {
  return function CompatibilityCell({ row }: { row: { original: unknown } }) {
    return usePresentation().cell(id, row.original);
  };
}
function createHeader(id: string) {
  return function CompatibilityHeader() {
    return usePresentation().header(id);
  };
}

export function DataTable<TRow extends object>(props: DataTableProps<TRow>) {
  const {
    columns, rows, getRowId, renderCard, view = "table", isLoading = false,
    skeletonRows = 8, emptyState, error, onRowClick, rowActions, bulkActions,
    selectable, sorting: controlledSorting, onSortingChange, totalCount,
    density = "comfortable", className, noun = "items", cardGridClassName,
    frame = false, height = "auto", infinite, search, externalBrowserToolbar = false, getSearchText,
    initialBatchSize = 50, label = noun, cardBreakpoint = 48,
  } = props;
  // A legacy paged caller is incomplete too, until its owner supplies append loading.
  const complete = (props.dataSource ?? (infinite || props.onPageIndexChange ? "server-infinite" : "complete")) === "complete";
  const tableId = props.id ?? `librarian:${typeof window === "undefined" ? "" : window.location.pathname}:${noun}:${columns.map((column) => column.id).join(",")}`;
  const refetching = Boolean(props.refetching || (isLoading && rows.length > 0));
  const batchSize = Number.isFinite(initialBatchSize) ? Math.max(1, Math.floor(initialBatchSize)) : 50;
  const [localState, setLocalState] = useState<DataTableState>(() => ({ sorting: [], filters: [], search: "", selection: {}, pagination: { pageIndex: 0, pageSize: batchSize } }));
  const columnMap = useMemo(() => new Map(columns.map((column) => [column.id, column])), [columns]);
  const latestColumns = useRef(columnMap);
  latestColumns.current = columnMap;
  const filterSpecs = columns.map((column) => column.filter);
  const stableFilterSpecs = useRef(filterSpecs);
  if (filterSpecs.length !== stableFilterSpecs.current.length || filterSpecs.some((filter, index) => filter !== stableFilterSpecs.current[index])) stableFilterSpecs.current = filterSpecs;
  const sortable = Boolean(onSortingChange) || (complete && controlledSorting === undefined);
  const defaultFilterKinds = useMemo(() => columns.map((column) => complete && !column.filter ? defaultFilterKind(column, rows) : null), [columns, rows, complete]);
  const defaultFilterSchema = JSON.stringify(defaultFilterKinds);
  const columnSchema = JSON.stringify(columns.map((column) => ({
    id: column.id, label: nodeText(column.header), size: column.size, align: column.align,
    sortable: column.sortable, hideBelow: column.hideBelow, wrap: column.wrap, numeric: column.numeric,
    priority: column.priority, cardRole: column.cardRole,
  })));
  const presentation = useMemo<Presentation>(() => ({
    cell: (id, row) => {
      const column = columnMap.get(id);
      return column ? <span className={cn("block min-w-0 max-w-full flex-1", column.wrap && "whitespace-normal break-words", column.numeric && "text-numeric", column.className)}>{column.cell(row as TRow)}</span> : null;
    },
    header: (id) => columnMap.get(id)?.header,
  }), [columnMap]);

  // Keep renderer identities for each column id for the lifetime of this table,
  // including changes to width, sorting support or the caller's inline closures.
  const renderers = useRef(new Map<string, { cell: ReturnType<typeof createCell>; header: ReturnType<typeof createHeader> }>());
  const adaptedColumns = useMemo<Array<GlassColumn<TRow>>>(() => {
    const col = createDataTableColumns<TRow>();
    return columns.map((column, index) => {
      let renderer = renderers.current.get(column.id);
      if (!renderer) {
        renderer = { cell: createCell(column.id), header: createHeader(column.id) };
        renderers.current.set(column.id, renderer);
      }
      const width = (column.size ?? (index === 0 ? 240 : 140)) / 16;
      const definition = col.accessor((row: TRow) => {
        const current = latestColumns.current.get(column.id);
        return current ? columnValue(current, row) : undefined;
      }, {
        id: column.id, header: nodeText(column.header) || column.id,
        cell: renderer.cell, width, minWidth: Math.min(width, index === 0 ? 10 : 5),
        grow: index === 0 || column.size === undefined, align: column.align,
        sortable: Boolean(column.sortable && sortable), sortDescFirst: false,
        priority: column.priority ?? (index === 0 ? 1 : column.hideBelow ? PRIORITY[column.hideBelow] : 2),
        cardRole: column.cardRole ?? (index === 0 ? "title" : /status/i.test(column.id) ? "status" : undefined),
        truncate: column.wrap || index === 0 ? "wrap" : "ellipsis",
        filter: complete ? column.filter ?? (defaultFilterKinds[index] === "number"
          ? filters.number({ field: column.id, server: "decimal" })
          : defaultFilterKinds[index] === "text" ? filters.text({ field: column.id }) : undefined) : undefined,
      });
      return { ...definition, id: column.id, header: renderer.header };
    });
    // Caller render/value functions arrive through context and latestColumns;
    // only declarative schema changes replace the Glass column definitions.
  }, [columnSchema, defaultFilterSchema, complete, sortable, stableFilterSpecs.current]);
  const url = useTableUrlState({
    namespace: tableId,
    columns: complete ? adaptedColumns : [],
    sortableColumns: complete && controlledSorting === undefined ? columns.filter((column) => column.sortable).map((column) => column.id) : [],
    search: complete && !search && !externalBrowserToolbar,
  });
  const printing = usePrintMode();
  const query = search?.value ?? (complete ? url.state.search : localState.search) ?? "";
  const sorting = controlledSorting ?? (complete ? url.state.sorting : localState.sorting);
  const activeFilters = complete ? url.state.filters : localState.filters;
  const datasetKey = JSON.stringify([tableId, props.queryKey ?? null, query, activeFilters, sorting]);
  const [dataset, setDataset] = useState({ key: datasetKey, revision: 0 });
  const datasetChanged = dataset.key !== datasetKey;
  const datasetRevision = dataset.revision + Number(datasetChanged);
  // Returning to a previous filter is a fresh result too, even if no batch was
  // requested in between. The revision invalidates both loaders' last request.
  const queryKey = JSON.stringify([datasetKey, datasetRevision]);
  // A refreshed snapshot may have exactly the same rows. Only loader identity
  // advances; the semantic dataset and owned selection stay intact.
  const loadQueryKey = JSON.stringify([queryKey, props.loadRevision ?? 0]);
  // Clear controlled selection on the first render of a new server result. Glass
  // prunes its selected-object cache when it receives this empty selection; no
  // remount is needed, so appending rows and background refresh preserve focus.
  const selection = !complete && datasetChanged ? EMPTY_SELECTION : localState.selection;
  useEffect(() => {
    if (!datasetChanged) return;
    setDataset({ key: datasetKey, revision: datasetRevision });
    if (!complete) setLocalState((previous) => ({ ...previous, selection: EMPTY_SELECTION }));
  }, [complete, datasetChanged, datasetKey, datasetRevision]);
  const state = useMemo(() => ({ ...localState, filters: activeFilters, sorting, search: query, selection }), [localState, activeFilters, sorting, query, selection]);
  const [batch, setBatch] = useState({ key: "", size: batchSize });
  const visibleCount = batch.key === queryKey ? batch.size : batchSize;
  const searchSpec = useMemo<DataTableSearch<TRow>>(() => ({
    fields: [{ field: "row", value: (row) => getSearchText ? getSearchText(row) : [rowText(row), ...columns.flatMap((column) => [rowText(columnValue(column, row)), nodeText(column.cell(row))])].join(" ") }],
    placeholder: search?.placeholder ?? `Search ${noun}`,
  }), [columns, getSearchText, noun, search?.placeholder]);
  const result = useMemo(() => complete ? localPage(rows, adaptedColumns, { ...state, pagination: { pageIndex: 0, pageSize: printing ? rows.length || 1 : visibleCount } }, searchSpec) : { rows, count: totalCount }, [complete, rows, adaptedColumns, state, visibleCount, searchSpec, totalCount, printing]);
  const loadMore = useCallback(() => setBatch((previous) => ({ key: queryKey, size: (previous.key === queryKey ? previous.size : batchSize) + batchSize })), [queryKey, batchSize]);
  const loading: DataTableInfinite = complete
    ? { hasMore: !printing && visibleCount < (result.count ?? 0), onLoadMore: loadMore, revision: props.loadRevision }
    : infinite ? { hasMore: !printing && infinite.hasMore, onLoadMore: infinite.loadMore, revision: props.loadRevision, loadingMore: infinite.loadingMore, error: infinite.error } : NO_MORE;
  const actions = useMemo(() => {
    const definitions: Array<ActionDefinition<TRow>> = [
      ...(rowActions ?? []).map((action): ActionDefinition<TRow> => ({
        id: `row:${action.key}`, label: (targets) => typeof action.label === "function" ? action.label(targets[0]!) : action.label,
        disabledReason: (targets) => action.disabledReason?.(targets[0]!),
        icon: compatibilityIcon(action.icon, action.key),
        group: action.destructive ? "danger" : "manage", surfaces: ["menu", "context"],
        visible: (targets) => targets.length === 1 && !action.hidden?.(targets[0]!),
        run: (targets) => action.onAction(targets[0]!),
      })),
      ...(bulkActions ?? []).map((action): ActionDefinition<TRow> => ({
        id: `bulk:${action.key}`, label: action.label, icon: compatibilityIcon(action.icon, action.key),
        group: action.destructive ? "danger" : "manage", surfaces: ["bulk", "header"], bulk: true,
        run: (targets) => action.onAction([...targets]),
      })),
    ];
    return defineActions<TRow>(noun, definitions);
  }, [rowActions, bulkActions, noun]);
  const changeState = useCallback((next: DataTableState, change: keyof DataTableState) => {
    if (complete) url.onStateChange(next, change);
    if (change === "sorting") onSortingChange?.(next.sorting);
    if (change === "search") search?.onChange(next.search ?? "");
    setLocalState(next);
  }, [complete, url, onSortingChange, search]);
  // Mount the search/filter bar together once inferred filters are available.
  // Glass 0.9.10 does not remeasure search when only hasFilters changes.
  const showSearch = !externalBrowserToolbar && !(isLoading && rows.length === 0) && (complete || Boolean(search));
  const selectedRows = rows.filter((row) => selection[getRowId(row)]);
  const showGallery = view === "card" && renderCard;
  const GalleryBody = height === "fill" ? ScrollArea : "div";
  const table = (
    <PresentationContext.Provider value={presentation}>
      <GlassDataTable
        id={tableId} label={label} columns={adaptedColumns} rows={result.rows} getRowId={getRowId}
        rowCount={result.count} queryKey={queryKey} state={state} onStateChange={changeState}
        search={showSearch ? searchSpec : undefined} height={height} density={density}
        loading={isLoading && rows.length === 0} refetching={refetching}
        error={error} emptyMessage={emptyState ?? "Nothing here yet"}
        actions={actions} selectable={Boolean(selectable && bulkActions?.length)}
        onRowOpen={onRowClick} rowOpen="click" infinite={loading}
        noun={{ one: noun === "movies" ? "movie" : noun.endsWith("ies") ? `${noun.slice(0, -3)}y` : noun.endsWith("s") ? noun.slice(0, -1) : noun, other: noun }} cardBreakpoint={cardBreakpoint} className={cn("min-w-0", height === "fill" && "flex-1")}
      />
    </PresentationContext.Provider>
  );

  return (
    <div data-print-flow className={cn("flex min-h-0 min-w-0 flex-col gap-3", height === "fill" && "flex-1 overflow-hidden p-(--focus-reach)", className)}>
      {showGallery ? <>
        {showSearch ? <SearchField className="shrink-0" aria-label={`Search ${noun}`} placeholder={searchSpec.placeholder} value={query} onValueChange={(value) => changeState({ ...state, search: value }, "search")} /> : null}
        {selectable && selectedRows.length > 0 ? <div className="flex shrink-0 flex-wrap items-center gap-2">
          <span>{selectedRows.length} selected</span>
          <ActionBar set={actions} targets={selectedRows} label="Selected records" />
          <Button variant="ghost" onClick={() => setLocalState((previous) => ({ ...previous, selection: {} }))}>Clear selection</Button>
        </div> : null}
        <GalleryBody className={height === "fill" ? "min-h-0 flex-1" : undefined} aria-label={height === "fill" ? `${label} results` : undefined}>
        <div className={height === "fill" ? "p-(--focus-reach)" : undefined}>
        {error ? error : !isLoading && result.rows.length === 0 ? emptyState ?? <p>{query ? `No ${noun} match “${query}”` : "Nothing here yet"}</p> : <div className={cn("poster-grid", cardGridClassName)}>
          {isLoading && rows.length === 0 ? Array.from({ length: skeletonRows }, (_, index) => <SkeletonCard key={index} />) : result.rows.map((row) => {
            const id = getRowId(row);
            return <div key={id} data-row-id={id} className="scroll-mt-4">{renderCard(row, Boolean(selection[id]))}</div>;
          })}
        </div>}
        {!error ? <GalleryTail infinite={loading} count={result.rows.length} queryKey={loadQueryKey} refetching={refetching || isLoading} /> : null}
        </div>
        </GalleryBody>
        {height === "fill" ? <div role="status" className="shrink-0 text-label-sm text-muted">{isLoading && rows.length === 0 ? "Loading…" : `${result.rows.length.toLocaleString()}${typeof result.count === "number" && result.count > result.rows.length ? ` of ${result.count.toLocaleString()}` : ""} ${noun}`}</div> : null}
      </> : frame ? <Surface material="panel" radius="surface" padding="none" className={cn("min-w-0", height === "fill" && "flex min-h-0 flex-1 flex-col")}>{table}</Surface> : table}
    </div>
  );
}

/** The media gallery keeps its own scroll sentinel; record tables use Glass's loader. */
function GalleryTail({ infinite, count, queryKey, refetching }: { infinite: DataTableInfinite; count: number; queryKey: string; refetching: boolean }) {
  const sentinel = useRef<HTMLDivElement>(null);
  const requested = useRef<string | null>(null);
  const inFlight = useRef(false);
  const [failure, setFailure] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const batch = JSON.stringify([queryKey, count]);
  const latest = useRef({ infinite, batch, refetching });
  latest.current = { infinite, batch, refetching };
  const run = useCallback(async () => {
    const current = latest.current;
    if (!current.infinite.hasMore || current.infinite.loadingMore || current.refetching || inFlight.current) return;
    inFlight.current = true;
    requested.current = current.batch;
    setFailure(null);
    setPending(true);
    try { await current.infinite.onLoadMore(); } catch { setFailure(current.batch); }
    finally { inFlight.current = false; setPending(false); }
  }, []);
  const failed = failure === batch || Boolean(infinite.error);
  useEffect(() => {
    const element = sentinel.current;
    if (!element || !infinite.hasMore || infinite.loadingMore || refetching || pending || failed || typeof IntersectionObserver === "undefined") return;
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting) && requested.current !== latest.current.batch) void run();
    }, { rootMargin: "600px 0px" });
    observer.observe(element);
    return () => observer.disconnect();
  }, [infinite.hasMore, infinite.loadingMore, refetching, pending, failed, batch, run]);
  return <div ref={sentinel} className="flex min-h-10 items-center justify-center gap-2 text-label-sm text-muted" aria-live="polite">
    {failed ? <>{infinite.error ?? "Could not load more"}<Button size="sm" onClick={() => void run()}>Retry</Button></> : pending || infinite.loadingMore ? "Loading more…" : null}
  </div>;
}
