import { createFileRoute, Link, redirect } from '@tanstack/react-router'
import { useState, useEffect, useCallback, useMemo } from 'react'
import {
  Button,
  Card,
  CardBody,
  Chip,
  Image,
  Spinner,
  Table,
  TableHeader,
  TableBody,
  TableColumn,
  TableRow,
  TableCell,
  Accordion,
  AccordionItem,
  addToast,
} from '@heroui/react'
import { useAuth } from '../../hooks/useAuth'
import {
  graphqlClient,
  TV_SHOW_QUERY,
  EPISODES_QUERY,
  REFRESH_TV_SHOW_MUTATION,
  type TvShow,
  type Episode,
  type EpisodeStatus,
} from '../../lib/graphql'

export const Route = createFileRoute('/shows/$showId')({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({ to: '/auth/login' })
    }
  },
  component: ShowDetailPage,
})

function formatBytes(bytes: number | null): string {
  if (!bytes) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let unitIndex = 0
  let size = bytes
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex++
  }
  return `${size.toFixed(1)} ${units[unitIndex]}`
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return 'TBA'
  const date = new Date(dateStr)
  return date.toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}

function getStatusColor(status: EpisodeStatus): 'success' | 'warning' | 'danger' | 'default' | 'primary' {
  switch (status) {
    case 'DOWNLOADED':
      return 'success'
    case 'DOWNLOADING':
      return 'primary'
    case 'WANTED':
      return 'warning'
    case 'MISSING':
      return 'danger'
    case 'IGNORED':
      return 'default'
    default:
      return 'default'
  }
}

function getStatusLabel(status: EpisodeStatus): string {
  switch (status) {
    case 'DOWNLOADED':
      return 'Downloaded'
    case 'DOWNLOADING':
      return 'Downloading'
    case 'WANTED':
      return 'Wanted'
    case 'MISSING':
      return 'Missing'
    case 'IGNORED':
      return 'Ignored'
    default:
      return status
  }
}

interface SeasonData {
  season: number
  episodes: Episode[]
  downloadedCount: number
  totalCount: number
}

function ShowDetailPage() {
  const { showId } = Route.useParams()
  const { user, loading: authLoading } = useAuth()
  const [show, setShow] = useState<TvShow | null>(null)
  const [episodes, setEpisodes] = useState<Episode[]>([])
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)

  const fetchData = useCallback(async () => {
    try {
      setLoading(true)

      const [showResult, episodesResult] = await Promise.all([
        graphqlClient
          .query<{ tvShow: TvShow | null }>(TV_SHOW_QUERY, { id: showId })
          .toPromise(),
        graphqlClient
          .query<{ episodes: Episode[] }>(EPISODES_QUERY, { tvShowId: showId })
          .toPromise(),
      ])

      if (showResult.data?.tvShow) {
        setShow(showResult.data.tvShow)
      }
      if (episodesResult.data?.episodes) {
        setEpisodes(episodesResult.data.episodes)
      }
    } catch (err) {
      console.error('Failed to fetch show data:', err)
    } finally {
      setLoading(false)
    }
  }, [showId])

  useEffect(() => {
    if (user) {
      fetchData()
    }
  }, [user, fetchData])

  const handleRefresh = async () => {
    setRefreshing(true)
    try {
      const { data, error } = await graphqlClient
        .mutation<{ refreshTvShow: { success: boolean; error: string | null } }>(
          REFRESH_TV_SHOW_MUTATION,
          { id: showId }
        )
        .toPromise()

      if (error || !data?.refreshTvShow.success) {
        addToast({
          title: 'Error',
          description: data?.refreshTvShow.error || 'Failed to refresh show',
          color: 'danger',
        })
        return
      }

      addToast({
        title: 'Refreshed',
        description: 'Show metadata updated',
        color: 'success',
      })

      await fetchData()
    } catch (err) {
      console.error('Failed to refresh show:', err)
    } finally {
      setRefreshing(false)
    }
  }

  // Group episodes by season
  const seasons = useMemo<SeasonData[]>(() => {
    const seasonMap = new Map<number, Episode[]>()

    for (const ep of episodes) {
      if (!seasonMap.has(ep.season)) {
        seasonMap.set(ep.season, [])
      }
      seasonMap.get(ep.season)!.push(ep)
    }

    return Array.from(seasonMap.entries())
      .map(([season, eps]) => ({
        season,
        episodes: eps.sort((a, b) => a.episode - b.episode),
        downloadedCount: eps.filter((e) => e.status === 'DOWNLOADED').length,
        totalCount: eps.length,
      }))
      .sort((a, b) => a.season - b.season)
  }, [episodes])

  // Calculate totals
  const totalEpisodes = episodes.length
  const downloadedEpisodes = episodes.filter((e) => e.status === 'DOWNLOADED').length
  const missingEpisodes = episodes.filter((e) => e.status === 'MISSING' || e.status === 'WANTED').length

  if (authLoading || loading) {
    return (
      <div className="flex items-center justify-center h-[calc(100vh-4rem)]">
        <Spinner size="lg" color="primary" />
      </div>
    )
  }

  if (!show) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-8">
        <Card className="bg-content1">
          <CardBody className="text-center py-12">
            <h2 className="text-xl font-semibold mb-4">Show not found</h2>
            <Link to="/libraries">
              <Button color="primary">Back to Libraries</Button>
            </Link>
          </CardBody>
        </Card>
      </div>
    )
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      {/* Header with Show Info */}
      <div className="flex flex-col md:flex-row gap-6 mb-8">
        {/* Poster */}
        <div className="flex-shrink-0">
          {show.posterUrl ? (
            <Image
              src={show.posterUrl}
              alt={show.name}
              className="w-48 h-72 object-cover rounded-lg shadow-lg"
            />
          ) : (
            <div className="w-48 h-72 bg-default-200 rounded-lg flex items-center justify-center">
              <span className="text-6xl">📺</span>
            </div>
          )}
        </div>

        {/* Show Details */}
        <div className="flex-1">
          <div className="flex items-center gap-2 mb-2 text-sm text-default-500">
            <Link to="/libraries" className="hover:text-default-700">
              Libraries
            </Link>
            <span>/</span>
            <Link
              to={`/libraries/${show.libraryId}` as any}
              className="hover:text-default-700"
            >
              Library
            </Link>
            <span>/</span>
            <span>{show.name}</span>
          </div>

          <h1 className="text-3xl font-bold mb-2">
            {show.name}
            {show.year && (
              <span className="text-default-500 ml-2">({show.year})</span>
            )}
          </h1>

          <div className="flex flex-wrap gap-2 mb-4">
            <Chip
              size="sm"
              color={show.status === 'CONTINUING' ? 'success' : 'default'}
              variant="flat"
            >
              {show.status}
            </Chip>
            {show.network && (
              <Chip size="sm" variant="flat">
                {show.network}
              </Chip>
            )}
            {show.monitored && (
              <Chip size="sm" color="primary" variant="flat">
                Monitored
              </Chip>
            )}
          </div>

          {show.overview && (
            <p className="text-default-600 mb-4 line-clamp-3">{show.overview}</p>
          )}

          <div className="flex gap-4 text-sm text-default-500 mb-4">
            <div>
              <span className="font-semibold text-foreground">{downloadedEpisodes}</span>
              <span> / {totalEpisodes} episodes</span>
            </div>
            {missingEpisodes > 0 && (
              <div className="text-warning">
                <span className="font-semibold">{missingEpisodes}</span>
                <span> missing</span>
              </div>
            )}
            {show.sizeBytes > 0 && (
              <div>
                <span className="font-semibold text-foreground">{formatBytes(show.sizeBytes)}</span>
                <span> on disk</span>
              </div>
            )}
          </div>

          <div className="flex gap-2">
            <Button
              color="primary"
              variant="flat"
              onPress={handleRefresh}
              isLoading={refreshing}
            >
              Refresh Metadata
            </Button>
            <Button
              variant="flat"
              as={Link}
              to={`/libraries/${show.libraryId}` as any}
            >
              Back to Library
            </Button>
          </div>
        </div>
      </div>

      {/* Seasons & Episodes */}
      <div className="space-y-4">
        <h2 className="text-xl font-semibold">Seasons & Episodes</h2>

        {seasons.length === 0 ? (
          <Card className="bg-content1/50 border-default-300 border-dashed border-2">
            <CardBody className="py-12 text-center">
              <span className="text-5xl mb-4 block">📋</span>
              <h3 className="text-lg font-semibold mb-2">No episodes found</h3>
              <p className="text-default-500 mb-4">
                Try refreshing the show metadata to fetch episodes.
              </p>
              <Button color="primary" onPress={handleRefresh} isLoading={refreshing}>
                Refresh Metadata
              </Button>
            </CardBody>
          </Card>
        ) : (
          <Accordion variant="splitted" selectionMode="multiple" defaultExpandedKeys={seasons.length <= 3 ? seasons.map(s => String(s.season)) : []}>
            {seasons.map((seasonData) => (
              <AccordionItem
                key={String(seasonData.season)}
                aria-label={`Season ${seasonData.season}`}
                title={
                  <div className="flex items-center justify-between w-full pr-4">
                    <span className="font-semibold">
                      {seasonData.season === 0 ? 'Specials' : `Season ${seasonData.season}`}
                    </span>
                    <div className="flex items-center gap-2">
                      <Chip
                        size="sm"
                        color={seasonData.downloadedCount === seasonData.totalCount ? 'success' : 'warning'}
                        variant="flat"
                      >
                        {seasonData.downloadedCount} / {seasonData.totalCount}
                      </Chip>
                    </div>
                  </div>
                }
                className="bg-content1"
              >
                <Table
                  aria-label={`Season ${seasonData.season} episodes`}
                  removeWrapper
                  className="min-w-full"
                >
                  <TableHeader>
                    <TableColumn width={80}>#</TableColumn>
                    <TableColumn>TITLE</TableColumn>
                    <TableColumn width={120}>AIR DATE</TableColumn>
                    <TableColumn width={100}>STATUS</TableColumn>
                  </TableHeader>
                  <TableBody>
                    {seasonData.episodes.map((ep) => (
                      <TableRow key={ep.id}>
                        <TableCell>
                          <span className="font-mono text-default-500">
                            {String(ep.episode).padStart(2, '0')}
                          </span>
                        </TableCell>
                        <TableCell>
                          <div>
                            <span className="font-medium">
                              {ep.title || `Episode ${ep.episode}`}
                            </span>
                          </div>
                        </TableCell>
                        <TableCell>
                          <span className="text-default-500 text-sm">
                            {formatDate(ep.airDate)}
                          </span>
                        </TableCell>
                        <TableCell>
                          <Chip
                            size="sm"
                            color={getStatusColor(ep.status)}
                            variant="flat"
                          >
                            {getStatusLabel(ep.status)}
                          </Chip>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </AccordionItem>
            ))}
          </Accordion>
        )}
      </div>
    </div>
  )
}
