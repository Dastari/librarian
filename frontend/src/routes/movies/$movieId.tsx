import {
  createFileRoute,
  Link,
  redirect,
  useNavigate,
} from "@tanstack/react-router";
import { useEffect, useCallback, useMemo } from "react";
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
import { Breadcrumbs, BreadcrumbItem } from "@heroui/breadcrumbs";
import { useDisclosure } from "@heroui/modal";
import { addToast } from "@heroui/toast";
import { RouteError } from "../../components/RouteError";
import { sanitizeError, formatBytes } from "../../lib/format";
import {
  LibraryDetailRouteDocument,
  MeDocument,
  MovieCollectionPeersRouteDocument,
  MovieDetailSetWantedDocument,
  MovieDetailRouteDocument,
  RefreshMovieRouteDocument,
  ShowPlaybackProgressByMediaDocument,
  ContentStatusType,
  type MovieCollectionPeersRouteQuery,
  type MovieDetailRouteQuery,
} from "../../lib/graphql/generated/graphql";
import {
  IconMovie,
  IconTrash,
  IconPlayerPlay,
  IconPlayerPause,
  IconDotsVertical,
  IconInfoCircle,
  IconCalendar,
  IconClock,
  IconStar,
  IconSearch,
  IconCheck,
  IconX,
  IconRefresh,
} from "@tabler/icons-react";
import { DeleteMovieModal } from "../../components/library";
import { FilePropertiesModal } from "../../components/FilePropertiesModal";
import { usePlaybackContext } from "../../contexts/PlaybackContext";
import { CollectionMoviesTable } from "../../components/library/CollectionMoviesTable";
import { MediaItemStatusChip } from "../../components/shared";
import { useContentStatuses } from "../../hooks/useContentStatuses";

export const Route = createFileRoute("/movies/$movieId")({
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
  component: MovieDetailPage,
  errorComponent: RouteError,
});

type MovieNode = NonNullable<MovieDetailRouteQuery["movie"]>;

function MovieDetailPage() {
  const { movieId } = Route.useParams();
  const navigate = useNavigate();
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
  const { startMoviePlayback, session, updatePlayback } = usePlaybackContext();

  // Query movie and media file
  const {
    data: movieData,
    previousData: previousMovieData,
    loading: movieLoading,
    refetch,
  } = useQuery(MovieDetailRouteDocument, {
    variables: { id: movieId },
    fetchPolicy: "cache-and-network",
  });
  const movie: MovieNode | null =
    movieData?.movie ?? previousMovieData?.movie ?? null;
  const movieStatusTargets = useMemo(
    () => [{ contentType: ContentStatusType.MOVIE, id: movieId }],
    [movieId],
  );
  const { getStatus: getMovieStatus } =
    useContentStatuses(movieStatusTargets);
  const { data: libraryData } = useQuery(LibraryDetailRouteDocument, {
    variables: { id: movie?.libraryId ?? "" },
    skip: !movie?.libraryId,
    fetchPolicy: "cache-and-network",
  });
  const { data: meData } = useQuery(MeDocument, {
    fetchPolicy: "cache-first",
  });
  const userId = meData?.me?.id;
  const { data: movieProgressData, previousData: previousMovieProgressData } =
    useQuery(ShowPlaybackProgressByMediaDocument, {
      variables: {
        where: {
          userId: { eq: userId },
          mediaFileId: { eq: movie?.mediaFileId ?? "" },
        },
        page: { limit: 1, offset: 0 },
        orderBy: [{ updatedAt: "DESC" }],
      },
      skip: !userId || !movie?.mediaFileId,
      fetchPolicy: "cache-and-network",
    });
  const movieProgressEdge =
    movieProgressData?.playbackProgresses?.edges?.[0] ??
    previousMovieProgressData?.playbackProgresses?.edges?.[0];
  const movieProgressNode = movieProgressEdge?.node;
  const movieWatchPosition = movieProgressNode?.currentPosition ?? 0;
  const hasResumeProgress =
    !movieProgressNode?.isWatched && movieWatchPosition > 0;

  const {
    data: collectionPeersData,
    previousData: previousCollectionPeersData,
  } = useQuery<MovieCollectionPeersRouteQuery>(MovieCollectionPeersRouteDocument, {
    variables: {
      libraryId: movie?.libraryId ?? "",
      collectionId: movie?.collectionId ?? -1,
    },
    skip: !movie?.libraryId || !movie?.collectionId,
    fetchPolicy: "cache-and-network",
  });
  const collectionMovies = useMemo(
    () =>
      collectionPeersData?.movieCollectionDetails?.movies ??
      previousCollectionPeersData?.movieCollectionDetails?.movies ??
      [],
    [
      collectionPeersData?.movieCollectionDetails?.movies,
      previousCollectionPeersData?.movieCollectionDetails?.movies,
    ],
  );
  const otherCollectionMovies = useMemo(
    () =>
      collectionMovies.filter((relatedMovie) => {
        if (
          relatedMovie.libraryMovieId &&
          relatedMovie.libraryMovieId === movieId
        )
          return false;
        if (movie?.tmdbId != null && relatedMovie.tmdbId === movie.tmdbId)
          return false;
        return true;
      }),
    [collectionMovies, movie?.tmdbId, movieId],
  );

  // Mutations
  const [refreshMovie] = useMutation(RefreshMovieRouteDocument);
  const [setMovieWanted] = useMutation(MovieDetailSetWantedDocument);

  // Update page title
  useEffect(() => {
    if (movie) {
      document.title = `Librarian - ${movie.title}`;
    }
    return () => {
      document.title = "Librarian";
    };
  }, [movie]);

  const handlePlay = useCallback(
    async (startFromBeginning = false) => {
      if (!movie?.mediaFileId) {
        addToast({
          title: "No media file",
          description: "No playable media file found for this movie",
          color: "warning",
        });
        return;
      }

      try {
        const startPosition =
          !startFromBeginning && hasResumeProgress ? movieWatchPosition : 0;
        await startMoviePlayback(
          movie.id,
          movie.mediaFileId,
          movie as unknown as Parameters<typeof startMoviePlayback>[2],
          startPosition,
          movie.mediaFile?.duration || movie.runtime || undefined,
        );
      } catch (err) {
        console.error("Failed to start playback:", err);
        addToast({
          title: "Error",
          description: "Failed to start playback",
          color: "danger",
        });
      }
    },
    [hasResumeProgress, movie, movieWatchPosition, startMoviePlayback],
  );

  const handleRefresh = async () => {
    try {
      const { data } = await refreshMovie({ variables: { id: movieId } });
      if (!data?.refreshMovie?.success) {
        addToast({
          title: "Error",
          description: sanitizeError(
            data?.refreshMovie?.error || "Failed to refresh metadata",
          ),
          color: "danger",
        });
        return;
      }
      addToast({
        title: "Refreshed",
        description: "Movie metadata and artwork updated",
        color: "success",
      });
      await refetch();
    } catch (err) {
      console.error("Failed to refresh movie:", err);
      addToast({
        title: "Error",
        description: "Failed to refresh metadata",
        color: "danger",
      });
    }
  };

  const handleSetWanted = useCallback(
    async (wanted: boolean) => {
      try {
        const { data } = await setMovieWanted({
          variables: { id: movieId, wanted: wanted },
        });
        if (!data?.updateMovie?.success) {
          addToast({
            title: "Error",
            description: sanitizeError(
              data?.updateMovie?.error || "Failed to update wanted status",
            ),
            color: "danger",
          });
          return;
        }
        addToast({
          title: wanted ? "Marked as wanted" : "Removed wanted",
          description: wanted
            ? "Movie marked as wanted"
            : "Movie removed from wanted",
          color: "success",
        });
        await refetch();
      } catch (err) {
        console.error("Failed to update movie wanted state:", err);
        addToast({
          title: "Error",
          description: "Failed to update wanted status",
          color: "danger",
        });
      }
    },
    [movieId, refetch, setMovieWanted],
  );

  const handleDeleted = () => {
    // Navigate back to library after deletion
    navigate({
      to: "/libraries/$libraryId",
      params: { libraryId: movie?.libraryId || "" },
    });
  };

  const handleSearchMovie = useCallback(() => {
    if (!movie) return;
    navigate({ to: "/settings/sources" });
  }, [movie, navigate]);

  const isThisMoviePlaying =
    session?.movieId === movieId && !!session?.isPlaying;

  // Loading state
  if (movieLoading && !movie) {
    return (
      <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex items-center justify-center min-h-[50vh]">
        <Spinner size="lg" />
      </div>
    );
  }

  // Not found state
  if (!movie) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-8">
        <Card className="bg-content1">
          <CardBody className="text-center py-12">
            <h2 className="text-xl font-semibold mb-4">Movie not found</h2>
            <Link to="/libraries">
              <Button color="primary">Back to Libraries</Button>
            </Link>
          </CardBody>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 mb-20">
      {/* Header */}
      <div className="flex flex-col md:flex-row gap-6 mb-8">
        {/* Poster */}
        <div className="shrink-0 relative group">
          {movie.posterUrl ? (
            <Image
              src={movie.posterUrl}
              alt={movie.title}
              className="w-64 h-96 object-cover rounded-lg shadow-lg"
            />
          ) : (
            <div className="w-64 h-96 bg-default-200 rounded-lg flex items-center justify-center">
              <IconMovie size={64} className="text-purple-400" />
            </div>
          )}
          {movie.mediaFileId && movie.mediaFile && (
            <button
              onClick={() => {
                if (isThisMoviePlaying) {
                  updatePlayback({ isPlaying: false });
                } else {
                  void handlePlay(false);
                }
              }}
              className="absolute inset-0 z-10 flex items-center justify-center bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity duration-200 rounded-lg cursor-pointer"
              aria-label={isThisMoviePlaying ? "Pause Movie" : "Play Movie"}
            >
              <div
                className={`w-16 h-16 rounded-full ${isThisMoviePlaying ? "bg-warning" : "bg-primary"} flex items-center justify-center shadow-lg hover:scale-110 transition-transform`}
              >
                {isThisMoviePlaying ? (
                  <IconPlayerPause size={32} className="text-white" />
                ) : (
                  <IconPlayerPlay size={32} className="text-white ml-1" />
                )}
              </div>
            </button>
          )}
        </div>

        {/* Details */}
        <div className="flex-1">
          <Breadcrumbs className="mb-2">
            <BreadcrumbItem><Link to="/libraries">Libraries</Link></BreadcrumbItem>
            <BreadcrumbItem>
              <Link to="/libraries/$libraryId" params={{ libraryId: movie.libraryId }}>
                {libraryData?.library?.name || "Library"}
              </Link>
            </BreadcrumbItem>
            <BreadcrumbItem isCurrent>{movie.title}</BreadcrumbItem>
          </Breadcrumbs>

          <div className="flex items-start justify-between gap-4 mb-2">
            <h1 className="text-3xl font-bold">
              {movie.title}
              {movie.year && (
                <span className="text-default-500 ml-2">({movie.year})</span>
              )}
            </h1>
            <div className="flex items-center gap-2">
              {movie.mediaFileId ? (
                <Button
                  color={isThisMoviePlaying ? "warning" : "primary"}
                  variant="solid"
                  startContent={
                    isThisMoviePlaying ? (
                      <IconPlayerPause size={16} />
                    ) : (
                      <IconPlayerPlay size={16} />
                    )
                  }
                  onPress={() => {
                    if (isThisMoviePlaying) {
                      void updatePlayback({ isPlaying: false });
                      return;
                    }
                    void handlePlay(false);
                  }}
                >
                  {isThisMoviePlaying
                    ? "Pause"
                    : hasResumeProgress
                      ? "Resume"
                      : "Play"}
                </Button>
              ) : null}
              {movie.mediaFileId && hasResumeProgress && !isThisMoviePlaying ? (
                <Button
                  color="default"
                  variant="flat"
                  startContent={<IconPlayerPlay size={16} />}
                  onPress={() => void handlePlay(true)}
                >
                  Start from beginning
                </Button>
              ) : null}
              <Dropdown>
                <DropdownTrigger>
                  <Button
                    isIconOnly
                    size="sm"
                    variant="light"
                    aria-label="Movie actions"
                  >
                    <IconDotsVertical size={18} />
                  </Button>
                </DropdownTrigger>
                <DropdownMenu
                  aria-label="Movie actions menu"
                  onAction={(key) => {
                    if (key === "search") {
                      handleSearchMovie();
                    } else if (key === "refresh") {
                      void handleRefresh();
                    } else if (key === "wanted-on") {
                      void handleSetWanted(true);
                    } else if (key === "wanted-off") {
                      void handleSetWanted(false);
                    } else if (key === "properties") {
                      onPropertiesOpen();
                    } else if (key === "delete") {
                      onDeleteOpen();
                    }
                  }}
                >
                  <DropdownItem
                    key="search"
                    startContent={<IconSearch size={16} />}
                  >
                    Search for Movie
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
                    isDisabled={movie.wanted}
                  >
                    Mark as Wanted
                  </DropdownItem>
                  <DropdownItem
                    key="wanted-off"
                    startContent={<IconX size={16} />}
                    isDisabled={!movie.wanted}
                  >
                    Remove as Wanted
                  </DropdownItem>
                  {movie.mediaFileId ? (
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
          </div>

          {/* Tagline */}
          {movie.tagline && (
            <p className="text-default-500 italic mb-4">"{movie.tagline}"</p>
          )}

          {/* Chips */}
          <div className="flex flex-wrap gap-2 mb-4">
            {/* File status */}
            <MediaItemStatusChip
              status={getMovieStatus(ContentStatusType.MOVIE, movie.id)}
              mediaFileId={movie.mediaFileId}
              wanted={movie.wanted}
            />

            {/* Rating */}
            {movie.tmdbRating && Number(movie.tmdbRating) > 0 && (
              <Chip
                size="sm"
                variant="flat"
                color={
                  Number(movie.tmdbRating) >= 7
                    ? "success"
                    : Number(movie.tmdbRating) >= 5
                      ? "warning"
                      : "danger"
                }
                startContent={<IconStar size={14} />}
              >
                {Number(movie.tmdbRating).toFixed(1)} (
                {movie.tmdbVoteCount?.toLocaleString()} votes)
              </Chip>
            )}

            {/* Certification */}
            {movie.certification && (
              <Chip size="sm" variant="flat">
                {movie.certification}
              </Chip>
            )}

            {/* Runtime */}
            {movie.runtime && (
              <Chip
                size="sm"
                variant="flat"
                startContent={<IconClock size={14} />}
              >
                {Math.floor(movie.runtime / 60)}h {movie.runtime % 60}m
              </Chip>
            )}

            {/* Release date */}
            {movie.releaseDate && (
              <Chip
                size="sm"
                variant="flat"
                startContent={<IconCalendar size={14} />}
              >
                {new Date(movie.releaseDate).toLocaleDateString()}
              </Chip>
            )}

            {/* Monitored */}
            <Chip
              size="sm"
              variant="flat"
              color={movie.monitored ? "success" : "default"}
            >
              {movie.monitored ? "Monitored" : "Unmonitored"}
            </Chip>
          </div>

          {/* Genres */}
          {movie.genres.length > 0 && (
            <div className="flex flex-wrap gap-1 mb-4">
              {movie.genres.map((genre: string, index: number) => (
                <Chip
                  key={`${genre}-${index}`}
                  size="sm"
                  variant="bordered"
                  className="text-xs"
                >
                  {genre}
                </Chip>
              ))}
            </div>
          )}

          {/* Overview */}
          {movie.overview && (
            <p className="text-default-600 mb-4 line-clamp-4">
              {movie.overview}
            </p>
          )}

          {/* Credits */}
          <div className="flex gap-8 text-sm mb-4">
            {movie.director && (
              <div>
                <span className="text-default-500">Director:</span>{" "}
                <span className="font-medium">{movie.director}</span>
              </div>
            )}
            {movie.castNames.length > 0 && (
              <div>
                <span className="text-default-500">Cast:</span>{" "}
                <span className="font-medium">
                  {movie.castNames.slice(0, 3).join(", ")}
                </span>
              </div>
            )}
          </div>

          {/* Stats */}
          {movie.mediaFile && movie.mediaFile.size > 0 && (
            <div className="flex gap-4 text-sm text-default-500 mb-4">
              <div>
                <span className="font-semibold text-foreground">
                  {formatBytes(movie.mediaFile.size)}
                </span>
                <span> on disk</span>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Collection peers */}
      {movie.collectionName && (
        <>
          {otherCollectionMovies.length > 0 && (
            <CollectionMoviesTable
              stateKey={`movie-collection-peers-${movie.id}`}
              movies={otherCollectionMovies}
              ariaLabel="Also in this collection"
              toolbarQueryPlaceholder="Search collection movies..."
              headerContent={
                <div className="px-2 py-1 text-sm text-default-600">
                  Also in {movie.collectionName}
                </div>
              }
            />
          )}
        </>
      )}

      {/* Delete Movie Modal */}
      <DeleteMovieModal
        isOpen={isDeleteOpen}
        onClose={onDeleteClose}
        movie={movie ? { id: movie.id, title: movie.title } : null}
        onDeleted={handleDeleted}
      />
      <FilePropertiesModal
        isOpen={isPropertiesOpen}
        onClose={onPropertiesClose}
        mediaFileId={movie.mediaFileId ?? null}
        title={movie ? movie.title : undefined}
      />
    </div>
  );
}
