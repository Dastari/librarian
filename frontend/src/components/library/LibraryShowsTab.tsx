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
import { TV_SHOWS_CONNECTION_QUERY, type TvShow } from '../../lib/graphql'
import type { Connection } from '../../lib/graphql/types'
import { IconPlus, IconTrash, IconEye, IconDeviceTv } from '@tabler/icons-react'
import { TvShowCard } from './TvShowCard'
import { MediaCardSkeleton } from './MediaCardSkeleton'
import { useInfiniteConnection } from '../../hooks/useInfiniteConnection'

// ============================================================================
// Component Props
// ============================================================================

interface LibraryShowsTabProps {
  libraryId: string
  /** Parent loading state (e.g., library context still loading) */
  loading?: boolean
  onDeleteShow: (showId: string, showName: string) => void
  onAddShow: () => void
}

// ============================================================================
// Types for GraphQL response
// ============================================================================

interface TvShowsConnectionResponse {
  tvShowsConnection: Connection<TvShow>
}

// ============================================================================
// Main Component
// ============================================================================

// Map column keys to GraphQL sort fields
const SORT_FIELD_MAP: Record<string, string> = {
  name: 'SORT_NAME',
  year: 'YEAR',
  seasons: 'SEASON_COUNT',
  episodes: 'EPISODE_COUNT',
}

export function LibraryShowsTab({ libraryId, loading: parentLoading, onDeleteShow, onAddShow }: LibraryShowsTabProps) {
  // URL-persisted state via nuqs (clean URLs when using defaults)
  const [selectedLetter, setSelectedLetter] = useQueryState('letter', parseAsString.withDefault(''))
  const [searchTerm, setSearchTerm] = useQueryState('q', parseAsString.withDefault(''))
  const [sortColumn, setSortColumn] = useQueryState('sort', parseAsString.withDefault('name'))
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

  // Build filter variables for GraphQL query
  const queryVariables = useMemo(() => {
    const vars: Record<string, unknown> = { libraryId }
    
    // Add search filter if there's a search term
    if (searchTerm) {
      vars.where = {
        name: { contains: searchTerm },
      }
    }
    
    // Add order by from sort state
    const graphqlField = SORT_FIELD_MAP[sortColumn || 'name'] || 'SORT_NAME'
    vars.orderBy = {
      field: graphqlField,
      direction: sortDirection.toUpperCase(),
    }
    
    return vars
  }, [libraryId, searchTerm, sortColumn, sortDirection])

  // Use infinite connection hook for server-side pagination
  const {
    items: shows,
    isLoading,
    isLoadingMore,
    hasMore,
    totalCount,
    loadMore,
  } = useInfiniteConnection<TvShowsConnectionResponse, TvShow>({
    query: TV_SHOWS_CONNECTION_QUERY,
    variables: queryVariables,
    getConnection: (data) => data.tvShowsConnection,
    batchSize: 50,
    enabled: !shouldSkipQueries,
    deps: [libraryId, searchTerm],
  })

  // Get letters that have shows (from loaded data)
  const availableLetters = useMemo(() => {
    const letters = new Set<string>()
    shows.forEach((show) => {
      letters.add(getFirstLetter(show.name))
    })
    return letters
  }, [shows])

  // Filter shows by selected letter (client-side for alphabet filter)
  const filteredShows = useMemo(() => {
    if (!normalizedLetter) return shows
    return shows.filter((show) => getFirstLetter(show.name) === normalizedLetter)
  }, [shows, normalizedLetter])

  // Handle letter change - toggle filter
  const handleLetterChange = useCallback((letter: string | null) => {
    setSelectedLetter(normalizedLetter === letter ? '' : (letter ?? ''))
  }, [normalizedLetter, setSelectedLetter])

  // Handle search change for server-side filtering
  const handleSearchChange = useCallback((term: string) => {
    setSearchTerm(term || '')
    setSelectedLetter('') // Reset letter filter when searching
  }, [setSearchTerm, setSelectedLetter])

  // Column definitions
  const columns: DataTableColumn<TvShow>[] = useMemo(
    () => [
      {
        key: 'name',
        label: 'SHOW',
        // sortable: true (default) - server handles actual sorting
        render: (show) => (
          <Link to="/shows/$showId" params={{ showId: show.id }} className="flex items-center gap-3 hover:opacity-80">
            {show.posterUrl ? (
              <Image
                src={show.posterUrl}
                alt={show.name}
                className="w-10 h-14 object-cover rounded"
              />
            ) : (
              <div className="w-10 h-14 bg-default-200 rounded flex items-center justify-center">
                <IconDeviceTv size={20} className="text-blue-400" />
              </div>
            )}
            <div>
              <p className="font-medium">{show.name}</p>
            </div>
          </Link>
        ),
      },
      {
        key: 'year',
        label: 'YEAR',
        width: 80,
        render: (show) => <span>{show.year || '—'}</span>,
      },
      {
        key: 'episodes',
        label: 'EPISODES',
        width: 150,
        render: (show) => {
          const missing = (show.episodeCount || 0) - (show.episodeFileCount || 0)
          return (
            <div className="flex items-center gap-2">
              <span>
                {show.episodeFileCount || 0}/{show.episodeCount || 0}
              </span>
              {missing > 0 && (
                <Chip size="sm" color="warning" variant="flat">
                  {missing} missing
                </Chip>
              )}
            </div>
          )
        },
      },
      {
        key: 'progress',
        label: 'PROGRESS',
        width: 80,
        sortable: false,
        render: (show) => {
          const downloaded = show.episodeFileCount ?? 0
          const total = show.episodeCount ?? 0
          const isComplete = total > 0 && downloaded >= total
          return (
            <span className={isComplete ? 'text-success font-medium' : 'text-warning font-medium'}>
              {downloaded}/{total}
            </span>
          )
        },
      },
    ],
    []
  )

  // Row actions
  const rowActions: RowAction<TvShow>[] = useMemo(
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
        onAction: (show) => onDeleteShow(show.id, show.name),
      },
    ],
    [onDeleteShow]
  )

  // Card renderer
  const cardRenderer = useCallback(
    ({ item }: CardRendererProps<TvShow>) => (
      <TvShowCard
        show={item}
        onDelete={() => onDeleteShow(item.id, item.name)}
      />
    ),
    [onDeleteShow]
  )

  return (
    <div className="flex flex-col grow w-full">
      <div className="flex-1 min-h-0">
        <DataTable
          stateKey="library-shows"
          skeletonDelay={500}
          data={filteredShows}
          columns={columns}
          getRowKey={(show) => show.id}
          searchPlaceholder="Search shows..."
          sortColumn={sortColumn || 'name'}
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
          ariaLabel="TV Shows table"
          fillHeight
          serverSide
          serverTotalCount={totalCount ?? undefined}
          onSearchChange={handleSearchChange}
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
                <IconDeviceTv size={48} className="mx-auto mb-4 text-blue-400" />
                <h3 className="text-lg font-semibold mb-2">No shows yet</h3>
                <p className="text-default-500 mb-4">
                  Add TV shows to start tracking episodes.
                </p>
                <Button color="primary" onPress={onAddShow}>
                  Add Show
                </Button>
              </CardBody>
            </Card>
          }
          toolbarContent={
            <Button color="primary" size="sm" onPress={onAddShow} isIconOnly>
              <IconPlus size={16} />
            </Button>
          }
          toolbarContentPosition="end"
        />
      </div>
    </div>
  )
}
