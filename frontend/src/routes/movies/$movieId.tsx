import { createFileRoute, Link, redirect, useNavigate } from '@tanstack/react-router'
import { useState, useEffect, useRef } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Chip } from '@heroui/chip'
import { Image } from '@heroui/image'
import { ShimmerLoader } from '../../components/shared/ShimmerLoader'
import { movieTemplate } from '../../lib/template-data'
import { Breadcrumbs, BreadcrumbItem } from '@heroui/breadcrumbs'
import { useDisclosure } from '@heroui/modal'
import { addToast } from '@heroui/toast'
import { Tooltip } from '@heroui/tooltip'
import { Spinner } from '@heroui/spinner'
import { RouteError } from '../../components/RouteError'
import { sanitizeError, formatBytes } from '../../lib/format'
import { useDataReactivity } from '../../hooks/useSubscription'
import {
  graphqlClient,
  MOVIE_QUERY,
  LIBRARY_QUERY,
  DELETE_MOVIE_MUTATION,
  MOVIE_MEDIA_FILE_QUERY,
  type Movie,
  type Library,
  type MediaFile,
} from '../../lib/graphql'
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
} from '@tabler/icons-react'
import { ConfirmModal } from '../../components/ConfirmModal'
import { usePlaybackContext } from '../../contexts/PlaybackContext'

export const Route = createFileRoute('/movies/$movieId')({
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
  component: MovieDetailPage,
  errorComponent: RouteError,
})

function MovieDetailPage() {
  const { movieId } = Route.useParams()
  const navigate = useNavigate()
  const [movie, setMovie] = useState<Movie | null>(null)
  const [library, setLibrary] = useState<Library | null>(null)
  const [mediaFile, setMediaFile] = useState<MediaFile | null>(null)
  const [loading, setLoading] = useState(true)
  const [deleting, setDeleting] = useState(false)
  const [loadingPlay, setLoadingPlay] = useState(false)
  const { isOpen: isDeleteOpen, onOpen: onDeleteOpen, onClose: onDeleteClose } = useDisclosure()
  const { startMoviePlayback } = usePlaybackContext()
  
  const initialLoadDone = useRef(false)

  // Update page title
  useEffect(() => {
    if (movie) {
      document.title = `Librarian - ${movie.title}`
    }
    return () => {
      document.title = 'Librarian'
    }
  }, [movie])

  const fetchData = async (isBackgroundRefresh = false) => {
    try {
      if (!isBackgroundRefresh) {
        setLoading(true)
      }

      const movieResult = await graphqlClient
        .query<{ movie: Movie | null }>(MOVIE_QUERY, { id: movieId })
        .toPromise()

      if (movieResult.data?.movie) {
        setMovie(movieResult.data.movie)

        // Fetch library info and media file in parallel
        const [libraryResult, mediaFileResult] = await Promise.all([
          graphqlClient
            .query<{ library: Library | null }>(LIBRARY_QUERY, {
              id: movieResult.data.movie.libraryId,
            })
            .toPromise(),
          movieResult.data.movie.mediaFileId
            ? graphqlClient
                .query<{ movieMediaFile: MediaFile | null }>(MOVIE_MEDIA_FILE_QUERY, {
                  movieId,
                })
                .toPromise()
            : Promise.resolve({ data: null }),
        ])

        if (libraryResult.data?.library) {
          setLibrary(libraryResult.data.library)
        }
        if (mediaFileResult.data?.movieMediaFile) {
          setMediaFile(mediaFileResult.data.movieMediaFile)
        }
      }
    } catch (err) {
      console.error('Failed to fetch movie:', err)
    } finally {
      setLoading(false)
      initialLoadDone.current = true
    }
  }

  useEffect(() => {
    fetchData()
  }, [movieId])

  useDataReactivity(
    () => {
      if (initialLoadDone.current) {
        fetchData(true)
      }
    },
    { onTorrentComplete: true, periodicInterval: 30000, onFocus: true }
  )

  const handlePlay = async () => {
    if (!movie) return
    
    setLoadingPlay(true)
    try {
      // If we don't have media file, fetch it first
      let fileToPlay = mediaFile
      if (!fileToPlay) {
        const result = await graphqlClient
          .query<{ movieMediaFile: MediaFile | null }>(MOVIE_MEDIA_FILE_QUERY, { movieId })
          .toPromise()
        
        if (result.data?.movieMediaFile) {
          fileToPlay = result.data.movieMediaFile
          setMediaFile(fileToPlay)
        } else {
          addToast({
            title: 'No media file',
            description: 'No playable media file found for this movie',
            color: 'warning',
          })
          return
        }
      }

      // Start playback using the PersistentPlayer
      // TODO: Add watch progress resume once backend returns it for movies
      await startMoviePlayback(movie.id, fileToPlay.id, movie, 0, fileToPlay.duration || undefined)
    } catch (err) {
      console.error('Failed to start playback:', err)
      addToast({
        title: 'Error',
        description: 'Failed to start playback',
        color: 'danger',
      })
    } finally {
      setLoadingPlay(false)
    }
  }

  const handleDelete = async () => {
    setDeleting(true)
    try {
      const { data, error } = await graphqlClient
        .mutation<{ deleteMovie: { success: boolean; error: string | null } }>(
          DELETE_MOVIE_MUTATION,
          { id: movieId }
        )
        .toPromise()

      if (error || !data?.deleteMovie.success) {
        addToast({
          title: 'Error',
          description: sanitizeError(data?.deleteMovie.error || 'Failed to delete movie'),
          color: 'danger',
        })
        return
      }

      addToast({
        title: 'Deleted',
        description: 'Movie has been removed from library',
        color: 'success',
      })

      onDeleteClose()
      navigate({ to: '/libraries/$libraryId', params: { libraryId: movie?.libraryId || '' } })
    } catch (err) {
      console.error('Failed to delete movie:', err)
      addToast({
        title: 'Error',
        description: 'Failed to delete movie',
        color: 'danger',
      })
    } finally {
      setDeleting(false)
    }
  }

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
    )
  }

  // Use template data during loading, real data when available
  const displayMovie = movie ?? movieTemplate

  return (
    <ShimmerLoader loading={loading} templateProps={{ movie: movieTemplate }}>
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 mb-20">
      {/* Header */}
      <div className="flex flex-col md:flex-row gap-6 mb-8">
        {/* Poster */}
        <div className="shrink-0">
          {displayMovie.posterUrl ? (
            <Image
              src={displayMovie.posterUrl}
              alt={displayMovie.title}
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
            <BreadcrumbItem href={`/libraries/${displayMovie.libraryId}`}>
              {library?.name || 'Library'}
            </BreadcrumbItem>
            <BreadcrumbItem isCurrent>{displayMovie.title}</BreadcrumbItem>
          </Breadcrumbs>

          <div className="flex items-start justify-between gap-4 mb-2">
            <h1 className="text-3xl font-bold">
              {displayMovie.title}
              {displayMovie.year && (
                <span className="text-default-500 ml-2">({displayMovie.year})</span>
              )}
            </h1>
            <div className="flex items-center gap-1 shrink-0">
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
          {displayMovie.tagline && (
            <p className="text-default-500 italic mb-4">"{displayMovie.tagline}"</p>
          )}

          {/* Chips */}
          <div className="flex flex-wrap gap-2 mb-4">
            {/* File status */}
            <Chip
              size="sm"
              color={displayMovie.mediaFileId ? 'success' : 'warning'}
              variant="flat"
              startContent={displayMovie.mediaFileId ? <IconCheck size={14} /> : <IconX size={14} />}
            >
              {displayMovie.mediaFileId ? 'Downloaded' : 'Missing'}
            </Chip>

            {/* Rating */}
            {displayMovie.tmdbRating && displayMovie.tmdbRating > 0 && (
              <Chip
                size="sm"
                variant="flat"
                color={displayMovie.tmdbRating >= 7 ? 'success' : displayMovie.tmdbRating >= 5 ? 'warning' : 'danger'}
                startContent={<IconStar size={14} />}
              >
                {displayMovie.tmdbRating.toFixed(1)} ({displayMovie.tmdbVoteCount?.toLocaleString()} votes)
              </Chip>
            )}

            {/* Certification */}
            {displayMovie.certification && (
              <Chip size="sm" variant="flat">
                {displayMovie.certification}
              </Chip>
            )}

            {/* Runtime */}
            {displayMovie.runtime && (
              <Chip size="sm" variant="flat" startContent={<IconClock size={14} />}>
                {Math.floor(displayMovie.runtime / 60)}h {displayMovie.runtime % 60}m
              </Chip>
            )}

            {/* Release date */}
            {displayMovie.releaseDate && (
              <Chip size="sm" variant="flat" startContent={<IconCalendar size={14} />}>
                {new Date(displayMovie.releaseDate).toLocaleDateString()}
              </Chip>
            )}
          </div>

          {/* Genres */}
          {displayMovie.genres.length > 0 && (
            <div className="flex flex-wrap gap-1 mb-4">
              {displayMovie.genres.map((genre) => (
                <Chip key={genre} size="sm" variant="bordered" className="text-xs">
                  {genre}
                </Chip>
              ))}
            </div>
          )}

          {/* Overview */}
          {displayMovie.overview && (
            <p className="text-default-600 mb-4 line-clamp-4">{displayMovie.overview}</p>
          )}

          {/* Credits */}
          <div className="flex gap-8 text-sm mb-4">
            {displayMovie.director && (
              <div>
                <span className="text-default-500">Director:</span>{' '}
                <span className="font-medium">{displayMovie.director}</span>
              </div>
            )}
            {displayMovie.castNames.length > 0 && (
              <div>
                <span className="text-default-500">Cast:</span>{' '}
                <span className="font-medium">{displayMovie.castNames.slice(0, 3).join(', ')}</span>
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
            {displayMovie.mediaFileId ? (
              <Button
                color="success"
                startContent={loadingPlay ? <Spinner size="sm" color="current" /> : <IconPlayerPlay size={16} />}
                onPress={handlePlay}
                isDisabled={loadingPlay || loading}
              >
                {loadingPlay ? 'Loading...' : 'Play'}
              </Button>
            ) : (
              <Button
                color="primary"
                startContent={<IconSearch size={16} />}
                isDisabled={loading}
                onPress={() => {
                  // Build search query: "Movie Title (Year)"
                  const searchQuery = displayMovie.year ? `${displayMovie.title} ${displayMovie.year}` : displayMovie.title
                  navigate({
                    to: '/hunt',
                    search: {
                      q: searchQuery,
                      type: 'movies',
                    },
                  })
                }}
              >
                Hunt for Movie
              </Button>
            )}
          </div>
        </div>
      </div>

      {/* Collection info */}
      {displayMovie.collectionName && (
        <Card className="bg-content1 mb-8">
          <CardBody>
            <div className="flex items-center gap-4">
              {displayMovie.collectionPosterUrl && (
                <Image
                  src={displayMovie.collectionPosterUrl}
                  alt={displayMovie.collectionName}
                  className="w-16 h-24 object-cover rounded"
                />
              )}
              <div>
                <h3 className="font-semibold">Part of {displayMovie.collectionName}</h3>
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
          message={`Are you sure you want to delete "${movie.title}"?`}
          description="This will remove the movie from your library. Downloaded files will not be deleted."
          confirmLabel="Delete"
          confirmColor="danger"
          isLoading={deleting}
        />
      )}
    </div>
    </ShimmerLoader>
  )
}
