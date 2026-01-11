import { useMemo, useState, useCallback } from 'react'
import { Button, ButtonGroup } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Chip } from '@heroui/chip'
import { Image } from '@heroui/image'
import { Link } from '@tanstack/react-router'
import {
  DataTable,
  type DataTableColumn,
  type RowAction,
  type CardRendererProps,
} from '../data-table'
import type { TvShow } from '../../lib/graphql'
import { formatBytes } from '../../lib/format'
import { PlusIcon, DeleteIcon, ViewIcon } from '../icons'
import { TvShowCard } from './TvShowCard'

// ============================================================================
// Constants
// ============================================================================

const ALPHABET = '#ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('')

// ============================================================================
// Utility Functions
// ============================================================================

function getFirstLetter(name: string): string {
  const firstChar = name.charAt(0).toUpperCase()
  return /[A-Z]/.test(firstChar) ? firstChar : '#'
}

// ============================================================================
// Component Props
// ============================================================================

interface LibraryShowsTabProps {
  shows: TvShow[]
  onDeleteShow: (showId: string, showName: string) => void
  onAddShow: () => void
}

// ============================================================================
// Main Component
// ============================================================================

export function LibraryShowsTab({ shows, onDeleteShow, onAddShow }: LibraryShowsTabProps) {
  const [selectedLetter, setSelectedLetter] = useState<string | null>(null)

  // Get letters that have shows
  const availableLetters = useMemo(() => {
    const letters = new Set<string>()
    shows.forEach((show) => {
      letters.add(getFirstLetter(show.name))
    })
    return letters
  }, [shows])

  // Filter shows by selected letter
  const filteredShows = useMemo(() => {
    if (!selectedLetter) return shows
    return shows.filter((show) => getFirstLetter(show.name) === selectedLetter)
  }, [shows, selectedLetter])

  // Handle letter click - toggle filter
  const handleLetterClick = (letter: string) => {
    setSelectedLetter((prev) => (prev === letter ? null : letter))
  }

  // Column definitions
  const columns: DataTableColumn<TvShow>[] = useMemo(
    () => [
      {
        key: 'name',
        label: 'SHOW',
        sortable: true,
        render: (show) => (
          <Link to={`/shows/${show.id}` as any} className="flex items-center gap-3 hover:opacity-80">
            {show.posterUrl ? (
              <Image
                src={show.posterUrl}
                alt={show.name}
                className="w-10 h-14 object-cover rounded"
              />
            ) : (
              <div className="w-10 h-14 bg-default-200 rounded flex items-center justify-center">
                <span className="text-lg">📺</span>
              </div>
            )}
            <div>
              <p className="font-medium">{show.name}</p>
              {show.genres.length > 0 && (
                <p className="text-xs text-default-400">
                  {show.genres.slice(0, 2).join(', ')}
                </p>
              )}
            </div>
          </Link>
        ),
        sortFn: (a, b) => a.name.localeCompare(b.name),
      },
      {
        key: 'year',
        label: 'YEAR',
        width: 80,
        sortable: true,
        render: (show) => <span>{show.year || '—'}</span>,
        sortFn: (a, b) => (a.year || 0) - (b.year || 0),
      },
      {
        key: 'network',
        label: 'NETWORK',
        width: 120,
        sortable: true,
        render: (show) => <span>{show.network || '—'}</span>,
        sortFn: (a, b) => (a.network || '').localeCompare(b.network || ''),
      },
      {
        key: 'episodes',
        label: 'EPISODES',
        width: 150,
        sortable: true,
        render: (show) => {
          const missing = show.episodeCount - show.episodeFileCount
          return (
            <div className="flex items-center gap-2">
              <span>
                {show.episodeFileCount}/{show.episodeCount}
              </span>
              {missing > 0 && (
                <Chip size="sm" color="warning" variant="flat">
                  {missing} missing
                </Chip>
              )}
            </div>
          )
        },
        sortFn: (a, b) => a.episodeCount - b.episodeCount,
      },
      {
        key: 'size',
        label: 'SIZE',
        width: 100,
        sortable: true,
        render: (show) => <span>{formatBytes(show.sizeBytes)}</span>,
        sortFn: (a, b) => a.sizeBytes - b.sizeBytes,
      },
      {
        key: 'status',
        label: 'STATUS',
        width: 120,
        sortable: true,
        render: (show) => (
          <Chip
            size="sm"
            color={show.monitored ? 'success' : 'default'}
            variant="flat"
          >
            {show.monitored ? 'Monitored' : 'Unmonitored'}
          </Chip>
        ),
        sortFn: (a, b) => (a.monitored === b.monitored ? 0 : a.monitored ? -1 : 1),
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
        icon: <ViewIcon />,
        inDropdown: true,
        onAction: () => {
          // Navigation is handled by the Link component in the column
        },
      },
      {
        key: 'delete',
        label: 'Delete',
        icon: <DeleteIcon />,
        isDestructive: true,
        inDropdown: true,
        onAction: (show) => onDeleteShow(show.id, show.name),
      },
    ],
    [onDeleteShow]
  )

  // Search function
  const searchFn = (show: TvShow, term: string) => {
    const lowerTerm = term.toLowerCase()
    return (
      show.name.toLowerCase().includes(lowerTerm) ||
      (show.network?.toLowerCase().includes(lowerTerm) ?? false)
    )
  }

  // Card renderer - using the shared TvShowCard component
  const cardRenderer = useCallback(
    ({ item }: CardRendererProps<TvShow>) => (
      <TvShowCard
        show={item}
        onDelete={() => onDeleteShow(item.id, item.name)}
      />
    ),
    [onDeleteShow]
  )

  if (shows.length === 0) {
    return (
      <Card className="bg-content1/50 border-default-300 border-dashed border-2 w-full">
        <CardBody className="py-12 text-center">
          <span className="text-5xl mb-4 block">📺</span>
          <h3 className="text-lg font-semibold mb-2">No shows yet</h3>
          <p className="text-default-500 mb-4">
            Add TV shows to start tracking episodes.
          </p>
          <Button color="primary" onPress={onAddShow}>
            Add Your First Show
          </Button>
        </CardBody>
      </Card>
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
            const hasShows = availableLetters.has(letter)
            const isSelected = selectedLetter === letter
            return (
              <Button
                key={letter}
                variant={isSelected ? 'solid' : 'flat'}
                color={isSelected ? 'primary' : 'default'}
                onPress={() => hasShows && handleLetterClick(letter)}
                isDisabled={!hasShows}
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
          stateKey="library-shows"
          data={filteredShows}
          columns={columns}
          getRowKey={(show) => show.id}
          searchFn={searchFn}
          searchPlaceholder="Search shows..."
          defaultSortColumn="name"
          showViewModeToggle
          defaultViewMode="cards"
          cardRenderer={cardRenderer}
          cardGridClassName="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4"
          rowActions={rowActions}
          showItemCount
          ariaLabel="TV Shows table"
          fillHeight
          toolbarContent={
            <Button color="primary" size="sm" onPress={onAddShow} isIconOnly>
              <PlusIcon />
            </Button>
          }
          toolbarContentPosition="end"
        />
      </div>
    </div>
  )
}
