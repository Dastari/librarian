import {
  createFileRoute,
  Link,
  redirect,
  useNavigate,
} from "@tanstack/react-router";
import { useEffect, useCallback, useMemo } from "react";
import { useQuery, useMutation, gql } from "../../lib/graphql/client";
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
  MovieDetailSetWantedDocument,
  MovieDetailRouteDocument,
  RefreshMovieRouteDocument,
  ShowPlaybackProgressByMediaDocument,
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

type MovieNode = NonNullable<MovieDetailRouteQuery["Movie"]>;

interface RelatedCollectionMovie {
  TmdbId: number;
  Title: string;
  Year: number | null;
  PosterUrl: string | null;
  LibraryMovieId: string | null;
  MediaFileId: string | null;
  FileSizeBytes: number | null;
  Resolution: string | null;
  VideoCodec: string | null;
  AudioCodec: string | null;
  AudioChannels: string | null;
  Wanted: boolean;
}

interface MovieCollectionPeersQueryData {
  MovieCollectionDetails: {
    Movies: RelatedCollectionMovie[];
  } | null;
}

const MOVIE_COLLECTION_PEERS_QUERY = gql`
  query MovieCollectionPeersRoute($LibraryId: String!, $CollectionId: Int!) {
    MovieCollectionDetails(LibraryId: $LibraryId, CollectionId: $CollectionId) {
      Movies {
        TmdbId
        Title
        Year
        PosterUrl
        LibraryMovieId
        MediaFileId
        FileSizeBytes
        Resolution
        VideoCodec
        AudioCodec
        AudioChannels
        Wanted
      }
    }
  }
`;

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
    variables: { Id: movieId },
    fetchPolicy: "cache-and-network",
  });
  const movie: MovieNode | null =
    movieData?.Movie ?? previousMovieData?.Movie ?? null;
  const { data: libraryData } = useQuery(LibraryDetailRouteDocument, {
    variables: { Id: movie?.LibraryId ?? "" },
    skip: !movie?.LibraryId,
    fetchPolicy: "cache-and-network",
  });
  const { data: meData } = useQuery(MeDocument, {
    fetchPolicy: "cache-first",
  });
  const userId = meData?.Me?.Id;
  const { data: movieProgressData, previousData: previousMovieProgressData } =
    useQuery(ShowPlaybackProgressByMediaDocument, {
      variables: {
        Where: {
          UserId: { eq: userId },
          MediaFileId: { eq: movie?.MediaFileId ?? "" },
        },
        Page: { limit: 1, offset: 0 },
        OrderBy: [{ UpdatedAt: "DESC" }],
      },
      skip: !userId || !movie?.MediaFileId,
      fetchPolicy: "cache-and-network",
    });
  const movieProgressEdge =
    movieProgressData?.PlaybackProgresses?.Edges?.[0] ??
    previousMovieProgressData?.PlaybackProgresses?.Edges?.[0];
  const movieProgressNode = movieProgressEdge?.Node;
  const movieWatchPosition = movieProgressNode?.CurrentPosition ?? 0;
  const hasResumeProgress =
    !movieProgressNode?.IsWatched && movieWatchPosition > 0;

  const {
    data: collectionPeersData,
    previousData: previousCollectionPeersData,
  } = useQuery<MovieCollectionPeersQueryData>(MOVIE_COLLECTION_PEERS_QUERY, {
    variables: {
      LibraryId: movie?.LibraryId ?? "",
      CollectionId: movie?.CollectionId ?? -1,
    },
    skip: !movie?.LibraryId || !movie?.CollectionId,
    fetchPolicy: "cache-and-network",
  });
  const collectionMovies = useMemo(
    () =>
      collectionPeersData?.MovieCollectionDetails?.Movies ??
      previousCollectionPeersData?.MovieCollectionDetails?.Movies ??
      [],
    [
      collectionPeersData?.MovieCollectionDetails?.Movies,
      previousCollectionPeersData?.MovieCollectionDetails?.Movies,
    ],
  );
  const otherCollectionMovies = useMemo(
    () =>
      collectionMovies.filter((relatedMovie) => {
        if (
          relatedMovie.LibraryMovieId &&
          relatedMovie.LibraryMovieId === movieId
        )
          return false;
        if (movie?.TmdbId != null && relatedMovie.TmdbId === movie.TmdbId)
          return false;
        return true;
      }),
    [collectionMovies, movie?.TmdbId, movieId],
  );

  // Mutations
  const [refreshMovie] = useMutation(RefreshMovieRouteDocument);
  const [setMovieWanted] = useMutation(MovieDetailSetWantedDocument);

  // Update page title
  useEffect(() => {
    if (movie) {
      document.title = `Librarian - ${movie.Title}`;
    }
    return () => {
      document.title = "Librarian";
    };
  }, [movie]);

  const handlePlay = useCallback(
    async (startFromBeginning = false) => {
      if (!movie?.MediaFileId) {
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
          movie.Id,
          movie.MediaFileId,
          movie as unknown as Parameters<typeof startMoviePlayback>[2],
          startPosition,
          movie.MediaFile?.Duration || movie.Runtime || undefined,
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
      const { data } = await refreshMovie({ variables: { Id: movieId } });
      if (!data?.RefreshMovie?.Success) {
        addToast({
          title: "Error",
          description: sanitizeError(
            data?.RefreshMovie?.Error || "Failed to refresh metadata",
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
          variables: { Id: movieId, Wanted: wanted },
        });
        if (!data?.UpdateMovie?.Success) {
          addToast({
            title: "Error",
            description: sanitizeError(
              data?.UpdateMovie?.Error || "Failed to update wanted status",
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
      params: { libraryId: movie?.LibraryId || "" },
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
          {movie.PosterUrl ? (
            <Image
              src={movie.PosterUrl}
              alt={movie.Title}
              className="w-64 h-96 object-cover rounded-lg shadow-lg"
            />
          ) : (
            <div className="w-64 h-96 bg-default-200 rounded-lg flex items-center justify-center">
              <IconMovie size={64} className="text-purple-400" />
            </div>
          )}
          {movie.MediaFileId && movie.MediaFile && (
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
            <BreadcrumbItem href="/libraries">Libraries</BreadcrumbItem>
            <BreadcrumbItem href={`/libraries/${movie.LibraryId}`}>
              {libraryData?.Library?.Name || "Library"}
            </BreadcrumbItem>
            <BreadcrumbItem isCurrent>{movie.Title}</BreadcrumbItem>
          </Breadcrumbs>

          <div className="flex items-start justify-between gap-4 mb-2">
            <h1 className="text-3xl font-bold">
              {movie.Title}
              {movie.Year && (
                <span className="text-default-500 ml-2">({movie.Year})</span>
              )}
            </h1>
            <div className="flex items-center gap-2">
              {movie.MediaFileId ? (
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
              {movie.MediaFileId && hasResumeProgress && !isThisMoviePlaying ? (
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
                    isDisabled={movie.Wanted}
                  >
                    Mark as Wanted
                  </DropdownItem>
                  <DropdownItem
                    key="wanted-off"
                    startContent={<IconX size={16} />}
                    isDisabled={!movie.Wanted}
                  >
                    Remove as Wanted
                  </DropdownItem>
                  {movie.MediaFileId ? (
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
          {movie.Tagline && (
            <p className="text-default-500 italic mb-4">"{movie.Tagline}"</p>
          )}

          {/* Chips */}
          <div className="flex flex-wrap gap-2 mb-4">
            {/* File status */}
            <Chip
              size="sm"
              color={
                movie.MediaFileId
                  ? "success"
                  : movie.Wanted
                    ? "warning"
                    : "danger"
              }
              variant="flat"
              startContent={
                movie.MediaFileId ? (
                  <IconCheck size={14} />
                ) : (
                  <IconX size={14} />
                )
              }
            >
              {movie.MediaFileId
                ? "Downloaded"
                : movie.Wanted
                  ? "Wanted"
                  : "Missing"}
            </Chip>

            {/* Rating */}
            {movie.TmdbRating && Number(movie.TmdbRating) > 0 && (
              <Chip
                size="sm"
                variant="flat"
                color={
                  Number(movie.TmdbRating) >= 7
                    ? "success"
                    : Number(movie.TmdbRating) >= 5
                      ? "warning"
                      : "danger"
                }
                startContent={<IconStar size={14} />}
              >
                {Number(movie.TmdbRating).toFixed(1)} (
                {movie.TmdbVoteCount?.toLocaleString()} votes)
              </Chip>
            )}

            {/* Certification */}
            {movie.Certification && (
              <Chip size="sm" variant="flat">
                {movie.Certification}
              </Chip>
            )}

            {/* Runtime */}
            {movie.Runtime && (
              <Chip
                size="sm"
                variant="flat"
                startContent={<IconClock size={14} />}
              >
                {Math.floor(movie.Runtime / 60)}h {movie.Runtime % 60}m
              </Chip>
            )}

            {/* Release date */}
            {movie.ReleaseDate && (
              <Chip
                size="sm"
                variant="flat"
                startContent={<IconCalendar size={14} />}
              >
                {new Date(movie.ReleaseDate).toLocaleDateString()}
              </Chip>
            )}

            {/* Monitored */}
            <Chip
              size="sm"
              variant="flat"
              color={movie.Monitored ? "success" : "default"}
            >
              {movie.Monitored ? "Monitored" : "Unmonitored"}
            </Chip>
          </div>

          {/* Genres */}
          {movie.Genres.length > 0 && (
            <div className="flex flex-wrap gap-1 mb-4">
              {movie.Genres.map((genre: string, index: number) => (
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
          {movie.Overview && (
            <p className="text-default-600 mb-4 line-clamp-4">
              {movie.Overview}
            </p>
          )}

          {/* Credits */}
          <div className="flex gap-8 text-sm mb-4">
            {movie.Director && (
              <div>
                <span className="text-default-500">Director:</span>{" "}
                <span className="font-medium">{movie.Director}</span>
              </div>
            )}
            {movie.CastNames.length > 0 && (
              <div>
                <span className="text-default-500">Cast:</span>{" "}
                <span className="font-medium">
                  {movie.CastNames.slice(0, 3).join(", ")}
                </span>
              </div>
            )}
          </div>

          {/* Stats */}
          {movie.MediaFile && movie.MediaFile.Size > 0 && (
            <div className="flex gap-4 text-sm text-default-500 mb-4">
              <div>
                <span className="font-semibold text-foreground">
                  {formatBytes(movie.MediaFile.Size)}
                </span>
                <span> on disk</span>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Collection peers */}
      {movie.CollectionName && (
        <>
          {otherCollectionMovies.length > 0 && (
            <CollectionMoviesTable
              stateKey={`movie-collection-peers-${movie.Id}`}
              movies={otherCollectionMovies}
              ariaLabel="Also in this collection"
              searchPlaceholder="Search collection movies..."
              headerContent={
                <div className="px-2 py-1 text-sm text-default-600">
                  Also in {movie.CollectionName}
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
        movie={movie ? { id: movie.Id, title: movie.Title } : null}
        onDeleted={handleDeleted}
      />
      <FilePropertiesModal
        isOpen={isPropertiesOpen}
        onClose={onPropertiesClose}
        mediaFileId={movie.MediaFileId ?? null}
        title={movie ? movie.Title : undefined}
      />
    </div>
  );
}
