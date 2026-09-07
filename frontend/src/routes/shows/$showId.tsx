import {
  createFileRoute,
  Link,
  redirect,
  useNavigate,
} from "@tanstack/react-router";
import { useState, useCallback, useMemo } from "react";
import { useQuery, useMutation } from "../../lib/graphql/client";
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
import { Spinner } from "@heroui/spinner";
import { Accordion, AccordionItem } from "@heroui/accordion";
import { Breadcrumbs, BreadcrumbItem } from "@heroui/breadcrumbs";
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
import { sanitizeError, formatBytes, formatDate } from "../../lib/format";
import {
  type DataTableColumn,
  type RowAction,
} from "../../components/data-table";
import {
  IconDeviceTv,
  IconClipboard,
  IconPlayerPlay,
  IconPlayerPause,
  IconPlayerStop,
  IconPlayerTrackNext,
  IconDotsVertical,
  IconRefresh,
  IconSearch,
  IconTrash,
  IconInfoCircle,
  IconCheck,
  IconX,
} from "@tabler/icons-react";
import {
  PlayPauseIndicator,
  MediaItemStatusChip,
} from "../../components/shared";
import { DetailItemsTable } from "../../components/media/DetailItemsTable";
import { usePlaybackContext } from "../../contexts/PlaybackContext";
import { SearchSourcesModal, type SearchSourcesModalProps } from "../../components/SearchSourcesModal";
import { FilePropertiesModal } from "../../components/FilePropertiesModal";
import { ShowSettingsModal, type ShowSettingsInput } from "../../components/shows/ShowSettingsModal";
import {
  DeleteShowRouteDocument,
  LibraryDetailRouteDocument,
  MeDocument,
  RefreshShowRouteDocument,
  ShowDetailRouteDocument,
  UpdateShowSettingsDocument,
  ShowDetailSetEpisodeWantedDocument,
  type ShowDetailRouteQuery,
  ShowPlaybackProgressByMediaDocument,
  ContentStatusType,
  type ContentStatus,
  type ShowPlaybackProgressByMediaQuery,
} from "../../lib/graphql/generated/graphql";
import { useContentStatuses } from "../../hooks/useContentStatuses";

export const Route = createFileRoute("/shows/$showId")({
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
  component: ShowDetailPage,
  errorComponent: RouteError,
});

// Type for episode - using PascalCase to match backend
interface Episode {
  id: string;
  showId: string;
  season: number;
  episode: number;
  absoluteNumber: number | null;
  title: string | null;
  overview: string | null;
  airDate: string | null;
  runtime: number | null;
  tvmazeId: number | null;
  tmdbId: number | null;
  tvdbId: number | null;
  imdbId: string | null;
  mediaFileId: string | null;
  resolution: string | null;
  videoCodec: string | null;
  audioCodec: string | null;
  audioChannels: string | null;
  isHdr: boolean | null;
  hdrType: string | null;
  fileSizeBytes: number | null;
  FileSizeFormatted: string | null;
  WatchProgress: number | null;
  WatchPosition: number | null;
  isWatched: boolean;
  wanted: boolean;
  DownloadProgress: number | null;
  contentStatus?: ContentStatus;
  createdAt: string;
  updatedAt: string;
}

interface SeasonData {
  season: number;
  episodes: Episode[];
  downloadedCount: number;
  totalCount: number;
}
type ShowDetailNode = NonNullable<ShowDetailRouteQuery["show"]>;
type ShowEpisodeNode = ShowDetailNode["episodes"]["edges"][number]["node"];
type PlaybackProgressNode =
  ShowPlaybackProgressByMediaQuery["playbackProgresses"]["edges"][number]["node"];

// Helper functions
function formatAirDate(dateStr: string | null): string {
  return formatDate(dateStr, "TBA");
}

function formatVideoCodec(codec: string | null): string {
  if (!codec) return "";
  const normalized = codec.toLowerCase();
  if (normalized.includes("hevc") || normalized === "h265") return "HEVC";
  if (normalized.includes("h264") || normalized === "avc") return "H.264";
  if (normalized.includes("av1")) return "AV1";
  if (normalized.includes("vp9")) return "VP9";
  return codec.toUpperCase();
}

function formatAudioCodec(
  codec: string | null,
  channels: string | null,
): string {
  if (!codec) return "";
  const normalized = codec.toLowerCase();
  let name = codec.toUpperCase();
  if (normalized.includes("truehd")) name = "TrueHD";
  else if (normalized.includes("atmos")) name = "Atmos";
  else if (normalized.includes("dts")) name = "DTS";
  else if (normalized.includes("aac")) name = "AAC";
  else if (normalized.includes("ac3") || normalized.includes("ac-3"))
    name = "AC3";
  else if (normalized.includes("eac3") || normalized.includes("e-ac-3"))
    name = "EAC3";
  else if (normalized.includes("flac")) name = "FLAC";
  else if (normalized.includes("opus")) name = "Opus";
  if (channels) return `${name} ${channels}`;
  return name;
}

// Episode table columns
const episodeColumns: DataTableColumn<Episode>[] = [
  {
    key: "episode",
    label: "#",
    width: 60,
    sortable: true,
    render: (ep) => (
      <span className="font-mono text-default-500">
        {String(ep.episode).padStart(2, "0")}
      </span>
    ),
  },
  {
    key: "title",
    label: "Title",
    sortable: true,
    render: (ep) => (
      <div className="flex items-center gap-2">
        <span className="font-medium">
          {ep.title || `Episode ${ep.episode}`}
        </span>
        {ep.isWatched && <span className="text-xs text-success">✓</span>}
      </div>
    ),
  },
  {
    key: "progress",
    label: "Progress",
    width: 100,
    render: (ep) => {
      if (!ep.mediaFileId) return <span className="text-default-400">-</span>;
      if (ep.isWatched)
        return <span className="text-success text-sm">Watched</span>;
      if (ep.WatchProgress !== null && ep.WatchProgress > 0) {
        return (
          <div className="flex items-center gap-2">
            <div className="h-1.5 w-16 bg-default-200 rounded-full overflow-hidden">
              <div
                className="h-full bg-primary rounded-full"
                style={{ width: `${Math.min(ep.WatchProgress * 100, 100)}%` }}
              />
            </div>
            <span className="text-xs text-default-400">
              {Math.round(ep.WatchProgress * 100)}%
            </span>
          </div>
        );
      }
      return <span className="text-default-400 text-sm">-</span>;
    },
  },
  {
    key: "airDate",
    label: "Air Date",
    width: 130,
    sortable: true,
    render: (ep) => (
      <span className="text-default-500 text-sm text-nowrap">
        {formatAirDate(ep.airDate)}
      </span>
    ),
  },
  {
    key: "quality",
    label: "Quality",
    width: 220,
    render: (ep) => {
      if (!ep.mediaFileId) return <span className="text-default-400">-</span>;
      return (
        <div className="flex items-center gap-1.5 flex-wrap">
          {ep.resolution && (
            <Chip
              size="sm"
              variant="flat"
              color="primary"
              className="h-5 text-xs"
            >
              {ep.resolution}
            </Chip>
          )}
          {ep.videoCodec && (
            <Chip
              size="sm"
              variant="flat"
              color="secondary"
              className="h-5 text-xs"
            >
              {formatVideoCodec(ep.videoCodec)}
            </Chip>
          )}
          {ep.isHdr && (
            <Chip
              size="sm"
              variant="flat"
              color="warning"
              className="h-5 text-xs"
            >
              {ep.hdrType || "HDR"}
            </Chip>
          )}
        </div>
      );
    },
  },
  {
    key: "audio",
    label: "Audio",
    width: 100,
    render: (ep) => {
      if (!ep.mediaFileId || !ep.audioCodec)
        return <span className="text-default-400">-</span>;
      return (
        <Chip size="sm" variant="flat" color="default" className="h-5 text-xs">
          {formatAudioCodec(ep.audioCodec, ep.audioChannels)}
        </Chip>
      );
    },
  },
  {
    key: "size",
    label: "Size",
    width: 100,
    render: (ep) => {
      if (!ep.mediaFileId || !ep.FileSizeFormatted)
        return <span className="text-default-400">-</span>;
      return (
        <span className="text-default-500 text-sm text-nowrap">
          {ep.FileSizeFormatted}
        </span>
      );
    },
  },
  {
    key: "status",
    label: "Status",
    width: 140,
    sortable: true,
    render: (ep) => (
      <MediaItemStatusChip
        status={ep.contentStatus}
        mediaFileId={ep.mediaFileId}
        downloadProgress={ep.DownloadProgress}
        wanted={ep.wanted}
      />
    ),
  },
];

interface EpisodeTableProps {
  episodes: Episode[];
  seasonNumber: number;
  showId: string;
  onPlay: (episode: Episode, startFromBeginning?: boolean) => void;
  onSearch: (episode: Episode) => void;
  onShowProperties: (episode: Episode) => void;
}

function EpisodeTable({
  episodes,
  seasonNumber,
  showId,
  onPlay,
  onSearch,
  onShowProperties,
}: EpisodeTableProps) {
  const { session, updatePlayback, stopPlayback } = usePlaybackContext();
  const handlePause = useCallback(
    () => updatePlayback({ isPlaying: false }),
    [updatePlayback],
  );
  const currentlyPlayingEpisodeId =
    session?.tvShowId === showId ? session?.episodeId : null;
  const isPlaying = session?.isPlaying ?? false;

  const hasResumeProgress = (ep: Episode) =>
    ep.WatchProgress !== null && ep.WatchProgress > 0 && !ep.isWatched;
  const isCurrentlyPlaying = (ep: Episode) =>
    currentlyPlayingEpisodeId === ep.id;
  const playActionKey = `play-${currentlyPlayingEpisodeId || "none"}-${isPlaying}`;
  const hasFile = (ep: Episode) => !!ep.mediaFileId;
  const isDownloading = (ep: Episode) =>
    ep.contentStatus === "DOWNLOADING" ||
    (!ep.mediaFileId && ep.DownloadProgress != null && ep.DownloadProgress > 0);
  const isWanted = (ep: Episode) =>
    ep.contentStatus === "WANTED" ||
    ep.contentStatus === "UPCOMING" ||
    (!ep.mediaFileId && ep.wanted && !isDownloading(ep));

  const rowActions: RowAction<Episode>[] = [
    {
      key: `playing-${currentlyPlayingEpisodeId || "none"}`,
      label: "Pause",
      alwaysVisible: true,
      icon: (
        <PlayPauseIndicator
          size={16}
          isPlaying={isPlaying}
          colorClass="bg-success"
        />
      ),
      color: "default",
      inDropdown: false,
      isVisible: (ep) => hasFile(ep) && isCurrentlyPlaying(ep) && isPlaying,
      onAction: () => { void handlePause(); },
    },
    {
      key: "resume-current",
      label: "Resume",
      icon: <IconPlayerPlay size={16} />,
      color: "success",
      inDropdown: false,
      alwaysVisible: true,
      isVisible: ep => hasFile(ep) && isCurrentlyPlaying(ep) && !isPlaying,
      onAction: () => { void updatePlayback({ isPlaying: true }); },
    },
    {
      key: "stop-current",
      label: "Stop",
      icon: <IconPlayerStop size={16} />,
      inDropdown: false,
      isVisible: ep => isCurrentlyPlaying(ep),
      onAction: () => { void stopPlayback(); },
    },
    {
      key: `resume-${playActionKey}`,
      label: "Resume",
      icon: <IconPlayerTrackNext size={16} />,
      color: "success",
      inDropdown: false,
      isVisible: (ep) =>
        hasFile(ep) && hasResumeProgress(ep) && !isCurrentlyPlaying(ep),
      onAction: (ep) => onPlay(ep),
    },
    {
      key: `restart-${playActionKey}`,
      label: "Start from beginning",
      icon: <IconPlayerPlay size={16} />,
      color: "default",
      inDropdown: true,
      isVisible: (ep) =>
        hasFile(ep) && hasResumeProgress(ep) && !isCurrentlyPlaying(ep),
      onAction: (ep) => onPlay(ep, true),
    },
    {
      key: `play-${playActionKey}`,
      label: "Play",
      icon: <IconPlayerPlay size={16} />,
      color: "success",
      inDropdown: false,
      isVisible: (ep) =>
        hasFile(ep) && !hasResumeProgress(ep) && !isCurrentlyPlaying(ep),
      onAction: (ep) => onPlay(ep),
    },
    {
      key: "search",
      label: "Search for Episode",
      icon: <IconSearch size={16} />,
      color: "default",
      inDropdown: false,
      isVisible: (ep) => isWanted(ep),
      onAction: (ep) => onSearch(ep),
    },
    {
      key: "properties",
      label: "File Properties",
      icon: <IconInfoCircle size={16} />,
      color: "default",
      inDropdown: true,
      isVisible: (ep) => hasFile(ep),
      onAction: (ep) => onShowProperties(ep),
    },
  ];

  const selectedKeys = useMemo(() => {
    if (currentlyPlayingEpisodeId) return new Set([currentlyPlayingEpisodeId]);
    return new Set<string>();
  }, [currentlyPlayingEpisodeId]);

  const tableKey = `episodes-${currentlyPlayingEpisodeId || "none"}-${isPlaying}`;

  return (
    <DetailItemsTable
      tableKey={tableKey}
      data={episodes}
      columns={episodeColumns}
      getRowKey={(ep) => ep.id}
      ariaLabel={`Season ${seasonNumber} episodes`}
      removeWrapper
      isCompact
      hideToolbar
      defaultSortColumn="episode"
      defaultSortDirection="asc"
      rowActions={rowActions}
      selectionMode={currentlyPlayingEpisodeId ? "single" : "none"}
      selectedKeys={selectedKeys}
    />
  );
}

function ShowDetailPage() {
  const [isShowSettingsOpen, setShowSettingsOpen] = useState(false);
  const [sourceSearch, setSourceSearch] = useState<Pick<SearchSourcesModalProps, "initialSearch" | "title"> | null>(null);
  const navigate = useNavigate();
  const { showId } = Route.useParams();
  const { startEpisodePlayback, session, updatePlayback } =
    usePlaybackContext();

  const {
    isOpen: isDeleteOpen,
    onOpen: onDeleteOpen,
    onClose: onDeleteClose,
  } = useDisclosure();
  const {
    isOpen: isPropertiesOpen,
    onOpen: onPropertiesOpen,
    onClose: onPropertiesClose,
  } = useDisclosure();
  const [propertiesEpisode, setPropertiesEpisode] = useState<Episode | null>(
    null,
  );

  // Query show data
  const {
    data: showData,
    previousData: previousShowData,
    loading: showLoading,
    refetch,
  } = useQuery(ShowDetailRouteDocument, {
    variables: { id: showId },
    fetchPolicy: "cache-and-network",
  });
  const show = showData?.show ?? previousShowData?.show;

  const rawEpisodes = useMemo<Episode[]>(() => {
    const edges = show?.episodes?.edges ?? [];
    return edges.map(({ node: ep }: { node: ShowEpisodeNode }) => {
      const mediaFile = ep.mediaFile ?? null;
      return {
        id: ep.id,
        showId: ep.showId,
        season: ep.season,
        episode: ep.episode,
        absoluteNumber: ep.absoluteNumber ?? null,
        title: ep.title ?? null,
        overview: ep.overview ?? null,
        airDate: ep.airDate ?? null,
        runtime: ep.runtime ?? mediaFile?.duration ?? null,
        tvmazeId: ep.tvmazeId ?? null,
        tmdbId: ep.tmdbId ?? null,
        tvdbId: ep.tvdbId ?? null,
        imdbId: null,
        mediaFileId: ep.mediaFileId ?? null,
        resolution: mediaFile?.resolution ?? null,
        videoCodec: mediaFile?.videoCodec ?? null,
        audioCodec: mediaFile?.audioCodec ?? null,
        audioChannels: mediaFile?.audioChannels ?? null,
        isHdr: mediaFile?.isHdr ?? null,
        hdrType: mediaFile?.hdrType ?? null,
        fileSizeBytes: mediaFile?.size ?? null,
        FileSizeFormatted: mediaFile?.size ? formatBytes(mediaFile.size) : null,
        WatchProgress: null,
        WatchPosition: null,
        isWatched: false,
        wanted: ep.wanted,
        DownloadProgress: null,
        createdAt: ep.createdAt,
        updatedAt: ep.updatedAt,
      };
    });
  }, [show]);
  const episodeStatusTargets = useMemo(
    () =>
      rawEpisodes.map((episode) => ({
        contentType: ContentStatusType.EPISODE,
        id: episode.id,
      })),
    [rawEpisodes],
  );
  const { getStatus: getEpisodeStatus } =
    useContentStatuses(episodeStatusTargets);

  const { data: meData } = useQuery(MeDocument, {
    fetchPolicy: "cache-first",
  });
  const userId = meData?.me?.id;
  const episodeMediaFileIds = useMemo(
    () => [
      ...new Set(
        rawEpisodes
          .map((ep) => ep.mediaFileId)
          .filter((id): id is string => Boolean(id)),
      ),
    ],
    [rawEpisodes],
  );
  const { data: progressData, previousData: previousProgressData } = useQuery(
    ShowPlaybackProgressByMediaDocument,
    {
      variables: {
        where: {
          userId: { eq: userId },
          mediaFileId: { inList: episodeMediaFileIds },
        },
        page: { limit: 5000, offset: 0 },
        orderBy: [{ updatedAt: "DESC" }],
      },
      skip: !userId || episodeMediaFileIds.length === 0,
      fetchPolicy: "cache-and-network",
    },
  );
  const progressEdges =
    progressData?.playbackProgresses?.edges ??
    previousProgressData?.playbackProgresses?.edges ??
    [];
  const progressByMediaFile = useMemo(() => {
    const map = new Map<string, PlaybackProgressNode>();
    for (const edge of progressEdges) {
      const node = edge.node;
      if (!node) continue;
      if (!node.mediaFileId) continue;
      if (!map.has(node.mediaFileId)) {
        map.set(node.mediaFileId, node);
      }
    }
    return map;
  }, [progressEdges]);

  const episodes = useMemo<Episode[]>(() => {
    return rawEpisodes.map((ep) => {
      const progress = ep.mediaFileId
        ? progressByMediaFile.get(ep.mediaFileId)
        : undefined;
      return {
        ...ep,
        contentStatus: getEpisodeStatus(ContentStatusType.EPISODE, ep.id),
        WatchProgress: progress
          ? Math.max(0, Math.min(1, progress.progressPercent))
          : ep.WatchProgress,
        WatchPosition: progress?.currentPosition ?? ep.WatchPosition,
        isWatched: progress?.isWatched ?? ep.isWatched,
      };
    });
  }, [getEpisodeStatus, progressByMediaFile, rawEpisodes]);

  // Query library
  const { data: libraryData, previousData: previousLibraryData } = useQuery(
    LibraryDetailRouteDocument,
    {
      variables: { id: show?.libraryId || "" },
      skip: !show?.libraryId,
      fetchPolicy: "cache-and-network",
    },
  );
  const library = libraryData?.library ?? previousLibraryData?.library;

  // Mutations
  const [refreshShow, { loading: refreshing }] = useMutation(
    RefreshShowRouteDocument,
  );
  const [setEpisodesWanted] = useMutation(ShowDetailSetEpisodeWantedDocument);
  const [updateShowSettings, { loading: savingShowSettings }] = useMutation(UpdateShowSettingsDocument);
  const handleSaveShowSettings = async (input: ShowSettingsInput) => {
    try {
      const { data } = await updateShowSettings({ variables: { id: showId, input } });
      if (!data?.updateShow.success) throw new Error(data?.updateShow.error ?? "Failed to save show settings");
      setShowSettingsOpen(false);
      addToast({ title: "Show settings saved", color: "success" });
    } catch (error) {
      addToast({ title: "Could not save show settings", description: sanitizeError(error), color: "danger" });
    }
  };
  const [deleteShow, { loading: deleting }] = useMutation(
    DeleteShowRouteDocument,
  );

  const handleRefresh = async () => {
    try {
      const { data } = await refreshShow({ variables: { id: showId } });
      if (!data?.refreshShow?.success) {
        addToast({
          title: "Error",
          description: sanitizeError(
            data?.refreshShow?.error || "Failed to refresh show",
          ),
          color: "danger",
        });
        return;
      }
      addToast({
        title: "Refreshed",
        description: "Show metadata updated",
        color: "success",
      });
      await refetch();
    } catch (err) {
      console.error("Failed to refresh show:", err);
      addToast({
        title: "Error",
        description: "Failed to refresh show",
        color: "danger",
      });
    }
  };

  const handleDelete = async () => {
    try {
      const { data } = await deleteShow({ variables: { id: showId } });
      if (!data?.deleteShow?.success) {
        addToast({
          title: "Error",
          description: sanitizeError(
            data?.deleteShow?.error || "Failed to delete show",
          ),
          color: "danger",
        });
        return;
      }
      addToast({
        title: "Deleted",
        description: "Show has been removed from library",
        color: "success",
      });
      onDeleteClose();
      navigate({
        to: "/libraries/$libraryId",
        params: { libraryId: show?.libraryId || "" },
      });
    } catch (err) {
      console.error("Failed to delete show:", err);
      addToast({
        title: "Error",
        description: "Failed to delete show",
        color: "danger",
      });
    }
  };

  const handlePlay = useCallback(
    async (episode: Episode, startFromBeginning = false) => {
      if (episode.mediaFileId && show) {
        let startPosition = 0;
        if (
          !startFromBeginning &&
          episode.WatchPosition &&
          episode.WatchProgress !== null
        ) {
          if (!episode.isWatched && episode.WatchProgress > 0) {
            startPosition = episode.WatchPosition;
          }
        }
        await startEpisodePlayback(
          episode.id,
          episode.mediaFileId,
          show.id,
          episode as any,
          show as any,
          startPosition,
        );
      }
    },
    [show, startEpisodePlayback],
  );

  const handleSearchEpisode = useCallback(
    (episode: Episode) => {
      if (!show) return;
      setSourceSearch({
        title: `Search for ${show.name} S${String(episode.season).padStart(2, "0")}E${String(episode.episode).padStart(2, "0")}`,
        initialSearch: { query: show.name, season: episode.season, episode: String(episode.episode), categories: [5000] },
      });
    },
    [show],
  );

  const handleShowProperties = useCallback(
    (episode: Episode) => {
      setPropertiesEpisode(episode);
      onPropertiesOpen();
    },
    [onPropertiesOpen],
  );

  const handleSetWantedForAllEpisodes = useCallback(
    async (wanted: boolean) => {
      if (episodes.length === 0) {
        addToast({
          title: "No episodes",
          description: "No episodes found to update",
          color: "warning",
        });
        return;
      }

      try {
        const { data } = await setEpisodesWanted({
          variables: { showId: showId, wanted: wanted },
        });
        if (!data?.updateEpisodes?.success) {
          addToast({
            title: "Error",
            description:
              data?.updateEpisodes?.error ||
              "Failed to update wanted status for episodes",
            color: "danger",
          });
          return;
        }

        addToast({
          title: wanted ? "Marked as wanted" : "Removed wanted",
          description: wanted
            ? `${data.updateEpisodes.affectedCount} episodes marked as wanted`
            : `${data.updateEpisodes.affectedCount} episodes removed from wanted`,
          color: "success",
        });

        await refetch();
      } catch (err) {
        console.error("Failed to update episode wanted state:", err);
        addToast({
          title: "Error",
          description: "Failed to update wanted status",
          color: "danger",
        });
      }
    },
    [episodes.length, refetch, setEpisodesWanted, showId],
  );

  // Group episodes by season
  const seasons = useMemo<SeasonData[]>(() => {
    const seasonMap = new Map<number, Episode[]>();
    for (const ep of episodes) {
      if (!seasonMap.has(ep.season)) seasonMap.set(ep.season, []);
      seasonMap.get(ep.season)!.push(ep);
    }
    return Array.from(seasonMap.entries())
      .map(([season, eps]) => ({
        season,
        episodes: eps.sort((a, b) => a.episode - b.episode),
        downloadedCount: eps.filter((e) => !!e.mediaFileId).length,
        totalCount: eps.length,
      }))
      .sort((a, b) => a.season - b.season);
  }, [episodes]);

  const { totalEpisodes, downloadedEpisodes, missingEpisodes, totalSizeBytes } =
    useMemo(
      () => ({
        totalEpisodes: episodes.length,
        downloadedEpisodes: episodes.filter((e: Episode) => !!e.mediaFileId)
          .length,
        missingEpisodes: episodes.filter((e: Episode) => !e.mediaFileId).length,
        totalSizeBytes: episodes.reduce(
          (sum: number, e: Episode) => sum + (e.fileSizeBytes || 0),
          0,
        ),
      }),
      [episodes],
    );

  const playableEpisodes = useMemo(
    () =>
      [...episodes]
        .filter((e) => !!e.mediaFileId)
        .sort((a, b) =>
          a.season === b.season ? a.episode - b.episode : a.season - b.season,
        ),
    [episodes],
  );

  const isThisShowPlaying =
    session?.tvShowId === showId && !!session?.isPlaying;
  const defaultEpisodeToPlay = useMemo(() => {
    const resumable = playableEpisodes.find(
      (episode) =>
        episode.WatchProgress !== null &&
        episode.WatchProgress > 0 &&
        !episode.isWatched,
    );
    return resumable ?? playableEpisodes[0];
  }, [playableEpisodes]);

  // Loading state
  if (showLoading && !show) {
    return (
      <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex items-center justify-center min-h-[50vh]">
        <Spinner size="lg" />
      </div>
    );
  }

  // Not found state
  if (!show) {
    return (
      <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <Card className="bg-content1">
          <CardBody className="text-center py-12">
            <h2 className="text-xl font-semibold mb-4">Show not found</h2>
            <Link to="/libraries">
              <Button color="primary">Back to Libraries</Button>
            </Link>
          </CardBody>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col mb-20">
      {/* Header */}
      <div className="flex flex-col md:flex-row gap-6 mb-8">
        <div className="shrink-0 relative group">
          {show.posterUrl ? (
            <Image
              src={show.posterUrl}
              alt={show.name}
              className="w-48 h-72 object-cover rounded-lg shadow-lg"
            />
          ) : (
            <div className="w-48 h-72 bg-default-200 rounded-lg flex items-center justify-center">
              <IconDeviceTv size={64} className="text-blue-400" />
            </div>
          )}
          {playableEpisodes.length > 0 && (
            <button
              onClick={() => {
                if (isThisShowPlaying) {
                  updatePlayback({ isPlaying: false });
                } else {
                  if (defaultEpisodeToPlay) {
                    void handlePlay(defaultEpisodeToPlay);
                  }
                }
              }}
              className="absolute inset-0 z-10 flex items-center justify-center bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity duration-200 rounded-lg cursor-pointer"
              aria-label={isThisShowPlaying ? "Pause Show" : "Play Show"}
            >
              <div
                className={`w-16 h-16 rounded-full ${isThisShowPlaying ? "bg-warning" : "bg-primary"} flex items-center justify-center shadow-lg hover:scale-110 transition-transform`}
              >
                {isThisShowPlaying ? (
                  <IconPlayerPause size={32} className="text-white" />
                ) : (
                  <IconPlayerPlay size={32} className="text-white ml-1" />
                )}
              </div>
            </button>
          )}
        </div>

        <div className="flex-1">
          <Breadcrumbs className="mb-2">
            <BreadcrumbItem><Link to="/libraries">Libraries</Link></BreadcrumbItem>
            <BreadcrumbItem>
              <Link to="/libraries/$libraryId" params={{ libraryId: show.libraryId }}>{library?.name ?? "Library"}</Link>
            </BreadcrumbItem>
            <BreadcrumbItem isCurrent>{show.name}</BreadcrumbItem>
          </Breadcrumbs>

          <div className="flex items-start justify-between gap-4 mb-2">
            <h1 className="text-3xl font-bold">
              {show.name}
              {show.year && (
                <span className="text-default-500 ml-2">({show.year})</span>
              )}
            </h1>
            <Dropdown>
              <DropdownTrigger>
                <Button
                  isIconOnly
                  size="sm"
                  variant="light"
                  aria-label="Show actions"
                >
                  <IconDotsVertical size={18} />
                </Button>
              </DropdownTrigger>
              <DropdownMenu
                aria-label="Show actions menu"
                onAction={(key) => {
                  if (key === "search") {
                    setSourceSearch({ title: `Search for ${show.name}`, initialSearch: { query: show.name, categories: [5000] } });
                  } else if (key === "refresh") {
                    void handleRefresh();
                  } else if (key === "wanted-on") {
                    void handleSetWantedForAllEpisodes(true);
                  } else if (key === "wanted-off") {
                    void handleSetWantedForAllEpisodes(false);
                  } else if (key === "properties") {
                    setShowSettingsOpen(true);
                  } else if (key === "delete") {
                    onDeleteOpen();
                  }
                }}
              >
                <DropdownItem
                  key="search"
                  startContent={<IconSearch size={16} />}
                >
                  Search for Show
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
                    episodes.length === 0 || episodes.every((ep) => ep.wanted)
                  }
                >
                  Mark as Wanted
                </DropdownItem>
                <DropdownItem
                  key="wanted-off"
                  startContent={<IconX size={16} />}
                  isDisabled={
                    episodes.length === 0 || episodes.every((ep) => !ep.wanted)
                  }
                >
                  Remove as Wanted
                </DropdownItem>
                  <DropdownItem
                    key="properties"
                    startContent={<IconInfoCircle size={16} />}
                  >
                    Properties
                  </DropdownItem>
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

          <div className="flex flex-wrap gap-2 mb-4">
            {show.network && (
              <Chip size="sm" variant="flat">
                {show.network}
              </Chip>
            )}
            {show.year && (
              <Chip size="sm" variant="flat">
                {show.year}
              </Chip>
            )}
          </div>

          {show.overview && (
            <p className="text-default-600 mb-4 line-clamp-3">
              {show.overview}
            </p>
          )}

          <div className="flex gap-4 text-sm text-default-500 mb-4">
            <div>
              <span className="font-semibold text-foreground">
                {downloadedEpisodes}
              </span>
              <span> / {totalEpisodes} episodes</span>
            </div>
            {missingEpisodes > 0 && (
              <div className="text-warning">
                <span className="font-semibold">{missingEpisodes}</span>
                <span> missing</span>
              </div>
            )}
            {totalSizeBytes > 0 && (
              <div>
                <span className="font-semibold text-foreground">
                  {formatBytes(totalSizeBytes)}
                </span>
                <span> on disk</span>
              </div>
            )}
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Chip
              size="sm"
              variant="flat"
              color={show.autoDownload ? "success" : "default"}
            >
              {show.autoDownload ? "Auto Download: On" : "Auto Download: Off"}
            </Chip>
            <Chip size="sm" variant="flat">
              Mode: {show.autoDownloadMode}
            </Chip>
          </div>
        </div>
      </div>

      {/* Seasons */}
      <div className="space-y-4">
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
          <h2 className="text-xl font-semibold">Seasons & Episodes</h2>
          {seasons.length > 0 && (
            <div className="flex flex-wrap gap-2">
              {seasons.map((s) => (
                <Button
                  key={s.season}
                  size="sm"
                  variant="flat"
                  onPress={() =>
                    document
                      .getElementById(`season-${s.season}`)
                      ?.scrollIntoView({ behavior: "smooth", block: "start" })
                  }
                >
                  {s.season === 0 ? "Specials" : `S${s.season}`}
                </Button>
              ))}
            </div>
          )}
        </div>

        {seasons.length === 0 ? (
          <Card className="bg-content1/50 border-default-300 border-dashed border-2">
            <CardBody className="py-12 text-center">
              <IconClipboard
                size={48}
                className="mx-auto mb-4 text-default-400"
              />
              <h3 className="text-lg font-semibold mb-2">No episodes found</h3>
              <p className="text-default-500 mb-4">
                Try refreshing the show metadata to fetch episodes.
              </p>
              <Button
                color="primary"
                onPress={handleRefresh}
                isLoading={refreshing}
              >
                Refresh Metadata
              </Button>
            </CardBody>
          </Card>
        ) : (
          <Accordion
            variant="splitted"
            selectionMode="multiple"
            defaultExpandedKeys={
              seasons.length <= 3 ? seasons.map((s) => String(s.season)) : []
            }
          >
            {seasons.map((seasonData) => (
              <AccordionItem
                key={String(seasonData.season)}
                aria-label={`Season ${seasonData.season}`}
                id={`season-${seasonData.season}`}
                title={
                  <div className="flex items-center justify-between w-full pr-4">
                    <span className="font-semibold">
                      {seasonData.season === 0
                        ? "Specials"
                        : `Season ${seasonData.season}`}
                    </span>
                    <div className="flex items-center gap-2">
                      <Chip
                        size="sm"
                        color={
                          seasonData.downloadedCount === seasonData.totalCount
                            ? "success"
                            : "warning"
                        }
                        variant="flat"
                      >
                        {seasonData.downloadedCount} / {seasonData.totalCount}
                      </Chip>
                    </div>
                  </div>
                }
                className="bg-content1"
              >
                <EpisodeTable
                  episodes={seasonData.episodes}
                  seasonNumber={seasonData.season}
                  showId={show.id}
                  onPlay={handlePlay}
                  onSearch={handleSearchEpisode}
                  onShowProperties={handleShowProperties}
                />
              </AccordionItem>
            ))}
          </Accordion>
        )}
      </div>

      {/* Delete Confirmation Modal */}
      {show && (
        <Modal isOpen={isDeleteOpen} onClose={onDeleteClose}>
          <ModalContent>
            <ModalHeader>Delete Show</ModalHeader>
            <ModalBody>
              <p>
                Are you sure you want to delete <strong>{show.name}</strong>?
              </p>
              <p className="text-sm text-default-500 mt-2">
                This will remove the show from the library. Associated files
                will not be deleted.
              </p>
            </ModalBody>
            <ModalFooter>
              <Button variant="flat" onPress={onDeleteClose}>
                Cancel
              </Button>
              <Button
                color="danger"
                onPress={handleDelete}
                isLoading={deleting}
              >
                Delete
              </Button>
            </ModalFooter>
          </ModalContent>
        </Modal>
      )}

      <SearchSourcesModal
        isOpen={sourceSearch !== null}
        onClose={() => setSourceSearch(null)}
        {...sourceSearch}
        libraryId={show.libraryId}
        showId={show.id}
      />

      {/* File Properties Modal */}
      {isShowSettingsOpen && (
        <ShowSettingsModal isOpen onClose={() => setShowSettingsOpen(false)}
          show={show} onSave={handleSaveShowSettings} isLoading={savingShowSettings} />
      )}
      <FilePropertiesModal
        isOpen={isPropertiesOpen}
        onClose={() => {
          onPropertiesClose();
          setPropertiesEpisode(null);
        }}
        mediaFileId={propertiesEpisode?.mediaFileId ?? null}
        title={
          propertiesEpisode
            ? `${show.name} - S${String(propertiesEpisode.season).padStart(2, "0")}E${String(propertiesEpisode.episode).padStart(2, "0")}${propertiesEpisode.title ? ` - ${propertiesEpisode.title}` : ""}`
            : undefined
        }
      />
    </div>
  );
}
