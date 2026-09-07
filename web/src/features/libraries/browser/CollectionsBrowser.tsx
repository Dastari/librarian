
import { useNavigate } from "@tanstack/react-router";
import { IconPlus, IconStack3 } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Button, DataTable, type DataTableColumn, EmptyState, ErrorState, PosterCard } from "@/components/ui";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { LibraryCollectionsDocument, type CollectionOrderByInput, type CollectionWhereInput, type LibraryCollectionsQuery } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { collectionPoster } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatRelative } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { BrowserLetterRail, BrowserToolbar } from "./BrowserToolbar";
import { ProgressBadge } from "./ShowsBrowser";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type CollectionRow = LibraryCollectionsQuery["collections"]["edges"][number]["node"];
const getRowId = (row: CollectionRow) => row.id;

export function CollectionsBrowser({ libraryId }: { libraryId: string }) {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const filters = useBrowserFilters(`collections.${libraryId}`);
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
    loadMore: list.loadMore,
    ensureTitleSort: () => table.setSorting([{ id: "name", desc: false }]),
  });

  const columns = useMemo<Array<DataTableColumn<CollectionRow>>>(
    () => [
      {
        id: "name",
        header: "Collection",
        sortable: true,
        cell: (row) => (
          <span className="flex items-center gap-3">
            <img src={collectionPoster(row)} alt="" className="h-12 w-8 shrink-0 rounded-sm object-cover" loading="lazy" />
            <span className="min-w-0">
              <span className="block truncate text-body-sm text-foreground">{row.name}</span>
              {row.overview ? <span className="block truncate text-label-sm text-muted">{row.overview}</span> : null}
            </span>
          </span>
        ),
      },
      { id: "movieCount", header: "Movies", size: 100, align: "end", sortable: true, cell: (row) => <ProgressBadge have={row.owned.pageInfo.totalCount ?? 0} total={row.movieCount} /> },
      { id: "lastSyncedAt", header: "Synced", size: 140, sortable: true, hideBelow: "md", cell: (row) => <span className="text-muted">{row.lastSyncedAt ? formatRelative(row.lastSyncedAt) : "—"}</span> },
    ],
    [],
  );

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      <BrowserToolbar
        query={filters.query}
        onQueryChange={filters.setQuery}
        letter={jump.active}
        onLetterChange={jump.jump}
        view={filters.view}
        onViewChange={filters.setView}
        total={list.totalCount}
        placeholder="Filter collections"
        trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size={16} /> Add</Button> : null}
      />
      <div className="flex min-h-0 flex-1 gap-4">
        <DataTable<CollectionRow>
          className="min-w-0 flex-1"
          view={filters.view}
          columns={columns}
          rows={list.rows}
          getRowId={getRowId}
          isLoading={list.loading}
          totalCount={list.totalCount}
          noun="collections"
          sorting={table.sorting}
          onSortingChange={table.setSorting}
          infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyState={<EmptyState icon={IconStack3} title="No collections" description="Collections appear when movies that belong to a TMDB collection are added." />}
          onRowClick={(row) => void navigate({ to: "/collections/$collectionId", params: { collectionId: row.id } })}
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
