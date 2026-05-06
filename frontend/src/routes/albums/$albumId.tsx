import {
  createFileRoute,
  Link,
  redirect,
  useNavigate,
} from "@tanstack/react-router";
import { useState, useCallback, useMemo } from "react";
import { Button } from "@heroui/button";
import {
  Dropdown,
  DropdownTrigger,
  DropdownMenu,
  DropdownItem,
} from "@heroui/dropdown";
import { Card, CardBody } from "@heroui/card";
import { Chip } from "@heroui/chip";
import { Image } from "@heroui/image";
import { Breadcrumbs, BreadcrumbItem } from "@heroui/breadcrumbs";
import { Progress } from "@heroui/progress";
import { useDisclosure } from "@heroui/modal";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/modal";
import { addToast } from "@heroui/toast";
import { RouteError } from "../../components/RouteError";
import { sanitizeError, formatBytes, formatDuration } from "../../lib/format";
import { useQuery, useMutation } from "../../lib/graphql/client";
import {
  AlbumDetailSetTrackWantedDocument,
  AlbumDetailRouteDocument,
  DeleteAlbumRouteDocument,
  LibraryDetailRouteDocument,
  type AlbumDetailRouteQuery,
} from "../../lib/graphql/generated/graphql";
import {
  type DataTableColumn,
  type RowAction,
} from "../../components/data-table";
import {
  IconDisc,
  IconMusic,
  IconSearch,
  IconRefresh,
  IconPlayerPlay,
  IconPlayerPause,
  IconInfoCircle,
  IconTrash,
  IconDotsVertical,
  IconCheck,
  IconX,
} from "@tabler/icons-react";
// Note: IconPlayerPause is used for artwork overlay
import { FilePropertiesModal } from "../../components/FilePropertiesModal";
import { TrackStatusChip, PlayPauseIndicator } from "../../components/shared";
import { usePlaybackContext } from "../../contexts/PlaybackContext";
import { useDataReactivity } from "../../hooks/useSubscription";
import { DetailItemsTable } from "../../components/media/DetailItemsTable";

export const Route = createFileRoute("/albums/$albumId")({
  beforeLoad: ({ context, location }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({
        to: "/",
        search: {
          signin: true,
          redirect: location.href,
        },
      });
    }
  },
  component: AlbumDetailPage,
  errorComponent: RouteError,
});

type AlbumDetailNode = NonNullable<AlbumDetailRouteQuery["Album"]>;
type AlbumTrackNode = AlbumDetailRouteQuery["Tracks"]["Edges"][number]["Node"];
type TrackStatusView = "missing" | "wanted" | "downloading" | "downloaded";

interface TrackWithStatus {
  track: {
    id: string;
    albumId: string;
    libraryId: string;
    title: string;
    trackNumber: number;
    discNumber: number;
    musicbrainzId: string | null;
    isrc: string | null;
    durationSecs: number | null;
    explicit: boolean;
    artistName: string | null;
    artistId: string | null;
    mediaFileId: string | null;
    hasFile: boolean;
    status: TrackStatusView;
    wanted: boolean;
    downloadProgress: number | null;
  };
  hasFile: boolean;
  filePath: string | null;
  fileSize: number | null;
  audioCodec: string | null;
  bitrate: number | null;
  audioChannels: string | null;
}

interface AlbumWithTracks {
  album: {
    id: string;
    artistId: string;
    libraryId: string;
    name: string;
    sortName: string | null;
    year: number | null;
    musicbrainzId: string | null;
    albumType: string | null;
    genres: string[];
    label: string | null;
    country: string | null;
    releaseDate: string | null;
    coverUrl: string | null;
    trackCount: number | null;
    discCount: number | null;
    totalDurationSecs: number | null;
    hasFiles: boolean;
    sizeBytes: number | null;
    path: string | null;
    downloadedTrackCount: number;
  };
  artistName: string | null;
  tracks: TrackWithStatus[];
  trackCount: number;
  tracksWithFiles: number;
  missingTracks: number;
  completionPercent: number;
}

// Helper to format audio codec display name
function formatAudioCodec(codec: string | null): string {
  if (!codec) return "";
  const normalized = codec.toLowerCase();
  if (normalized.includes("flac")) return "FLAC";
  if (normalized.includes("alac")) return "ALAC";
  if (normalized.includes("aac")) return "AAC";
  if (normalized.includes("mp3") || normalized.includes("mpeg")) return "MP3";
  if (normalized.includes("opus")) return "Opus";
  if (normalized.includes("vorbis")) return "Vorbis";
  if (normalized.includes("wav") || normalized.includes("pcm")) return "WAV";
  return codec.toUpperCase();
}

// Helper to format bitrate
function formatBitrate(bitrate: number | null): string {
  if (!bitrate) return "";
  if (bitrate >= 1000) {
    return `${(bitrate / 1000).toFixed(0)} Mbps`;
  }
  return `${bitrate} kbps`;
}

// Track table columns
const trackColumns: DataTableColumn<TrackWithStatus>[] = [
  {
    key: "trackNumber",
    label: "#",
    width: 60,
    sortable: true,
    render: (t) => (
      <span className="font-mono text-default-500">
        {t.track.discNumber > 1 && `${t.track.discNumber}-`}
        {String(t.track.trackNumber).padStart(2, "0")}
      </span>
    ),
  },
  {
    key: "title",
    label: "Title",
    sortable: true,
    render: (t) => (
      <div className="flex flex-col">
        <span className="font-medium">{t.track.title}</span>
        {t.track.artistName && (
          <span className="text-xs text-default-400">{t.track.artistName}</span>
        )}
      </div>
    ),
  },
  {
    key: "duration",
    label: "Duration",
    width: 80,
    render: (t) => (
      <span className="text-default-500 text-sm">
        {t.track.durationSecs ? formatDuration(t.track.durationSecs) : "-"}
      </span>
    ),
  },
  {
    key: "quality",
    label: "Quality",
    width: 150,
    render: (t) => {
      if (!t.hasFile || !t.track.mediaFileId) {
        return <span className="text-default-400">-</span>;
      }
      return (
        <div className="flex items-center gap-1.5 flex-wrap">
          {t.audioCodec && (
            <Chip
              size="sm"
              variant="flat"
              color="primary"
              className="h-5 text-xs"
            >
              {formatAudioCodec(t.audioCodec)}
            </Chip>
          )}
          {t.bitrate && (
            <Chip
              size="sm"
              variant="flat"
              color="secondary"
              className="h-5 text-xs"
            >
              {formatBitrate(t.bitrate)}
            </Chip>
          )}
        </div>
      );
    },
  },
  {
    key: "size",
    label: "Size",
    width: 100,
    render: (t) => {
      if (!t.hasFile || !t.fileSize) {
        return <span className="text-default-400">-</span>;
      }
      return (
        <span className="text-default-500 text-sm text-nowrap">
          {formatBytes(t.fileSize)}
        </span>
      );
    },
  },
  {
    key: "status",
    label: "Status",
    width: 140,
    sortable: true,
    render: (t) => (
      <TrackStatusChip
        mediaFileId={t.track.mediaFileId}
        downloadProgress={t.track.downloadProgress}
        wanted={t.track.wanted}
      />
    ),
  },
];

interface TrackTableProps {
  tracks: TrackWithStatus[];
  albumId: string;
  onPlay: (track: TrackWithStatus) => void;
  onSearch: (track: TrackWithStatus) => void;
  onShowProperties: (track: TrackWithStatus) => void;
  fetchAlbum: () => void;
}

function TrackTable({
  tracks,
  albumId,
  onPlay,
  onSearch,
  onShowProperties,
  fetchAlbum,
}: TrackTableProps) {
  // Get session and updatePlayback directly from context for reliable updates
  const { session, updatePlayback } = usePlaybackContext();

  // Handle pause directly using context
  const handlePause = useCallback(() => {
    updatePlayback({ isPlaying: false });
  }, [updatePlayback]);

  // Compute playing state from session
  const currentlyPlayingTrackId =
    session?.albumId === albumId ? session?.trackId : null;
  const isPlaying = session?.isPlaying ?? false;
  // Row actions - computed fresh on each render to ensure playing state is always current
  const rowActions: RowAction<TrackWithStatus>[] = [
    // Playing indicator with pause on hover - shown for currently playing track
    {
      key: `playing-${currentlyPlayingTrackId || "none"}`,
      label: "Pause",
      icon: (
        <PlayPauseIndicator
          size={16}
          isPlaying={isPlaying}
          colorClass="bg-success"
        />
      ),
      color: "default",
      inDropdown: false,
      isVisible: (t) =>
        t.track.status === "downloaded" &&
        !!t.track.mediaFileId &&
        currentlyPlayingTrackId === t.track.id &&
        isPlaying,
      onAction: () => handlePause(),
    },
    // Play action - shown for all other tracks or when paused
    {
      key: `play-${currentlyPlayingTrackId || "none"}-${isPlaying}`,
      label: "Play",
      icon: <IconPlayerPlay size={16} />,
      color: "success",
      inDropdown: false,
      isVisible: (t) =>
        t.track.status === "downloaded" &&
        !!t.track.mediaFileId &&
        !(currentlyPlayingTrackId === t.track.id && isPlaying),
      onAction: (t) => onPlay(t),
    },
    {
      key: "search",
      label: "Search for Track",
      icon: <IconSearch size={16} />,
      color: "default",
      inDropdown: false,
      // Show search for missing or wanted tracks (not downloading or downloaded)
      isVisible: (t) =>
        t.track.status === "missing" || t.track.status === "wanted",
      onAction: (t) => onSearch(t),
    },
    {
      key: "properties",
      label: "File Properties",
      icon: <IconInfoCircle size={16} />,
      color: "default",
      inDropdown: true,
      // Only show properties for downloaded tracks with a media file
      isVisible: (t) =>
        t.track.status === "downloaded" && !!t.track.mediaFileId,
      onAction: (t) => onShowProperties(t),
    },
  ];

  // Create selection set for highlighting currently playing track
  const selectedKeys = useMemo(() => {
    if (currentlyPlayingTrackId) {
      return new Set([currentlyPlayingTrackId]);
    }
    return new Set<string>();
  }, [currentlyPlayingTrackId]);

  // Key that changes when playback state changes to force re-render
  const tableKey = `tracks-${currentlyPlayingTrackId || "none"}-${isPlaying}`;

  return (
    <DetailItemsTable
      tableKey={tableKey}
      headerContent={
        <div className="p-4">
          <h2 className="text-lg font-semibold flex items-center gap-2">
            <IconMusic size={20} className="text-green-400" />
            Tracks
          </h2>
        </div>
      }
      data={tracks}
      columns={trackColumns}
      emptyContent={
        <div className="p-8 text-center">
          <IconMusic size={48} className="mx-auto mb-4 text-default-400" />
          <h3 className="text-lg font-semibold mb-2">No Tracks</h3>
          <p className="text-default-500 mb-4">
            Track information hasn't been fetched yet.
          </p>
          <Button variant="flat" onPress={fetchAlbum}>
            Refresh Album
          </Button>
        </div>
      }
      getRowKey={(t) => t.track.id}
      ariaLabel="Album tracks"
      isCompact
      showItemCount={false}
      hideToolbar
      defaultSortColumn="trackNumber"
      defaultSortDirection="asc"
      rowActions={rowActions}
      selectionMode={currentlyPlayingTrackId ? "single" : "none"}
      selectedKeys={selectedKeys}
    />
  );
}

function AlbumDetailPage() {
  const { albumId } = Route.useParams();
  const navigate = useNavigate();
  const { startTrackPlayback, session, updatePlayback } = usePlaybackContext();

  // Check if this album is currently playing
  const isThisAlbumPlaying = session?.albumId === albumId && session?.isPlaying;

  const {
    isOpen: isPropertiesOpen,
    onOpen: onPropertiesOpen,
    onClose: onPropertiesClose,
  } = useDisclosure();
  const {
    isOpen: isDeleteOpen,
    onOpen: onDeleteOpen,
    onClose: onDeleteClose,
  } = useDisclosure();
  const [propertiesTrack, setPropertiesTrack] =
    useState<TrackWithStatus | null>(null);
  const {
    data: albumQueryData,
    previousData: previousAlbumQueryData,
    loading: isLoading,
    refetch: refetchAlbum,
  } = useQuery(AlbumDetailRouteDocument, {
    variables: { Id: albumId },
    fetchPolicy: "cache-and-network",
  });

  const toTrackStatus = useCallback(
    (
      mediaFileId: string | null | undefined,
      wanted: boolean,
      backendStatus?: string | null,
    ): TrackWithStatus["track"]["status"] => {
      if (mediaFileId) return "downloaded";
      const normalized = backendStatus?.toLowerCase() ?? "";
      if (normalized === "downloading") return "downloading";
      return wanted ? "wanted" : "missing";
    },
    [],
  );

  const albumNode: AlbumDetailNode | null =
    albumQueryData?.Album ?? previousAlbumQueryData?.Album ?? null;
  const albumTrackEdges =
    albumQueryData?.Tracks?.Edges ??
    previousAlbumQueryData?.Tracks?.Edges ??
    [];

  const albumData = useMemo<AlbumWithTracks | null>(() => {
    if (!albumNode) return null;
    const tracks: TrackWithStatus[] = albumTrackEdges.map(
      (edge: { Node: AlbumTrackNode }) => ({
        track: {
          id: edge.Node.Id,
          albumId: edge.Node.AlbumId,
          libraryId: edge.Node.LibraryId,
          title: edge.Node.Title,
          trackNumber: edge.Node.TrackNumber,
          discNumber: edge.Node.DiscNumber ?? 1,
          musicbrainzId: edge.Node.MusicbrainzId ?? null,
          isrc: edge.Node.Isrc ?? null,
          durationSecs: edge.Node.DurationSecs ?? null,
          explicit: edge.Node.Explicit,
          artistName: edge.Node.ArtistName ?? null,
          artistId: edge.Node.ArtistId ?? null,
          mediaFileId: edge.Node.MediaFileId ?? null,
          hasFile: Boolean(edge.Node.MediaFileId),
          status: toTrackStatus(edge.Node.MediaFileId, edge.Node.Wanted),
          wanted: edge.Node.Wanted,
          downloadProgress: null,
        },
        hasFile: Boolean(edge.Node.MediaFileId),
        filePath: null,
        fileSize: null,
        audioCodec: null,
        bitrate: null,
        audioChannels: null,
      }),
    );
    const tracksWithFiles = tracks.filter((t) => t.hasFile).length;
    const missingTracks = tracks.filter((t) => !t.hasFile).length;
    const completionPercent =
      tracks.length > 0 ? (tracksWithFiles / tracks.length) * 100 : 0;
    return {
      album: {
        id: albumNode.Id,
        artistId: albumNode.ArtistId,
        libraryId: albumNode.LibraryId,
        name: albumNode.Name,
        sortName: albumNode.SortName ?? null,
        year: albumNode.Year ?? null,
        musicbrainzId: albumNode.MusicbrainzId ?? null,
        albumType: albumNode.AlbumType ?? null,
        genres: albumNode.Genres,
        label: albumNode.Label ?? null,
        country: albumNode.Country ?? null,
        releaseDate: albumNode.ReleaseDate ?? null,
        coverUrl: albumNode.CoverUrl ?? null,
        trackCount: albumNode.TrackCount ?? null,
        discCount: albumNode.DiscCount ?? null,
        totalDurationSecs: albumNode.TotalDurationSecs ?? null,
        hasFiles: albumNode.HasFiles,
        sizeBytes: albumNode.SizeBytes ?? null,
        path: albumNode.Path ?? null,
        downloadedTrackCount: tracksWithFiles,
      },
      artistName:
        tracks.find((t) => Boolean(t.track.artistName))?.track.artistName ??
        null,
      tracks,
      trackCount: albumNode.TrackCount ?? tracks.length,
      tracksWithFiles,
      missingTracks,
      completionPercent,
    };
  }, [albumNode, albumTrackEdges, toTrackStatus]);

  const {
    data: libraryData,
    previousData: previousLibraryData,
    refetch: refetchLibrary,
  } = useQuery(LibraryDetailRouteDocument, {
    variables: { Id: albumData?.album.libraryId ?? "" },
    skip: !albumData?.album.libraryId,
    fetchPolicy: "cache-and-network",
  });

  const library = libraryData?.Library ?? previousLibraryData?.Library ?? null;
  const error = !isLoading && !albumData ? "Album not found" : null;

  const fetchAlbum = useCallback(() => {
    void refetchAlbum();
    if (albumData?.album.libraryId) {
      void refetchLibrary();
    }
  }, [albumData?.album.libraryId, refetchAlbum, refetchLibrary]);

  const [deleteAlbum, { loading: isDeleting }] = useMutation(
    DeleteAlbumRouteDocument,
  );
  const [setTracksWanted] = useMutation(AlbumDetailSetTrackWantedDocument);

  // Keep data fresh with periodic updates and torrent completion events
  // This ensures download progress is updated in real-time
  useDataReactivity(fetchAlbum, {
    onTorrentComplete: true,
    periodicInterval: false,
    onFocus: false,
  });

  // Navigate to sources page for this album
  const handleManualHunt = useCallback(() => {
    navigate({ to: "/settings/sources" });
  }, [navigate]);

  // Handle play track - start playback with the audio player
  const handlePlayTrack = useCallback(
    (track: TrackWithStatus) => {
      if (track.track.mediaFileId && albumData) {
        // Get all tracks that have media files for the queue
        const allTracks = albumData.tracks
          .filter((t) => t.track.mediaFileId)
          .map((t) => t.track);

        startTrackPlayback(track.track, albumData.album, allTracks);
      }
    },
    [albumData, startTrackPlayback],
  );

  // Note: Playing state and pause handling are now inside TrackTable component directly from context

  // Navigate to sources page for a specific track
  const handleSearchTrack = useCallback(
    (_track: TrackWithStatus) => {
      navigate({ to: "/settings/sources" });
    },
    [navigate],
  );

  // Show file properties modal for a track
  const handleShowProperties = useCallback(
    (track: TrackWithStatus) => {
      setPropertiesTrack(track);
      onPropertiesOpen();
    },
    [onPropertiesOpen],
  );

  const handleDeleteAlbum = useCallback(async () => {
    if (!albumData) return;
    try {
      const result = await deleteAlbum({
        variables: { Id: albumData.album.id },
      });

      if (result.data?.DeleteAlbum?.Success) {
        addToast({
          title: "Album deleted",
          description: `${albumData.album.name} has been removed.`,
          color: "success",
        });
        navigate({
          to: "/libraries/$libraryId",
          params: { libraryId: albumData.album.libraryId },
        });
      } else {
        addToast({
          title: "Delete failed",
          description:
            result.data?.DeleteAlbum?.Error || "Failed to delete album",
          color: "danger",
        });
      }
    } catch (e) {
      addToast({
        title: "Error",
        description: sanitizeError(e),
        color: "danger",
      });
    } finally {
      onDeleteClose();
    }
  }, [albumData, deleteAlbum, navigate, onDeleteClose]);

  const handleSetWantedForAllTracks = useCallback(
    async (wanted: boolean) => {
      if (!albumData || albumData.tracks.length === 0) {
        addToast({
          title: "No tracks",
          description: "No tracks found to update",
          color: "warning",
        });
        return;
      }

      try {
        const { data } = await setTracksWanted({
          variables: { AlbumId: albumData.album.id, Wanted: wanted },
        });
        if (!data?.UpdateTracks?.success) {
          addToast({
            title: "Error",
            description:
              data?.UpdateTracks?.error ||
              "Failed to update wanted status for tracks",
            color: "danger",
          });
          return;
        }

        addToast({
          title: wanted ? "Marked as wanted" : "Removed wanted",
          description: wanted
            ? `${data.UpdateTracks.affectedCount} tracks marked as wanted`
            : `${data.UpdateTracks.affectedCount} tracks removed from wanted`,
          color: "success",
        });

        fetchAlbum();
      } catch (err) {
        console.error("Failed to update track wanted state:", err);
        addToast({
          title: "Error",
          description: "Failed to update wanted status",
          color: "danger",
        });
      }
    },
    [albumData, fetchAlbum, setTracksWanted],
  );

  // Calculate totals from tracks if not available on album
  const totalDurationSecs = useMemo(() => {
    if (!albumData) return 0;
    if (albumData.album.totalDurationSecs)
      return albumData.album.totalDurationSecs;
    return albumData.tracks.reduce(
      (sum, t) => sum + (t.track.durationSecs || 0),
      0,
    );
  }, [albumData]);

  const totalSizeBytes = useMemo(() => {
    if (!albumData) return 0;
    if (albumData.album.sizeBytes) return albumData.album.sizeBytes;
    return albumData.tracks.reduce((sum, t) => sum + (t.fileSize || 0), 0);
  }, [albumData]);

  // Get playable tracks for "Play All" functionality
  const playableTracks = useMemo(() => {
    if (!albumData) return [];
    return albumData.tracks.filter(
      (t) => t.track.status === "downloaded" && t.track.mediaFileId,
    );
  }, [albumData]);

  // Handle "Play All" - start playback from the first track
  const handlePlayAll = useCallback(() => {
    if (playableTracks.length > 0 && albumData) {
      const allTracks = playableTracks.map((t) => t.track);
      startTrackPlayback(allTracks[0], albumData.album, allTracks);
    }
  }, [playableTracks, albumData, startTrackPlayback]);

  if (!albumData) {
    if (isLoading) {
      return (
        <div className="container mx-auto p-4">
          <Card>
            <CardBody className="flex flex-col items-center justify-center py-12 text-center">
              <IconDisc size={48} className="mx-auto mb-4 text-default-400" />
              <p className="text-default-500">Loading album...</p>
            </CardBody>
          </Card>
        </div>
      );
    }

    return (
      <div className="container mx-auto p-4">
        <Card>
          <CardBody className="text-center py-12">
            <IconDisc size={48} className="mx-auto mb-4 text-default-400" />
            <h2 className="text-xl font-semibold mb-2">Album Not Found</h2>
            <p className="text-default-500 mb-4">
              {error || "The album could not be loaded."}
            </p>
            <Button as={Link} to="/libraries" variant="flat">
              Back to Libraries
            </Button>
          </CardBody>
        </Card>
      </div>
    );
  }

  const { album, tracks } = albumData;

  // Check if album is fully complete (100%)
  const isComplete = albumData.completionPercent === 100;

  return (
    <div className="container mx-auto p-4">
      {/* Breadcrumbs */}
      <Breadcrumbs className="mb-4">
        <BreadcrumbItem>
          <Link to="/libraries">Libraries</Link>
        </BreadcrumbItem>
        {library ? (
          <BreadcrumbItem>
            <Link to="/libraries/$libraryId" params={{ libraryId: library.Id }}>
              {library.Name}
            </Link>
          </BreadcrumbItem>
        ) : (
          <BreadcrumbItem>Library</BreadcrumbItem>
        )}
        <BreadcrumbItem>{album.name}</BreadcrumbItem>
      </Breadcrumbs>

      {/* Album Header */}
      <div className="flex flex-col md:flex-row gap-6 mb-6">
        {/* Cover Art with Play Button */}
        <div className="w-64 shrink-0 relative group">
          {album.coverUrl ? (
            <Image
              src={album.coverUrl}
              alt={album.name}
              classNames={{
                wrapper: "w-64 h-64",
                img: "w-full h-full object-cover rounded-lg",
              }}
            />
          ) : (
            <div className="w-64 h-64 bg-content2 rounded-lg flex items-center justify-center">
              <IconDisc size={64} className="text-default-400" />
            </div>
          )}
          {/* Play/Pause Overlay Button - z-10 ensures it appears above HeroUI Image */}
          {playableTracks.length > 0 && (
            <button
              onClick={() => {
                if (isThisAlbumPlaying) {
                  updatePlayback({ isPlaying: false });
                } else {
                  handlePlayAll();
                }
              }}
              className="absolute inset-0 z-10 flex items-center justify-center bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity duration-200 rounded-lg cursor-pointer"
              aria-label={isThisAlbumPlaying ? "Pause" : "Play All"}
            >
              <div
                className={`w-16 h-16 rounded-full ${isThisAlbumPlaying ? "bg-warning" : "bg-primary"} flex items-center justify-center shadow-lg hover:scale-110 transition-transform`}
              >
                {isThisAlbumPlaying ? (
                  <IconPlayerPause size={32} className="text-white" />
                ) : (
                  <IconPlayerPlay size={32} className="text-white ml-1" />
                )}
              </div>
            </button>
          )}
        </div>

        {/* Album Info */}
        <div className="flex-1">
          <div className="flex items-start justify-between gap-4 mb-1">
            <h1 className="text-3xl font-bold">{album.name}</h1>
            <Dropdown>
              <DropdownTrigger>
                <Button
                  isIconOnly
                  size="sm"
                  variant="light"
                  aria-label="Album actions"
                >
                  <IconDotsVertical size={18} />
                </Button>
              </DropdownTrigger>
              <DropdownMenu
                aria-label="Album actions menu"
                onAction={(key) => {
                  if (key === "search") {
                    handleManualHunt();
                  } else if (key === "refresh") {
                    void fetchAlbum();
                  } else if (key === "wanted-on") {
                    void handleSetWantedForAllTracks(true);
                  } else if (key === "wanted-off") {
                    void handleSetWantedForAllTracks(false);
                  } else if (key === "properties") {
                    const firstPlayable = playableTracks[0];
                    if (firstPlayable) {
                      setPropertiesTrack(firstPlayable);
                      onPropertiesOpen();
                    }
                  } else if (key === "delete") {
                    onDeleteOpen();
                  }
                }}
              >
                <DropdownItem
                  key="search"
                  startContent={<IconSearch size={16} />}
                >
                  Search for Album
                </DropdownItem>
                <DropdownItem
                  key="refresh"
                  startContent={<IconRefresh size={16} />}
                >
                  Refresh
                </DropdownItem>
                <DropdownItem
                  key="wanted-on"
                  startContent={<IconCheck size={16} />}
                  isDisabled={
                    tracks.length === 0 || tracks.every((t) => t.track.wanted)
                  }
                >
                  Mark as Wanted
                </DropdownItem>
                <DropdownItem
                  key="wanted-off"
                  startContent={<IconX size={16} />}
                  isDisabled={
                    tracks.length === 0 || tracks.every((t) => !t.track.wanted)
                  }
                >
                  Remove as Wanted
                </DropdownItem>
                {playableTracks.length > 0 ? (
                  <DropdownItem
                    key="properties"
                    startContent={<IconInfoCircle size={16} />}
                  >
                    Properties
                  </DropdownItem>
                ) : null}
                <DropdownItem
                  key="delete"
                  startContent={
                    <IconTrash size={16} className="text-red-400" />
                  }
                  className="text-danger"
                  color="danger"
                >
                  Delete
                </DropdownItem>
              </DropdownMenu>
            </Dropdown>
          </div>

          {/* Artist Name */}
          {albumData.artistName && (
            <p className="text-xl text-default-500 mb-3">
              {albumData.artistName}
            </p>
          )}

          <div className="flex flex-wrap items-center gap-2 text-default-500 mb-4">
            {album.albumType && (
              <Chip size="sm" variant="flat">
                {album.albumType.charAt(0).toUpperCase() +
                  album.albumType.slice(1)}
              </Chip>
            )}
            {album.year && <span>{album.year}</span>}
            {album.label && <span>• {album.label}</span>}
          </div>

          {/* Completion Progress */}
          <div className="mb-4">
            <div className="flex items-center justify-between text-sm mb-1">
              <span className="text-default-500">
                {albumData.tracksWithFiles} of {albumData.trackCount} tracks
              </span>
              <span className="font-medium">
                {albumData.completionPercent.toFixed(0)}%
              </span>
            </div>
            <Progress
              aria-label="Album completion"
              value={albumData.completionPercent}
              color={isComplete ? "success" : "primary"}
              size="sm"
            />
            {albumData.missingTracks > 0 && (
              <p className="text-sm text-warning mt-1">
                {albumData.missingTracks} track
                {albumData.missingTracks !== 1 ? "s" : ""} wanted
              </p>
            )}
          </div>

          {/* Stats */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
            <div>
              <p className="text-xs text-default-400">Tracks</p>
              <p className="text-lg font-semibold">{albumData.trackCount}</p>
            </div>
            <div>
              <p className="text-xs text-default-400">Discs</p>
              <p className="text-lg font-semibold">{album.discCount || 1}</p>
            </div>
            <div>
              <p className="text-xs text-default-400">Duration</p>
              <p className="text-lg font-semibold">
                {totalDurationSecs > 0
                  ? formatDuration(totalDurationSecs)
                  : "-"}
              </p>
            </div>
            <div>
              <p className="text-xs text-default-400">Size</p>
              <p className="text-lg font-semibold">
                {totalSizeBytes > 0 ? formatBytes(totalSizeBytes) : "-"}
              </p>
            </div>
          </div>
        </div>
      </div>

      <TrackTable
        fetchAlbum={fetchAlbum}
        tracks={tracks}
        albumId={albumData.album.id}
        onPlay={handlePlayTrack}
        onSearch={handleSearchTrack}
        onShowProperties={handleShowProperties}
      />

      {/* File Properties Modal */}
      <FilePropertiesModal
        isOpen={isPropertiesOpen}
        onClose={() => {
          onPropertiesClose();
          setPropertiesTrack(null);
        }}
        mediaFileId={propertiesTrack?.track.mediaFileId ?? null}
        title={
          propertiesTrack
            ? `${album.name} - ${propertiesTrack.track.title}`
            : undefined
        }
      />
      <Modal isOpen={isDeleteOpen} onClose={onDeleteClose}>
        <ModalContent>
          <ModalHeader>Delete Album</ModalHeader>
          <ModalBody>
            <p>
              Are you sure you want to delete <strong>{album.name}</strong>?
            </p>
            <p className="text-sm text-default-500 mt-2">
              This will remove the album from the library. Associated files will
              not be deleted.
            </p>
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={onDeleteClose}>
              Cancel
            </Button>
            <Button
              color="danger"
              onPress={handleDeleteAlbum}
              isLoading={isDeleting}
            >
              Delete
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </div>
  );
}
