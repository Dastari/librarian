import { createDataTableColumns, TextCell } from "glass-ux/data-table";
import { useNavigate } from "@tanstack/react-router";

import { IconPlus } from "glass-ux/icons";

import { useCallback, useMemo, useState } from "react";

import { Button, PosterCard } from "@/components/ui";
import { useAlbumEntity } from "@/features/media/useAlbumEntity";
import { useGridReset } from "@/features/media/useProgressOwners";
import { AlbumIdentity } from "@/features/media/AlbumIdentity";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { LibraryAlbumsDocument, type AlbumOrderByInput, type AlbumWhereInput, type LibraryAlbumsQuery } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { albumCover } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatBytes, formatYear } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { ProgressBadge } from "./ShowsBrowser";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { MediaBrowserTable } from "./MediaBrowserTable";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type AlbumRow = LibraryAlbumsQuery["albums"]["edges"][number]["node"];
const getRowId = (row: AlbumRow) => row.id;

const column = createDataTableColumns<AlbumRow>();
const columns = column.columns([
  column.accessor("name", { header: "Album", width: 30, priority: 1, cardRole: "title", sortable: true, cell: ({ row }) => <AlbumIdentity album={row.original} /> }),
  column.accessor("year", { header: "Year", width: 6, priority: 2, grow: false, sortable: true, sortDescFirst: true, cell: ({ row }) => <TextCell muted>{row.original.year ?? ""}</TextCell> }),
  column.display({ id: "type", header: "Type", width: 8, priority: 3, cell: ({ row }) => <TextCell muted>{row.original.albumType ?? ""}</TextCell> }),
  column.display({ id: "tracks", header: "Tracks", width: 8, priority: 2, align: "end", grow: false, cell: ({ row }) => <TextCell muted>{`${row.original.downloaded.pageInfo.totalCount ?? 0} of ${row.original.trackCount ?? 0}`}</TextCell> }),
  column.accessor("sizeBytes", { header: "Size", width: 7, priority: 3, align: "end", grow: false, sortable: true, sortDescFirst: true, cell: ({ row }) => <TextCell muted>{formatBytes(row.original.sizeBytes ?? 0)}</TextCell> }),
]);

export function AlbumsBrowser({ libraryId }: { libraryId: string }) {
  const isAdmin = useIsAdmin();
  const navigate = useNavigate();
  const filters = useBrowserFilters();
  const reset = useGridReset<AlbumRow>("album");
  const { actions, dialogs } = useAlbumEntity<AlbumRow>({ reset });
  const table = useServerTable<AlbumOrderByInput>({ persistKey: `albums.${libraryId}`, defaultSorting: [{ id: "name", desc: false }], sortMap: { name: "sortName" } });
  const [adding, setAdding] = useState(false);

  const where = useMemo<AlbumWhereInput>(() => {
    const clause: AlbumWhereInput = { libraryId: { eq: libraryId } };
    const name = titleFilter(filters.debouncedQuery, null);
    if (name) clause.name = name;
    if (filters.availability === "available") clause.hasFiles = { eq: true };
    if (filters.availability === "wanted" || filters.availability === "missing") clause.hasFiles = { eq: false };
    return clause;
  }, [libraryId, filters.debouncedQuery, filters.availability]);

  const list = useInfiniteConnection(LibraryAlbumsDocument, { where, orderBy: table.orderBy }, (data) => data.albums);
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

  const renderPoster = useCallback((row: AlbumRow) => (
      <PosterCard
      aspect="square"
      title={row.name}
      meta={[formatYear(row.releaseDate) || row.year, row.albumType].filter(Boolean).join(" · ")}
      image={albumCover(row)}
      tint={LIBRARY_TYPES.music.tintVar}
      to="/albums/$albumId"
      params={{ albumId: row.id }}
      badge={row.trackCount ? <ProgressBadge have={row.downloaded.pageInfo.totalCount ?? 0} total={row.trackCount} className="text-foreground" /> : undefined}
      />
  ), []);

  return (
    <>
    <MediaBrowserTable<AlbumRow>
      persistKey="albums"
      label="Albums"
      noun={{ one: "album", other: "albums" }}
      placeholder="Filter albums"
      columns={columns}
      list={list}
      sorting={table.sorting}
      onSortingChange={table.setSorting}
      query={filters.query}
      onQueryChange={filters.setQuery}
      availability={filters.availability}
      onAvailabilityChange={filters.setAvailability}
      getRowId={getRowId}
      getRowLabel={(row) => row.name}
      renderPoster={renderPoster}
      actions={actions}
      onRowOpen={(row) => void navigate({ to: "/albums/$albumId", params: { albumId: row.id } })}
      emptyMessage="No albums yet. Scan the library folder or add an album from MusicBrainz."
      emptyAction={isAdmin ? <Button variant="primary" onPress={() => setAdding(true)}><IconPlus size="md" /> Add album</Button> : undefined}
      trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size="md" /> Add</Button> : null}
      letter={{ active: jump.active, jump: jump.jump, containerRef: jump.containerRef }}
    />
      {dialogs}
      <ProviderSearchDialog kind="album" libraryId={libraryId} isOpen={adding} onOpenChange={setAdding} onAdded={() => void list.refetch()} />
    </>
  );
}
