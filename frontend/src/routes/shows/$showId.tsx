import { createFileRoute, Link, redirect, useNavigate } from '@tanstack/react-router'
import { useState, useEffect, useCallback, useMemo, useRef } from 'react'
import { Button } from '@heroui/button'
import { Card, CardBody } from '@heroui/card'
import { Chip } from '@heroui/chip'
import { Image } from '@heroui/image'
import { ShimmerLoader } from '../../components/shared/ShimmerLoader'
import { showTemplate, libraryTemplate } from '../../lib/template-data'
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
  DELETE_TV_SHOW_MUTATION,
  UPDATE_TV_SHOW_MUTATION,
  MEDIA_FILE_UPDATED_SUBSCRIPTION,
  type TvShow,
  type Library,
  type Episode,
  type TvShowResult,
  type MediaFileUpdatedEvent,
} from '../../lib/graphql'
import { formatBytes, formatDate } from '../../lib/format'
import { DataTable, type DataTableColumn, type RowAction } from '../../components/data-table'
import { IconDeviceTv, IconClipboard, IconPlayerPlay, IconPlayerTrackNext, IconRefresh, IconSearch, IconSettings, IconTrash, IconInfoCircle } from '@tabler/icons-react'
import { Tooltip } from '@heroui/tooltip'
import { DeleteShowModal, ShowSettingsModal, type ShowSettingsInput } from '../../components/shows'
import {
  EpisodeStatusChip,
  AutoDownloadBadge,
  AutoHuntBadge,
  FileOrganizationBadge,
  MonitoredBadge,
  QualityFilterBadge,
  PlayPauseIndicator,
} from '../../components/shared'
import { usePlaybackContext } from '../../contexts/PlaybackContext'
import { FilePropertiesModal } from '../../components/FilePropertiesModal'

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

// Helper to format video codec display name
function formatVideoCodec(codec: string | null): string {
  if (!codec) return ''
  const normalized = codec.toLowerCase()
  if (normalized.includes('hevc') || normalized === 'h265') return 'HEVC'
  if (normalized.includes('h264') || normalized === 'avc') return 'H.264'
  if (normalized.includes('av1')) return 'AV1'
  if (normalized.includes('vp9')) return 'VP9'
  return codec.toUpperCase()
}

// Helper to format audio codec display name
function formatAudioCodec(codec: string | null, channels: string | null): string {
  if (!codec) return ''
  const normalized = codec.toLowerCase()
  let name = codec.toUpperCase()
  if (normalized.includes('truehd')) name = 'TrueHD'
  else if (normalized.includes('atmos')) name = 'Atmos'
  else if (normalized.includes('dts')) name = 'DTS'
  else if (normalized.includes('aac')) name = 'AAC'
  else if (normalized.includes('ac3') || normalized.includes('ac-3')) name = 'AC3'
  else if (normalized.includes('eac3') || normalized.includes('e-ac-3')) name = 'EAC3'
  else if (normalized.includes('flac')) name = 'FLAC'
  else if (normalized.includes('opus')) name = 'Opus'

  if (channels) {
    return `${name} ${channels}`
  }
  return name
}

// Episode table columns
const episodeColumns: DataTableColumn<Episode>[] = [
  {
    key: 'episode',
    label: '#',
    width: 60,
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
      <div className="flex items-center gap-2">
        <span className="font-medium">
          {ep.title || `Episode ${ep.episode}`}
        </span>
        {ep.isWatched && (
          <span className="text-xs text-success">✓</span>
        )}
      </div>
    ),
  },
  {
    key: 'progress',
    label: 'Progress',
    width: 100,
    render: (ep) => {
      // Show nothing if no media file (not downloaded)
      if (!ep.mediaFileId) {
        return <span className="text-default-400">-</span>
      }
      // Show checkmark if fully watched
      if (ep.isWatched) {
        return <span className="text-success text-sm">Watched</span>
      }
      // Show progress bar for partially watched
      if (ep.watchProgress !== null && ep.watchProgress > 0) {
        return (
          <div className="flex items-center gap-2">
            <div className="h-1.5 w-16 bg-default-200 rounded-full overflow-hidden">
              <div
                className="h-full bg-primary rounded-full"
                style={{ width: `${Math.min(ep.watchProgress * 100, 100)}%` }}
              />
            </div>
            <span className="text-xs text-default-400">
              {Math.round(ep.watchProgress * 100)}%
            </span>
          </div>
        )
      }
      // Not started
      return <span className="text-default-400 text-sm">-</span>
    },
  },
  {
    key: 'airDate',
    label: 'Air Date',
    width: 130,
    sortable: true,
    render: (ep) => (
      <span className="text-default-500 text-sm text-nowrap">
        {formatAirDate(ep.airDate)}
      </span>
    ),
  },
  {
    key: 'quality',
    label: 'Quality',
    width: 220,
    render: (ep) => {
      if (!ep.mediaFileId) {
        return <span className="text-default-400">-</span>
      }
      return (
        <div className="flex items-center gap-1.5 flex-wrap">
          {ep.resolution && (
            <Chip size="sm" variant="flat" color="primary" className="h-5 text-xs">
              {ep.resolution}
            </Chip>
          )}
          {ep.videoCodec && (
            <Chip size="sm" variant="flat" color="secondary" className="h-5 text-xs">
              {formatVideoCodec(ep.videoCodec)}
            </Chip>
          )}
          {ep.isHdr && (
            <Chip size="sm" variant="flat" color="warning" className="h-5 text-xs">
              {ep.hdrType || 'HDR'}
            </Chip>
          )}
        </div>
      )
    },
  },
  {
    key: 'audio',
    label: 'Audio',
    width: 100,
    render: (ep) => {
      if (!ep.mediaFileId || !ep.audioCodec) {
        return <span className="text-default-400">-</span>
      }
      return (
        <Chip size="sm" variant="flat" color="default" className="h-5 text-xs">
          {formatAudioCodec(ep.audioCodec, ep.audioChannels)}
        </Chip>
      )
    },
  },
  {
    key: 'size',
    label: 'Size',
    width: 100,
    render: (ep) => {
      if (!ep.mediaFileId || !ep.fileSizeFormatted) {
        return <span className="text-default-400">-</span>
      }
      return (
        <span className="text-default-500 text-sm text-nowrap">
          {ep.fileSizeFormatted}
        </span>
      )
    },
  },
  {
    key: 'status',
    label: 'Status',
    width: 140,
    sortable: true,
    render: (ep) => (
      <EpisodeStatusChip
        mediaFileId={ep.mediaFileId}
        downloadProgress={ep.downloadProgress}
      />
    ),
  },
]

interface EpisodeTableProps {
  episodes: Episode[]
  seasonNumber: number
  showId: string
  onPlay: (episode: Episode) => void
  onSearch: (episode: Episode) => void
  onShowProperties: (episode: Episode) => void
}

function EpisodeTable({ episodes, seasonNumber, showId, onPlay, onSearch, onShowProperties }: EpisodeTableProps) {
  // Get session and updatePlayback directly from context for reliable updates
  const { session, updatePlayback } = usePlaybackContext()

  // Handle pause directly using context
  const handlePause = useCallback(() => {
    updatePlayback({ isPlaying: false })
  }, [updatePlayback])

  // Compute playing state from session
  const currentlyPlayingEpisodeId = session?.tvShowId === showId ? session?.episodeId : null
  const isPlaying = session?.isPlaying ?? false

  // Helper to determine if episode has resumable progress (any progress but not fully watched)
  const hasResumeProgress = (ep: Episode) =>
    ep.watchProgress !== null && ep.watchProgress > 0 && !ep.isWatched

  // Helper to check if this episode is currently playing
  const isCurrentlyPlaying = (ep: Episode) => currentlyPlayingEpisodeId === ep.id

  // Dynamic key for play actions
  const playActionKey = `play-${currentlyPlayingEpisodeId || 'none'}-${isPlaying}`

  // Helper to check if episode has file (downloaded)
  const hasFile = (ep: Episode) => !!ep.mediaFileId
  // Helper to check if episode is downloading
  const isDownloading = (ep: Episode) => !ep.mediaFileId && ep.downloadProgress != null && ep.downloadProgress > 0
  // Helper to check if episode is wanted (no file, not downloading)
  const isWanted = (ep: Episode) => !ep.mediaFileId && !isDownloading(ep)

  // Row actions - computed fresh on each render to ensure playing state is always current
  const rowActions: RowAction<Episode>[] = [
    // Playing indicator with pause on hover - shown for currently playing episode
    {
      key: `playing-${currentlyPlayingEpisodeId || 'none'}`,
      label: 'Pause',
      icon: <PlayPauseIndicator size={16} isPlaying={isPlaying} colorClass="bg-success" />,
      color: 'default',
      inDropdown: false,
      isVisible: (ep) => hasFile(ep) && isCurrentlyPlaying(ep) && isPlaying,
      onAction: () => handlePause(),
    },
    // Resume action - shown for episodes with progress but not currently playing
    {
      key: `resume-${playActionKey}`,
      label: 'Resume',
      icon: <IconPlayerTrackNext size={16} />,
      color: 'success',
      inDropdown: false,
      isVisible: (ep) => hasFile(ep) && hasResumeProgress(ep) && !isCurrentlyPlaying(ep),
      onAction: (ep) => onPlay(ep),
    },
    // Play action - shown for episodes without progress and not currently playing
    {
      key: `play-${playActionKey}`,
      label: 'Play',
      icon: <IconPlayerPlay size={16} />,
      color: 'success',
      inDropdown: false,
      isVisible: (ep) => hasFile(ep) && !hasResumeProgress(ep) && !isCurrentlyPlaying(ep),
      onAction: (ep) => onPlay(ep),
    },
    {
      key: 'search',
      label: 'Search for Episode',
      icon: <IconSearch size={16} />,
      color: 'default',
      inDropdown: false,
      isVisible: (ep) => isWanted(ep),
      onAction: (ep) => onSearch(ep),
    },
    {
      key: 'properties',
      label: 'File Properties',
      icon: <IconInfoCircle size={16} />,
      color: 'default',
      inDropdown: true,
      isVisible: (ep) => hasFile(ep),
      onAction: (ep) => onShowProperties(ep),
    },
  ]

  // Create selection set for highlighting currently playing episode
  const selectedKeys = useMemo(() => {
    if (currentlyPlayingEpisodeId) {
      return new Set([currentlyPlayingEpisodeId])
    }
    return new Set<string>()
  }, [currentlyPlayingEpisodeId])

  // Key that changes when playback state changes to force re-render
  const tableKey = `episodes-${currentlyPlayingEpisodeId || 'none'}-${isPlaying}`

  return (
    <DataTable
      key={tableKey}
      skeletonDelay={500}
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
      selectionMode={currentlyPlayingEpisodeId ? 'single' : 'none'}
      selectedKeys={selectedKeys}
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
  const [deleting, setDeleting] = useState(false)
  const [savingSettings, setSavingSettings] = useState(false)
  const [togglingAutoDownload, setTogglingAutoDownload] = useState(false)
  const { isOpen: isDeleteOpen, onOpen: onDeleteOpen, onClose: onDeleteClose } = useDisclosure()
  const { isOpen: isSettingsOpen, onOpen: onSettingsOpen, onClose: onSettingsClose } = useDisclosure()
  const { isOpen: isPropertiesOpen, onOpen: onPropertiesOpen, onClose: onPropertiesClose } = useDisclosure()
  const [propertiesEpisode, setPropertiesEpisode] = useState<Episode | null>(null)
  const { startEpisodePlayback } = usePlaybackContext()

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

  // Subscribe to media file updates for real-time quality info updates
  useEffect(() => {
    if (!show) return

    // Subscribe to media file updates (FFmpeg analysis completes)
    const sub = graphqlClient
      .subscription<{ mediaFileUpdated: MediaFileUpdatedEvent }>(
        MEDIA_FILE_UPDATED_SUBSCRIPTION,
        { libraryId: show.libraryId }
      )
      .subscribe({
        next: (result) => {
          if (result.data?.mediaFileUpdated) {
            const event = result.data.mediaFileUpdated
            // Check if this update is for one of our episodes
            if (event.episodeId) {
              setEpisodes((prev) =>
                prev.map((ep) =>
                  ep.id === event.episodeId
                    ? {
                      ...ep,
                      resolution: event.resolution ?? ep.resolution,
                      videoCodec: event.videoCodec ?? ep.videoCodec,
                      audioCodec: event.audioCodec ?? ep.audioCodec,
                      audioChannels: event.audioChannels ?? ep.audioChannels,
                      isHdr: event.isHdr ?? ep.isHdr,
                      hdrType: event.hdrType ?? ep.hdrType,
                    }
                    : ep
                )
              )
            }
          }
        },
      })

    return () => sub.unsubscribe()
  }, [show?.id, show?.libraryId])

  // Subscribe to torrent completions and focus events for data refresh
  useDataReactivity(
    () => {
      if (initialLoadDone.current) {
        fetchData(true)
      }
    },
    { onTorrentComplete: true, periodicInterval: false, onFocus: true }
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
      // Determine start position:
      // - If watched (>=90%), restart from beginning
      // - If has progress (5-90%), resume from saved position
      // - Otherwise, start from beginning
      let startPosition = 0
      if (episode.watchPosition && episode.watchProgress !== null) {
        if (!episode.isWatched && episode.watchProgress > 0) {
          // Resume from saved position
          startPosition = episode.watchPosition
        }
        // If watched, start from 0 (replay)
      }

      // Start playback (this will trigger the persistent player)
      await startEpisodePlayback(episode.id, episode.mediaFileId, show.id, episode, show, startPosition)
    }
  }, [show, startEpisodePlayback])

  // Note: Playing state and pause handling are now inside EpisodeTable component directly from context

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

  // Show file properties modal for an episode
  const handleShowProperties = useCallback((episode: Episode) => {
    setPropertiesEpisode(episode)
    onPropertiesOpen()
  }, [onPropertiesOpen])

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
        downloadedCount: eps.filter((e) => !!e.mediaFileId).length,
        totalCount: eps.length,
      }))
      .sort((a, b) => a.season - b.season)
  }, [episodes])

  // Calculate totals (memoized to avoid recalculating on every render)
  const { totalEpisodes, downloadedEpisodes, missingEpisodes } = useMemo(() => ({
    totalEpisodes: episodes.length,
    downloadedEpisodes: episodes.filter((e) => !!e.mediaFileId).length,
    missingEpisodes: episodes.filter((e) => !e.mediaFileId).length,
  }), [episodes])

  // Show not found state only after loading is complete
  if (!loading && !show) {
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

  // Use template data during loading, real data when available
  const displayShow = show ?? showTemplate
  const displayLibrary = library ?? libraryTemplate

  return (
    <ShimmerLoader loading={loading} delay={500} templateProps={{ show: showTemplate }}>
      <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex flex-col mb-20 ">
        {/* Header with Show Info */}
        <div className="flex flex-col md:flex-row gap-6 mb-8">
          {/* Poster */}
          <div className="shrink-0">
            {displayShow.posterUrl ? (
              <Image
                src={displayShow.posterUrl}
                alt={displayShow.name}
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
              <BreadcrumbItem href={`/libraries/${displayShow.libraryId}`}>
                {displayLibrary.name}
              </BreadcrumbItem>
              <BreadcrumbItem isCurrent>{displayShow.name}</BreadcrumbItem>
            </Breadcrumbs>

            <div className="flex items-start justify-between gap-4 mb-2">
              <h1 className="text-3xl font-bold">
                {displayShow.name}
                {displayShow.year && (
                  <span className="text-default-500 ml-2">({displayShow.year})</span>
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
                color={displayShow.status === 'CONTINUING' ? 'success' : 'default'}
                variant="flat"
              >
                {displayShow.status}
              </Chip>
              {displayShow.network && (
                <Chip size="sm" variant="flat">
                  {displayShow.network}
                </Chip>
              )}
            </div>

            {displayShow.overview && (
              <p className="text-default-600 mb-4 line-clamp-3">{displayShow.overview}</p>
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
              {displayShow.sizeBytes > 0 && (
                <div>
                  <span className="font-semibold text-foreground">{formatBytes(displayShow.sizeBytes)}</span>
                  <span> on disk</span>
                </div>
              )}
            </div>

            {/* Settings Badges - inline with show details */}
            <div className="flex flex-wrap items-center gap-2">
              {/* Auto Download Badge */}
              {(() => {
                const isInherited = displayShow.autoDownloadOverride === null
                const effectiveValue = isInherited ? (displayLibrary.autoDownload ?? false) : displayShow.autoDownloadOverride
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
                const isInherited = displayShow.organizeFilesOverride === null
                const effectiveValue = isInherited ? (displayLibrary.organizeFiles ?? false) : displayShow.organizeFilesOverride
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
                const isInherited = displayShow.autoHuntOverride === null
                const effectiveValue = isInherited ? (displayLibrary.autoHunt ?? false) : displayShow.autoHuntOverride
                const isEnabled = effectiveValue === true

                return (
                  <AutoHuntBadge
                    isInherited={isInherited}
                    isEnabled={isEnabled}
                  />
                )
              })()}

              {/* Monitored Badge */}
              <MonitoredBadge monitorType={displayShow.monitorType} />

              {/* Quality Filter Badge */}
              {(() => {
                const isInherited = displayShow.allowedResolutionsOverride === null
                const resolutions = isInherited
                  ? (displayLibrary.allowedResolutions || [])
                  : (displayShow.allowedResolutionsOverride || [])
                const codecs = isInherited
                  ? (displayLibrary.allowedVideoCodecs || [])
                  : (displayShow.allowedVideoCodecsOverride || [])
                const requireHdr = isInherited
                  ? (displayLibrary.requireHdr || false)
                  : (displayShow.requireHdrOverride || false)

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
                    showId={displayShow.id}
                    onPlay={handlePlay}
                    onSearch={handleSearchEpisode}
                    onShowProperties={handleShowProperties}
                  />
                </AccordionItem>
              ))}
            </Accordion>
          )}
        </div>

        {/* Delete Confirmation Modal */}
        {show && (
          <DeleteShowModal
            isOpen={isDeleteOpen}
            onClose={onDeleteClose}
            showName={show.name}
            onConfirm={handleDelete}
            isLoading={deleting}
          />
        )}

        {/* Settings Modal */}
        {show && (
          <ShowSettingsModal
            isOpen={isSettingsOpen}
            onClose={onSettingsClose}
            show={show}
            onSave={handleSaveSettings}
            isLoading={savingSettings}
          />
        )}

        {/* File Properties Modal */}
        <FilePropertiesModal
          isOpen={isPropertiesOpen}
          onClose={() => {
            onPropertiesClose()
            setPropertiesEpisode(null)
          }}
          mediaFileId={propertiesEpisode?.mediaFileId ?? null}
          title={propertiesEpisode ? `${displayShow.name} - S${String(propertiesEpisode.season).padStart(2, '0')}E${String(propertiesEpisode.episode).padStart(2, '0')}${propertiesEpisode.title ? ` - ${propertiesEpisode.title}` : ''}` : undefined}
        />

      </div>
    </ShimmerLoader>
  )
}
