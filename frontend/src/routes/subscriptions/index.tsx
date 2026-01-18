import { createFileRoute, redirect, Link } from '@tanstack/react-router'
import { useState, useEffect, useCallback, useRef, useMemo } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody, CardHeader } from '@heroui/card'
import { Chip } from '@heroui/chip'
import { Divider } from '@heroui/divider'
import { Skeleton } from '@heroui/skeleton'
import { Image } from '@heroui/image'
import {
  graphqlClient,
  LIBRARIES_QUERY,
  TV_SHOWS_QUERY,
  type Library,
  type TvShow,
} from '../../lib/graphql'
import { useDataReactivity } from '../../hooks/useSubscription'
import { DataTable } from '../../components/data-table'
import type { DataTableColumn, CardRendererProps, FilterOption } from '../../components/data-table/types'
import { RouteError } from '../../components/RouteError'
import { IconDeviceTv } from '@tabler/icons-react'

export const Route = createFileRoute('/subscriptions/')({
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
  component: SubscriptionsPage,
  errorComponent: RouteError,
})

interface SubscriptionItem {
  show: TvShow
  libraryName: string
}

function SubscriptionsPage() {
  const [shows, setShows] = useState<SubscriptionItem[]>([])
  const [isLoading, setIsLoading] = useState(true)

  const initialLoadDone = useRef(false)

  const fetchData = useCallback(async (isBackgroundRefresh = false) => {
    try {
      if (!isBackgroundRefresh) {
        setIsLoading(true)
      }

      // Fetch all libraries
      const librariesResult = await graphqlClient
        .query<{ libraries: Library[] }>(LIBRARIES_QUERY)
        .toPromise()

      if (!librariesResult.data?.libraries) {
        setShows([])
        return
      }

      // Filter to TV libraries and fetch shows for each
      const tvLibraries = librariesResult.data.libraries.filter(
        (lib) => lib.libraryType === 'TV'
      )

      const allShowsWithLibrary: SubscriptionItem[] = []

      for (const library of tvLibraries) {
        const showsResult = await graphqlClient
          .query<{ tvShows: TvShow[] }>(TV_SHOWS_QUERY, { libraryId: library.id })
          .toPromise()

        if (showsResult.data?.tvShows) {
          for (const show of showsResult.data.tvShows) {
            allShowsWithLibrary.push({
              show,
              libraryName: library.name,
            })
          }
        }
      }

      // Sort by name
      allShowsWithLibrary.sort((a, b) => a.show.name.localeCompare(b.show.name))

      setShows(allShowsWithLibrary)
    } catch (err) {
      console.error('Failed to fetch subscriptions:', err)
    } finally {
      setIsLoading(false)
      initialLoadDone.current = true
    }
  }, [])

  useEffect(() => {
    fetchData()
  }, [fetchData])

  // Subscribe to data changes for live updates
  useDataReactivity(
    () => {
      if (initialLoadDone.current) {
        fetchData(true)
      }
    },
    { onTorrentComplete: true, periodicInterval: 60000, onFocus: true }
  )

  // Counts for filter options
  const monitoredCount = useMemo(() => shows.filter((s) => s.show.monitored).length, [shows])
  const libraryCount = useMemo(() => new Set(shows.map((s) => s.libraryName)).size, [shows])

  // Define columns (needed for DataTable but we use card view)
  const columns: DataTableColumn<SubscriptionItem>[] = useMemo(
    () => [
      {
        key: 'name',
        label: 'Name',
        sortFn: (a, b) => a.show.name.localeCompare(b.show.name),
      },
      {
        key: 'library',
        label: 'Library',
        sortFn: (a, b) => a.libraryName.localeCompare(b.libraryName),
      },
      {
        key: 'monitored',
        label: 'Monitored',
        sortFn: (a, b) => Number(b.show.monitored) - Number(a.show.monitored),
      },
    ],
    []
  )

  // Filter options for monitored status
  const monitoredFilterOptions: FilterOption[] = useMemo(
    () => [
      { key: 'monitored', label: 'Monitored', color: 'success', count: monitoredCount },
      { key: 'not_monitored', label: 'Not Monitored', color: 'default', count: shows.length - monitoredCount },
    ],
    [monitoredCount, shows.length]
  )

  // Card renderer for subscription items
  const renderCard = useCallback(
    ({ item }: CardRendererProps<SubscriptionItem>) => {
      const { show, libraryName } = item
      return (
        <Link to="/shows/$showId" params={{ showId: show.id }}>
          <Card className="bg-content1 hover:bg-content2 transition-colors h-full">
            <CardBody>
              <div className="flex gap-3">
                {show.posterUrl ? (
                  <Image
                    src={show.posterUrl}
                    alt={show.name}
                    className="h-24 object-cover rounded-md aspect-[2/3]"
                    fallbackSrc="/placeholder.jpg"
                  />
                ) : (
                  <div className="w-12 h-18 rounded-md bg-default-200 flex items-center justify-center">
                    <IconDeviceTv size={24} className="text-blue-400" />
                  </div>
                )}
                <div className="flex-1 min-w-0">
                  <div className="flex items-start justify-between mb-1">
                    <div className="min-w-0">
                      <h3 className="font-semibold text-foreground truncate">{show.name}</h3>
                      <p className="text-default-500 text-xs">{libraryName}</p>
                    </div>
                    <Chip
                      size="sm"
                      color={show.monitored ? 'success' : 'default'}
                      variant="flat"
                    >
                      {show.monitored ? 'Monitored' : 'Not Monitored'}
                    </Chip>
                  </div>

                  <div className="flex gap-2 text-xs text-default-400 mt-2">
                    <span>{show.year || 'Unknown year'}</span>
                    {show.network && (
                      <>
                        <span>•</span>
                        <span>{show.network}</span>
                      </>
                    )}
                    {show.status && (
                      <>
                        <span>•</span>
                        <Chip size="sm" variant="flat" className="text-xs h-5">
                          {show.status}
                        </Chip>
                      </>
                    )}
                  </div>

                  <div className="flex gap-2 text-xs text-default-500 mt-1">
                    <span>{show.episodeFileCount ?? 0} / {show.episodeCount ?? 0} episodes</span>
                  </div>
                </div>
              </div>
            </CardBody>
          </Card>
        </Link>
      )
    },
    []
  )

  // Search function
  const searchFn = useCallback((item: SubscriptionItem, searchTerm: string) => {
    const term = searchTerm.toLowerCase()
    return (
      item.show.name.toLowerCase().includes(term) ||
      item.libraryName.toLowerCase().includes(term) ||
      (item.show.network?.toLowerCase().includes(term) ?? false)
    )
  }, [])

  // Empty content
  const emptyContent = useMemo(
    () => (
      <Card className="bg-content1">
        <CardBody className="text-center py-12">
          <IconDeviceTv size={48} className="mx-auto mb-4 text-blue-400" />
          <p className="text-lg text-default-600 mb-2">No TV shows yet</p>
          <p className="text-sm text-default-400 mb-4">
            Add shows to your TV libraries to start monitoring for new episodes.
          </p>
          <Link to="/libraries">
            <Button color="primary">Go to Libraries</Button>
          </Link>
        </CardBody>
      </Card>
    ),
    []
  )

  // Header content
  const headerContent = useMemo(
    () => (
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold">Subscriptions</h1>
          <span className="text-default-500 text-sm block">
            {isLoading ? (
              <Skeleton className="w-48 h-4 rounded inline-block" />
            ) : (
              `${monitoredCount} monitored shows across ${libraryCount} libraries`
            )}
          </span>
        </div>
        <Link to="/libraries">
          <Button color="primary">Manage Libraries</Button>
        </Link>
      </div>
    ),
    [isLoading, monitoredCount, libraryCount]
  )

  // Footer content with info box
  const footerContent = useMemo(
    () => (
      <Card className="mt-4 bg-content2">
        <CardHeader>
          <h3 className="font-semibold">How Subscriptions Work</h3>
        </CardHeader>
        <Divider />
        <CardBody>
          <ul className="text-default-500 text-sm space-y-1">
            <li>• Monitored shows are checked for new episodes via RSS feeds</li>
            <li>• When new episodes match your quality profile, they're automatically downloaded</li>
            <li>• Toggle monitoring on individual show pages or in the library settings</li>
            <li>• Missing episodes are highlighted and can be manually searched</li>
          </ul>
        </CardBody>
      </Card>
    ),
    []
  )

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col grow">
      <DataTable
        stateKey="subscriptions"
        data={shows}
        columns={columns}
        getRowKey={(item) => item.show.id}
        isLoading={isLoading}
        skeletonRowCount={6}
        emptyContent={emptyContent}
        // Search
        searchFn={searchFn}
        searchPlaceholder="Search shows..."
        // Filters
        filters={[
          {
            key: 'monitored',
            label: 'Status',
            type: 'select',
            options: monitoredFilterOptions,
            filterFn: (item, value) => {
              if (!value) return true
              if (value === 'monitored') return item.show.monitored
              if (value === 'not_monitored') return !item.show.monitored
              return true
            },
          },
        ]}
        // Sorting
        defaultSortColumn="name"
        defaultSortDirection="asc"
        // View mode - card only
        defaultViewMode="cards"
        cardRenderer={renderCard}
        cardGridClassName="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4"
        // Layout
        headerContent={headerContent}
        footerContent={footerContent}
        showItemCount={false}
        fillHeight
      />
    </div>
  )
}
