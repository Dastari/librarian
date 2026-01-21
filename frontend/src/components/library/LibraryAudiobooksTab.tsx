import { useMemo, useState, useCallback, useEffect } from 'react'
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
import {
  graphqlClient,
  AUDIOBOOKS_CONNECTION_QUERY,
  AUDIOBOOK_AUTHORS_QUERY,
  type Audiobook,
  type AudiobookAuthor,
} from '../../lib/graphql'
import type { Connection } from '../../lib/graphql/types'
import { IconPlus, IconTrash, IconEye, IconHeadphones, IconUser } from '@tabler/icons-react'
import { AudiobookCard } from './AudiobookCard'
import { useInfiniteConnection } from '../../hooks/useInfiniteConnection'

// ============================================================================
// Component Props
// ============================================================================

interface LibraryAudiobooksTabProps {
  libraryId: string
  onDeleteAudiobook?: (audiobookId: string, audiobookTitle: string) => void
  onAddAudiobook?: () => void
}

// ============================================================================
// Types for GraphQL response
// ============================================================================

interface AudiobooksConnectionResponse {
  audiobooksConnection: Connection<Audiobook>
}

// ============================================================================
// Main Component
// ============================================================================

export function LibraryAudiobooksTab({
  libraryId,
  onDeleteAudiobook,
  onAddAudiobook,
}: LibraryAudiobooksTabProps) {
  // URL-persisted state via nuqs
  const [selectedLetter, setSelectedLetter] = useQueryState('letter', parseAsString.withDefault(''))
  const [searchTerm, setSearchTerm] = useQueryState('q', parseAsString.withDefault(''))
  const [sortColumn, setSortColumn] = useQueryState('sort', parseAsString.withDefault('title'))
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
  
  const [authors, setAuthors] = useState<AudiobookAuthor[]>([])
  const [authorsLoading, setAuthorsLoading] = useState(true)

  // Fetch authors separately (still needed for name lookup)
  useEffect(() => {
    const fetchAuthors = async () => {
      try {
        const result = await graphqlClient
          .query<{ audiobookAuthors: AudiobookAuthor[] }>(AUDIOBOOK_AUTHORS_QUERY, { libraryId })
          .toPromise()
        if (result.data?.audiobookAuthors) {
          setAuthors(result.data.audiobookAuthors)
        }
      } catch (err) {
        console.error('Failed to fetch authors:', err)
      } finally {
        setAuthorsLoading(false)
      }
    }
    fetchAuthors()
  }, [libraryId])

  // Map column keys to GraphQL sort fields
  const sortFieldMap: Record<string, string> = {
    title: 'TITLE',
    author: 'AUTHOR_NAME',
    duration: 'DURATION',
  }

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
    const graphqlField = sortFieldMap[sortColumn || 'title'] || 'TITLE'
    vars.orderBy = {
      field: graphqlField,
      direction: sortDirection.toUpperCase(),
    }
    
    return vars
  }, [libraryId, searchTerm, sortColumn, sortDirection])

  // Use infinite connection hook for server-side pagination
  const {
    items: audiobooks,
    isLoading: audiobooksLoading,
    isLoadingMore,
    hasMore,
    totalCount,
    loadMore,
  } = useInfiniteConnection<AudiobooksConnectionResponse, Audiobook>({
    query: AUDIOBOOKS_CONNECTION_QUERY,
    variables: queryVariables,
    getConnection: (data) => data.audiobooksConnection,
    batchSize: 50,
    deps: [libraryId, searchTerm],
  })

  const isLoading = audiobooksLoading || authorsLoading

  // Create author lookup map
  const authorMap = useMemo(() => {
    const map = new Map<string, string>()
    authors.forEach((author) => {
      map.set(author.id, author.name)
    })
    return map
  }, [authors])

  // Get letters that have audiobooks
  const availableLetters = useMemo(() => {
    const letters = new Set<string>()
    audiobooks.forEach((audiobook) => {
      letters.add(getFirstLetter(audiobook.title))
    })
    return letters
  }, [audiobooks])

  // Filter audiobooks by selected letter
  const filteredAudiobooks = useMemo(() => {
    if (!normalizedLetter) return audiobooks
    return audiobooks.filter((audiobook) => getFirstLetter(audiobook.title) === normalizedLetter)
  }, [audiobooks, normalizedLetter])

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
  const columns: DataTableColumn<Audiobook>[] = useMemo(
    () => [
      {
        key: 'title',
        label: 'AUDIOBOOK',
        // sortable: true (default) - server handles actual sorting
        render: (audiobook) => (
          <Link
            to="/audiobooks/$audiobookId"
            params={{ audiobookId: audiobook.id }}
            className="flex items-center gap-3 hover:opacity-80"
          >
            {audiobook.coverUrl ? (
              <Image
                src={audiobook.coverUrl}
                alt={audiobook.title}
                className="w-10 h-14 object-cover rounded"
              />
            ) : (
              <div className="w-10 h-14 bg-default-200 rounded flex items-center justify-center">
                <IconHeadphones size={20} className="text-orange-400" />
              </div>
            )}
            <div>
              <p className="font-medium">{audiobook.title}</p>
              {audiobook.authorId && authorMap.get(audiobook.authorId) && (
                <p className="text-xs text-default-400">
                  {authorMap.get(audiobook.authorId)}
                </p>
              )}
            </div>
          </Link>
        ),
      },
      {
        key: 'author',
        label: 'AUTHOR',
        width: 150,
        sortable: false,
        render: (audiobook) => (
          <span className="flex items-center gap-1">
            <IconUser size={14} className="text-default-400" />
            {(audiobook.authorId && authorMap.get(audiobook.authorId)) || '—'}
          </span>
        ),
      },
      {
        key: 'series',
        label: 'SERIES',
        width: 150,
        sortable: false,
        render: (audiobook) => <span>{audiobook.seriesName || '—'}</span>,
      },
      {
        key: 'status',
        label: 'STATUS',
        width: 120,
        sortable: false,
        render: (audiobook) => (
          <Chip
            size="sm"
            color={audiobook.hasFiles ? 'success' : 'warning'}
            variant="flat"
          >
            {audiobook.hasFiles ? 'Downloaded' : 'Wanted'}
          </Chip>
        ),
      },
    ],
    [authorMap]
  )

  // Row actions
  const rowActions: RowAction<Audiobook>[] = useMemo(
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
      ...(onDeleteAudiobook
        ? [
            {
              key: 'delete',
              label: 'Delete',
              icon: <IconTrash size={16} className="text-red-400" />,
              isDestructive: true,
              inDropdown: true,
              onAction: (audiobook: Audiobook) => onDeleteAudiobook(audiobook.id, audiobook.title),
            },
          ]
        : []),
    ],
    [onDeleteAudiobook]
  )

  // Card renderer
  const cardRenderer = useCallback(
    ({ item }: CardRendererProps<Audiobook>) => (
      <AudiobookCard
        audiobook={item}
        authorName={item.authorId ? authorMap.get(item.authorId) : undefined}
        onDelete={onDeleteAudiobook ? () => onDeleteAudiobook(item.id, item.title) : undefined}
      />
    ),
    [authorMap, onDeleteAudiobook]
  )

  return (
    <div className="flex flex-col grow w-full">
      <div className="flex-1 min-h-0">
        <DataTable
          stateKey="library-audiobooks"
          data={filteredAudiobooks}
          columns={columns}
          getRowKey={(audiobook) => audiobook.id}
          searchPlaceholder="Search audiobooks..."
          sortColumn={sortColumn || 'title'}
          sortDirection={sortDirection}
          onSortChange={handleSortChange}
          showViewModeToggle
          defaultViewMode="cards"
          cardRenderer={cardRenderer}
          cardGridClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"
          rowActions={rowActions}
          showItemCount
          ariaLabel="Audiobooks table"
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
                <IconHeadphones size={48} className="mx-auto mb-4 text-orange-400" />
                <h3 className="text-lg font-semibold mb-2">No audiobooks yet</h3>
                <p className="text-default-500 mb-4">
                  Add audiobooks to this library to start listening.
                </p>
                {onAddAudiobook && (
                  <Button color="primary" onPress={onAddAudiobook}>
                    Add Audiobook
                  </Button>
                )}
              </CardBody>
            </Card>
          }
          toolbarContent={
            onAddAudiobook ? (
              <Button color="primary" size="sm" onPress={onAddAudiobook} isIconOnly>
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
