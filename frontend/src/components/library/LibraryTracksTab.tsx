import { useMemo, useCallback } from 'react'
import { useQueryState, parseAsString, parseAsStringLiteral } from 'nuqs'
import { Card, CardBody } from '@heroui/card'
import {
  DataTable,
  AlphabetFilter,
  getFirstLetter,
  type DataTableColumn,
} from '../data-table'
import { TRACKS_CONNECTION_QUERY, type Track } from '../../lib/graphql'
import type { Connection } from '../../lib/graphql/types'
import { IconMusicBolt, IconCircleCheck, IconDownload } from '@tabler/icons-react'
import { useInfiniteConnection } from '../../hooks/useInfiniteConnection'
import { formatDuration } from '../../lib/format'
import { TrackStatusChip } from '../shared/TrackStatusChip'

// ============================================================================
// Component Props
// ============================================================================

interface LibraryTracksTabProps {
  libraryId: string
  /** Parent loading state (e.g., library context still loading) */
  loading?: boolean
}

// ============================================================================
// Types for GraphQL response
// ============================================================================

interface TracksConnectionResponse {
  tracksConnection: Connection<Track>
}

// ============================================================================
// Main Component
// ============================================================================

// Map column keys to GraphQL sort fields
const SORT_FIELD_MAP: Record<string, string> = {
  title: 'TITLE',
  trackNumber: 'TRACK_NUMBER',
  artistName: 'ARTIST_NAME',
  duration: 'DURATION',
}

export function LibraryTracksTab({ libraryId, loading: parentLoading }: LibraryTracksTabProps) {
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
        title: { contains: searchTerm },
      }
    }
    
    // Add order by from sort state
    const graphqlField = SORT_FIELD_MAP[sortColumn || 'title'] || 'TITLE'
    vars.orderBy = {
      field: graphqlField,
      direction: sortDirection.toUpperCase(),
    }
    
    return vars
  }, [libraryId, searchTerm, sortColumn, sortDirection])

  // Use infinite connection hook for server-side pagination
  const {
    items: tracks,
    isLoading,
    isLoadingMore,
    hasMore,
    totalCount,
    loadMore,
  } = useInfiniteConnection<TracksConnectionResponse, Track>({
    query: TRACKS_CONNECTION_QUERY,
    variables: queryVariables,
    getConnection: (data) => data.tracksConnection,
    batchSize: 50,
    deps: [libraryId, searchTerm],
  })

  // Get letters that have tracks (from loaded data)
  const availableLetters = useMemo(() => {
    const letters = new Set<string>()
    tracks.forEach((track) => {
      letters.add(getFirstLetter(track.title))
    })
    return letters
  }, [tracks])

  // Filter tracks by selected letter (client-side for alphabet filter)
  const filteredTracks = useMemo(() => {
    if (!normalizedLetter) return tracks
    return tracks.filter((track) => getFirstLetter(track.title) === normalizedLetter)
  }, [tracks, normalizedLetter])

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
  const columns: DataTableColumn<Track>[] = useMemo(
    () => [
      {
        key: 'title',
        label: 'TITLE',
        render: (track) => (
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-default-200 rounded flex items-center justify-center shrink-0">
              <IconMusicBolt size={16} className="text-green-400" />
            </div>
            <div>
              <p className="font-medium">{track.title}</p>
              {track.artistName && (
                <p className="text-xs text-default-500">{track.artistName}</p>
              )}
            </div>
          </div>
        ),
      },
      {
        key: 'trackNumber',
        label: '#',
        width: 60,
        render: (track) => (
          <span className="text-default-500">
            {track.discNumber > 1 ? `${track.discNumber}-` : ''}{track.trackNumber}
          </span>
        ),
      },
      {
        key: 'artistName',
        label: 'ARTIST',
        width: 200,
        render: (track) => (
          <span className="text-default-500">{track.artistName || '—'}</span>
        ),
      },
      {
        key: 'duration',
        label: 'DURATION',
        width: 100,
        render: (track) => (
          <span className="text-default-500">
            {track.durationSecs ? formatDuration(track.durationSecs) : '—'}
          </span>
        ),
      },
      {
        key: 'status',
        label: 'STATUS',
        width: 120,
        sortable: false,
        render: (track) => (
          <TrackStatusChip mediaFileId={track.mediaFileId} downloadProgress={track.downloadProgress} />
        ),
      },
      {
        key: 'hasFile',
        label: 'FILE',
        width: 80,
        sortable: false,
        render: (track) => (
          track.hasFile ? (
            <IconCircleCheck size={18} className="text-green-400" />
          ) : (
            <IconDownload size={18} className="text-default-400" />
          )
        ),
      },
    ],
    []
  )

  return (
    <div className="flex flex-col grow w-full">
      <div className="flex-1 min-h-0">
        <DataTable
          stateKey="library-tracks"
          data={filteredTracks}
          columns={columns}
          getRowKey={(track) => track.id}
          searchPlaceholder="Search tracks..."
          sortColumn={sortColumn || 'title'}
          sortDirection={sortDirection}
          onSortChange={handleSortChange}
          showViewModeToggle={false}
          defaultViewMode="table"
          showItemCount
          ariaLabel="Tracks table"
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
                <IconMusicBolt size={48} className="mx-auto mb-4 text-green-400" />
                <h3 className="text-lg font-semibold mb-2">No tracks yet</h3>
                <p className="text-default-500 mb-4">
                  Tracks will appear here as you add albums to your library.
                </p>
              </CardBody>
            </Card>
          }
        />
      </div>
    </div>
  )
}
