import {
  createFileRoute,
  Link,
  redirect,
  useNavigate,
} from "@tanstack/react-router";
import { useState, useEffect, useCallback } from "react";
import { useQuery, useMutation, gql } from "../../lib/graphql/client";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { Chip } from "@heroui/chip";
import { Image } from "@heroui/image";
import { Spinner } from "@heroui/spinner";
import { Breadcrumbs, BreadcrumbItem } from "@heroui/breadcrumbs";
import { useDisclosure } from "@heroui/modal";
import { addToast } from "@heroui/toast";
import { Tooltip } from "@heroui/tooltip";
import { RouteError } from "../../components/RouteError";
import { sanitizeError, formatBytes } from "../../lib/format";
import type { Movie, Library } from "../../lib/graphql/generated/graphql";
import {
  IconMovie,
  IconTrash,
  IconPlayerPlay,
  IconCalendar,
  IconClock,
  IconStar,
  IconSearch,
  IconCheck,
  IconX,
  IconRefresh,
} from "@tabler/icons-react";
import { DeleteMovieModal } from "../../components/library";
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

// MediaFile type for the nested query
interface MediaFile {
  Id: string;
  SizeBytes: number;
  Duration: number | null;
}

// GraphQL queries
const MOVIE_QUERY = gql`
  query Movie($Id: String!) {
    Movie(Id: $Id) {
      Id
      LibraryId
      Title
      SortTitle
      OriginalTitle
      Year
      TmdbId
      ImdbId
      Status
      Overview
      Tagline
      Runtime
      Genres
      Director
      CastNames
      PosterUrl
      BackdropUrl
      Monitored
      MediaFileId
      CollectionId
      CollectionName
      CollectionPosterUrl
      TmdbRating
      TmdbVoteCount
      Certification
      ReleaseDate
      ProductionCountries
      SpokenLanguages
      Library {
        Id
        Name
      }
      MediaFile {
        Id
        SizeBytes
        Duration
      }
    }
  }
`;

const REFRESH_MOVIE = gql`
  mutation RefreshMovie($id: String!) {
    RefreshMovie(id: $id) {
      success
      error
      movie {
        Id
        Title
        Overview
        Tagline
        PosterUrl
        BackdropUrl
        TmdbRating
        TmdbVoteCount
      }
    }
  }
`;

function MovieDetailPage() {
  const { movieId } = Route.useParams();
  const navigate = useNavigate();
  const [loadingPlay, setLoadingPlay] = useState(false);
  const {
    isOpen: isDeleteOpen,
    onOpen: onDeleteOpen,
    onClose: onDeleteClose,
  } = useDisclosure();
  const { startMoviePlayback } = usePlaybackContext();

  // Query movie with library and media file
  const {
    data: movieData,
    previousData: previousMovieData,
    loading: movieLoading,
    refetch,
  } = useQuery<{
    Movie: (Movie & { Library?: Library; MediaFile?: MediaFile }) | null;
  }>(MOVIE_QUERY, {
    variables: { Id: movieId },
    fetchPolicy: "cache-and-network",
  });
  const movie = movieData?.Movie ?? previousMovieData?.Movie;

  // Mutations
  const [refreshMovie, { loading: refreshing }] = useMutation<{
    refreshMovie: {
      success: boolean;
      error: string | null;
      movie: Partial<Movie> | null;
    };
  }>(REFRESH_MOVIE);

  // Update page title
  useEffect(() => {
    if (movie) {
      document.title = `Librarian - ${movie.Title}`;
    }
    return () => {
      document.title = "Librarian";
    };
  }, [movie]);

  const handlePlay = useCallback(async () => {
    if (!movie?.MediaFileId || !movie?.MediaFile) {
      addToast({
        title: "No media file",
        description: "No playable media file found for this movie",
        color: "warning",
      });
      return;
    }

    setLoadingPlay(true);
    try {
      await startMoviePlayback(
        movie.Id,
        movie.MediaFile.Id,
        movie,
        0,
        movie.MediaFile.Duration || undefined
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
  }, [movie, startMoviePlayback]);

  const handleRefresh = async () => {
    try {
      const { data } = await refreshMovie({ variables: { id: movieId } });
      if (!data?.refreshMovie?.success) {
        addToast({
          title: "Error",
          description: sanitizeError(
            data?.refreshMovie?.error || "Failed to refresh metadata"
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

  const handleDeleted = () => {
    // Navigate back to library after deletion
    navigate({
      to: "/libraries/$libraryId",
      params: { libraryId: movie?.LibraryId || "" },
    });
  };

  const handleSearchMovie = useCallback(() => {
    if (!movie) return;
    const searchQuery = movie.Year
      ? `${movie.Title} ${movie.Year}`
      : movie.Title;
    navigate({ to: "/hunt", search: { q: searchQuery, type: "movies" } });
  }, [movie, navigate]);

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
        <div className="shrink-0">
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
        </div>

        {/* Details */}
        <div className="flex-1">
          <Breadcrumbs className="mb-2">
            <BreadcrumbItem href="/libraries">Libraries</BreadcrumbItem>
            <BreadcrumbItem href={`/libraries/${movie.LibraryId}`}>
              {movie.Library?.Name || "Library"}
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
            <div className="flex items-center gap-1 shrink-0">
              <Tooltip content="Refresh Metadata & Artwork">
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
          {movie.Tagline && (
            <p className="text-default-500 italic mb-4">"{movie.Tagline}"</p>
          )}

          {/* Chips */}
          <div className="flex flex-wrap gap-2 mb-4">
            {/* File status */}
            <Chip
              size="sm"
              color={movie.MediaFileId ? "success" : "warning"}
              variant="flat"
              startContent={
                movie.MediaFileId ? (
                  <IconCheck size={14} />
                ) : (
                  <IconX size={14} />
                )
              }
            >
              {movie.MediaFileId ? "Downloaded" : "Missing"}
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
          {movie.MediaFile && movie.MediaFile.SizeBytes > 0 && (
            <div className="flex gap-4 text-sm text-default-500 mb-4">
              <div>
                <span className="font-semibold text-foreground">
                  {formatBytes(movie.MediaFile.SizeBytes)}
                </span>
                <span> on disk</span>
              </div>
            </div>
          )}

          {/* Actions */}
          <div className="flex gap-2 mt-6">
            {movie.MediaFileId ? (
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
                isDisabled={loadingPlay}
              >
                {loadingPlay ? "Loading..." : "Play"}
              </Button>
            ) : (
              <Button
                color="primary"
                startContent={<IconSearch size={16} />}
                onPress={handleSearchMovie}
              >
                Hunt for Movie
              </Button>
            )}
          </div>
        </div>
      </div>

      {/* Collection info */}
      {movie.CollectionName && (
        <Card className="bg-content1 mb-8">
          <CardBody>
            <div className="flex items-center gap-4">
              {movie.CollectionPosterUrl && (
                <Image
                  src={movie.CollectionPosterUrl}
                  alt={movie.CollectionName}
                  className="w-16 h-24 object-cover rounded"
                />
              )}
              <div>
                <h3 className="font-semibold">
                  Part of {movie.CollectionName}
                </h3>
                <p className="text-sm text-default-500">
                  View all movies in this collection
                </p>
              </div>
            </div>
          </CardBody>
        </Card>
      )}

      {/* Delete Movie Modal */}
      <DeleteMovieModal
        isOpen={isDeleteOpen}
        onClose={onDeleteClose}
        movie={movie ? { id: movie.Id, title: movie.Title } : null}
        onDeleted={handleDeleted}
      />
    </div>
  );
}
