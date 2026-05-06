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
  type ShowPlaybackProgressByMediaQuery,
} from "../../lib/graphql/generated/graphql";
import { formatBytes } from "../../lib/format";

export interface CollectionMovieTableItem {
  TmdbId: number;
  Title: string;
  Year: number | null;
  PosterUrl: string | null;
  LibraryMovieId: string | null;
  MediaFileId: string | null;
  FileSizeBytes?: number | null;
  Resolution?: string | null;
  VideoCodec?: string | null;
  AudioCodec?: string | null;
  AudioChannels?: string | null;
  Wanted: boolean;
}

interface CollectionMoviesTableProps {
  movies: CollectionMovieTableItem[];
  stateKey: string;
  ariaLabel: string;
  searchPlaceholder: string;
  isLoading?: boolean;
  headerContent?: ReactNode;
}

type PlaybackProgressNode =
  ShowPlaybackProgressByMediaQuery["PlaybackProgresses"]["Edges"][number]["Node"];

interface CollectionMovieResolvedRow extends CollectionMovieTableItem {
  ResolvedWanted: boolean;
  ResolvedMediaFileId: string | null;
  FileSizeBytes: number | null;
  Resolution: string | null;
  VideoCodec: string | null;
  AudioCodec: string | null;
  AudioChannels: string | null;
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
  searchPlaceholder,
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
  const resolvedMovies = useMemo<CollectionMovieResolvedRow[]>(() => {
    return movies.map((movie) => {
      return {
        ...movie,
        ResolvedWanted: movie.Wanted,
        ResolvedMediaFileId: movie.MediaFileId ?? null,
        FileSizeBytes: movie.FileSizeBytes ?? null,
        Resolution: movie.Resolution ?? null,
        VideoCodec: movie.VideoCodec ?? null,
        AudioCodec: movie.AudioCodec ?? null,
        AudioChannels: movie.AudioChannels ?? null,
      };
    });
  }, [movies]);

  const { data: meData } = useQuery(MeDocument, {
    fetchPolicy: "cache-first",
  });
  const userId = meData?.Me?.Id;
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
        Where: {
          UserId: { eq: userId },
          MediaFileId: { inList: mediaFileIds },
        },
        Page: { limit: 5000, offset: 0 },
        OrderBy: [{ UpdatedAt: "DESC" }],
      },
      skip: !userId || mediaFileIds.length === 0,
      fetchPolicy: "cache-and-network",
    },
  );
  const progressEdges =
    progressData?.PlaybackProgresses?.Edges ??
    previousProgressData?.PlaybackProgresses?.Edges ??
    [];
  const progressByMediaFile = useMemo(() => {
    const map = new Map<string, PlaybackProgressNode>();
    for (const edge of progressEdges) {
      const node = edge.Node;
      if (!node) continue;
      if (!node.MediaFileId) continue;
      if (!map.has(node.MediaFileId)) {
        map.set(node.MediaFileId, node);
      }
    }
    return map;
  }, [progressEdges]);

  const isCurrentMovieRow = useCallback(
    (movie: CollectionMovieResolvedRow) =>
      Boolean(movie.LibraryMovieId) && movie.LibraryMovieId === currentMovieId,
    [currentMovieId],
  );

  const columns: DataTableColumn<CollectionMovieResolvedRow>[] = [
    {
      key: "title",
      label: "Title",
      sortable: true,
      render: (movie) => (
        <div className="flex items-center gap-3 min-w-0">
          {movie.PosterUrl ? (
            <Image
              src={movie.PosterUrl}
              alt={movie.Title}
              className="w-10 h-14 object-cover rounded shrink-0"
              loading="lazy"
            />
          ) : (
            <div className="w-10 h-14 bg-default-200 rounded flex items-center justify-center shrink-0">
              <IconMovie size={16} className="text-purple-400" />
            </div>
          )}
          {movie.LibraryMovieId ? (
            <Link
              to="/movies/$movieId"
              params={{ movieId: movie.LibraryMovieId }}
              className="font-medium hover:opacity-80 truncate"
            >
              {movie.Title}
            </Link>
          ) : (
            <span className="font-medium truncate">{movie.Title}</span>
          )}
        </div>
      ),
    },
    {
      key: "progress",
      label: "Progress",
      width: 110,
      render: (movie) => {
        if (!movie.ResolvedMediaFileId || !movie.LibraryMovieId) {
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
        if (playbackProgress.IsWatched) {
          return <span className="text-success text-sm">Watched</span>;
        }
        if (playbackProgress.ProgressPercent > 0) {
          const percentage = Math.round(
            Math.max(0, Math.min(1, playbackProgress.ProgressPercent)) * 100,
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
          {movie.Year ?? "—"}
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
          movie.Resolution,
          formatVideoCodec(movie.VideoCodec),
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
          movie.AudioCodec,
          movie.AudioChannels,
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
        if (!movie.ResolvedMediaFileId || !movie.FileSizeBytes) {
          return <span className="text-default-400">-</span>;
        }
        return (
          <span className="text-default-500 text-sm text-nowrap">
            {formatBytes(movie.FileSizeBytes)}
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
        Boolean(movie.LibraryMovieId) &&
        Boolean(movie.ResolvedMediaFileId) &&
        !(isCurrentMovieRow(movie) && isPlaying),
      onAction: (movie) => {
        if (!movie.LibraryMovieId || !movie.ResolvedMediaFileId) return;
        const playbackMovie = {
          Id: movie.LibraryMovieId,
          Title: movie.Title,
          Year: movie.Year,
          CollectionPosterUrl: movie.PosterUrl,
        };
        void startMoviePlayback(
          movie.LibraryMovieId,
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
        getRowKey={(movie) => movie.LibraryMovieId ?? `tmdb-${movie.TmdbId}`}
        ariaLabel={ariaLabel}
        searchPlaceholder={searchPlaceholder}
        showItemCount
        showViewModeToggle
        cardGridClassName="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3"
        isLoading={isLoading}
        headerContent={headerContent}
        cardRenderer={({ item }) => (
          <Card className="bg-content1 border border-default-200 w-full">
            <CardBody className="p-3">
              <div className="flex gap-3">
                {item.PosterUrl ? (
                  <Image
                    src={item.PosterUrl}
                    alt={item.Title}
                    className="w-14 h-20 object-cover rounded-md shrink-0"
                    loading="lazy"
                  />
                ) : (
                  <div className="w-14 h-20 bg-default-200 rounded-md shrink-0 flex items-center justify-center">
                    <IconMovie size={16} className="text-purple-400" />
                  </div>
                )}
                <div className="min-w-0 flex-1 space-y-2">
                  {item.LibraryMovieId ? (
                    <Link
                      to="/movies/$movieId"
                      params={{ movieId: item.LibraryMovieId }}
                      className="block font-medium truncate hover:opacity-80"
                    >
                      {item.Title}
                    </Link>
                  ) : (
                    <p className="font-medium truncate">{item.Title}</p>
                  )}
                  <p className="text-xs text-default-500">
                    {item.Year ?? "Unknown year"}
                  </p>
                  <MediaItemStatusChip
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
