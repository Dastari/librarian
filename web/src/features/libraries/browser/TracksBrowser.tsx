import { defineActions } from "glass-ux/actions";
import { Badge } from "glass-ux/badge";
import { createDataTableColumns, DataTable, IdentityCell, TextCell, type DataTableSearch, type DataTableState, type DataTableStateChange } from "glass-ux/data-table";
import { IconExternalLink, IconPlay, IconPlus } from "glass-ux/icons";
import { createContext, useContext, useMemo, useState } from "react";

import { TableToolbar } from "@/components/ui/TableToolbar";
import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { useTableSelection } from "@/hooks/useTableSelection";
import { ArtGlyph } from "@/components/entities/ArtGlyph";
import { EntityChip } from "@/components/entities/EntityChip";
import { refCellEntity } from "@/lib/entities/resolver";
import { ErrorState, GlassSegmented } from "@/components/ui";
import { SaveToListDialog } from "@/features/personal/SaveToListDialog";
import { trackPlayItem } from "@/features/player/playItems";
import type { PlayItem } from "@/features/player/store";
import { useQueueActions } from "@/features/player/queueActions";
import { usePlayer } from "@/features/player/usePlayer";
import { EntityLibraryGetDocument, LibraryTracksDocument, type ContentStatus, type LibraryTracksQuery, type TrackOrderByInput, type TrackWhereInput } from "@/graphql/generated/graphql";
import { useContentStatuses, contentStatusMeta } from "@/hooks/useContentStatuses";
import { useServerTable } from "@/hooks/useServerTable";
import { useSession } from "@/lib/auth/useSession";
import { sizedArtwork } from "@/lib/api/urls";
import { albumCover } from "@/lib/artwork";
import { ENTITY_KINDS, entityPath } from "@/lib/entities/kinds";
import { badgeTone } from "@/lib/entities/linked";
import { formatBytes, formatClock } from "@/lib/format";
import { useLiveQuery } from "@/lib/graphql/live/hooks";

import type { AvailabilityFilter } from "./BrowserToolbar";
import { useBrowserFilters, titleFilter } from "./useBrowserFilters";
import { useInfiniteConnection } from "./useInfiniteConnection";

type TrackRow = LibraryTracksQuery["tracks"]["edges"][number]["node"];
const getRowId = (row: TrackRow) => row.id;
const TrackGlyph = ENTITY_KINDS.track.icon;

const StatusContext = createContext<Map<string, ContentStatus>>(new Map());
function TrackStatusCell({ row }: { row: TrackRow }) {
  const statuses = useContext(StatusContext);
  const meta = contentStatusMeta(statuses, row.id);
  return <Badge size="sm" marker="dot" tone={badgeTone(meta.tone)}>{meta.label}</Badge>;
}
function columnsFor() {
  const column = createDataTableColumns<TrackRow>();
  return column.columns([
    column.accessor("title", {
      header: "Track", width: 26, priority: 1, cardRole: "title", sortable: true,
      cell: ({ row }) => <IdentityCell icon={<ArtGlyph src={row.original.album ? sizedArtwork(albumCover(row.original.album), 96) : null} fallback={<TrackGlyph />} />} title={row.original.title} subtitle={row.original.artistName ?? undefined} status={row.original.mediaFileId ? "online" : "offline"} statusLabel={row.original.mediaFileId ? "Playable" : "No file"} />,
    }),
    column.accessor("trackNumber", { header: "#", width: 4, priority: 3, align: "end", grow: false, sortable: true, cell: ({ row }) => <TextCell muted>{row.original.discNumber && row.original.discNumber > 1 ? `${row.original.discNumber}-${row.original.trackNumber}` : String(row.original.trackNumber ?? "")}</TextCell> }),
    column.display({ id: "album", header: "Album", width: 16, priority: 2, cardRole: "subtitle", entity: (row) => refCellEntity(row.album ? { kind: "album", id: row.album.id, label: row.album.name, href: entityPath("album", row.album.id) } : null), cell: ({ row }) => (row.original.album ? <EntityChip entity={{ kind: "album", id: row.original.album.id, label: row.original.album.name, href: entityPath("album", row.original.album.id) }} /> : <TextCell muted>—</TextCell>) }),
    column.display({ id: "status", header: "Status", width: 9, priority: 2, cardRole: "status", cell: ({ row }) => <TrackStatusCell row={row.original} /> }),
    column.accessor("durationSecs", { header: "Length", width: 6, priority: 2, align: "end", grow: false, sortable: true, cell: ({ row }) => <TextCell muted>{formatClock(row.original.durationSecs)}</TextCell> }),
    column.display({ id: "codec", header: "Format", width: 10, priority: 4, cell: ({ row }) => <TextCell muted>{row.original.mediaFile ? [row.original.mediaFile.audioCodec?.toUpperCase(), formatBytes(row.original.mediaFile.size)].filter(Boolean).join(" · ") : "—"}</TextCell> }),
  ]);
}

export const trackToPlayItem = trackPlayItem;

const search: DataTableSearch<TrackRow> = { fields: [], placeholder: "Filter tracks", toCondition: (query) => ({ title: { contains: query } }) };

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

  const columns = useMemo(() => columnsFor(), []);
  const selection = useTableSelection({ scope: [user?.id, libraryId, where, table.sorting], rows: list.rows, ready: !list.fetching });
  const state = useMemo<DataTableState>(() => ({ pagination: { pageIndex: 0, pageSize: 50 }, selection: selection.selection, filters: [], sorting: table.sorting, search: filters.query }), [table.sorting, selection.selection, filters.query]);
  const onStateChange = (next: DataTableState, change: DataTableStateChange) => { if (change === "search") filters.setQuery(next.search ?? ""); else if (change === "sorting") table.setSorting(next.sorting); else if (change === "selection") selection.onSelectionChange(next.selection); };
  const selectedQueue = (rows: readonly TrackRow[]) => {
    const ids = new Set(rows.map((row) => row.id));
    return list.rows.filter((row) => ids.has(row.id)).map(trackToPlayItem).filter((item): item is PlayItem => item !== null);
  };
  // Play next / Add to queue (the play queue): the selected tracks, in table order.
  const queueActions = useQueueActions<TrackRow>((rows) => [selectedQueue(rows)], { disabledReason: (rows) => rows.some((row) => !row.mediaFileId) ? "Some selected tracks have no file. Deselect them to queue." : false });
  const actions = useMemo(() => defineActions<TrackRow>("track", [
    { id: "play", label: (rows) => rows.length === 1 ? "Play from here" : `Play ${rows.length} selected tracks`, icon: IconPlay, group: "open", priority: 1, bulk: true,
      disabledReason: (rows) => rows.some((row) => !row.mediaFileId) ? "Some selected tracks have no file. Deselect them to play." : false,
      run: (rows) => { if (rows.length === 1) playFrom(rows[0]!); else player.playAudio(selectedQueue(rows)); } },
    ...queueActions,
    { id: "open-album", bulk: true, label: "Open album", icon: IconExternalLink, group: "open", priority: 2, visible: (rows) => rows.length === 1, run: (rows, context) => context.navigate(`/albums/${rows[0]!.albumId}`) },
    { id: "save", bulk: true, label: "Save to list…", icon: IconPlus, group: "organise", priority: 2, visible: (rows) => ownLibrary && rows.length === 1, run: (rows) => setSavingTrack(rows[0]!) },
  ]), [ownLibrary, playFrom, player, queueActions, list.rows]);

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 sm:gap-4">
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <StatusContext.Provider value={statuses}>
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
          selectable
          renderBulkActions={(rows) => <TableBulkActions set={actions} rows={rows} noun={{ one: "track", other: "tracks" }} />}
          noun={{ one: "track", other: "tracks" }}
          search={search}
          toolbar={
            <TableToolbar filter={<GlassSegmented<AvailabilityFilter>
              ariaLabel="Availability"
              size="sm"
              value={filters.availability}
              onChange={filters.setAvailability}
              items={[{ key: "all", label: "All" }, { key: "available", label: "Available" }, { key: "wanted", label: "Wanted" }, { key: "missing", label: "Missing" }]}
            />} />
          }
          onRowOpen={(row) => { if (row.mediaFileId) playFrom(row); }}
          rowOpen="click"
          error={list.error && list.rows.length === 0 ? <ErrorState error={list.error} onRetry={() => void list.refetch()} /> : undefined}
          emptyMessage="No tracks. Tracks appear once albums are added or scanned."
        />
        </StatusContext.Provider>
      </div>
      {savingTrack ? <SaveToListDialog open onOpenChange={(open) => { if (!open) setSavingTrack(null); }} target={{ kind: "track", id: savingTrack.id }} title={savingTrack.title} /> : null}
    </div>
  );
}
