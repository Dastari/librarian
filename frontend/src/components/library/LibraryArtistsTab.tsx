import { useMemo, useState, useCallback, useEffect } from 'react'
import { useQueryState, parseAsString, parseAsStringLiteral } from 'nuqs'
import { Card, CardBody } from '@heroui/card'
import {
  DataTable,
  AlphabetFilter,
  getFirstLetter,
  type DataTableColumn,
  type CardRendererProps,
} from '../data-table'
import { graphqlClient, ARTISTS_CONNECTION_QUERY, ALBUMS_QUERY, type Artist, type Album } from '../../lib/graphql'
import type { Connection } from '../../lib/graphql/types'
import { IconUser, IconDisc, IconMicrophone } from '@tabler/icons-react'
import { useInfiniteConnection } from '../../hooks/useInfiniteConnection'

// ============================================================================
// Component Props
// ============================================================================

interface LibraryArtistsTabProps {
  libraryId: string
  onSelectArtist?: (artistId: string) => void
}

// ============================================================================
// Types for GraphQL response
// ============================================================================

interface ArtistsConnectionResponse {
  artistsConnection: Connection<Artist>
}

// ============================================================================
// Artist Card Component
// ============================================================================

interface ArtistCardProps {
  artist: Artist
  albumCount: number
  onSelect?: () => void
}

function ArtistCard({ artist, albumCount, onSelect }: ArtistCardProps) {
  return (
    <div className="aspect-square">
      <Card
        isPressable={!!onSelect}
        onPress={onSelect}
        className="relative overflow-hidden h-full w-full group border-none bg-content2"
      >
        {/* Background with gradient */}
        <div className="absolute inset-0 bg-gradient-to-br from-green-900 via-emerald-800 to-teal-900">
          <div className="absolute inset-0 flex items-center justify-center opacity-30">
            <IconMicrophone size={64} className="text-green-400" />
          </div>
        </div>

        {/* Album count badge - top right */}
        {albumCount > 0 && (
          <div className="absolute top-2 right-2 z-10 pointer-events-none">
            <div className="px-2 py-1 rounded-md bg-black/50 backdrop-blur-sm text-xs font-medium text-white/90">
              <IconDisc size={12} className="inline mr-1" />
              {albumCount}
            </div>
          </div>
        )}

        {/* Bottom content */}
        <div className="absolute bottom-0 left-0 right-0 z-10 p-3 pointer-events-none bg-black/50 backdrop-blur-sm h-16 flex flex-col justify-center">
          <h3 className="text-sm font-bold text-white mb-0.5 line-clamp-2 drop-shadow-lg">
            {artist.name}
          </h3>
          <div className="flex items-center gap-1.5 text-xs text-white/70">
            <span>{albumCount} {albumCount === 1 ? 'album' : 'albums'}</span>
          </div>
        </div>
      </Card>
    </div>
  )
}

// ============================================================================
// Main Component
// ============================================================================

export function LibraryArtistsTab({
  libraryId,
  onSelectArtist,
}: LibraryArtistsTabProps) {
  // URL-persisted state via nuqs
  const [selectedLetter, setSelectedLetter] = useQueryState('letter', parseAsString.withDefault(''))
  const [searchTerm, setSearchTerm] = useQueryState('q', parseAsString.withDefault(''))
  const [sortColumn, setSortColumn] = useQueryState('sort', parseAsString.withDefault('name'))
  const [sortDirection, setSortDirection] = useQueryState(
    'order',
    parseAsStringLiteral(['asc', 'desc'] as const).withDefault('asc')
  )
  const normalizedLetter = selectedLetter === '' ? null : selectedLetter

  // Handle sort change from DataTable
  const handleSortChange = useCallback((column: string, direction: 'asc' | 'desc') => {
    setSortColumn(column)
    setSortDirection(direction)
  }, [setSortColumn, setSortDirection])
  
  const [albums, setAlbums] = useState<Album[]>([])
  const [albumsLoading, setAlbumsLoading] = useState(true)

  // Fetch albums for counting (still load all for accurate counts)
  useEffect(() => {
    const fetchAlbums = async () => {
      try {
        const result = await graphqlClient
          .query<{ albums: Album[] }>(ALBUMS_QUERY, { libraryId })
          .toPromise()
        if (result.data?.albums) {
          setAlbums(result.data.albums)
        }
      } catch (err) {
        console.error('Failed to fetch albums:', err)
      } finally {
        setAlbumsLoading(false)
      }
    }
    fetchAlbums()
  }, [libraryId])

  // Map column keys to GraphQL sort fields
  const sortFieldMap: Record<string, string> = {
    name: 'NAME',
    albums: 'ALBUM_COUNT',
  }

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
    const graphqlField = sortFieldMap[sortColumn || 'name'] || 'NAME'
    vars.orderBy = {
      field: graphqlField,
      direction: sortDirection.toUpperCase(),
    }
    
    return vars
  }, [libraryId, searchTerm, sortColumn, sortDirection])

  // Use infinite connection hook for server-side pagination
  const {
    items: artists,
    isLoading: artistsLoading,
    isLoadingMore,
    hasMore,
    totalCount,
    loadMore,
  } = useInfiniteConnection<ArtistsConnectionResponse, Artist>({
    query: ARTISTS_CONNECTION_QUERY,
    variables: queryVariables,
    getConnection: (data) => data.artistsConnection,
    batchSize: 50,
    deps: [libraryId, searchTerm],
  })

  const isLoading = artistsLoading || albumsLoading

  // Count albums per artist
  const albumCountByArtist = useMemo(() => {
    const counts = new Map<string, number>()
    albums.forEach((album) => {
      const current = counts.get(album.artistId) || 0
      counts.set(album.artistId, current + 1)
    })
    return counts
  }, [albums])

  // Get letters that have artists
  const availableLetters = useMemo(() => {
    const letters = new Set<string>()
    artists.forEach((artist) => {
      letters.add(getFirstLetter(artist.name))
    })
    return letters
  }, [artists])

  // Filter artists by selected letter
  const filteredArtists = useMemo(() => {
    if (!normalizedLetter) return artists
    return artists.filter((artist) => getFirstLetter(artist.name) === normalizedLetter)
  }, [artists, normalizedLetter])

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
  const columns: DataTableColumn<Artist>[] = useMemo(
    () => [
      {
        key: 'name',
        label: 'ARTIST',
        // sortable: true (default) - server handles actual sorting
        render: (artist) => (
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-default-200 flex items-center justify-center">
              <IconUser size={20} className="text-green-400" />
            </div>
            <div>
              <p className="font-medium">{artist.name}</p>
            </div>
          </div>
        ),
      },
      {
        key: 'albums',
        label: 'ALBUMS',
        width: 100,
        sortable: false,
        render: (artist) => (
          <span className="flex items-center gap-1">
            <IconDisc size={14} className="text-default-400" />
            {albumCountByArtist.get(artist.id) || 0}
          </span>
        ),
      },
    ],
    [albumCountByArtist]
  )

  // Card renderer
  const cardRenderer = useCallback(
    ({ item }: CardRendererProps<Artist>) => (
      <ArtistCard
        artist={item}
        albumCount={albumCountByArtist.get(item.id) || 0}
        onSelect={onSelectArtist ? () => onSelectArtist(item.id) : undefined}
      />
    ),
    [albumCountByArtist, onSelectArtist]
  )

  return (
    <div className="flex flex-col grow w-full">
      <div className="flex-1 min-h-0">
        <DataTable
          stateKey="library-artists"
          data={filteredArtists}
          columns={columns}
          getRowKey={(artist) => artist.id}
          searchPlaceholder="Search artists..."
          sortColumn={sortColumn || 'name'}
          sortDirection={sortDirection}
          onSortChange={handleSortChange}
          showViewModeToggle
          defaultViewMode="cards"
          cardRenderer={cardRenderer}
          cardGridClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"
          showItemCount
          ariaLabel="Artists table"
          fillHeight
          serverSide
          serverTotalCount={totalCount ?? undefined}
          onSearchChange={handleSearchChange}
          paginationMode="infinite"
          hasMore={hasMore}
          onLoadMore={loadMore}
          isLoading={isLoading}
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
                <IconMicrophone size={48} className="mx-auto mb-4 text-green-400" />
                <h3 className="text-lg font-semibold mb-2">No artists yet</h3>
                <p className="text-default-500 mb-4">
                  Artists will appear here as you add albums to your library.
                </p>
              </CardBody>
            </Card>
          }
        />
      </div>
    </div>
  )
}
