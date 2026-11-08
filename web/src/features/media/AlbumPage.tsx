import { copyText } from "glass-ux/copy-button";
import { useApolloClient, useMutation } from "@apollo/client/react";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { Link } from "@tanstack/react-router";
import { defineActions, type ActionSet } from "glass-ux/actions";
import { Button } from "glass-ux/button";
import { createDataTableColumns, filters, NumericCell, TextCell, TwoLineCell } from "glass-ux/data-table";
import { IconBookmark, IconCopy, IconInfo, IconPlay, IconPlus } from "glass-ux/icons";
import { createContext, useCallback, useContext, useMemo } from "react";

import { EmptyState, ErrorState, MetaLine, SkeletonHero, StatusChip } from "@/components/ui";
import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { TableBulkActions } from "@/components/ui/TableBulkActions";
import { AcquisitionChip } from "@/features/acquisition/AcquisitionChip";
import { autoDownloadMeta } from "@/features/acquisition/mode";
import { trackToPlayItem } from "@/features/libraries/browser/TracksBrowser";
import { usePlayer } from "@/features/player/usePlayer";
import { playerStore, type PlayItem } from "@/features/player/store";
import { AlbumDetailDocument, AlbumTracksPageDocument, EntityTrackUpdateDocument, EntityTrackUpdateManyDocument, type AlbumDetailQuery } from "@/graphql/generated/graphql";
import { useContentStatuses, contentStatusMeta } from "@/hooks/useContentStatuses";
import { albumCover } from "@/lib/artwork";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { formatClock, formatRuntime, formatYear } from "@/lib/format";
import { activeContentStatusQueries } from "@/lib/graphql/content-status";
import { assertSuccess } from "@/lib/graphql/errors";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { statusMeta } from "@/lib/status";

import { MediaActions } from "./MediaActions";
import { useAlbumEntity } from "./useAlbumEntity";
import { noRowMenu, PlayProgressCell, RowActions, type RowPlayState } from "./RowPlayAction";
import { useMediaProgress } from "./useMediaProgress";
import { DetailShell } from "./DetailShell";
import { useAudioPages } from "./useAudioPages";

type TrackRow = NonNullable<AlbumDetailQuery["album"]>["tracks"]["edges"][number]["node"];

type TrackCellProps = { row: { original: TrackRow } };
const AlbumTableContext = createContext<{
  actions: ActionSet<TrackRow>;
  statuses: ReturnType<typeof useContentStatuses>;
  currentItemId: string | undefined;
  discs: number;
  fractionOf: (track: TrackRow) => number;
  playStateOf: (track: TrackRow) => RowPlayState;
  press: (track: TrackRow) => void;
  want?: (track: TrackRow) => Promise<unknown>;
} | null>(null);

function useAlbumTable() {
  const context = useContext(AlbumTableContext);
  if (!context) throw new Error("Album cells require AlbumTableContext");
  return context;
}

function TrackTitleCell({ row: { original: track } }: TrackCellProps) {
  const { currentItemId } = useAlbumTable();
  return <TwoLineCell title={<span className={currentItemId === track.id ? "text-brand" : undefined}>{track.title}</span>} subtitle={track.artistName || undefined} />;
}

function TrackNumberCell({ row: { original: track } }: TrackCellProps) {
  const { discs } = useAlbumTable();
  return discs > 1 ? <TextCell identifier>{`${track.discNumber ?? 1}-${track.trackNumber}`}</TextCell> : <NumericCell value={track.trackNumber} />;
}

function TrackStatusCell({ row: { original: track } }: TrackCellProps) {
  const { statuses } = useAlbumTable();
  return <StatusChip status={track.mediaFileId ? statusMeta("AVAILABLE") : contentStatusMeta(statuses, track.id)} />;
}

function TrackFormatCell({ row: { original: track } }: TrackCellProps) {
  return <TextCell muted>{track.mediaFile ? [track.mediaFile.audioCodec?.toUpperCase(), track.mediaFile.bitrate ? `${Math.round(track.mediaFile.bitrate / 1000)} kb/s` : null].filter(Boolean).join(" · ") || "—" : "—"}</TextCell>;
}

function TrackDurationCell({ row: { original: track } }: TrackCellProps) {
  return <TextCell identifier>{formatClock(track.durationSecs ?? track.mediaFile?.duration)}</TextCell>;
}

function TrackProgressCell({ row: { original: track } }: TrackCellProps) {
  const { fractionOf } = useAlbumTable();
  return <PlayProgressCell fraction={fractionOf(track)} label={`Listening progress for ${track.title}`} available={Boolean(track.mediaFileId)} />;
}

function TrackActionsCell({ row: { original: track } }: TrackCellProps) {
  const { actions, playStateOf, press, want } = useAlbumTable();
  return (
    <RowActions
      set={actions}
      target={track}
      label={track.title}
      play={track.mediaFileId ? { state: playStateOf(track), onPress: () => press(track) } : undefined}
      want={want && !track.mediaFileId ? { wanted: track.wanted, onPress: () => want(track) } : undefined}
    />
  );
}

// flexRender treats cell functions as component types. Context keeps live values
// reactive through memoized rows without replacing the focused cell's component.
const trackColumn = createDataTableColumns<TrackRow>();
const trackColumns = trackColumn.columns([
  trackColumn.accessor("title", { header: "Title", sortable: true, filter: filters.text({ field: "title" }), cardRole: "title", priority: 1, width: 20, minWidth: 10, hideable: false, cell: TrackTitleCell }),
  trackColumn.accessor("trackNumber", { header: "Track", sortable: true, filter: filters.number({ field: "trackNumber", server: "int" }), priority: 2, width: 5, minWidth: 4, align: "end", grow: false, cell: TrackNumberCell }),
  trackColumn.display({ id: "status", header: "Status", cardRole: "status", priority: 2, width: 9, minWidth: 7, cell: TrackStatusCell }),
  trackColumn.display({ id: "progress", header: "Progress", priority: 3, width: 10, minWidth: 8, cell: TrackProgressCell }),
  trackColumn.display({ id: "format", header: "Format", priority: 4, width: 14, minWidth: 8, cell: TrackFormatCell }),
  trackColumn.display({ id: "duration", header: "Length", priority: 3, width: 6, minWidth: 5, align: "end", grow: false, cell: TrackDurationCell }),
  trackColumn.display({ id: "trackActions", header: "", priority: 1, width: 7, minWidth: 6, grow: false, hideable: false, cell: TrackActionsCell }),
]);

export function AlbumPage({ albumId }: { albumId: string }) {
  const client = useApolloClient();
  const { user } = useSession();
  const player = usePlayer();
  const isAdmin = useIsAdmin();
  const { data, previousData, loading, error, refetch } = useQuery(AlbumDetailDocument, { variables: { id: albumId }, context: { liveManaged: true } });
  const detail = (data ?? previousData)?.album;
  const album = detail?.id === albumId ? detail : null;
  const loadPage = useCallback(async (offset: number) => {
    const result = await client.query({ query: AlbumTracksPageDocument, variables: { id: albumId, offset }, fetchPolicy: "no-cache" });
    if (!result.data?.album) throw new Error("Album tracks unavailable");
    return result.data.album.tracks;
  }, [client, albumId]);
  const pages = useAudioPages(albumId, album?.tracks, loadPage, { document: AlbumDetailDocument, variables: { id: albumId }, refreshHead: refetch });
  const tracks = useMemo(() => [...pages.rows].sort((a, b) => (a.discNumber ?? 1) - (b.discNumber ?? 1) || a.trackNumber - b.trackNumber), [pages.rows]);
  const ids = useMemo(() => tracks.map((track) => track.id), [tracks]);
  const statuses = useContentStatuses("TRACK", ids);
  const [updateTrack] = useMutation(EntityTrackUpdateDocument, { refetchQueries: () => activeContentStatusQueries(client) });
  const [updateTracks] = useMutation(EntityTrackUpdateManyDocument, { refetchQueries: () => activeContentStatusQueries(client) });

  const toggleWanted = useCallback(async (track: TrackRow) => {
    if (!isAdmin) return;
    const result = assertSuccess((await updateTrack({ variables: { id: track.id, input: { wanted: !track.wanted } } })).data?.updateTrack, "Could not update track");
    if (result.track) pages.updateRow({ ...track, ...result.track });
    await refetch();
  }, [isAdmin, updateTrack, refetch, pages.updateRow]);

  const queue = useMemo(() => album ? tracks.map((track) => trackToPlayItem({ ...track, album })).filter((item): item is PlayItem => item !== null) : [], [tracks, album]);
  const playFrom = useCallback((track?: TrackRow) => {
    // A loaded row remains usable after a paging failure without claiming a full queue.
    const items = pages.complete ? queue : queue.filter((item) => item.entity.id === track?.id);
    const index = track ? items.findIndex((item) => item.entity.id === track.id) : 0;
    if (items.length && index >= 0) player.playAudio(items, index);
  }, [pages.complete, queue, player]);
  const discs = new Set(tracks.map((track) => track.discNumber ?? 1)).size;
  const fileIds = useMemo(() => tracks.map((track) => track.mediaFileId).filter((id): id is string => Boolean(id)), [tracks]);
  const { progressByFile } = useMediaProgress(fileIds, user?.id);
  const setSelectedWanted = useCallback(async (selected: readonly TrackRow[], wanted: boolean) => {
    if (!isAdmin) throw new Error("Administrator access required");
    const missing = selected.filter((item) => !item.mediaFileId && item.wanted !== wanted);
    if (!missing.length) throw new Error("No applicable missing tracks selected");
    try {
      const { data } = await updateTracks({ variables: { where: { id: { inList: missing.map((item) => item.id) }, albumId: { eq: albumId }, mediaFileId: { isNull: true } }, input: { wanted } } });
      const result = assertSuccess(data?.updateTracks, "Could not update selected tracks");
      if (result.affectedCount !== missing.length) throw new Error(`Updated ${result.affectedCount} of ${missing.length} selected tracks. Refresh and retry the remaining items.`);
      for (const item of missing) pages.updateRow({ ...item, wanted });
    } finally { await refetch(); }
  }, [isAdmin, albumId, updateTracks, pages.updateRow, refetch]);
  const selectedQueue = useCallback((selectedRows: readonly TrackRow[]) => {
    const selected = new Set(selectedRows.map((item) => item.id));
    return queue.filter((item) => selected.has(item.entity.id));
  }, [queue]);
  const selectedPlaybackUnavailable = (rows: readonly TrackRow[]) => rows.some((item) => !item.mediaFileId) ? "Some selected tracks have no file. Deselect them to play this selection." : undefined;
  const actions = useMemo(() => defineActions<TrackRow>("track", [
    { id: "play", label: "Play", icon: IconPlay, group: "open", primary: true, bulk: true,
      visible: (rows) => rows.length === 1 && Boolean(rows[0]?.mediaFileId),
      run: ([track]) => { if (track?.mediaFileId) playFrom(track); } },
    { id: "wanted", label: ([track]) => track?.wanted ? "Wanted" : "Want", icon: IconBookmark, group: "manage", priority: 1, bulk: true,
      visible: (rows) => rows.length === 1 && isAdmin && Boolean(rows[0] && !rows[0].mediaFileId),
      run: ([track]) => track && toggleWanted(track) },
    { id: "play-selected", label: "Play selected tracks", icon: IconPlay, group: "open", bulk: true,
      visible: (rows) => rows.length > 1, disabledReason: selectedPlaybackUnavailable,
      run: (rows) => { const items = selectedQueue(rows); if (!items.length || selectedPlaybackUnavailable(rows)) throw new Error("Selected tracks are not ready to play"); player.playAudio(items); } },
    { id: "enqueue", label: "Add to queue", icon: IconPlus, group: "organise", bulk: true,
      disabledReason: selectedPlaybackUnavailable,
      run: (rows) => { const items = selectedQueue(rows); if (!items.length || selectedPlaybackUnavailable(rows)) throw new Error("Selected tracks are not ready to queue"); player.enqueue(items); }, successMessage: () => "Added to queue" },
    { id: "want-selected", label: "Want selected missing tracks", icon: IconBookmark, group: "manage", bulk: true,
      visible: (rows) => isAdmin && rows.length > 1 && rows.some((item) => !item.mediaFileId && !item.wanted),
      run: (rows) => setSelectedWanted(rows, true), successMessage: () => "Selected missing tracks wanted" },
    { id: "unwant-selected", label: "Unwant selected missing tracks", icon: IconBookmark, group: "manage", bulk: true,
      visible: (rows) => isAdmin && rows.length > 1 && rows.some((item) => !item.mediaFileId && item.wanted),
      run: (rows) => setSelectedWanted(rows, false), successMessage: () => "Selected missing tracks no longer wanted" },
    { id: "copy-titles", label: "Copy track titles", icon: IconCopy, group: "organise", bulk: true,
      run: async (rows) => { await copyText(rows.map((item) => item.title).join("\n")); }, successMessage: () => "Track titles copied" },
  ]), [isAdmin, playFrom, toggleWanted, setSelectedWanted, selectedQueue, player]);

  const currentItemId = player.audio?.entity.id;
  const fractionOf = (track: TrackRow) => {
    if (!track.mediaFileId) return 0;
    const saved = progressByFile.get(track.mediaFileId);
    if (track.id === currentItemId && player.transport.duration > 0) return Math.min(1, player.transport.position / player.transport.duration);
    if (saved?.isWatched) return 1;
    const seconds = track.mediaFile?.duration ?? track.durationSecs ?? saved?.duration ?? 0;
    return saved && seconds > 0 ? Math.min(1, saved.currentPosition / seconds) : 0;
  };
  const playStateOf = (track: TrackRow): RowPlayState => {
    if (track.id === currentItemId && player.transport.playing) return "playing";
    if (track.id !== currentItemId && track.mediaFileId && progressByFile.get(track.mediaFileId)?.isWatched) return "finished";
    return fractionOf(track) > 0 ? "resume" : "ready";
  };
  // Pressing what's already playing pauses or carries on; pressing anything else starts it.
  const press = (track: TrackRow) => {
    if (track.id === currentItemId) playerStore.updateTransport({ playing: !player.transport.playing });
    else playFrom(track);
  };

  const playAlbum = queue.length > 0 ? { label: "Play", run: () => playFrom(), disabledReason: !pages.complete ? "Load all tracks to play the album" : undefined } : undefined;
  const enqueue = pages.complete && queue.length > 0 ? () => player.enqueue(queue) : undefined;
  const { actions: albumActions, dialogs: albumDialogs, openSettings } = useAlbumEntity<NonNullable<typeof album>>({ open: false, play: playAlbum, enqueue, artist: tracks[0]?.artistName });

  if (error && !album) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!album) return loading ? <SkeletonHero /> : <EmptyState icon={IconInfo} title="Album not found" description="This item may have been removed from the library." className="page-gutter my-8" />;

  return (
    <DetailShell
      backdrop={null}
      hero={{
        poster: albumCover(album),
        posterAspect: "square",
        tint: LIBRARY_TYPES.music.tintVar,
        eyebrow: album.library ? (
          <Link to="/libraries/$libraryId/albums" params={{ libraryId: album.library.id }} className="nav-focus rounded hover:underline">
            {album.library.name}
          </Link>
        ) : null,
        title: album.name,
        meta: (
          <>
            <MetaLine
              items={[
                <Link key="artist" to="/artists/$artistId" params={{ artistId: album.artistId }} className="nav-focus rounded text-foreground hover:underline">
                  {tracks[0]?.artistName ?? "Artist"}
                </Link>,
                formatYear(album.releaseDate) || album.year,
                album.albumType,
                `${album.tracks.pageInfo.totalCount ?? album.trackCount ?? tracks.length} tracks`,
                formatRuntime(album.totalDurationSecs),
              ]}
            />
            <AcquisitionChip status={autoDownloadMeta(album.autoDownloadMode)} onPress={isAdmin ? () => openSettings(album, "downloads") : undefined} />
          </>
        ),
        actions: (
          <MediaActions target={album} label={album.name} actions={albumActions} play={playAlbum} />
        ),
      }}
    >
      <AlbumTableContext.Provider value={{ actions, statuses, currentItemId, discs, fractionOf, playStateOf, press, want: isAdmin ? toggleWanted : undefined }}>
        <LoadedDataTable
          id={`album-tracks:${user?.id ?? "anonymous"}:${album.libraryId}:${album.id}`}
          label={`${album.name} tracks`} columns={trackColumns} rows={tracks} getRowId={(track) => track.id} getRowLabel={(track) => track.title}
          height="auto" noun={{ one: "track", other: "tracks" }} selectable actions={actions} renderRowActions={noRowMenu}
          renderBulkActions={(rows) => <TableBulkActions set={actions} rows={rows} noun={{ one: "track", other: "tracks" }} />}
          complete={pages.complete} search={{ fields: ["title", { field: "artistName", value: (track) => track.artistName }], placeholder: "Search tracks" }} refetching={loading || pages.loading}
          status={pages.error ? <span role="status">Could not load all tracks. Play individual tracks or <Button size="sm" variant="ghost" onClick={pages.retry}>Retry tracks</Button></span> : pages.loading ? "Loading remaining tracks…" : error ? <span role="status">Could not refresh tracks. <Button size="sm" variant="ghost" onClick={() => void refetch()}>Retry refresh</Button></span> : undefined}
          emptyMessage="No tracks" onRowOpen={(track) => { if (track.mediaFileId) playFrom(track); }}
        />
      </AlbumTableContext.Provider>
      {albumDialogs}
    </DetailShell>
  );
}
