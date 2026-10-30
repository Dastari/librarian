import { useLiveQuery } from "@/lib/graphql/live/hooks";
import { useSession } from "@/lib/auth/useSession";
import { useState } from "react";
import { Button as GlassButton } from "glass-ux/button";
import { IconPlay } from "glass-ux/icons";
import { SaveToListDialog } from "@/features/personal/SaveToListDialog";
import { IconMusic } from "@tabler/icons-react";
import { createContext, useContext, useMemo } from "react";

import { DataTable, EmptyState, ErrorState, StatusChip, type DataTableColumn } from "@/components/ui";
import { usePlayer } from "@/features/player/usePlayer";
import type { PlayItem } from "@/features/player/store";
import { EntityLibraryGetDocument, LibraryTracksDocument, type LibraryTracksQuery, type TrackOrderByInput, type TrackWhereInput, type ContentStatus } from "@/graphql/generated/graphql";
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
const TrackCellsContext = createContext<{ statuses: Map<string, ContentStatus>; playFrom: (row: TrackRow) => void } | null>(null);
function TrackStatusCell({ row }: { row: TrackRow }) {
  return <StatusChip status={statusMeta(useContext(TrackCellsContext)!.statuses.get(row.id))} />;
}


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
  const { user } = useSession();
  const ownership = useLiveQuery(EntityLibraryGetDocument, { variables: { id: libraryId } });
  const ownLibrary = ownership.data?.library?.userId === user?.id && Boolean(user);
  const [savingTrack, setSavingTrack] = useState<{ id: string; title: string } | null>(null);
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
        cell: (row) => <TrackIdentityCell row={row} />,
      },
      { id: "trackNumber", header: "#", size: 70, align: "end", numeric: true, sortable: true, hideBelow: "sm", cell: (row) => (row.discNumber && row.discNumber > 1 ? `${row.discNumber}-${row.trackNumber}` : row.trackNumber) },
      { id: "status", header: "Status", size: 140, cell: (row) => <TrackStatusCell row={row} /> },
      { id: "durationSecs", header: "Length", size: 90, align: "end", numeric: true, sortable: true, cell: (row) => formatClock(row.durationSecs) },
      { id: "codec", header: "Format", size: 150, hideBelow: "md", cell: (row) => <span className="text-muted">{row.mediaFile ? [row.mediaFile.audioCodec?.toUpperCase(), formatBytes(row.mediaFile.size)].filter(Boolean).join(" · ") : "—"}</span> },
    ],
    [],
  );

  return (
    <TrackCellsContext value={{ statuses, playFrom }}>
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 sm:gap-4">
      <BrowserToolbar
        tableOnly
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
        height="fill"
        className="min-h-0 min-w-0 flex-1"
        columns={columns}
        rows={list.rows}
        getRowId={getRowId}
        isLoading={list.loading}
          refetching={list.fetching}
          queryKey={list.queryKey} loadRevision={list.loadRevision}
          externalBrowserToolbar
        totalCount={list.totalCount}
        noun="tracks"
        density="compact"
        sorting={table.sorting}
        onSortingChange={table.setSorting}
        infinite={{ hasMore: list.hasMore, loadMore: list.loadMore, loadingMore: list.loadingMore }}
        error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
        emptyState={<EmptyState icon={IconMusic} title="No tracks" description="Tracks appear once albums are added or scanned." />}
        rowActions={ownLibrary ? [{ key: "save", label: "Save to list", onAction: (row) => setSavingTrack(row) }] : []}
        onRowClick={(row) => row.mediaFileId && playFrom(row)}
      />
    </div>
    {savingTrack ? <SaveToListDialog open onOpenChange={(open) => { if (!open) setSavingTrack(null); }} target={{ kind: "track", id: savingTrack.id }} title={savingTrack.title} /> : null}
    </TrackCellsContext>
  );
}

function TrackIdentityCell({ row }: { row: TrackRow }) {
  const { playFrom } = useContext(TrackCellsContext)!;
  return (
          <span className="flex items-center gap-3">
            <GlassButton
              variant="ghost" shape="circle" icon={<IconPlay />}
              aria-label={`Play ${row.title}`} disabled={!row.mediaFileId}
              onClick={(event) => { event.stopPropagation(); playFrom(row); }}
            />
            <span className="min-w-0">
              <span className="block truncate text-body-sm text-foreground">{row.title}</span>
              <span className="block truncate text-label-sm text-muted">{[row.artistName, row.album?.name].filter(Boolean).join(" · ")}</span>
            </span>
          </span>
  );
}
