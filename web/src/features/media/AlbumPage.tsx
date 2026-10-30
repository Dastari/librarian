import { useApolloClient, useMutation } from "@apollo/client/react";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { toast } from "glass-ux/toast";
import { Link, useNavigate } from "@tanstack/react-router";
import { ActionBar, defineActions, type ActionSet } from "glass-ux/actions";
import { Button } from "glass-ux/button";
import { createDataTableColumns, filters, NumericCell, TextCell, TwoLineCell } from "glass-ux/data-table";
import { IconBookmark, IconInfo, IconPlay } from "glass-ux/icons";
import { createContext, useCallback, useContext, useMemo, useState } from "react";

import { ConfirmDialog, EmptyState, ErrorState, KeyValueList, MetaLine, Panel, SkeletonHero, StatusChip } from "@/components/ui";
import { LoadedDataTable } from "@/components/ui/LoadedDataTable";
import { AcquisitionChip } from "@/features/acquisition/AcquisitionChip";
import { AcquisitionDialog } from "@/features/acquisition/AcquisitionDialog";
import { autoDownloadMeta } from "@/features/acquisition/mode";
import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { trackToPlayItem } from "@/features/libraries/browser/TracksBrowser";
import { usePlayer } from "@/features/player/usePlayer";
import type { PlayItem } from "@/features/player/store";
import { AlbumDetailDocument, AlbumTracksPageDocument, EntityAlbumDeleteDocument, EntityTrackUpdateDocument, type AlbumDetailQuery } from "@/graphql/generated/graphql";
import { useContentStatuses } from "@/hooks/useContentStatuses";
import { albumCover } from "@/lib/artwork";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { formatBytes, formatClock, formatDate, formatRuntime, formatYear } from "@/lib/format";
import { activeContentStatusQueries } from "@/lib/graphql/content-status";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { statusMeta } from "@/lib/status";

import { MediaActions } from "./MediaActions";
import { DetailShell } from "./DetailShell";
import { useAudioPages } from "./useAudioPages";

type TrackRow = NonNullable<AlbumDetailQuery["album"]>["tracks"]["edges"][number]["node"];

type TrackCellProps = { row: { original: TrackRow } };
const AlbumTableContext = createContext<{
  actions: ActionSet<TrackRow>;
  statuses: ReturnType<typeof useContentStatuses>;
  currentItemId: string | undefined;
  discs: number;
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
  return <StatusChip status={statusMeta(track.mediaFileId ? "AVAILABLE" : statuses.get(track.id))} />;
}

function TrackFormatCell({ row: { original: track } }: TrackCellProps) {
  return <TextCell muted>{track.mediaFile ? [track.mediaFile.audioCodec?.toUpperCase(), track.mediaFile.bitrate ? `${Math.round(track.mediaFile.bitrate / 1000)} kb/s` : null, formatBytes(track.mediaFile.size)].filter(Boolean).join(" · ") : "—"}</TextCell>;
}

function TrackDurationCell({ row: { original: track } }: TrackCellProps) {
  return <TextCell identifier>{formatClock(track.durationSecs ?? track.mediaFile?.duration)}</TextCell>;
}

function TrackActionsCell({ row }: TrackCellProps) {
  const { actions } = useAlbumTable();
  return <ActionBar set={actions} targets={[row.original]} label={row.original.title} maxButtons={1} />;
}

// flexRender treats cell functions as component types. Context keeps live values
// reactive through memoized rows without replacing the focused cell's component.
const trackColumn = createDataTableColumns<TrackRow>();
const trackColumns = trackColumn.columns([
  trackColumn.accessor("title", { header: "Title", sortable: true, filter: filters.text({ field: "title" }), cardRole: "title", priority: 1, width: 20, minWidth: 10, hideable: false, cell: TrackTitleCell }),
  trackColumn.accessor("trackNumber", { header: "Track", sortable: true, filter: filters.number({ field: "trackNumber", server: "int" }), priority: 2, width: 5, minWidth: 4, align: "end", grow: false, cell: TrackNumberCell }),
  trackColumn.display({ id: "status", header: "Status", cardRole: "status", priority: 2, width: 9, minWidth: 7, cell: TrackStatusCell }),
  trackColumn.display({ id: "format", header: "Format", priority: 4, width: 14, minWidth: 8, cell: TrackFormatCell }),
  trackColumn.display({ id: "duration", header: "Length", priority: 3, width: 6, minWidth: 5, align: "end", grow: false, cell: TrackDurationCell }),
  trackColumn.display({ id: "trackActions", header: "Actions", priority: 1, width: 12, minWidth: 10, grow: false, hideable: false, cell: TrackActionsCell }),
]);

export function AlbumPage({ albumId }: { albumId: string }) {
  const navigate = useNavigate();
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
  const [deleteAlbum, { loading: deleting }] = useMutation(EntityAlbumDeleteDocument);
  const [updateTrack] = useMutation(EntityTrackUpdateDocument, { refetchQueries: () => activeContentStatusQueries(client) });
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [searching, setSearching] = useState(false);
  const [acquisition, setAcquisition] = useState(false);

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
  const actions = useMemo(() => defineActions<TrackRow>("track", [
    { id: "play", label: "Play", icon: IconPlay, group: "open", primary: true,
      visible: ([track]) => Boolean(track?.mediaFileId),
      run: ([track]) => { if (track?.mediaFileId) playFrom(track); } },
    { id: "wanted", label: ([track]) => track?.wanted ? "Wanted" : "Want", icon: IconBookmark, group: "manage", priority: 1,
      visible: ([track]) => isAdmin && Boolean(track && !track.mediaFileId),
      run: ([track]) => track && toggleWanted(track) },
  ]), [isAdmin, playFrom, toggleWanted]);

  if (error && !album) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!album) return loading ? <SkeletonHero /> : <EmptyState icon={IconInfo} title="Album not found" description="This item may have been removed from the library." className="page-gutter my-8" />;

  const remove = async () => {
    try {
      const { data: result } = await deleteAlbum({ variables: { id: album.id } });
      assertSuccess(result?.deleteAlbum, "Could not remove album");
      toast.success("Album removed");
      await navigate({ to: "/libraries/$libraryId/albums", params: { libraryId: album.libraryId } });
    } catch (mutationError) {
      toast.danger(errorMessage(mutationError));
    }
  };

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
            <AcquisitionChip status={autoDownloadMeta(album.autoDownloadMode)} onPress={isAdmin ? () => setAcquisition(true) : undefined} />
          </>
        ),
        actions: (
          <MediaActions
            personal={album.userId === user?.id ? { kind: "album", id: album.id } : undefined}
            id={album.id}
            label={album.name}
            play={queue.length > 0 ? { label: "Play", run: () => playFrom(), disabledReason: !pages.complete ? "Load all tracks to play the album" : undefined } : undefined}
            enqueue={pages.complete && queue.length > 0 ? () => player.enqueue(queue) : undefined}
            find={isAdmin ? () => setSearching(true) : undefined}
            settings={isAdmin ? () => setAcquisition(true) : undefined}
            remove={isAdmin ? () => setConfirmDelete(true) : undefined}
          />
        ),
      }}
    >
      <AlbumTableContext.Provider value={{ actions, statuses, currentItemId: player.audio?.entity.id, discs }}>
        <LoadedDataTable
          id={`album-tracks:${user?.id ?? "anonymous"}:${album.libraryId}:${album.id}`}
          label={`${album.name} tracks`} columns={trackColumns} rows={tracks} getRowId={(track) => track.id} getRowLabel={(track) => track.title}
          height="auto" density="compact" noun={{ one: "track", other: "tracks" }} selectable={false} actions={actions}
          complete={pages.complete} search={{ fields: ["title", { field: "artistName", value: (track) => track.artistName }], placeholder: "Search tracks" }} refetching={loading || pages.loading}
          status={pages.error ? <span role="status">Could not load all tracks. Play individual tracks or <Button size="sm" variant="ghost" onClick={pages.retry}>Retry tracks</Button></span> : pages.loading ? "Loading remaining tracks…" : error ? <span role="status">Could not refresh tracks. <Button size="sm" variant="ghost" onClick={() => void refetch()}>Retry refresh</Button></span> : undefined}
          emptyMessage="No tracks" onRowOpen={(track) => { if (track.mediaFileId) playFrom(track); }}
        />
      </AlbumTableContext.Provider>
      <Panel title="Details" className="lg:max-w-2xl">
        <KeyValueList columns={2} items={[{ label: "Label", value: album.label }, { label: "Country", value: album.country }, { label: "Genres", value: album.genres.join(", ") || undefined }, { label: "Released", value: formatDate(album.releaseDate) }, { label: "Discs", value: album.discCount }, { label: "Size", value: formatBytes(album.sizeBytes) }, { label: "Folder", value: album.path, mono: true }, { label: "MusicBrainz", value: album.musicbrainzId ? <a className="nav-focus rounded text-brand hover:underline" href={`https://musicbrainz.org/release-group/${album.musicbrainzId}`} target="_blank" rel="noreferrer">{album.musicbrainzId}</a> : undefined }]} />
      </Panel>
      <AcquisitionDialog isOpen={acquisition} onOpenChange={setAcquisition} target={{ kind: "album", id: album.id, title: album.name, libraryId: album.libraryId, autoDownloadMode: album.autoDownloadMode, qualityProfileId: album.qualityProfileId }} onSaved={() => void refetch()} />
      <ReleaseSearchDialog isOpen={searching} onOpenChange={setSearching} query={[tracks[0]?.artistName, album.name].filter(Boolean).join(" ")} year={album.year} libraryId={album.libraryId} artist={tracks[0]?.artistName} album={album.name} target={{ albumId: album.id }} />
      <ConfirmDialog isOpen={confirmDelete} onOpenChange={setConfirmDelete} title={`Remove ${album.name}?`} description="The album and its tracks leave the catalogue. Files stay on disk." confirmLabel="Remove" destructive isPending={deleting} onConfirm={remove} />
    </DetailShell>
  );
}
