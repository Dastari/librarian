import type { ActionSet } from "glass-ux/actions";
import { DataTable, type DataTableColumn, type DataTableCard, type DataTableState, type DataTableStateChange } from "glass-ux/data-table";
import { useCallback, useMemo, type KeyboardEvent, type ReactNode, type RefObject } from "react";

import { AlphabetRail, ErrorState, PosterInTable, GlassSegmented, SkeletonCard, ViewToggle, type ViewMode } from "@/components/ui";
import { TableToolbar } from "@/components/ui/TableToolbar";
import { useSession } from "@/lib/auth/useSession";
import { usePref } from "@/lib/prefs";
import { useIsDesktop } from "@/hooks/useMediaQuery";

import { BrowserLetterRail, type AvailabilityFilter } from "./BrowserToolbar";

/** The poster width the grid is laid out from, in rem. The grid fills the row from there. */
const POSTER_MIN = 9.5;

const renderPosterSkeleton = () => <SkeletonCard />;

/** A remote's Play key plays the focused poster, whose play button isn't a stop of its own. */
function playFocused(event: KeyboardEvent<HTMLElement>) {
  if (event.key !== "MediaPlay" && event.key !== "MediaPlayPause") return;
  const play = event.target instanceof HTMLElement ? event.target.querySelector<HTMLElement>('.fa-dt-card-slot button[aria-label^="Play "]') : null;
  if (!play) return;
  event.preventDefault();
  play.click();
}

const AVAILABILITY_ITEMS: Array<{ key: AvailabilityFilter; label: string }> = [
  { key: "all", label: "All" },
  { key: "available", label: "Available" },
  { key: "wanted", label: "Wanted" },
  { key: "missing", label: "Missing" },
];

interface MediaBrowserTableProps<T extends object> {
  /** What this list's view and columns are remembered under. */
  persistKey: string;
  label: string;
  noun: { one: string; other: string };
  placeholder: string;
  columns: readonly DataTableColumn<T>[];
  list: {
    rows: T[];
    queryKey: string;
    loading: boolean;
    fetching: boolean;
    totalCount?: number | null;
    hasMore: boolean;
    loadingMore?: boolean;
    loadMore: () => Promise<unknown>;
    loadRevision?: string | number;
    error?: Error | null;
    refetch: () => Promise<unknown>;
  };
  sorting: { id: string; desc: boolean }[];
  onSortingChange: (sorting: { id: string; desc: boolean }[]) => void;
  query: string;
  onQueryChange: (query: string) => void;
  availability?: AvailabilityFilter;
  onAvailabilityChange?: (value: AvailabilityFilter) => void;
  getRowId: (row: T) => string;
  getRowLabel: (row: T) => string;
  /** The poster a row shows as, with the table's frame taken away. Keep it stable. */
  renderPoster: (row: T) => ReactNode;
  actions?: ActionSet<T>;
  onRowOpen: (row: T) => void;
  emptyMessage: string;
  emptyAction?: ReactNode;
  trailing?: ReactNode;
  letter: { active: string | null; jump: (letter: string | null) => void; containerRef: RefObject<HTMLDivElement | null> };
}

/**
 * A library's titles in one table, shown as posters or rows. Either way the search, sort, filters,
 * batches and right-click are the table's. Letters jump within it, beside it on a desktop and above
 * it elsewhere.
 */
export function MediaBrowserTable<T extends object>({ persistKey, label, noun, placeholder, columns, list, sorting, onSortingChange, query, onQueryChange, availability, onAvailabilityChange, getRowId, getRowLabel, renderPoster, actions, onRowOpen, emptyMessage, emptyAction, trailing, letter }: MediaBrowserTableProps<T>) {
  const { user } = useSession();
  const desktop = useIsDesktop();
  const [view, setView] = usePref<ViewMode>(`browser.view.${persistKey}`, "card");
  const search = useMemo(() => ({ fields: [], placeholder, toCondition: () => ({}) }), [placeholder]);
  const state = useMemo<DataTableState>(() => ({ pagination: { pageIndex: 0, pageSize: 50 }, selection: {}, filters: [], sorting, search: query }), [sorting, query]);
  const onStateChange = useCallback((next: DataTableState, change: DataTableStateChange) => {
    if (change === "search") onQueryChange(next.search ?? "");
    else if (change === "sorting") onSortingChange(next.sorting);
  }, [onQueryChange, onSortingChange]);
  const renderCard = useCallback((row: T, _card: DataTableCard) => renderPoster(row), [renderPoster]);

  return (
    <div ref={letter.containerRef} className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 sm:gap-4">
      {!desktop ? <AlphabetRail orientation="horizontal" value={letter.active} onChange={letter.jump} /> : null}
      <div className="flex min-h-0 min-w-0 flex-1 gap-3 sm:gap-4">
        <div className="media-browser flex min-h-0 min-w-0 flex-1 flex-col" data-view={view} onKeyDown={playFocused}>
          <PosterInTable.Provider value={true}>
          <DataTable<T>
            id={`librarian.${user?.id ?? "session"}.browser.${persistKey}`}
            label={label}
            queryKey={list.queryKey}
            height="fill"
            // A person's choice of table still gets cards where a table can't fit.
            view={view === "card" ? "cards" : "auto"}
            cardMinWidth={POSTER_MIN}
            renderCard={renderCard}
            renderCardSkeleton={renderPosterSkeleton}
            columns={columns as DataTableColumn<T>[]}
            rows={list.rows}
            getRowId={getRowId}
            getRowLabel={getRowLabel}
            loading={list.loading}
            refetching={list.fetching && !list.loading}
            rowCount={list.totalCount ?? undefined}
            state={state}
            onStateChange={onStateChange}
            infinite={{ revision: list.loadRevision, hasMore: list.hasMore, onLoadMore: list.loadMore, loadingMore: list.loadingMore }}
            selectable={false}
            actions={actions}
            noun={noun}
            search={search}
            toolbar={(
              <TableToolbar
                filter={onAvailabilityChange ? <GlassSegmented<AvailabilityFilter> ariaLabel="Availability" size="sm" value={availability ?? "all"} onChange={onAvailabilityChange} items={AVAILABILITY_ITEMS} /> : undefined}
              >
                <ViewToggle value={view} onChange={setView} className="shrink-0" />
                {trailing}
              </TableToolbar>
            )}
            onRowOpen={onRowOpen}
            rowOpen="click"
            error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
            emptyMessage={emptyMessage}
            emptyAction={emptyAction}
          />
          </PosterInTable.Provider>
        </div>
        <BrowserLetterRail letter={letter.active} onLetterChange={letter.jump} />
      </div>
    </div>
  );
}
