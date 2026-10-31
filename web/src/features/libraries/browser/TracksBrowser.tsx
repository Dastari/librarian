import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, IdentityCell, TextCell, type DataTableState, type DataTableStateChange } from "glass-ux/data-table";
import { IconExternalLink, IconPlay, IconPlus } from "glass-ux/icons";
import { useMemo, useState } from "react";

import { EntityChip } from "@/components/entities/EntityChip";
import { ErrorState } from "@/components/ui";
import { SaveToListDialog } from "@/features/personal/SaveToListDialog";
import type { PlayItem } from "@/features/player/store";
import { usePlayer } from "@/features/player/usePlayer";
import { EntityLibraryGetDocument, LibraryTracksDocument, type ContentStatus, type LibraryTracksQuery, type TrackOrderByInput, type TrackWhereInput } from "@/graphql/generated/graphql";
import { useContentStatuses } from "@/hooks/useContentStatuses";
import { useServerTable } from "@/hooks/useServerTable";
import { useSession } from "@/lib/auth/useSession";
import { albumCover } from "@/lib/artwork";
import { ENTITY_KINDS, entityPath } from "@/lib/entities/kinds";
import { badgeTone } from "@/lib/entities/linked";
import { formatBytes, formatClock } from "@/lib/format";
import { useLiveQuery } from "@/lib/graphql/live/hooks";
import { statusMeta } from "@/lib/status";

import { BrowserToolbar } from "./BrowserToolbar";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";

type TrackRow = LibraryTracksQuery["tracks"]["edges"][number]["node"];
const getRowId = (row: TrackRow) => row.id;
const TrackGlyph = ENTITY_KINDS.track.icon;

function columnsFor(statuses: Map<string, ContentStatus>) {
  const column = createDataTableColumns<TrackRow>();
  return column.columns([
    column.accessor("title", {
      header: "Track", width: 26, priority: 1, cardRole: "title", sortable: true,
      cell: ({ row }) => <IdentityCell icon={<TrackGlyph />} title={row.original.title} subtitle={row.original.artistName ?? undefined} status={row.original.mediaFileId ? "online" : "offline"} statusLabel={row.original.mediaFileId ? "Playable" : "No file"} />,
    }),
    column.accessor("trackNumber", { header: "#", width: 4, priority: 3, align: "end", grow: false, sortable: true, cell: ({ row }) => <TextCell muted>{row.original.discNumber && row.original.discNumber > 1 ? `${row.original.discNumber}-${row.original.trackNumber}` : String(row.original.trackNumber ?? "")}</TextCell> }),
    column.display({ id: "album", header: "Album", width: 16, priority: 2, cardRole: "subtitle", cell: ({ row }) => (row.original.album ? <EntityChip entity={{ kind: "album", id: row.original.album.id, label: row.original.album.name, href: entityPath("album", row.original.album.id) }} /> : <TextCell muted>—</TextCell>) }),
    column.display({ id: "status", header: "Status", width: 9, priority: 2, cardRole: "status", cell: ({ row }) => { const meta = statusMeta(statuses.get(row.original.id)); return <Badge size="sm" marker="dot" tone={badgeTone(meta.tone)}>{meta.label}</Badge>; } }),
    column.accessor("durationSecs", { header: "Length", width: 6, priority: 2, align: "end", grow: false, sortable: true, cell: ({ row }) => <TextCell muted>{formatClock(row.original.durationSecs)}</TextCell> }),
    column.display({ id: "codec", header: "Format", width: 10, priority: 4, cell: ({ row }) => <TextCell muted>{row.original.mediaFile ? [row.original.mediaFile.audioCodec?.toUpperCase(), formatBytes(row.original.mediaFile.size)].filter(Boolean).join(" · ") : "—"}</TextCell> }),
  ]);
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
  const filters = useBrowserFilters();
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

  const columns = useMemo(() => columnsFor(statuses), [statuses]);
  const state = useMemo<DataTableState>(() => ({ pagination: { pageIndex: 0, pageSize: 50 }, selection: {}, filters: [], sorting: table.sorting }), [table.sorting]);
  const onStateChange = (next: DataTableState, change: DataTableStateChange) => { if (change === "sorting") table.setSorting(next.sorting); };
  const actions = useMemo(() => defineActions<TrackRow>("track", [
    { id: "play", label: "Play from here", icon: IconPlay, group: "open", priority: 1, visible: (rows) => rows.length === 1 && Boolean(rows[0]!.mediaFileId), run: (rows) => playFrom(rows[0]!) },
    { id: "open-album", label: "Open album", icon: IconExternalLink, group: "open", priority: 2, visible: (rows) => rows.length === 1, run: (rows, context) => context.navigate(`/albums/${rows[0]!.albumId}`) },
    { id: "save", label: "Save to list…", icon: IconPlus, group: "organise", priority: 2, visible: (rows) => ownLibrary && rows.length === 1, run: (rows) => setSavingTrack(rows[0]!) },
  ]), [ownLibrary, playFrom]);

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 sm:gap-4">
      <BrowserToolbar
        tableOnly
        query={filters.query}
        onQueryChange={filters.setQuery}
        letter={null}
        onLetterChange={() => undefined}
        availability={filters.availability}
        onAvailabilityChange={filters.setAvailability}
        total={list.totalCount}
        placeholder="Filter tracks"
      />
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <DataTable<TrackRow>
          id={`librarian.${user?.id ?? "session"}.tracks`}
          label="Tracks"
          queryKey={list.queryKey}
          height="fill"
          columns={columns}
          rows={list.rows}
          getRowId={getRowId}
          getRowLabel={(row) => row.title}
          loading={list.loading}
          refetching={list.fetching && !list.loading}
          rowCount={list.totalCount}
          state={state}
          onStateChange={onStateChange}
          infinite={{ revision: list.loadRevision, hasMore: list.hasMore, onLoadMore: list.loadMore, loadingMore: list.loadingMore }}
          actions={actions}
          density="compact"
          noun={{ one: "track", other: "tracks" }}
          onRowOpen={(row) => { if (row.mediaFileId) playFrom(row); }}
          rowOpen="click"
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyMessage="No tracks. Tracks appear once albums are added or scanned."
        />
      </div>
      {savingTrack ? <SaveToListDialog open onOpenChange={(open) => { if (!open) setSavingTrack(null); }} target={{ kind: "track", id: savingTrack.id }} title={savingTrack.title} /> : null}
    </div>
  );
}
