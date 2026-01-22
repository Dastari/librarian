import { useMemo, useState, useCallback, useEffect } from 'react'
import { useQueryState, parseAsString, parseAsStringLiteral } from 'nuqs'
import { Button } from '@heroui/button'
import { Image } from '@heroui/image'
import { Card, CardBody } from '@heroui/card'
import {
  DataTable,
  AlphabetFilter,
  getFirstLetter,
  type DataTableColumn,
  type RowAction,
  type CardRendererProps,
} from '../data-table'
import { graphqlClient, ALBUMS_CONNECTION_QUERY, ARTISTS_QUERY, type Album, type Artist } from '../../lib/graphql'
import type { Connection } from '../../lib/graphql/types'
import { IconPlus, IconTrash, IconEye, IconDisc, IconMusic, IconCalendar } from '@tabler/icons-react'
import { AlbumCard } from './AlbumCard'
import { SquareCardSkeleton } from './MediaCardSkeleton'
import { useInfiniteConnection } from '../../hooks/useInfiniteConnection'

// ============================================================================
// Component Props
// ============================================================================

interface LibraryAlbumsTabProps {
  libraryId: string
  /** Parent loading state (e.g., library context still loading) */
  loading?: boolean
  onDeleteAlbum?: (albumId: string, albumName: string) => void
  onAddAlbum?: () => void
}

// ============================================================================
// Types for GraphQL response
// ============================================================================

interface AlbumsConnectionResponse {
  albumsConnection: Connection<Album>
}

// ============================================================================
// Main Component
// ============================================================================

export function LibraryAlbumsTab({
  libraryId,
  loading: parentLoading,
  onDeleteAlbum,
  onAddAlbum,
}: LibraryAlbumsTabProps) {
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
  
  const [artists, setArtists] = useState<Artist[]>([])
  const [artistsLoading, setArtistsLoading] = useState(true)

  // Check if we should skip queries (loading or template ID)
  const shouldSkipQueries = parentLoading || libraryId.startsWith('template')

  // Fetch artists separately (still needed for name lookup)
  useEffect(() => {
    if (shouldSkipQueries) {
      return
    }
    const fetchArtists = async () => {
      try {
        const result = await graphqlClient
          .query<{ artists: Artist[] }>(ARTISTS_QUERY, { libraryId })
          .toPromise()
        if (result.data?.artists) {
          setArtists(result.data.artists)
        }
      } catch (err) {
        console.error('Failed to fetch artists:', err)
      } finally {
        setArtistsLoading(false)
      }
    }
    fetchArtists()
  }, [libraryId, shouldSkipQueries])

  // Map column keys to GraphQL sort fields
  const sortFieldMap: Record<string, string> = {
    name: 'NAME',
    year: 'YEAR',
    artist: 'ARTIST_NAME',
    tracks: 'TRACK_COUNT',
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
    items: albums,
    isLoading: albumsLoading,
    isLoadingMore,
    hasMore,
    totalCount,
    loadMore,
  } = useInfiniteConnection<AlbumsConnectionResponse, Album>({
    query: ALBUMS_CONNECTION_QUERY,
    variables: queryVariables,
    getConnection: (data) => data.albumsConnection,
    batchSize: 50,
    enabled: !shouldSkipQueries,
    deps: [libraryId, searchTerm],
  })

  const isLoading = albumsLoading || artistsLoading

  // Create artist lookup map
  const artistMap = useMemo(() => {
    const map = new Map<string, string>()
    artists.forEach((artist) => {
      map.set(artist.id, artist.name)
    })
    return map
  }, [artists])

  // Get letters that have albums
  const availableLetters = useMemo(() => {
    const letters = new Set<string>()
    albums.forEach((album) => {
      letters.add(getFirstLetter(album.name))
    })
    return letters
  }, [albums])

  // Filter albums by selected letter
  const filteredAlbums = useMemo(() => {
    if (!normalizedLetter) return albums
    return albums.filter((album) => getFirstLetter(album.name) === normalizedLetter)
  }, [albums, normalizedLetter])

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
  const columns: DataTableColumn<Album>[] = useMemo(
    () => [
      {
        key: 'name',
        label: 'ALBUM',
        // sortable: true (default) - server handles actual sorting
        render: (album) => (
          <div className="flex items-center gap-3">
            {album.coverUrl ? (
              <Image
                src={album.coverUrl}
                alt={album.name}
                className="w-10 h-10 object-cover rounded"
              />
            ) : (
              <div className="w-10 h-10 bg-default-200 rounded flex items-center justify-center">
                <IconDisc size={20} className="text-green-400" />
              </div>
            )}
            <div>
              <p className="font-medium">{album.name}</p>
              {artistMap.get(album.artistId) && (
                <p className="text-xs text-default-400">
                  {artistMap.get(album.artistId)}
                </p>
              )}
            </div>
          </div>
        ),
      },
      {
        key: 'year',
        label: 'YEAR',
        width: 80,
        render: (album) => (
          <span className="flex items-center gap-1">
            {album.year ? (
              <>
                <IconCalendar size={14} className="text-default-400" />
                {album.year}
              </>
            ) : '—'}
          </span>
        ),
      },
      {
        key: 'tracks',
        label: 'TRACKS',
        width: 100,
        render: (album) => (
          <span className="flex items-center gap-1">
            {album.trackCount ? (
              <>
                <IconMusic size={14} className="text-default-400" />
                {album.trackCount}
              </>
            ) : '—'}
          </span>
        ),
      },
      {
        key: 'progress',
        label: 'PROGRESS',
        width: 80,
        sortable: false,
        render: (album) => {
          const downloaded = album.downloadedTrackCount ?? 0
          const total = album.trackCount ?? 0
          const isComplete = total > 0 && downloaded >= total
          return (
            <span className={isComplete ? 'text-success font-medium' : 'text-warning font-medium'}>
              {downloaded}/{total}
            </span>
          )
        },
      },
    ],
    [artistMap]
  )

  // Row actions
  const rowActions: RowAction<Album>[] = useMemo(
    () => [
      {
        key: 'view',
        label: 'View',
        icon: <IconEye size={16} />,
        inDropdown: true,
        onAction: () => {
          // View details when route is available
        },
      },
      ...(onDeleteAlbum
        ? [
            {
              key: 'delete',
              label: 'Delete',
              icon: <IconTrash size={16} className="text-red-400" />,
              isDestructive: true,
              inDropdown: true,
              onAction: (album: Album) => onDeleteAlbum(album.id, album.name),
            },
          ]
        : []),
    ],
    [onDeleteAlbum]
  )

  // Card renderer
  const cardRenderer = useCallback(
    ({ item }: CardRendererProps<Album>) => (
      <AlbumCard
        album={item}
        artistName={artistMap.get(item.artistId)}
        onDelete={onDeleteAlbum ? () => onDeleteAlbum(item.id, item.name) : undefined}
      />
    ),
    [artistMap, onDeleteAlbum]
  )

  return (
    <div className="flex flex-col grow w-full">
      <div className="flex-1 min-h-0">
        <DataTable
          stateKey="library-albums"
          skeletonDelay={500}
          data={filteredAlbums}
          columns={columns}
          getRowKey={(album) => album.id}
          searchPlaceholder="Search albums..."
          sortColumn={sortColumn || 'name'}
          sortDirection={sortDirection}
          onSortChange={handleSortChange}
          showViewModeToggle
          defaultViewMode="cards"
          cardRenderer={cardRenderer}
          cardSkeleton={() => <SquareCardSkeleton />}
          skeletonCardCount={12}
          cardGridClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"
          rowActions={rowActions}
          showItemCount
          ariaLabel="Albums table"
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
                <IconDisc size={48} className="mx-auto mb-4 text-green-400" />
                <h3 className="text-lg font-semibold mb-2">No albums yet</h3>
                <p className="text-default-500 mb-4">
                  Add albums to this library to start building your music collection.
                </p>
                {onAddAlbum && (
                  <Button color="primary" onPress={onAddAlbum}>
                    Add Album
                  </Button>
                )}
              </CardBody>
            </Card>
          }
          toolbarContent={
            onAddAlbum ? (
              <Button color="primary" size="sm" onPress={onAddAlbum} isIconOnly>
                <IconPlus size={16} />
              </Button>
            ) : undefined
          }
          toolbarContentPosition="end"
        />
      </div>
    </div>
  )
}
