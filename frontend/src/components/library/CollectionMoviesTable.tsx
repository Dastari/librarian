import { useCallback, useMemo, useState, type ReactNode } from "react";
import { Link, useNavigate } from "@tanstack/react-router";
import { Card, CardBody } from "@heroui/card";
import { Image } from "@heroui/image";
import { useDisclosure } from "@heroui/modal";
import { type DataTableColumn, type RowAction } from "../data-table";
import { FilePropertiesModal } from "../FilePropertiesModal";
import { MediaItemStatusChip, PlayPauseIndicator } from "../shared";
import { usePlaybackContext } from "../../contexts/PlaybackContext";
import { IconInfoCircle, IconMovie, IconPlayerPlay, IconSearch } from "@tabler/icons-react";
import { DetailItemsTable } from "../media/DetailItemsTable";

export interface CollectionMovieTableItem {
  TmdbId: number;
  Title: string;
  Year: number | null;
  PosterUrl: string | null;
  LibraryMovieId: string | null;
  MediaFileId: string | null;
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
  const [propertiesMediaFileId, setPropertiesMediaFileId] = useState<string | null>(null);
  const {
    isOpen: isPropertiesOpen,
    onOpen: onPropertiesOpen,
    onClose: onPropertiesClose,
  } = useDisclosure();

  const currentMovieId = session?.movieId ?? null;
  const isPlaying = session?.isPlaying ?? false;
  const isCurrentMovieRow = useCallback(
    (movie: CollectionMovieTableItem) =>
      Boolean(movie.LibraryMovieId) && movie.LibraryMovieId === currentMovieId,
    [currentMovieId],
  );

  const columns: DataTableColumn<CollectionMovieTableItem>[] = [
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
        if (!movie.MediaFileId || !movie.LibraryMovieId) {
          return <span className="text-default-400">-</span>;
        }
        if (isCurrentMovieRow(movie)) {
          return (
            <span className={isPlaying ? "text-success text-sm" : "text-default-500 text-sm"}>
              {isPlaying ? "Playing" : "Paused"}
            </span>
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
      render: () => <span className="text-default-400">-</span>,
    },
    {
      key: "audio",
      label: "Audio",
      width: 100,
      render: () => <span className="text-default-400">-</span>,
    },
    {
      key: "size",
      label: "Size",
      width: 100,
      render: () => <span className="text-default-400">-</span>,
    },
    {
      key: "status",
      label: "Status",
      width: 140,
      sortable: true,
      render: (movie) => (
        <MediaItemStatusChip
          mediaFileId={movie.MediaFileId}
          wanted={movie.Wanted}
        />
      ),
    },
  ];

  const rowActions: RowAction<CollectionMovieTableItem>[] = [
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
        Boolean(movie.MediaFileId) && isCurrentMovieRow(movie) && isPlaying,
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
        Boolean(movie.MediaFileId) &&
        !(isCurrentMovieRow(movie) && isPlaying),
      onAction: (movie) => {
        if (!movie.LibraryMovieId || !movie.MediaFileId) return;
        const playbackMovie = {
          Id: movie.LibraryMovieId,
          Title: movie.Title,
          Year: movie.Year,
          PosterUrl: movie.PosterUrl,
          BackdropUrl: null,
        };
        void startMoviePlayback(
          movie.LibraryMovieId,
          movie.MediaFileId,
          playbackMovie as Parameters<typeof startMoviePlayback>[2],
        );
      },
    },
    {
      key: "search",
      label: "Search for Movie",
      icon: <IconSearch size={16} />,
      color: "default",
      inDropdown: false,
      isVisible: (movie) => !movie.MediaFileId,
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
      isVisible: (movie) => Boolean(movie.MediaFileId),
      onAction: (movie) => {
        if (!movie.MediaFileId) return;
        setPropertiesMediaFileId(movie.MediaFileId);
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
        data={movies}
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
                  <p className="text-xs text-default-500">{item.Year ?? "Unknown year"}</p>
                  <MediaItemStatusChip
                    mediaFileId={item.MediaFileId}
                    wanted={item.Wanted}
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
