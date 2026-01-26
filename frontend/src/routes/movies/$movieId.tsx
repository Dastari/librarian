import {
  createFileRoute,
  Link,
  redirect,
  useNavigate,
} from "@tanstack/react-router";
import { useState, useEffect, useRef } from "react";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { Chip } from "@heroui/chip";
import { Image } from "@heroui/image";
import { ShimmerLoader } from "../../components/shared/ShimmerLoader";
// PascalCase template for loading state (matches generated Movie type)
const movieTemplate: Movie = {
  Id: "template",
  LibraryId: "template",
  CreatedAt: "",
  UpdatedAt: "",
  UserId: "",
  Title: "Loading...",
  SortTitle: null,
  OriginalTitle: null,
  Year: 2024,
  TmdbId: null,
  ImdbId: null,
  Status: "RELEASED",
  Overview: "Loading...",
  Tagline: null,
  Runtime: 120,
  Genres: [],
  Director: null,
  CastNames: [],
  PosterUrl: null,
  BackdropUrl: null,
  Monitored: true,
  MediaFileId: null,
  CollectionId: null,
  CollectionName: null,
  CollectionPosterUrl: null,
  TmdbRating: null,
  TmdbVoteCount: null,
  Certification: null,
  ReleaseDate: null,
  HasFile: false,
  ProductionCountries: [],
  SpokenLanguages: [],
};
import { Breadcrumbs, BreadcrumbItem } from "@heroui/breadcrumbs";
import { useDisclosure } from "@heroui/modal";
import { addToast } from "@heroui/toast";
import { Tooltip } from "@heroui/tooltip";
import { Spinner } from "@heroui/spinner";
import { RouteError } from "../../components/RouteError";
import { sanitizeError, formatBytes } from "../../lib/format";
import { useDataReactivity } from "../../hooks/useSubscription";
import type { Movie } from "../../lib/graphql/generated/graphql";
import {
  graphqlClient,
  MOVIE_QUERY,
  LIBRARY_QUERY,
  DELETE_MOVIE_MUTATION,
  REFRESH_MOVIE_MUTATION,
  MOVIE_MEDIA_FILE_QUERY,
  type Library,
  type MediaFile,
} from "../../lib/graphql";
import {
  IconMovie,
  IconTrash,
  IconSettings,
  IconPlayerPlay,
  IconCalendar,
  IconClock,
  IconStar,
  IconSearch,
  IconCheck,
  IconX,
  IconRefresh,
} from "@tabler/icons-react";
import { ConfirmModal } from "../../components/ConfirmModal";
import { usePlaybackContext } from "../../contexts/PlaybackContext";

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

function MovieDetailPage() {
  const { movieId } = Route.useParams();
  const navigate = useNavigate();
  const [movie, setMovie] = useState<Movie | null>(null);
  const [library, setLibrary] = useState<Library | null>(null);
  const [mediaFile, setMediaFile] = useState<MediaFile | null>(null);
  const [loading, setLoading] = useState(true);
  const [deleting, setDeleting] = useState(false);
  const [loadingPlay, setLoadingPlay] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const {
    isOpen: isDeleteOpen,
    onOpen: onDeleteOpen,
    onClose: onDeleteClose,
  } = useDisclosure();
  const { startMoviePlayback } = usePlaybackContext();

  const initialLoadDone = useRef(false);

  // Update page title
  useEffect(() => {
    if (movie) {
      document.title = `Librarian - ${movie.Title}`;
    }
    return () => {
      document.title = "Librarian";
    };
  }, [movie]);

  const fetchData = async (isBackgroundRefresh = false) => {
    try {
      if (!isBackgroundRefresh) {
        setLoading(true);
      }

      const movieResult = await graphqlClient
        .query<{ Movie: Movie | null }>(MOVIE_QUERY, { Id: movieId })
        .toPromise();

      if (movieResult.data?.Movie) {
        setMovie(movieResult.data.Movie);

        // Fetch library info and media file in parallel
        const [libraryResult, mediaFileResult] = await Promise.all([
          graphqlClient
            .query<{
              Library: import("../../lib/graphql/generated/graphql").Library | null;
            }>(LIBRARY_QUERY, { Id: movieResult.data.Movie.LibraryId })
            .toPromise(),
          movieResult.data.Movie.MediaFileId
            ? graphqlClient
                .query<{ movieMediaFile: MediaFile | null }>(
                  MOVIE_MEDIA_FILE_QUERY,
                  {
                    movieId,
                  },
                )
                .toPromise()
            : Promise.resolve({ data: null }),
        ]);

        if (libraryResult.data?.Library) {
          setLibrary(libraryResult.data.Library);
        }
        if (mediaFileResult.data?.movieMediaFile) {
          setMediaFile(mediaFileResult.data.movieMediaFile);
        }
      }
    } catch (err) {
      console.error("Failed to fetch movie:", err);
    } finally {
      setLoading(false);
      initialLoadDone.current = true;
    }
  };

  useEffect(() => {
    fetchData();
  }, [movieId]);

  useDataReactivity(
    () => {
      if (initialLoadDone.current) {
        fetchData(true);
      }
    },
    { onTorrentComplete: true, periodicInterval: 30000, onFocus: true },
  );

  const handlePlay = async () => {
    if (!movie) return;

    setLoadingPlay(true);
    try {
      // If we don't have media file, fetch it first
      let fileToPlay = mediaFile;
      if (!fileToPlay) {
        const result = await graphqlClient
          .query<{
            movieMediaFile: MediaFile | null;
          }>(MOVIE_MEDIA_FILE_QUERY, { movieId })
          .toPromise();

        if (result.data?.movieMediaFile) {
          fileToPlay = result.data.movieMediaFile;
          setMediaFile(fileToPlay);
        } else {
          addToast({
            title: "No media file",
            description: "No playable media file found for this movie",
            color: "warning",
          });
          return;
        }
      }

      // Start playback using the PersistentPlayer
      // TODO: Add watch progress resume once backend returns it for movies
      await startMoviePlayback(
        movie.Id,
        fileToPlay.id,
        movie,
        0,
        fileToPlay.duration || undefined,
      );
    } catch (err) {
      console.error("Failed to start playback:", err);
      addToast({
        title: "Error",
        description: "Failed to start playback",
        color: "danger",
      });
    } finally {
      setLoadingPlay(false);
    }
  };

  const handleDelete = async () => {
    setDeleting(true);
    try {
      const { data, error } = await graphqlClient
        .mutation<{
          deleteMovie: { success: boolean; error: string | null };
        }>(DELETE_MOVIE_MUTATION, { id: movieId })
        .toPromise();

      if (error || !data?.deleteMovie.success) {
        addToast({
          title: "Error",
          description: sanitizeError(
            data?.deleteMovie.error || "Failed to delete movie",
          ),
          color: "danger",
        });
        return;
      }

      addToast({
        title: "Deleted",
        description: "Movie has been removed from library",
        color: "success",
      });

      onDeleteClose();
      navigate({
        to: "/libraries/$libraryId",
        params: { libraryId: movie?.LibraryId || "" },
      });
    } catch (err) {
      console.error("Failed to delete movie:", err);
      addToast({
        title: "Error",
        description: "Failed to delete movie",
        color: "danger",
      });
    } finally {
      setDeleting(false);
    }
  };

  const handleRefresh = async () => {
    if (!movie) return;

    setRefreshing(true);
    try {
      const { data, error } = await graphqlClient
        .mutation<{
          refreshMovie: {
            success: boolean;
            movie: Movie | null;
            error: string | null;
          };
        }>(REFRESH_MOVIE_MUTATION, { id: movieId })
        .toPromise();

      if (error || !data?.refreshMovie.success) {
        addToast({
          title: "Error",
          description: sanitizeError(
            data?.refreshMovie.error || "Failed to refresh metadata",
          ),
          color: "danger",
        });
        return;
      }

      // Update local state with refreshed movie data
      if (data.refreshMovie.movie) {
        setMovie((prev) =>
          prev
            ? {
                ...prev,
                ...data.refreshMovie.movie,
              }
            : null,
        );
      }

      addToast({
        title: "Refreshed",
        description: "Movie metadata and artwork updated",
        color: "success",
      });
    } catch (err) {
      console.error("Failed to refresh movie:", err);
      addToast({
        title: "Error",
        description: "Failed to refresh metadata",
        color: "danger",
      });
    } finally {
      setRefreshing(false);
    }
  };

  // Show not found state only after loading is complete
  if (!loading && !movie) {
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

  // Use template data during loading, real data when available
  const displayMovie = movie ?? movieTemplate;

  return (
    <ShimmerLoader loading={loading} templateProps={{ movie: movieTemplate }}>
      <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 mb-20">
        {/* Header */}
        <div className="flex flex-col md:flex-row gap-6 mb-8">
          {/* Poster */}
          <div className="shrink-0">
            {displayMovie.PosterUrl ? (
              <Image
                src={displayMovie.PosterUrl}
                alt={displayMovie.Title}
                className="w-64 h-96 object-cover rounded-lg shadow-lg"
              />
            ) : (
              <div className="w-64 h-96 bg-default-200 rounded-lg flex items-center justify-center">
                <IconMovie size={64} className="text-purple-400" />
              </div>
            )}
          </div>

          {/* Details */}
          <div className="flex-1">
            <Breadcrumbs className="mb-2">
              <BreadcrumbItem href="/libraries">Libraries</BreadcrumbItem>
              <BreadcrumbItem href={`/libraries/${displayMovie.LibraryId}`}>
                {library?.Name || "Library"}
              </BreadcrumbItem>
              <BreadcrumbItem isCurrent>{displayMovie.Title}</BreadcrumbItem>
            </Breadcrumbs>

            <div className="flex items-start justify-between gap-4 mb-2">
              <h1 className="text-3xl font-bold">
                {displayMovie.Title}
                {displayMovie.Year && (
                  <span className="text-default-500 ml-2">
                    ({displayMovie.Year})
                  </span>
                )}
              </h1>
              <div className="flex items-center gap-1 shrink-0">
                <Tooltip content="Refresh Metadata & Artwork">
                  <Button
                    isIconOnly
                    variant="light"
                    size="sm"
                    onPress={handleRefresh}
                    isDisabled={refreshing || loading}
                  >
                    {refreshing ? (
                      <Spinner size="sm" />
                    ) : (
                      <IconRefresh size={18} />
                    )}
                  </Button>
                </Tooltip>
                <Tooltip content="Settings">
                  <Button isIconOnly variant="light" size="sm">
                    <IconSettings size={18} />
                  </Button>
                </Tooltip>
                <Tooltip content="Delete Movie">
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

            {/* Tagline */}
            {displayMovie.Tagline && (
              <p className="text-default-500 italic mb-4">
                "{displayMovie.Tagline}"
              </p>
            )}

            {/* Chips */}
            <div className="flex flex-wrap gap-2 mb-4">
              {/* File status */}
              <Chip
                size="sm"
                color={displayMovie.MediaFileId ? "success" : "warning"}
                variant="flat"
                startContent={
                  displayMovie.MediaFileId ? (
                    <IconCheck size={14} />
                  ) : (
                    <IconX size={14} />
                  )
                }
              >
                {displayMovie.MediaFileId ? "Downloaded" : "Missing"}
              </Chip>

              {/* Rating */}
              {displayMovie.TmdbRating && Number(displayMovie.TmdbRating) > 0 && (
                <Chip
                  size="sm"
                  variant="flat"
                  color={
                    Number(displayMovie.TmdbRating) >= 7
                      ? "success"
                      : Number(displayMovie.TmdbRating) >= 5
                        ? "warning"
                        : "danger"
                  }
                  startContent={<IconStar size={14} />}
                >
                  {Number(displayMovie.TmdbRating).toFixed(1)} (
                  {displayMovie.TmdbVoteCount?.toLocaleString()} votes)
                </Chip>
              )}

              {/* Certification */}
              {displayMovie.Certification && (
                <Chip size="sm" variant="flat">
                  {displayMovie.Certification}
                </Chip>
              )}

              {/* Runtime */}
              {displayMovie.Runtime && (
                <Chip
                  size="sm"
                  variant="flat"
                  startContent={<IconClock size={14} />}
                >
                  {Math.floor(displayMovie.Runtime / 60)}h{" "}
                  {displayMovie.Runtime % 60}m
                </Chip>
              )}

              {/* Release date */}
              {displayMovie.ReleaseDate && (
                <Chip
                  size="sm"
                  variant="flat"
                  startContent={<IconCalendar size={14} />}
                >
                  {new Date(displayMovie.ReleaseDate).toLocaleDateString()}
                </Chip>
              )}
            </div>

            {/* Genres */}
            {displayMovie.Genres.length > 0 && (
              <div className="flex flex-wrap gap-1 mb-4">
                {displayMovie.Genres.map((genre, index) => (
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
            {displayMovie.Overview && (
              <p className="text-default-600 mb-4 line-clamp-4">
                {displayMovie.Overview}
              </p>
            )}

            {/* Credits */}
            <div className="flex gap-8 text-sm mb-4">
              {displayMovie.Director && (
                <div>
                  <span className="text-default-500">Director:</span>{" "}
                  <span className="font-medium">{displayMovie.Director}</span>
                </div>
              )}
              {displayMovie.CastNames.length > 0 && (
                <div>
                  <span className="text-default-500">Cast:</span>{" "}
                  <span className="font-medium">
                    {displayMovie.CastNames.slice(0, 3).join(", ")}
                  </span>
                </div>
              )}
            </div>

            {/* Stats */}
            <div className="flex gap-4 text-sm text-default-500">
              {mediaFile && mediaFile.sizeBytes > 0 && (
                <div>
                  <span className="font-semibold text-foreground">
                    {formatBytes(mediaFile.sizeBytes)}
                  </span>
                  <span> on disk</span>
                </div>
              )}
            </div>

            {/* Actions */}
            <div className="flex gap-2 mt-6">
              {displayMovie.MediaFileId ? (
                <Button
                  color="success"
                  startContent={
                    loadingPlay ? (
                      <Spinner size="sm" color="current" />
                    ) : (
                      <IconPlayerPlay size={16} />
                    )
                  }
                  onPress={handlePlay}
                  isDisabled={loadingPlay || loading}
                >
                  {loadingPlay ? "Loading..." : "Play"}
                </Button>
              ) : (
                <Button
                  color="primary"
                  startContent={<IconSearch size={16} />}
                  isDisabled={loading}
                  onPress={() => {
                    // Build search query: "Movie Title (Year)"
                    const searchQuery = displayMovie.Year
                      ? `${displayMovie.Title} ${displayMovie.Year}`
                      : displayMovie.Title;
                    navigate({
                      to: "/hunt",
                      search: {
                        q: searchQuery,
                        type: "movies",
                      },
                    });
                  }}
                >
                  Hunt for Movie
                </Button>
              )}
            </div>
          </div>
        </div>

        {/* Collection info */}
        {displayMovie.CollectionName && (
          <Card className="bg-content1 mb-8">
            <CardBody>
              <div className="flex items-center gap-4">
                {displayMovie.CollectionPosterUrl && (
                  <Image
                    src={displayMovie.CollectionPosterUrl}
                    alt={displayMovie.CollectionName}
                    className="w-16 h-24 object-cover rounded"
                  />
                )}
                <div>
                  <h3 className="font-semibold">
                    Part of {displayMovie.CollectionName}
                  </h3>
                  <p className="text-sm text-default-500">
                    View all movies in this collection
                  </p>
                </div>
              </div>
            </CardBody>
          </Card>
        )}

        {/* Delete Confirmation */}
        {movie && (
          <ConfirmModal
            isOpen={isDeleteOpen}
            onClose={onDeleteClose}
            onConfirm={handleDelete}
            title="Delete Movie"
            message={`Are you sure you want to delete "${movie.Title}"?`}
            description="This will remove the movie from your library. Downloaded files will not be deleted."
            confirmLabel="Delete"
            confirmColor="danger"
            isLoading={deleting}
          />
        )}
      </div>
    </ShimmerLoader>
  );
}
