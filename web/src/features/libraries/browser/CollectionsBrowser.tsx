
import { IconPlus, IconStack3 } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Button, EmptyState, ErrorState, PosterCard, PosterGallery } from "@/components/ui";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { LibraryCollectionsDocument, type CollectionOrderByInput, type CollectionWhereInput, type LibraryCollectionsQuery } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { collectionPoster } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { BrowserLetterRail, BrowserToolbar, type BrowserSortOption } from "./BrowserToolbar";
import { ProgressBadge } from "./ShowsBrowser";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type CollectionRow = LibraryCollectionsQuery["collections"]["edges"][number]["node"];
const getRowId = (row: CollectionRow) => row.id;
const SORTS: readonly BrowserSortOption[] = [
  { id: "name", desc: false, label: "Name" },
  { id: "movieCount", desc: true, label: "Most movies" },
  { id: "lastSyncedAt", desc: true, label: "Recently synced" },
];

export function CollectionsBrowser({ libraryId }: { libraryId: string }) {
  const isAdmin = useIsAdmin();
  const filters = useBrowserFilters();
  const table = useServerTable<CollectionOrderByInput>({ persistKey: `collections.${libraryId}`, defaultSorting: [{ id: "name", desc: false }] });
  const [adding, setAdding] = useState(false);

  const where = useMemo<CollectionWhereInput>(() => {
    const clause: CollectionWhereInput = { libraryId: { eq: libraryId } };
    const name = titleFilter(filters.debouncedQuery, null);
    if (name) clause.name = name;
    return clause;
  }, [libraryId, filters.debouncedQuery]);

  const list = useInfiniteConnection(LibraryCollectionsDocument, { where, orderBy: table.orderBy }, (data) => data.collections);
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

  return (
    <div ref={jump.containerRef} className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 sm:gap-4">
      <BrowserToolbar
        query={filters.query}
        onQueryChange={filters.setQuery}
        letter={jump.active}
        onLetterChange={jump.jump} sort={{ options: SORTS, value: table.sorting, onChange: table.setSorting }}
        total={list.totalCount}
        placeholder="Filter collections"
        trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size={16} /> Add</Button> : null}
      />
      <div className="flex min-h-0 min-w-0 flex-1 gap-3 sm:gap-4">
        <PosterGallery<CollectionRow>
          label="Collections"
          rows={list.rows}
          getRowId={getRowId}
          loading={list.loading}
          refetching={list.fetching}
          queryKey={list.queryKey}
          totalCount={list.totalCount}
          noun="collections"
          infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyState={<EmptyState icon={IconStack3} title="No collections" description="Collections appear when movies that belong to a TMDB collection are added." />}
          renderCard={(row) => (
            <PosterCard
              title={row.name}
              meta={`${row.owned.pageInfo.totalCount ?? 0} of ${row.movieCount} movies`}
              image={collectionPoster(row)}
              tint={LIBRARY_TYPES.movies.tintVar}
              to="/collections/$collectionId"
              params={{ collectionId: row.id }}
              badge={<ProgressBadge have={row.owned.pageInfo.totalCount ?? 0} total={row.movieCount} className="text-foreground" />}
            />
          )}
        />
        <BrowserLetterRail letter={jump.active} onLetterChange={jump.jump} />
      </div>
      <ProviderSearchDialog kind="collection" libraryId={libraryId} isOpen={adding} onOpenChange={setAdding} onAdded={() => void list.refetch()} />
    </div>
  );
}
