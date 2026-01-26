import { useMemo, useCallback } from 'react'
import { useQueryState, parseAsString, parseAsStringLiteral } from 'nuqs'
import { Button } from '@heroui/button'
import { Chip } from '@heroui/chip'
import { Image } from '@heroui/image'
import { Card, CardBody } from '@heroui/card'
import { Link } from '@tanstack/react-router'
import {
  DataTable,
  AlphabetFilter,
  getFirstLetter,
  type DataTableColumn,
  type RowAction,
  type CardRendererProps,
} from '../data-table'
import type { Movie } from '../../lib/graphql/generated/graphql'
import { MOVIES_CONNECTION_QUERY } from '../../lib/graphql'
import { IconPlus, IconTrash, IconEye, IconMovie, IconClock, IconStar } from '@tabler/icons-react'
import { MovieCard } from './MovieCard'
import { MediaCardSkeleton } from './MediaCardSkeleton'
import { useInfiniteConnection } from '../../hooks/useInfiniteConnection'

// ============================================================================
// Component Props
// ============================================================================

interface LibraryMoviesTabProps {
  libraryId: string
  /** Parent loading state (e.g., library context still loading) */
  loading?: boolean
  onDeleteMovie: (movieId: string, movieTitle: string) => void
  onAddMovie: () => void
}

// ============================================================================
// Types for GraphQL response
// ============================================================================

interface MoviesConnectionResponse {
  Movies: {
    Edges: Array<{ Node: Movie; Cursor: string }>
    PageInfo: {
      HasNextPage: boolean
      HasPreviousPage: boolean
      StartCursor: string | null
      EndCursor: string | null
      TotalCount: number | null
    }
  }
}

// ============================================================================
// Main Component
// ============================================================================

// Map column keys to GraphQL MovieOrderByInput field names
const SORT_FIELD_MAP: Record<string, string> = {
  title: 'SortTitle',
  year: 'Year',
  runtime: 'Runtime',
  rating: 'SortTitle',
  size: 'Runtime',
}

export function LibraryMoviesTab({ libraryId, loading: parentLoading, onDeleteMovie, onAddMovie }: LibraryMoviesTabProps) {
  // URL-persisted state via nuqs (clean URLs when using defaults)
  const [selectedLetter, setSelectedLetter] = useQueryState('letter', parseAsString.withDefault(''))
  const [searchTerm, setSearchTerm] = useQueryState('q', parseAsString.withDefault(''))
  const [sortColumn, setSortColumn] = useQueryState('sort', parseAsString.withDefault('title'))
  const [sortDirection, setSortDirection] = useQueryState(
    'order',
    parseAsStringLiteral(['asc', 'desc'] as const).withDefault('asc')
  )
  
  // Normalize selectedLetter: empty string becomes null for the filter logic
  const normalizedLetter = selectedLetter === '' ? null : selectedLetter

  // Check if we should skip queries (loading or template ID)
  const shouldSkipQueries = parentLoading || libraryId.startsWith('template')

  // Handle sort change from DataTable
  const handleSortChange = useCallback((column: string, direction: 'asc' | 'desc') => {
    setSortColumn(column)
    setSortDirection(direction)
  }, [setSortColumn, setSortDirection])

  // Build filter variables for GraphQL query (PascalCase schema)
  const queryVariables = useMemo(() => {
    const where: Record<string, unknown> = { LibraryId: { Eq: libraryId } }
    if (searchTerm) {
      where.Title = { Contains: searchTerm }
    }
    const graphqlField = SORT_FIELD_MAP[sortColumn || 'title'] || 'SortTitle'
    const orderBy = [{ [graphqlField]: sortDirection === 'asc' ? 'Asc' : 'Desc' }]
    return {
      Where: where,
      Page: { Limit: 500 },
      OrderBy: orderBy,
    }
  }, [libraryId, searchTerm, sortColumn, sortDirection])

  // Use infinite connection hook; map schema response to Connection shape
  const {
    items: movies,
    isLoading,
    isLoadingMore,
    hasMore,
    totalCount,
    loadMore,
  } = useInfiniteConnection<MoviesConnectionResponse, Movie>({
    query: MOVIES_CONNECTION_QUERY,
    variables: queryVariables,
    getConnection: (data) => ({
      edges: data.Movies.Edges.map((e) => ({ node: e.Node, cursor: e.Cursor })),
      pageInfo: {
        hasNextPage: data.Movies.PageInfo.HasNextPage,
        hasPreviousPage: data.Movies.PageInfo.HasPreviousPage,
        startCursor: data.Movies.PageInfo.StartCursor ?? null,
        endCursor: data.Movies.PageInfo.EndCursor ?? null,
        totalCount: data.Movies.PageInfo.TotalCount ?? null,
      },
    }),
    batchSize: 50,
    enabled: !shouldSkipQueries,
    deps: [libraryId, searchTerm],
  })

  // Get letters that have movies (from loaded data)
  const availableLetters = useMemo(() => {
    const letters = new Set<string>()
    movies.forEach((movie) => {
      letters.add(getFirstLetter(movie.Title))
    })
    return letters
  }, [movies])

  // Filter movies by selected letter (client-side for alphabet filter)
  const filteredMovies = useMemo(() => {
    if (!normalizedLetter) return movies
    return movies.filter((movie) => getFirstLetter(movie.Title) === normalizedLetter)
  }, [movies, normalizedLetter])

  // Handle letter change - toggle filter
  const handleLetterChange = useCallback((letter: string | null) => {
    setSelectedLetter(normalizedLetter === letter ? '' : (letter ?? ''))
  }, [normalizedLetter, setSelectedLetter])

  // Handle search change from DataTable
  const handleSearchChange = useCallback((term: string) => {
    setSearchTerm(term || '')
    setSelectedLetter('') // Reset letter filter on search
  }, [setSearchTerm, setSelectedLetter])

  // Column definitions
  const columns: DataTableColumn<Movie>[] = useMemo(
    () => [
      {
        key: 'title',
        label: 'MOVIE',
        // sortable: true (default) - server handles actual sorting
        render: (movie) => (
          <Link to="/movies/$movieId" params={{ movieId: movie.Id }} className="flex items-center gap-3 hover:opacity-80">
            {movie.PosterUrl ? (
              <Image
                src={movie.PosterUrl}
                alt={movie.Title}
                className="w-10 h-14 object-cover rounded"
              />
            ) : (
              <div className="w-10 h-14 bg-default-200 rounded flex items-center justify-center">
                <IconMovie size={20} className="text-purple-400" />
              </div>
            )}
            <div>
              <p className="font-medium">{movie.Title}</p>
              {movie.Genres && movie.Genres.length > 0 && (
                <p className="text-xs text-default-400">
                  {movie.Genres.slice(0, 2).join(', ')}
                </p>
              )}
            </div>
          </Link>
        ),
      },
      {
        key: 'year',
        label: 'YEAR',
        width: 80,
        render: (movie) => <span>{movie.Year ?? '—'}</span>,
      },
      {
        key: 'runtime',
        label: 'RUNTIME',
        width: 100,
        render: (movie) => (
          <span className="flex items-center gap-1">
            {movie.Runtime != null ? (
              <>
                <IconClock size={14} className="text-default-400" />
                {Math.floor(movie.Runtime / 60)}h {movie.Runtime % 60}m
              </>
            ) : '—'}
          </span>
        ),
      },
      {
        key: 'rating',
        label: 'RATING',
        width: 100,
        render: (movie) => (
          movie.TmdbRating && Number(movie.TmdbRating) > 0 ? (
            <Chip
              size="sm"
              variant="flat"
              color={Number(movie.TmdbRating) >= 7 ? 'success' : Number(movie.TmdbRating) >= 5 ? 'warning' : 'danger'}
              startContent={<IconStar size={12} />}
            >
              {Number(movie.TmdbRating).toFixed(1)}
            </Chip>
          ) : <span>—</span>
        ),
      },
      {
        key: 'status',
        label: 'STATUS',
        width: 120,
        sortable: false, // Status is not sortable
        render: (movie) => (
          <Chip
            size="sm"
            color={movie.MediaFileId ? 'success' : 'warning'}
            variant="flat"
          >
            {movie.MediaFileId ? 'Downloaded' : 'Missing'}
          </Chip>
        ),
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
        onAction: (movie) => onDeleteMovie(movie.Id, movie.Title),
      },
    ],
    [onDeleteMovie]
  )

  // Card renderer
  const cardRenderer = useCallback(
    ({ item }: CardRendererProps<Movie>) => (
      <MovieCard
        movie={item}
        onDelete={() => onDeleteMovie(item.Id, item.Title)}
      />
    ),
    [onDeleteMovie]
  )

  return (
    <div className="flex flex-col grow w-full">
      <div className="flex-1 min-h-0">
        <DataTable
          stateKey="library-movies"
          skeletonDelay={500}
          data={filteredMovies}
          columns={columns}
          getRowKey={(movie) => movie.Id}
          searchPlaceholder="Search movies..."
          sortColumn={sortColumn || 'title'}
          sortDirection={sortDirection}
          onSortChange={handleSortChange}
          showViewModeToggle
          defaultViewMode="cards"
          cardRenderer={cardRenderer}
          cardSkeleton={() => <MediaCardSkeleton />}
          skeletonCardCount={12}
          cardGridClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"
          rowActions={rowActions}
          showItemCount
          ariaLabel="Movies table"
          fillHeight
          // Server-side mode
          serverSide
          serverTotalCount={totalCount ?? undefined}
          onSearchChange={handleSearchChange}
          // Infinite loading
          paginationMode="infinite"
          hasMore={hasMore}
          onLoadMore={loadMore}
          isLoading={parentLoading || isLoading}
          isLoadingMore={isLoadingMore}
          headerContent={
            <AlphabetFilter
              selectedLetter={normalizedLetter}
              availableLetters={availableLetters}
              onLetterChange={handleLetterChange}
            />
          }
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
