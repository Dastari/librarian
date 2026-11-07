import { createDataTableColumns, TextCell } from "glass-ux/data-table";
import { useNavigate } from "@tanstack/react-router";

import { useCallback, useMemo } from "react";

import { PosterCard } from "@/components/ui";
import { LibraryArtistsDocument, type ArtistOrderByInput, type ArtistWhereInput, type LibraryArtistsQuery } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { artistImage } from "@/lib/artwork";
import { pluralize } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { MediaBrowserTable } from "./MediaBrowserTable";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type ArtistRow = LibraryArtistsQuery["artists"]["edges"][number]["node"];
const getRowId = (row: ArtistRow) => row.id;

const column = createDataTableColumns<ArtistRow>();
const columns = column.columns([
  column.accessor("name", { header: "Artist", width: 32, priority: 1, cardRole: "title", sortable: true, cell: ({ row }) => <TextCell>{row.original.name}</TextCell> }),
  column.accessor("albumCount", { header: "Albums", width: 7, priority: 2, align: "end", grow: false, sortable: true, sortDescFirst: true, cell: ({ row }) => <TextCell muted>{row.original.albumCount ?? 0}</TextCell> }),
]);

export function ArtistsBrowser({ libraryId }: { libraryId: string }) {
  const navigate = useNavigate();
  const filters = useBrowserFilters();
  const table = useServerTable<ArtistOrderByInput>({ persistKey: `artists.${libraryId}`, defaultSorting: [{ id: "name", desc: false }], sortMap: { name: "sortName" } });

  const where = useMemo<ArtistWhereInput>(() => {
    const clause: ArtistWhereInput = { libraryId: { eq: libraryId } };
    const name = titleFilter(filters.debouncedQuery, null);
    if (name) clause.name = name;
    return clause;
  }, [libraryId, filters.debouncedQuery]);

  const list = useInfiniteConnection(LibraryArtistsDocument, { where, orderBy: table.orderBy }, (data) => data.artists);
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

  const renderPoster = useCallback((row: ArtistRow) => (
      <PosterCard aspect="square" title={row.name} meta={pluralize(row.albumCount ?? 0, "album")} image={artistImage(row)} tint={LIBRARY_TYPES.music.tintVar} to="/artists/$artistId" params={{ artistId: row.id }} className="[&_img]:rounded-full [&>div>div]:rounded-full" />
  ), []);

  return (
    <MediaBrowserTable<ArtistRow>
      persistKey="artists"
      label="Artists"
      noun={{ one: "artist", other: "artists" }}
      placeholder="Filter artists"
      columns={columns}
      list={list}
      sorting={table.sorting}
      onSortingChange={table.setSorting}
      query={filters.query}
      onQueryChange={filters.setQuery}
      getRowId={getRowId}
      getRowLabel={(row) => row.name}
      renderPoster={renderPoster}
      onRowOpen={(row) => void navigate({ to: "/artists/$artistId", params: { artistId: row.id } })}
      emptyMessage="No artists yet."
      letter={{ active: jump.active, jump: jump.jump, containerRef: jump.containerRef }}
    />
  );
}
