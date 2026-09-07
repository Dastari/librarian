import { useCallback, useMemo, useState, type ReactNode } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { Card, CardBody } from "@heroui/card";
import { Image } from "@heroui/image";
import { useDisclosure } from "@heroui/modal";
import { type DataTableColumn, type RowAction } from "../data-table";
import { FilePropertiesModal } from "../FilePropertiesModal";
import { MediaItemStatusChip, PlayPauseIndicator } from "../shared";
import { usePlaybackContext } from "../../contexts/PlaybackContext";
import {
  IconInfoCircle,
  IconMovie,
  IconPlayerPlay,
  IconSearch,
} from "@tabler/icons-react";
import { DetailItemsTable } from "../media/DetailItemsTable";
import { useQuery } from "../../lib/graphql/client";
import {
  MeDocument,
  ShowPlaybackProgressByMediaDocument,
  ContentStatusType,
  type ContentStatus,
  type ShowPlaybackProgressByMediaQuery,
} from "../../lib/graphql/generated/graphql";
import { formatBytes } from "../../lib/format";
import { useContentStatuses } from "../../hooks/useContentStatuses";

export interface CollectionMovieTableItem {
  tmdbId: number;
  title: string;
  year: number | null;
  posterUrl: string | null;
  libraryMovieId: string | null;
  mediaFileId: string | null;
  fileSizeBytes?: number | null;
  resolution?: string | null;
  videoCodec?: string | null;
  audioCodec?: string | null;
  audioChannels?: string | null;
  wanted: boolean;
}

interface CollectionMoviesTableProps {
  movies: CollectionMovieTableItem[];
  stateKey: string;
  ariaLabel: string;
  toolbarQueryPlaceholder: string;
  isLoading?: boolean;
  headerContent?: ReactNode;
}

type PlaybackProgressNode =
  ShowPlaybackProgressByMediaQuery["playbackProgresses"]["edges"][number]["node"];

interface CollectionMovieResolvedRow extends CollectionMovieTableItem {
  ResolvedWanted: boolean;
  ResolvedMediaFileId: string | null;
  fileSizeBytes: number | null;
  resolution: string | null;
  videoCodec: string | null;
  audioCodec: string | null;
  audioChannels: string | null;
  contentStatus?: ContentStatus;
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

export function CollectionMoviesTable({
  movies,
  stateKey,
  ariaLabel,
  toolbarQueryPlaceholder,
  isLoading = false,
  headerContent,
}: CollectionMoviesTableProps) {
  const navigate = useNavigate();
  const { startMoviePlayback, session, updatePlayback } = usePlaybackContext();
  const [propertiesMediaFileId, setPropertiesMediaFileId] = useState<
    string | null
  >(null);
  const {
    isOpen: isPropertiesOpen,
    onOpen: onPropertiesOpen,
    onClose: onPropertiesClose,
  } = useDisclosure();

  const currentMovieId = session?.movieId ?? null;
  const isPlaying = session?.isPlaying ?? false;
  const statusTargets = useMemo(
    () =>
      movies.flatMap((movie) =>
        movie.libraryMovieId
          ? [
              {
                contentType: ContentStatusType.MOVIE,
                id: movie.libraryMovieId,
              },
            ]
          : [],
      ),
    [movies],
  );
  const { getStatus } = useContentStatuses(statusTargets);
  const resolvedMovies = useMemo<CollectionMovieResolvedRow[]>(() => {
    return movies.map((movie) => {
      return {
        ...movie,
        ResolvedWanted: movie.wanted,
        ResolvedMediaFileId: movie.mediaFileId ?? null,
        fileSizeBytes: movie.fileSizeBytes ?? null,
        resolution: movie.resolution ?? null,
        videoCodec: movie.videoCodec ?? null,
        audioCodec: movie.audioCodec ?? null,
        audioChannels: movie.audioChannels ?? null,
        contentStatus: movie.libraryMovieId
          ? getStatus(ContentStatusType.MOVIE, movie.libraryMovieId)
          : undefined,
      };
    });
  }, [getStatus, movies]);

  const { data: meData } = useQuery(MeDocument, {
    fetchPolicy: "cache-first",
  });
  const userId = meData?.me?.id;
  const mediaFileIds = useMemo(
    () => [
      ...new Set(
        resolvedMovies
          .map((movie) => movie.ResolvedMediaFileId)
          .filter((id): id is string => Boolean(id)),
      ),
    ],
    [resolvedMovies],
  );
  const { data: progressData, previousData: previousProgressData } = useQuery(
    ShowPlaybackProgressByMediaDocument,
    {
      variables: {
        where: {
          userId: { eq: userId },
          mediaFileId: { inList: mediaFileIds },
        },
        page: { limit: 5000, offset: 0 },
        orderBy: [{ updatedAt: "DESC" }],
      },
      skip: !userId || mediaFileIds.length === 0,
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

  const isCurrentMovieRow = useCallback(
    (movie: CollectionMovieResolvedRow) =>
      Boolean(movie.libraryMovieId) && movie.libraryMovieId === currentMovieId,
    [currentMovieId],
  );

  const columns: DataTableColumn<CollectionMovieResolvedRow>[] = [
    {
      key: "title",
      label: "Title",
      sortable: true,
      render: (movie) => (
        <div className="flex items-center gap-3 min-w-0">
          {movie.posterUrl ? (
            <Image
              src={movie.posterUrl}
              alt={movie.title}
              className="w-10 h-14 object-cover rounded shrink-0"
              loading="lazy"
            />
          ) : (
            <div className="w-10 h-14 bg-default-200 rounded flex items-center justify-center shrink-0">
              <IconMovie size={16} className="text-purple-400" />
            </div>
          )}
          {movie.libraryMovieId ? (
            <Link
              to="/movies/$movieId"
              params={{ movieId: movie.libraryMovieId }}
              className="font-medium hover:opacity-80 truncate"
            >
              {movie.title}
            </Link>
          ) : (
            <span className="font-medium truncate">{movie.title}</span>
          )}
        </div>
      ),
    },
    {
      key: "progress",
      label: "Progress",
      width: 110,
      render: (movie) => {
        if (!movie.ResolvedMediaFileId || !movie.libraryMovieId) {
          return <span className="text-default-400">-</span>;
        }
        if (isCurrentMovieRow(movie)) {
          return (
            <span
              className={
                isPlaying ? "text-success text-sm" : "text-default-500 text-sm"
              }
            >
              {isPlaying ? "Playing" : "Paused"}
            </span>
          );
        }
        const playbackProgress = progressByMediaFile.get(
          movie.ResolvedMediaFileId,
        );
        if (!playbackProgress) {
          return <span className="text-default-400">-</span>;
        }
        if (playbackProgress.isWatched) {
          return <span className="text-success text-sm">Watched</span>;
        }
        if (playbackProgress.progressPercent > 0) {
          const percentage = Math.round(
            Math.max(0, Math.min(1, playbackProgress.progressPercent)) * 100,
          );
          return (
            <div className="flex items-center gap-2">
              <div className="h-1.5 w-16 bg-default-200 rounded-full overflow-hidden">
                <div
                  className="h-full bg-primary rounded-full"
                  style={{ width: `${percentage}%` }}
                />
              </div>
              <span className="text-xs text-default-400">{percentage}%</span>
            </div>
          );
        }
        return <span className="text-default-400">-</span>;
      },
    },
    {
      key: "airDate",
      label: "Air Date",
      width: 130,
      sortable: true,
      render: (movie) => (
        <span className="text-default-500 text-sm text-nowrap">
          {movie.year ?? "—"}
        </span>
      ),
    },
    {
      key: "quality",
      label: "Quality",
      width: 120,
      render: (movie) => {
        if (!movie.ResolvedMediaFileId)
          return <span className="text-default-400">-</span>;
        const qualityParts = [
          movie.resolution,
          formatVideoCodec(movie.videoCodec),
        ].filter(Boolean);
        if (qualityParts.length === 0)
          return <span className="text-default-400">-</span>;
        return (
          <span className="text-default-500 text-sm">
            {qualityParts.join(" · ")}
          </span>
        );
      },
    },
    {
      key: "audio",
      label: "Audio",
      width: 100,
      render: (movie) => {
        if (!movie.ResolvedMediaFileId)
          return <span className="text-default-400">-</span>;
        const audioLabel = formatAudioCodec(
          movie.audioCodec,
          movie.audioChannels,
        );
        if (!audioLabel) return <span className="text-default-400">-</span>;
        return <span className="text-default-500 text-sm">{audioLabel}</span>;
      },
    },
    {
      key: "size",
      label: "Size",
      width: 100,
      render: (movie) => {
        if (!movie.ResolvedMediaFileId || !movie.fileSizeBytes) {
          return <span className="text-default-400">-</span>;
        }
        return (
          <span className="text-default-500 text-sm text-nowrap">
            {formatBytes(movie.fileSizeBytes)}
          </span>
        );
      },
    },
    {
      key: "status",
      label: "Status",
      width: 140,
      sortable: true,
      render: (movie) => (
        <MediaItemStatusChip
          status={movie.contentStatus}
          mediaFileId={movie.ResolvedMediaFileId}
          wanted={movie.ResolvedWanted}
        />
      ),
    },
  ];

  const rowActions: RowAction<CollectionMovieResolvedRow>[] = [
    {
      key: `pause-${currentMovieId || "none"}-${isPlaying ? "playing" : "paused"}`,
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
      isVisible: (movie) =>
        Boolean(movie.ResolvedMediaFileId) &&
        isCurrentMovieRow(movie) &&
        isPlaying,
      onAction: () => {
        void updatePlayback({ isPlaying: false });
      },
    },
    {
      key: `play-${currentMovieId || "none"}-${isPlaying ? "playing" : "paused"}`,
      label: "Play",
      icon: <IconPlayerPlay size={16} />,
      color: "success",
      inDropdown: false,
      isVisible: (movie) =>
        Boolean(movie.libraryMovieId) &&
        Boolean(movie.ResolvedMediaFileId) &&
        !(isCurrentMovieRow(movie) && isPlaying),
      onAction: (movie) => {
        if (!movie.libraryMovieId || !movie.ResolvedMediaFileId) return;
        const playbackMovie = {
          id: movie.libraryMovieId,
          title: movie.title,
          year: movie.year,
          collectionPosterUrl: movie.posterUrl,
        };
        void startMoviePlayback(
          movie.libraryMovieId,
          movie.ResolvedMediaFileId,
          playbackMovie as unknown as Parameters<typeof startMoviePlayback>[2],
        );
      },
    },
    {
      key: "search",
      label: "Search for Movie",
      icon: <IconSearch size={16} />,
      color: "default",
      inDropdown: false,
      isVisible: (movie) => !movie.ResolvedMediaFileId,
      onAction: () => {
        void navigate({ to: "/settings/sources" });
      },
    },
    {
      key: "properties",
      label: "File Properties",
      icon: <IconInfoCircle size={16} />,
      color: "default",
      inDropdown: true,
      isVisible: (movie) => Boolean(movie.ResolvedMediaFileId),
      onAction: (movie) => {
        if (!movie.ResolvedMediaFileId) return;
        setPropertiesMediaFileId(movie.ResolvedMediaFileId);
        onPropertiesOpen();
      },
    },
  ];

  const selectedKeys = useMemo(() => {
    if (currentMovieId) return new Set([currentMovieId]);
    return new Set<string>();
  }, [currentMovieId]);

  const tableKey = `${stateKey}-${currentMovieId || "none"}-${isPlaying ? "playing" : "paused"}`;

  return (
    <>
      <DetailItemsTable
        tableKey={tableKey}
        stateKey={stateKey}
        data={resolvedMovies}
        columns={columns}
        rowActions={rowActions}
        getRowKey={(movie) => movie.libraryMovieId ?? `tmdb-${movie.tmdbId}`}
        ariaLabel={ariaLabel}
        toolbarQueryPlaceholder={toolbarQueryPlaceholder}
        showItemCount
        showViewModeToggle
        cardGridClassName="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3"
        isLoading={isLoading}
        headerContent={headerContent}
        cardRenderer={({ item }) => (
          <Card className="bg-content1 border border-default-200 w-full">
            <CardBody className="p-3">
              <div className="flex gap-3">
                {item.posterUrl ? (
                  <Image
                    src={item.posterUrl}
                    alt={item.title}
                    className="w-14 h-20 object-cover rounded-md shrink-0"
                    loading="lazy"
                  />
                ) : (
                  <div className="w-14 h-20 bg-default-200 rounded-md shrink-0 flex items-center justify-center">
                    <IconMovie size={16} className="text-purple-400" />
                  </div>
                )}
                <div className="min-w-0 flex-1 space-y-2">
                  {item.libraryMovieId ? (
                    <Link
                      to="/movies/$movieId"
                      params={{ movieId: item.libraryMovieId }}
                      className="block font-medium truncate hover:opacity-80"
                    >
                      {item.title}
                    </Link>
                  ) : (
                    <p className="font-medium truncate">{item.title}</p>
                  )}
                  <p className="text-xs text-default-500">
                    {item.year ?? "Unknown year"}
                  </p>
                  <MediaItemStatusChip
                    status={item.contentStatus}
                    mediaFileId={item.ResolvedMediaFileId}
                    wanted={item.ResolvedWanted}
                  />
                </div>
              </div>
            </CardBody>
          </Card>
        )}
        selectionMode={currentMovieId ? "single" : "none"}
        selectedKeys={selectedKeys}
      />
      <FilePropertiesModal
        isOpen={isPropertiesOpen}
        onClose={() => {
          onPropertiesClose();
          setPropertiesMediaFileId(null);
        }}
        mediaFileId={propertiesMediaFileId}
      />
    </>
  );
}
