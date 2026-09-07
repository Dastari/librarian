
import { useNavigate } from "@tanstack/react-router";
import { IconMusic, IconPlus } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Button, DataTable, type DataTableColumn, EmptyState, ErrorState, PosterCard } from "@/components/ui";
import { ProviderSearchDialog } from "@/features/library-add/ProviderSearchDialog";
import { LibraryAlbumsDocument, type AlbumOrderByInput, type AlbumWhereInput, type LibraryAlbumsQuery } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { albumCover } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatBytes, formatRuntime, formatYear } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { BrowserLetterRail, BrowserToolbar } from "./BrowserToolbar";
import { ProgressBadge } from "./ShowsBrowser";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type AlbumRow = LibraryAlbumsQuery["albums"]["edges"][number]["node"];
const getRowId = (row: AlbumRow) => row.id;

export function AlbumsBrowser({ libraryId }: { libraryId: string }) {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const filters = useBrowserFilters(`albums.${libraryId}`);
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
    getTitle: (row) => row.name,
    hasMore: list.hasMore,
    loadMore: list.loadMore,
    ensureTitleSort: () => table.setSorting([{ id: "name", desc: false }]),
  });

  const columns = useMemo<Array<DataTableColumn<AlbumRow>>>(
    () => [
      {
        id: "name",
        header: "Album",
        sortable: true,
        cell: (row) => (
          <span className="flex items-center gap-3">
            <img src={albumCover(row)} alt="" className="size-10 shrink-0 rounded-sm object-cover" loading="lazy" />
            <span className="min-w-0">
              <span className="block truncate text-body-sm text-foreground">{row.name}</span>
              <span className="block truncate text-label-sm text-muted">{[formatYear(row.releaseDate) || row.year, row.albumType, row.label].filter(Boolean).join(" · ")}</span>
            </span>
          </span>
        ),
      },
      { id: "tracks", header: "Tracks", size: 100, align: "end", cell: (row) => <ProgressBadge have={row.downloaded.pageInfo.totalCount ?? 0} total={row.trackCount ?? 0} /> },
      { id: "duration", header: "Length", size: 100, align: "end", numeric: true, hideBelow: "md", cell: (row) => formatRuntime(row.totalDurationSecs) },
      { id: "sizeBytes", header: "Size", size: 100, align: "end", numeric: true, sortable: true, hideBelow: "lg", cell: (row) => formatBytes(row.sizeBytes) },
      { id: "year", header: "Year", size: 80, align: "end", numeric: true, sortable: true, hideBelow: "sm", cell: (row) => row.year ?? "—" },
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
        availability={filters.availability}
        onAvailabilityChange={filters.setAvailability}
        view={filters.view}
        onViewChange={filters.setView}
        total={list.totalCount}
        placeholder="Filter albums"
        trailing={isAdmin ? <Button variant="primary" size="sm" onPress={() => setAdding(true)}><IconPlus size={16} /> Add</Button> : null}
      />
      <div className="flex min-h-0 flex-1 gap-4">
        <DataTable<AlbumRow>
          className="min-w-0 flex-1"
          view={filters.view}
          columns={columns}
          rows={list.rows}
          getRowId={getRowId}
          isLoading={list.loading}
          totalCount={list.totalCount}
          noun="albums"
          sorting={table.sorting}
          onSortingChange={table.setSorting}
          infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyState={<EmptyState icon={IconMusic} title="No albums yet" description="Scan the library folder or add an album from MusicBrainz." action={isAdmin ? <Button variant="primary" onPress={() => setAdding(true)}><IconPlus size={16} /> Add album</Button> : undefined} />}
          onRowClick={(row) => void navigate({ to: "/albums/$albumId", params: { albumId: row.id } })}
          renderCard={(row) => (
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
          )}
        />
        <BrowserLetterRail letter={jump.active} onLetterChange={jump.jump} />
      </div>
      <ProviderSearchDialog kind="album" libraryId={libraryId} isOpen={adding} onOpenChange={setAdding} onAdded={() => void list.refetch()} />
    </div>
  );
}
