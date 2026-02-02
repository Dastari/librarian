import { createFileRoute, Link, redirect, useNavigate } from '@tanstack/react-router'
import { useState, useCallback, useMemo } from "react";
import { useQuery, useMutation, gql } from "../../lib/graphql/client";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { Chip } from "@heroui/chip";
import { Image } from "@heroui/image";
import { Spinner } from "@heroui/spinner";
import { Accordion, AccordionItem } from '@heroui/accordion'
import { Breadcrumbs, BreadcrumbItem } from '@heroui/breadcrumbs'
import { useDisclosure } from '@heroui/modal'
import { addToast } from "@heroui/toast";
import { RouteError } from "../../components/RouteError";
import { sanitizeError, formatBytes, formatDate } from "../../lib/format";
import { DataTable, type DataTableColumn, type RowAction } from '../../components/data-table'
import {
  IconDeviceTv,
  IconClipboard,
  IconPlayerPlay,
  IconPlayerTrackNext,
  IconRefresh,
  IconSearch,
  IconTrash,
  IconInfoCircle,
} from "@tabler/icons-react";
import { Tooltip } from "@heroui/tooltip";
import {
  PlayPauseIndicator,
  MediaItemStatusChip,
} from "../../components/shared";
import { usePlaybackContext } from "../../contexts/PlaybackContext";
import { FilePropertiesModal } from "../../components/FilePropertiesModal";
import type { Show, Library } from "../../lib/graphql/generated/graphql";
import {
  TV_SHOW_QUERY,
  LIBRARY_QUERY,
  EPISODES_QUERY,
} from "../../lib/graphql/queries";
import {
  REFRESH_TV_SHOW_MUTATION,
  DELETE_TV_SHOW_MUTATION,
} from "../../lib/graphql/mutations";

export const Route = createFileRoute('/shows/$showId')({
  beforeLoad: ({ context, location }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({
        to: '/',
        search: {
          signin: true,
          redirect: location.href,
        },
      })
    }
  },
  component: ShowDetailPage,
  errorComponent: RouteError,
})

// Type for episode - using PascalCase to match backend
interface Episode {
  Id: string
  ShowId: string
  Season: number
  Episode: number
  AbsoluteNumber: number | null
  Title: string | null
  Overview: string | null
  AirDate: string | null
  Runtime: number | null
  TvmazeId: number | null
  TmdbId: number | null
  TvdbId: number | null
  ImdbId: string | null
  MediaFileId: string | null
  Resolution: string | null
  VideoCodec: string | null
  AudioCodec: string | null
  AudioChannels: string | null
  IsHdr: boolean | null
  HdrType: string | null
  FileSizeBytes: number | null
  FileSizeFormatted: string | null
  WatchProgress: number | null
  WatchPosition: number | null
  IsWatched: boolean
  DownloadProgress: number | null
  CreatedAt: string
  UpdatedAt: string
}

interface SeasonData {
  season: number
  episodes: Episode[]
  downloadedCount: number
  totalCount: number
}

// GraphQL queries
const SHOW_QUERY = gql`${TV_SHOW_QUERY}`
const LIBRARY_GQL = gql`${LIBRARY_QUERY}`
const EPISODES = gql`${EPISODES_QUERY}`
const REFRESH_SHOW = gql`${REFRESH_TV_SHOW_MUTATION}`
const DELETE_SHOW = gql`${DELETE_TV_SHOW_MUTATION}`

// Helper functions
function formatAirDate(dateStr: string | null): string {
  return formatDate(dateStr, 'TBA')
}

function formatVideoCodec(codec: string | null): string {
  if (!codec) return ''
  const normalized = codec.toLowerCase()
  if (normalized.includes('hevc') || normalized === 'h265') return 'HEVC'
  if (normalized.includes('h264') || normalized === 'avc') return 'H.264'
  if (normalized.includes('av1')) return 'AV1'
  if (normalized.includes('vp9')) return 'VP9'
  return codec.toUpperCase()
}

function formatAudioCodec(
  codec: string | null,
  channels: string | null
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
        {String(ep.Episode).padStart(2, "0")}
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
          {ep.Title || `Episode ${ep.Episode}`}
        </span>
        {ep.IsWatched && <span className="text-xs text-success">✓</span>}
      </div>
    ),
  },
  {
    key: "progress",
    label: "Progress",
    width: 100,
    render: (ep) => {
      if (!ep.MediaFileId) return <span className="text-default-400">-</span>;
      if (ep.IsWatched)
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
        {formatAirDate(ep.AirDate)}
      </span>
    ),
  },
  {
    key: "quality",
    label: "Quality",
    width: 220,
    render: (ep) => {
      if (!ep.MediaFileId) return <span className="text-default-400">-</span>;
      return (
        <div className="flex items-center gap-1.5 flex-wrap">
          {ep.Resolution && (
            <Chip
              size="sm"
              variant="flat"
              color="primary"
              className="h-5 text-xs"
            >
              {ep.Resolution}
            </Chip>
          )}
          {ep.VideoCodec && (
            <Chip
              size="sm"
              variant="flat"
              color="secondary"
              className="h-5 text-xs"
            >
              {formatVideoCodec(ep.VideoCodec)}
            </Chip>
          )}
          {ep.IsHdr && (
            <Chip
              size="sm"
              variant="flat"
              color="warning"
              className="h-5 text-xs"
            >
              {ep.HdrType || "HDR"}
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
      if (!ep.MediaFileId || !ep.AudioCodec)
        return <span className="text-default-400">-</span>;
      return (
        <Chip size="sm" variant="flat" color="default" className="h-5 text-xs">
          {formatAudioCodec(ep.AudioCodec, ep.AudioChannels)}
        </Chip>
      );
    },
  },
  {
    key: "size",
    label: "Size",
    width: 100,
    render: (ep) => {
      if (!ep.MediaFileId || !ep.FileSizeFormatted)
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
        mediaFileId={ep.MediaFileId}
        downloadProgress={ep.DownloadProgress}
      />
    ),
  },
];

interface EpisodeTableProps {
  episodes: Episode[];
  seasonNumber: number;
  showId: string;
  onPlay: (episode: Episode) => void;
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
  const { session, updatePlayback } = usePlaybackContext();
  const handlePause = useCallback(
    () => updatePlayback({ isPlaying: false }),
    [updatePlayback]
  );
  const currentlyPlayingEpisodeId =
    session?.tvShowId === showId ? session?.episodeId : null;
  const isPlaying = session?.isPlaying ?? false;

  const hasResumeProgress = (ep: Episode) =>
    ep.WatchProgress !== null && ep.WatchProgress > 0 && !ep.IsWatched;
  const isCurrentlyPlaying = (ep: Episode) =>
    currentlyPlayingEpisodeId === ep.Id;
  const playActionKey = `play-${currentlyPlayingEpisodeId || "none"}-${isPlaying}`;
  const hasFile = (ep: Episode) => !!ep.MediaFileId;
  const isDownloading = (ep: Episode) =>
    !ep.MediaFileId && ep.DownloadProgress != null && ep.DownloadProgress > 0;
  const isWanted = (ep: Episode) => !ep.MediaFileId && !isDownloading(ep);

  const rowActions: RowAction<Episode>[] = [
    {
      key: `playing-${currentlyPlayingEpisodeId || "none"}`,
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
      isVisible: (ep) => hasFile(ep) && isCurrentlyPlaying(ep) && isPlaying,
      onAction: () => handlePause(),
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
    <DataTable
      key={tableKey}
      skeletonDelay={500}
      data={episodes}
      columns={episodeColumns}
      getRowKey={(ep) => ep.Id}
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
  const navigate = useNavigate();
  const { showId } = Route.useParams();
  const { startEpisodePlayback } = usePlaybackContext();

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
    null
  );

  // Query show data
  const {
    data: showData,
    previousData: previousShowData,
    loading: showLoading,
    refetch,
  } = useQuery<{ Show: Show | null }>(SHOW_QUERY, {
    variables: { Id: showId },
    fetchPolicy: "cache-and-network",
  });
  const show = showData?.Show ?? previousShowData?.Show;

  // Query episodes
  const { data: episodesData, previousData: previousEpisodesData } = useQuery<{
    episodes: Episode[];
  }>(EPISODES, {
    variables: { tvShowId: showId },
    skip: !show,
    fetchPolicy: "cache-and-network",
  });
  const episodes =
    episodesData?.episodes ?? previousEpisodesData?.episodes ?? [];

  // Query library
  const { data: libraryData, previousData: previousLibraryData } = useQuery<{
    Library: Library | null;
  }>(LIBRARY_GQL, {
    variables: { Id: show?.LibraryId || "" },
    skip: !show?.LibraryId,
    fetchPolicy: "cache-and-network",
  });
  const library = libraryData?.Library ?? previousLibraryData?.Library;

  // Mutations
  const [refreshShow, { loading: refreshing }] = useMutation<{
    refreshTvShow: { success: boolean; error: string | null };
  }>(REFRESH_SHOW);
  const [deleteShow, { loading: deleting }] = useMutation<{
    deleteTvShow: { success: boolean; error: string | null };
  }>(DELETE_SHOW);

  const handleRefresh = async () => {
    try {
      const { data } = await refreshShow({ variables: { id: showId } });
      if (!data?.refreshTvShow?.success) {
        addToast({
          title: "Error",
          description: sanitizeError(
            data?.refreshTvShow?.error || "Failed to refresh show"
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
      if (!data?.deleteTvShow?.success) {
        addToast({
          title: "Error",
          description: sanitizeError(
            data?.deleteTvShow?.error || "Failed to delete show"
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
        params: { libraryId: show?.LibraryId || "" },
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
    async (episode: Episode) => {
      if (episode.MediaFileId && show) {
        let startPosition = 0;
        if (episode.WatchPosition && episode.WatchProgress !== null) {
          if (!episode.IsWatched && episode.WatchProgress > 0) {
            startPosition = episode.WatchPosition;
          }
        }
        await startEpisodePlayback(
          episode.Id,
          episode.MediaFileId,
          show.Id,
          episode as any,
          show as any,
          startPosition
        );
      }
    },
    [show, startEpisodePlayback]
  );

  const handleSearchEpisode = useCallback(
    (episode: Episode) => {
      if (!show) return;
      const seasonPadded = String(episode.Season).padStart(2, "0");
      const episodePadded = String(episode.Episode).padStart(2, "0");
      const searchQuery = `${show.Name} S${seasonPadded}E${episodePadded}`;
      navigate({ to: "/hunt", search: { q: searchQuery, type: "tv" } });
    },
    [show, navigate]
  );

  const handleShowProperties = useCallback(
    (episode: Episode) => {
      setPropertiesEpisode(episode);
      onPropertiesOpen();
    },
    [onPropertiesOpen]
  );

  // Group episodes by season
  const seasons = useMemo<SeasonData[]>(() => {
    const seasonMap = new Map<number, Episode[]>();
    for (const ep of episodes) {
      if (!seasonMap.has(ep.Season)) seasonMap.set(ep.Season, []);
      seasonMap.get(ep.Season)!.push(ep);
    }
    return Array.from(seasonMap.entries())
      .map(([season, eps]) => ({
        season,
        episodes: eps.sort((a, b) => a.Episode - b.Episode),
        downloadedCount: eps.filter((e) => !!e.MediaFileId).length,
        totalCount: eps.length,
      }))
      .sort((a, b) => a.season - b.season);
  }, [episodes]);

  const { totalEpisodes, downloadedEpisodes, missingEpisodes, totalSizeBytes } =
    useMemo(
      () => ({
        totalEpisodes: episodes.length,
        downloadedEpisodes: episodes.filter((e: Episode) => !!e.MediaFileId)
          .length,
        missingEpisodes: episodes.filter((e: Episode) => !e.MediaFileId).length,
        totalSizeBytes: episodes.reduce(
          (sum: number, e: Episode) => sum + (e.FileSizeBytes || 0),
          0
        ),
      }),
      [episodes]
    );

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
        <div className="shrink-0">
          {show.PosterUrl ? (
            <Image
              src={show.PosterUrl}
              alt={show.Name}
              className="w-48 h-72 object-cover rounded-lg shadow-lg"
            />
          ) : (
            <div className="w-48 h-72 bg-default-200 rounded-lg flex items-center justify-center">
              <IconDeviceTv size={64} className="text-blue-400" />
            </div>
          )}
        </div>

        <div className="flex-1">
          <Breadcrumbs className="mb-2">
            <BreadcrumbItem href="/libraries">Libraries</BreadcrumbItem>
            <BreadcrumbItem href={`/libraries/${show.LibraryId}`}>
              {library?.Name ?? "Library"}
            </BreadcrumbItem>
            <BreadcrumbItem isCurrent>{show.Name}</BreadcrumbItem>
          </Breadcrumbs>

          <div className="flex items-start justify-between gap-4 mb-2">
            <h1 className="text-3xl font-bold">
              {show.Name}
              {show.Year && (
                <span className="text-default-500 ml-2">({show.Year})</span>
              )}
            </h1>
            <div className="flex items-center gap-1 shrink-0">
              <Tooltip content="Refresh Metadata">
                <Button
                  isIconOnly
                  variant="light"
                  size="sm"
                  onPress={handleRefresh}
                  isLoading={refreshing}
                >
                  <IconRefresh size={18} />
                </Button>
              </Tooltip>
              <Tooltip content="Delete Show">
                <Button
                  isIconOnly
                  variant="light"
                  size="sm"
                  color="danger"
                  onPress={onDeleteOpen}
                >
                  <IconTrash size={18} />
                </Button>
              </Tooltip>
            </div>
          </div>

          <div className="flex flex-wrap gap-2 mb-4">
            {show.Network && (
              <Chip size="sm" variant="flat">
                {show.Network}
              </Chip>
            )}
            {show.Year && (
              <Chip size="sm" variant="flat">
                {show.Year}
              </Chip>
            )}
          </div>

          {show.Overview && (
            <p className="text-default-600 mb-4 line-clamp-3">
              {show.Overview}
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
              color={show.AutoDownload ? "success" : "default"}
            >
              {show.AutoDownload ? "Auto Download: On" : "Auto Download: Off"}
            </Chip>
            <Chip size="sm" variant="flat">
              Mode: {show.AutoDownloadMode}
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
                  showId={show.Id}
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
      {show && isDeleteOpen && (
        <Card>
          <CardBody>
            <p>Are you sure you want to delete {show.Name}?</p>
            <div className="flex gap-2 mt-4">
              <Button onPress={onDeleteClose}>Cancel</Button>
              <Button
                color="danger"
                onPress={handleDelete}
                isLoading={deleting}
              >
                Delete
              </Button>
            </div>
          </CardBody>
        </Card>
      )}

      {/* File Properties Modal */}
      <FilePropertiesModal
        isOpen={isPropertiesOpen}
        onClose={() => {
          onPropertiesClose();
          setPropertiesEpisode(null);
        }}
        mediaFileId={propertiesEpisode?.MediaFileId ?? null}
        title={
          propertiesEpisode
            ? `${show.Name} - S${String(propertiesEpisode.Season).padStart(2, "0")}E${String(propertiesEpisode.Episode).padStart(2, "0")}${propertiesEpisode.Title ? ` - ${propertiesEpisode.Title}` : ""}`
            : undefined
        }
      />
    </div>
  );
}
