import { createFileRoute, Link, redirect, useNavigate } from '@tanstack/react-router'
import { useState, useEffect, useCallback, useMemo, useRef } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Chip } from '@heroui/chip'
import { Image } from '@heroui/image'
import { Skeleton } from '@heroui/skeleton'
import { Accordion, AccordionItem } from '@heroui/accordion'
import { Breadcrumbs, BreadcrumbItem } from '@heroui/breadcrumbs'
import { useDisclosure } from '@heroui/modal'
import { addToast } from '@heroui/toast'
import { useDataReactivity } from '../../hooks/useSubscription'
import { RouteError } from '../../components/RouteError'
import { sanitizeError } from '../../lib/format'
import {
  graphqlClient,
  TV_SHOW_QUERY,
  LIBRARY_QUERY,
  EPISODES_QUERY,
  REFRESH_TV_SHOW_MUTATION,
  DOWNLOAD_EPISODE_MUTATION,
  DELETE_TV_SHOW_MUTATION,
  UPDATE_TV_SHOW_MUTATION,
  type TvShow,
  type Library,
  type Episode,
  type EpisodeStatus,
  type DownloadEpisodeResult,
  type TvShowResult,
} from '../../lib/graphql'
import { formatBytes, formatDate } from '../../lib/format'
import { DataTable, type DataTableColumn, type RowAction } from '../../components/data-table'
import { IconDownload, IconDeviceTv, IconClipboard, IconPlayerPlay, IconRefresh, IconSearch, IconSettings, IconTrash } from '@tabler/icons-react'
import { Tooltip } from '@heroui/tooltip'
import { DeleteShowModal, ShowSettingsModal, type ShowSettingsInput } from '../../components/shows'
import { 
  EpisodeStatusChip,
  AutoDownloadBadge,
  AutoHuntBadge,
  FileOrganizationBadge,
  MonitoredBadge,
  QualityFilterBadge,
} from '../../components/shared'
import { usePlaybackContext } from '../../contexts/PlaybackContext'

export const Route = createFileRoute('/shows/$showId')({
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
  component: ShowDetailPage,
  errorComponent: RouteError,
})

// Helper to format air date with 'TBA' fallback for unknown dates
function formatAirDate(dateStr: string | null): string {
  return formatDate(dateStr, 'TBA')
}


interface SeasonData {
  season: number
  episodes: Episode[]
  downloadedCount: number
  totalCount: number
}

// Episode table columns
const episodeColumns: DataTableColumn<Episode>[] = [
  {
    key: 'episode',
    label: '#',
    width: 80,
    sortable: true,
    render: (ep) => (
      <span className="font-mono text-default-500">
        {String(ep.episode).padStart(2, '0')}
      </span>
    ),
  },
  {
    key: 'title',
    label: 'Title',
    sortable: true,
    render: (ep) => (
      <span className="font-medium">
        {ep.title || `Episode ${ep.episode}`}
      </span>
    ),
  },
  {
    key: 'airDate',
    label: 'Air Date',
    width: 120,
    sortable: true,
    render: (ep) => (
      <span className="text-default-500 text-sm">
        {formatAirDate(ep.airDate)}
      </span>
    ),
  },
  {
    key: 'status',
    label: 'Status',
    width: 120,
    sortable: true,
    render: (ep) => <EpisodeStatusChip status={ep.status} />,
  },
]

interface EpisodeTableProps {
  episodes: Episode[]
  seasonNumber: number
  downloadingEpisodes: Set<string>
  onDownload: (episodeId: string) => void
  onPlay: (episode: Episode) => void
  onSearch: (episode: Episode) => void
}

function EpisodeTable({ episodes, seasonNumber, downloadingEpisodes, onDownload, onPlay, onSearch }: EpisodeTableProps) {
  const rowActions = useMemo<RowAction<Episode>[]>(() => [
    {
      key: 'play',
      label: 'Play',
      icon: <IconPlayerPlay size={16} />,
      color: 'success',
      inDropdown: false,
      isVisible: (ep) => ep.status === 'DOWNLOADED' && !!ep.mediaFileId,
      onAction: (ep) => onPlay(ep),
    },
    {
      key: 'search',
      label: 'Search for Episode',
      icon: <IconSearch size={16} />,
      color: 'default',
      inDropdown: false,
      isVisible: (ep) => ep.status === 'MISSING' || ep.status === 'WANTED',
      onAction: (ep) => onSearch(ep),
    },
    {
      key: 'download',
      label: 'Download Episode',
      icon: <IconDownload size={16} />,
      color: 'primary',
      inDropdown: true,
      isVisible: (ep) => ep.status === 'AVAILABLE',
      isDisabled: (ep) => downloadingEpisodes.has(ep.id),
      onAction: (ep) => onDownload(ep.id),
    },
  ], [downloadingEpisodes, onDownload, onPlay, onSearch])

  return (
    <DataTable
      data={episodes}
      columns={episodeColumns}
      getRowKey={(ep) => ep.id}
      ariaLabel={`Season ${seasonNumber} episodes`}
      removeWrapper
      isCompact
      showItemCount={false}
      hideToolbar
      defaultSortColumn="episode"
      defaultSortDirection="asc"
      rowActions={rowActions}
    />
  )
}

function ShowDetailPage() {
  const { showId } = Route.useParams()
  const navigate = useNavigate()
  const [show, setShow] = useState<TvShow | null>(null)
  const [library, setLibrary] = useState<Library | null>(null)
  const [episodes, setEpisodes] = useState<Episode[]>([])
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(false)
  const [downloadingEpisodes, setDownloadingEpisodes] = useState<Set<string>>(new Set())
  const [deleting, setDeleting] = useState(false)
  const [savingSettings, setSavingSettings] = useState(false)
  const [togglingAutoDownload, setTogglingAutoDownload] = useState(false)
  const { isOpen: isDeleteOpen, onOpen: onDeleteOpen, onClose: onDeleteClose } = useDisclosure()
  const { isOpen: isSettingsOpen, onOpen: onSettingsOpen, onClose: onSettingsClose } = useDisclosure()
  const { startPlayback, setCurrentEpisode, setCurrentShow } = usePlaybackContext()

  // Track if initial load is done to avoid showing spinner on background refreshes
  const initialLoadDone = useRef(false)

  // Update page title when show data is loaded
  useEffect(() => {
    if (show) {
      document.title = `Librarian - ${show.name}`
    }
    return () => {
      document.title = 'Librarian'
    }
  }, [show])

  const fetchData = useCallback(async (isBackgroundRefresh = false) => {
    try {
      // Only show loading spinner on initial load
      if (!isBackgroundRefresh) {
        setLoading(true)
      }

      // First fetch the show to get libraryId
      const showResult = await graphqlClient
        .query<{ tvShow: TvShow | null }>(TV_SHOW_QUERY, { id: showId })
        .toPromise()

      if (showResult.data?.tvShow) {
        setShow(showResult.data.tvShow)

        // Now fetch library and episodes in parallel
        const [libraryResult, episodesResult] = await Promise.all([
          graphqlClient
            .query<{ library: Library | null }>(LIBRARY_QUERY, { id: showResult.data.tvShow.libraryId })
            .toPromise(),
          graphqlClient
            .query<{ episodes: Episode[] }>(EPISODES_QUERY, { tvShowId: showId })
            .toPromise(),
        ])

        if (libraryResult.data?.library) {
          setLibrary(libraryResult.data.library)
        }
        if (episodesResult.data?.episodes) {
          setEpisodes(episodesResult.data.episodes)
        }
      }
    } catch (err) {
      console.error('Failed to fetch show data:', err)
    } finally {
      setLoading(false)
      initialLoadDone.current = true
    }
  }, [showId])

  useEffect(() => {
    fetchData()
  }, [fetchData])

  // Subscribe to data changes for live updates (especially episode status changes)
  useDataReactivity(
    () => {
      if (initialLoadDone.current) {
        fetchData(true)
      }
    },
    { onTorrentComplete: true, periodicInterval: 15000, onFocus: true }
  )


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
          description: sanitizeError(data?.refreshTvShow.error || 'Failed to refresh show'),
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

  const handleDownloadEpisode = async (episodeId: string) => {
    setDownloadingEpisodes(prev => new Set([...prev, episodeId]))
    try {
      const { data, error } = await graphqlClient
        .mutation<{ downloadEpisode: DownloadEpisodeResult }>(
          DOWNLOAD_EPISODE_MUTATION,
          { episodeId }
        )
        .toPromise()

      if (error || !data?.downloadEpisode.success) {
        addToast({
          title: 'Download Failed',
          description: sanitizeError(data?.downloadEpisode.error || 'Failed to start download'),
          color: 'danger',
        })
        return
      }

      addToast({
        title: 'Download Started',
        description: 'Episode download has been queued',
        color: 'success',
      })

      // Update the episode status in local state
      setEpisodes(prev =>
        prev.map(ep =>
          ep.id === episodeId
            ? { ...ep, status: 'DOWNLOADING' as EpisodeStatus }
            : ep
        )
      )
    } catch (err) {
      console.error('Failed to download episode:', err)
      addToast({
        title: 'Error',
        description: 'Failed to start download',
        color: 'danger',
      })
    } finally {
      setDownloadingEpisodes(prev => {
        const next = new Set(prev)
        next.delete(episodeId)
        return next
      })
    }
  }

  const handleDelete = async () => {
    setDeleting(true)
    try {
      const { data, error } = await graphqlClient
        .mutation<{ deleteTvShow: { success: boolean; error: string | null } }>(
          DELETE_TV_SHOW_MUTATION,
          { id: showId }
        )
        .toPromise()

      if (error || !data?.deleteTvShow.success) {
        addToast({
          title: 'Error',
          description: sanitizeError(data?.deleteTvShow.error || 'Failed to delete show'),
          color: 'danger',
        })
        return
      }

      addToast({
        title: 'Deleted',
        description: 'Show has been removed from library',
        color: 'success',
      })

      onDeleteClose()
      navigate({ to: '/libraries/$libraryId', params: { libraryId: show?.libraryId || '' } })
    } catch (err) {
      console.error('Failed to delete show:', err)
      addToast({
        title: 'Error',
        description: 'Failed to delete show',
        color: 'danger',
      })
    } finally {
      setDeleting(false)
    }
  }

  const handleSaveSettings = async (input: ShowSettingsInput) => {
    setSavingSettings(true)
    try {
      const { data, error } = await graphqlClient
        .mutation<{ updateTvShow: TvShowResult }>(
          UPDATE_TV_SHOW_MUTATION,
          { id: showId, input }
        )
        .toPromise()

      if (error || !data?.updateTvShow.success) {
        addToast({
          title: 'Error',
          description: sanitizeError(data?.updateTvShow.error || 'Failed to save settings'),
          color: 'danger',
        })
        return
      }

      if (data.updateTvShow.tvShow) {
        setShow(prev => prev ? { ...prev, ...data.updateTvShow.tvShow } : null)
      }

      addToast({
        title: 'Saved',
        description: 'Show settings updated',
        color: 'success',
      })

      onSettingsClose()
    } catch (err) {
      console.error('Failed to save settings:', err)
      addToast({
        title: 'Error',
        description: 'Failed to save settings',
        color: 'danger',
      })
    } finally {
      setSavingSettings(false)
    }
  }

  const handlePlay = useCallback(async (episode: Episode) => {
    if (episode.mediaFileId && show) {
      // Set metadata for the persistent player
      setCurrentEpisode(episode)
      setCurrentShow(show)
      // Start playback (this will trigger the persistent player)
      await startPlayback({
        episodeId: episode.id,
        mediaFileId: episode.mediaFileId,
        tvShowId: show.id,
      }, episode, show)
    }
  }, [show, startPlayback, setCurrentEpisode, setCurrentShow])

  // Navigate to hunt page with pre-filled query for missing episode
  const handleSearchEpisode = useCallback((episode: Episode) => {
    if (!show) return
    
    // Build search query: "Show Name S01E05"
    const seasonPadded = String(episode.season).padStart(2, '0')
    const episodePadded = String(episode.episode).padStart(2, '0')
    const searchQuery = `${show.name} S${seasonPadded}E${episodePadded}`
    
    // Navigate to hunt page with query params
    navigate({
      to: '/hunt',
      search: {
        q: searchQuery,
        type: 'tv',
      },
    })
  }, [show, navigate])

  const handleToggleAutoDownload = async (enabled: boolean) => {
    setTogglingAutoDownload(true)
    try {
      const { data, error } = await graphqlClient
        .mutation<{ updateTvShow: TvShowResult }>(
          UPDATE_TV_SHOW_MUTATION,
          { id: showId, input: { autoDownloadOverride: enabled } }
        )
        .toPromise()

      if (error || !data?.updateTvShow.success) {
        addToast({
          title: 'Error',
          description: sanitizeError(data?.updateTvShow.error || 'Failed to update auto-download'),
          color: 'danger',
        })
        return
      }

      if (data.updateTvShow.tvShow) {
        setShow(prev => prev ? { ...prev, ...data.updateTvShow.tvShow } : null)
      }
    } catch (err) {
      console.error('Failed to toggle auto-download:', err)
      addToast({
        title: 'Error',
        description: 'Failed to update auto-download',
        color: 'danger',
      })
    } finally {
      setTogglingAutoDownload(false)
    }
  }

  // Scroll to a season accordion
  const scrollToSeason = (seasonNumber: number) => {
    const element = document.getElementById(`season-${seasonNumber}`)
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' })
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

  // Calculate totals (memoized to avoid recalculating on every render)
  const { totalEpisodes, downloadedEpisodes, missingEpisodes } = useMemo(() => ({
    totalEpisodes: episodes.length,
    downloadedEpisodes: episodes.filter((e) => e.status === 'DOWNLOADED').length,
    missingEpisodes: episodes.filter((e) => e.status === 'MISSING' || e.status === 'WANTED').length,
  }), [episodes])

  // Loading skeleton for show detail page
  if (loading) {
    return (
      <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col mb-20">
        {/* Header Skeleton */}
        <div className="flex flex-col md:flex-row gap-6 mb-8">
          {/* Poster Skeleton */}
          <Skeleton className="w-48 h-72 rounded-lg shrink-0" />
          
          {/* Details Skeleton */}
          <div className="flex-1">
            <Skeleton className="w-48 h-4 rounded mb-4" />
            <Skeleton className="w-64 h-8 rounded mb-4" />
            <div className="flex gap-2 mb-4">
              <Skeleton className="w-20 h-6 rounded-full" />
              <Skeleton className="w-16 h-6 rounded-full" />
              <Skeleton className="w-24 h-6 rounded-full" />
            </div>
            <Skeleton className="w-full h-16 rounded mb-4" />
            <div className="flex gap-4 mb-4">
              <Skeleton className="w-32 h-4 rounded" />
              <Skeleton className="w-24 h-4 rounded" />
            </div>
            <div className="flex gap-2">
              <Skeleton className="w-36 h-10 rounded-lg" />
              <Skeleton className="w-28 h-10 rounded-lg" />
              <Skeleton className="w-24 h-10 rounded-lg" />
            </div>
          </div>
        </div>

        {/* Seasons Skeleton */}
        <div className="space-y-4">
          <Skeleton className="w-48 h-6 rounded" />
          <div className="space-y-2">
            <Skeleton className="w-full h-14 rounded-lg" />
            <Skeleton className="w-full h-14 rounded-lg" />
            <Skeleton className="w-full h-14 rounded-lg" />
          </div>
        </div>
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
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col mb-20 ">
      {/* Header with Show Info */}
      <div className="flex flex-col md:flex-row gap-6 mb-8">
        {/* Poster */}
        <div className="shrink-0">
          {show.posterUrl ? (
            <Image
              src={show.posterUrl}
              alt={show.name}
              className="w-48 h-72 object-cover rounded-lg shadow-lg"
            />
          ) : (
            <div className="w-48 h-72 bg-default-200 rounded-lg flex items-center justify-center">
              <IconDeviceTv size={64} className="text-blue-400" />
            </div>
          )}
        </div>

        {/* Show Details */}
        <div className="flex-1">
          <Breadcrumbs className="mb-2">
            <BreadcrumbItem href="/libraries">Libraries</BreadcrumbItem>
            <BreadcrumbItem href={`/libraries/${show.libraryId}`}>
              {library?.name || 'Library'}
            </BreadcrumbItem>
            <BreadcrumbItem isCurrent>{show.name}</BreadcrumbItem>
          </Breadcrumbs>

          <div className="flex items-start justify-between gap-4 mb-2">
            <h1 className="text-3xl font-bold">
              {show.name}
              {show.year && (
                <span className="text-default-500 ml-2">({show.year})</span>
              )}
            </h1>
            <div className="flex items-center gap-1 shrink-0">
              <Tooltip content="Refresh Metadata">
                <Button
                  isIconOnly
                  variant="light"
                  size="sm"
                  onPress={handleRefresh}
                  isLoading={refreshing}
                >
                  <IconRefresh size={18} />
                </Button>
              </Tooltip>
              <Tooltip content="Settings">
                <Button
                  isIconOnly
                  variant="light"
                  size="sm"
                  onPress={onSettingsOpen}
                >
                  <IconSettings size={18} />
                </Button>
              </Tooltip>
              <Tooltip content="Delete Show">
                <Button
                  isIconOnly
                  variant="light"
                  size="sm"
                  color="danger"
                  onPress={onDeleteOpen}
                >
                  <IconTrash size={18} />
                </Button>
              </Tooltip>
            </div>
          </div>

          <div className="flex flex-wrap gap-2 mb-4">
            <Chip
              size="sm"
              className='capitalize'
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

          {/* Settings Badges - inline with show details */}
          <div className="flex flex-wrap items-center gap-2">
            {/* Auto Download Badge */}
            {(() => {
              const isInherited = show.autoDownloadOverride === null
              const effectiveValue = isInherited ? (library?.autoDownload ?? false) : show.autoDownloadOverride
              const isEnabled = effectiveValue === true
              
              return (
                <AutoDownloadBadge
                  isInherited={isInherited}
                  isEnabled={isEnabled}
                  isLoading={togglingAutoDownload}
                  onClick={() => handleToggleAutoDownload(!effectiveValue)}
                />
              )
            })()}

            {/* File Organization Badge */}
            {(() => {
              const isInherited = show.organizeFilesOverride === null
              const effectiveValue = isInherited ? (library?.organizeFiles ?? false) : show.organizeFilesOverride
              const isEnabled = effectiveValue === true
              
              return (
                <FileOrganizationBadge
                  isInherited={isInherited}
                  isEnabled={isEnabled}
                />
              )
            })()}

            {/* Auto Hunt Badge */}
            {(() => {
              const isInherited = show.autoHuntOverride === null
              const effectiveValue = isInherited ? (library?.autoHunt ?? false) : show.autoHuntOverride
              const isEnabled = effectiveValue === true
              
              return (
                <AutoHuntBadge
                  isInherited={isInherited}
                  isEnabled={isEnabled}
                />
              )
            })()}

            {/* Monitored Badge */}
            <MonitoredBadge monitorType={show.monitorType} />

            {/* Quality Filter Badge */}
            {(() => {
              const isInherited = show.allowedResolutionsOverride === null
              const resolutions = isInherited 
                ? (library?.allowedResolutions || [])
                : (show.allowedResolutionsOverride || [])
              const codecs = isInherited
                ? (library?.allowedVideoCodecs || [])
                : (show.allowedVideoCodecsOverride || [])
              const requireHdr = isInherited
                ? (library?.requireHdr || false)
                : (show.requireHdrOverride || false)

              return (
                <QualityFilterBadge
                  resolutions={resolutions}
                  codecs={codecs}
                  requireHdr={requireHdr}
                  isInherited={isInherited}
                />
              )
            })()}
          </div>
        </div>
      </div>

      {/* Seasons & Episodes */}
      <div className="space-y-4">
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
          <h2 className="text-xl font-semibold">Seasons & Episodes</h2>
          {seasons.length > 0 && (
            <div className="flex flex-wrap gap-2">
              {seasons.map((s) => (
                <Button
                  key={s.season}
                  size="sm"
                  variant="flat"
                  onPress={() => scrollToSeason(s.season)}
                >
                  {s.season === 0 ? 'Specials' : `S${s.season}`}
                </Button>
              ))}
            </div>
          )}
        </div>

        {seasons.length === 0 ? (
          <Card className="bg-content1/50 border-default-300 border-dashed border-2">
            <CardBody className="py-12 text-center">
              <IconClipboard size={48} className="mx-auto mb-4 text-default-400" />
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
                id={`season-${seasonData.season}`}
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
                <EpisodeTable
                  episodes={seasonData.episodes}
                  seasonNumber={seasonData.season}
                  downloadingEpisodes={downloadingEpisodes}
                  onDownload={handleDownloadEpisode}
                  onPlay={handlePlay}
                  onSearch={handleSearchEpisode}
                />
              </AccordionItem>
            ))}
          </Accordion>
        )}
      </div>

      {/* Delete Confirmation Modal */}
      <DeleteShowModal
        isOpen={isDeleteOpen}
        onClose={onDeleteClose}
        showName={show.name}
        onConfirm={handleDelete}
        isLoading={deleting}
      />

      {/* Settings Modal */}
      <ShowSettingsModal
        isOpen={isSettingsOpen}
        onClose={onSettingsClose}
        show={show}
        onSave={handleSaveSettings}
        isLoading={savingSettings}
      />

    </div>
  )
}
