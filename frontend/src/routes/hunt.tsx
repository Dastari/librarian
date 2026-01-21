import { createFileRoute, redirect } from '@tanstack/react-router'
import { useState, useEffect, useCallback, useMemo } from 'react'
import { useQueryState, parseAsString, parseAsStringLiteral } from 'nuqs'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Input } from '@heroui/input'
import { Select, SelectItem } from '@heroui/select'
import { Chip } from '@heroui/chip'
import { Spinner } from '@heroui/spinner'
import { Tooltip } from '@heroui/tooltip'
import { useDisclosure } from '@heroui/modal'
import { addToast } from '@heroui/toast'
import {
  IconSearch,
  IconDownload,
  IconPlus,
  IconClock,
  IconExternalLink,
  IconMovie,
  IconDeviceTv,
  IconMusic,
  IconHeadphones,
  IconCategory,
  IconServer,
  IconCloud,
} from '@tabler/icons-react'
import {
  graphqlClient,
  SEARCH_INDEXERS_QUERY,
  type IndexerSearchResultSet,
  type TorrentRelease,
  type IndexerSearchInput,
} from '../lib/graphql'
import { DataTable, type DataTableColumn, type RowAction } from '../components/data-table'
import { AddToLibraryModal } from '../components/search'
import { RouteError } from '../components/RouteError'
import { formatBytes, sanitizeError } from '../lib/format'

export const Route = createFileRoute('/hunt')({
  beforeLoad: ({ context, location }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({
        to: '/',
        search: {
          signin: true,
          redirect: location.href,
        },
      })
    }
  },
  component: SearchPage,
  errorComponent: RouteError,
})

// Media type options for filtering
const MEDIA_TYPES = [
  { value: 'all', label: 'All', icon: IconCategory },
  { value: 'tv', label: 'TV Shows', icon: IconDeviceTv },
  { value: 'movies', label: 'Movies', icon: IconMovie },
  { value: 'music', label: 'Music', icon: IconMusic },
  { value: 'audiobooks', label: 'Audiobooks', icon: IconHeadphones },
] as const

type MediaType = typeof MEDIA_TYPES[number]['value']

// Torznab category mapping
const CATEGORY_MAP: Record<MediaType, number[] | undefined> = {
  all: undefined,
  tv: [5000, 5010, 5020, 5030, 5040, 5045, 5050, 5060, 5070, 5080],
  movies: [2000, 2010, 2020, 2030, 2040, 2045, 2050, 2060, 2070, 2080],
  music: [3000, 3010, 3020, 3030, 3040, 3050, 3060],
  audiobooks: [3030], // Audiobooks are under music/audiobook in Torznab
}

// Parse relative time from publish date
function formatAge(publishDate: string): string {
  const date = new Date(publishDate)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)
  const diffHours = Math.floor(diffMs / 3600000)
  const diffDays = Math.floor(diffMs / 86400000)

  if (diffMins < 60) return `${diffMins}m`
  if (diffHours < 24) return `${diffHours}h`
  if (diffDays < 30) return `${diffDays}d`
  if (diffDays < 365) return `${Math.floor(diffDays / 30)}mo`
  return `${Math.floor(diffDays / 365)}y`
}

function SearchPage() {
  // Query params via nuqs
  const [query, setQuery] = useQueryState('q', parseAsString.withDefault(''))
  const [mediaType, setMediaType] = useQueryState(
    'type',
    parseAsStringLiteral(['all', 'tv', 'movies', 'music', 'audiobooks'] as const).withDefault('all')
  )
  // Target IDs for linking downloads to library items
  const [albumId] = useQueryState('albumId', parseAsString)
  const [movieId] = useQueryState('movieId', parseAsString)
  const [episodeId] = useQueryState('episodeId', parseAsString)
  
  // Table sorting state - persisted in URL
  const [sortColumn, setSortColumn] = useQueryState('sort', parseAsString.withDefault('seeders'))
  const [sortDirection, setSortDirection] = useQueryState(
    'order',
    parseAsStringLiteral(['asc', 'desc'] as const).withDefault('desc')
  )

  // Handle sort change for URL persistence
  const handleSortChange = useCallback((column: string, direction: 'asc' | 'desc') => {
    setSortColumn(column)
    setSortDirection(direction)
  }, [setSortColumn, setSortDirection])

  // Local state
  const [searchInput, setSearchInput] = useState(query)
  const [isSearching, setIsSearching] = useState(false)
  const [results, setResults] = useState<TorrentRelease[]>([])
  const [searchMeta, setSearchMeta] = useState<{
    totalReleases: number
    totalElapsedMs: number
    indexerCount: number
    errors: string[]
  } | null>(null)
  const [, setDownloadingIds] = useState<Set<string>>(new Set())
  const [selectedRelease, setSelectedRelease] = useState<TorrentRelease | null>(null)
  const { isOpen: isAddModalOpen, onOpen: onAddModalOpen, onClose: onAddModalClose } = useDisclosure()

  // Sync search input with query param on mount/change
  useEffect(() => {
    setSearchInput(query)
  }, [query])

  // Auto-search when query params change (from deep links)
  useEffect(() => {
    if (query && query.length >= 2) {
      handleSearch()
    }
  }, []) // Only on mount - deep link support

  const handleSearch = useCallback(async () => {
    const searchQuery = searchInput.trim()
    if (!searchQuery || searchQuery.length < 2) {
      addToast({
        title: 'Search Error',
        description: 'Please enter at least 2 characters',
        color: 'warning',
      })
      return
    }

    // Update URL
    setQuery(searchQuery)

    setIsSearching(true)
    setResults([])
    setSearchMeta(null)

    try {
      const input: IndexerSearchInput = {
        query: searchQuery,
        categories: CATEGORY_MAP[mediaType],
        limit: 100,
      }

      const { data, error } = await graphqlClient
        .query<{ searchIndexers: IndexerSearchResultSet }>(SEARCH_INDEXERS_QUERY, { input })
        .toPromise()

      if (error) {
        throw new Error(sanitizeError(error))
      }

      if (data?.searchIndexers) {
        // Flatten results from all indexers
        const allReleases: TorrentRelease[] = []
        const errors: string[] = []

        for (const indexer of data.searchIndexers.indexers) {
          if (indexer.error) {
            errors.push(`${indexer.indexerName}: ${indexer.error}`)
          }
          for (const release of indexer.releases) {
            allReleases.push({
              ...release,
              indexerId: indexer.indexerId,
              indexerName: indexer.indexerName,
            })
          }
        }

        // Sort by seeders (descending)
        allReleases.sort((a, b) => (b.seeders ?? 0) - (a.seeders ?? 0))

        setResults(allReleases)
        setSearchMeta({
          totalReleases: data.searchIndexers.totalReleases,
          totalElapsedMs: data.searchIndexers.totalElapsedMs,
          indexerCount: data.searchIndexers.indexers.length,
          errors,
        })

        if (errors.length > 0 && allReleases.length === 0) {
          addToast({
            title: 'Search Failed',
            description: errors.join('; '),
            color: 'danger',
          })
        }
      }
    } catch (err) {
      console.error('Search failed:', err)
      addToast({
        title: 'Search Error',
        description: err instanceof Error ? err.message : 'Failed to search indexers',
        color: 'danger',
      })
    } finally {
      setIsSearching(false)
    }
  }, [searchInput, mediaType, setQuery])

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleSearch()
    }
  }

  const handleDownload = useCallback(async (release: TorrentRelease) => {
    // Prefer magnet link, fall back to torrent file URL
    const magnetUri = release.magnetUri
    const torrentUrl = release.link

    if (!magnetUri && !torrentUrl) {
      addToast({
        title: 'Download Error',
        description: 'No download link available for this release',
        color: 'danger',
      })
      return
    }

    setDownloadingIds(prev => new Set(prev).add(release.guid))

    try {
      const ADD_TORRENT = `
        mutation AddTorrent($input: AddTorrentInput!) {
          addTorrent(input: $input) {
            success
            torrent { id name }
            error
          }
        }
      `

      interface AddTorrentResponse {
        addTorrent: {
          success: boolean
          torrent: { id: string; name: string } | null
          error: string | null
        }
      }

      // Use magnet field for magnet links, url field for .torrent file URLs
      const isMagnet = magnetUri?.startsWith('magnet:')

      const { data, error } = await graphqlClient
        .mutation<AddTorrentResponse>(ADD_TORRENT, {
          input: {
            magnet: isMagnet ? magnetUri : undefined,
            url: !isMagnet ? (magnetUri || torrentUrl) : undefined,
            // Pass indexer ID for authenticated .torrent downloads
            indexerId: !isMagnet && release.indexerId ? release.indexerId : undefined,
            // Pass target IDs for file-level matching when available
            albumId: albumId || undefined,
            movieId: movieId || undefined,
            episodeId: episodeId || undefined,
          },
        })
        .toPromise()

      if (error) {
        throw new Error(sanitizeError(error))
      }

      if (data?.addTorrent?.success) {
        addToast({
          title: 'Download Started',
          description: `Added: ${release.title}`,
          color: 'success',
        })
      } else {
        throw new Error(data?.addTorrent?.error || 'Failed to add torrent')
      }
    } catch (err) {
      console.error('Download failed:', err)
      addToast({
        title: 'Download Error',
        description: err instanceof Error ? err.message : 'Failed to start download',
        color: 'danger',
      })
    } finally {
      setDownloadingIds(prev => {
        const next = new Set(prev)
        next.delete(release.guid)
        return next
      })
    }
  }, [albumId, movieId, episodeId])

  // Table columns
  const columns: DataTableColumn<TorrentRelease>[] = useMemo(
    () => [
      {
        key: 'title',
        label: 'NAME',
        sortable: true,
        render: (release) => (
          <div className="max-w-lg">
            <div className="flex items-center gap-2">
              <span className="font-medium line-clamp-1" title={release.title}>
                {release.title}
              </span>
              {release.isFreeleech && (
                <Chip size="sm" color="success" variant="flat">
                  FL
                </Chip>
              )}
            </div>
            <div className="flex items-center gap-2 text-xs text-default-400 mt-0.5">
              {/* Source type indicator based on magnet/link type */}
              {release.magnetUri || release.link ? (
                <Chip size="sm" variant="flat" color="primary" className="h-4 text-[10px]">
                  <IconDownload size={10} className="mr-0.5" />
                  Torrent
                </Chip>
              ) : (
                <Chip size="sm" variant="flat" color="secondary" className="h-4 text-[10px]">
                  <IconServer size={10} className="mr-0.5" />
                  Usenet
                </Chip>
              )}
              <span>{release.indexerName}</span>
              {release.details && (
                <a
                  href={release.details}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-primary"
                  onClick={(e) => e.stopPropagation()}
                >
                  <IconExternalLink size={12} />
                </a>
              )}
            </div>
          </div>
        ),
        sortFn: (a, b) => a.title.localeCompare(b.title),
      },
      {
        key: 'size',
        label: 'SIZE',
        width: 100,
        sortable: true,
        render: (release) => (
          <span className="text-sm">
            {release.sizeFormatted || (release.size ? formatBytes(release.size) : '—')}
          </span>
        ),
        sortFn: (a, b) => (a.size ?? 0) - (b.size ?? 0),
      },
      {
        key: 'seeders',
        label: 'S/L',
        width: 80,
        sortable: true,
        render: (release) => (
          <div className="flex items-center gap-1 text-sm">
            <span className="text-success">{release.seeders ?? '?'}</span>
            <span className="text-default-400">/</span>
            <span className="text-danger">{release.leechers ?? '?'}</span>
          </div>
        ),
        sortFn: (a, b) => (a.seeders ?? 0) - (b.seeders ?? 0),
      },
      {
        key: 'age',
        label: 'AGE',
        width: 70,
        sortable: true,
        render: (release) => (
          <Tooltip content={new Date(release.publishDate).toLocaleString()}>
            <span className="text-sm text-default-500">
              {formatAge(release.publishDate)}
            </span>
          </Tooltip>
        ),
        sortFn: (a, b) =>
          new Date(b.publishDate).getTime() - new Date(a.publishDate).getTime(),
      },
    ],
    []
  )

  // Row actions
  const rowActions: RowAction<TorrentRelease>[] = useMemo(
    () => [
      {
        key: 'download',
        label: 'Download',
        icon: <IconDownload size={16} />,
        onAction: handleDownload,
      },
      {
        key: 'add-to-library',
        label: 'Download + Add to Library',
        icon: <IconPlus size={16} />,
        inDropdown: true,
        onAction: (release) => {
          setSelectedRelease(release)
          onAddModalOpen()
        },
      },
    ],
    [handleDownload, onAddModalOpen]
  )

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 min-w-0 grow flex flex-col  gap-4">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold">Hunt</h1>
        <p className="text-default-500 text-sm">
          Hunt for content across all configured indexers
        </p>
      </div>

      {/* Search Bar */}
      <Card>
        <CardBody className="flex flex-row items-center gap-4">
          <Select
            label="Type"
            selectedKeys={[mediaType]}
            onChange={(e) => setMediaType(e.target.value as MediaType)}
            className="w-40"
            size="sm"
          >
            {MEDIA_TYPES.map((type) => (
              <SelectItem key={type.value} textValue={type.label}>
                <div className="flex items-center gap-2">
                  <type.icon size={16} />
                  {type.label}
                </div>
              </SelectItem>
            ))}
          </Select>

          <Input
            labelPlacement="inside"
            variant="flat"
            placeholder="Search for torrents..."
            value={searchInput}
            onChange={(e) => setSearchInput(e.target.value)}
            onKeyDown={handleKeyDown}
            startContent={<IconSearch size={18} />}
            className="flex-1"
            classNames={{
              label: 'text-sm font-medium text-primary!',
            }}
            size="lg"
            autoFocus
            endContent={
              <Button
                size="sm"
                variant="light"
                color="primary"
                className="font-semibold"
                onPress={handleSearch}
                isLoading={isSearching}
                isDisabled={!searchInput.trim() || searchInput.trim().length < 2}
              >
                Search
              </Button>
            }
          />
        </CardBody>
      </Card>

      {/* Search Meta */}
      {searchMeta && (
        <div className="flex items-center gap-4 text-sm text-default-500">
          <span>
            Found <strong className="text-foreground">{searchMeta.totalReleases}</strong> releases
            from {searchMeta.indexerCount} indexer{searchMeta.indexerCount !== 1 ? 's' : ''}
          </span>
          <span>
            <IconClock size={14} className="inline mr-1" />
            {searchMeta.totalElapsedMs}ms
          </span>
          {searchMeta.errors.length > 0 && (
            <Tooltip content={searchMeta.errors.join('\n')}>
              <Chip size="sm" color="warning" variant="flat">
                {searchMeta.errors.length} error{searchMeta.errors.length !== 1 ? 's' : ''}
              </Chip>
            </Tooltip>
          )}
        </div>
      )}

      {/* Results */}
      {isSearching ? (
        <div className="flex justify-center items-center py-12">
          <Spinner size="lg" />
          <span className="ml-3 text-default-500">Searching indexers...</span>
        </div>
      ) : results.length > 0 ? (
        <div className="flex-1 min-h-0">
          <DataTable
            stateKey="search-results"
            data={results}
            columns={columns}
            getRowKey={(release) => release.guid}
            searchPlaceholder="Filter results..."
            sortColumn={sortColumn ?? 'seeders'}
            sortDirection={sortDirection}
            onSortChange={handleSortChange}
            rowActions={rowActions}
            showItemCount
            ariaLabel="Search results"
            fillHeight
          />
        </div>
      ) : query && !isSearching ? (
        <Card className="bg-content1/50 border-default-300 border-dashed border-2">
          <CardBody className="py-12 text-center">
            <IconSearch size={48} className="mx-auto mb-4 text-default-400" />
            <h3 className="text-lg font-semibold mb-2">No results found</h3>
            <p className="text-default-500">
              Try a different search term or check your indexer configuration.
            </p>
          </CardBody>
        </Card>
      ) : (
        <Card className="bg-content1/50 border-default-300 border-dashed border-2">
          <CardBody className="py-12 text-center">
            <IconSearch size={48} className="mx-auto mb-4 text-primary-400" />
            <h3 className="text-lg font-semibold mb-2">Search for torrents</h3>
            <p className="text-default-500">
              Enter a search term above to find releases across all your configured indexers.
            </p>
          </CardBody>
        </Card>
      )}

      {/* Add to Library Modal */}
      <AddToLibraryModal
        isOpen={isAddModalOpen}
        onClose={onAddModalClose}
        release={selectedRelease}
        onAdded={() => {
          // Refresh or update UI as needed
        }}
      />
    </div>
  )
}
