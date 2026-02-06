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
import { ARTISTS_QUERY, ALBUMS_QUERY, type Artist, type Album } from '../../lib/graphql'
import { IconUser, IconDisc, IconMicrophone } from '@tabler/icons-react'
import { SquareCardSkeleton } from './MediaCardSkeleton'

// ============================================================================
// Component Props
// ============================================================================

interface LibraryArtistsTabProps {
  libraryId: string
  /** Parent loading state (e.g., library context still loading) */
  loading?: boolean
  onSelectArtist?: (artistId: string) => void
}

interface AlbumNode {
  Id: string
  ArtistId: string
  LibraryId: string
  Name: string
  SortName: string | null
  Year: number | null
  MusicbrainzId: string | null
  AlbumType: string | null
  Genres: string[]
  Label: string | null
  Country: string | null
  ReleaseDate: string | null
  CoverUrl: string | null
  TrackCount: number | null
  DiscCount: number | null
  TotalDurationSecs: number | null
  HasFiles: boolean
  SizeBytes: number | null
  Path: string | null
}

interface ArtistNode {
  Id: string
  LibraryId: string
  Name: string
  SortName: string | null
  MusicbrainzId: string | null
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
  loading: parentLoading,
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
  const [artists, setArtists] = useState<Artist[]>([])
  const [artistsLoading, setArtistsLoading] = useState(true)

  // Check if we should skip queries (loading or template ID)
  const shouldSkipQueries = parentLoading || libraryId.startsWith('template')

  useEffect(() => {
    if (shouldSkipQueries) return

    const fetchAlbums = async () => {
      try {
        const result = await queryPromise<{ Albums: { Edges: Array<{ Node: AlbumNode }> } }>(ALBUMS_QUERY, { libraryId })
          
        const edges = result.data?.Albums?.Edges ?? []
        setAlbums(
          edges.map((e) => ({
            id: e.Node.Id,
            artistId: e.Node.ArtistId,
            libraryId: e.Node.LibraryId,
            name: e.Node.Name,
            sortName: e.Node.SortName,
            year: e.Node.Year,
            musicbrainzId: e.Node.MusicbrainzId,
            albumType: e.Node.AlbumType,
            genres: e.Node.Genres,
            label: e.Node.Label,
            country: e.Node.Country,
            releaseDate: e.Node.ReleaseDate,
            coverUrl: e.Node.CoverUrl,
            trackCount: e.Node.TrackCount,
            discCount: e.Node.DiscCount,
            totalDurationSecs: e.Node.TotalDurationSecs,
            hasFiles: e.Node.HasFiles,
            sizeBytes: e.Node.SizeBytes,
            path: e.Node.Path,
            downloadedTrackCount: null,
          }))
        )
      } catch (err) {
        console.error('Failed to fetch albums:', err)
      } finally {
        setAlbumsLoading(false)
      }
    }

    const fetchArtists = async () => {
      try {
        const result = await queryPromise<{ Artists: { Edges: Array<{ Node: ArtistNode }> } }>(ARTISTS_QUERY, { libraryId })
          
        const edges = result.data?.Artists?.Edges ?? []
        setArtists(edges.map((e) => ({
          id: e.Node.Id,
          libraryId: e.Node.LibraryId,
          name: e.Node.Name,
          sortName: e.Node.SortName,
          musicbrainzId: e.Node.MusicbrainzId,
        })))
      } catch (err) {
        console.error('Failed to fetch artists:', err)
      } finally {
        setArtistsLoading(false)
      }
    }

    void fetchAlbums()
    void fetchArtists()
  }, [libraryId, shouldSkipQueries])

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

  const sortedArtists = useMemo(() => {
    const sorted = [...artists]
    sorted.sort((a, b) => {
      if (sortColumn === 'albums') {
        const av = albumCountByArtist.get(a.id) ?? 0
        const bv = albumCountByArtist.get(b.id) ?? 0
        return sortDirection === 'asc' ? av - bv : bv - av
      }
      const cmp = a.name.localeCompare(b.name)
      return sortDirection === 'asc' ? cmp : -cmp
    })
    return sorted
  }, [artists, sortColumn, sortDirection, albumCountByArtist])

  // Filter artists by selected letter and search term
  const filteredArtists = useMemo(() => {
    let list = sortedArtists
    if (searchTerm) {
      const q = searchTerm.toLowerCase()
      list = list.filter((artist) => artist.name.toLowerCase().includes(q))
    }
    if (normalizedLetter) {
      list = list.filter((artist) => getFirstLetter(artist.name) === normalizedLetter)
    }
    return list
  }, [sortedArtists, searchTerm, normalizedLetter])

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
          skeletonDelay={500}
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
          cardSkeleton={() => <SquareCardSkeleton />}
          skeletonCardCount={12}
          cardGridClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"
          showItemCount
          ariaLabel="Artists table"
          fillHeight
          serverTotalCount={filteredArtists.length}
          onSearchChange={handleSearchChange}
          isLoading={parentLoading || isLoading}
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
