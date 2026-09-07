import { useNavigate } from "@tanstack/react-router";
import { IconMicrophone2 } from "@tabler/icons-react";
import { useMemo } from "react";

import { DataTable, EmptyState, ErrorState, PosterCard, type DataTableColumn } from "@/components/ui";
import { LibraryArtistsDocument, type ArtistOrderByInput, type ArtistWhereInput, type LibraryArtistsQuery } from "@/graphql/generated/graphql";
import { useServerTable } from "@/hooks/useServerTable";
import { artistImage } from "@/lib/artwork";
import { formatRuntime, pluralize } from "@/lib/format";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { BrowserLetterRail, BrowserToolbar } from "./BrowserToolbar";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";
import { useLetterJump } from "./useLetterJump";

type ArtistRow = LibraryArtistsQuery["artists"]["edges"][number]["node"];
const getRowId = (row: ArtistRow) => row.id;

export function ArtistsBrowser({ libraryId }: { libraryId: string }) {
  const navigate = useNavigate();
  const filters = useBrowserFilters(`artists.${libraryId}`);
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
    getTitle: (row) => row.name,
    hasMore: list.hasMore,
    loadMore: list.loadMore,
    ensureTitleSort: () => table.setSorting([{ id: "name", desc: false }]),
  });

  const columns = useMemo<Array<DataTableColumn<ArtistRow>>>(
    () => [
      {
        id: "name",
        header: "Artist",
        sortable: true,
        cell: (row) => (
          <span className="flex items-center gap-3">
            <img src={artistImage(row)} alt="" className="size-10 shrink-0 rounded-full object-cover" loading="lazy" />
            <span className="min-w-0">
              <span className="block truncate text-body-sm text-foreground">{row.name}</span>
              {row.disambiguation ? <span className="block truncate text-label-sm text-muted">{row.disambiguation}</span> : null}
            </span>
          </span>
        ),
      },
      { id: "albumCount", header: "Albums", size: 90, align: "end", numeric: true, sortable: true, cell: (row) => row.albumCount ?? 0 },
      { id: "trackCount", header: "Tracks", size: 90, align: "end", numeric: true, hideBelow: "sm", cell: (row) => row.trackCount ?? 0 },
      { id: "duration", header: "Length", size: 100, align: "end", numeric: true, hideBelow: "md", cell: (row) => formatRuntime(row.totalDurationSecs) },
    ],
    [],
  );

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      <BrowserToolbar query={filters.query} onQueryChange={filters.setQuery} letter={jump.active} onLetterChange={jump.jump} view={filters.view} onViewChange={filters.setView} total={list.totalCount} placeholder="Filter artists" />
      <div className="flex min-h-0 flex-1 gap-4">
        <DataTable<ArtistRow>
          className="min-w-0 flex-1"
          view={filters.view}
          columns={columns}
          rows={list.rows}
          getRowId={getRowId}
          isLoading={list.loading}
          totalCount={list.totalCount}
          noun="artists"
          sorting={table.sorting}
          onSortingChange={table.setSorting}
          infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyState={<EmptyState icon={IconMicrophone2} title="No artists yet" description="Artists are created when albums are added or scanned." />}
          onRowClick={(row) => void navigate({ to: "/artists/$artistId", params: { artistId: row.id } })}
          renderCard={(row) => (
            <PosterCard aspect="square" title={row.name} meta={pluralize(row.albumCount ?? 0, "album")} image={artistImage(row)} tint={LIBRARY_TYPES.music.tintVar} to="/artists/$artistId" params={{ artistId: row.id }} className="[&_img]:rounded-full [&>div>div]:rounded-full" />
          )}
        />
        <BrowserLetterRail letter={jump.active} onLetterChange={jump.jump} />
      </div>
    </div>
  );
}
