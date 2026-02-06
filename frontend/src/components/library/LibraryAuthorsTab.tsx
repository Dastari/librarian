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
import {
  AUDIOBOOK_AUTHORS_QUERY,
  AUDIOBOOKS_QUERY,
  type AudiobookAuthor,
  type Audiobook,
} from '../../lib/graphql'
import { IconUser, IconBook, IconHeadphones } from '@tabler/icons-react'
import { SquareCardSkeleton } from './MediaCardSkeleton'

// ============================================================================
// Component Props
// ============================================================================

interface LibraryAuthorsTabProps {
  libraryId: string
  /** Parent loading state (e.g., library context still loading) */
  loading?: boolean
  onSelectAuthor?: (authorId: string) => void
}

interface AudiobookNode {
  Id: string
  AuthorId: string | null
  LibraryId: string
  Title: string
  SortTitle: string | null
  Subtitle: string | null
  OpenlibraryId: string | null
  Isbn: string | null
  Description: string | null
  Publisher: string | null
  Language: string | null
  Narrators: string[]
  SeriesName: string | null
  DurationSecs: number | null
  CoverUrl: string | null
  HasFiles: boolean
  SizeBytes: number | null
  Path: string | null
  ChapterCount: number | null
  DownloadedChapterCount: number | null
}

interface AudiobookAuthorNode {
  Id: string
  LibraryId: string
  Name: string
  SortName: string | null
  OpenlibraryId: string | null
}

// ============================================================================
// Author Card Component
// ============================================================================

interface AuthorCardProps {
  author: AudiobookAuthor
  bookCount: number
  onSelect?: () => void
}

function AuthorCard({ author, bookCount, onSelect }: AuthorCardProps) {
  return (
    <div className="aspect-square">
      <Card
        isPressable={!!onSelect}
        onPress={onSelect}
        className="relative overflow-hidden h-full w-full group border-none bg-content2"
      >
        {/* Background with gradient */}
        <div className="absolute inset-0 bg-gradient-to-br from-orange-900 via-amber-800 to-yellow-900">
          <div className="absolute inset-0 flex items-center justify-center opacity-30">
            <IconUser size={64} className="text-orange-400" />
          </div>
        </div>

        {/* Book count badge - top right */}
        {bookCount > 0 && (
          <div className="absolute top-2 right-2 z-10 pointer-events-none">
            <div className="px-2 py-1 rounded-md bg-black/50 backdrop-blur-sm text-xs font-medium text-white/90">
              <IconBook size={12} className="inline mr-1" />
              {bookCount}
            </div>
          </div>
        )}

        {/* Bottom content */}
        <div className="absolute bottom-0 left-0 right-0 z-10 p-3 pointer-events-none bg-black/50 backdrop-blur-sm h-16 flex flex-col justify-center">
          <h3 className="text-sm font-bold text-white mb-0.5 line-clamp-2 drop-shadow-lg">
            {author.name}
          </h3>
          <div className="flex items-center gap-1.5 text-xs text-white/70">
            <span>{bookCount} {bookCount === 1 ? 'audiobook' : 'audiobooks'}</span>
          </div>
        </div>
      </Card>
    </div>
  )
}

// ============================================================================
// Main Component
// ============================================================================

export function LibraryAuthorsTab({
  libraryId,
  loading: parentLoading,
  onSelectAuthor,
}: LibraryAuthorsTabProps) {
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
  
  const [audiobooks, setAudiobooks] = useState<Audiobook[]>([])
  const [audiobooksLoading, setAudiobooksLoading] = useState(true)
  const [authors, setAuthors] = useState<AudiobookAuthor[]>([])
  const [authorsLoading, setAuthorsLoading] = useState(true)

  // Check if we should skip queries (loading or template ID)
  const shouldSkipQueries = parentLoading || libraryId.startsWith('template')

  useEffect(() => {
    if (shouldSkipQueries) return

    const fetchAudiobooks = async () => {
      try {
        const result = await queryPromise<{ Audiobooks: { Edges: Array<{ Node: AudiobookNode }> } }>(AUDIOBOOKS_QUERY, { libraryId })
          
        const edges = result.data?.Audiobooks?.Edges ?? []
        setAudiobooks(edges.map((e) => ({
          id: e.Node.Id,
          authorId: e.Node.AuthorId,
          libraryId: e.Node.LibraryId,
          title: e.Node.Title,
          sortTitle: e.Node.SortTitle,
          subtitle: e.Node.Subtitle,
          openlibraryId: e.Node.OpenlibraryId,
          isbn: e.Node.Isbn,
          description: e.Node.Description,
          publisher: e.Node.Publisher,
          language: e.Node.Language,
          narrators: e.Node.Narrators,
          seriesName: e.Node.SeriesName,
          durationSecs: e.Node.DurationSecs,
          coverUrl: e.Node.CoverUrl,
          hasFiles: e.Node.HasFiles,
          sizeBytes: e.Node.SizeBytes,
          path: e.Node.Path,
          chapterCount: e.Node.ChapterCount,
          downloadedChapterCount: e.Node.DownloadedChapterCount,
        })))
      } catch (err) {
        console.error('Failed to fetch audiobooks:', err)
      } finally {
        setAudiobooksLoading(false)
      }
    }

    const fetchAuthors = async () => {
      try {
        const result = await queryPromise<{ AudiobookAuthors: { Edges: Array<{ Node: AudiobookAuthorNode }> } }>(AUDIOBOOK_AUTHORS_QUERY, { libraryId })
          
        const edges = result.data?.AudiobookAuthors?.Edges ?? []
        setAuthors(edges.map((e) => ({
          id: e.Node.Id,
          libraryId: e.Node.LibraryId,
          name: e.Node.Name,
          sortName: e.Node.SortName,
          openlibraryId: e.Node.OpenlibraryId,
        })))
      } catch (err) {
        console.error('Failed to fetch authors:', err)
      } finally {
        setAuthorsLoading(false)
      }
    }

    void fetchAudiobooks()
    void fetchAuthors()
  }, [libraryId, shouldSkipQueries])

  const isLoading = authorsLoading || audiobooksLoading

  // Count audiobooks per author
  const bookCountByAuthor = useMemo(() => {
    const counts = new Map<string, number>()
    audiobooks.forEach((audiobook) => {
      if (audiobook.authorId) {
        const current = counts.get(audiobook.authorId) || 0
        counts.set(audiobook.authorId, current + 1)
      }
    })
    return counts
  }, [audiobooks])

  // Get letters that have authors
  const availableLetters = useMemo(() => {
    const letters = new Set<string>()
    authors.forEach((author) => {
      letters.add(getFirstLetter(author.name))
    })
    return letters
  }, [authors])

  const sortedAuthors = useMemo(() => {
    const sorted = [...authors]
    sorted.sort((a, b) => {
      if (sortColumn === 'audiobooks') {
        const av = bookCountByAuthor.get(a.id) || 0
        const bv = bookCountByAuthor.get(b.id) || 0
        return sortDirection === 'asc' ? av - bv : bv - av
      }
      const cmp = a.name.localeCompare(b.name)
      return sortDirection === 'asc' ? cmp : -cmp
    })
    return sorted
  }, [authors, sortColumn, sortDirection, bookCountByAuthor])

  const filteredAuthors = useMemo(() => {
    let list = sortedAuthors
    if (searchTerm) {
      const q = searchTerm.toLowerCase()
      list = list.filter((author) => author.name.toLowerCase().includes(q))
    }
    if (normalizedLetter) {
      list = list.filter((author) => getFirstLetter(author.name) === normalizedLetter)
    }
    return list
  }, [sortedAuthors, searchTerm, normalizedLetter])

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
  const columns: DataTableColumn<AudiobookAuthor>[] = useMemo(
    () => [
      {
        key: 'name',
        label: 'AUTHOR',
        // sortable: true (default) - server handles actual sorting
        render: (author) => (
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-full bg-default-200 flex items-center justify-center">
              <IconUser size={20} className="text-orange-400" />
            </div>
            <div>
              <p className="font-medium">{author.name}</p>
            </div>
          </div>
        ),
      },
      {
        key: 'audiobooks',
        label: 'AUDIOBOOKS',
        width: 120,
        sortable: false,
        render: (author) => (
          <span className="flex items-center gap-1">
            <IconHeadphones size={14} className="text-default-400" />
            {bookCountByAuthor.get(author.id) || 0}
          </span>
        ),
      },
    ],
    [bookCountByAuthor]
  )

  // Card renderer
  const cardRenderer = useCallback(
    ({ item }: CardRendererProps<AudiobookAuthor>) => (
      <AuthorCard
        author={item}
        bookCount={bookCountByAuthor.get(item.id) || 0}
        onSelect={onSelectAuthor ? () => onSelectAuthor(item.id) : undefined}
      />
    ),
    [bookCountByAuthor, onSelectAuthor]
  )

  return (
    <div className="flex flex-col grow w-full">
      <div className="flex-1 min-h-0">
        <DataTable
          stateKey="library-authors"
          skeletonDelay={500}
          data={filteredAuthors}
          columns={columns}
          getRowKey={(author) => author.id}
          searchPlaceholder="Search authors..."
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
          ariaLabel="Authors table"
          fillHeight
          serverTotalCount={filteredAuthors.length}
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
                <IconUser size={48} className="mx-auto mb-4 text-orange-400" />
                <h3 className="text-lg font-semibold mb-2">No authors yet</h3>
                <p className="text-default-500 mb-4">
                  Authors will appear here as you add audiobooks to your library.
                </p>
              </CardBody>
            </Card>
          }
        />
      </div>
    </div>
  )
}
