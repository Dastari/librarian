import { useMemo, useState, useCallback, useEffect } from 'react'
import { Button, ButtonGroup } from '@heroui/button'
import { Chip } from '@heroui/chip'
import { Image } from '@heroui/image'
import { Spinner } from '@heroui/spinner'
import { Card, CardBody } from '@heroui/card'
import { Link } from '@tanstack/react-router'
import {
  DataTable,
  type DataTableColumn,
  type RowAction,
  type CardRendererProps,
} from '../data-table'
import { graphqlClient, MOVIES_QUERY, type Movie } from '../../lib/graphql'
import { formatBytes } from '../../lib/format'
import { IconPlus, IconTrash, IconEye, IconMovie, IconClock, IconStar } from '@tabler/icons-react'
import { MovieCard } from './MovieCard'

// ============================================================================
// Constants
// ============================================================================

const ALPHABET = '#ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('')

// ============================================================================
// Utility Functions
// ============================================================================

function getFirstLetter(title: string): string {
  // Skip common articles for sorting
  const titleLower = title.toLowerCase()
  let sortTitle = title
  for (const article of ['the ', 'a ', 'an ']) {
    if (titleLower.startsWith(article)) {
      sortTitle = title.slice(article.length)
      break
    }
  }
  const firstChar = sortTitle.charAt(0).toUpperCase()
  return /[A-Z]/.test(firstChar) ? firstChar : '#'
}

// ============================================================================
// Component Props
// ============================================================================

interface LibraryMoviesTabProps {
  libraryId: string
  onDeleteMovie: (movieId: string, movieTitle: string) => void
  onAddMovie: () => void
}

// ============================================================================
// Main Component
// ============================================================================

export function LibraryMoviesTab({ libraryId, onDeleteMovie, onAddMovie }: LibraryMoviesTabProps) {
  const [selectedLetter, setSelectedLetter] = useState<string | null>(null)
  const [movies, setMovies] = useState<Movie[]>([])
  const [loading, setLoading] = useState(true)

  const fetchMovies = useCallback(async () => {
    try {
      setLoading(true)
      const result = await graphqlClient
        .query<{ movies: Movie[] }>(MOVIES_QUERY, { libraryId })
        .toPromise()

      if (result.data?.movies) {
        setMovies(result.data.movies)
      }
    } catch (err) {
      console.error('Failed to fetch movies:', err)
    } finally {
      setLoading(false)
    }
  }, [libraryId])

  useEffect(() => {
    fetchMovies()
  }, [fetchMovies])

  // Get letters that have movies
  const availableLetters = useMemo(() => {
    const letters = new Set<string>()
    movies.forEach((movie) => {
      letters.add(getFirstLetter(movie.title))
    })
    return letters
  }, [movies])

  // Filter movies by selected letter
  const filteredMovies = useMemo(() => {
    if (!selectedLetter) return movies
    return movies.filter((movie) => getFirstLetter(movie.title) === selectedLetter)
  }, [movies, selectedLetter])

  // Handle letter click - toggle filter
  const handleLetterClick = (letter: string) => {
    setSelectedLetter((prev) => (prev === letter ? null : letter))
  }

  // Column definitions
  const columns: DataTableColumn<Movie>[] = useMemo(
    () => [
      {
        key: 'title',
        label: 'MOVIE',
        sortable: true,
        render: (movie) => (
          <Link to="/movies/$movieId" params={{ movieId: movie.id }} className="flex items-center gap-3 hover:opacity-80">
            {movie.posterUrl ? (
              <Image
                src={movie.posterUrl}
                alt={movie.title}
                className="w-10 h-14 object-cover rounded"
              />
            ) : (
              <div className="w-10 h-14 bg-default-200 rounded flex items-center justify-center">
                <IconMovie size={20} className="text-purple-400" />
              </div>
            )}
            <div>
              <p className="font-medium">{movie.title}</p>
              {movie.genres.length > 0 && (
                <p className="text-xs text-default-400">
                  {movie.genres.slice(0, 2).join(', ')}
                </p>
              )}
            </div>
          </Link>
        ),
        sortFn: (a, b) => a.title.localeCompare(b.title),
      },
      {
        key: 'year',
        label: 'YEAR',
        width: 80,
        sortable: true,
        render: (movie) => <span>{movie.year || '—'}</span>,
        sortFn: (a, b) => (a.year || 0) - (b.year || 0),
      },
      {
        key: 'runtime',
        label: 'RUNTIME',
        width: 100,
        sortable: true,
        render: (movie) => (
          <span className="flex items-center gap-1">
            {movie.runtime ? (
              <>
                <IconClock size={14} className="text-default-400" />
                {Math.floor(movie.runtime / 60)}h {movie.runtime % 60}m
              </>
            ) : '—'}
          </span>
        ),
        sortFn: (a, b) => (a.runtime || 0) - (b.runtime || 0),
      },
      {
        key: 'rating',
        label: 'RATING',
        width: 100,
        sortable: true,
        render: (movie) => (
          movie.tmdbRating && movie.tmdbRating > 0 ? (
            <Chip
              size="sm"
              variant="flat"
              color={movie.tmdbRating >= 7 ? 'success' : movie.tmdbRating >= 5 ? 'warning' : 'danger'}
              startContent={<IconStar size={12} />}
            >
              {movie.tmdbRating.toFixed(1)}
            </Chip>
          ) : <span>—</span>
        ),
        sortFn: (a, b) => (a.tmdbRating || 0) - (b.tmdbRating || 0),
      },
      {
        key: 'size',
        label: 'SIZE',
        width: 100,
        sortable: true,
        render: (movie) => <span>{movie.hasFile ? formatBytes(movie.sizeBytes) : '—'}</span>,
        sortFn: (a, b) => a.sizeBytes - b.sizeBytes,
      },
      {
        key: 'status',
        label: 'STATUS',
        width: 120,
        sortable: true,
        render: (movie) => (
          <Chip
            size="sm"
            color={movie.hasFile ? 'success' : 'warning'}
            variant="flat"
          >
            {movie.hasFile ? 'Downloaded' : 'Missing'}
          </Chip>
        ),
        sortFn: (a, b) => (a.hasFile === b.hasFile ? 0 : a.hasFile ? -1 : 1),
      },
    ],
    []
  )

  // Row actions
  const rowActions: RowAction<Movie>[] = useMemo(
    () => [
      {
        key: 'view',
        label: 'View',
        icon: <IconEye size={16} />,
        inDropdown: true,
        onAction: () => {
          // Navigation is handled by the Link component in the column
        },
      },
      {
        key: 'delete',
        label: 'Delete',
        icon: <IconTrash size={16} className="text-red-400" />,
        isDestructive: true,
        inDropdown: true,
        onAction: (movie) => onDeleteMovie(movie.id, movie.title),
      },
    ],
    [onDeleteMovie]
  )

  // Search function
  const searchFn = (movie: Movie, term: string) => {
    const lowerTerm = term.toLowerCase()
    return (
      movie.title.toLowerCase().includes(lowerTerm) ||
      (movie.director?.toLowerCase().includes(lowerTerm) ?? false) ||
      movie.genres.some((g) => g.toLowerCase().includes(lowerTerm))
    )
  }

  // Card renderer - using the MovieCard component
  const cardRenderer = useCallback(
    ({ item }: CardRendererProps<Movie>) => (
      <MovieCard
        movie={item}
        onDelete={() => onDeleteMovie(item.id, item.title)}
      />
    ),
    [onDeleteMovie]
  )

  if (loading) {
    return (
      <div className="flex justify-center items-center py-12">
        <Spinner size="lg" />
      </div>
    )
  }

  return (
    <div className="flex flex-col grow w-full">
      {/* A-Z Navigation - Sticky at top */}
      <div className="flex items-center p-2 bg-content2 rounded-lg overflow-x-auto shrink-0 mb-4">
        <ButtonGroup size="sm" variant="flat">
          <Button
            variant={selectedLetter === null ? 'solid' : 'flat'}
            color={selectedLetter === null ? 'primary' : 'default'}
            onPress={() => setSelectedLetter(null)}
            className="min-w-8 px-2"
          >
            All
          </Button>
          {ALPHABET.map((letter) => {
            const hasMovies = availableLetters.has(letter)
            const isSelected = selectedLetter === letter
            return (
              <Button
                key={letter}
                variant={isSelected ? 'solid' : 'flat'}
                color={isSelected ? 'primary' : 'default'}
                onPress={() => hasMovies && handleLetterClick(letter)}
                isDisabled={!hasMovies}
                className="w-4 min-w-4 lg:w-6 lg:min-w-6 p-0 text-xs font-medium xl:min-w-7 xl:w-7"
              >
                {letter}
              </Button>
            )
          })}
        </ButtonGroup>
      </div>

      {/* Data Table - Fills remaining height with sticky header */}
      <div className="flex-1 min-h-0">
        <DataTable
          stateKey="library-movies"
          data={filteredMovies}
          columns={columns}
          getRowKey={(movie) => movie.id}
          searchFn={searchFn}
          searchPlaceholder="Search movies..."
          defaultSortColumn="title"
          showViewModeToggle
          defaultViewMode="cards"
          cardRenderer={cardRenderer}
          cardGridClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"
          rowActions={rowActions}
          showItemCount
          ariaLabel="Movies table"
          fillHeight
          emptyContent={
            <Card className="bg-content1/50 border-default-300 border-dashed border-2">
              <CardBody className="py-12 text-center">
                <IconMovie size={48} className="mx-auto mb-4 text-purple-400" />
                <h3 className="text-lg font-semibold mb-2">No movies yet</h3>
                <p className="text-default-500 mb-4">
                  Add movies to this library to start building your collection.
                </p>
                <Button color="primary" onPress={onAddMovie}>
                  Add Movie
                </Button>
              </CardBody>
            </Card>
          }
          toolbarContent={
            <Button color="primary" size="sm" onPress={onAddMovie} isIconOnly>
              <IconPlus size={16} />
            </Button>
          }
          toolbarContentPosition="end"
        />
      </div>
    </div>
  )
}
