import { useMutation, useQuery } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { Link, useNavigate } from "@tanstack/react-router";
import { IconAdjustments, IconBookmark, IconBookmarkFilled, IconDownload, IconPlayerPlayFilled, IconPlaylistAdd, IconTrash } from "@tabler/icons-react";
import { useMemo, useState } from "react";

import { Button, ConfirmDialog, DataTable, ErrorState, GlassButton, KeyValueList, MetaLine, Panel, SkeletonHero, StatusChip, type DataTableColumn } from "@/components/ui";
import { AcquisitionChip } from "@/features/acquisition/AcquisitionChip";
import { AcquisitionDialog } from "@/features/acquisition/AcquisitionDialog";
import { autoDownloadMeta } from "@/features/acquisition/mode";
import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { trackToPlayItem } from "@/features/libraries/browser/TracksBrowser";
import { usePlayer } from "@/features/player/usePlayer";
import type { PlayItem } from "@/features/player/store";
import { AlbumDetailDocument, EntityAlbumDeleteDocument, EntityTrackUpdateDocument, type AlbumDetailQuery } from "@/graphql/generated/graphql";
import { useContentStatuses } from "@/hooks/useContentStatuses";
import { albumCover } from "@/lib/artwork";
import { useIsAdmin } from "@/lib/auth/useSession";
import { formatBytes, formatClock, formatDate, formatRuntime, formatYear } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPES } from "@/lib/library-types";
import { statusMeta } from "@/lib/status";

import { DetailShell } from "./DetailShell";

type TrackRow = NonNullable<AlbumDetailQuery["album"]>["tracks"]["edges"][number]["node"];

export function AlbumPage({ albumId }: { albumId: string }) {
  const navigate = useNavigate();
  const player = usePlayer();
  const isAdmin = useIsAdmin();
  const { data, previousData, loading, error, refetch } = useQuery(AlbumDetailDocument, { variables: { id: albumId } });
  const album = (data ?? previousData)?.album ?? null;
  const tracks = useMemo(() => [...(album?.tracks.edges.map((edge) => edge.node) ?? [])].sort((a, b) => (a.discNumber ?? 1) - (b.discNumber ?? 1) || a.trackNumber - b.trackNumber), [album]);
  const ids = useMemo(() => tracks.map((track) => track.id), [tracks]);
  const statuses = useContentStatuses("TRACK", ids);
  const [deleteAlbum, { loading: deleting }] = useMutation(EntityAlbumDeleteDocument);
  const [updateTrack] = useMutation(EntityTrackUpdateDocument);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [searching, setSearching] = useState(false);
  const [acquisition, setAcquisition] = useState(false);

  const toggleWanted = async (track: TrackRow) => {
    try {
      assertSuccess((await updateTrack({ variables: { id: track.id, input: { wanted: !track.wanted } } })).data?.updateTrack, "Could not update track");
      void refetch();
    } catch (mutationError) {
      toast.danger(errorMessage(mutationError));
    }
  };

  if (error && !album) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!album) return loading ? <SkeletonHero /> : null;

  const queue = tracks.map((track) => trackToPlayItem({ ...track, album })).filter((item): item is PlayItem => item !== null);
  const playFrom = (track?: TrackRow) => {
    if (queue.length === 0) return;
    const index = track ? Math.max(0, queue.findIndex((item) => item.entity.id === track.id)) : 0;
    player.playAudio(queue, index);
  };
  const isCurrent = (track: TrackRow) => player.audio?.entity.id === track.id;
  const discs = new Set(tracks.map((track) => track.discNumber ?? 1)).size;

  const columns: Array<DataTableColumn<TrackRow>> = [
    { id: "n", header: "#", size: 64, align: "end", numeric: true, cell: (track) => (isCurrent(track) ? <IconPlayerPlayFilled size={14} className="ml-auto text-brand" /> : discs > 1 ? `${track.discNumber ?? 1}-${track.trackNumber}` : track.trackNumber) },
    {
      id: "title",
      header: "Title",
      cell: (track) => (
        <span className="min-w-0">
          <span className={`block truncate text-body-sm ${isCurrent(track) ? "text-brand" : "text-foreground"}`}>{track.title}</span>
          {track.artistName && track.artistName !== album.name ? <span className="block truncate text-label-sm text-muted">{track.artistName}</span> : null}
        </span>
      ),
    },
    { id: "status", header: "", size: 130, cell: (track) => (track.mediaFileId ? null : <StatusChip status={statusMeta(statuses.get(track.id))} />) },
    { id: "format", header: "Format", size: 150, hideBelow: "md", cell: (track) => <span className="text-muted">{track.mediaFile ? [track.mediaFile.audioCodec?.toUpperCase(), track.mediaFile.bitrate ? `${Math.round(track.mediaFile.bitrate / 1000)} kb/s` : null, formatBytes(track.mediaFile.size)].filter(Boolean).join(" · ") : "—"}</span> },
    { id: "duration", header: "Length", size: 90, align: "end", numeric: true, cell: (track) => formatClock(track.durationSecs ?? track.mediaFile?.duration) },
    ...(isAdmin
      ? [
          {
            id: "want",
            header: "",
            size: 110,
            align: "end" as const,
            cell: (track: TrackRow) =>
              track.mediaFileId ? null : (
                <Button size="sm" variant="ghost" onPress={() => void toggleWanted(track)}>
                  {track.wanted ? <IconBookmarkFilled size={14} className="text-brand" /> : <IconBookmark size={14} />} {track.wanted ? "Wanted" : "Want"}
                </Button>
              ),
          },
        ]
      : []),
  ];

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
      hero={{
        backdrop: albumCover(album),
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
                `${tracks.length} tracks`,
                formatRuntime(album.totalDurationSecs),
              ]}
            />
            <AcquisitionChip status={autoDownloadMeta(album.autoDownloadMode)} onPress={() => setAcquisition(true)} />
          </>
        ),
        actions: (
          <>
            {queue.length > 0 ? (
              <GlassButton emphasis="brand" size="lg" refract onPress={() => playFrom()}>
                <IconPlayerPlayFilled /> Play
              </GlassButton>
            ) : null}
            {queue.length > 0 ? (
              <GlassButton size="lg" refract onPress={() => player.enqueue(queue)}>
                <IconPlaylistAdd /> Add to queue
              </GlassButton>
            ) : null}
            {isAdmin ? (
              <>
                <GlassButton size="lg" refract onPress={() => setSearching(true)}>
                  <IconDownload /> Find releases
                </GlassButton>
                <GlassButton size="lg" refract isIconOnly aria-label="Download settings" onPress={() => setAcquisition(true)}>
                  <IconAdjustments />
                </GlassButton>
                <GlassButton size="lg" refract isIconOnly aria-label="Remove from library" onPress={() => setConfirmDelete(true)}>
                  <IconTrash />
                </GlassButton>
              </>
            ) : null}
          </>
        ),
      }}
    >
      <DataTable<TrackRow> columns={columns} rows={tracks} getRowId={(track) => track.id} density="compact" noun="tracks" onRowClick={(track) => track.mediaFileId && playFrom(track)} />
      <Panel title="Details" className="lg:max-w-2xl">
        <KeyValueList columns={2} items={[{ label: "Label", value: album.label }, { label: "Country", value: album.country }, { label: "Genres", value: album.genres.join(", ") || undefined }, { label: "Released", value: formatDate(album.releaseDate) }, { label: "Discs", value: album.discCount }, { label: "Size", value: formatBytes(album.sizeBytes) }, { label: "Folder", value: album.path, mono: true }, { label: "MusicBrainz", value: album.musicbrainzId ? <a className="nav-focus rounded text-brand hover:underline" href={`https://musicbrainz.org/release-group/${album.musicbrainzId}`} target="_blank" rel="noreferrer">{album.musicbrainzId}</a> : undefined }]} />
      </Panel>
      <AcquisitionDialog isOpen={acquisition} onOpenChange={setAcquisition} target={{ kind: "album", id: album.id, title: album.name, libraryId: album.libraryId, autoDownloadMode: album.autoDownloadMode, qualityProfileId: album.qualityProfileId }} onSaved={() => void refetch()} />
      <ReleaseSearchDialog isOpen={searching} onOpenChange={setSearching} query={[tracks[0]?.artistName, album.name].filter(Boolean).join(" ")} year={album.year} libraryId={album.libraryId} artist={tracks[0]?.artistName} album={album.name} target={{ albumId: album.id }} />
      <ConfirmDialog isOpen={confirmDelete} onOpenChange={setConfirmDelete} title={`Remove ${album.name}?`} description="The album and its tracks leave the catalogue. Files stay on disk." confirmLabel="Remove" destructive isPending={deleting} onConfirm={remove} />
    </DetailShell>
  );
}
