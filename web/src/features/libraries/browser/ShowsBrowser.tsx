
import { IconDeviceTv, IconPlus } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Button, EmptyState, ErrorState, PosterCard, PosterGallery } from "@/components/ui";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { LibraryShowsDocument, type LibraryShowsQuery, type ShowOrderByInput, type ShowWhereInput } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { showPoster } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { cn } from "@/lib/utils";

import { BrowserLetterRail, BrowserToolbar, type BrowserSortOption } from "./BrowserToolbar";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type ShowRow = LibraryShowsQuery["shows"]["edges"][number]["node"];
const getRowId = (row: ShowRow) => row.id;
const SORTS: readonly BrowserSortOption[] = [
  { id: "name", desc: false, label: "Name" },
  { id: "createdAt", desc: true, label: "Recently added" },
];

/** "12/24" style progress used for shows, albums and audiobooks. */
export function ProgressBadge({ have, total, className }: { have: number; total: number; className?: string }) {
  if (total === 0) return <span className={cn("text-numeric text-muted", className)}>—</span>;
  const complete = have >= total;
  return (
    <span className={cn("text-numeric", complete ? "text-success" : "text-warning", className)}>
      {have}/{total}
    </span>
  );
}

export function ShowsBrowser({ libraryId }: { libraryId: string }) {
  const isAdmin = useIsAdmin();
  const filters = useBrowserFilters();
  const table = useServerTable<ShowOrderByInput>({ persistKey: `shows.${libraryId}`, defaultSorting: [{ id: "name", desc: false }], sortMap: { name: "sortName" } });
  const [adding, setAdding] = useState(false);

  const where = useMemo<ShowWhereInput>(() => {
    const clause: ShowWhereInput = { libraryId: { eq: libraryId } };
    const name = titleFilter(filters.debouncedQuery, null);
    if (name) clause.name = name;
    return clause;
  }, [libraryId, filters.debouncedQuery]);

  const list = useInfiniteConnection(LibraryShowsDocument, { where, orderBy: table.orderBy }, (data) => data.shows);
  const jump = useLetterJump({
    rows: list.rows,
    getRowId,
    getTitle: (row) => row.name,
    hasMore: list.hasMore,
    loading: list.fetching || list.loadingMore,
    queryKey: JSON.stringify(where),
    isTitleSorted: table.sorting.length === 1 && table.sorting[0]?.id === "name" && !table.sorting[0].desc,
    loadMore: list.loadMore,
    ensureTitleSort: () => table.setSorting([{ id: "name", desc: false }]),
  });
  const counts = (row: ShowRow) => ({ have: row.downloaded.pageInfo.totalCount ?? 0, total: row.episodes.pageInfo.totalCount ?? 0 });

  const filtered = Boolean(filters.debouncedQuery);
  return (
    <div ref={jump.containerRef} className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 sm:gap-4">
      <BrowserToolbar
        query={filters.query}
        onQueryChange={filters.setQuery}
        letter={jump.active}
        onLetterChange={jump.jump} sort={{ options: SORTS, value: table.sorting, onChange: table.setSorting }}
        total={list.totalCount}
        placeholder="Filter shows"
        trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size={16} /> Add</Button> : null}
      />
      <div className="flex min-h-0 min-w-0 flex-1 gap-3 sm:gap-4">
        <PosterGallery<ShowRow>
          label="Shows"
          rows={list.rows}
          getRowId={getRowId}
          loading={list.loading}
          refetching={list.fetching}
          queryKey={list.queryKey}
          totalCount={list.totalCount}
          noun="shows"
          infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyState={
            <EmptyState
              icon={IconDeviceTv}
              title={filtered ? "No shows match" : "No shows yet"}
              description={filtered ? "Try clearing the filters." : "Scan the library folder or add a show from TVmaze."}
              action={isAdmin && !filtered ? <Button variant="primary" onPress={() => setAdding(true)}><IconPlus size={16} /> Add show</Button> : undefined}
            />
          }
          renderCard={(row) => {
            const { have, total } = counts(row);
            return (
              <PosterCard
                title={row.name}
                meta={[row.year, row.network].filter(Boolean).join(" · ")}
                image={showPoster(row)}
                tint={LIBRARY_TYPES.tv.tintVar}
                to="/shows/$showId"
                params={{ showId: row.id }}
                badge={total > 0 ? <ProgressBadge have={have} total={total} className="text-foreground" /> : undefined}
              />
            );
          }}
        />
        <BrowserLetterRail letter={jump.active} onLetterChange={jump.jump} />
      </div>
      <ProviderSearchDialog kind="show" libraryId={libraryId} isOpen={adding} onOpenChange={setAdding} onAdded={() => void list.refetch()} />
    </div>
  );
}
