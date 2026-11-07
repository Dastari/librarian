import { createDataTableColumns, RelativeTime, TextCell } from "glass-ux/data-table";
import { useNavigate } from "@tanstack/react-router";

import { IconPlus } from "glass-ux/icons";

import { useCallback, useMemo, useState } from "react";

import { Button, PosterCard } from "@/components/ui";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { LibraryCollectionsDocument, type CollectionOrderByInput, type CollectionWhereInput, type LibraryCollectionsQuery } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { collectionPoster } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { ProgressBadge } from "./ShowsBrowser";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { parseTimestamp } from "@/lib/format";
import { MediaBrowserTable } from "./MediaBrowserTable";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type CollectionRow = LibraryCollectionsQuery["collections"]["edges"][number]["node"];
const getRowId = (row: CollectionRow) => row.id;

const column = createDataTableColumns<CollectionRow>();
const columns = column.columns([
  column.accessor("name", { header: "Collection", width: 32, priority: 1, cardRole: "title", sortable: true, cell: ({ row }) => <TextCell>{row.original.name}</TextCell> }),
  column.accessor("movieCount", { header: "Movies", width: 9, priority: 2, align: "end", grow: false, sortable: true, sortDescFirst: true, cell: ({ row }) => <TextCell muted>{`${row.original.owned.pageInfo.totalCount ?? 0} of ${row.original.movieCount}`}</TextCell> }),
  column.accessor("lastSyncedAt", { header: "Synced", width: 8, priority: 3, sortable: true, sortDescFirst: true, cell: ({ row }) => (row.original.lastSyncedAt ? <RelativeTime value={parseTimestamp(row.original.lastSyncedAt)} /> : <TextCell muted>Never</TextCell>) }),
]);

export function CollectionsBrowser({ libraryId }: { libraryId: string }) {
  const isAdmin = useIsAdmin();
  const navigate = useNavigate();
  const filters = useBrowserFilters();
  const table = useServerTable<CollectionOrderByInput>({ persistKey: `collections.${libraryId}`, defaultSorting: [{ id: "name", desc: false }], sortMap: { name: "sortName" } });
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
    getTitle: (row) => row.sortName || row.name,
    hasMore: list.hasMore,
    loading: list.fetching || list.loadingMore,
    queryKey: JSON.stringify(where),
    isTitleSorted: table.sorting.length === 1 && table.sorting[0]?.id === "name" && !table.sorting[0].desc,
    loadMore: list.loadMore,
    ensureTitleSort: () => table.setSorting([{ id: "name", desc: false }]),
  });

  const renderPoster = useCallback((row: CollectionRow) => (
      <PosterCard
      title={row.name}
      meta={`${row.owned.pageInfo.totalCount ?? 0} of ${row.movieCount} movies`}
      image={collectionPoster(row)}
      tint={LIBRARY_TYPES.movies.tintVar}
      to="/collections/$collectionId"
      params={{ collectionId: row.id }}
      badge={<ProgressBadge have={row.owned.pageInfo.totalCount ?? 0} total={row.movieCount} className="text-foreground" />}
      />
  ), []);

  return (
    <>
    <MediaBrowserTable<CollectionRow>
      persistKey="collections"
      label="Collections"
      noun={{ one: "collection", other: "collections" }}
      placeholder="Filter collections"
      columns={columns}
      list={list}
      sorting={table.sorting}
      onSortingChange={table.setSorting}
      query={filters.query}
      onQueryChange={filters.setQuery}
      getRowId={getRowId}
      getRowLabel={(row) => row.name}
      renderPoster={renderPoster}
      onRowOpen={(row) => void navigate({ to: "/collections/$collectionId", params: { collectionId: row.id } })}
      emptyMessage="No collections yet."
      emptyAction={isAdmin ? <Button variant="primary" onPress={() => setAdding(true)}><IconPlus size="md" /> Add collection</Button> : undefined}
      trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size="md" /> Add</Button> : null}
      letter={{ active: jump.active, jump: jump.jump, containerRef: jump.containerRef }}
    />
      <ProviderSearchDialog kind="collection" libraryId={libraryId} isOpen={adding} onOpenChange={setAdding} onAdded={() => void list.refetch()} />
    </>
  );
}
