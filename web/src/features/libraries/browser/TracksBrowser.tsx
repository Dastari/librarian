import { IconMusic, IconPlayerPlayFilled } from "@tabler/icons-react";
import { useMemo } from "react";

import { DataTable, EmptyState, ErrorState, StatusChip, type DataTableColumn } from "@/components/ui";
import { usePlayer } from "@/features/player/usePlayer";
import type { PlayItem } from "@/features/player/store";
import { LibraryTracksDocument, type LibraryTracksQuery, type TrackOrderByInput, type TrackWhereInput } from "@/graphql/generated/graphql";
import { useContentStatuses } from "@/hooks/useContentStatuses";
import { useServerTable } from "@/hooks/useServerTable";
import { albumCover } from "@/lib/artwork";
import { formatBytes, formatClock } from "@/lib/format";
import { statusMeta } from "@/lib/status";

import { BrowserToolbar } from "./BrowserToolbar";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";

type TrackRow = LibraryTracksQuery["tracks"]["edges"][number]["node"];
const getRowId = (row: TrackRow) => row.id;

export function trackToPlayItem(row: { id: string; title: string; mediaFileId?: string | null; artistName?: string | null; albumId: string; album?: { id: string; name: string; coverUrl?: string | null } | null; durationSecs?: number | null }): PlayItem | null {
  if (!row.mediaFileId) return null;
  return {
    mediaFileId: row.mediaFileId,
    title: row.title,
    subtitle: [row.artistName, row.album?.name].filter(Boolean).join(" · ") || undefined,
    artwork: row.album ? albumCover(row.album) : undefined,
    entity: { kind: "track", id: row.id },
    href: `/albums/${row.albumId}`,
    duration: row.durationSecs ?? undefined,
  };
}

export function TracksBrowser({ libraryId }: { libraryId: string }) {
  const player = usePlayer();
  const filters = useBrowserFilters(`tracks.${libraryId}`, "table");
  const table = useServerTable<TrackOrderByInput>({ persistKey: `tracks.${libraryId}`, defaultSorting: [{ id: "title", desc: false }] });

  const where = useMemo<TrackWhereInput>(() => {
    const clause: TrackWhereInput = { libraryId: { eq: libraryId } };
    const title = titleFilter(filters.debouncedQuery, null);
    if (title) clause.title = title;
    if (filters.availability === "available") clause.mediaFileId = { isNull: false };
    if (filters.availability === "wanted") {
      clause.mediaFileId = { isNull: true };
      clause.wanted = { eq: true };
    }
    if (filters.availability === "missing") {
      clause.mediaFileId = { isNull: true };
      clause.wanted = { eq: false };
    }
    return clause;
  }, [libraryId, filters.debouncedQuery, filters.availability]);

  const list = useInfiniteConnection(LibraryTracksDocument, { where, orderBy: table.orderBy }, (data) => data.tracks);
  const ids = useMemo(() => list.rows.map((row) => row.id), [list.rows]);
  const statuses = useContentStatuses("TRACK", ids);

  const playFrom = (row: TrackRow) => {
    const playable = list.rows.map(trackToPlayItem).filter((item): item is PlayItem => item !== null);
    const index = playable.findIndex((item) => item.entity.id === row.id);
    if (index >= 0) player.playAudio(playable, index);
  };

  const columns = useMemo<Array<DataTableColumn<TrackRow>>>(
    () => [
      {
        id: "title",
        header: "Track",
        sortable: true,
        cell: (row) => (
          <span className="flex items-center gap-3">
            <button
              type="button"
              aria-label={`Play ${row.title}`}
              disabled={!row.mediaFileId}
              onClick={(event) => {
                event.stopPropagation();
                playFrom(row);
              }}
              className="nav-focus relative grid size-10 shrink-0 place-items-center overflow-hidden rounded-sm bg-surface-secondary disabled:opacity-60"
            >
              {row.album ? <img src={albumCover(row.album)} alt="" className="absolute inset-0 size-full object-cover" loading="lazy" /> : null}
              {row.mediaFileId ? <IconPlayerPlayFilled size={16} className="relative text-white drop-shadow" /> : null}
            </button>
            <span className="min-w-0">
              <span className="block truncate text-body-sm text-foreground">{row.title}</span>
              <span className="block truncate text-label-sm text-muted">{[row.artistName, row.album?.name].filter(Boolean).join(" · ")}</span>
            </span>
          </span>
        ),
      },
      { id: "trackNumber", header: "#", size: 70, align: "end", numeric: true, sortable: true, hideBelow: "sm", cell: (row) => (row.discNumber && row.discNumber > 1 ? `${row.discNumber}-${row.trackNumber}` : row.trackNumber) },
      { id: "status", header: "Status", size: 140, cell: (row) => <StatusChip status={statusMeta(statuses.get(row.id))} /> },
      { id: "durationSecs", header: "Length", size: 90, align: "end", numeric: true, sortable: true, cell: (row) => formatClock(row.durationSecs) },
      { id: "codec", header: "Format", size: 150, hideBelow: "md", cell: (row) => <span className="text-muted">{row.mediaFile ? [row.mediaFile.audioCodec?.toUpperCase(), formatBytes(row.mediaFile.size)].filter(Boolean).join(" · ") : "—"}</span> },
    ],
    [statuses, list.rows],
  );

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-4">
      <BrowserToolbar
        query={filters.query}
        onQueryChange={filters.setQuery}
        letter={null}
        onLetterChange={() => undefined}
        availability={filters.availability}
        onAvailabilityChange={filters.setAvailability}
        view="table"
        onViewChange={() => undefined}
        total={list.totalCount}
        placeholder="Filter tracks"
      />
      <DataTable<TrackRow>
        columns={columns}
        rows={list.rows}
        getRowId={getRowId}
        isLoading={list.loading}
        totalCount={list.totalCount}
        noun="tracks"
        density="compact"
        sorting={table.sorting}
        onSortingChange={table.setSorting}
        infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
        error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
        emptyState={<EmptyState icon={IconMusic} title="No tracks" description="Tracks appear once albums are added or scanned." />}
        onRowClick={(row) => row.mediaFileId && playFrom(row)}
      />
    </div>
  );
}
